# Quan Voice V1 - Step 1 QA Checklist (End-to-End)

Date: 2026-02-17
Scope: Hotkey hold/release, recording, local transcription, injection, stability.

## Automated Checks (Completed)

- [x] Rust compile check: `cargo check` in `src-tauri/`
- [x] Frontend build check: `npm.cmd run build`
- [x] Whisper model presence + binary wiring validated
- [x] Debug telemetry available in app UI:
  - Device
  - RMS
  - Raw transcript
  - Cleaned transcript

## Manual End-to-End Matrix (Run In App)

Preconditions:
- App launched via `npm.cmd run tauri:dev`
- Settings saved
- Correct microphone selected in `Input device`
- App activated

### Core Flow

1. Hold push-to-talk hotkey, speak 2-3 seconds, release
- Expected:
  - Status: `Ready -> Listening -> Idle`
  - Floating wave animates while speaking
  - `Last Transcription` shows text
  - Text appears in focused target input
- Result: [x] Pass [ ] Fail
- Notes:

2. Quick tap (no speech)
- Expected:
  - No crash
  - No garbage injection
- Result: [x] Pass [ ] Fail
- Notes:

3. Long hold (10-20 seconds speech)
- Expected:
  - No freeze
  - Reasonable transcription
  - Injected text appears after release
- Result: [x] Pass [ ] Fail
- Notes: It is cleaning up the text, it is nice but it should change the content that is user trying to say.

### Target App Validation

1. Notepad
- Result: [x] Pass [ ] Fail
- Notes:

2. VS Code editor
- Result: [x] Pass [ ] Fail
- Notes:

3. Browser text area (e.g. web form)
- Result: [x] Pass [ ] Fail
- Notes:

4. Terminal prompt input
- Result: [x] Pass [ ] Fail
- Notes:

5. Word / Discord (at least one)
- Result: [x] Pass [ ] Fail
- Notes:

### Edge Cases

1. Hotkey changed in settings then re-activated
- Expected: new key works immediately
- Result: [x] Pass [ ] Fail
- Notes:

2. Switch apps while floating widget visible
- Expected: widget stays visible, hotkey still works
- Result: [x] Pass [ ] Fail
- Notes:

3. Deactivate then reactivate
- Expected: no duplicate listeners, no crash
- Result: [x] Pass [ ] Fail
- Notes:

## Failure Capture Format

If a test fails, record:
- Device: <from debug line>
- RMS: <from debug line>
- Raw: <from debug line>
- Cleaned: <from debug line>
- Target app: <name>
- What happened: <short>

## Exit Criteria for Step 1

- All Core Flow tests pass
- At least 4/5 Target App validations pass (including VS Code + Browser + one plain editor)
- No crash in Deactivate/Reactivate cycles
