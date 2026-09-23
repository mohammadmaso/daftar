# ADR-0005: Visual language — "calm paper, precise ink"

* Status: accepted
* Date: 2026-09-23

## Decision
* One accent: **ink teal** — `#22505E` on light paper `#F7F4EE`; `#86B6C2` on charcoal `#1B1B1A`.
  Chosen over indigo because it reads as ink rather than "tech", and stays distinct from the muted
  semantic colours (green = confirmed, amber = proposed, rust = critical).
* Surfaces separated by 1 px hairlines; no drop shadows; radii 10/12/14.
* Custom stroke icon set (`lib/design/icons.dart`); Material icon font is not bundled
  (`uses-material-design: false`).
* Type: Inter (Latin) and Vazirmatn (Persian), Persian ×1.12 size and 1.8 body leading.
* Every text/surface pair is checked for WCAG AA by `test/design/contrast_test.dart`.
* App monogram: the first letter of the product name in an ink ring (placeholder for the app icon
  delivered in M9).
