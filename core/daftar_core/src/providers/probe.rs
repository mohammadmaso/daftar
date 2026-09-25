//! The "Test" button behind each model role (§9): one real, minimal call that exercises exactly
//! what the role needs (chat, image input, speech-to-text, text-to-speech, embeddings) and reports
//! how long it took.

use std::io::Cursor;
use std::time::Instant;

use base64::Engine;
use serde::{Deserialize, Serialize};

use super::{ChatRequest, DynProvider, Message, Part, ProviderResult, Role, RoleConfig};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeResult {
    pub latency_ms: u64,
    /// What came back, shortened: the reply, the transcript, "1.2 KB of audio", "1536 numbers".
    pub detail: String,
}

/// Runs the minimal call for `role` against `provider` with the role's model and parameters.
pub async fn probe(provider: &DynProvider, rc: &RoleConfig) -> ProviderResult<ProbeResult> {
    let started = Instant::now();
    let detail = match rc.role {
        Role::Stt => {
            let text = provider
                .transcribe(&rc.model, silent_wav(), "probe.wav", None)
                .await?;
            if text.trim().is_empty() {
                "no speech (as expected)".to_owned()
            } else {
                shorten(&text)
            }
        }
        Role::Tts => {
            let voice = rc
                .params
                .get("voice")
                .and_then(|v| v.as_str())
                .unwrap_or("alloy");
            let audio = provider.speech(&rc.model, voice, "OK").await?;
            format!("{:.1} KB of audio", audio.len() as f64 / 1024.0)
        }
        Role::Embedding => {
            let v = provider.embed(&rc.model, &["ok".to_owned()]).await?;
            format!("{} numbers", v.first().map_or(0, Vec::len))
        }
        Role::Vision => {
            let mut msg = Message::user("What colour is this square? Answer with one word.");
            msg.parts.push(Part::Image {
                media_type: "image/png".into(),
                data: base64::engine::general_purpose::STANDARD.encode(red_square_png()),
            });
            shorten(&chat(provider, rc, msg).await?)
        }
        Role::Router | Role::Ingest | Role::Chat | Role::Voice | Role::Reflect | Role::Lint => {
            shorten(
                &chat(
                    provider,
                    rc,
                    Message::user("Reply with the single word OK."),
                )
                .await?,
            )
        }
    };
    Ok(ProbeResult {
        latency_ms: started.elapsed().as_millis() as u64,
        detail,
    })
}

async fn chat(provider: &DynProvider, rc: &RoleConfig, msg: Message) -> ProviderResult<String> {
    let req = ChatRequest {
        model: rc.model.clone(),
        system: "You are a connectivity check. Answer as briefly as possible.".into(),
        messages: vec![msg],
        tools: vec![],
        // Reasoning models spend tokens before answering; keep headroom but stay cheap.
        max_tokens: 64,
        temperature: Some(0.0),
        json: false,
        params: rc.params.clone(),
    };
    Ok(provider.chat(&req, None).await?.text)
}

fn shorten(s: &str) -> String {
    let t = s.trim();
    let mut out: String = t.chars().take(60).collect();
    if t.chars().count() > 60 {
        out.push('…');
    }
    out
}

/// Half a second of 16 kHz mono silence as a WAV file.
fn silent_wav() -> Vec<u8> {
    let samples = 8_000u32;
    let data_len = samples * 2;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&16_000u32.to_le_bytes()); // sample rate
    w.extend_from_slice(&32_000u32.to_le_bytes()); // byte rate
    w.extend_from_slice(&2u16.to_le_bytes()); // block align
    w.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    w.resize(44 + data_len as usize, 0);
    w
}

fn red_square_png() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(32, 32, image::Rgb([200, 30, 30]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .expect("in-memory PNG encoding cannot fail");
    out.into_inner()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::providers::{MockProvider, mock::Script};

    fn rc(role: Role) -> RoleConfig {
        RoleConfig {
            role,
            provider: "p".into(),
            model: "m".into(),
            params: Default::default(),
        }
    }

    #[tokio::test]
    async fn chat_and_vision_probes_make_real_calls() {
        let mock = Arc::new(MockProvider::with_fn(|req| {
            crate::providers::ChatResponse {
                text: if req.messages[0].parts.len() == 2 {
                    "Red".into()
                } else {
                    "OK".into()
                },
                tool_calls: vec![],
                usage: Default::default(),
                stop: crate::providers::StopReason::EndTurn,
            }
        }));
        let p: DynProvider = mock.clone();
        assert_eq!(probe(&p, &rc(Role::Chat)).await.unwrap().detail, "OK");
        assert_eq!(probe(&p, &rc(Role::Vision)).await.unwrap().detail, "Red");
        let reqs = mock.requests.lock().unwrap();
        assert!(
            matches!(&reqs[1].messages[0].parts[1], Part::Image { media_type, .. } if media_type == "image/png")
        );
    }

    #[tokio::test]
    async fn stt_probe_sends_a_valid_wav() {
        let p: DynProvider = Arc::new(MockProvider::new(Script {
            transcripts: vec![String::new()],
            ..Default::default()
        }));
        let r = probe(&p, &rc(Role::Stt)).await.unwrap();
        assert_eq!(r.detail, "no speech (as expected)");
        let wav = silent_wav();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 16_000);
    }

    #[tokio::test]
    async fn unsupported_capability_is_a_readable_error() {
        let p: DynProvider = Arc::new(MockProvider::new(Script::default()));
        let e = probe(&p, &rc(Role::Tts)).await.unwrap_err();
        assert!(e.message.contains("text-to-speech"), "{}", e.message);
    }
}
