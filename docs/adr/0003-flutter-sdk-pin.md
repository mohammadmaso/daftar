# ADR-0003: Flutter SDK pinned to 3.41.1 for now

* Status: accepted
* Date: 2026-09-23

## Context
The development machine has Flutter 3.41.1 / Dart 3.11. The latest riverpod (3.4), go_router (18)
and record (7.1) require Dart 3.12 / Flutter 3.44. Upgrading the owner's global SDK was out of scope.

## Decision
Pin CI to 3.41.1 and use the newest package versions compatible with it. Revisit at M9 (or earlier
if a needed fix lands only in newer packages): `flutter upgrade`, bump `FLUTTER_VERSION` in CI, bump
constraints, regenerate goldens.

## Consequences
Slightly older packages; no functional gaps identified for M0–M2.
