# Quan Voice Latency Plan

Last updated: 2026-03-04

## Goal
- Make release-to-text feel instant for users.
- Primary KPI: `release -> first injected text`.

## Targets
- p50 `release -> inject_done`: <= 350 ms (short utterances)
- p95 `release -> inject_done`: <= 900 ms

## Current Status
- Phase 1 (Telemetry): `COMPLETED`
- Phase 2 (Cold-start + per-utterance overhead): `COMPLETED`
- Phase 3 (Streaming/partial output): `COMPLETED`
- Phase 4 (Runtime profiles + hardware acceleration): `COMPLETED`
- Phase 5 (Benchmark gate + release criteria): `COMPLETED`

## Phase 1: Telemetry (COMPLETED)
### Implemented
- Per-utterance timestamp metrics:
  - `release_ts`
  - `wav_ready_ts`
  - `decode_start_ts`
  - `first_token_ts`
  - `final_text_ts`
  - `inject_done_ts`
- Derived durations:
  - `stop_audio_ms`
  - `transcribe_ms`
  - `clean_ms`
  - `inject_ms`
  - `total_ms`
- Emitted event: `transcription-metrics`
- Added stderr structured metrics log line
- Persisted metrics JSONL to:
  - `%LOCALAPPDATA%\\QuanVoice\\metrics\\transcription_metrics.jsonl`

## Phase 2: Reduce Overhead (COMPLETED)
### Implemented
- Audio stop path optimization:
  - removed extra buffer clone (`mem::take`)
  - speed-first postprocess path enabled by default
  - env rollback: `QUAN_VOICE_AUDIO_FAST=0`
- Whisper startup optimization:
  - one-time per-model warmup after model-ready
- Whisper CLI path optimization:
  - removed text-file round-trip (`-otxt`)
  - parse stdout directly
  - explicit thread control (`-t`)
  - first-token timestamp capture from streaming stdout

### Remaining
- Remove per-utterance process spawn fully (resident engine or daemon worker).
- Keep model resident between utterances without relaunch cost.

## Phase 3: Streaming / Partial Injection (IN PROGRESS)
### Planned
- Decode incrementally while hotkey is held (200-400 ms intervals).
- Maintain stable committed text buffer.
- On release, inject final delta only.

### Implemented
- While hotkey is held, preview decode runs on a fixed cadence against rolling audio clip.
- On release, latest fresh preview is injected immediately (if available).
- Final decode still runs; only safe suffix delta is injected when final text is a prefix-extension.
- Hybrid accuracy/latency path:
  - preview decode always uses `tiny.en` for speed
  - final decode uses selected effective model
  - when preview/final mismatch, bounded backspace + retype correction payload is applied
- Hardened guards:
  - preview decode cadence reduced to cut lock contention and final-decode delay
  - preview decode uses non-blocking whisper lock (`try_lock`) to avoid blocking final path
  - preview cancellation on release to prevent overlap with final decode
  - word-boundary suffix delta logic for safer append behavior
  - non-speech filter prevents symbols/music-only output from being injected

## Phase 4: Runtime Profiles + Hardware (NOT STARTED)
### Planned
- Add user-selectable `Latency` vs `Accuracy` mode.
- Latency profile defaults:
  - greedy decode
  - no timestamps
  - low-overhead thresholds
- Optional acceleration backends by capability:
  - NVIDIA/CUDA
  - Intel/OpenVINO
  - CPU fallback

### Implemented
- Added `latency_mode` setting (default ON).
- When latency mode is ON:
  - effective model is forced to `tiny.en` for readiness + transcription path.
  - low-latency whisper decode flags are enforced (`-bo 1`, `-bs 1`, `-nf`, `-nt`, `--suppress-nst`).

## Phase 5: Benchmark Gate (COMPLETED)
### Planned
- Compare old vs new on low/mid/high hardware tiers.
- Publish p50/p95 latency table before release.
- Ship gate requires target compliance and evidence archive.

### Implemented
- Added summary command for p50/p95:
  - `npm.cmd run latency:summary`
- Added release gate command with pass/fail exit code:
  - `npm.cmd run latency:gate`
  - Gate evaluates `release_ts -> inject_done_ts` p50/p95 against targets.

## Immediate Next Actions
1. Implement Phase 3 streaming decode and partial injection.
2. Add diagnostics command/output to summarize p50/p95 from JSONL metrics.
3. Add latency/accuracy runtime mode in settings.
