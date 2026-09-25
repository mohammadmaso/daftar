# ADR-0017: Undo is a revert scoped to pages; compensation runs as a queued job

* Status: accepted
* Date: 2026-09-25

## Context
§7 asks for undo as `git revert` of the op commit with a compensating op as fallback. An op commit
also contains its ledger entry, a log line, regenerated indexes and the capture's status line.
Reverting those literally would delete ledger history and log lines, and restore a stale index.

## Decision
* Undo computes the revert in memory (`Repository::revert_commit`) and takes only the op's changes to
  wiki pages and Review items. The ledger and log stay append-only (the undo adds its own entries),
  indexes are regenerated, and the capture's status is set explicitly: `excluded` after undo/exclude,
  `pending` before move/re-run, `ingested` again when an undo is undone.
* The result is committed through the changeset pipeline as a `revert-op` (one commit, one ledger
  entry, one log line). Local changes are committed first so files on disk match `HEAD`.
* If a page or Review item conflicts, a `compensate` job is queued (it needs the network and a
  model) and runs the compensate prompt through the normal agent loop, tools and validator. While
  compensating, `page_edit` removes the op's capture from a page's `sources` instead of adding it.
* Move to vault and re-run with note are "undo, then an ingest job with `forced_vault` / `note`";
  queue order guarantees the ingest runs after a queued compensation.

## Consequences
Undo is instant and offline whenever later ops did not touch the same lines, which is the common
case (undoing the latest filing). Overlapping undos wait for the network like any AI job.
