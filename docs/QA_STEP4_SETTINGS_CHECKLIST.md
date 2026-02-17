# Quan Voice V1 - Step 4 Settings & Startup Consistency Checklist

Date: 2026-02-17
Scope: Persistent settings, restart consistency, and edge-case recovery.

## Automated Hardening Implemented

- Corrupt config recovery:
  - Invalid `%APPDATA%/QuanVoice/config.json` is backed up as `config.corrupt.json`
  - App regenerates default config and continues
- Settings sanitization on load/save:
  - Empty hotkey -> `RightCtrl`
  - Model forced to `tiny.en` (V1 scope)
  - Empty input device -> `Default`
- Startup device consistency:
  - If saved `input_device` no longer exists, app falls back to `Default` and re-saves

## Manual Checks

1. Persisted settings survive restart
- Change hotkey and input device, Save, restart app
- [x] Pass [ ] Fail
- Notes:

2. Activation state on restart
- Activate app, close app, relaunch
- Expected: app starts inactive until activated again
- [x] Pass [ ] Fail
- Notes:

3. Input device removed edge case
- Select a non-default mic, disconnect/disable it, restart app
- Expected: fallback to `Default` without crash
- [x] Pass [ ] Fail
- Notes:

4. Corrupt config edge case
- Corrupt `%APPDATA%/QuanVoice/config.json` manually, restart app
- Expected: app recovers and creates a new config, keeps running
- [x] Pass [ ] Fail
- Notes:

5. Deactivate/activate cycle consistency
- Perform 5 cycles
- Expected: no duplicate listener behavior, no crash
- [x] Pass [ ] Fail
- Notes:

## Exit Criteria

- All 5 checks pass
- No startup crash after config corruption
- No stale/missing device crash at startup
