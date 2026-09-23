# ADR-0004: Compile-time feature gates

* Status: accepted
* Date: 2026-09-23

## Context
§0.5 forbids "coming soon" screens; unfinished features must be hidden behind a compile-time flag.

## Decision
`lib/app/features.dart` holds `bool.fromEnvironment` constants. A destination/route is only
registered when its gate is on. Gates default to on once the feature's milestone acceptance passes;
the gate is then deleted. `DAFTAR_PREVIEW` additionally exposes developer surfaces (design gallery).

## Consequences
At M0 the shipped app consists of Settings (appearance, about). Tree-shaking removes gated code.
