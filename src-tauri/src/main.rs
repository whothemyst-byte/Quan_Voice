#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex as StdMutex,
    Arc,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::Instant;

mod audio;
mod assemblyai;
mod asr_worker;
mod hotkey;
mod inject;
mod settings;
mod startup;
mod whisper;

use audio::AudioRecorder;
use asr_worker::{AsrWorkerHandle, AsrWorkerSnapshot};
use hotkey::HotkeyManager;
use inject::TextInjector;
use settings::{AppSettings, SettingsManager};
use tauri::Emitter;
use tauri::Manager;
use tauri::State;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use whisper::WhisperEngine;

struct AppState {
    settings: Arc<Mutex<SettingsManager>>,
    hotkey: Arc<Mutex<HotkeyManager>>,
    audio: Arc<Mutex<AudioRecorder>>,
    whisper: Arc<Mutex<WhisperEngine>>,
    injector: Arc<Mutex<TextInjector>>,
    workflow: Arc<Mutex<()>>,
    recording_flag: Arc<AtomicBool>,
    asr_worker: Arc<AsrWorkerHandle>,
    preview_busy: Arc<AtomicBool>,
    preview_cancel: Arc<AtomicBool>,
    preview_state: Arc<StdMutex<PreviewState>>,
    live: Arc<StdMutex<LiveState>>,
}

#[derive(Clone, serde::Serialize)]
struct LiveState {
    status: String,
    input_level: f64,
}

#[derive(Clone, serde::Serialize)]
struct TranscriptionRun {
    raw: String,
    cleaned: String,
    rms: f32,
    device: String,
}

#[derive(Clone, serde::Serialize)]
struct TranscriptionMetrics {
    provider: String,
    model: String,
    device: String,
    release_ts: u128,
    wav_ready_ts: u128,
    decode_start_ts: u128,
    first_token_ts: u128,
    final_text_ts: u128,
    inject_done_ts: u128,
    stop_audio_ms: u128,
    transcribe_ms: u128,
    clean_ms: u128,
    inject_ms: u128,
    total_ms: u128,
    raw_chars: usize,
    cleaned_chars: usize,
}

#[derive(Clone, Copy)]
enum TranscriptionBackend {
    Offline,
    Online,
}

struct TranscriptionOutcome {
    run: TranscriptionRun,
    metrics: TranscriptionMetrics,
}

#[derive(Default)]
struct PreviewState {
    text: String,
    updated_ts: u128,
    last_seen_text: String,
    stable_count: u8,
}

const ENABLE_PREVIEW_PHASE3: bool = true;

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let manager = state.settings.lock().await;
    let settings = manager.load().map_err(|err| err.to_string())?;
    startup::sync_startup(settings.start_with_windows).map_err(|err| err.to_string())?;
    Ok(settings)
}

#[tauri::command]
async fn update_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    let manager = state.settings.lock().await;
    manager.save(&settings).map_err(|err| err.to_string())?;
    startup::sync_startup(settings.start_with_windows).map_err(|err| err.to_string())
}

#[tauri::command]
async fn ensure_model_ready(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let model = {
        let manager = state.settings.lock().await;
        let settings = manager.load().map_err(|err| err.to_string())?;
        effective_model_id(&settings)
    };
    let mut whisper = state.whisper.lock().await;
    whisper
        .ensure_model_exists(&model, |progress| {
            app.emit("model-download-progress", progress)
                .map_err(|err| whisper::WhisperError::DownloadFailed(err.to_string()))
        })
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn list_available_models() -> Vec<whisper::AvailableModel> {
    whisper::list_phase_one_models()
}

#[tauri::command]
async fn check_assemblyai_connectivity() -> Result<bool, String> {
    if resolve_assemblyai_key().trim().is_empty() {
        return Ok(false);
    }
    Ok(assemblyai::can_reach_assemblyai().await)
}

#[tauri::command]
async fn register_hotkey(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let settings = {
        let manager = state.settings.lock().await;
        manager.load().map_err(|err| err.to_string())?
    };
    {
        let mut audio = state.audio.lock().await;
        audio.set_preferred_device(Some(settings.input_device.clone()));
    }

    let audio_for_press = Arc::clone(&state.audio);
    let audio_for_release = Arc::clone(&state.audio);
    let whisper_for_release = Arc::clone(&state.whisper);
    let injector_for_release = Arc::clone(&state.injector);
    let settings_for_release = Arc::clone(&state.settings);
    let workflow_for_release = Arc::clone(&state.workflow);
    let recording_flag_for_press = Arc::clone(&state.recording_flag);
    let recording_flag_for_release = Arc::clone(&state.recording_flag);
    let preview_busy_for_press = Arc::clone(&state.preview_busy);
    let preview_state_for_press = Arc::clone(&state.preview_state);
    let preview_state_for_release = Arc::clone(&state.preview_state);
    let preview_cancel_for_press = Arc::clone(&state.preview_cancel);
    let preview_cancel_for_release = Arc::clone(&state.preview_cancel);
    let whisper_for_preview = Arc::clone(&state.whisper);
    let settings_for_preview = Arc::clone(&state.settings);
    let live_for_press = Arc::clone(&state.live);
    let live_for_release = Arc::clone(&state.live);
    let on_press_app = app.clone();
    let on_release_app = app;

    let mut hotkey = state.hotkey.lock().await;
    hotkey
        .register_system_hotkey(
            &settings.push_to_talk_key,
            move || {
                let audio = Arc::clone(&audio_for_press);
                let recording_flag = Arc::clone(&recording_flag_for_press);
                let preview_busy = Arc::clone(&preview_busy_for_press);
                let preview_state = Arc::clone(&preview_state_for_press);
                let preview_cancel = Arc::clone(&preview_cancel_for_press);
                let whisper = Arc::clone(&whisper_for_preview);
                let settings = Arc::clone(&settings_for_preview);
                let live = Arc::clone(&live_for_press);
                let app = on_press_app.clone();
                tauri::async_runtime::spawn(async move {
                    let mut recorder = audio.lock().await;
                    if recorder.start().is_ok() {
                        if let Ok(mut preview) = preview_state.lock() {
                            preview.text.clear();
                            preview.updated_ts = 0;
                            preview.last_seen_text.clear();
                            preview.stable_count = 0;
                        }
                        preview_cancel.store(false, Ordering::Relaxed);
                        recording_flag.store(true, Ordering::Relaxed);
                        if let Ok(mut guard) = live.lock() {
                            guard.status = "Listening".to_string();
                            guard.input_level = 0.0;
                        }
                        let _ = app.emit("hotkey-status", "Listening");
                        let _ = app.emit_to("floating", "hotkey-status", "Listening");
                        let _ = app.emit("input-level", 0.0_f64);
                        let _ = app.emit_to("floating", "input-level", 0.0_f64);
                        drop(recorder);

                        let mut last_preview_tick = Instant::now();
                        while recording_flag.load(Ordering::Relaxed) {
                            let level = {
                                let recorder = audio.lock().await;
                                recorder.current_input_level() as f64
                            };
                            let visual = (level * 8.0).clamp(0.0, 1.0);
                            if let Ok(mut guard) = live.lock() {
                                guard.input_level = visual;
                            }
                            let _ = app.emit("input-level", visual);
                            let _ = app.emit_to("floating", "input-level", visual);

                            let should_preview = ENABLE_PREVIEW_PHASE3
                                && last_preview_tick.elapsed() >= Duration::from_millis(360);
                            if should_preview && !preview_busy.swap(true, Ordering::Relaxed) {
                                last_preview_tick = Instant::now();
                                let audio_for_preview = Arc::clone(&audio);
                                let whisper_for_preview = Arc::clone(&whisper);
                                let settings_for_preview = Arc::clone(&settings);
                                let preview_state_for_task = Arc::clone(&preview_state);
                                let preview_busy_for_task = Arc::clone(&preview_busy);
                                let preview_cancel_for_task = Arc::clone(&preview_cancel);
                                let app_for_preview = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    if preview_cancel_for_task.load(Ordering::Relaxed) {
                                        preview_busy_for_task.store(false, Ordering::Relaxed);
                                        return;
                                    }
                                    let samples = {
                                        let recorder = audio_for_preview.lock().await;
                                        recorder.preview_clip_16k(10)
                                    };

                                    if let Some(samples) = samples {
                                        if samples.len() >= 16_000 {
                                            let model = {
                                                let manager = settings_for_preview.lock().await;
                                                let loaded = manager.load().ok();
                                                loaded
                                                    .map(|s| preview_model_id(&s))
                                                    .unwrap_or_else(|| "tiny.en".to_string())
                                            };

                                            if let Ok(engine) = whisper_for_preview.try_lock() {
                                                if !preview_cancel_for_task.load(Ordering::Relaxed) {
                                                    let decode = engine.transcribe_preview_samples(&samples, &model).await;
                                                    if let Ok(decoded) = decode {
                                                        let cleaned = settings::text::clean_transcript(&decoded.text);
                                                        let low_signal = {
                                                            let recorder = audio_for_preview.lock().await;
                                                            recorder.current_input_level()
                                                        };
                                                        if !settings::text::should_reject_for_low_signal(&cleaned, low_signal) {
                                                            if let Ok(mut preview) = preview_state_for_task.lock() {
                                                                if preview.last_seen_text.eq_ignore_ascii_case(&cleaned) {
                                                                    preview.stable_count = preview.stable_count.saturating_add(1);
                                                                } else {
                                                                    preview.last_seen_text = cleaned.clone();
                                                                    preview.stable_count = 1;
                                                                }

                                                                // Commit quickly for instant-first-text; final path still validates.
                                                                if preview.stable_count >= 1 {
                                                                    preview.text = cleaned.clone();
                                                                    preview.updated_ts = epoch_ms();
                                                                }
                                                            }
                                                            let _ = app_for_preview.emit("transcription-preview", &cleaned);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    preview_busy_for_task.store(false, Ordering::Relaxed);
                                });
                            }
                            sleep(Duration::from_millis(45)).await;
                        }
                    }
                });
            },
            move || {
                let audio = Arc::clone(&audio_for_release);
                let whisper = Arc::clone(&whisper_for_release);
                let injector = Arc::clone(&injector_for_release);
                let settings = Arc::clone(&settings_for_release);
                let workflow = Arc::clone(&workflow_for_release);
                let recording_flag = Arc::clone(&recording_flag_for_release);
                let preview_cancel = Arc::clone(&preview_cancel_for_release);
                let preview_state = Arc::clone(&preview_state_for_release);
                let live = Arc::clone(&live_for_release);
                let app = on_release_app.clone();
                tauri::async_runtime::spawn(async move {
                    let release_ts = epoch_ms();
                    recording_flag.store(false, Ordering::Relaxed);
                    preview_cancel.store(true, Ordering::Relaxed);
                    if let Ok(mut guard) = live.lock() {
                        guard.status = "Ready".to_string();
                        guard.input_level = 0.0;
                    }
                    let _ = app.emit("hotkey-status", "Ready");
                    let _ = app.emit_to("floating", "hotkey-status", "Ready");
                    let _ = app.emit("input-level", 0.0_f64);
                    let _ = app.emit_to("floating", "input-level", 0.0_f64);

                    let recent_preview = if ENABLE_PREVIEW_PHASE3 {
                        let now = epoch_ms();
                        preview_state
                            .lock()
                            .ok()
                            .and_then(|state| {
                                if state.text.is_empty() {
                                    return None;
                                }
                                if now.saturating_sub(state.updated_ts) <= 1_500 {
                                    Some(state.text.clone())
                                } else {
                                    None
                                }
                            })
                    } else {
                        None
                    };

                    if ENABLE_PREVIEW_PHASE3 {
                        if let Some(preview_text) = recent_preview.as_ref() {
                            let inject_preview = {
                                let target = injector.lock().await;
                                target.inject_text(preview_text).map_err(|err| err.to_string())
                            };
                            if inject_preview.is_ok() {
                                let _ = app.emit("transcription-result", preview_text);
                                let _ = app.emit_to("floating", "transcription-result", preview_text);
                            }
                        }
                    }

                    let _guard = workflow.lock().await;
                    let outcome = transcribe_and_inject(
                        Some(app.clone()),
                        audio,
                        whisper,
                        injector,
                        settings,
                        release_ts,
                        recent_preview,
                    )
                    .await;
                    if let Ok(mut guard) = live.lock() {
                        guard.status = "Ready".to_string();
                        guard.input_level = 0.0;
                    }
                    let _ = app.emit("hotkey-status", "Ready");
                    let _ = app.emit_to("floating", "hotkey-status", "Ready");
                    let _ = app.emit("input-level", 0.0_f64);
                    let _ = app.emit_to("floating", "input-level", 0.0_f64);

                    match outcome {
                        Ok(outcome) => {
                            let _ = app.emit("transcription-result", &outcome.run.cleaned);
                            let _ = app.emit_to("floating", "transcription-result", &outcome.run.cleaned);
                            let _ = app.emit("transcription-metrics", &outcome.metrics);
                            persist_transcription_metrics(&outcome.metrics);
                            eprintln!(
                                "transcription metrics | model={} | release_ts={} | wav_ready_ts={} | decode_start_ts={} | first_token_ts={} | final_text_ts={} | inject_done_ts={} | total={}ms | stop={}ms | transcribe={}ms | clean={}ms | inject={}ms | raw_chars={} | cleaned_chars={}",
                                outcome.metrics.model,
                                outcome.metrics.release_ts,
                                outcome.metrics.wav_ready_ts,
                                outcome.metrics.decode_start_ts,
                                outcome.metrics.first_token_ts,
                                outcome.metrics.final_text_ts,
                                outcome.metrics.inject_done_ts,
                                outcome.metrics.total_ms,
                                outcome.metrics.stop_audio_ms,
                                outcome.metrics.transcribe_ms,
                                outcome.metrics.clean_ms,
                                outcome.metrics.inject_ms,
                                outcome.metrics.raw_chars,
                                outcome.metrics.cleaned_chars
                            );
                        }
                        Err(err) => {
                            let _ = app.emit("hotkey-error", &err);
                            let _ = app.emit_to("floating", "hotkey-error", &err);
                        }
                    }
                });
            },
        )
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn unregister_hotkey(state: State<'_, AppState>) -> Result<(), String> {
    state.recording_flag.store(false, Ordering::Relaxed);
    let mut hotkey = state.hotkey.lock().await;
    hotkey.unregister_system_hotkey();
    if let Ok(mut live) = state.live.lock() {
        live.status = "Inactive".to_string();
        live.input_level = 0.0;
    }
    Ok(())
}

#[tauri::command]
async fn list_input_devices() -> Result<Vec<String>, String> {
    let mut out = vec!["Default".to_string()];
    let mut names = AudioRecorder::list_input_devices().map_err(|err| err.to_string())?;
    out.append(&mut names);
    Ok(out)
}

#[tauri::command]
fn show_floating_widget(app: tauri::AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("floating") else {
        return Err("floating window not found".to_string());
    };

    window.show().map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn hide_floating_widget(app: tauri::AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("floating") else {
        return Err("floating window not found".to_string());
    };

    window.hide().map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn start_floating_drag(app: tauri::AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("floating") else {
        return Err("floating window not found".to_string());
    };

    window.start_dragging().map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_live_state(state: State<'_, AppState>) -> Result<LiveState, String> {
    let live = state.live.lock().map_err(|_| "live state lock poisoned".to_string())?;
    Ok(live.clone())
}

#[tauri::command]
fn set_live_ready(state: State<'_, AppState>) -> Result<(), String> {
    let mut live = state.live.lock().map_err(|_| "live state lock poisoned".to_string())?;
    live.status = "Ready".to_string();
    live.input_level = 0.0;
    Ok(())
}

#[tauri::command]
async fn get_asr_worker_snapshot(state: State<'_, AppState>) -> Result<AsrWorkerSnapshot, String> {
    Ok(state.asr_worker.snapshot().await)
}

#[tauri::command]
async fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    let settings = {
        let manager = state.settings.lock().await;
        manager.load().map_err(|err| err.to_string())?
    };
    let mut audio = state.audio.lock().await;
    audio.set_preferred_device(Some(settings.input_device));
    audio.start().map_err(|err| err.to_string())
}

#[tauri::command]
async fn stop_recording_and_inject(state: State<'_, AppState>) -> Result<String, String> {
    let release_ts = epoch_ms();
    let outcome = transcribe_and_inject(
        None,
        Arc::clone(&state.audio),
        Arc::clone(&state.whisper),
        Arc::clone(&state.injector),
        Arc::clone(&state.settings),
        release_ts,
        None,
    )
    .await?;
    Ok(outcome.run.cleaned)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "floating",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Quan Voice Widget")
            .inner_size(180.0, 44.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible_on_all_workspaces(true)
            .visible(false)
            .build()
            .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

            Ok(())
        })
        .manage(AppState {
            settings: Arc::new(Mutex::new(SettingsManager::new())),
            hotkey: Arc::new(Mutex::new(HotkeyManager::new())),
            audio: Arc::new(Mutex::new(AudioRecorder::new())),
            whisper: Arc::new(Mutex::new(WhisperEngine::new())),
            injector: Arc::new(Mutex::new(TextInjector::new())),
            workflow: Arc::new(Mutex::new(())),
            recording_flag: Arc::new(AtomicBool::new(false)),
            asr_worker: Arc::new(AsrWorkerHandle::spawn(32)),
            preview_busy: Arc::new(AtomicBool::new(false)),
            preview_cancel: Arc::new(AtomicBool::new(false)),
            preview_state: Arc::new(StdMutex::new(PreviewState::default())),
            live: Arc::new(StdMutex::new(LiveState {
                status: "Inactive".to_string(),
                input_level: 0.0,
            })),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            ensure_model_ready,
            check_assemblyai_connectivity,
            register_hotkey,
            unregister_hotkey,
            show_floating_widget,
            hide_floating_widget,
            start_floating_drag,
            get_live_state,
            set_live_ready,
            get_asr_worker_snapshot,
            list_available_models,
            list_input_devices,
            start_recording,
            stop_recording_and_inject
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Quan Voice");
}

async fn transcribe_and_inject(
    app: Option<tauri::AppHandle>,
    audio: Arc<Mutex<AudioRecorder>>,
    whisper: Arc<Mutex<WhisperEngine>>,
    injector: Arc<Mutex<TextInjector>>,
    settings: Arc<Mutex<SettingsManager>>,
    release_ts: u128,
    preview_injected: Option<String>,
) -> Result<TranscriptionOutcome, String> {
    let loaded_settings = {
        let manager = settings.lock().await;
        manager.load().map_err(|err| err.to_string())?
    };
    let selected_model = effective_model_id(&loaded_settings);
    let assemblyai_key = resolve_assemblyai_key();
    let backend = resolve_backend(&loaded_settings).await;
    if loaded_settings.online_mode && !matches!(backend, TranscriptionBackend::Online) {
        if let Some(app) = app.as_ref() {
            let _ = app.emit(
                "mode-notification",
                "Internet is off or company key QUAN_VOICE_ASSEMBLYAI_API_KEY is not configured. Switched to offline mode.",
            );
            let _ = app.emit_to(
                "floating",
                "mode-notification",
                "Internet is off or company key QUAN_VOICE_ASSEMBLYAI_API_KEY is not configured. Switched to offline mode.",
            );
        }
    };

    let (audio_path, device_name, level_hint) = {
        let mut recorder = audio.lock().await;
        let level_hint = recorder.current_input_level();
        let device_name = recorder
            .active_device_name()
            .unwrap_or_else(|| "Unknown input device".to_string());
        let path = recorder.stop().map_err(|err| err.to_string())?;
        (path, device_name, level_hint)
    };
    let wav_ready_ts = epoch_ms();

    if level_hint < 0.002 {
        return Err(format!(
            "No microphone signal detected from `{}`. Select the correct mic in Settings > Input device.",
            device_name
        ));
    }

    let decode_start_ts = epoch_ms();
    let decode = match backend {
        TranscriptionBackend::Offline => {
            let mut engine = whisper.lock().await;
            engine
                .transcribe(audio_path.as_path(), &selected_model)
                .await
                .map_err(|err| err.to_string())?
        }
        TranscriptionBackend::Online => {
            let online = assemblyai::transcribe_wav(
                audio_path.as_path(),
                &assemblyai_key,
            )
            .await;
            match online {
                Ok(decoded) => whisper::DecodeResult {
                    text: decoded.text,
                    first_token_ts: decoded.first_token_ts,
                    final_text_ts: decoded.final_text_ts,
                },
                Err(err) => {
                    if let Some(app) = app.as_ref() {
                        let message = format!(
                            "Online transcription failed ({err}). Falling back to offline mode for this utterance."
                        );
                        let _ = app.emit("mode-notification", &message);
                        let _ = app.emit_to("floating", "mode-notification", &message);
                    }
                    let mut engine = whisper.lock().await;
                    engine
                        .transcribe(audio_path.as_path(), &selected_model)
                        .await
                        .map_err(|offline_err| {
                            format!("online failure: {err}; offline fallback failure: {offline_err}")
                        })?
                }
            }
        }
    };
    let raw_text = decode.text;
    let first_token_ts = decode.first_token_ts.unwrap_or(decode_start_ts);
    let final_text_ts = decode.final_text_ts;
    let transcribe_ms = final_text_ts.saturating_sub(decode_start_ts);

    let clean_started = epoch_ms();
    let cleaned = settings::text::clean_transcript(&raw_text);
    let clean_done = epoch_ms();
    let clean_ms = clean_done.saturating_sub(clean_started);
    if cleaned.is_empty() {
        let inject_done_ts = epoch_ms();
        let run = TranscriptionRun {
            raw: raw_text,
            cleaned,
            rms: level_hint,
            device: device_name,
        };
        let metrics = TranscriptionMetrics {
            provider: backend_label(backend).to_string(),
            model: selected_model,
            device: run.device.clone(),
            release_ts,
            wav_ready_ts,
            decode_start_ts,
            first_token_ts,
            final_text_ts,
            inject_done_ts,
            stop_audio_ms: wav_ready_ts.saturating_sub(release_ts),
            transcribe_ms,
            clean_ms,
            inject_ms: 0,
            total_ms: inject_done_ts.saturating_sub(release_ts),
            raw_chars: run.raw.chars().count(),
            cleaned_chars: 0,
        };
        return Ok(TranscriptionOutcome { run, metrics });
    }
    if settings::text::should_reject_for_low_signal(&cleaned, level_hint) {
        let inject_done_ts = epoch_ms();
        let run = TranscriptionRun {
            raw: raw_text,
            cleaned: String::new(),
            rms: level_hint,
            device: device_name,
        };
        let metrics = TranscriptionMetrics {
            provider: backend_label(backend).to_string(),
            model: selected_model,
            device: run.device.clone(),
            release_ts,
            wav_ready_ts,
            decode_start_ts,
            first_token_ts,
            final_text_ts,
            inject_done_ts,
            stop_audio_ms: wav_ready_ts.saturating_sub(release_ts),
            transcribe_ms,
            clean_ms,
            inject_ms: 0,
            total_ms: inject_done_ts.saturating_sub(release_ts),
            raw_chars: run.raw.chars().count(),
            cleaned_chars: 0,
        };
        return Ok(TranscriptionOutcome { run, metrics });
    }

    let inject_start_ts = epoch_ms();
    let to_inject = match preview_injected {
        Some(prefix) if !prefix.is_empty() => suffix_delta_at_word_boundary(&prefix, &cleaned),
        _ => cleaned.clone(),
    };

    if !to_inject.is_empty() {
        let target = injector.lock().await;
        target.inject_text(&to_inject).map_err(|err| err.to_string())?;
    }
    let inject_done_ts = epoch_ms();
    let inject_ms = inject_done_ts.saturating_sub(inject_start_ts);

    let run = TranscriptionRun {
        raw: raw_text,
        cleaned,
        rms: level_hint,
        device: device_name,
    };
        let metrics = TranscriptionMetrics {
        provider: backend_label(backend).to_string(),
        model: selected_model,
        device: run.device.clone(),
        release_ts,
        wav_ready_ts,
        decode_start_ts,
        first_token_ts,
        final_text_ts,
        inject_done_ts,
        stop_audio_ms: wav_ready_ts.saturating_sub(release_ts),
        transcribe_ms,
        clean_ms,
        inject_ms,
        total_ms: inject_done_ts.saturating_sub(release_ts),
        raw_chars: run.raw.chars().count(),
        cleaned_chars: run.cleaned.chars().count(),
    };

    Ok(TranscriptionOutcome { run, metrics })
}

fn effective_model_id(settings: &AppSettings) -> String {
    if settings.latency_mode {
        "tiny.en".to_string()
    } else {
        settings.model.clone()
    }
}

fn preview_model_id(_settings: &AppSettings) -> String {
    // Preview path is optimized for instant appearance; use fastest model always.
    "tiny.en".to_string()
}

async fn resolve_backend(settings: &AppSettings) -> TranscriptionBackend {
    if !settings.online_mode {
        return TranscriptionBackend::Offline;
    }
    if resolve_assemblyai_key().trim().is_empty() {
        return TranscriptionBackend::Offline;
    }
    if !assemblyai::can_reach_assemblyai().await {
        return TranscriptionBackend::Offline;
    }

    TranscriptionBackend::Online
}

fn resolve_assemblyai_key() -> String {
    if let Ok(value) = std::env::var("QUAN_VOICE_ASSEMBLYAI_API_KEY") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    String::new()
}

fn backend_label(backend: TranscriptionBackend) -> &'static str {
    match backend {
        TranscriptionBackend::Offline => "offline",
        TranscriptionBackend::Online => "assemblyai",
    }
}

fn epoch_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn persist_transcription_metrics(metrics: &TranscriptionMetrics) {
    let Ok(line) = serde_json::to_string(metrics) else {
        return;
    };

    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.push("QuanVoice");
    dir.push("metrics");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    let file_path = dir.join("transcription_metrics.jsonl");
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(file_path) else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

fn suffix_delta_at_word_boundary(preview: &str, final_text: &str) -> String {
    if final_text.starts_with(preview) {
        return final_text[preview.len()..].to_string();
    }

    let preview_words = preview.split_whitespace().collect::<Vec<_>>();
    let final_words = final_text.split_whitespace().collect::<Vec<_>>();
    if preview_words.is_empty() || final_words.is_empty() {
        return String::new();
    }

    let mut lcp = 0usize;
    while lcp < preview_words.len()
        && lcp < final_words.len()
        && preview_words[lcp].eq_ignore_ascii_case(final_words[lcp])
    {
        lcp += 1;
    }

    if lcp == 0 || lcp >= final_words.len() {
        return String::new();
    }

    let mut suffix = final_words[lcp..].join(" ");
    if !suffix.is_empty() && !suffix.starts_with([' ', '\n', '\t']) {
        suffix.insert(0, ' ');
    }
    suffix
}
