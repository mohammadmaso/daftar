//! Scenario 11 (§13): a simulated audio stream with barge-in stops TTS within 200 ms and captures
//! the new utterance. Also: first audio starts before the answer is complete, "remember that…"
//! files a capture, and the conversation transcript is saved as a capture at the end.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{Device, Remote};
use daftar_core::ledger::Usage;
use daftar_core::providers::{
    AiConfig, ChatRequest, ChatResponse, LlmProvider, OnDelta, ProviderConfig, ProviderKind,
    ProviderResult, Role, RoleConfig, StopReason,
};
use daftar_core::raw::{self, RawKind};
use daftar_core::runtime::AiRuntime;
use daftar_core::session::Session;
use daftar_core::voice::{RealtimeVoice, VoiceConfig, VoiceEvent, VoiceSession, VoiceState};
use tokio::sync::mpsc::UnboundedReceiver;

/// Speech-to-text from a list, a chat model that streams a three-sentence answer word by word, and
/// a text-to-speech that takes 120 ms per sentence.
struct Studio {
    heard: Mutex<Vec<&'static str>>,
    stt_calls: AtomicUsize,
    tts_calls: AtomicUsize,
}

#[async_trait::async_trait]
impl LlmProvider for Studio {
    fn name(&self) -> &str {
        "Studio"
    }
    async fn chat(&self, _req: &ChatRequest, d: OnDelta<'_>) -> ProviderResult<ChatResponse> {
        let text = "Sara is your cousin. She lives in Shiraz now. She called you last week about the trip.";
        for w in text.split_inclusive(' ') {
            if let Some(f) = d {
                f(w);
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok(ChatResponse {
            text: text.into(),
            tool_calls: vec![],
            usage: Usage::default(),
            stop: StopReason::EndTurn,
        })
    }
    async fn transcribe(
        &self,
        _m: &str,
        audio: Vec<u8>,
        _f: &str,
        _l: Option<&str>,
    ) -> ProviderResult<String> {
        assert_eq!(&audio[..4], b"RIFF");
        self.stt_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.heard.lock().unwrap().remove(0).to_owned())
    }
    async fn speech(&self, _m: &str, voice: &str, text: &str) -> ProviderResult<Vec<u8>> {
        self.tts_calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(120)).await;
        Ok(format!("{voice}:{text}").into_bytes())
    }
}

fn config() -> AiConfig {
    let role = |r: Role, params: serde_json::Value| RoleConfig {
        role: r,
        provider: "studio".into(),
        model: "m".into(),
        params: params.as_object().cloned().unwrap_or_default(),
    };
    AiConfig {
        providers: vec![ProviderConfig {
            id: "studio".into(),
            name: "Studio".into(),
            kind: ProviderKind::Mock,
            base_url: String::new(),
            extra_headers: Default::default(),
            timeout_s: 10,
        }],
        roles: vec![
            role(Role::Chat, serde_json::json!({})),
            role(Role::Stt, serde_json::json!({})),
            role(
                Role::Tts,
                serde_json::json!({"voice_en": "nova", "voice_fa": "dara"}),
            ),
        ],
        prices: Default::default(),
    }
}

fn tone(ms: u32, amp: f32) -> Vec<i16> {
    (0..(16 * ms))
        .map(|i| ((i as f32 * 0.21).sin() * amp) as i16)
        .collect()
}

fn drain(rx: &mut UnboundedReceiver<VoiceEvent>, into: &mut Vec<VoiceEvent>) {
    while let Ok(e) = rx.try_recv() {
        into.push(e);
    }
}

async fn wait_for(
    rx: &mut UnboundedReceiver<VoiceEvent>,
    seen: &mut Vec<VoiceEvent>,
    pred: impl Fn(&VoiceEvent) -> bool,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        drain(rx, seen);
        if seen.iter().any(&pred) {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out; saw {seen:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn setup(heard: Vec<&'static str>) -> (Remote, Device, Arc<Session>, Arc<Studio>, Arc<AiRuntime>) {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "pixel-8");
    let session = Arc::new(Session::open(d.root()).unwrap());
    let studio = Arc::new(Studio {
        heard: Mutex::new(heard),
        stt_calls: AtomicUsize::new(0),
        tts_calls: AtomicUsize::new(0),
    });
    let rt = Arc::new(
        AiRuntime::new(config(), Default::default()).with_provider("studio", studio.clone()),
    );
    (remote, d, session, studio, rt)
}

#[tokio::test(flavor = "multi_thread")]
async fn barge_in_stops_speech_within_200ms_and_hears_the_new_utterance() {
    let (_r, d, session, studio, rt) = setup(vec!["Tell me about Sara", "Thanks, that's enough"]);
    let (mut voice, mut rx) = VoiceSession::start(session, rt, VoiceConfig::default());
    let mut seen = vec![];

    voice.feed(&tone(400, 40.0)); // room
    voice.feed(&tone(1200, 9000.0)); // question
    voice.feed(&tone(800, 40.0)); // end of speech (700 ms silence)
    wait_for(&mut rx, &mut seen, |e| {
        matches!(e, VoiceEvent::UserCaption { .. })
    })
    .await;
    wait_for(&mut rx, &mut seen, |e| {
        matches!(e, VoiceEvent::Audio { .. })
    })
    .await;
    let first_audio = seen
        .iter()
        .position(|e| matches!(e, VoiceEvent::Audio { .. }))
        .unwrap();
    let Some(VoiceEvent::Audio { bytes, lang, .. }) = seen.get(first_audio) else {
        unreachable!()
    };
    assert_eq!(
        std::str::from_utf8(bytes).unwrap(),
        "nova:Sara is your cousin.",
        "speaks the first sentence first"
    );
    assert_eq!(lang, "en");
    assert!(seen.contains(&VoiceEvent::State {
        state: VoiceState::Speaking
    }));

    // The user talks over the answer: feed 20 ms frames until playback is stopped.
    let mut fed_ms = 0;
    let mut stopped_after = None;
    for frame in tone(600, 9000.0).chunks(320) {
        voice.feed(frame);
        fed_ms += 20;
        drain(&mut rx, &mut seen);
        if seen.iter().any(|e| matches!(e, VoiceEvent::StopPlayback)) {
            stopped_after = Some(fed_ms);
            break;
        }
    }
    let stopped_after = stopped_after.expect("barge-in stops playback");
    assert!(
        stopped_after <= 200,
        "stopped after {stopped_after} ms of speech"
    );
    let audio_before_stop = seen
        .iter()
        .filter(|e| matches!(e, VoiceEvent::Audio { .. }))
        .count();

    voice.feed(&tone(400, 9000.0));
    voice.feed(&tone(800, 40.0));
    wait_for(
        &mut rx,
        &mut seen,
        |e| matches!(e, VoiceEvent::UserCaption { text } if text == "Thanks, that's enough"),
    )
    .await;
    assert_eq!(studio.stt_calls.load(Ordering::SeqCst), 2);
    // The interrupted answer's remaining sentences are never delivered.
    let stop = seen
        .iter()
        .position(|e| matches!(e, VoiceEvent::StopPlayback))
        .unwrap();
    let second_caption = seen
        .iter()
        .position(|e| matches!(e, VoiceEvent::UserCaption { text } if text.starts_with("Thanks")))
        .unwrap();
    assert!(
        seen[stop..second_caption]
            .iter()
            .all(|e| !matches!(e, VoiceEvent::Audio { .. })),
        "no audio from the cancelled turn after the stop: {seen:?}"
    );
    assert!(audio_before_stop < 3);

    wait_for(&mut rx, &mut seen, |e| {
        matches!(
            e,
            VoiceEvent::State {
                state: VoiceState::Speaking
            }
        )
    })
    .await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    voice.playback_finished();
    let raw_id = voice.end().expect("transcript saved");
    let item = raw::find(&d.lib, raw_id.parse().unwrap()).unwrap().unwrap();
    assert_eq!(item.meta.kind, RawKind::VoiceConversation);
    assert!(item.body.contains("**Me:** Tell me about Sara"));
    assert!(item.body.contains("**Me:** Thanks, that's enough"));
}

#[tokio::test(flavor = "multi_thread")]
async fn remember_that_files_a_capture() {
    let (_r, d, session, _studio, rt) = setup(vec!["یادت باشه که کد در ۴۴۱۲ است"]);
    let (mut voice, mut rx) = VoiceSession::start(
        session,
        rt,
        VoiceConfig {
            save_transcript: false,
            ..Default::default()
        },
    );
    let mut seen = vec![];
    voice.feed(&tone(300, 40.0));
    voice.feed(&tone(900, 9000.0));
    voice.feed(&tone(800, 40.0));
    wait_for(&mut rx, &mut seen, |e| {
        matches!(e, VoiceEvent::Captured { .. })
    })
    .await;
    let Some(VoiceEvent::Captured { raw_id }) = seen
        .iter()
        .find(|e| matches!(e, VoiceEvent::Captured { .. }))
    else {
        unreachable!()
    };
    let item = raw::find(&d.lib, raw_id.parse().unwrap()).unwrap().unwrap();
    assert_eq!(item.body, "کد در ۴۴۱۲ است");
    wait_for(
        &mut rx,
        &mut seen,
        |e| matches!(e, VoiceEvent::Audio { lang, .. } if lang == "fa"),
    )
    .await;
    assert!(
        voice.end().is_none(),
        "transcript saving was off for this session"
    );
}
