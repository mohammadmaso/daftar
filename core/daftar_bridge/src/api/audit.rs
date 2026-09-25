//! Activity, undo and the Review stack for the Flutter app (§7, §8.5).

use daftar_core::audit::{self, FileChange, LineKind};
use daftar_core::ledger::OpType;
use daftar_core::ops::IngestOptions;
use daftar_core::review::ReviewKind;
use daftar_core::review_ops::Resolution;
use daftar_core::session::UndoState;

use super::library::LibraryHandle;

pub enum OpKind {
    Ingest,
    Undo,
    Compensate,
    Review,
    SaveAnswer,
    Lint,
    Reflect,
}

pub struct RouteTarget {
    pub vault: String,
    pub reason: String,
    pub confidence: f64,
}

pub struct Operation {
    pub op_id: String,
    pub kind: OpKind,
    /// English one-line summary from the ledger (the app builds its own localized line).
    pub summary: String,
    pub started_at: String,
    pub device: String,
    pub vaults: Vec<String>,
    pub pages_created: u32,
    pub pages_updated: u32,
    pub claims_added: u32,
    pub sources: Vec<String>,
    pub reverts: Option<String>,
    pub replayed_from: Option<String>,
    pub undone: bool,
    pub note: Option<String>,
    pub forced_vault: Option<String>,
    pub models: Vec<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    /// Why the vaults were chosen (router targets with reasons and confidences).
    pub route: Vec<RouteTarget>,
}

pub enum DiffLineKind {
    Context,
    Added,
    Removed,
    Gap,
}

pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

pub enum PageChange {
    Added,
    Modified,
    Deleted,
}

pub struct PageDiff {
    pub path: String,
    pub change: PageChange,
    pub lines: Vec<DiffLine>,
}

pub enum UndoResult {
    Done,
    /// Later changes overlap; a compensating op runs with the AI jobs.
    Queued,
}

pub enum CardKind {
    Claim,
    Routing,
    Lint,
    SyncConflict,
    Schema,
    Question,
    HumanEdit,
}

pub struct ClaimInfo {
    pub id: String,
    pub text: String,
    pub status: String,
    pub confidence: Option<String>,
    pub sources: Vec<String>,
}

pub struct ReviewCardDto {
    pub id: String,
    pub kind: CardKind,
    pub created_at: String,
    pub op_id: Option<String>,
    /// Page the card is about (claims, conflicts), if any.
    pub page: Option<String>,
    pub claim: Option<ClaimInfo>,
    pub replaces: Option<ClaimInfo>,
    /// Routing cards: the vaults the capture went to and the capture id.
    pub vaults: Vec<String>,
    pub raw_id: Option<String>,
    /// Question cards and anything else: the text to show.
    pub text: Option<String>,
}

pub enum ReviewAction {
    Confirm,
    Reject,
    Dismiss,
}

fn err(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

fn op_dto(o: audit::OpSummary) -> Operation {
    let route = o
        .router
        .as_ref()
        .and_then(|r| r.get("targets"))
        .and_then(|t| t.as_array())
        .map(|ts| {
            ts.iter()
                .map(|t| RouteTarget {
                    vault: t["vault"].as_str().unwrap_or_default().to_owned(),
                    reason: t["reason"].as_str().unwrap_or_default().to_owned(),
                    confidence: t["confidence"].as_f64().unwrap_or(1.0),
                })
                .collect()
        })
        .unwrap_or_default();
    Operation {
        op_id: o.op_id,
        kind: match o.op_type {
            OpType::Ingest => OpKind::Ingest,
            OpType::RevertOp => OpKind::Undo,
            OpType::Compensate => OpKind::Compensate,
            OpType::Review => OpKind::Review,
            OpType::SaveAnswer => OpKind::SaveAnswer,
            OpType::Lint => OpKind::Lint,
            OpType::Reflect => OpKind::Reflect,
        },
        summary: o.summary,
        started_at: o.started_at,
        device: o.device,
        vaults: o.vaults,
        pages_created: o.pages_created as u32,
        pages_updated: o.pages_updated as u32,
        claims_added: o.claims_added as u32,
        sources: o.sources,
        reverts: o.reverts,
        replayed_from: o.replayed_from,
        undone: o.reverted,
        note: o.note,
        forced_vault: o.forced_vault,
        models: o.models,
        input_tokens: o.input_tokens,
        output_tokens: o.output_tokens,
        cost_usd: o.cost_usd,
        route,
    }
}

fn claim_dto(c: daftar_core::wiki::Claim) -> ClaimInfo {
    ClaimInfo {
        id: c.id,
        text: c.text,
        status: c.status,
        confidence: c.confidence,
        sources: c.sources,
    }
}

fn undo_dto(s: UndoState) -> UndoResult {
    match s {
        UndoState::Done(_) => UndoResult::Done,
        UndoState::Queued => UndoResult::Queued,
    }
}

impl LibraryHandle {
    /// Operations newest first; pass the last op id seen as `before` to page.
    pub fn activity(&self, limit: u32, before: Option<String>) -> anyhow::Result<Vec<Operation>> {
        Ok(self
            .session()
            .activity(limit as usize, before.as_deref())
            .map_err(err)?
            .into_iter()
            .map(op_dto)
            .collect())
    }

    pub fn operation(&self, op_id: String) -> anyhow::Result<Operation> {
        self.session().op(&op_id).map(op_dto).map_err(err)
    }

    pub fn operation_diff(&self, op_id: String) -> anyhow::Result<Vec<PageDiff>> {
        Ok(self
            .session()
            .op_diff(&op_id)
            .map_err(err)?
            .into_iter()
            .map(|f| PageDiff {
                path: f.path,
                change: match f.change {
                    FileChange::Added => PageChange::Added,
                    FileChange::Modified => PageChange::Modified,
                    FileChange::Deleted => PageChange::Deleted,
                },
                lines: f
                    .lines
                    .into_iter()
                    .map(|l| DiffLine {
                        kind: match l.kind {
                            LineKind::Context => DiffLineKind::Context,
                            LineKind::Added => DiffLineKind::Added,
                            LineKind::Removed => DiffLineKind::Removed,
                            LineKind::Gap => DiffLineKind::Gap,
                        },
                        text: l.text,
                    })
                    .collect(),
            })
            .collect())
    }

    /// Undo (and Exclude source): the capture stays in raw/ as excluded.
    pub fn undo(&self, op_id: String) -> anyhow::Result<UndoResult> {
        self.session().undo(&op_id, None).map(undo_dto).map_err(err)
    }

    pub fn move_to_vault(&self, op_id: String, vault: String) -> anyhow::Result<UndoResult> {
        let opts = IngestOptions {
            forced_vault: Some(vault),
            ..Default::default()
        };
        self.session()
            .undo(&op_id, Some(opts))
            .map(undo_dto)
            .map_err(err)
    }

    pub fn rerun_with_note(&self, op_id: String, note: String) -> anyhow::Result<UndoResult> {
        let opts = IngestOptions {
            note: Some(note.trim().to_owned()),
            ..Default::default()
        };
        self.session()
            .undo(&op_id, Some(opts))
            .map(undo_dto)
            .map_err(err)
    }

    /// Files an excluded capture again.
    pub fn include_capture(&self, raw_id: String) -> anyhow::Result<()> {
        self.session().include(&raw_id).map_err(err)
    }

    pub fn review_cards(&self) -> anyhow::Result<Vec<ReviewCardDto>> {
        Ok(self
            .session()
            .review_cards()
            .map_err(err)?
            .into_iter()
            .map(|c| {
                let p = &c.item.payload;
                let s = |k: &str| p.get(k).and_then(|v| v.as_str()).map(str::to_owned);
                ReviewCardDto {
                    id: c.item.id.clone(),
                    kind: match c.item.kind {
                        ReviewKind::Claim => CardKind::Claim,
                        ReviewKind::Routing => CardKind::Routing,
                        ReviewKind::Lint => CardKind::Lint,
                        ReviewKind::SyncConflict => CardKind::SyncConflict,
                        ReviewKind::Schema => CardKind::Schema,
                        ReviewKind::Question => CardKind::Question,
                        ReviewKind::HumanEdit => CardKind::HumanEdit,
                    },
                    created_at: c.item.created_at.clone(),
                    op_id: c.item.op_id.clone(),
                    page: s("page").or_else(|| s("path")),
                    claim: c.claim.map(claim_dto),
                    replaces: c.replaces.map(claim_dto),
                    vaults: p
                        .get("targets")
                        .and_then(|t| t.as_array())
                        .map(|ts| {
                            ts.iter()
                                .filter_map(|t| t["vault"].as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default(),
                    raw_id: s("raw_id"),
                    text: s("question").or_else(|| s("text")).or_else(|| s("message")),
                }
            })
            .collect())
    }

    /// `edited_text` only with `Confirm`: the user's corrected wording of the claim.
    pub fn resolve_review(
        &self,
        card_id: String,
        action: ReviewAction,
        edited_text: Option<String>,
    ) -> anyhow::Result<()> {
        let r = match action {
            ReviewAction::Confirm => Resolution::ConfirmClaim {
                text: edited_text.filter(|t| !t.trim().is_empty()),
            },
            ReviewAction::Reject => Resolution::RejectClaim,
            ReviewAction::Dismiss => Resolution::Dismiss,
        };
        self.session()
            .resolve_review(&card_id, r)
            .map(|_| ())
            .map_err(err)
    }
}
