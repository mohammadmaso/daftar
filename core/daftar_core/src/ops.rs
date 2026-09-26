//! AI operations run as jobs (§4): transcribe, describe, ingest. Each ingest is one commit + one
//! ledger entry through the changeset pipeline.

use std::collections::BTreeSet;

use base64::Engine;
use jiff::Zoned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::agent::{self, AgentError, AgentSpec, Cancel};
use crate::changeset::{self, CommitInfo};
use crate::config::{FICTION_VAULT, JOURNAL_VAULT};
use crate::ledger::{self, LedgerEntry, OpType, Usage};
use crate::library::{Library, LocalDevice};
use crate::providers::{ChatRequest, Message, Part, ProviderError, Role};
use crate::raw::{self, RawItem, RawKind, RawStatus};
use crate::review::{ReviewItem, ReviewKind};
use crate::runtime::AiRuntime;
use crate::tools::{self, OpContext, Scope};
use crate::{assets, prompts, validate};

/// Why an op did not complete; drives retry vs. permanent failure.
#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("{0}")]
    Permanent(String),
    #[error(transparent)]
    Core(#[from] crate::Error),
}

impl OpError {
    pub fn transient(&self) -> bool {
        match self {
            OpError::Provider(p) => p.transient(),
            _ => false,
        }
    }
}

impl From<AgentError> for OpError {
    fn from(e: AgentError) -> Self {
        match e {
            AgentError::Provider(p) => OpError::Provider(p),
            other => OpError::Permanent(other.to_string()),
        }
    }
}

// ─────────────────────────── shared prompt context ───────────────────────────

fn languages(_lib: &Library) -> String {
    "Persian (fa) and English (en)".into()
}

fn vault_lines(lib: &Library) -> crate::Result<String> {
    Ok(lib.config()?.vaults_for_prompt())
}

fn index_summaries(lib: &Library) -> crate::Result<String> {
    let c = lib.config()?;
    let mut out = String::new();
    for v in c.active_vaults() {
        let idx = std::fs::read_to_string(lib.path(&crate::layout::vault_index(&v.id)))
            .unwrap_or_default();
        let lines: Vec<&str> = idx
            .lines()
            .filter(|l| l.starts_with("- [["))
            .take(40)
            .collect();
        out.push_str(&format!(
            "### {}\n{}\n",
            v.id,
            if lines.is_empty() {
                "(empty)".to_owned()
            } else {
                lines.join("\n")
            }
        ));
    }
    Ok(out)
}

fn tz_name(now: &Zoned) -> String {
    now.time_zone()
        .iana_name()
        .unwrap_or("local time")
        .to_owned()
}

fn kind_label(k: RawKind) -> &'static str {
    match k {
        RawKind::Voice => "voice note",
        RawKind::Text => "note",
        RawKind::Photo => "photo",
        RawKind::VoiceConversation => "voice conversation",
        RawKind::ChatAnswer => "saved answer",
        RawKind::Import => "import",
    }
}

// ─────────────────────────── transcribe / describe ───────────────────────────

pub async fn transcribe(lib: &Library, rt: &AiRuntime, item: &RawItem) -> Result<(), OpError> {
    if !item.body.is_empty() {
        return Ok(()); // already sealed (idempotent after crash)
    }
    let audio = assets::audio_for(lib, item.id())
        .ok_or_else(|| OpError::Permanent("The recording is no longer on this device.".into()))?;
    let bytes = std::fs::read(&audio).map_err(crate::Error::from)?;
    let (p, rc) = rt.for_role(Role::Stt)?;
    let lang = rc
        .params
        .get("language")
        .and_then(Value::as_str)
        .filter(|l| *l != "auto");
    let name = audio
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.m4a")
        .to_owned();
    let text = p.transcribe(&rc.model, bytes, &name, lang).await?;
    let model = format!("{}/{}", rc.provider, rc.model);
    if text.trim().is_empty() {
        raw::seal(lib, &item.path, "_(no speech detected)_", Some(&model))?;
        raw::set_status(lib, &item.path, RawStatus::Excluded)?;
    } else {
        raw::seal(lib, &item.path, &text, Some(&model))?;
    }
    Ok(())
}

pub async fn describe(
    lib: &Library,
    rt: &AiRuntime,
    item: &RawItem,
    note: Option<&str>,
    now: &Zoned,
) -> Result<(), OpError> {
    if !item.body.is_empty() {
        return Ok(());
    }
    let asset = item
        .meta
        .assets
        .first()
        .ok_or_else(|| OpError::Permanent("This photo has no image file.".into()))?;
    let bytes = std::fs::read(lib.path(asset)).map_err(crate::Error::from)?;
    let (p, rc) = rt.for_role(Role::Vision)?;
    let note_line = note
        .filter(|n| !n.trim().is_empty())
        .map(|n| format!("\nThe user added this note: \"{n}\"\n"))
        .unwrap_or_default();
    let system = prompts::render(
        prompts::VISION_DESCRIBE,
        &[
            ("today", &now.strftime("%Y-%m-%d").to_string()),
            ("languages", &languages(lib)),
            ("note", &note_line),
        ],
    );
    let mut msg = Message::user("Describe this photo.");
    msg.parts.push(Part::Image {
        media_type: "image/jpeg".into(),
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
    });
    let req = ChatRequest {
        model: rc.model.clone(),
        system,
        messages: vec![msg],
        tools: vec![],
        max_tokens: 2000,
        temperature: Some(0.1),
        json: false,
        params: rc.params.clone(),
    };
    let resp = p.chat(&req, None).await?;
    let mut body = String::new();
    if let Some(n) = note.filter(|n| !n.trim().is_empty()) {
        body.push_str(&format!("> {}\n\n", n.trim()));
    }
    body.push_str(resp.text.trim());
    raw::seal(
        lib,
        &item.path,
        &body,
        Some(&format!("{}/{}", rc.provider, rc.model)),
    )?;
    Ok(())
}

// ─────────────────────────── routing ───────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub vault: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default = "one")]
    pub confidence: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub targets: Vec<Target>,
    #[serde(default)]
    pub is_fiction: bool,
    #[serde(default)]
    pub story: Option<String>,
    #[serde(default)]
    pub lang: Vec<String>,
    /// "router" | "hint" | "forced"
    #[serde(default)]
    pub decided_by: String,
}

/// First balanced JSON object in a model reply (tolerates prose or code fences around it).
pub fn extract_json(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let mut depth = 0;
    let mut in_str = false;
    let mut esc = false;
    for (i, c) in text[start..].char_indices() {
        if in_str {
            match c {
                '\\' if !esc => esc = true,
                '"' if !esc => in_str = false,
                _ => esc = false,
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return serde_json::from_str(&text[start..=start + i]).ok();
                }
            }
            _ => {}
        }
    }
    None
}

pub async fn route(
    lib: &Library,
    rt: &AiRuntime,
    item: &RawItem,
    forced: Option<&str>,
    now: &Zoned,
) -> Result<(Route, Usage, String), OpError> {
    let config = lib.config()?;
    let fixed = forced
        .map(|v| (v.to_owned(), "forced"))
        .or_else(|| item.meta.vault_hint.clone().map(|v| (v, "hint")));
    // A fixed personal vault needs no model call; a fixed `stories` still needs the story slug.
    // A pin to a vault archived since the capture falls back to the router.
    let fixed = fixed.filter(|(v, _)| config.active_vault(v).is_some());
    if let Some((v, by)) = &fixed
        && v != FICTION_VAULT
    {
        return Ok((
            Route {
                targets: vec![Target {
                    vault: v.clone(),
                    reason: format!("chosen by the user ({by})"),
                    confidence: 1.0,
                }],
                is_fiction: false,
                story: None,
                lang: item.meta.lang.clone(),
                decided_by: (*by).into(),
            },
            Usage::default(),
            String::new(),
        ));
    }
    let (p, rc) = rt.for_role(Role::Router)?;
    let system = prompts::render(
        prompts::ROUTER,
        &[
            ("today", &now.strftime("%Y-%m-%d").to_string()),
            ("timezone", &tz_name(now)),
            ("languages", &languages(lib)),
            ("vaults", &vault_lines(lib)?),
            ("index_summaries", &index_summaries(lib)?),
        ],
    );
    let user = format!(
        "ROUTER task. Capture ({}, {}):\n<capture>\n{}\n</capture>",
        kind_label(item.meta.kind),
        item.meta.captured_at,
        item.body
    );
    let req = ChatRequest {
        model: rc.model.clone(),
        system,
        messages: vec![Message::user(user)],
        tools: vec![],
        max_tokens: 600,
        temperature: Some(0.0),
        json: true,
        params: rc.params.clone(),
    };
    let resp = p.chat(&req, None).await?;
    let v = extract_json(&resp.text)
        .ok_or_else(|| OpError::Permanent("The router's answer could not be read.".into()))?;
    let mut r: Route = serde_json::from_value(v)
        .map_err(|_| OpError::Permanent("The router's answer had an unexpected shape.".into()))?;
    r.targets
        .retain(|t| config.active_vault(&t.vault).is_some());
    r.decided_by = "router".into();
    if let Some((v, by)) = fixed {
        // Pinned `stories`: keep the router's story slug, force fiction.
        r.targets = vec![Target {
            vault: v,
            reason: format!("chosen by the user ({by})"),
            confidence: 1.0,
        }];
        r.is_fiction = true;
        r.decided_by = by.into();
    }
    if r.is_fiction || r.targets.iter().any(|t| t.vault == FICTION_VAULT) {
        r.is_fiction = true;
        r.targets = vec![Target {
            vault: FICTION_VAULT.into(),
            reason: r
                .targets
                .first()
                .map(|t| t.reason.clone())
                .unwrap_or_default(),
            confidence: r
                .targets
                .iter()
                .map(|t| t.confidence)
                .fold(0.0, f64::max)
                .max(0.01),
        }];
        let slug = r
            .story
            .as_deref()
            .map(crate::wiki::sanitize_slug)
            .filter(|s| !s.starts_with("page-"))
            .unwrap_or_else(|| "untitled".into());
        r.story = Some(slug);
    }
    if r.targets.is_empty() {
        r.targets.push(Target {
            vault: JOURNAL_VAULT.into(),
            reason: "no vault fitted clearly; filed in the journal vault".into(),
            confidence: 0.3,
        });
    }
    Ok((r, resp.usage, format!("{}/{}", rc.provider, rc.model)))
}

// ─────────────────────────── ingest ───────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestOptions {
    pub replayed_from: Option<String>,
    pub forced_vault: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub op_id: String,
    pub summary: String,
    pub vaults: Vec<String>,
    pub pages_created: Vec<String>,
    pub pages_updated: Vec<String>,
    pub claims_proposed: usize,
    pub review_items: usize,
}

pub enum IngestOutcome {
    Filed(IngestResult),
    /// Nothing to do: already filed (possibly by another device) or excluded.
    Skipped(String),
}

#[allow(clippy::too_many_arguments)]
pub async fn ingest(
    lib: &Library,
    dev: &LocalDevice,
    rt: &AiRuntime,
    raw_id: ulid::Ulid,
    opts: &IngestOptions,
    now: &Zoned,
    cancel: &Cancel,
    commit_lock: &std::sync::Mutex<()>,
) -> Result<IngestOutcome, OpError> {
    let item = raw::find(lib, raw_id)?
        .ok_or_else(|| OpError::Permanent("The capture no longer exists.".into()))?;
    if item.meta.status != RawStatus::Pending {
        return Ok(IngestOutcome::Skipped(format!(
            "capture is {}",
            item.meta.status.as_str()
        )));
    }
    // Double-ingest guard (§5.4 step 5): another device may have filed it already.
    let live = ledger::live_ingests_by_source(&ledger::all(lib)?);
    if live.contains_key(&item.meta.id) {
        return Ok(IngestOutcome::Skipped("already filed".into()));
    }
    if item.meta.kind.needs_model_body() && item.body.is_empty() {
        return Err(OpError::Permanent(
            "This capture has not been transcribed yet.".into(),
        ));
    }

    let started = crate::time::rfc3339(now);
    let (route, route_usage, router_model) =
        route(lib, rt, &item, opts.forced_vault.as_deref(), now).await?;
    let scope = match (&route.is_fiction, &route.story) {
        (true, Some(s)) => Scope::Story(s.clone()),
        _ => Scope::Personal,
    };
    let op_id = crate::ids::new_id().to_string();
    let mut ctx = OpContext::new(
        lib,
        now.clone(),
        op_id.clone(),
        dev.id.clone(),
        scope.clone(),
        Some(item.clone()),
    )?;

    let threshold = ctx.config.routing_threshold;
    let best = route
        .targets
        .iter()
        .map(|t| t.confidence)
        .fold(0.0, f64::max);
    if route.decided_by == "router" && best < threshold {
        ctx.cs.review_items.push(ReviewItem::new(
            ReviewKind::Routing,
            &dev.id,
            now,
            Some(op_id.clone()),
            json!({"raw_id": item.meta.id, "targets": route.targets, "is_fiction": route.is_fiction}),
        ));
    }

    let (provider, rc) = rt.for_role(Role::Ingest)?;
    let raw_no_ext = item.path.trim_end_matches(".md").to_owned();
    let label = tools::source_label(&item);
    let isolation = match &scope {
        Scope::Story(s) => format!("This capture is FICTION for the story `{s}`. Write only inside `vaults/stories/{s}/` (characters/, places/, timeline.md, threads/, chapters/). Nothing here is about the user: no journal entry, no claims, no people pages outside the story."),
        _ => "This capture is about the user's real life. Never write into `vaults/stories/`. Story material mentioned in passing stays out of the personal pages.".into(),
    };
    let vault_ids = ctx
        .config
        .active_vaults()
        .map(|v| v.id.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let schema = prompts::schema(lib);
    let system = prompts::render(
        prompts::INGEST,
        &[
            ("today", &now.strftime("%Y-%m-%d").to_string()),
            ("timezone", &tz_name(now)),
            ("languages", &languages(lib)),
            ("vault_ids", &vault_ids),
            ("schema", &schema),
            ("isolation", &isolation),
            ("raw_path_no_ext", &raw_no_ext),
            ("source_label", &label),
        ],
    );
    let hhmm = item.meta.captured_at.get(11..16).unwrap_or("");
    let mut user = format!(
        "New capture to file.\nid: {}\npath: {}\nkind: {}\ncaptured_at: {} (journal time {hhmm})\nlanguages: {}\nrouted to: {}\n",
        item.meta.id,
        item.path,
        kind_label(item.meta.kind),
        item.meta.captured_at,
        item.meta.lang.join(", "),
        route
            .targets
            .iter()
            .map(|t| format!("{} ({:.2} — {})", t.vault, t.confidence, t.reason))
            .collect::<Vec<_>>()
            .join("; "),
    );
    if let Scope::Story(s) = &scope {
        user.push_str(&format!("story: {s}\n"));
    }
    if !item.meta.assets.is_empty() {
        user.push_str(&format!("assets: {}\n", item.meta.assets.join(", ")));
    }
    if let Some(n) = &opts.note {
        user.push_str(&format!("correction from the user (follow it): {n}\n"));
    }
    let rejected = ledger::rejected_for_source(&ledger::all(lib)?, &raw_no_ext);
    if !rejected.is_empty() {
        user.push_str("the user already rejected these claims from this capture; do not propose them again:\n");
        for c in &rejected {
            user.push_str(&format!("- {} ({})\n", c.text, c.page));
        }
    }
    user.push_str(&format!(
        "cite it as: [[{raw_no_ext}|{label}]]\n\n<capture>\n{}\n</capture>",
        item.body
    ));

    let spec = AgentSpec {
        model: rc.model.clone(),
        system,
        tools: tools::specs(true),
        max_steps: rc
            .params
            .get("max_steps")
            .and_then(Value::as_u64)
            .unwrap_or(40) as usize,
        max_tokens: rc
            .params
            .get("max_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(4096) as u32,
        temperature: Some(0.2),
        context_chars: 400_000,
        params: rc
            .params
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "max_steps" | "max_tokens"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        external: None,
    };
    let lib_ref = lib;
    let validator = move |c: &OpContext<'_>| -> Vec<String> {
        let blame = |p: &str| -> BTreeSet<usize> { tools::human_lines_at_head(lib_ref, p) };
        validate::validate(lib_ref, &c.cs, &c.scope, &blame).errors
    };
    let outcome = agent::run(
        &provider,
        &spec,
        &mut ctx,
        vec![Message::user(user)],
        Some(&validator),
        2,
        cancel,
        None,
    )
    .await?;

    let mut usage = outcome.usage.clone();
    usage.input_tokens += route_usage.input_tokens;
    usage.output_tokens += route_usage.output_tokens;
    usage.cost_usd = rt.config.cost(&rc.model, &outcome.usage);

    ctx.cs
        .raw_status
        .insert(item.path.clone(), RawStatus::Ingested);
    let (created, updated) = ctx.cs.page_changes();
    let vaults = ctx.cs.touched_vaults();
    let summary = summary_line(
        lib,
        &item,
        &vaults,
        &created,
        &updated,
        ctx.cs.claims_added.len(),
    );
    let mut models = vec![format!("{}/{}", rc.provider, rc.model)];
    if !router_model.is_empty() && !models.contains(&router_model) {
        models.push(router_model);
    }
    models.push(prompts::version(prompts::INGEST).to_owned());
    let entry = LedgerEntry {
        op_id: op_id.clone(),
        op_type: OpType::Ingest,
        sources: vec![item.meta.id.clone()],
        router: Some(serde_json::to_value(&route).unwrap_or_default()),
        models,
        pages_created: vec![],
        pages_updated: vec![],
        claims_added: vec![],
        review_items: vec![],
        usage,
        started_at: started,
        finished_at: crate::time::rfc3339(&Zoned::now().with_time_zone(now.time_zone().clone())),
        device: dev.id.clone(),
        summary: summary.clone(),
        note: opts.note.clone(),
        forced_vault: opts.forced_vault.clone(),
        replayed_from: opts.replayed_from.clone(),
        reverts: None,
        rejected_claims: vec![],
    };
    let n_pages = created.len() + updated.len();
    let subject = format!(
        "ingest: {} → {} ({} {})",
        kind_label(item.meta.kind),
        if vaults.is_empty() {
            "nothing filed".to_owned()
        } else {
            vaults.join(", ")
        },
        n_pages,
        if n_pages == 1 { "page" } else { "pages" }
    );
    let result = IngestResult {
        op_id: op_id.clone(),
        summary,
        vaults,
        pages_created: created,
        pages_updated: updated,
        claims_proposed: ctx
            .cs
            .review_items
            .iter()
            .filter(|r| r.kind == ReviewKind::Claim)
            .count(),
        review_items: ctx.cs.review_items.len(),
    };
    let log_title = item
        .body
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .to_owned();
    {
        let _g = commit_lock.lock().unwrap_or_else(|p| p.into_inner());
        // Re-check after waiting: a sync may have brought in another device's ingest meanwhile.
        if ledger::live_ingests_by_source(&ledger::all(lib)?).contains_key(&item.meta.id) {
            return Ok(IngestOutcome::Skipped(
                "already filed by another device".into(),
            ));
        }
        changeset::commit(
            lib,
            dev,
            now,
            &ctx.cs,
            entry,
            CommitInfo {
                subject,
                source_path: Some(item.path.clone()),
                log_title,
            },
        )?;
    }
    Ok(IngestOutcome::Filed(result))
}

fn summary_line(
    lib: &Library,
    item: &RawItem,
    vaults: &[String],
    created: &[String],
    updated: &[String],
    claims: usize,
) -> String {
    let config = lib.config().ok();
    let vname = |v: &str| {
        config
            .as_ref()
            .and_then(|c| c.vault(v))
            .map(|x| x.title.en.clone())
            .unwrap_or_else(|| v.to_owned())
    };
    let title = |p: &str| {
        crate::pages::read(lib, p)
            .map(|pg| pg.title("en").to_owned())
            .unwrap_or_else(|_| crate::wiki::slug_of(p).to_owned())
    };
    if vaults.is_empty() {
        return format!(
            "Read your {}; nothing needed filing.",
            kind_label(item.meta.kind)
        );
    }
    let vs = match vaults.len() {
        1 => vname(&vaults[0]),
        _ => format!(
            "{} and {}",
            vaults[..vaults.len() - 1]
                .iter()
                .map(|v| vname(v))
                .collect::<Vec<_>>()
                .join(", "),
            vname(&vaults[vaults.len() - 1])
        ),
    };
    let mut parts = Vec::new();
    if !updated.is_empty() {
        parts.push(format!(
            "updated {}",
            updated
                .iter()
                .take(4)
                .map(|p| title(p))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !created.is_empty() {
        parts.push(format!(
            "created {}",
            created
                .iter()
                .take(4)
                .map(|p| title(p))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if claims > 0 {
        parts.push(format!(
            "{claims} claim{}",
            if claims == 1 { "" } else { "s" }
        ));
    }
    format!(
        "Filed your {} to {vs}: {}",
        kind_label(item.meta.kind),
        parts.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::extract_json;

    #[test]
    fn json_extraction_tolerates_fences_and_braces_in_strings() {
        let t = "Here:\n```json\n{\"targets\": [{\"vault\": \"life\", \"reason\": \"a {weird} one\"}], \"is_fiction\": false}\n```";
        let v = extract_json(t).unwrap();
        assert_eq!(v["targets"][0]["reason"], "a {weird} one");
    }
}
