# ADR-0012: Vault pinning is a separate control, not a long-press

* Status: accepted
* Date: 2026-09-23

§8.1 assigns both "hold to record" and "long-press opens the vault picker" to the capture button;
the two gestures are indistinguishable. Hold-to-record is the primary, most frequent action, so it
keeps the button. Pinning a vault uses a small chip beside the capture button (tap → vault picker;
the pinned vault shows on the chip and applies to the next capture only).
