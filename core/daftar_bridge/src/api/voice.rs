//! Voice mode for the Flutter app (§8.4): the app streams microphone PCM in and plays the audio
//! events it receives; the conversation logic lives in `daftar_core::voice`.

use std::sync::{Arc, Mutex};

use daftar_core::voice::{
    RealtimeVoice, VadConfig, VoiceConfig, VoiceEvent, VoiceSession, VoiceState,
};
use flutter_rust_bridge::frb;

use super::ai::ApiKey;
use super::library::LibraryHandle;
use crate::frb_generated::StreamSink;

pub enum VoiceStateDto {
    Listening,
    Hearing,
    Thinking,
    Speaking,
    Muted,
}

pub enum VoiceEventKind {
    State,
    UserCaption,
    AssistantCaption,
    /// Encoded audio for one sentence (format as returned by the TTS provider, usually MP3).
    Audio,
    StopPlayback,
    NeedsHelp,
    Captured,
    Error,
}

pub struct VoiceEventDto {
    pub kind: VoiceEventKind,
    pub state: Option<VoiceStateDto>,
    /// Caption text, captured raw id, or error sentence.
    pub text: Option<String>,
    pub seq: u64,
    pub lang: Option<String>,
    pub bytes: Option<Vec<u8>>,
}

fn ev(kind: VoiceEventKind) -> VoiceEventDto {
    VoiceEventDto {
        kind,
        state: None,
        text: None,
        seq: 0,
        lang: None,
        bytes: None,
    }
}

pub struct VoiceOptions {
    pub sample_rate: u32,
    /// Silence that ends an utterance, in ms (default 700).
    pub silence_ms: u32,
    /// 0 (least) … 1 (most sensitive); default 0.5.
    pub sensitivity: f32,
    pub save_transcript: bool,
}

#[frb(opaque)]
pub struct VoiceHandle {
    inner: Arc<Mutex<Option<VoiceSession>>>,
}

fn dto(e: VoiceEvent) -> VoiceEventDto {
    let with_text = |kind, t: String| VoiceEventDto {
        text: Some(t),
        ..ev(kind)
    };
    match e {
        VoiceEvent::State { state } => VoiceEventDto {
            state: Some(match state {
                VoiceState::Listening => VoiceStateDto::Listening,
                VoiceState::Hearing => VoiceStateDto::Hearing,
                VoiceState::Thinking => VoiceStateDto::Thinking,
                VoiceState::Speaking => VoiceStateDto::Speaking,
                VoiceState::Muted => VoiceStateDto::Muted,
            }),
            ..ev(VoiceEventKind::State)
        },
        VoiceEvent::UserCaption { text } => with_text(VoiceEventKind::UserCaption, text),
        VoiceEvent::AssistantCaption { text } => with_text(VoiceEventKind::AssistantCaption, text),
        VoiceEvent::Audio { seq, lang, bytes } => VoiceEventDto {
            seq,
            lang: Some(lang),
            bytes: Some(bytes),
            ..ev(VoiceEventKind::Audio)
        },
        VoiceEvent::StopPlayback => ev(VoiceEventKind::StopPlayback),
        VoiceEvent::NeedsHelp => ev(VoiceEventKind::NeedsHelp),
        VoiceEvent::Captured { raw_id } => with_text(VoiceEventKind::Captured, raw_id),
        VoiceEvent::Error { message } => with_text(VoiceEventKind::Error, message),
    }
}

impl LibraryHandle {
    /// Starts a voice conversation; events arrive on `sink` until `VoiceHandle::end`.
    pub async fn start_voice(
        &self,
        options: VoiceOptions,
        api_keys: Vec<ApiKey>,
        sink: StreamSink<VoiceEventDto>,
    ) -> anyhow::Result<VoiceHandle> {
        let session = self.session_arc();
        let keys = api_keys
            .into_iter()
            .filter(|k| !k.key.is_empty())
            .map(|k| (k.provider_id, k.key))
            .collect();
        let rt = Arc::new(
            session
                .runtime(keys)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );
        let cfg = VoiceConfig {
            sample_rate: options.sample_rate.max(8000),
            vad: VadConfig {
                silence_ms: options.silence_ms.clamp(300, 3000),
                // 0.5 → 12 dB above the room; the most sensitive setting is 6 dB, the least 18 dB.
                margin_db: 18.0 - 12.0 * options.sensitivity.clamp(0.0, 1.0),
                ..VadConfig::default()
            },
            save_transcript: options.save_transcript,
        };
        let (voice, mut rx) = VoiceSession::start(session, rt, cfg);
        tokio::spawn(async move {
            while let Some(e) = rx.recv().await {
                if sink.add(dto(e)).is_err() {
                    break;
                }
            }
        });
        Ok(VoiceHandle {
            inner: Arc::new(Mutex::new(Some(voice))),
        })
    }
}

impl VoiceHandle {
    /// Microphone PCM, mono 16-bit at the session's sample rate.
    pub fn feed(&self, pcm: Vec<i16>) {
        if let Some(v) = self
            .inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_mut()
        {
            v.feed(&pcm);
        }
    }

    pub fn playback_finished(&self) {
        if let Some(v) = self
            .inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_mut()
        {
            v.playback_finished();
        }
    }

    pub fn set_muted(&self, muted: bool) {
        if let Some(v) = self
            .inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_mut()
        {
            v.set_muted(muted);
        }
    }

    /// Ends the conversation; returns the id of the saved transcript capture, if any.
    pub fn end(&self) -> Option<String> {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
            .and_then(|v| v.end())
    }
}
