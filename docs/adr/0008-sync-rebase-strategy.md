# ADR-0008: Sync integrates by rebasing local commits (in memory)

* Status: accepted
* Date: 2026-09-23

## Context
§5.4 asks for fetch → 3-way merge → special handling of AI op commits (discard + replay), human
conflicts (callouts + Review), and the double-ingest guard.

## Decision
After fetching, local unpushed commits are replayed one by one onto the remote head with libgit2
in-memory cherry-picks (`cherrypick_commit`), never touching the worktree until the end:

1. **Double-ingest guard first.** For every source ingested by both a local and a remote live op, the
   earlier ULID wins. Remote losers are reverted (`revert-op` commit with its own ledger entry) before
   local commits are replayed; local losers are skipped.
2. **Clean pick** → recommitted with the original author and message (trailers preserved).
3. **Conflicting AI op commit** → skipped and returned in `SyncOutcome.replays`; the job runner
   re-enqueues an ingest with `replayed_from`. Its raw item is `pending` again because its status
   change was part of the dropped commit.
4. **Conflicting human commit** → libgit2 3-way text merge; hunks it cannot merge become
   `> [!conflict] From <device> · <date>` callouts (remote text stays in place, local text in the
   callout) and a `sync_conflict` Review item. Modify/delete conflicts keep the modified side.
   Generated `index.md` conflicts take the remote side and are regenerated.
5. Safe checkout of the new tip, move the branch, push. Rejected push → start over (max 3).

`log/*.md` is merged with the union driver via `.gitattributes`; libgit2 honours it during
cherry-picks (verified by `two_devices_offline_captures_merge_without_conflicts`).

## Consequences
Linear history, no merge commits, each op remains exactly one commit (easy `git revert`). Rewriting
only ever touches unpushed commits.
