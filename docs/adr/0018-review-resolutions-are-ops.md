# ADR-0018: Review resolutions are ledger ops

* Status: accepted
* Date: 2026-09-25

## Context
§3.4: rejected claims must be recorded in the op ledger so the same evidence does not re-propose
them; §7: everything should be auditable and reversible.

## Decision
* Confirming, editing, rejecting or dismissing a Review card is one `review` op (new `OpType::Review`)
  with its own commit, ledger entry and log line, so it appears in Activity and can be undone.
* Ledger entries gain `rejected_claims` (text, page, cited raw paths; omitted when empty). Ingest adds
  the claims rejected for the capture being filed to the model's instructions.
* A confirmed claim loses its `(confidence:: …)` field. Rejecting a superseding claim removes it and
  restores the superseded claim to `confirmed`.

## Consequences
Older app versions skip ledger files with the unknown `review` op type (they log a warning); nothing
else depends on those entries.
