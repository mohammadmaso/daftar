# ADR-0009: Raw captures are committed only once sealed

* Status: accepted
* Date: 2026-09-23

## Context
§3.1: raw files are immutable, but a voice/photo capture's body (transcript / vision description)
is produced after capture, possibly much later if offline.

## Decision
A capture file is written immediately (the UI confirms in < 100 ms). Text captures are sealed on
creation. Voice/photo captures are sealed when their transcribe/describe job writes the body — on the
capturing device, which alone holds the audio/original. `commit_local` skips unsealed captures (and
their assets), so no other device ever sees a raw file change content. After sealing only `status`
changes.

## Consequences
Offline voice notes appear on other devices once transcribed, not before (they could not be filed
earlier anyway). Scenario 1 still holds: they sync in capture order.
