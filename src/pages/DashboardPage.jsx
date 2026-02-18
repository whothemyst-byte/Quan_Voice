import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import StatusIndicator from "../components/StatusIndicator.jsx";
import SettingsPage from "./SettingsPage.jsx";

const fallbackSettings = {
  push_to_talk_key: "RightCtrl",
  auto_punctuation: true,
  model: "tiny.en",
  start_with_windows: false,
  input_device: "Default",
};

/* ── SVG Icons ──────────────────────────────────────────────── */
const initialProgress = {
  downloaded_bytes: 0,
  total_bytes: null,
  progress_percent: 0,
};

function formatBytes(bytes) {
  if (typeof bytes !== "number" || bytes < 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let idx = 0;
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024;
    idx += 1;
  }
  return `${value.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
}

const MicSvg = () => (
  <svg viewBox="0 0 24 24"><path d="M12 14a3 3 0 0 0 3-3V5a3 3 0 0 0-6 0v6a3 3 0 0 0 3 3Zm5-3a5 5 0 0 1-10 0H5a7 7 0 0 0 6 6.93V21h2v-3.07A7 7 0 0 0 19 11h-2Z"/></svg>
);

const DashIcon = () => (
  <svg viewBox="0 0 24 24"><path d="M4 13h6a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H4a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1Zm0 8h6a1 1 0 0 0 1-1v-4a1 1 0 0 0-1-1H4a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1Zm10 0h6a1 1 0 0 0 1-1v-8a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1Zm0-18v4a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1Z" fill="currentColor"/></svg>
);

const SettingsIcon = () => (
  <svg viewBox="0 0 24 24"><path d="M19.14 12.94a7.07 7.07 0 0 0 .06-.94 7.07 7.07 0 0 0-.06-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.04 7.04 0 0 0-1.62-.94l-.36-2.54a.48.48 0 0 0-.48-.41h-3.84a.48.48 0 0 0-.48.41l-.36 2.54a7.04 7.04 0 0 0-1.62.94l-2.39-.96a.49.49 0 0 0-.59.22L2.74 8.87a.48.48 0 0 0 .12.61l2.03 1.58a7.07 7.07 0 0 0-.06.94c0 .32.02.64.06.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32a.49.49 0 0 0 .59.22l2.39-.96c.5.38 1.04.7 1.62.94l.36 2.54c.05.24.26.41.48.41h3.84c.22 0 .43-.17.48-.41l.36-2.54a7.04 7.04 0 0 0 1.62-.94l2.39.96a.49.49 0 0 0 .59-.22l1.92-3.32a.49.49 0 0 0-.12-.61l-2.03-1.58ZM12 15.6A3.6 3.6 0 1 1 12 8.4a3.6 3.6 0 0 1 0 7.2Z" fill="currentColor"/></svg>
);

const CheckSvg = () => (
  <svg viewBox="0 0 24 24"><path d="M9 16.17 4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17Z" fill="currentColor"/></svg>
);

const KeyboardSvg = () => (
  <svg viewBox="0 0 24 24"><path d="M20 5H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2Zm-9 3h2v2h-2V8Zm0 3h2v2h-2v-2ZM8 8h2v2H8V8Zm0 3h2v2H8v-2Zm-1 2H5v-2h2v2Zm0-3H5V8h2v2Zm9 7H8v-2h8v2Zm0-4h-2v-2h2v2Zm0-3h-2V8h2v2Zm3 3h-2v-2h2v2Zm0-3h-2V8h2v2Z" fill="currentColor"/></svg>
);

const LockSvg = () => (
  <svg viewBox="0 0 24 24"><path d="M18 8h-1V6a5 5 0 0 0-10 0v2H6a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V10a2 2 0 0 0-2-2Zm-6 9a2 2 0 1 1 0-4 2 2 0 0 1 0 4Zm3.1-9H8.9V6a3.1 3.1 0 0 1 6.2 0v2Z" fill="currentColor"/></svg>
);

export default function DashboardPage() {
  const [settings, setSettings] = useState(fallbackSettings);
  const [inputDevices, setInputDevices] = useState(["Default"]);
  const [availableModels, setAvailableModels] = useState([
    {
      id: "tiny.en",
      label: "tiny.en (Fastest)",
      approx_download_mb: 75,
      perf_warning: "Fastest and lightest option for most PCs.",
    },
  ]);
  const [persistedModel, setPersistedModel] = useState("tiny.en");
  const [pendingModelConfirm, setPendingModelConfirm] = useState(null);
  const [showDownloadModal, setShowDownloadModal] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(initialProgress);
  const [downloadError, setDownloadError] = useState("");
  const [downloadCompleted, setDownloadCompleted] = useState(false);
  const [saveState, setSaveState] = useState("idle");
  const [status, setStatus] = useState("Inactive");
  const [activated, setActivated] = useState(false);
  const [lastTranscript, setLastTranscript] = useState("");
  const [lastError, setLastError] = useState("");
  const [currentView, setCurrentView] = useState("dashboard");

  useEffect(() => {
    let unlistenStatus;
    let unlistenError;
    let unlistenTranscript;

    async function loadSettings() {
      try {
        const models = await invoke("list_available_models");
        const devices = await invoke("list_input_devices");
        const loaded = await invoke("get_settings");
        let nextSettings = loaded;
        if (Array.isArray(models) && models.length > 0) {
          setAvailableModels(models);
          const selectedModel = loaded.model || "tiny.en";
          const modelExists = models.some((model) => model.id === selectedModel);
          if (!modelExists) {
            nextSettings = { ...nextSettings, model: models[0].id };
          }
        }
        if (Array.isArray(devices) && devices.length > 0) {
          setInputDevices(devices);
          const selected = nextSettings.input_device || "Default";
          if (!devices.includes(selected)) {
            nextSettings = { ...nextSettings, input_device: "Default" };
          }
        }
        if (nextSettings !== loaded) {
          await invoke("update_settings", { settings: nextSettings });
        }
        setSettings(nextSettings);
        setPersistedModel(nextSettings.model || "tiny.en");
      } catch (error) {
        setSaveState(`error:${String(error)}`);
      }
    }

    async function wireEvents() {
      unlistenStatus = await listen("hotkey-status", (event) => {
        setStatus(String(event.payload ?? "Idle"));
      });

      unlistenError = await listen("hotkey-error", (event) => {
        const message = String(event.payload ?? "Unknown hotkey error");
        setSaveState(`error:${message}`);
        setLastError(message);
      });

      unlistenTranscript = await listen("transcription-result", (event) => {
        setLastTranscript(String(event.payload ?? ""));
        setLastError("");
      });

    }

    wireEvents();
    loadSettings();

    return () => {
      if (unlistenStatus) unlistenStatus();
      if (unlistenError) unlistenError();
      if (unlistenTranscript) unlistenTranscript();
    };
  }, []);

  function handleSettingChange(field, value) {
    setSettings((prev) => ({ ...prev, [field]: value }));
    if (saveState !== "idle") setSaveState("idle");
  }

  async function runModelDownload() {
    setDownloadError("");
    setDownloadCompleted(false);
    setDownloadProgress(initialProgress);
    setShowDownloadModal(true);
    let unlistenProgress;

    try {
      unlistenProgress = await listen("model-download-progress", (event) => {
        setDownloadProgress(event.payload ?? initialProgress);
      });
      await invoke("ensure_model_ready");
      setDownloadCompleted(true);
      await new Promise((resolve) => setTimeout(resolve, 1200));
      setShowDownloadModal(false);
    } catch (error) {
      setDownloadError(String(error));
      throw error;
    } finally {
      if (unlistenProgress) {
        unlistenProgress();
      }
    }
  }

  async function persistSettings({ forceModelConfirm = false } = {}) {
    try {
      const modelChanged = settings.model !== persistedModel;
      const selectedModel = availableModels.find((item) => item.id === settings.model);
      if (modelChanged && selectedModel && selectedModel.id !== "tiny.en" && !forceModelConfirm) {
        setPendingModelConfirm(selectedModel);
        return;
      }

      setSaveState("saving");
      await invoke("update_settings", { settings });
      if (modelChanged) {
        await runModelDownload();
      }
      setPersistedModel(settings.model);
      setSaveState("saved");
      if (activated) {
        await invoke("register_hotkey");
        setStatus("Ready");
      }
    } catch (error) {
      setSaveState(`error:${String(error)}`);
    }
  }

  function handleSaveSettings() {
    void persistSettings();
  }

  function handleConfirmModelSwitch() {
    setPendingModelConfirm(null);
    void persistSettings({ forceModelConfirm: true });
  }

  function handleCancelModelSwitch() {
    setPendingModelConfirm(null);
  }

  async function handleToggleActivation() {
    try {
      if (!activated) {
        await invoke("update_settings", { settings });
        await invoke("register_hotkey");
        await invoke("set_live_ready");
        await invoke("show_floating_widget");
        setActivated(true);
        setStatus("Ready");
        setSaveState("saved");
        setLastError("");
        return;
      }

      await invoke("unregister_hotkey");
      await invoke("hide_floating_widget");
      setActivated(false);
      setStatus("Inactive");
    } catch (error) {
      setSaveState(`error:${String(error)}`);
    }
  }

  const isListening = status === "Listening";
  const statusLabel = isListening ? "Listening…" : activated ? "Ready to Listen" : "Inactive";
  const downloadPercent = Math.max(0, Math.min(100, downloadProgress?.progress_percent ?? 0));
  const downloaded = formatBytes(downloadProgress?.downloaded_bytes ?? 0);
  const total =
    downloadProgress?.total_bytes == null
      ? "unknown size"
      : formatBytes(downloadProgress.total_bytes);

  return (
    <div className="app-shell">
      {/* ── Sidebar ──────────────────────────────── */}
      <aside className="sidebar">
        <div className="sidebar-brand">
          <div className="sidebar-logo"><MicSvg /></div>
          <span className="sidebar-brand-text">Quan Voice</span>
        </div>

        <nav className="sidebar-nav">
          <button
            className={`nav-item${currentView === "dashboard" ? " active" : ""}`}
            onClick={() => setCurrentView("dashboard")}
          >
            <DashIcon />
            Dashboard
          </button>
          <button
            className={`nav-item${currentView === "settings" ? " active" : ""}`}
            onClick={() => setCurrentView("settings")}
          >
            <SettingsIcon />
            Settings
          </button>
        </nav>

        <div className="sidebar-footer">
          <div className="plan-badge">
            <span className="plan-badge-label">Free Plan</span>
            <span className="plan-badge-upgrade">Upgrade</span>
          </div>
        </div>
      </aside>

      {/* ── Main Content ─────────────────────────── */}
      <main className="main-content">
        <div className="traffic-lights">
          <span className="traffic-dot red" />
          <span className="traffic-dot yellow" />
          <span className="traffic-dot green" />
        </div>

        {currentView === "dashboard" ? (
          <>
            {/* Status Card */}
            <section className="status-card">
              <div className={`mic-ring${isListening ? " listening" : ""}`}>
                <MicSvg />
              </div>
              <h2 className="status-title">{statusLabel}</h2>
              <p className="status-instruction">
                Hold <kbd>{settings.push_to_talk_key}</kbd> to start voice typing
              </p>
              <button
                className={`activate-btn ${activated ? "stop" : "start"}`}
                type="button"
                onClick={handleToggleActivation}
              >
                {activated ? "Deactivate Quan Voice" : "Activate Quan Voice"}
              </button>
            </section>

            {/* Stat Cards */}
            <div className="stat-cards">
              <div className="stat-card">
                <div className="stat-icon model"><CheckSvg /></div>
                <div className="stat-info">
                  <span className="stat-label">Model</span>
                  <span className="stat-value">{settings.model}</span>
                </div>
              </div>
              <div className="stat-card">
                <div className="stat-icon hotkey"><KeyboardSvg /></div>
                <div className="stat-info">
                  <span className="stat-label">Hotkey</span>
                  <span className="stat-value">{settings.push_to_talk_key}</span>
                </div>
              </div>
              <div className="stat-card">
                <div className="stat-icon privacy"><LockSvg /></div>
                <div className="stat-info">
                  <span className="stat-label">Status</span>
                  <span className="stat-value">Offline · Private</span>
                </div>
              </div>
            </div>

            {/* Recent Transcriptions */}
            <section className="transcriptions-section">
              <h3 className="section-header">Recent Transcriptions</h3>
              {lastTranscript ? (
                <div className="transcript-item">
                  <span className="transcript-dot" />
                  <div className="transcript-content">
                    <p className="transcript-text">{lastTranscript}</p>
                    <p className="transcript-meta">Transcribed from voice input</p>
                  </div>
                </div>
              ) : (
                <p className="transcript-empty">No transcriptions yet. Activate and hold your hotkey to speak.</p>
              )}
            </section>

            {/* Error */}
            {lastError && <div className="error-banner">Error: {lastError}</div>}

            <StatusIndicator state={status} />
          </>
        ) : (
          /* Settings View */
          <div className="settings-panel">
            <SettingsPage
              settings={settings}
              inputDevices={inputDevices}
              availableModels={availableModels}
              onChange={handleSettingChange}
              onSave={handleSaveSettings}
              saveState={saveState}
            />
          </div>
        )}
      </main>

      {pendingModelConfirm ? (
        <div className="modal-overlay" role="dialog" aria-modal="true">
          <div className="modal-card">
            <h3>Switch Model?</h3>
            <p>
              {pendingModelConfirm.label} will be downloaded (~{pendingModelConfirm.approx_download_mb} MB).
            </p>
            <p className="modal-note">{pendingModelConfirm.perf_warning}</p>
            <div className="modal-actions">
              <button className="btn-secondary" type="button" onClick={handleCancelModelSwitch}>
                Cancel
              </button>
              <button className="primary-btn" type="button" onClick={handleConfirmModelSwitch}>
                Continue
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {showDownloadModal ? (
        <div className="modal-overlay" role="dialog" aria-modal="true">
          <div className="modal-card">
            <h3>{downloadCompleted ? "Download Complete" : "Downloading Model..."}</h3>
            <p>
              {downloadCompleted
                ? "Selected model is ready to use."
                : "Preparing selected speech model."}
            </p>
            {downloadCompleted ? (
              <p className="success-text">Model downloaded successfully.</p>
            ) : (
              <>
                <div className="progress-track" aria-label="Model download progress">
                  <div className="progress-fill" style={{ width: `${downloadPercent}%` }} />
                </div>
                <p className="progress-meta">
                  {downloadPercent.toFixed(1)}% ({downloaded} / {total})
                </p>
              </>
            )}
            {downloadError ? (
              <>
                <div className="error-banner">{downloadError}</div>
                <div className="modal-actions">
                  <button className="btn-secondary" type="button" onClick={() => setShowDownloadModal(false)}>
                    Close
                  </button>
                </div>
              </>
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}
