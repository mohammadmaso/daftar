//! Ask for the Flutter app (§4.3, §8.1): streamed answers with checked citations.

use daftar_core::agent::Cancel;
use daftar_core::ask::{AskScope, Turn};

use super::ai::ApiKey;
use super::library::LibraryHandle;
use crate::frb_generated::StreamSink;

pub enum AskScopeKind {
    All,
    Vault,
    Story,
}

pub struct AskScopeDto {
    pub kind: AskScopeKind,
    /// Vault id or story slug.
    pub id: Option<String>,
}

pub struct AskTurn {
    pub question: String,
    pub answer: String,
}

pub struct AskImage {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct AnswerCitation {
    pub target: String,
    pub label: Option<String>,
    /// `None`: the answer cited something that does not exist (shown as plain text).
    pub path: Option<String>,
}

#[derive(Clone)]
pub struct AskAnswer {
    pub text: String,
    pub citations: Vec<AnswerCitation>,
    pub needs_help: bool,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
}

#[derive(Clone)]
pub enum AskEventKind {
    /// Streamed text as it arrives (includes text the model writes between tool calls).
    Delta,
    Done,
    Failed,
}

#[derive(Clone)]
pub struct AskEvent {
    pub kind: AskEventKind,
    /// The delta, or the failure sentence.
    pub text: Option<String>,
    pub answer: Option<AskAnswer>,
}

fn event(kind: AskEventKind, text: Option<String>, answer: Option<AskAnswer>) -> AskEvent {
    AskEvent { kind, text, answer }
}

fn scope(s: AskScopeDto) -> AskScope {
    match (s.kind, s.id) {
        (AskScopeKind::Vault, Some(v)) => AskScope::Vault(v),
        (AskScopeKind::Story, Some(v)) => AskScope::Story(v),
        _ => AskScope::All,
    }
}

impl LibraryHandle {
    /// Streams `Delta`s, then exactly one `Done` or `Failed`.
    #[allow(clippy::too_many_arguments)]
    pub async fn ask(
        &self,
        history: Vec<AskTurn>,
        question: String,
        image: Option<AskImage>,
        scope_dto: AskScopeDto,
        api_keys: Vec<ApiKey>,
        sink: StreamSink<AskEvent>,
    ) -> anyhow::Result<()> {
        let s = self.session();
        let keys = api_keys
            .into_iter()
            .filter(|k| !k.key.is_empty())
            .map(|k| (k.provider_id, k.key))
            .collect();
        let rt = s
            .runtime(keys)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let history: Vec<Turn> = history
            .into_iter()
            .map(|t| Turn {
                question: t.question,
                answer: t.answer,
            })
            .collect();
        let delta_sink = sink.clone();
        let on_delta = move |t: &str| {
            let _ = delta_sink.add(event(AskEventKind::Delta, Some(t.to_owned()), None));
        };
        let res = s
            .ask(
                &rt,
                &history,
                &question,
                image.map(|i| (i.media_type, i.bytes)),
                &scope(scope_dto),
                None,
                &Cancel::default(),
                Some(&on_delta),
            )
            .await;
        let _ = sink.add(match res {
            Ok(a) => event(
                AskEventKind::Done,
                None,
                Some(AskAnswer {
                    text: a.text,
                    citations: a
                        .citations
                        .into_iter()
                        .map(|c| AnswerCitation {
                            target: c.target,
                            label: c.label,
                            path: c.path,
                        })
                        .collect(),
                    needs_help: a.needs_help,
                    model: a.model,
                    input_tokens: a.usage.input_tokens,
                    output_tokens: a.usage.output_tokens,
                    cost_usd: a.usage.cost_usd,
                }),
            ),
            Err(e) => event(AskEventKind::Failed, Some(e.to_string()), None),
        });
        Ok(())
    }

    /// "Save to wiki": files the question and answer through the normal pipeline.
    pub fn save_answer(
        &self,
        question: String,
        answer: String,
        scope_dto: AskScopeDto,
    ) -> anyhow::Result<String> {
        Ok(self
            .session()
            .save_answer(&question, &answer, &scope(scope_dto))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .meta
            .id)
    }

    /// Story mode: saves a draft under `drafts/`; returns its path.
    pub fn save_draft(&self, story: String, title: String, text: String) -> anyhow::Result<String> {
        self.session()
            .save_draft(&story, &title, &text)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}
