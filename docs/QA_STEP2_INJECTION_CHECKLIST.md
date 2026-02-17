# Quan Voice V1 - Step 2 Injection Reliability Checklist

Date: 2026-02-17
Scope: SendInput reliability for punctuation, newlines, tabs, unicode, and app-specific input behavior.

## Preconditions

- App running via `npm.cmd run tauri:dev`
- Activation enabled
- Working microphone/transcription path confirmed (Step 1 pass)

## Test Strings (Speak or paste via debug helper flow)

1. Basic sentence:
- `hello this is a quick reliability test`

2. Punctuation sentence:
- `hello, world. this should keep punctuation!`

3. Multi-line dictation:
- `line one`
- `line two`

4. Tabs / indentation case:
- `function test` then tab behavior in target editors/terminals

5. Unicode words:
- `cafe na�ve r�sum�`

6. Numbers/symbols:
- `version 1.2.3 / build #42`

## Target Matrix

1. Notepad
- [x] Basic
- [x] Multi-line
- [x] Punctuation
- Notes:

2. VS Code editor
- [x] Basic
- [x] Multi-line
- [x] Tab/indent
- [x] Unicode
- Notes:

3. Browser textarea
- [x] Basic
- [x] Multi-line
- [x] Punctuation
- Notes:

4. Terminal
- [x] Basic
- [x] Enter/newline behavior
- [x] Symbols
- Notes:

5. Word/Discord
- [x] Basic
- [x] Punctuation
- [x] Unicode
- Notes:

## Reliability Edge Cases

1. Rapid successive dictation (3 runs back-to-back)
- [x] Pass
- Notes:

2. Long paragraph injection
- [x] Pass
- Notes:

3. Deactivate/reactivate between injections
- [x] Pass
- Notes:

## Failure Capture

- Target app:
- Spoken content:
- Actual injected content:
- Debug line (Device/RMS/Raw/Cleaned):
- Repro steps:

## Step 2 Exit Criteria

- No dropped input events in normal dictation
- Newline handling is consistent across editors + browser
- Unicode and punctuation are preserved in at least 4 target apps
