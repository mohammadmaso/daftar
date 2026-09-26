# ADR-0020: Voice pipeline in the core; energy VAD first

* Status: accepted
* Date: 2026-09-25

## Context
§8.4 needs VAD, STT → LLM → TTS streaming, barge-in within 200 ms (scenario 11) and a seam for a
future realtime speech-to-speech API. ADR-0002 deferred the VAD choice: `ort` 2.0 (for Silero VAD)
is still a release candidate and adds a native ONNX runtime to five platform builds.

## Decision
* The app streams 16-bit mono PCM to `voice::VoiceSession` and plays the audio events it gets back;
  everything else (VAD, turn-taking, STT, the `voice` model with Ask's read-only tools, sentence
  chunking, per-sentence voice choice by script, barge-in, "remember that…", transcript capture)
  lives in the core and is tested there with a simulated stream.
* VAD is an energy detector with an adaptive noise floor (20 ms frames, 90 ms to start, 700 ms of
  silence to end, short bursts discarded). `feed` is synchronous, so barge-in latency is the start
  window, independent of the network. Silero via `ort` can replace `EnergyVad::process` later.
* TTS voices come from the `tts` role's params: `voice_fa`, `voice_en`, else `voice`.
* `RealtimeVoice` is the seam for a speech-to-speech provider.

## Consequences
Echo cancellation must come from the platform audio session (voice-processing modes on mobile);
on desktop without AEC, speaker echo can trigger barge-in, so headsets are recommended there.
