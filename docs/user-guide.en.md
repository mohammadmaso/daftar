# Daftar — user guide

*Persian version: [user-guide.fa.md](user-guide.fa.md)*

Daftar is a notebook that files itself. You talk, type or take a photo. Each capture is saved at
once to a private Git repository that you own. The AI models you choose then file it into a
bilingual wiki: your journal, the people in your life, your health, your work and your stories.
You can read, edit and correct everything, and every change the assistant makes can be undone.

---

## 1. Getting started

1. **Connect a private repository**, or tap **Start on this device for now** and connect one
   later (Settings › Repository › Connect…). Two ways to connect:
   * **HTTPS + token**: paste the repository URL and a personal access token with write access.
   * **SSH key**: the app makes a key for you. Add the public key to the repository as a deploy
     key with write access (GitHub: repository settings › Deploy keys).
2. **Name this device**, for example "Phone" or "Laptop". The name appears in the history of what
   was captured where.
3. **Set up AI** in Settings:
   * **Providers**: add one or more, choosing OpenAI-compatible, Anthropic or Gemini. Paste the API
     key and tap **Check** to see the models it offers. Keys are stored only in this device's
     secure storage. They never go into the repository, and each device asks for them once.
   * **Models**: choose a provider and model for each role. Roles without a model of their own
     use the **Chat** model. Tap **Test** to make a real, tiny call and see the latency.

| Role | What it does |
|---|---|
| Routing | decides which vault a capture belongs to |
| Filing | writes and updates wiki pages |
| Chat | answers your questions in Ask |
| Voice conversation | the voice you talk to in Talk |
| Photos | describes photos and reads text in them |
| Speech to text / Text to speech | transcribes voice notes; speaks answers |
| Reflection | daily summaries and weekly reviews |
| Upkeep | checks the wiki for contradictions and stale facts |

You can capture before any of this is set up. Captures wait, and Today says what they are waiting
for.

## 2. Capturing

At the bottom of **Today**:

* **Hold** the round button to record. Let go to save. Slide toward the start edge to cancel, or
  slide up to lock and record hands-free, then tap **Save** when you're done.
* **Tap** the round button to type a note.
* **Camera**: take a photo or pick one; the text in it is read too.
* **Auto** chip: by default the assistant decides where a capture belongs. Tap the chip to send
  just the next capture to a particular vault.

Captures are saved on the device in under a tenth of a second and work offline. They sync and
are filed when a connection and models are available.

**Other ways in**
* **Share** text or photos from another app into Daftar (Android).
* **Long-press the app icon** for Record, New note and Ask (Android and iPhone).
* **Record widget** on the Android home screen opens straight into recording.
* **Screen readers**: the record button has a "Record a voice note" action that starts recording
  without holding.

## 3. Today and filing

Today lists the day's captures, newest first. Under each one you see what happened to it:
* **Saved**: stored, not filed yet.
* **Waiting…**: for a transcription, a photo description or a model.
* **Filed**: for example "Filed to Life · Health — 4 pages updated, 1 claim to review". Tap it to
  see exactly what changed.
* **Couldn't file**: with the reason and a **Retry** button.

At the top, the sync line shows **Synced**, **n changes to sync**, **Syncing…**, **Offline**,
**Needs attention** or **On this device only**. Pull down to sync now.

## 4. The wiki

**Wiki** shows your vaults:
* **Life**: journal, people, places, goals, concerns and ideas.
* **Health**: your medical profile.
* **Mind**: moods, patterns and values.
* **Work**: projects and learning.
* **Stories**: fiction, one folder per story. Nothing in Stories is ever treated as a fact about
  you.

* **Search** is instant and works in both languages at once.
* Pages show links, callouts, tables and **claims** (statements about you). Each claim has a
  status:
  * *confirmed*: you said it.
  * *proposed*: the assistant inferred it and waits for you.
  * *superseded*: replaced by something newer.
* **Linked from** lists pages that link here, and **Nearby pages** shows a small graph.
* **Edit** opens the page as Markdown. Your own edits are saved as your commits, and the assistant
  never rewrites lines you wrote. If the page changed on another device while you were editing,
  you are told before anything is overwritten.
* The repository folder is plain Markdown, so you can open it in **Obsidian** or any editor. Changes
  made there are picked up when you come back to the app.
* **SCHEMA.md** at the top of the repository holds the rules the assistant follows (vaults, page
  types, citations, style). Edit it to change how filing works. The assistant may propose changes
  to it, but never edits it itself.

## 5. Review

Some things wait for you in **Review**: proposed claims, questions from the assistant, conflicts
between devices, and findings from the wiki check. For each one, **Confirm**, **Reject** or **Edit**
it (on a phone, swipe right to confirm and left to reject). A rejected claim is not proposed again
from the same evidence.

## 6. Activity and undo

**Activity** lists everything the assistant did. Open an entry to see the exact changes, then:
* **Undo**: removes that filing from the wiki. If later changes overlap, it is undone carefully the
  next time you're online. **Undo this undo** brings it back.
* **Move to vault…**: files the capture somewhere else instead.
* **Re-run with a note…**: files it again with a hint, for example "Sara is my cousin, not my
  colleague".

## 7. Ask

**Ask** answers from your wiki, and only from your wiki:
* Every answer links to the pages it used. If something isn't in your notes, it says so instead of
  guessing.
* The scope chip limits the search to **Everything**, one vault or one story.
* Attach a photo to ask about it.
* **Save to wiki** files a useful answer. In a story, **Save as draft** puts the text in that
  story's drafts.
* With outside tools connected (§9), Ask can use them too, and asks you first when a tool would
  change something.

## 8. Talk (voice mode)

Tap **Talk** in Ask for a hands-free conversation:
* Speak naturally. Interrupt at any moment and it stops talking and listens.
* Captions show both sides.
* Say "remember that…" to save a note.
* **Mute** pauses the microphone, and **End** saves the conversation so it gets filed.

## 9. Outside tools (MCP)

Settings › **Outside tools (MCP)** connects servers such as a calendar or a notes service. Server
settings sync between your devices, and each device signs in once. Credentials never leave the
device. For each server, choose whether its tools always ask first, run automatically when they
only read, or always run. Servers that run a local program work on desktop only.

## 10. Reflect

Settings › **Reflect**:
* **Daily reflection**: at the time you choose, a few cited lines at the end of the day's journal
  page.
* **Weekly review**: on the day you choose, a page in Life › reviews covering what happened,
  recurring themes and open concerns. A pattern about you needs at least three captures, and waits
  in Review until you confirm it.
* **Notifications**: always neutral ("Your daily reflection is ready"). They never say what a day
  was about.
* **Helpline country**: which help lines to show if you need them.
* **Check the wiki now**: finds broken links, orphan pages and captures that never got filed.
  Model checks for contradictions and stale facts follow. Findings go to Review, and nothing is
  changed without you.

If a capture or a day sounds like a crisis, the app shows **Talk to someone** with real help
lines. It never analyses that moment or puts it into a notification.

## 11. Keyboard (desktop)

| Keys | Does |
|---|---|
| Ctrl+K (⌘K on Mac) | Commands: search pages, ask, save a note, go anywhere |
| Ctrl+N (⌘N) | New note |
| Ctrl+Shift+N (⌘⇧N) | Record a voice note |

In Commands, type and press Enter. The first entry asks your question, the second saves the text
as a note, and matching pages follow. Use the arrow keys to choose.

## 12. Devices and sync

Install the app on each device and connect the same repository. Sync runs when the app opens,
after each capture, every few minutes while it is open, and roughly hourly in the background on
Android. Two devices can capture offline at the same time without conflicts. If you edit the same
lines of a page on two devices, both versions are kept in a callout and a card appears in Review.

## 13. Privacy

* No telemetry. The app talks only to your Git remote, your AI providers and your MCP servers.
* Keys, tokens and passwords live in the device's secure storage, never in the repository.
* A capture that contains a password or key is held back from syncing until you remove it.
* Your knowledge lives in the repository, as Markdown files you can read without the app.

## 14. When something is wrong

* **"Filing waits for a … model"**: set that role up in Settings › Models.
* **"Couldn't file"**: read the reason, fix it (often a key or a model name), then tap **Retry**.
* **"Needs attention"** on sync: open Settings › Repository for the message. It is usually an
  expired token or a missing deploy key.
* **Search misses something** after editing files elsewhere: Settings › Repository › **Rebuild
  search index**.
