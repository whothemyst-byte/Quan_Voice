use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug, Clone, Serialize, Default)]
pub struct AsrWorkerSnapshot {
    pub active_session: Option<u64>,
    pub queued_messages: usize,
    pub dropped_partial_jobs: u64,
}

#[allow(dead_code)]
pub enum AsrWorkerCommand {
    StartSession {
        session_id: u64,
        model_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    PushAudioChunk {
        session_id: u64,
        pcm_16k: Vec<f32>,
    },
    PollPartial {
        session_id: u64,
        reply: oneshot::Sender<Option<String>>,
    },
    Finalize {
        session_id: u64,
        reply: oneshot::Sender<Result<Option<String>, String>>,
    },
    Cancel {
        session_id: u64,
    },
    Shutdown,
}

struct AsrWorkerState {
    active_session: Option<u64>,
    dropped_partial_jobs: u64,
}

#[derive(Clone)]
pub struct AsrWorkerHandle {
    tx: mpsc::Sender<AsrWorkerCommand>,
    state: Arc<Mutex<AsrWorkerState>>,
}

impl AsrWorkerHandle {
    pub fn spawn(queue_capacity: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<AsrWorkerCommand>(queue_capacity);
        let state = Arc::new(Mutex::new(AsrWorkerState {
            active_session: None,
            dropped_partial_jobs: 0,
        }));
        let state_for_task = Arc::clone(&state);

        tauri::async_runtime::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    AsrWorkerCommand::StartSession {
                        session_id,
                        model_id: _,
                        reply,
                    } => {
                        let mut guard = state_for_task.lock().await;
                        guard.active_session = Some(session_id);
                        let _ = reply.send(Ok(()));
                    }
                    AsrWorkerCommand::PushAudioChunk { .. } => {
                        // Phase A skeleton: queue is in place, decode loop arrives in Phase B.
                    }
                    AsrWorkerCommand::PollPartial {
                        session_id: _,
                        reply,
                    } => {
                        let _ = reply.send(None);
                    }
                    AsrWorkerCommand::Finalize {
                        session_id: _,
                        reply,
                    } => {
                        let _ = reply.send(Ok(None));
                    }
                    AsrWorkerCommand::Cancel { session_id } => {
                        let mut guard = state_for_task.lock().await;
                        if guard.active_session == Some(session_id) {
                            guard.active_session = None;
                        }
                    }
                    AsrWorkerCommand::Shutdown => break,
                }
            }
        });

        Self { tx, state }
    }

    pub async fn snapshot(&self) -> AsrWorkerSnapshot {
        let guard = self.state.lock().await;
        AsrWorkerSnapshot {
            active_session: guard.active_session,
            queued_messages: self.tx.max_capacity().saturating_sub(self.tx.capacity()),
            dropped_partial_jobs: guard.dropped_partial_jobs,
        }
    }

    #[allow(dead_code)]
    pub async fn start_session(&self, session_id: u64, model_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(AsrWorkerCommand::StartSession {
                session_id,
                model_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "worker unavailable".to_string())?;
        reply_rx
            .await
            .map_err(|_| "worker reply canceled".to_string())?
    }

    #[allow(dead_code)]
    pub async fn cancel(&self, session_id: u64) -> Result<(), String> {
        self.tx
            .send(AsrWorkerCommand::Cancel { session_id })
            .await
            .map_err(|_| "worker unavailable".to_string())
    }
}
