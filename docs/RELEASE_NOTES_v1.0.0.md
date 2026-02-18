# Quan Voice Release Notes

## v1.0.0 (Production)

Release date: February 18, 2026

### Highlights
- Offline voice typing workflow for Windows desktop
- Push-to-talk activation with cross-app text injection
- Floating widget with live status and drag support
- Local model bootstrap flow with checksum-aware validation path
- Settings persistence and startup toggle support

### Included in this release
- Desktop app build and NSIS installer
- Main dashboard and settings experience
- Input device selection and saved preferences
- Model setup/download progress UI
- Hotkey registration/unregistration flow

### Stability and UX fixes shipped
- Removed production debug transcript telemetry line from dashboard UI
- Removed terminal flashing by running helper subprocesses in hidden mode
- Added native floating-window drag command and wired widget drag handling
- Hardened checksum parsing to support more upstream response formats

### Security and reliability notes
- CSP enabled for desktop webview
- Model file validity checks include size gate and checksum path (when available)
- Corrupt settings file recovery with fallback defaults

### Installer
- File: `Quan Voice_0.1.0_x64-setup.exe`
- Platform: Windows x64

### Upgrade steps
1. Close Quan Voice completely.
2. Run the latest installer over the existing install.
3. Launch app and verify hotkey, input device, and widget behavior.

### Known limitations
- First-run model setup requires internet access.
- Current automated coverage is backend-focused; UI E2E automation is not yet included.

### Support
When reporting issues, include:
- Windows version
- Microphone device name
- Reproduction steps
- Screenshot or short screen recording (if applicable)
