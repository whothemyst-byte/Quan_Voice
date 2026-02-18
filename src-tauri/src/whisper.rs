use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use futures_util::StreamExt;
use serde::Serialize;
use thiserror::Error;
use tokio::process::Command as AsyncCommand;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const FALLBACK_MODEL_ID: &str = "tiny.en";
const MODEL_SOURCE_ROOT: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

struct ModelSpec {
    id: &'static str,
    label: &'static str,
    filename: &'static str,
    min_bytes: u64,
    approx_download_mb: u32,
    perf_warning: &'static str,
    phase_one_enabled: bool,
}

const MODEL_CATALOG: [ModelSpec; 5] = [
    ModelSpec {
        id: "tiny.en",
        label: "tiny.en (Fastest)",
        filename: "ggml-tiny.en.bin",
        min_bytes: 30 * 1024 * 1024,
        approx_download_mb: 75,
        perf_warning: "Fastest and lightest option for most PCs.",
        phase_one_enabled: true,
    },
    ModelSpec {
        id: "base.en",
        label: "base.en",
        filename: "ggml-base.en.bin",
        min_bytes: 120 * 1024 * 1024,
        approx_download_mb: 145,
        perf_warning: "Better accuracy, but slower inference and higher memory use than tiny.en.",
        phase_one_enabled: true,
    },
    ModelSpec {
        id: "small.en",
        label: "small.en",
        filename: "ggml-small.en.bin",
        min_bytes: 220 * 1024 * 1024,
        approx_download_mb: 470,
        perf_warning: "Highest accuracy in phase 1, but much heavier on CPU and RAM.",
        phase_one_enabled: true,
    },
    ModelSpec {
        id: "medium.en",
        label: "medium.en (Advanced)",
        filename: "ggml-medium.en.bin",
        min_bytes: 700 * 1024 * 1024,
        approx_download_mb: 1500,
        perf_warning: "Advanced model with high memory and latency cost.",
        phase_one_enabled: false,
    },
    ModelSpec {
        id: "turbo",
        label: "turbo (Advanced)",
        filename: "ggml-large-v3-turbo.bin",
        min_bytes: 1300 * 1024 * 1024,
        approx_download_mb: 1600,
        perf_warning: "Advanced model; use only on high-end hardware.",
        phase_one_enabled: false,
    },
];

#[derive(Debug, Clone, Serialize)]
pub struct ModelDownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableModel {
    pub id: &'static str,
    pub label: &'static str,
    pub approx_download_mb: u32,
    pub perf_warning: &'static str,
}

#[derive(Debug, Error)]
pub enum WhisperError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("download failed: {0}")]
    DownloadFailed(String),
    #[error("whisper binary not found. expected whisper-cli.exe/main.exe at {0}")]
    MissingBinary(String),
    #[error("unsupported model `{0}`")]
    UnsupportedModel(String),
}

pub struct WhisperEngine {
    models_root: PathBuf,
    binary_path: PathBuf,
}

impl WhisperEngine {
    pub fn new() -> Self {
        let mut root = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        root.push("QuanVoice");
        root.push("models");

        let binary_path = resolve_whisper_binary_path();
        Self {
            models_root: root,
            binary_path,
        }
    }

    pub async fn ensure_model_exists<F>(
        &mut self,
        model_id: &str,
        mut on_progress: F,
    ) -> Result<bool, WhisperError>
    where
        F: FnMut(ModelDownloadProgress) -> Result<(), WhisperError>,
    {
        let spec = phase_one_model_spec(model_id).ok_or_else(|| {
            WhisperError::UnsupportedModel(model_id.to_string())
        })?;
        let model_path = self.model_path_for(spec);
        let expected_sha256 = self.fetch_expected_sha256(spec).await;

        if self.model_is_valid(model_path.as_path(), spec.min_bytes, expected_sha256.as_deref())? {
            on_progress(ModelDownloadProgress {
                downloaded_bytes: 1,
                total_bytes: Some(1),
                progress_percent: 100.0,
            })?;
            return Ok(true);
        }

        if let Some(parent) = model_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let partial = self.partial_model_path(model_path.as_path());
        if partial.exists() {
            let _ = fs::remove_file(&partial);
        }

        self.download_model(
            spec,
            model_path.as_path(),
            expected_sha256.as_deref(),
            &mut on_progress,
        )
            .await?;
        Ok(true)
    }

    pub async fn transcribe(
        &self,
        wav_path: &std::path::Path,
        model_id: &str,
    ) -> Result<String, WhisperError> {
        let spec = phase_one_model_spec(model_id).ok_or_else(|| {
            WhisperError::UnsupportedModel(model_id.to_string())
        })?;
        let model_path = self.model_path_for(spec);

        if !model_path.exists() {
            return Err(WhisperError::TranscriptionFailed(
                "missing whisper model".to_string(),
            ));
        }

        if !self.binary_path.exists() {
            return Err(WhisperError::MissingBinary(
                self.binary_path.display().to_string(),
            ));
        }

        self.transcribe_with_binary(wav_path, model_path.as_path()).await
    }

    async fn download_model<F>(
        &self,
        spec: &ModelSpec,
        model_path: &Path,
        expected_sha256: Option<&str>,
        on_progress: &mut F,
    ) -> Result<(), WhisperError>
    where
        F: FnMut(ModelDownloadProgress) -> Result<(), WhisperError>,
    {
        let response = reqwest::get(model_download_url(spec))
            .await
            .map_err(|err| WhisperError::DownloadFailed(err.to_string()))?;

        if !response.status().is_success() {
            return Err(WhisperError::DownloadFailed(format!(
                "model source returned status {}",
                response.status()
            )));
        }

        let total = response.content_length();
        let mut downloaded: u64 = 0;
        let partial = self.partial_model_path(model_path);
        let mut file = File::create(&partial)?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|err| WhisperError::DownloadFailed(err.to_string()))?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            let progress_percent = match total {
                Some(total_bytes) if total_bytes > 0 => {
                    (downloaded as f64 / total_bytes as f64) * 100.0
                }
                _ => 0.0,
            };

            on_progress(ModelDownloadProgress {
                downloaded_bytes: downloaded,
                total_bytes: total,
                progress_percent,
            })?;
        }

        file.flush()?;
        drop(file);
        if let Some(expected) = total {
            let actual = fs::metadata(&partial)?.len();
            if actual != expected {
                let _ = fs::remove_file(&partial);
                return Err(WhisperError::DownloadFailed(format!(
                    "incomplete model download: expected {expected} bytes, got {actual}"
                )));
            }
        }
        if fs::metadata(&partial)?.len() < spec.min_bytes {
            let _ = fs::remove_file(&partial);
            return Err(WhisperError::DownloadFailed(
                "downloaded model appears corrupted (file too small)".to_string(),
            ));
        }
        if let Some(expected_sha256) = expected_sha256 {
            let actual_sha256 = file_sha256_hex(&partial)?;
            if actual_sha256 != expected_sha256 {
                let _ = fs::remove_file(&partial);
                return Err(WhisperError::DownloadFailed(format!(
                    "model checksum mismatch: expected {expected_sha256}, got {actual_sha256}"
                )));
            }
        }

        fs::rename(&partial, model_path)?;
        on_progress(ModelDownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes: total.or(Some(downloaded)),
            progress_percent: 100.0,
        })?;
        Ok(())
    }

    async fn transcribe_with_binary(
        &self,
        wav_path: &Path,
        model_path: &Path,
    ) -> Result<String, WhisperError> {
        let output_prefix = wav_path.with_file_name("transcript");
        let output_txt = output_prefix.with_extension("txt");
        let _ = fs::remove_file(&output_txt);

        let mut command = AsyncCommand::new(&self.binary_path);
        command
            .arg("-m")
            .arg(model_path)
            .arg("-f")
            .arg(wav_path)
            .arg("-l")
            .arg("en")
            .arg("-nt")
            .arg("--suppress-nst")
            .arg("--temperature")
            .arg("0")
            .arg("--no-speech-thold")
            .arg("0.7")
            .arg("--logprob-thold")
            .arg("-1.0")
            .arg("-otxt")
            .arg("-of")
            .arg(&output_prefix);

        hide_async_command_window(&mut command);

        let output = command
            .output()
            .await
            .map_err(|err| WhisperError::TranscriptionFailed(err.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(WhisperError::TranscriptionFailed(if stderr.is_empty() {
                format!("whisper exited with status {}", output.status)
            } else {
                stderr
            }));
        }

        if output_txt.exists() {
            let text = fs::read_to_string(&output_txt)?;
            return Ok(text.trim().to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(extract_transcript_from_stdout(&stdout))
    }
}

impl WhisperEngine {
    fn model_path_for(&self, spec: &ModelSpec) -> PathBuf {
        self.models_root.join(spec.filename)
    }

    fn partial_model_path(&self, model_path: &Path) -> PathBuf {
        model_path.with_extension("bin.part")
    }

    fn model_is_valid(
        &self,
        model_path: &Path,
        min_bytes: u64,
        expected_sha256: Option<&str>,
    ) -> Result<bool, WhisperError> {
        if !model_path.exists() {
            return Ok(false);
        }

        let size = fs::metadata(model_path)?.len();
        if size < min_bytes {
            return Ok(false);
        }

        let Some(expected_sha256) = expected_sha256 else {
            return Ok(true);
        };

        let actual_sha256 = file_sha256_hex(model_path)?;
        Ok(actual_sha256 == expected_sha256)
    }

    async fn fetch_expected_sha256(&self, spec: &ModelSpec) -> Option<String> {
        let response = reqwest::get(model_sha256_url(spec)).await.ok()?;
        if !response.status().is_success() {
            return None;
        }

        let body = response.text().await.ok()?;
        parse_sha256_hex(&body)
    }
}

pub fn list_phase_one_models() -> Vec<AvailableModel> {
    MODEL_CATALOG
        .iter()
        .filter(|spec| spec.phase_one_enabled)
        .map(|spec| AvailableModel {
            id: spec.id,
            label: spec.label,
            approx_download_mb: spec.approx_download_mb,
            perf_warning: spec.perf_warning,
        })
        .collect()
}

pub fn is_phase_one_model(model_id: &str) -> bool {
    phase_one_model_spec(model_id).is_some()
}

pub fn fallback_model_id() -> &'static str {
    FALLBACK_MODEL_ID
}

fn phase_one_model_spec(model_id: &str) -> Option<&'static ModelSpec> {
    MODEL_CATALOG
        .iter()
        .find(|spec| spec.id == model_id && spec.phase_one_enabled)
}

fn model_download_url(spec: &ModelSpec) -> String {
    format!("{MODEL_SOURCE_ROOT}/{}", spec.filename)
}

fn model_sha256_url(spec: &ModelSpec) -> String {
    format!("{MODEL_SOURCE_ROOT}/{}.sha256", spec.filename)
}

fn resolve_whisper_binary_path() -> PathBuf {
    if let Ok(from_env) = std::env::var("QUAN_VOICE_WHISPER_BIN") {
        let from_env = PathBuf::from(from_env);
        if !from_env.as_os_str().is_empty() {
            return from_env;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let whisper_dir = exe_dir.join("whisper");
            let cli = whisper_dir.join("whisper-cli.exe");
            if cli.exists() {
                return cli;
            }

            let main = whisper_dir.join("main.exe");
            if main.exists() {
                return main;
            }
        }
    }

    let mut local = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    local.push("QuanVoice");
    local.push("whisper");
    let cli = local.join("whisper-cli.exe");
    if cli.exists() {
        return cli;
    }

    local.join("main.exe")
}

fn extract_transcript_from_stdout(stdout: &str) -> String {
    let mut parts = Vec::new();
    for line in stdout.lines() {
        let text = line.trim();
        if text.is_empty() {
            continue;
        }

        let stripped = if let Some((_, right)) = text.rsplit_once(']') {
            right.trim()
        } else {
            text
        };

        if !stripped.is_empty() && !stripped.starts_with("whisper_") {
            parts.push(stripped.to_string());
        }
    }

    parts.join(" ").trim().to_string()
}

fn file_sha256_hex(path: &Path) -> Result<String, WhisperError> {
    let mut command = std::process::Command::new("certutil");
    command.args(["-hashfile", &path.display().to_string(), "SHA256"]);
    hide_std_command_window(&mut command);

    let output = command
        .output()
        .map_err(|err| WhisperError::DownloadFailed(err.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WhisperError::DownloadFailed(if stderr.is_empty() {
            "failed to calculate model checksum".to_string()
        } else {
            stderr
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_sha256_hex(&stdout).ok_or_else(|| {
        WhisperError::DownloadFailed("failed to parse model checksum output".to_string())
    })
}

fn parse_sha256_hex(input: &str) -> Option<String> {
    let mut run_start: Option<usize> = None;
    let mut run_len: usize = 0;

    for (idx, ch) in input.char_indices() {
        if ch.is_ascii_hexdigit() {
            if run_start.is_none() {
                run_start = Some(idx);
            }
            run_len += 1;
            if run_len == 64 {
                let start = run_start?;
                let end = idx + ch.len_utf8();
                return Some(input[start..end].to_ascii_lowercase());
            }
            continue;
        }

        run_start = None;
        run_len = 0;
    }

    None
}

fn hide_std_command_window(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn hide_async_command_window(command: &mut AsyncCommand) {
    #[cfg(windows)]
    {
        command.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_sha256_hex;

    #[test]
    fn parses_sha256_plain_line() {
        let v = parse_sha256_hex(
            "a52f3d4f5f345c3717fcb07fef7cfeb2a95b7ad7fb4a27d8f9f7f4054ca11f44",
        );
        assert_eq!(
            v.as_deref(),
            Some("a52f3d4f5f345c3717fcb07fef7cfeb2a95b7ad7fb4a27d8f9f7f4054ca11f44")
        );
    }

    #[test]
    fn parses_sha256_with_filename() {
        let v = parse_sha256_hex(
            "a52f3d4f5f345c3717fcb07fef7cfeb2a95b7ad7fb4a27d8f9f7f4054ca11f44  ggml-tiny.en.bin",
        );
        assert_eq!(
            v.as_deref(),
            Some("a52f3d4f5f345c3717fcb07fef7cfeb2a95b7ad7fb4a27d8f9f7f4054ca11f44")
        );
    }

    #[test]
    fn rejects_non_hex_checksum() {
        assert!(parse_sha256_hex("not-a-checksum").is_none());
    }

    #[test]
    fn parses_prefixed_sha256() {
        let v = parse_sha256_hex(
            "oid sha256:a52f3d4f5f345c3717fcb07fef7cfeb2a95b7ad7fb4a27d8f9f7f4054ca11f44",
        );
        assert_eq!(
            v.as_deref(),
            Some("a52f3d4f5f345c3717fcb07fef7cfeb2a95b7ad7fb4a27d8f9f7f4054ca11f44")
        );
    }
}
