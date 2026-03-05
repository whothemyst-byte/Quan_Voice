# AssemblyAI Online Mode Implementation (Phase 1)

## What is implemented

- Added settings fields:
  - `online_mode` (boolean toggle)
  - `assemblyai_api_key` (string)
- Added Settings UI controls:
  - Online mode checkbox
- Added runtime backend selection in Rust:
  - If `online_mode=false` -> offline Whisper path
  - If `online_mode=true` and internet + `QUAN_VOICE_ASSEMBLYAI_API_KEY` is set -> AssemblyAI path
  - If `online_mode=true` but internet is unavailable OR env key is missing -> automatic fallback to offline mode
- Added user notification event for fallback:
  - Message: `Internet is off or company key QUAN_VOICE_ASSEMBLYAI_API_KEY is not configured. Switched to offline mode.`
- Added dashboard startup/offline behavior:
  - On app load, if online mode is enabled but AssemblyAI is unreachable, settings are auto-switched to offline and a notification banner is shown.
- Added backend connectivity command:
  - `check_assemblyai_connectivity`
- Added AssemblyAI module:
  - Upload WAV -> create transcript -> poll completion -> return text
  - File: `src-tauri/src/assemblyai.rs`

## Important note

This implementation uses AssemblyAI cloud transcription endpoint with polling. It is the first working online path and fallback framework.
The true low-latency websocket streaming path (partial interim tokens while speaking) should be implemented next as a dedicated Phase 2 to achieve "instant" behavior.

## Files changed in this phase

- `src-tauri/src/settings.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/assemblyai.rs` (new)
- `src-tauri/Cargo.toml`
- `src/pages/SettingsPage.jsx`
- `src/pages/DashboardPage.jsx`

## Manual test commands

1. Run backend + UI:

```powershell
npm.cmd run tauri:dev
```

2. Optional Rust tests:

```powershell
npm.cmd run test
```

## Manual test checklist

1. Open Settings.
2. Enable `Online mode (AssemblyAI)` and Save.
3. Disconnect internet.
4. Restart app.
5. Confirm:
   - Online mode is auto-switched to offline.
   - Notification banner appears indicating internet is off and offline mode is active.
6. Reconnect internet.
7. Set environment variable `QUAN_VOICE_ASSEMBLYAI_API_KEY`.
8. Enable online mode and Save.
9. Activate Quan Voice and test dictation.
10. Unset the env key and test again:
   - It should fallback to offline and show notification.
