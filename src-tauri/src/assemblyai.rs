use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CloudDecodeResult {
    pub text: String,
    pub first_token_ts: Option<u128>,
    pub final_text_ts: u128,
}

#[derive(Debug, Error)]
pub enum AssemblyAiError {
    #[error("missing AssemblyAI API key")]
    MissingApiKey,
    #[error("upload failed: {0}")]
    UploadFailed(String),
    #[error("request failed: {0}")]
    RequestFailed(String),
    #[error("transcript failed: {0}")]
    TranscriptFailed(String),
    #[error("transcript timed out")]
    Timeout,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct PollResponse {
    status: String,
    text: Option<String>,
    error: Option<String>,
}

pub async fn transcribe_wav(
    wav_path: &std::path::Path,
    api_key: &str,
) -> Result<CloudDecodeResult, AssemblyAiError> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err(AssemblyAiError::MissingApiKey);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| AssemblyAiError::RequestFailed(err.to_string()))?;

    let bytes = tokio::fs::read(wav_path)
        .await
        .map_err(|err| AssemblyAiError::UploadFailed(err.to_string()))?;

    let upload = client
        .post("https://api.assemblyai.com/v2/upload")
        .header("Authorization", key)
        .header("content-type", "application/octet-stream")
        .body(bytes)
        .send()
        .await
        .map_err(|err| AssemblyAiError::UploadFailed(err.to_string()))?;

    if !upload.status().is_success() {
        let status = upload.status();
        let body = upload.text().await.unwrap_or_default();
        return Err(AssemblyAiError::UploadFailed(format!(
            "status {} body {}",
            status,
            body.trim()
        )));
    }

    let upload_payload: UploadResponse = upload
        .json()
        .await
        .map_err(|err| AssemblyAiError::UploadFailed(err.to_string()))?;

    let create = client
        .post("https://api.assemblyai.com/v2/transcript")
        .header("Authorization", key)
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "audio_url": upload_payload.upload_url,
            "language_code": "en_us",
            "speech_models": ["universal-2"]
        }))
        .send()
        .await
        .map_err(|err| AssemblyAiError::RequestFailed(err.to_string()))?;

    if !create.status().is_success() {
        let status = create.status();
        let body = create.text().await.unwrap_or_default();
        return Err(AssemblyAiError::RequestFailed(format!(
            "status {} body {}",
            status,
            body.trim()
        )));
    }

    let create_payload: TranscriptResponse = create
        .json()
        .await
        .map_err(|err| AssemblyAiError::RequestFailed(err.to_string()))?;

    let started = epoch_ms();
    let mut first_token_ts = None;

    loop {
        if epoch_ms().saturating_sub(started) > 45_000 {
            return Err(AssemblyAiError::Timeout);
        }

        let poll = client
            .get(format!(
                "https://api.assemblyai.com/v2/transcript/{}",
                create_payload.id
            ))
            .header("Authorization", key)
            .send()
            .await
            .map_err(|err| AssemblyAiError::RequestFailed(err.to_string()))?;

        if !poll.status().is_success() {
            let status = poll.status();
            let body = poll.text().await.unwrap_or_default();
            return Err(AssemblyAiError::RequestFailed(format!(
                "status {} body {}",
                status,
                body.trim()
            )));
        }

        let payload: PollResponse = poll
            .json()
            .await
            .map_err(|err| AssemblyAiError::RequestFailed(err.to_string()))?;

        match payload.status.as_str() {
            "queued" => {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            "processing" => {
                if first_token_ts.is_none() {
                    first_token_ts = Some(epoch_ms());
                }
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            "completed" => {
                let now = epoch_ms();
                return Ok(CloudDecodeResult {
                    text: payload.text.unwrap_or_default(),
                    first_token_ts: first_token_ts.or(Some(now)),
                    final_text_ts: now,
                });
            }
            "error" => {
                return Err(AssemblyAiError::TranscriptFailed(
                    payload.error.unwrap_or_else(|| "unknown error".to_string()),
                ));
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        }
    }
}

pub async fn can_reach_assemblyai() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    client
        .get("https://api.assemblyai.com")
        .send()
        .await
        .map(|response| response.status().is_success() || response.status().is_client_error())
        .unwrap_or(false)
}

fn epoch_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
