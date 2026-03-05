# Quan Voice Release Notes

## v2.0.0 (Online + Offline Hybrid, Hardened Key Flow)

Release date: March 5, 2026

### Highlights
- Added online transcription mode with AssemblyAI integration.
- Kept offline local transcription path as automatic fallback.
- Hardened key handling: app now accepts only company-provided environment key.
- Removed end-user API key input from Settings.
- Added improved in-app notifications for mode switching and status events.

### Security Hardening
- AssemblyAI key source is now environment-only:
  - `QUAN_VOICE_ASSEMBLYAI_API_KEY`
- Local settings key input is removed from the app UI and settings model.
- If env key is missing, online mode auto-falls back to offline mode.

### UX Improvements
- Online/offline mode switch remains in Settings.
- Automatic notification when internet is unavailable and offline mode is activated.
- Toast-style notifications for:
  - settings save
  - activation/deactivation
  - mode transition events
  - transcription status

### Installer
- File: `Quan Voice_0.2.0_x64-setup.exe`
- Platform: Windows x64

### Upgrade Steps
1. Close Quan Voice fully.
2. Install `Quan Voice_0.2.0_x64-setup.exe`.
3. Set company environment key in runtime environment.
4. Launch app and enable Online mode if needed.

### Environment Requirement (Online Mode)
```powershell
$env:QUAN_VOICE_ASSEMBLYAI_API_KEY="<company-key>"
```

### Known Notes
- If internet is unavailable, app will continue dictation via offline mode.
- Online mode requires both internet connectivity and environment key.
