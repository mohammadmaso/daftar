//! Ask (§4.3): answers from the wiki through read-only tools, streamed, in the user's language,
//! citing pages and captures as wikilinks. Citations are checked against the repository before the
//! app shows them, so a link in an answer always opens something real.

use base64::Engine;
use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::agent::{self, AgentSpec, Cancel};
use crate::changeset::{self, Changeset, CommitInfo};
use crate::ledger::{LedgerEntry, OpType, Usage};
use crate::library::{Library, LocalDevice};
use crate::ops::OpError;
use crate::providers::{Message, OnDelta, Part, Role};
use crate::raw::{self, NewCapture, RawItem, RawKind};
use crate::runtime::AiRuntime;
use crate::tools::{self, OpContext, Scope};
use crate::{Result, pages, prompts, wiki};

/// What an Ask conversation may read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum AskScope {
    All,
    Vault(String),
    /// Story co-writer mode: only `vaults/stories/<story>/`.
    Story(String),
}

impl AskScope {
    fn prefix(&self) -> Option<String> {
        match self {
            AskScope::All => None,
            AskScope::Vault(v) => Some(format!("vaults/{v}/")),
            AskScope::Story(s) => Some(format!("vaults/stories/{s}/")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    /// As written in the answer (`vaults/health/labs/vitamin-d`, `sara`).
    pub target: String,
    pub label: Option<String>,
    /// Repo path it resolves to; `None` when the answer cited something that does not exist.
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    /// Markdown with wikilinks; links that do not resolve are turned into plain text.
    pub text: String,
    pub citations: Vec<Citation>,
    /// Show the "Talk to someone" card (§4.7).
    pub needs_help: bool,
    pub usage: Usage,
    pub model: String,
}

pub const HELP_MARKER: &str = "[[talk-to-someone]]";

/// Resolves every wikilink in `text`; unresolvable links become their label (or target) as plain
/// text so the reader never taps into nothing.
pub fn check_citations(lib: &Library, text: &str) -> Result<(String, Vec<Citation>)> {
    let resolver = pages::resolver(lib)?;
    let mut out = String::with_capacity(text.len());
    let mut cites: Vec<Citation> = Vec::new();
    let mut last = 0;
    for l in wiki::links(text) {
        if l.target == "talk-to-someone" {
            continue;
        }
        let path = resolver.resolve(&l.target);
        if path.is_none() {
            out.push_str(&text[last..l.start]);
            out.push_str(l.label.as_deref().unwrap_or(&l.target));
            last = l.end;
        }
        if !cites.iter().any(|c| c.target == l.target) {
            cites.push(Citation {
                target: l.target.clone(),
                label: l.label.clone(),
                path,
            });
        }
    }
    out.push_str(&text[last..]);
    Ok((out, cites))
}

#[allow(clippy::too_many_arguments)]
pub async fn ask(
    lib: &Library,
    dev: &LocalDevice,
    rt: &AiRuntime,
    history: &[Turn],
    question: &str,
    image: Option<(String, Vec<u8>)>,
    scope: &AskScope,
    voice: bool,
    external: Option<std::sync::Arc<dyn agent::ExternalTools>>,
    now: &Zoned,
    cancel: &Cancel,
    on_delta: OnDelta<'_>,
) -> std::result::Result<Answer, OpError> {
    let (provider, rc) = rt.for_role(if voice { Role::Voice } else { Role::Chat })?;
    let config = lib.config()?;
    let schema = std::fs::read_to_string(lib.path(crate::layout::SCHEMA_FILE)).unwrap_or_default();
    let today = now.strftime("%Y-%m-%d").to_string();
    let tz = now
        .time_zone()
        .iana_name()
        .unwrap_or("local time")
        .to_owned();
    let languages = "Persian (fa) and English (en)";
    let vault_list = config
        .active_vaults()
        .map(|v| {
            format!(
                "- `{}` ({} · {}): {}",
                v.id, v.title.en, v.title.fa, v.purpose
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let system = match scope {
        _ if voice => prompts::render(
            prompts::VOICE,
            &[
                ("today", &today),
                ("timezone", &tz),
                ("languages", languages),
                ("vaults", &vault_list),
                ("schema", &schema),
            ],
        ),
        AskScope::Story(story) => prompts::render(
            prompts::STORY_COWRITER,
            &[
                ("story", story),
                ("today", &today),
                ("languages", languages),
                ("schema", &schema),
            ],
        ),
        _ => {
            let vaults = config
                .active_vaults()
                .map(|v| {
                    format!(
                        "- `{}` ({} · {}): {}",
                        v.id, v.title.en, v.title.fa, v.purpose
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let scope_line = match scope {
                AskScope::Vault(v) => format!("only the `{v}` vault"),
                _ => "the whole wiki".to_owned(),
            };
            prompts::render(
                prompts::QUERY,
                &[
                    ("today", &today),
                    ("timezone", &tz),
                    ("languages", languages),
                    ("vaults", &vaults),
                    ("scope", &scope_line),
                    ("schema", &schema),
                ],
            )
        }
    };

    let mut ctx = OpContext::new(
        lib,
        now.clone(),
        crate::ids::new_id().to_string(),
        dev.id.clone(),
        Scope::ReadOnly,
        None,
    )?;
    ctx.read_prefix = scope.prefix();

    let mut messages = Vec::new();
    for t in history {
        messages.push(Message::user(t.question.clone()));
        messages.push(Message::assistant(t.answer.clone(), vec![]));
    }
    let mut m = Message::user(question.to_owned());
    if let Some((media_type, bytes)) = image {
        m.parts.push(Part::Image {
            media_type,
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
        });
    }
    messages.push(m);

    let spec = AgentSpec {
        model: rc.model.clone(),
        system,
        tools: tools::specs(false)
            .into_iter()
            .filter(|t| t.name != "review_add")
            .collect(),
        max_steps: if voice { 8 } else { 16 },
        max_tokens: rc
            .params
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(if voice { 600 } else { 2048 }) as u32,
        temperature: Some(0.3),
        context_chars: 300_000,
        params: rc
            .params
            .iter()
            .filter(|(k, _)| k.as_str() != "max_tokens")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        external,
    };
    let outcome = agent::run(
        &provider, &spec, &mut ctx, messages, None, 0, cancel, on_delta,
    )
    .await?;
    let marked = outcome.text.contains(HELP_MARKER);
    let text = outcome
        .text
        .lines()
        .filter(|l| l.trim() != HELP_MARKER)
        .collect::<Vec<_>>()
        .join("\n");
    let (text, citations) = check_citations(lib, text.trim())?;
    let mut usage = outcome.usage;
    usage.cost_usd = rt.config.cost(&rc.model, &usage);
    Ok(Answer {
        text,
        citations,
        needs_help: marked || crate::wellbeing::signals_crisis(question),
        usage,
        model: format!("{}/{}", rc.provider, rc.model),
    })
}

/// "Save to wiki" (§4.3): the question and answer become a `chat-answer` capture, filed by the
/// normal ingest pipeline (as an `answer` page or merged into an existing one).
pub fn answer_capture(
    lib: &Library,
    dev: &LocalDevice,
    now: &Zoned,
    question: &str,
    answer: &str,
    scope: &AskScope,
) -> Result<RawItem> {
    let text = format!(
        "**Question:** {}\n\n**Answer:** {}\n",
        question.trim(),
        answer.trim()
    );
    raw::create(
        lib,
        dev,
        now,
        RawKind::ChatAnswer,
        NewCapture {
            text,
            vault_hint: match scope {
                AskScope::Vault(v) => Some(v.clone()),
                AskScope::Story(_) => Some("stories".into()),
                AskScope::All => None,
            },
            assets: vec![],
        },
    )
}

/// Story mode: saves a draft as its own page under `drafts/`, never touching the user's chapters.
pub fn save_draft(
    lib: &Library,
    dev: &LocalDevice,
    now: &Zoned,
    story: &str,
    title: &str,
    text: &str,
) -> Result<String> {
    let story = wiki::sanitize_slug(story);
    let date = now.strftime("%Y-%m-%d").to_string();
    let mut slug = format!("{date}-{}", wiki::sanitize_slug(title));
    let dir = format!("vaults/stories/{story}/drafts");
    let mut n = 2;
    while lib.path(&format!("{dir}/{slug}.md")).exists() {
        slug = format!("{date}-{}-{n}", wiki::sanitize_slug(title));
        n += 1;
    }
    let path = format!("{dir}/{slug}.md");
    let meta: wiki::PageMeta = serde_json::from_value(serde_json::json!({
        "id": crate::ids::new_id().to_string(),
        "type": "draft",
        "vault": "stories",
        "title": {"en": format!("Draft · {title}"), "fa": format!("پیش‌نویس · {title}")},
        "summary": format!("Draft for {story}, {date}."),
        "created": date,
        "updated": date,
        "status": "active",
    }))?;
    let doc = wiki::render(&meta, &format!("# {title}\n\n{}\n", text.trim()));
    let mut cs = Changeset::default();
    cs.files.insert(path.clone(), Some(doc));
    cs.created.insert(path.clone());
    let entry = LedgerEntry {
        op_id: crate::ids::new_id().to_string(),
        op_type: OpType::SaveAnswer,
        sources: vec![],
        router: None,
        models: vec![],
        pages_created: vec![],
        pages_updated: vec![],
        claims_added: vec![],
        review_items: vec![],
        usage: Default::default(),
        started_at: crate::time::rfc3339(now),
        finished_at: crate::time::rfc3339(now),
        device: dev.id.clone(),
        summary: format!("Saved a draft for {story}: {title}"),
        note: None,
        forced_vault: None,
        replayed_from: None,
        reverts: None,
        rejected_claims: vec![],
    };
    changeset::commit(
        lib,
        dev,
        now,
        &cs,
        entry,
        CommitInfo {
            subject: format!("save-answer: draft for {story}"),
            source_path: None,
            log_title: title.to_owned(),
        },
    )?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::lib_in;

    #[test]
    fn unresolvable_citations_become_plain_text() {
        let (_d, lib) = lib_in();
        crate::fsutil::atomic_write(
            &lib.path("vaults/life/people/sara.md"),
            b"---\ntype: person\ntitle: { en: \"Sara\", fa: \"\xd8\xb3\xd8\xa7\xd8\xb1\xd8\xa7\" }\nsummary: \"s\"\n---\n\nx\n",
        )
        .unwrap();
        let (text, cites) = check_citations(
            &lib,
            "Sara is your cousin ([[sara|Sara]]). She lives in Shiraz ([[vaults/life/places/shiraz|Shiraz]]).",
        )
        .unwrap();
        assert_eq!(
            text,
            "Sara is your cousin ([[sara|Sara]]). She lives in Shiraz (Shiraz)."
        );
        assert_eq!(cites.len(), 2);
        assert_eq!(cites[0].path.as_deref(), Some("vaults/life/people/sara.md"));
        assert!(cites[1].path.is_none());
    }
}
