//! Hands-free voice mode (§8.4). The app owns the microphone and the speaker; the core owns the
//! conversation: voice activity detection on 16-bit PCM frames, speech-to-text, the `voice`-role
//! model with the same read-only tools as Ask, sentence-chunked text-to-speech that starts with the
//! first sentence, and barge-in (speech while the assistant talks stops playback at once).
//!
//! `VoiceSession::feed` is synchronous and cheap, so a barge-in is detected within one VAD start
//! window (90 ms of audio by default) of the user starting to speak, independent of network latency.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::agent::Cancel;
use crate::ask::{AskScope, HELP_MARKER, Turn};
use crate::providers::Role;
use crate::runtime::AiRuntime;
use crate::session::Session;

// ─────────────────────────── VAD ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VadConfig {
    /// Silence that ends an utterance (§8.4 default 700 ms).
    pub silence_ms: u32,
    /// Voiced audio needed to call it speech; also the barge-in latency.
    pub start_ms: u32,
    /// Utterances shorter than this are dropped (coughs, clicks).
    pub min_speech_ms: u32,
    /// dB above the tracked noise floor that counts as voice. Lower = more sensitive.
    pub margin_db: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            silence_ms: 700,
            start_ms: 90,
            min_speech_ms: 250,
            margin_db: 12.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadEvent {
    None,
    SpeechStart,
    SpeechEnd,
    /// Speech started but was too short to be an utterance.
    Discarded,
}

/// Energy VAD with an adaptive noise floor. The always-available fallback of §2; a model-based VAD
/// can replace it behind the same `process` call.
pub struct EnergyVad {
    cfg: VadConfig,
    noise_db: f32,
    in_speech: bool,
    voiced_ms: u32,
    silent_ms: u32,
    speech_ms: u32,
}

pub fn frame_db(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return -100.0;
    }
    let sum: f64 = frame.iter().map(|&s| (s as f64) * (s as f64)).sum();
    let rms = (sum / frame.len() as f64).sqrt();
    (20.0 * (rms.max(1.0) / 32768.0).log10()) as f32
}

impl EnergyVad {
    pub fn new(cfg: VadConfig) -> Self {
        Self {
            cfg,
            noise_db: -60.0,
            in_speech: false,
            voiced_ms: 0,
            silent_ms: 0,
            speech_ms: 0,
        }
    }

    pub fn in_speech(&self) -> bool {
        self.in_speech
    }

    /// One frame of `frame_ms` milliseconds.
    pub fn process(&mut self, frame: &[i16], frame_ms: u32) -> VadEvent {
        let db = frame_db(frame);
        let voiced = db > (self.noise_db + self.cfg.margin_db).max(-50.0);
        if !voiced && !self.in_speech {
            // Track the room slowly; never let speech raise the floor.
            self.noise_db = 0.95 * self.noise_db + 0.05 * db;
        }
        if !self.in_speech {
            if voiced {
                self.voiced_ms += frame_ms;
                if self.voiced_ms >= self.cfg.start_ms {
                    self.in_speech = true;
                    self.silent_ms = 0;
                    self.speech_ms = self.voiced_ms;
                    return VadEvent::SpeechStart;
                }
            } else {
                self.voiced_ms = 0;
            }
            return VadEvent::None;
        }
        self.speech_ms += frame_ms;
        if voiced {
            self.silent_ms = 0;
            return VadEvent::None;
        }
        self.silent_ms += frame_ms;
        if self.silent_ms < self.cfg.silence_ms {
            return VadEvent::None;
        }
        self.in_speech = false;
        self.voiced_ms = 0;
        if self.speech_ms.saturating_sub(self.silent_ms) < self.cfg.min_speech_ms {
            VadEvent::Discarded
        } else {
            VadEvent::SpeechEnd
        }
    }
}

// ─────────────────────────── sentences ───────────────────────────

/// Plain speech from model text: wikilinks become their label, Markdown marks go away.
pub fn speakable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for l in crate::wiki::links(text) {
        out.push_str(&text[last..l.start]);
        if l.target != "talk-to-someone" {
            out.push_str(l.label.as_deref().unwrap_or(&l.target));
        }
        last = l.end;
    }
    out.push_str(&text[last..]);
    out.lines()
        .map(|l| {
            l.trim_start_matches(['#', '>', '-', '*', ' '])
                .replace("**", "")
                .replace('`', "")
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Splits streamed text into sentences as soon as each is complete, so speech can start early.
#[derive(Default)]
pub struct SentenceChunker {
    buf: String,
}

const ENDERS: &[char] = &['.', '!', '?', '؟', '…', '۔', '\n'];
const MAX_SENTENCE: usize = 240;

impl SentenceChunker {
    pub fn push(&mut self, delta: &str) -> Vec<String> {
        self.buf.push_str(delta);
        let mut out = Vec::new();
        loop {
            let chars: Vec<(usize, char)> = self.buf.char_indices().collect();
            let mut cut = None;
            for (i, &(pos, c)) in chars.iter().enumerate() {
                if !ENDERS.contains(&c) {
                    continue;
                }
                let next = chars.get(i + 1).map(|x| x.1);
                // "3.5", "e.g.x" — not an end; need a following space/newline (or a newline itself).
                let ends = c == '\n' || next.is_some_and(char::is_whitespace);
                let decimal = c == '.'
                    && i > 0
                    && chars[i - 1].1.is_ascii_digit()
                    && next.is_some_and(|n| n.is_ascii_digit());
                if ends && !decimal {
                    cut = Some(pos + c.len_utf8());
                    break;
                }
            }
            if cut.is_none() && self.buf.chars().count() > MAX_SENTENCE {
                // A run-on sentence: break at the last comma or space so speech keeps flowing.
                cut = self
                    .buf
                    .rfind([',', '،', ';'])
                    .or_else(|| self.buf.rfind(' '))
                    .map(|p| p + 1);
            }
            let Some(cut) = cut else { break };
            let sentence = speakable(&self.buf[..cut]);
            self.buf.drain(..cut);
            if sentence.chars().any(char::is_alphanumeric) {
                out.push(sentence);
            }
        }
        out
    }

    pub fn flush(&mut self) -> Option<String> {
        let s = speakable(&std::mem::take(&mut self.buf));
        s.chars().any(char::is_alphanumeric).then_some(s)
    }
}

/// `fa` when a sentence is mostly Arabic script, else `en` (§8.4: voice chosen per sentence).
pub fn lang_of(text: &str) -> &'static str {
    let (mut arabic, mut latin) = (0, 0);
    for c in text.chars() {
        match c {
            '\u{0600}'..='\u{06FF}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}' => {
                arabic += 1
            }
            c if c.is_ascii_alphabetic() => latin += 1,
            _ => {}
        }
    }
    if arabic > latin { "fa" } else { "en" }
}

/// Mono 16-bit PCM as a WAV file for speech-to-text.
pub fn wav(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&sample_rate.to_le_bytes());
    w.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

/// "remember that …" / «یادت باشه …»: returns what to remember.
pub fn remember_command(text: &str) -> Option<String> {
    let t = text.trim();
    let lower = t.to_lowercase();
    for p in [
        "remember that",
        "remember:",
        "note that",
        "یادت باشه که",
        "یادت باشه",
        "یادت بماند که",
        "به خاطر بسپار که",
        "به خاطر بسپار",
        "یادداشت کن که",
        "یادداشت کن",
    ] {
        if lower.starts_with(p) {
            let rest = t[p.len()..].trim_start_matches([',', '،', ':', ' ']).trim();
            if !rest.is_empty() {
                return Some(rest.to_owned());
            }
        }
    }
    None
}

// ─────────────────────────── session ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceState {
    Listening,
    /// The user is speaking.
    Hearing,
    Thinking,
    Speaking,
    Muted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VoiceEvent {
    State {
        state: VoiceState,
    },
    /// What the user said (after speech-to-text).
    UserCaption {
        text: String,
    },
    /// One sentence the assistant is about to speak.
    AssistantCaption {
        text: String,
    },
    /// Encoded audio (as the TTS provider returns it) for one sentence, in order.
    Audio {
        seq: u64,
        lang: String,
        bytes: Vec<u8>,
    },
    /// Stop and drop all queued audio now (barge-in or end).
    StopPlayback,
    /// Show the "Talk to someone" card.
    NeedsHelp,
    /// "remember that…" was filed as a capture.
    Captured {
        raw_id: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub sample_rate: u32,
    pub vad: VadConfig,
    /// Save the conversation as a `voice-conversation` capture at the end (§8.4, per session).
    pub save_transcript: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            vad: VadConfig::default(),
            save_transcript: true,
        }
    }
}

/// A speech-to-speech realtime API can implement this later (§8.4); the pipeline below is the
/// shipped implementation.
pub trait RealtimeVoice: Send {
    /// Feeds microphone PCM (mono, 16-bit, `sample_rate`).
    fn feed(&mut self, pcm: &[i16]);
    /// The app finished playing all audio it was given.
    fn playback_finished(&mut self);
    fn set_muted(&mut self, muted: bool);
}

struct Shared {
    state: Mutex<VoiceState>,
    tx: UnboundedSender<VoiceEvent>,
    playing: AtomicBool,
    seq: AtomicU64,
    history: Mutex<Vec<Turn>>,
    transcript: Mutex<Vec<(bool, String)>>,
}

impl Shared {
    fn send(&self, e: VoiceEvent) {
        let _ = self.tx.send(e);
    }
    fn set(&self, s: VoiceState) {
        let mut g = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if *g != s {
            *g = s;
            self.send(VoiceEvent::State { state: s });
        }
    }
    fn state(&self) -> VoiceState {
        *self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
}

pub struct VoiceSession {
    session: Arc<Session>,
    rt: Arc<AiRuntime>,
    cfg: VoiceConfig,
    vad: EnergyVad,
    shared: Arc<Shared>,
    frame: Vec<i16>,
    preroll: VecDeque<i16>,
    utterance: Vec<i16>,
    turn: Option<(Cancel, tokio::task::JoinHandle<()>)>,
    muted: bool,
}

impl VoiceSession {
    /// Must be created inside a tokio runtime (turns run as tasks).
    pub fn start(
        session: Arc<Session>,
        rt: Arc<AiRuntime>,
        cfg: VoiceConfig,
    ) -> (Self, UnboundedReceiver<VoiceEvent>) {
        let (tx, rx) = unbounded_channel();
        let shared = Arc::new(Shared {
            state: Mutex::new(VoiceState::Listening),
            tx,
            playing: AtomicBool::new(false),
            seq: AtomicU64::new(0),
            history: Mutex::new(vec![]),
            transcript: Mutex::new(vec![]),
        });
        shared.send(VoiceEvent::State {
            state: VoiceState::Listening,
        });
        (
            Self {
                session,
                rt,
                vad: EnergyVad::new(cfg.vad),
                cfg,
                shared,
                frame: Vec::new(),
                preroll: VecDeque::new(),
                utterance: Vec::new(),
                turn: None,
                muted: false,
            },
            rx,
        )
    }

    fn frame_len(&self) -> usize {
        (self.cfg.sample_rate / 50) as usize // 20 ms
    }

    fn busy(&self) -> bool {
        self.turn.as_ref().is_some_and(|(_, h)| !h.is_finished())
            || self.shared.playing.load(Ordering::SeqCst)
    }

    fn barge_in(&mut self) {
        if let Some((cancel, _)) = self.turn.take() {
            cancel.cancel();
        }
        self.shared.playing.store(false, Ordering::SeqCst);
        self.shared.send(VoiceEvent::StopPlayback);
    }

    fn on_frame(&mut self, frame: &[i16]) {
        let ev = self.vad.process(frame, 20);
        let preroll_max = (self.cfg.sample_rate as usize) * 3 / 10;
        self.preroll.extend(frame.iter().copied());
        while self.preroll.len() > preroll_max {
            self.preroll.pop_front();
        }
        match ev {
            VadEvent::SpeechStart => {
                if self.busy() {
                    self.barge_in();
                }
                self.utterance = self.preroll.iter().copied().collect();
                self.shared.set(VoiceState::Hearing);
            }
            VadEvent::SpeechEnd => {
                self.utterance.extend_from_slice(frame);
                let audio = std::mem::take(&mut self.utterance);
                self.spawn_turn(audio);
            }
            VadEvent::Discarded => {
                self.utterance.clear();
                self.shared.set(if self.busy() {
                    VoiceState::Speaking
                } else {
                    VoiceState::Listening
                });
            }
            VadEvent::None if self.vad.in_speech() => self.utterance.extend_from_slice(frame),
            VadEvent::None => {}
        }
    }

    fn spawn_turn(&mut self, audio: Vec<i16>) {
        let cancel = Cancel::default();
        self.shared.set(VoiceState::Thinking);
        let task = run_turn(
            self.session.clone(),
            self.rt.clone(),
            self.shared.clone(),
            audio,
            self.cfg.sample_rate,
            cancel.clone(),
        );
        self.turn = Some((cancel, tokio::spawn(task)));
    }

    /// Ends the session: stops everything and, if enabled, files the conversation (§8.4).
    pub fn end(mut self) -> Option<String> {
        self.barge_in();
        let transcript = std::mem::take(
            &mut *self
                .shared
                .transcript
                .lock()
                .unwrap_or_else(|p| p.into_inner()),
        );
        if !self.cfg.save_transcript || transcript.is_empty() {
            return None;
        }
        let text = transcript
            .iter()
            .map(|(user, t)| format!("**{}:** {t}", if *user { "Me" } else { "Assistant" }))
            .collect::<Vec<_>>()
            .join("\n\n");
        match self
            .session
            .capture_conversation(&text, &jiff::Zoned::now())
        {
            Ok(item) => Some(item.meta.id),
            Err(e) => {
                self.shared.send(VoiceEvent::Error {
                    message: e.to_string(),
                });
                None
            }
        }
    }
}

impl RealtimeVoice for VoiceSession {
    fn feed(&mut self, pcm: &[i16]) {
        if self.muted {
            return;
        }
        let n = self.frame_len();
        self.frame.extend_from_slice(pcm);
        while self.frame.len() >= n {
            let f: Vec<i16> = self.frame.drain(..n).collect();
            self.on_frame(&f);
        }
    }

    fn playback_finished(&mut self) {
        self.shared.playing.store(false, Ordering::SeqCst);
        if self.shared.state() == VoiceState::Speaking {
            self.shared.set(VoiceState::Listening);
        }
    }

    fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if muted {
            self.utterance.clear();
            self.vad = EnergyVad::new(self.cfg.vad);
            self.shared.set(VoiceState::Muted);
        } else {
            self.shared.set(if self.busy() {
                VoiceState::Speaking
            } else {
                VoiceState::Listening
            });
        }
    }
}

fn voice_for(
    rt: &AiRuntime,
    lang: &str,
) -> Option<(crate::providers::DynProvider, String, String)> {
    let (p, rc) = rt.for_role(Role::Tts).ok()?;
    let voice = rc
        .params
        .get(&format!("voice_{lang}"))
        .or_else(|| rc.params.get("voice"))
        .and_then(|v| v.as_str())
        .unwrap_or("alloy")
        .to_owned();
    Some((p, rc.model, voice))
}

async fn speak(shared: &Shared, rt: &AiRuntime, sentence: &str, cancel: &Cancel) {
    if cancel.is_cancelled() {
        return;
    }
    shared.send(VoiceEvent::AssistantCaption {
        text: sentence.to_owned(),
    });
    let lang = lang_of(sentence);
    let Some((p, model, voice)) = voice_for(rt, lang) else {
        return; // No TTS configured: captions only.
    };
    match p.speech(&model, &voice, sentence).await {
        Ok(bytes) if !cancel.is_cancelled() => {
            shared.playing.store(true, Ordering::SeqCst);
            shared.set(VoiceState::Speaking);
            shared.send(VoiceEvent::Audio {
                seq: shared.seq.fetch_add(1, Ordering::SeqCst),
                lang: lang.to_owned(),
                bytes,
            });
        }
        Ok(_) => {}
        Err(e) => shared.send(VoiceEvent::Error { message: e.message }),
    }
}

async fn run_turn(
    session: Arc<Session>,
    rt: Arc<AiRuntime>,
    shared: Arc<Shared>,
    audio: Vec<i16>,
    sample_rate: u32,
    cancel: Cancel,
) {
    let heard = match rt.for_role(Role::Stt) {
        Ok((p, rc)) => {
            p.transcribe(&rc.model, wav(&audio, sample_rate), "utterance.wav", None)
                .await
        }
        Err(e) => Err(e),
    };
    let text = match heard {
        Ok(t) => t.trim().to_owned(),
        Err(e) => {
            shared.send(VoiceEvent::Error { message: e.message });
            shared.set(VoiceState::Listening);
            return;
        }
    };
    if cancel.is_cancelled() {
        return;
    }
    if text.is_empty() {
        shared.set(VoiceState::Listening);
        return;
    }
    shared.send(VoiceEvent::UserCaption { text: text.clone() });
    shared
        .transcript
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push((true, text.clone()));

    if let Some(fact) = remember_command(&text) {
        match session.capture_text(&fact, None, &jiff::Zoned::now()) {
            Ok(item) => shared.send(VoiceEvent::Captured {
                raw_id: item.meta.id,
            }),
            Err(e) => shared.send(VoiceEvent::Error {
                message: e.to_string(),
            }),
        }
        let ok = if lang_of(&text) == "fa" {
            "باشه، یادداشتش کردم."
        } else {
            "Got it, noted."
        };
        speak(&shared, &rt, ok, &cancel).await;
        if !shared.playing.load(Ordering::SeqCst) {
            shared.set(VoiceState::Listening);
        }
        return;
    }

    // Sentences go to a speaker task as they complete, so TTS starts with the first one.
    let (stx, mut srx) = unbounded_channel::<String>();
    let chunker = Mutex::new(SentenceChunker::default());
    let speaker = {
        let (shared, rt, cancel) = (shared.clone(), rt.clone(), cancel.clone());
        tokio::spawn(async move {
            while let Some(s) = srx.recv().await {
                speak(&shared, &rt, &s, &cancel).await;
            }
        })
    };
    let on_delta = |d: &str| {
        let done = chunker.lock().unwrap_or_else(|p| p.into_inner()).push(d);
        for s in done {
            let _ = stx.send(s);
        }
    };
    let history = shared
        .history
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let result = crate::ask::ask(
        session.library(),
        session.device(),
        &rt,
        &history,
        &text,
        None,
        &AskScope::All,
        true,
        None,
        &jiff::Zoned::now(),
        &cancel,
        Some(&on_delta),
    )
    .await;
    if let Some(rest) = chunker.lock().unwrap_or_else(|p| p.into_inner()).flush() {
        let _ = stx.send(rest);
    }
    drop(stx);
    let _ = speaker.await;
    if cancel.is_cancelled() {
        return;
    }
    match result {
        Ok(a) => {
            let spoken = speakable(&a.text.replace(HELP_MARKER, ""));
            shared
                .history
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(Turn {
                    question: text,
                    answer: spoken.clone(),
                });
            shared
                .transcript
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push((false, spoken));
            if a.needs_help {
                shared.send(VoiceEvent::NeedsHelp);
            }
        }
        Err(e) => shared.send(VoiceEvent::Error {
            message: e.to_string(),
        }),
    }
    if !shared.playing.load(Ordering::SeqCst) {
        shared.set(VoiceState::Listening);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(ms: u32, amp: f32) -> Vec<i16> {
        (0..(16 * ms))
            .map(|i| ((i as f32 * 0.17).sin() * amp) as i16)
            .collect()
    }

    #[test]
    fn vad_finds_utterances_and_ignores_clicks() {
        let mut vad = EnergyVad::new(VadConfig::default());
        let mut events = vec![];
        let mut feed = |pcm: Vec<i16>, events: &mut Vec<VadEvent>| {
            for f in pcm.chunks(320) {
                let e = vad.process(f, 20);
                if e != VadEvent::None {
                    events.push(e);
                }
            }
        };
        feed(tone(500, 30.0), &mut events); // room noise
        feed(tone(60, 9000.0), &mut events); // click: too short to start
        feed(tone(400, 30.0), &mut events);
        feed(tone(1000, 9000.0), &mut events); // speech
        feed(tone(800, 30.0), &mut events);
        assert_eq!(events, vec![VadEvent::SpeechStart, VadEvent::SpeechEnd]);
    }

    #[test]
    fn sentences_stream_out_early_and_cleanly() {
        let mut c = SentenceChunker::default();
        let mut out = vec![];
        for d in [
            "Sara is your cou",
            "sin. She moved to ",
            "Shiraz in 2024.5 no, ",
            "in May! ",
            "سلام؟ خوبی",
        ] {
            out.extend(c.push(d));
        }
        out.extend(c.flush());
        assert_eq!(
            out,
            vec![
                "Sara is your cousin.",
                "She moved to Shiraz in 2024.5 no, in May!",
                "سلام؟",
                "خوبی"
            ]
        );
        assert_eq!(
            speakable("See [[vaults/life/people/sara|Sara]] **now**."),
            "See Sara now."
        );
        assert_eq!(lang_of("سلام خوبی Sara"), "fa");
        assert_eq!(lang_of("Hello سارا"), "en");
        assert_eq!(
            remember_command("Remember that the gate code is 4412"),
            Some("the gate code is 4412".into())
        );
        assert_eq!(
            remember_command("یادت باشه که فردا دندان‌پزشکی دارم"),
            Some("فردا دندان‌پزشکی دارم".into())
        );
        assert_eq!(remember_command("What do I remember?"), None);
    }
}
