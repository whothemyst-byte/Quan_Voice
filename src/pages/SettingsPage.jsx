export default function SettingsPage({
  settings,
  inputDevices,
  availableModels,
  onChange,
  onSave,
  saveState,
}) {
  const isSaving = saveState === "saving";
  const selectedModel =
    (availableModels || []).find((model) => model.id === settings.model) || null;

  function normalizeHotkey(event) {
    const { code, key } = event;
    if (code === "ControlLeft") return "Ctrl";
    if (code === "ControlRight") return "RightCtrl";
    if (code === "ShiftLeft") return "Shift";
    if (code === "ShiftRight") return "RightShift";
    if (code === "AltLeft") return "Alt";
    if (code === "AltRight") return "RightAlt";
    if (code === "Tab") return "Tab";
    if (code === "Space") return "Space";
    if (code === "Enter" || code === "NumpadEnter") return "Enter";
    if (code === "Escape") return "Escape";
    if (code === "CapsLock") return "CapsLock";
    if (code === "Backspace") return "Backspace";
    if (code === "Delete") return "Delete";
    if (code === "Insert") return "Insert";
    if (code === "Home") return "Home";
    if (code === "End") return "End";
    if (code === "PageUp") return "PageUp";
    if (code === "PageDown") return "PageDown";
    if (code === "ArrowUp") return "Up";
    if (code === "ArrowDown") return "Down";
    if (code === "ArrowLeft") return "Left";
    if (code === "ArrowRight") return "Right";
    if (/^F([1-9]|1[0-2])$/.test(code)) return code;
    if (/^Key[A-Z]$/.test(code)) return code.replace("Key", "");
    if (/^Digit[0-9]$/.test(code)) return code.replace("Digit", "");
    if (/^Numpad[0-9]$/.test(code)) return code.replace("Numpad", "");
    if (key && key.length === 1) return key.toUpperCase();
    return "";
  }

  function handleHotkeyCapture(event) {
    event.preventDefault();
    const hotkey = normalizeHotkey(event);
    if (hotkey) {
      onChange("push_to_talk_key", hotkey);
    }
  }

  return (
    <section>
      <h2 style={{ fontSize: 18, fontWeight: 700, marginBottom: 24, letterSpacing: '-0.2px' }}>Settings</h2>
      <div className="settings-grid">
        <label className="field">
          <span>Push-to-talk key</span>
          <input
            type="text"
            value={settings.push_to_talk_key}
            readOnly
            onKeyDown={handleHotkeyCapture}
            placeholder="Click here and press a key"
          />
          <small className="field-note">Click this field, then press a key to assign.</small>
        </label>

        <label className="field checkbox">
          <input
            type="checkbox"
            checked={settings.auto_punctuation}
            onChange={(event) => onChange("auto_punctuation", event.target.checked)}
          />
          <span>Auto punctuation</span>
        </label>

        <label className="field">
          <span>Model</span>
          <select
            value={settings.model}
            onChange={(event) => onChange("model", event.target.value)}
          >
            {(availableModels || []).map((model) => (
              <option key={model.id} value={model.id}>
                {model.label}
              </option>
            ))}
          </select>
          {selectedModel ? (
            <small className="field-note">
              Download: ~{selectedModel.approx_download_mb} MB. {selectedModel.perf_warning}
            </small>
          ) : null}
        </label>

        <label className="field">
          <span>Input device</span>
          <select
            value={settings.input_device || "Default"}
            onChange={(event) => onChange("input_device", event.target.value)}
          >
            {(inputDevices || ["Default"]).map((device) => (
              <option key={device} value={device}>
                {device}
              </option>
            ))}
          </select>
        </label>

        <label className="field checkbox">
          <input
            type="checkbox"
            checked={settings.start_with_windows}
            onChange={(event) => onChange("start_with_windows", event.target.checked)}
          />
          <span>Start with Windows</span>
        </label>
      </div>
      <button className="primary-btn" type="button" onClick={onSave} disabled={isSaving}>
        {isSaving ? "Saving…" : "Save Settings"}
      </button>
      {saveState === "saved" ? <p className="success-text">Settings saved.</p> : null}
      {saveState.startsWith("error:") ? (
        <p className="error-text">{saveState.slice(6)}</p>
      ) : null}
    </section>
  );
}
