use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex as StdMutex,
    Arc,
};

mod audio;
mod hotkey;
mod inject;
mod settings;
mod startup;
mod whisper;

use audio::AudioRecorder;
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
    let mut whisper = state.whisper.lock().await;
    whisper
        .ensure_model_exists(|progress| {
            app.emit("model-download-progress", progress)
                .map_err(|err| whisper::WhisperError::DownloadFailed(err.to_string()))
        })
        .await
        .map_err(|err| err.to_string())
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
    let workflow_for_release = Arc::clone(&state.workflow);
    let recording_flag_for_press = Arc::clone(&state.recording_flag);
    let recording_flag_for_release = Arc::clone(&state.recording_flag);
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
                let live = Arc::clone(&live_for_press);
                let app = on_press_app.clone();
                tauri::async_runtime::spawn(async move {
                    let mut recorder = audio.lock().await;
                    if recorder.start().is_ok() {
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
                            sleep(Duration::from_millis(45)).await;
                        }
                    }
                });
            },
            move || {
                let audio = Arc::clone(&audio_for_release);
                let whisper = Arc::clone(&whisper_for_release);
                let injector = Arc::clone(&injector_for_release);
                let workflow = Arc::clone(&workflow_for_release);
                let recording_flag = Arc::clone(&recording_flag_for_release);
                let live = Arc::clone(&live_for_release);
                let app = on_release_app.clone();
                tauri::async_runtime::spawn(async move {
                    recording_flag.store(false, Ordering::Relaxed);
                    let _guard = workflow.lock().await;
                    let outcome = transcribe_and_inject(audio, whisper, injector).await;
                    if let Ok(mut guard) = live.lock() {
                        guard.status = "Idle".to_string();
                        guard.input_level = 0.0;
                    }
                    let _ = app.emit("hotkey-status", "Idle");
                    let _ = app.emit_to("floating", "hotkey-status", "Idle");
                    let _ = app.emit("input-level", 0.0_f64);
                    let _ = app.emit_to("floating", "input-level", 0.0_f64);

                    match outcome {
                        Ok(run) => {
                            let _ = app.emit("transcription-result", &run.cleaned);
                            let _ = app.emit_to("floating", "transcription-result", &run.cleaned);
                            let _ = app.emit("transcription-debug", &run);
                            let _ = app.emit_to("floating", "transcription-debug", &run);
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
    let run = transcribe_and_inject(
        Arc::clone(&state.audio),
        Arc::clone(&state.whisper),
        Arc::clone(&state.injector),
    )
    .await?;
    Ok(run.cleaned)
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
            live: Arc::new(StdMutex::new(LiveState {
                status: "Inactive".to_string(),
                input_level: 0.0,
            })),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            ensure_model_ready,
            register_hotkey,
            unregister_hotkey,
            show_floating_widget,
            hide_floating_widget,
            get_live_state,
            set_live_ready,
            list_input_devices,
            start_recording,
            stop_recording_and_inject
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Quan Voice");
}

async fn transcribe_and_inject(
    audio: Arc<Mutex<AudioRecorder>>,
    whisper: Arc<Mutex<WhisperEngine>>,
    injector: Arc<Mutex<TextInjector>>,
) -> Result<TranscriptionRun, String> {
    let (audio_path, device_name) = {
        let mut recorder = audio.lock().await;
        let device_name = recorder
            .active_device_name()
            .unwrap_or_else(|| "Unknown input device".to_string());
        let path = recorder.stop().map_err(|err| err.to_string())?;
        (path, device_name)
    };

    let rms = wav_rms(audio_path.as_path());
    if rms < 0.0005 {
        return Err(format!(
            "No microphone signal detected (RMS near zero) from `{}`. Select the correct mic in Settings > Input device.",
            device_name
        ));
    }

    let raw_text = {
        let engine = whisper.lock().await;
        engine
            .transcribe(audio_path.as_path())
            .await
            .map_err(|err| err.to_string())?
    };

    let cleaned = settings::text::clean_transcript(&raw_text);
    if cleaned.is_empty() {
        return Ok(TranscriptionRun {
            raw: raw_text,
            cleaned,
            rms,
            device: device_name,
        });
    }

    {
        let target = injector.lock().await;
        target.inject_text(&cleaned).map_err(|err| err.to_string())?;
    }

    Ok(TranscriptionRun {
        raw: raw_text,
        cleaned,
        rms,
        device: device_name,
    })
}

fn wav_rms(path: &std::path::Path) -> f32 {
    let Ok(reader) = hound::WavReader::open(path) else {
        return 0.0;
    };

    let mut count: usize = 0;
    let mut acc: f64 = 0.0;
    for sample in reader.into_samples::<i16>().flatten() {
        let v = sample as f64 / i16::MAX as f64;
        acc += v * v;
        count += 1;
    }

    if count == 0 {
        return 0.0;
    }

    (acc / count as f64).sqrt() as f32
}
