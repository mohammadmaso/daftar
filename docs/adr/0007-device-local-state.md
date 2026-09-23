# ADR-0007: Device-local state lives in `.git/daftar/`

* Status: accepted
* Date: 2026-09-23

## Context
Each device keeps state that must never sync: the job queue, the search/embedding cache, the device
identity, recent audio recordings (§3.1, 14-day retention).

## Decision
Store it under `<checkout>/.git/daftar/` (`state.sqlite`, `device.json`, `audio/`). Git never tracks
anything inside `.git/`, the state is naturally scoped to one checkout, and deleting the checkout
removes it. Obsidian ignores `.git/`.

## Consequences
No separate app-data bookkeeping per library; multi-library support is "one checkout per library".
