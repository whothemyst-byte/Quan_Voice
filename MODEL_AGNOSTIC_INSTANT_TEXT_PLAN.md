# Model-Agnostic Instant Text Plan

Date: 2026-03-04
Owner: Quan Voice
Status: Implemented (v1 rollout) - requires validation run on target machines

## Why a new plan
Current "hybrid" behavior improved first text in some cases, but introduced instability (freeze risk) and inconsistent UX. We need one architecture that:
- works with all supported models (`tiny.en`, `base.en`, `small.en`, and future models)
- never freezes UI
- gives instant perceived response without forcing a single model

## Research-backed constraints
- `small.en` is materially slower than `tiny.en`; model size and memory scale significantly (whisper.cpp model/memory table).
- `whisper.cpp` has a real-time example (`whisper-stream`) built around periodic chunk processing (`--step`, `--length`).
- VAD in whisper.cpp can reduce decoded audio volume and speed up processing by removing silence.
- Hardware acceleration options in whisper.cpp (CUDA/OpenVINO/Vulkan) improve throughput but first-run warmup cost exists.

## Product targets
- First visible text after release: p50 <= 350 ms, p95 <= 900 ms.
- Final corrected text completion: p50 <= 2.5 s, p95 <= 8 s (model-dependent).
- Zero UI freezes.
- Silent hold/release: inject nothing.

## Core design (applies to all models)
### 1) Session-based ASR worker (no lock contention)
- Create one dedicated ASR worker per active model session.
- Main thread never performs decode calls directly.
- Communication via bounded message queue:
  - `StartSession(model, profile)`
  - `PushAudioChunk(samples)`
  - `PollPartial`
  - `Finalize`
  - `Cancel`
- Enforce hard time budgets and cancellation on release.

### 2) Streaming decode pipeline (selected model, not forced tiny)
- During hold:
  - Push audio every 200-320 ms.
  - Decode rolling window 3-6 s with context carry.
- On release:
  - Stop capture immediately.
  - Finalize only remaining tail audio.
- This removes "full decode starts only at release" behavior.

### 3) Stable-word commit policy (no destructive correction by default)
- Do not inject full partial hypotheses repeatedly.
- Only inject words that are stable across consecutive partials (N=2) or confidence threshold.
- Finalization appends remaining suffix only.
- Backspace-based rewrite is disabled by default; allowed only in explicit "aggressive correction" mode.

### 4) No-speech gate (anti-hallucination)
- Pre-decode gate: level + minimum voiced duration.
- Optional VAD prefilter for chunk/finalization.
- Post-decode gate:
  - reject low-signal short hallucinations (e.g., "you", "thanks")
  - reject symbol/music-only output

### 5) Model profiles (same architecture, different budgets)
- `tiny.en`: aggressive step size (200-260 ms), larger stable threshold optional.
- `base.en`: balanced step (260-320 ms).
- `small.en`: conservative step (320-450 ms), smaller window to cap compute.
- All profiles share the same streaming/commit pipeline.

### 6) Performance controls
- Keep low-latency decode flags configurable per profile (`beam/best_of/no-fallback/no-timestamps`).
- Add worker watchdog (e.g., decode task > 2.5 s for partial => drop partial, continue stream).
- Add queue backpressure (drop stale partial jobs; keep latest only).

### 7) Hardware acceleration track (optional)
- Detect acceleration capability at startup.
- Use accelerated backend when available:
  - NVIDIA CUDA (whisper.cpp)
  - Intel OpenVINO (whisper.cpp)
- Keep CPU fallback deterministic.

## Implementation phases

## Phase A - Stabilize before speed (1 day)
- Remove/disable experimental rewrite paths that can freeze or deadlock.
- Add worker thread + bounded queue skeleton.
- Add explicit cancellation on release and deactivate.
Exit criteria:
- No freezes in 200 continuous press/release cycles.

### Phase A Progress (implemented)
- Added dedicated ASR worker skeleton module with bounded queue and command types:
  - `StartSession`, `PushAudioChunk`, `PollPartial`, `Finalize`, `Cancel`, `Shutdown`
- Wired worker handle into app state and added diagnostics command:
  - `get_asr_worker_snapshot`
- Disabled unstable Phase 3 preview path by default behind constant gate:
  - `ENABLE_PREVIEW_PHASE3 = false`

## Phase B - Streaming engine integration (2-3 days)
- Implement `StartSession/Push/Finalize` for selected model.
- Partial decode cadence per profile.
- Preserve existing final decode path as fallback switch.
Exit criteria:
- First partial available before release on typical 2-4 s utterances.

## Phase C - Stable-word injection policy (1-2 days)
- Token/word stability tracker.
- Append-only commit strategy.
- Final suffix append on finalize.
Exit criteria:
- No duplicated text and no destructive rewrites in default mode.

## Phase D - VAD + hallucination suppression (1-2 days)
- Integrate VAD preprocessing (configurable) for chunk/final.
- Keep current no-speech hallucination rules and tune thresholds.
Exit criteria:
- Silent hold/release injects nothing in 100/100 trials.

## Phase E - Benchmark + tuning (1 day)
- Benchmark by model:
  - tiny.en
  - base.en
  - small.en
- Publish p50/p95 for:
  - `release->first_text`
  - `release->final`
- Tune per-model step/window thresholds.
Exit criteria:
- Targets met or documented per-model limits with defaults.

## Implementation Status Snapshot
- Phase A: Completed
- Phase B: Completed
- Phase C: Completed
- Phase D: Completed
- Phase E: Completed

## Observability required
For every utterance keep:
- `release_ts`
- `wav_ready_ts`
- `decode_start_ts`
- `first_token_ts`
- `final_text_ts`
- `inject_done_ts`
- model + profile + chunk settings
- queue lag and dropped-partial count

Store in `%LOCALAPPDATA%\\QuanVoice\\metrics\\transcription_metrics.jsonl`.

## Manual test matrix (post-implementation)
1. Silent hold/release x50 (expect zero injection).
2. Short utterance (1-2 s) x30.
3. Medium utterance (3-6 s) x30.
4. Fast repeated activations (10 utterances in 60 s).
5. Across models: tiny/base/small.
6. Across devices: low-end laptop, mid desktop, high-end desktop.

## Risks and mitigations
- Risk: worker backlog with slow models.
  - Mitigation: drop stale partial jobs; keep latest job only.
- Risk: correction jitter from unstable partials.
  - Mitigation: stable-word commit threshold + append-only default.
- Risk: backend lock contention.
  - Mitigation: single-owner ASR worker, no cross-thread model lock sharing.

## Sources
- whisper.cpp README (real-time stream example, VAD, model/memory, acceleration): https://github.com/ggml-org/whisper.cpp
- faster-whisper README (batched/streaming considerations, VAD behavior): https://github.com/SYSTRAN/faster-whisper
- CTranslate2 quantization options and compute types: https://opennmt.net/CTranslate2/quantization.html
