# Quan Voice

Offline voice typing for Windows, built with Tauri + Rust + whisper.cpp.

## Overview
Quan Voice captures microphone audio with push-to-talk, transcribes locally, and injects text into the active application.

Core goals:
- Offline-first speech workflow
- Low-friction push-to-talk dictation
- Cross-app text injection (editor, browser, notes, terminal)
- Lightweight floating widget for live status

## Features
- Push-to-talk hotkey activation/deactivation
- Local speech model download and integrity checks
- Real-time input level and status indicator
- Floating microphone widget (always-on-top, draggable)
- Input device selection
- Settings persistence with corrupt-config recovery
- Optional start with Windows toggle

## Requirements
- Windows x64
- Microphone input device
- Internet access on first run to download the speech model

## Installation (End Users)
1. Download the installer from release assets:
   - `Quan Voice_0.2.0_x64-setup.exe`
2. Run the installer.
3. If SmartScreen appears, click `More info` -> `Run anyway`.
4. Launch Quan Voice and complete first-time model setup.

## Quick Start
1. Open `Settings` and choose your input device.
2. Set your preferred push-to-talk key.
3. Click `Activate Quan Voice`.
4. Hold the hotkey, speak, then release to inject text in the focused app.
5. Drag the floating widget to position it where you want.

## Project Structure
- `src/`: React frontend (dashboard, settings, floating widget)
- `src-tauri/`: Rust backend (audio, hotkey, whisper integration, injector)
- `docs/`: release notes and QA documents
- `release/`: tracked installer artifact for distribution

## Development
### Prerequisites
- Node.js + npm
- Rust toolchain
- Tauri prerequisites for Windows

### Commands
- `npm run dev`: run frontend in dev mode
- `npm run tauri:dev`: run desktop app in dev mode
- `npm run build`: production frontend build
- `npm run tauri:build`: production installer build
- `npm test`: run Rust unit tests via npm script
- `cargo test --manifest-path src-tauri/Cargo.toml`: run backend tests directly

## Troubleshooting
- Model download problems:
  - Retry from the app.
  - Ensure network access to Hugging Face model files.
- No mic input/transcription:
  - Confirm correct `Input device` in Settings.
  - Check Windows microphone permissions.
- Widget not visible:
  - Activate Quan Voice from dashboard.
  - Re-toggle activation if needed.

## Privacy
- Speech audio and transcription are handled locally in the desktop app runtime.
- No cloud transcription service is required for normal dictation flow.

## License
Proprietary (see `src-tauri/Cargo.toml`).
