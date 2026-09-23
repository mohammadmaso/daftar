---
schema_version: 1
---

# SCHEMA — how this wiki works · این ویکی چطور کار می‌کند

> This file is yours. Edit it to change how the assistant files and writes. The assistant reads it
> before every ingest, question and review, may *propose* changes to it through Review, and never
> edits it directly.
>
> این فایل مال شماست. با ویرایش آن، نحوه‌ی بایگانی و نوشتن دستیار را تغییر دهید. دستیار پیش از هر
> ثبت، پرسش و بازبینی آن را می‌خواند، ممکن است از طریق «بازبینی» تغییری *پیشنهاد* کند، ولی هرگز
> مستقیماً آن را ویرایش نمی‌کند.

## 1. Layers · لایه‌ها

- `raw/` — every capture, exactly as captured. Immutable. The source of truth.
  هر ثبت، دقیقاً همان‌طور که ثبت شد. تغییرناپذیر. مرجع اصلی.
- `vaults/` — the wiki, written and maintained by the assistant, readable and editable by you.
  ویکی، نوشته و نگه‌داری‌شده توسط دستیار؛ خواندنی و ویرایش‌پذیر برای شما.
- `SCHEMA.md` — these rules. · همین قواعد.
- `vaults/<vault>/index.md` — generated catalogue; never edited by hand or by the assistant.
- `log/<yyyy>-<mm>.md` — chronological record of everything the assistant did.

## 2. Vaults · خزانه‌ها

| Vault | Purpose | Typical pages |
|---|---|---|
| `life` · زندگی | journal, people, places, goals, concerns, ideas | `journal/<yyyy>/<yyyy-mm-dd>.md`, `people/<name>.md`, `places/`, `goals/`, `concerns/`, `ideas/`, `profile.md`, `reviews/` |
| `health` · سلامت | medical profile | `profile.md`, `conditions/`, `medications/`, `labs/`, `visits/`, `symptoms-log.md` |
| `mind` · ذهن | moods, patterns, values | `profile.md`, `patterns/`, `moods/<yyyy-mm>.md`, `values.md` |
| `work` · کار | projects, learning, professional notes | `projects/`, `topics/` |
| `stories` · داستان‌ها | fiction, one folder per story | `<story>/characters/`, `<story>/places/`, `<story>/timeline.md`, `<story>/threads/`, `<story>/chapters/`, `<story>/drafts/` |

One capture may touch several vaults (a journal entry about a headache updates `life` and proposes a
symptom in `health`).

**Isolation.** Nothing in `stories` is ever a fact about me, and nothing about me is ever written into
`stories`. A character's diagnosis is not my diagnosis.
**جداسازی.** هیچ چیزی در «داستان‌ها» واقعیتی درباره‌ی من نیست و هیچ چیزی درباره‌ی من در «داستان‌ها»
نوشته نمی‌شود.

## 3. Pages · صفحه‌ها

- File names are ASCII kebab-case slugs (`vitamin-d-deficiency.md`). Titles live in frontmatter and
  are always bilingual.
- Frontmatter fields: `id`, `type`, `vault`, `title: {en, fa}`, `aliases` (both languages),
  `summary` (one line, feeds the index), `sources` (raw ids), `created`, `updated`, `status`
  (`active | stale | archived | stub`).
- Page types and required sections:
  - `person` — **Who** (relationship to me), **Timeline**, **Notes**.
  - `concern` — **What**, **Why it matters**, **Timeline**, **Open questions**.
  - `topic`, `idea`, `pattern` — **Summary**, **Details**, **Related**.
  - `condition`, `medication` — **Summary**, **Claims**, **Timeline**.
  - `lab` — **Results** (table: date, value, unit, reference range, source), **Notes**.
  - `journal-day` — one `## HH:MM` section per capture, then **Day summary** (added in the evening).
  - `profile` — **Claims** grouped by theme.
  - `character`, `place`, `thread` (stories) — **Summary**, **Facts**, **Who knows what**, **Timeline**.
  - `answer` — saved answers to questions: **Question**, **Answer**, **Sources**.
- Prefer updating an existing page to creating a near-duplicate. Search first.

## 4. Links and citations · پیوندها و ارجاع‌ها

- Link entities with wikilinks: `[[sara|Sara]]`, `[[vitamin-d-deficiency|کمبود ویتامین D]]`. Link the
  first mention of a known entity in each section.
- Every factual sentence added by the assistant cites at least one raw source inline:
  `([[raw/2026/09/23/…|voice · 23 Sep]])`.
- Contradictions are never overwritten silently: the old statement is marked superseded and linked to
  the new one.

## 5. Claims about me · ادعاها درباره‌ی من

Anything asserted about me in `health`, `mind`, `life/profile.md` and `life/concerns/` is a claim:

```markdown
- Takes vitamin D 50,000 IU weekly (status:: confirmed) (src:: [[raw/…|voice · 3 Mar]]) ^c-01JAB9
- Sleeps worse after late coffee (status:: proposed) (confidence:: medium) (src:: [[raw/…]], [[raw/…]], [[raw/…]]) ^c-01JAC2
```

- Things I said directly may be `confirmed`. Anything inferred — patterns, interpretations, anything
  I did not literally say — is `proposed` and waits for me in Review.
- Patterns need at least three supporting entries.
- Rejected claims are not proposed again from the same evidence.
- هر چیزی که استنباط شده باشد «پیشنهادی» است تا خودم تأیید کنم.

## 6. Writing style · سبک نوشتن

- Concise, factual, specific. No filler, no motivational fluff, no therapy-speak.
- Keep my own words, in quotes, for emotional and personal content rather than paraphrasing them
  into clinical language.
- Write in the language of the source. Mixed Persian and English is fine.
- Describe; never diagnose or label me.
- کوتاه، دقیق، مشخص. حرف‌های خودم را، به‌خصوص درباره‌ی احساسات، با همان کلمات و در گیومه نگه دار.

## 7. Journal · روزنوشت

- Each personal capture becomes a `## HH:MM` section in `life/journal/<yyyy>/<yyyy-mm-dd>.md`: the gist
  in my language, links to the people/places/concerns involved, and a link to the raw capture.
- In the evening a short **Day summary** is added at the end of the day page.

## 8. Stories · داستان‌ها

- Each story has its own folder; retrieval and writing stay inside it.
- Track continuity: who knows what, and since when.
- My prose is never overwritten. Drafts go to `drafts/` as separate files.
