# ADR-0016: `config.json` merges structurally, never as text

* Status: accepted
* Date: 2026-09-25

## Context
From M2 the app edits `.daftar/config.json` (providers, model roles). Two devices changing settings
while apart produce a text conflict in JSON; the generic human-conflict path (§5.4 step 4) would wrap
the conflict in Markdown callouts and leave an unreadable config, which stops the library opening.

## Decision
* Local config changes are committed as `settings: shared settings changed` (not as an external
  `edit:`).
* On a merge conflict in `config.json`, `config::merge_json` does a three-way merge: objects key by
  key, lists of objects with an `id` or `role` element by element, a value changed on both sides takes
  this device's value, and a deletion wins unless the other side changed the element.

## Consequences
Settings never produce a Review card or break the library; in the rare case of the same field being
changed on both devices, the device that syncs second wins, which is what users expect from settings.
