\# Quan Voice – V1



\## Product Overview



Quan Voice is a Windows-only, privacy-first, offline voice typing software built for professionals and developers.



Core Promise:

\- Works anywhere typing is possible

\- Fully offline speech-to-text

\- Push-to-talk mechanism

\- Fast, lightweight, and accurate

\- Clean professional UI

\- Freemium product model



This is NOT a demo.

This is a production-ready V1 to be shipped.



---



\# Core Objectives



1\. Press and hold a global hotkey.

2\. Record microphone audio while held.

3\. On release:

&nbsp;  - Stop recording.

&nbsp;  - Transcribe audio locally using whisper.cpp.

&nbsp;  - Inject the transcribed text at the active cursor location using Windows SendInput API.

4\. Must work in:

&nbsp;  - VS Code

&nbsp;  - Terminals

&nbsp;  - Browsers

&nbsp;  - Word

&nbsp;  - Discord

&nbsp;  - Any input field



No application-specific integration.



---



\# Platform



\- OS: Windows 10/11

\- Architecture: x64

\- Internet: Not required after model download



---



\# Technology Stack



Frontend:

\- Tauri

\- HTML/CSS/JavaScript (clean professional UI)



Backend:

\- Rust (Tauri backend)

\- whisper.cpp (local speech engine)

\- Windows WinAPI (global hotkey + SendInput)



---



\# Architecture Overview



Frontend (UI Layer)

&nbsp;   ↓

Tauri Command Bridge

&nbsp;   ↓

Rust Core Engine

&nbsp;   ↓

whisper.cpp (via binary execution or FFI)

&nbsp;   ↓

Windows API (Global Hotkey + SendInput)



---



\# Core Functional Modules



\## 1. Global Hotkey Manager

\- Register system-wide hotkey (default: Right Ctrl)

\- Detect key press and release

\- Must work even if app window not focused



\## 2. Audio Recorder

\- Start recording on hotkey press

\- Stop on hotkey release

\- Save temporary WAV file

\- Use low-latency format (16kHz mono)



\## 3. Whisper Engine

\- Use whisper.cpp binary

\- Model loaded from:

&nbsp; %LOCALAPPDATA%/QuanVoice/models/

\- On first launch:

&nbsp; - Download tiny model automatically

&nbsp; - Show progress bar in UI

\- Run transcription command

\- Capture stdout result



\## 4. Text Processor

\- Trim whitespace

\- Remove extra line breaks

\- Basic punctuation correction (if enabled)

\- Return clean string



\## 5. Text Injection Engine

\- Use Windows SendInput API

\- Simulate real keyboard typing

\- Inject at active cursor position

\- Must not require window focus



\## 6. Settings Manager

Store in:

%APPDATA%/QuanVoice/config.json



Settings:

\- Push-to-talk key

\- Auto punctuation toggle

\- Model selection (Free = tiny only)

\- Start with Windows toggle



---



\# Freemium Structure



Free:

\- tiny model only

\- default push-to-talk

\- basic transcription



Pro (future flag system):

\- base/small models

\- smart punctuation

\- custom hotkeys

\- startup with Windows

\- developer formatting mode



Do NOT implement payment logic in V1.

Structure code to allow feature flags.



---



\# File Structure



/quan-voice

│

├── src-tauri/

│   ├── src/

│   │   ├── main.rs

│   │   ├── hotkey.rs

│   │   ├── audio.rs

│   │   ├── whisper.rs

│   │   ├── inject.rs

│   │   ├── settings.rs

│   │

│   └── tauri.conf.json

│

├── src/

│   ├── App.jsx

│   ├── components/

│   ├── pages/

│   ├── styles/

│

├── public/

├── CODEX.md

└── README.md



---



\# UI Requirements



Style:

\- Clean professional productivity software

\- Neutral colors

\- Minimal animations

\- System tray integration

\- Small floating status indicator (Listening / Idle)



Main Views:

\- Dashboard

\- Settings

\- Model download screen (first launch)



---



\# Model Download Behavior



On first launch:

1\. Check if model exists.

2\. If not:

&nbsp;  - Show modal: "Downloading Speech Model..."

&nbsp;  - Download tiny.en model from official source.

&nbsp;  - Save to:

&nbsp;    %LOCALAPPDATA%/QuanVoice/models/

3\. Continue to dashboard after download.



---



\# Performance Constraints



\- Must run smoothly on 8GB RAM

\- Must not exceed 300MB memory usage

\- Must not block UI thread

\- Use async where necessary



---



\# Security \& Privacy



\- No audio leaves device

\- No telemetry in V1

\- No external APIs

\- All processing local



Privacy-first positioning is mandatory.



---



\# V1 Scope Limitations



Do NOT implement:

\- Streaming transcription

\- Multi-language support

\- Payment gateway

\- Cloud sync

\- AI command parsing

\- Plugin system



Keep V1 minimal and stable.



---



\# Definition of Done



Quan Voice V1 is complete when:



1\. App launches successfully.

2\. Model auto-download works.

3\. Push-to-talk records audio.

4\. Transcription runs locally.

5\. Text injects wherever cursor is active.

6\. Settings persist correctly.

7\. Installer builds successfully.

8\. App runs without crashing.



Ship only when stable.



---



\# Branding



Product Name: Quan Voice

Tagline: "Offline Voice. Unlimited Focus."



Positioning:

Professional, privacy-first productivity tool.



---



\# Final Instruction to Coding Model



Implement clean, modular, production-grade code.

Avoid hacks.

Keep system-level logic in Rust.

UI should remain minimal and clean.



