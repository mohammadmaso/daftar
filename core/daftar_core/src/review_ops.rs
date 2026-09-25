//! Resolving Review cards (§8.5). Each resolution is one small `review` op: it edits the claim
//! line, removes the card and is recorded in the ledger (so it shows in Activity and can be undone).
//! Rejected claims are remembered so the same evidence does not propose them again (§3.4).

use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::changeset::{self, Changeset, CommitInfo};
use crate::ledger::{LedgerEntry, OpType, RejectedClaim};
use crate::library::{Library, LocalDevice};
use crate::review::{self, ReviewItem, ReviewKind};
use crate::{Error, Result, wiki};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Resolution {
    /// Confirm a proposed claim, optionally with the user's corrected wording.
    ConfirmClaim {
        text: Option<String>,
    },
    RejectClaim,
    /// "Right as it is", "resolved", "skip": the card goes away, nothing else changes.
    Dismiss,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewCard {
    pub item: ReviewItem,
    /// For claim cards: the claim as it stands in the page (it may have been edited since).
    pub claim: Option<wiki::Claim>,
    /// For superseding claims: the claim being replaced.
    pub replaces: Option<wiki::Claim>,
}

fn claim_in(text: &str, id: &str) -> Option<wiki::Claim> {
    wiki::claims(text).into_iter().find(|c| c.id == id)
}

/// Review items with the page state they refer to, oldest first. Claim cards whose claim no
/// longer exists (edited away by hand, undone) are dropped from the stack.
pub fn cards(lib: &Library) -> Result<Vec<ReviewCard>> {
    let mut out = Vec::new();
    for item in review::list(lib)? {
        if item.kind != ReviewKind::Claim {
            out.push(ReviewCard {
                item,
                claim: None,
                replaces: None,
            });
            continue;
        }
        let page = item.payload["page"].as_str().unwrap_or_default();
        let id = item.payload["claim_id"].as_str().unwrap_or_default();
        let text = std::fs::read_to_string(lib.path(page)).unwrap_or_default();
        let Some(claim) = claim_in(&text, id) else {
            continue;
        };
        let replaces = item.payload["supersedes"]
            .as_str()
            .and_then(|old| claim_in(&text, old));
        out.push(ReviewCard {
            item,
            claim: Some(claim),
            replaces,
        });
    }
    Ok(out)
}

/// Rewrites the `(status:: …)` field (and optionally the text) of claim `id` in `doc`.
fn rewrite_claim(doc: &str, id: &str, status: &str, text: Option<&str>) -> Option<String> {
    let marker = format!(" ^{id}");
    let mut found = false;
    let lines: Vec<String> = doc
        .lines()
        .map(|l| {
            if !l.trim_end().ends_with(&marker) {
                return l.to_owned();
            }
            found = true;
            let indent = &l[..l.len() - l.trim_start().len()];
            let mut out = l.to_owned();
            for s in ["proposed", "confirmed", "superseded"] {
                out = out.replace(&format!("(status:: {s})"), &format!("(status:: {status})"));
            }
            if status == "confirmed" {
                // A confirmed fact is no longer a guess.
                if let Some(start) = out.find(" (confidence:: ")
                    && let Some(end) = out[start + 1..].find(')')
                {
                    out.replace_range(start..start + 1 + end + 1, "");
                }
            }
            if let Some(t) = text.map(str::trim).filter(|t| !t.is_empty()) {
                let body = out
                    .trim_start()
                    .trim_start_matches("- ")
                    .trim_start_matches("* ");
                let first_field = body.find(" (").unwrap_or(body.len());
                let rest = &body[first_field..];
                out = format!("{indent}- {}{rest}", t.trim_end_matches('.'));
            }
            out
        })
        .collect();
    found.then(|| {
        let mut s = lines.join("\n");
        s.push('\n');
        s
    })
}

fn remove_claim(doc: &str, id: &str) -> String {
    let marker = format!(" ^{id}");
    let mut s = doc
        .lines()
        .filter(|l| !l.trim_end().ends_with(&marker))
        .collect::<Vec<_>>()
        .join("\n");
    s.push('\n');
    s
}

/// Drops the `(superseded_by:: …)` link from claim `id` (its replacement was rejected).
fn restore_superseded(doc: &str, id: &str) -> String {
    let marker = format!(" ^{id}");
    let mut s = doc
        .lines()
        .map(|l| {
            if !l.trim_end().ends_with(&marker) {
                return l.to_owned();
            }
            let mut out = l.replace("(status:: superseded)", "(status:: confirmed)");
            if let Some(start) = out.find(" (superseded_by:: ")
                && let Some(end) = out[start + 1..].find(")")
            {
                out.replace_range(start..start + 1 + end + 1, "");
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n");
    s.push('\n');
    s
}

/// Applies a resolution as one `review` op. Returns the new op id.
pub fn resolve(
    lib: &Library,
    dev: &LocalDevice,
    now: &Zoned,
    item_id: &str,
    resolution: Resolution,
) -> Result<String> {
    let item = review::list(lib)?
        .into_iter()
        .find(|r| r.id == item_id)
        .ok_or_else(|| Error::invalid("This card was already resolved."))?;
    let mut cs = Changeset::default();
    cs.files.insert(item.rel_path(), None);
    let mut rejected = Vec::new();
    let mut pages_updated = Vec::new();
    let summary;
    match (&item.kind, &resolution) {
        (ReviewKind::Claim, Resolution::ConfirmClaim { .. } | Resolution::RejectClaim) => {
            let page = item.payload["page"].as_str().unwrap_or_default().to_owned();
            let id = item.payload["claim_id"].as_str().unwrap_or_default();
            let doc = std::fs::read_to_string(lib.path(&page))
                .map_err(|_| Error::invalid("The page with this claim no longer exists."))?;
            let claim = claim_in(&doc, id)
                .ok_or_else(|| Error::invalid("This claim is no longer on its page."))?;
            let new_doc = match &resolution {
                Resolution::ConfirmClaim { text } => {
                    summary = format!("Confirmed: {}", text.as_deref().unwrap_or(&claim.text));
                    rewrite_claim(&doc, id, "confirmed", text.as_deref()).expect("claim found")
                }
                _ => {
                    summary = format!("Rejected: {}", claim.text);
                    rejected.push(RejectedClaim {
                        text: claim.text.clone(),
                        page: page.clone(),
                        sources: claim.sources.clone(),
                    });
                    let without = remove_claim(&doc, id);
                    match item.payload["supersedes"].as_str() {
                        Some(old) => restore_superseded(&without, old),
                        None => without,
                    }
                }
            };
            let mut p = wiki::parse(&page, &new_doc)?;
            p.meta.updated = now.strftime("%Y-%m-%d").to_string();
            cs.files.insert(page.clone(), Some(p.render()));
            pages_updated.push(page);
        }
        (_, Resolution::Dismiss) => {
            summary = match item.kind {
                ReviewKind::Routing => "Kept the filing as it was".to_owned(),
                ReviewKind::SyncConflict => "Marked a sync conflict resolved".to_owned(),
                _ => "Dismissed a review card".to_owned(),
            };
        }
        _ => return Err(Error::invalid("That action does not fit this card.")),
    }
    let entry = LedgerEntry {
        op_id: crate::ids::new_id().to_string(),
        op_type: OpType::Review,
        sources: vec![],
        router: None,
        models: vec![],
        pages_created: vec![],
        pages_updated,
        claims_added: vec![],
        review_items: vec![item.id.clone()],
        usage: Default::default(),
        started_at: crate::time::rfc3339(now),
        finished_at: crate::time::rfc3339(now),
        device: dev.id.clone(),
        summary: summary.clone(),
        note: None,
        forced_vault: None,
        replayed_from: None,
        reverts: None,
        rejected_claims: rejected,
    };
    let op_id = entry.op_id.clone();
    let subject = format!("review: {}", summary.chars().take(72).collect::<String>());
    changeset::commit(
        lib,
        dev,
        now,
        &cs,
        entry,
        CommitInfo {
            subject,
            source_path: None,
            log_title: summary,
        },
    )?;
    Ok(op_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\ntype: profile\nvault: health\ntitle: { en: \"Health\", fa: \"سلامت\" }\nsummary: \"s\"\n---\n\n## Symptoms\n- Takes vitamin D weekly (status:: superseded) (superseded_by:: [[#^c-2]]) (src:: [[raw/a|voice]]) ^c-1\n- Takes vitamin D daily (status:: proposed) (confidence:: high) (src:: [[raw/b|voice]]) ^c-2\n";

    #[test]
    fn claim_line_edits() {
        let c = rewrite_claim(
            DOC,
            "c-2",
            "confirmed",
            Some("Takes 1000 IU vitamin D daily."),
        )
        .unwrap();
        assert!(
            c.contains(
                "- Takes 1000 IU vitamin D daily (status:: confirmed) (src:: [[raw/b|voice]]) ^c-2"
            ),
            "{c}"
        );
        let r = restore_superseded(&remove_claim(DOC, "c-2"), "c-1");
        assert!(
            r.contains(
                "- Takes vitamin D weekly (status:: confirmed) (src:: [[raw/a|voice]]) ^c-1"
            ),
            "{r}"
        );
        assert!(!r.contains("^c-2"));
        assert!(rewrite_claim(DOC, "c-9", "confirmed", None).is_none());
    }
}
