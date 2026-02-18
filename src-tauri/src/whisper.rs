use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use futures_util::StreamExt;
use serde::Serialize;
use thiserror::Error;
use tokio::process::Command as AsyncCommand;

const TINY_EN_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin";
const TINY_EN_MODEL_SHA256_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin.sha256";
const MIN_MODEL_BYTES: u64 = 30 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ModelDownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress_percent: f64,
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
}

pub struct WhisperEngine {
    model_path: PathBuf,
    binary_path: PathBuf,
}

impl WhisperEngine {
    pub fn new() -> Self {
        let mut root = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        root.push("QuanVoice");
        root.push("models");
        let model_path = root.join("ggml-tiny.en.bin");

        let binary_path = resolve_whisper_binary_path();
        Self {
            model_path,
            binary_path,
        }
    }

    pub async fn ensure_model_exists<F>(&mut self, mut on_progress: F) -> Result<bool, WhisperError>
    where
        F: FnMut(ModelDownloadProgress) -> Result<(), WhisperError>,
    {
        let expected_sha256 = self.fetch_expected_sha256().await?;

        if self.model_is_valid(&expected_sha256)? {
            on_progress(ModelDownloadProgress {
                downloaded_bytes: 1,
                total_bytes: Some(1),
                progress_percent: 100.0,
            })?;
            return Ok(true);
        }

        if let Some(parent) = self.model_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let partial = self.partial_model_path();
        if partial.exists() {
            let _ = fs::remove_file(&partial);
        }

        self.download_model(&expected_sha256, &mut on_progress).await?;
        Ok(true)
    }

    pub async fn transcribe(&self, _wav_path: &std::path::Path) -> Result<String, WhisperError> {
        if !self.model_path.exists() {
            return Err(WhisperError::TranscriptionFailed(
                "missing whisper model".to_string(),
            ));
        }

        if !self.binary_path.exists() {
            return Err(WhisperError::MissingBinary(
                self.binary_path.display().to_string(),
            ));
        }

        self.transcribe_with_binary(_wav_path).await
    }

    async fn download_model<F>(
        &self,
        expected_sha256: &str,
        on_progress: &mut F,
    ) -> Result<(), WhisperError>
    where
        F: FnMut(ModelDownloadProgress) -> Result<(), WhisperError>,
    {
        let response = reqwest::get(TINY_EN_MODEL_URL)
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
        let partial = self.partial_model_path();
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
        if fs::metadata(&partial)?.len() < MIN_MODEL_BYTES {
            let _ = fs::remove_file(&partial);
            return Err(WhisperError::DownloadFailed(
                "downloaded model appears corrupted (file too small)".to_string(),
            ));
        }
        let actual_sha256 = file_sha256_hex(&partial)?;
        if actual_sha256 != expected_sha256 {
            let _ = fs::remove_file(&partial);
            return Err(WhisperError::DownloadFailed(format!(
                "model checksum mismatch: expected {expected_sha256}, got {actual_sha256}"
            )));
        }

        fs::rename(&partial, &self.model_path)?;
        on_progress(ModelDownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes: total.or(Some(downloaded)),
            progress_percent: 100.0,
        })?;
        Ok(())
    }

    async fn transcribe_with_binary(&self, wav_path: &Path) -> Result<String, WhisperError> {
        let output_prefix = wav_path.with_file_name("transcript");
        let output_txt = output_prefix.with_extension("txt");
        let _ = fs::remove_file(&output_txt);

        let output = AsyncCommand::new(&self.binary_path)
            .arg("-m")
            .arg(&self.model_path)
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
            .arg(&output_prefix)
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
    fn partial_model_path(&self) -> PathBuf {
        self.model_path.with_extension("bin.part")
    }

    fn model_is_valid(&self, expected_sha256: &str) -> Result<bool, WhisperError> {
        if !self.model_path.exists() {
            return Ok(false);
        }

        let size = fs::metadata(&self.model_path)?.len();
        if size < MIN_MODEL_BYTES {
            return Ok(false);
        }

        let actual_sha256 = file_sha256_hex(&self.model_path)?;
        Ok(actual_sha256 == expected_sha256)
    }

    async fn fetch_expected_sha256(&self) -> Result<String, WhisperError> {
        let body = reqwest::get(TINY_EN_MODEL_SHA256_URL)
            .await
            .map_err(|err| WhisperError::DownloadFailed(err.to_string()))?
            .text()
            .await
            .map_err(|err| WhisperError::DownloadFailed(err.to_string()))?;

        parse_sha256_hex(&body).ok_or_else(|| {
            WhisperError::DownloadFailed("invalid checksum response format".to_string())
        })
    }
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
    let output = std::process::Command::new("certutil")
        .args(["-hashfile", &path.display().to_string(), "SHA256"])
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
    let token = input
        .split_whitespace()
        .find(|part| part.len() == 64 && part.chars().all(|ch| ch.is_ascii_hexdigit()))?;
    Some(token.to_ascii_lowercase())
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
}
