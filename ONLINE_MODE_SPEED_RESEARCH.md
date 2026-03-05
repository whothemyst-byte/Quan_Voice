# Online Mode Research: Faster, More Instant Text Appearance

Date: 2026-03-04
Product: Quan Voice

## Short answer
Yes. Moving to online streaming STT can be much faster for **first text appearance** than your current offline path, especially on heavier models like `small.en`.

Why:
- Cloud models run on stronger infrastructure.
- Streaming APIs return interim/partial text while user is still speaking.
- Endpointing/VAD can finalize turns quickly.

## What your current data shows
From your recent runs:
- `tiny.en` is much faster than `small.en`.
- `small.en` is the main reason p95 is very high.
- Injection is already fast; decode is the bottleneck.

So if you want instant UX + better vocabulary, online streaming is a strong option.

---

## Research summary (simple)

## 1) Google Cloud Speech-to-Text (Streaming)
- Supports real-time streaming recognition.
- Recommends ~100ms frame size for latency/efficiency balance.
- Supports `single_utterance=true` for command-like quick end detection.
- Strong enterprise reliability and regional deployment options.

Best for:
- Enterprise apps, compliance-heavy customers, predictable infra.

## 2) Deepgram Streaming
- Strong controls for endpointing and interim results.
- `endpointing` and `utterance_end_ms` help you tune “when transcript is finalized”.
- Good for voice agent style turn detection.

Best for:
- Fast conversational UX with tuneable endpointing behavior.

## 3) AssemblyAI Streaming
- WebSocket streaming with immutable transcript behavior (useful for stable UI updates).
- Public docs describe partial/final handling and low-latency operation.
- Public product pages claim sub-second and low-hundreds-ms style performance (depending on setup/model).

Best for:
- Quick integration and strong real-time developer ergonomics.

## 4) OpenAI Realtime / Audio APIs
- Supports streaming audio processing and low-latency speech workflows.
- Good if you want unified speech + LLM pipeline in one vendor stack.

Best for:
- Voice + AI assistant workflows in one architecture.

---

## Expected latency (practical)
Online systems usually improve **first text latency** a lot, but exact results depend on:
- model choice
- endpointing settings
- network RTT to region
- audio chunk size and client buffering

Typical practical targets you can aim for in online mode:
- First visible partial: 200-700ms
- End-of-turn final text: 400-1500ms (short utterances)

Note: if RTT is high or region is far, this gets worse quickly.

---

## Recommended architecture for Quan Voice (online)

## A) Keep local capture + injection, move STT to streaming cloud
- Keep your current push-to-talk and SendInput injection layer.
- Replace offline final decode with WebSocket streaming to cloud STT.

## B) Stream while key is held
- Send 20-100ms audio chunks continuously.
- Render interim text in app.
- Commit only stable words (avoid jitter).

## C) On key release
- Stop streaming input.
- Wait for final segment/turn message.
- Inject only remaining suffix (append-only default).

## D) Fallbacks
- If network fails or latency > threshold, switch to offline tiny model automatically.
- User-facing status: `Online`, `Offline Fallback`, `Reconnecting`.

---

## Must-have controls (to make online actually fast)
1. Region pinning close to user.
2. Small frame size (20-100ms).
3. Interim results ON.
4. Endpointing tuned (e.g., 200-500ms for chat UX).
5. Keep one persistent connection/session (avoid reconnect per utterance).
6. Queue backpressure: drop stale partial jobs.
7. Network timeout budget + auto fallback.

---

## Tradeoffs you must accept
Pros:
- Faster first text
- Better vocabulary/accuracy (without forcing tiny model)
- More predictable high-end throughput

Cons:
- Internet dependency
- Ongoing API cost
- Privacy/compliance changes (audio leaves device unless self-hosted/private deployment)
- Vendor lock-in risk

---

## Rollout plan (safe)

## Phase O1 - Online prototype (2-4 days)
- Add provider abstraction (`stt_provider` interface).
- Implement one provider first (recommended: Deepgram or Google streaming).
- Keep offline as hard fallback.

## Phase O2 - Hybrid production mode (2-3 days)
- Default: online streaming for partial/final.
- Auto fallback to offline tiny on network errors.
- Add settings toggle:
  - `Offline only`
  - `Online preferred (fallback offline)`

## Phase O3 - Quality/latency tuning (1-2 days)
- Tune endpointing thresholds for your use case.
- Per-model/provider profile presets.
- Add metrics by mode/provider:
  - `release -> first_partial`
  - `release -> final`
  - failover frequency

## Phase O4 - Public release readiness (1 day)
- Cost guardrails (rate limits, usage caps).
- Privacy/legal update in app and site.
- Multi-region selection and outage strategy.

---

## Recommendation
For your current bottleneck, **online preferred + offline fallback** is the best path:
- Instant perceived response from online streaming partials.
- Better vocabulary than forced tiny offline mode.
- Reliability retained through automatic offline fallback.

If you want, next I can create the exact implementation spec for your codebase:
- interface files
- command/event contracts
- per-provider config schema
- migration steps from current `whisper.rs`

---

## Sources
- OpenAI Audio guide (Realtime/streaming): https://platform.openai.com/docs/guides/audio
- Google STT best practices (frame size, latency tips): https://docs.cloud.google.com/speech-to-text/docs/best-practices-provide-speech-data
- Google STT streaming recognize docs: https://docs.cloud.google.com/speech-to-text/v2/docs/streaming-recognize
- Deepgram endpointing docs: https://developers.deepgram.com/docs/endpointing
- Deepgram utterance end docs: https://developers.deepgram.com/docs/utterance-end
- Deepgram interim results docs: https://developers.deepgram.com/docs/interim-results
- AssemblyAI streaming guide: https://www.assemblyai.com/docs/guides/real-time-streaming-transcription
- AssemblyAI streaming overview: https://www.assemblyai.com/docs/guides/streaming
- AssemblyAI Universal-3 Pro streaming docs: https://www.assemblyai.com/docs/streaming/universal-3-pro
