# ADR-0022: Reflect and Lint run as queued ops, idempotent across devices

* Status: accepted
* Date: 2026-09-25

## Context
§4.6/§4.7: reflections and lint are scheduled locally (no server), may run on several devices, must
be evidence-based, and must never surface unverifiable claims or sensitive notifications.

## Decision
* `reflect::due` decides what is due (today's summary after the daily time, a missed yesterday, the
  week ending on the configured weekday); `Session::schedule_reflections` queues `reflect` jobs.
  Each reflection is one `reflect` op whose ledger `note` is `daily:<date>` / `weekly:<week>`; a
  device skips a reflection another device already committed (checked again under the commit lock).
* The daily summary is a single JSON reply (no tools) written as the journal page's
  `## Day summary`, validated like any AI write (it must cite captures). The weekly review uses the
  agent with write tools; `pattern` claims need three distinct cited captures, enforced in code.
* Review pages follow the slug rule (§3.3): `vaults/life/reviews/2026-w38.md`.
* Crisis signals in the captures (or the model's `care` flag) mark the result for the help card and
  force the neutral notification "Your day, filed." Helpline defaults (Iran, a few countries, and
  findahelpline.com) live in `wellbeing::helplines` and need the owner's review before release.
* Lint = deterministic checks + model findings whose every quote is found on the named page (else
  dropped; a story never "contradicts" the user). Findings become `lint` Review cards keyed so the
  same problem is not queued twice; indexes are regenerated in the same op. Lint is queued after
  `lint_every_ingests` filings (default 25) or when the last pass is a week old.
* Changesets drop Review cards and `claims_added` for claims that no longer exist when the op
  commits (proposed and then removed in the same op).

## Consequences
Two devices opening the app in the evening produce one summary. The quality of reviews and lint on
real models is measured with `daftar eval` on the fixture month.
