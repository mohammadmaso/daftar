# ADR-0027: Logo in the app, and a reading font per language

* Status: accepted
* Date: 2026-09-26

## Context

ADR-0005 left a placeholder mark in the app: the first letter of the product name in a ring. The app
icon (M9, `packaging/icon/make_icon.py`) is a ruled notebook page with a bookmark on an ink tile, so
the app and the home screen showed two different identities. Type was fixed to Inter and Vazirmatn;
people who read long pages asked for a serif for English and for a naskh face for Persian.

## Decision

* **Logo.** `Logo` (`lib/design/components/logo.dart`) paints the icon as a vector from the same
  1024-unit grid as `make_icon.py`, in fixed brand colours (`Brand.ink`, `Brand.paper`) in both
  themes. `Wordmark` puts the mark before the product name in the current reading face (or a face
  passed in). The mark replaces the Monogram on Today and Settings; the wordmark opens onboarding.
  `packaging/icon/make_logo.py` writes `logo.svg` and English sans, English serif and Persian
  lockups with the name as outlines, so they need no fonts to render.
* **Fonts.** Each language has its own device-local choice (ADR-0006), in Settings › Appearance:
  * English (`LatinFont`): Sans — Inter; Serif — Source Serif 4; Book — Literata;
    Legible — Atkinson Hyperlegible Next.
  * Persian (`PersianFont`): Vazirmatn; IBM Plex Sans Arabic; Noto Naskh Arabic; Markazi Text.
  * The UI script's face is the primary family and the other script's face the fallback, so mixed
    text uses both choices.
  * Each face carries a size factor from measured x-heights (Latin) and letter heights (Persian), so
    a choice changes the voice of the text, not its size. Markazi also gets tighter leading.
* **Bundling.** All faces are OFL and come from the google/fonts repository. `tools/fetch_fonts.py`
  instances the variable fonts at 400/500/600/700 (text optical size) and subsets the Latin-only
  faces to Latin and Latin Extended. Together they add about 3.8 MB of assets.

## Consequences

* The Monogram component is gone. ADR-0005's type line now describes the defaults.
* Goldens render every bundled family (`test/flutter_test_config.dart` loads them).
* Changing the icon means changing both `make_icon.py` and `Logo`; they share the grid numbers.
