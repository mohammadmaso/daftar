//! Reflect (§4.7): the daily summary on the journal page and the weekly review, scheduled locally
//! and run when due (at app open or in the foreground). Each run is one `reflect` op; a ledger note
//! (`daily:<date>` / `weekly:<yyyy>-W<ww>`) keeps two devices from writing the same reflection.
//!
//! Guardrail: reflections describe, never diagnose; when the captures suggest a crisis the result
//! is flagged for the "Talk to someone" card and the notification stays neutral.

use jiff::Zoned;
use jiff::civil::{Date, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::{self, AgentSpec, Cancel};
use crate::changeset::{self, CommitInfo};
use crate::ledger::{self, LedgerEntry, OpType};
use crate::library::{Library, LocalDevice};
use crate::ops::OpError;
use crate::providers::{ChatRequest, Message, Role};
use crate::runtime::AiRuntime;
use crate::tools::{self, OpContext, Scope};
use crate::{prompts, validate, wiki};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectSettings {
    #[serde(default = "yes")]
    pub daily: bool,
    /// Local time after which the day is summarised, "HH:MM".
    #[serde(default = "daily_time")]
    pub daily_time: String,
    #[serde(default = "yes")]
    pub weekly: bool,
    /// ISO weekday number, 1 = Monday … 7 = Sunday. Default Friday.
    #[serde(default = "weekly_day")]
    pub weekly_day: u8,
    #[serde(default = "yes")]
    pub notifications: bool,
    /// ISO country for the helpline card (`IR`, `US`, …); empty = international.
    #[serde(default)]
    pub helpline_country: String,
    /// A helpline the user prefers, shown first.
    #[serde(default)]
    pub helpline_custom: Option<String>,
    /// Lint after this many ingests (0 = only weekly and on demand).
    #[serde(default = "lint_every")]
    pub lint_every_ingests: u32,
}

fn yes() -> bool {
    true
}
fn daily_time() -> String {
    "21:00".into()
}
fn weekly_day() -> u8 {
    5
}
fn lint_every() -> u32 {
    25
}

impl Default for ReflectSettings {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("defaults")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Due {
    Daily {
        date: String,
    },
    Weekly {
        week: String,
        from: String,
        to: String,
    },
}

impl Due {
    pub fn key(&self) -> String {
        match self {
            Due::Daily { date } => format!("daily:{date}"),
            Due::Weekly { week, .. } => format!("weekly:{week}"),
        }
    }
}

fn done_keys(lib: &Library, since: Date) -> crate::Result<Vec<String>> {
    Ok(
        ledger::since(lib, since.year() as i32, since.month() as u8)?
            .into_iter()
            .filter(|e| e.op_type == OpType::Reflect)
            .filter_map(|e| e.note)
            .collect(),
    )
}

pub fn journal_path(date: Date) -> String {
    format!("vaults/life/journal/{:04}/{date}.md", date.year())
}

pub fn iso_week(date: Date) -> String {
    let w = date.iso_week_date();
    format!("{:04}-W{:02}", w.year(), w.week())
}

/// Reflections due at `now` that have not run yet on any device (§4.7): today's summary after the
/// daily time, yesterday's if it was missed, and last week's review once its weekday has come.
pub fn due(lib: &Library, settings: &ReflectSettings, now: &Zoned) -> crate::Result<Vec<Due>> {
    let today = now.date();
    let done = done_keys(lib, today.checked_sub(jiff::Span::new().days(40))?)?;
    let mut out = Vec::new();
    if settings.daily {
        let (h, m) = settings
            .daily_time
            .split_once(':')
            .and_then(|(h, m)| Some((h.parse::<i8>().ok()?, m.parse::<i8>().ok()?)))
            .unwrap_or((21, 0));
        let after = now.time() >= jiff::civil::time(h.clamp(0, 23), m.clamp(0, 59), 0, 0);
        let yesterday = today.yesterday()?;
        for (d, ready) in [(yesterday, true), (today, after)] {
            let key = format!("daily:{d}");
            if ready && !done.contains(&key) && lib.path(&journal_path(d)).exists() {
                out.push(Due::Daily {
                    date: d.to_string(),
                });
            }
        }
    }
    if settings.weekly {
        let wd = Weekday::from_monday_one_offset(settings.weekly_day.clamp(1, 7) as i8)?;
        // The review covers the seven days ending on the review day.
        let days_since =
            (today.weekday().to_monday_one_offset() - wd.to_monday_one_offset()).rem_euclid(7);
        let end = today.checked_sub(jiff::Span::new().days(days_since as i64))?;
        let ready = days_since > 0 || now.time() >= jiff::civil::time(18, 0, 0, 0);
        let from = end.checked_sub(jiff::Span::new().days(6))?;
        let week = iso_week(end);
        if ready && !done.contains(&format!("weekly:{week}")) {
            out.push(Due::Weekly {
                week,
                from: from.to_string(),
                to: end.to_string(),
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectOutcome {
    pub op_id: Option<String>,
    /// Local notification text (§4.7), at most three lines; `None` when notifications are off or
    /// nothing was written.
    pub notification: Option<String>,
    /// Show the "Talk to someone" card.
    pub needs_help: bool,
    pub page: Option<String>,
}

fn raw_bodies(lib: &Library, page_body: &str) -> String {
    let mut out = String::new();
    let mut seen = std::collections::BTreeSet::new();
    for l in wiki::links(page_body) {
        if l.target.starts_with("raw/")
            && seen.insert(l.target.clone())
            && let Ok(item) = crate::raw::read(lib, &format!("{}.md", l.target))
        {
            out.push_str(&format!(
                "\n<capture path=\"{}\" at=\"{}\">\n{}\n</capture>\n",
                item.path, item.meta.captured_at, item.body
            ));
        }
    }
    out
}

fn entry(
    dev: &LocalDevice,
    now: &Zoned,
    key: &str,
    summary: String,
    models: Vec<String>,
    usage: crate::ledger::Usage,
) -> LedgerEntry {
    LedgerEntry {
        op_id: crate::ids::new_id().to_string(),
        op_type: OpType::Reflect,
        sources: vec![],
        router: None,
        models,
        pages_created: vec![],
        pages_updated: vec![],
        claims_added: vec![],
        review_items: vec![],
        usage,
        started_at: crate::time::rfc3339(now),
        finished_at: crate::time::rfc3339(now),
        device: dev.id.clone(),
        summary,
        note: Some(key.to_owned()),
        forced_vault: None,
        replayed_from: None,
        reverts: None,
        rejected_claims: vec![],
    }
}

/// Replaces (or adds) the `## Day summary` section of a journal page.
fn with_day_summary(doc: &str, summary: &str) -> String {
    let secs = wiki::sections(doc);
    let lines: Vec<&str> = doc.lines().collect();
    let block = format!("## Day summary\n{}\n", summary.trim());
    match secs
        .iter()
        .find(|s| s.level == 2 && s.heading.trim().eq_ignore_ascii_case("Day summary"))
    {
        Some(s) => {
            let mut out: Vec<String> = lines[..s.start - 1].iter().map(|l| l.to_string()).collect();
            out.extend(block.lines().map(str::to_owned));
            out.push(String::new());
            out.extend(
                lines[s.end.min(lines.len())..]
                    .iter()
                    .map(|l| l.to_string()),
            );
            let mut t = out.join("\n").trim_end().to_owned();
            t.push('\n');
            t
        }
        None => format!("{}\n\n{block}", doc.trim_end()),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn daily(
    lib: &Library,
    dev: &LocalDevice,
    rt: &AiRuntime,
    date: Date,
    notifications: bool,
    now: &Zoned,
    commit_lock: &std::sync::Mutex<()>,
) -> Result<ReflectOutcome, OpError> {
    let key = format!("daily:{date}");
    let path = journal_path(date);
    let Ok(doc) = std::fs::read_to_string(lib.path(&path)) else {
        return Ok(ReflectOutcome {
            op_id: None,
            notification: None,
            needs_help: false,
            page: None,
        });
    };
    let page = wiki::parse(&path, &doc).map_err(OpError::from)?;
    let captures = raw_bodies(lib, &page.body);
    let crisis = crate::wellbeing::signals_crisis(&captures);
    let (p, rc) = rt.for_role(Role::Reflect)?;
    let schema = prompts::schema(lib);
    let system = prompts::render(
        prompts::REFLECT_DAILY,
        &[
            ("today", &date.to_string()),
            (
                "timezone",
                now.time_zone().iana_name().unwrap_or("local time"),
            ),
            ("languages", "Persian (fa) and English (en)"),
            ("schema", &schema),
            ("day", &format!("{doc}\n{captures}")),
        ],
    );
    let req = ChatRequest {
        model: rc.model.clone(),
        system,
        messages: vec![Message::user("DAILY REFLECT task. Write the day summary.")],
        tools: vec![],
        max_tokens: 1200,
        temperature: Some(0.3),
        json: true,
        params: rc.params.clone(),
    };
    let resp = p.chat(&req, None).await?;
    let v = crate::ops::extract_json(&resp.text)
        .ok_or_else(|| OpError::Permanent("The reflection could not be read.".into()))?;
    let summary = v["summary"].as_str().unwrap_or_default().trim().to_owned();
    if summary.is_empty() {
        return Err(OpError::Permanent("The reflection was empty.".into()));
    }
    let care = crisis || v["care"].as_bool().unwrap_or(false);
    let _g = commit_lock.lock().unwrap_or_else(|p| p.into_inner());
    if done_keys(lib, date)?.contains(&key) {
        return Ok(ReflectOutcome {
            op_id: None,
            notification: None,
            needs_help: care,
            page: Some(path),
        });
    }
    let current = std::fs::read_to_string(lib.path(&path)).map_err(crate::Error::from)?;
    let mut updated =
        wiki::parse(&path, &with_day_summary(&current, &summary)).map_err(OpError::from)?;
    updated.meta.updated = now.strftime("%Y-%m-%d").to_string();
    let mut cs = crate::changeset::Changeset::default();
    cs.files.insert(path.clone(), Some(updated.render()));
    let errors = validate::validate(lib, &cs, &Scope::Personal, &|p| {
        tools::human_lines_at_head(lib, p)
    })
    .errors;
    if !errors.is_empty() {
        return Err(OpError::Permanent(format!(
            "The day summary failed validation: {}",
            errors.join(" · ")
        )));
    }
    let mut usage = resp.usage;
    usage.cost_usd = rt.config.cost(&rc.model, &usage);
    let e = entry(
        dev,
        now,
        &key,
        format!("Summarised {date}"),
        vec![
            format!("{}/{}", rc.provider, rc.model),
            prompts::version(prompts::REFLECT_DAILY).to_owned(),
        ],
        usage,
    );
    let op_id = e.op_id.clone();
    changeset::commit(
        lib,
        dev,
        now,
        &cs,
        e,
        CommitInfo {
            subject: format!("reflect: day summary {date}"),
            source_path: None,
            log_title: format!("Day summary {date}"),
        },
    )?;
    // The notification never carries details of a heavy day (§4.7).
    let note = if care {
        "Your day, filed.".to_owned()
    } else {
        v["notification"]
            .as_str()
            .unwrap_or("Your day, filed.")
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(ReflectOutcome {
        op_id: Some(op_id),
        notification: notifications.then_some(note),
        needs_help: care,
        page: Some(path),
    })
}

/// Pattern claims need at least three distinct supporting captures (§4.7), checked in code.
pub fn pattern_errors(cs: &crate::changeset::Changeset) -> Vec<String> {
    let mut errors = Vec::new();
    for (path, content) in &cs.files {
        let Some(text) = content else { continue };
        let in_scope = path.starts_with("vaults/mind/") || path.starts_with("vaults/health/");
        if !in_scope {
            continue;
        }
        let is_pattern_page = wiki::parse(path, text).is_ok_and(|p| p.meta.kind == "pattern");
        for c in wiki::claims(text) {
            if !cs.claims_added.contains(&c.id) {
                continue;
            }
            let distinct: std::collections::BTreeSet<&String> =
                c.sources.iter().filter(|s| s.starts_with("raw/")).collect();
            if (is_pattern_page || c.status == "proposed") && distinct.len() < 3 {
                errors.push(format!(
                    "{path}: pattern claim ^{} cites {} capture(s); patterns need at least three distinct supporting captures, or leave it out.",
                    c.id,
                    distinct.len()
                ));
            }
        }
    }
    errors
}

#[allow(clippy::too_many_arguments)]
pub async fn weekly(
    lib: &Library,
    dev: &LocalDevice,
    rt: &AiRuntime,
    week: &str,
    from: Date,
    to: Date,
    notifications: bool,
    now: &Zoned,
    cancel: &Cancel,
    commit_lock: &std::sync::Mutex<()>,
) -> Result<ReflectOutcome, OpError> {
    let key = format!("weekly:{week}");
    let (provider, rc) = rt.for_role(Role::Reflect)?;
    let config = lib.config()?;
    let schema = prompts::schema(lib);
    let vault_list = config.vaults_for_prompt();
    let system = prompts::render(
        prompts::REFLECT_WEEKLY,
        &[
            ("week", week),
            ("from", &from.to_string()),
            ("to", &to.to_string()),
            ("today", &now.strftime("%Y-%m-%d").to_string()),
            (
                "timezone",
                now.time_zone().iana_name().unwrap_or("local time"),
            ),
            ("languages", "Persian (fa) and English (en)"),
            ("vaults", &vault_list),
            ("schema", &schema),
        ],
    );
    // The week's journal pages, and whether they carry signs of crisis.
    let mut days = Vec::new();
    let mut captures = String::new();
    let mut d = from;
    while d <= to {
        let p = journal_path(d);
        if let Ok(t) = std::fs::read_to_string(lib.path(&p)) {
            captures.push_str(&raw_bodies(lib, &t));
            days.push(p);
        }
        d = d.tomorrow().map_err(crate::Error::from)?;
    }
    if days.is_empty() {
        return Ok(ReflectOutcome {
            op_id: None,
            notification: None,
            needs_help: false,
            page: None,
        });
    }
    let crisis = crate::wellbeing::signals_crisis(&captures);
    // Page slugs are lower-case ASCII (§3.3), so `2026-W38` is filed as `2026-w38.md`.
    let page_path = format!("vaults/life/reviews/{}.md", week.to_lowercase());
    let mut ctx = OpContext::new(
        lib,
        now.clone(),
        crate::ids::new_id().to_string(),
        dev.id.clone(),
        Scope::Personal,
        None,
    )?;
    // Captures of the week may be cited without opening each one first.
    let mut d = from;
    while d <= to {
        for item in crate::raw::list_day(
            lib,
            crate::layout::Date {
                year: d.year() as i32,
                month: d.month() as u8,
                day: d.day() as u8,
            },
        )? {
            ctx.citable.insert(item.meta.id.clone(), item);
        }
        d = d.tomorrow().map_err(crate::Error::from)?;
    }
    let user = format!(
        "WEEKLY REFLECT task for {week}.\nWrite the review to `{page_path}`.\nJournal pages this week:\n{}\n{}",
        days.iter()
            .map(|p| format!("- {p}"))
            .collect::<Vec<_>>()
            .join("\n"),
        if crisis {
            "Recent entries contain signs of distress; follow the care rule.\n"
        } else {
            ""
        }
    );
    let spec = AgentSpec {
        model: rc.model.clone(),
        system,
        tools: tools::specs(true),
        max_steps: 40,
        max_tokens: 4096,
        temperature: Some(0.3),
        context_chars: 400_000,
        params: rc.params.clone(),
        external: None,
    };
    let validator = move |c: &OpContext<'_>| -> Vec<String> {
        let blame = |p: &str| tools::human_lines_at_head(lib, p);
        let mut e = validate::validate(lib, &c.cs, &c.scope, &blame).errors;
        e.extend(pattern_errors(&c.cs));
        e
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
    if !ctx.cs.files.contains_key(&page_path) {
        return Err(OpError::Permanent(
            "The weekly review page was not written.".into(),
        ));
    }
    let needs_help = crisis
        || outcome.text.contains(crate::ask::HELP_MARKER)
        || ctx
            .cs
            .files
            .get(&page_path)
            .and_then(|c| c.as_deref())
            .is_some_and(|t| t.contains(crate::ask::HELP_MARKER));
    let _g = commit_lock.lock().unwrap_or_else(|p| p.into_inner());
    if done_keys(lib, from)?.contains(&key) {
        return Ok(ReflectOutcome {
            op_id: None,
            notification: None,
            needs_help,
            page: Some(page_path),
        });
    }
    let mut usage = outcome.usage.clone();
    usage.cost_usd = rt.config.cost(&rc.model, &usage);
    let e = entry(
        dev,
        now,
        &key,
        format!("Wrote the weekly review {week}"),
        vec![
            format!("{}/{}", rc.provider, rc.model),
            prompts::version(prompts::REFLECT_WEEKLY).to_owned(),
        ],
        usage,
    );
    let op_id = e.op_id.clone();
    changeset::commit(
        lib,
        dev,
        now,
        &ctx.cs,
        e,
        CommitInfo {
            subject: format!("reflect: weekly review {week}"),
            source_path: None,
            log_title: format!("Weekly review {week}"),
        },
    )?;
    Ok(ReflectOutcome {
        op_id: Some(op_id),
        notification: notifications.then(|| "Your week, reviewed.".to_owned()),
        needs_help,
        page: Some(page_path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_summary_section_is_replaced_not_duplicated() {
        let doc = "---\ntype: journal-day\n---\n\n# 23 Sep\n\n## Entries\n### 10:00\nx\n\n## Day summary\nold\n";
        let once = with_day_summary(doc, "new one");
        assert!(once.contains("## Day summary\nnew one") && !once.contains("old"));
        let twice = with_day_summary(&once, "newer");
        assert_eq!(twice.matches("## Day summary").count(), 1);
        let fresh = with_day_summary("---\ntype: journal-day\n---\n\n## Entries\nx\n", "s");
        assert!(fresh.ends_with("## Day summary\ns\n"));
    }

    #[test]
    fn schedule() {
        let (_d, lib) = crate::testutil::lib_in();
        let s = ReflectSettings::default();
        let at = |t: &str| crate::testutil::zoned(t);
        // No journal pages: nothing daily is due; the weekly review waits for its day.
        assert!(
            due(&lib, &s, &at("2026-09-23T22:00:00+03:30[Asia/Tehran]"))
                .unwrap()
                .iter()
                .all(|d| matches!(d, Due::Weekly { .. }))
        );
        crate::fsutil::atomic_write(
            &lib.path(&journal_path("2026-09-23".parse().unwrap())),
            b"x",
        )
        .unwrap();
        let before = due(&lib, &s, &at("2026-09-23T20:00:00+03:30[Asia/Tehran]")).unwrap();
        assert!(!before.iter().any(|d| matches!(d, Due::Daily { .. })));
        let after = due(&lib, &s, &at("2026-09-23T21:30:00+03:30[Asia/Tehran]")).unwrap();
        assert!(after.contains(&Due::Daily {
            date: "2026-09-23".into()
        }));
        // Friday review: Wed 23 Sep → the previous week ending Fri 18 Sep.
        assert!(after.contains(&Due::Weekly {
            week: "2026-W38".into(),
            from: "2026-09-12".into(),
            to: "2026-09-18".into()
        }));
        let next_morning = due(&lib, &s, &at("2026-09-24T08:00:00+03:30[Asia/Tehran]")).unwrap();
        assert!(
            next_morning.contains(&Due::Daily {
                date: "2026-09-23".into()
            }),
            "a missed day is caught up"
        );
    }

    #[test]
    fn patterns_need_three_captures() {
        let mut cs = crate::changeset::Changeset {
            claims_added: vec!["c-1".into(), "c-2".into()],
            ..Default::default()
        };
        cs.files.insert(
            "vaults/mind/patterns/sleep.md".into(),
            Some("---\ntype: pattern\ntitle: { en: \"Sleep\", fa: \"خواب\" }\n---\n\n- Sleeps badly after late coffee (status:: proposed) (src:: [[raw/a|1]], [[raw/b|2]]) ^c-1\n- Low mood on Sundays (status:: proposed) (src:: [[raw/a|1]], [[raw/b|2]], [[raw/c|3]]) ^c-2\n".into()),
        );
        let e = pattern_errors(&cs);
        assert_eq!(e.len(), 1);
        assert!(e[0].contains("^c-1"));
    }
}
