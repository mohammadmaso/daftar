# ADR-0010: Review items and ledger entries are one file each

* Status: accepted
* Date: 2026-09-23

* Review items live at `.daftar/review/<ulid>.json`; resolving deletes the file. Unique names mean
  devices never conflict and all devices share one queue.
* Ledger entries are never edited (§7). Relationships are recorded on the newer op (`reverts`,
  `replayed_from`); "reverted by" is derived by resolving reverts newest-first (so undo-of-undo
  re-activates the original op).
* A ledger file cannot contain the SHA of the commit it is part of; the commit is found through the
  `Op-Id:` trailer.
