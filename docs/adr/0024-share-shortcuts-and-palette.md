# ADR-0024: Share, quick actions, shortcuts, palette and background work (M9)

* Status: accepted
* Date: 2026-09-25

## Decisions

**One capture-request channel.** The command palette, keyboard shortcuts and app-icon quick actions
all ask the capture bar on Today to start a capture (`CaptureRequest.note | record | photo`). The
request is held until the bar takes it, so it survives a cold start and a navigation to Today.
"Record" from outside the bar starts already locked (tap to stop), which also gives screen-reader
users a way to record without hold-and-slide.

**Quick actions:** `quick_actions` 1.1.1 (flutter.dev, latest). It resolves
`quick_actions_android` 1.0.30, because 1.0.33 needs a newer Flutter (ADR-0003).

**Share into the app, Android:** handled in our own `MainActivity` (SEND and SEND_MULTIPLE for
`text/plain` and `image/*`) over a `daftar/share` method channel. It needs no plugin.
`receive_sharing_intent` 1.9.0 is maintained, but it supports only Swift Package Manager on iOS, and
this project builds with CocoaPods. Taking it would have broken the iOS build for a feature it can't
deliver there anyway, because iOS needs a Share Extension target.

**Share into the app, iOS:** a Share Extension target has to be added in Xcode with an App Group
shared with the app. That can't be done or verified without Xcode, so it is documented in
`docs/packaging.md` and listed as not yet done in PROGRESS.md.

**Home-screen widget, Android:** a native `AppWidgetProvider` with one "Record" button, built from
RemoteViews and the app's own accent colours and stroke microphone. It sends
`dev.daftar.daftar.RECORD` to `MainActivity`, which reaches Dart over the same `daftar/share`
channel. A single static button needs no plugin, so `home_widget` 0.10.0 isn't used. Its label is a
native string resource (`values/`, `values-fa/`), because the launcher renders it without Flutter.
The iOS widget needs a WidgetKit extension target in Xcode; it is documented like the Share
Extension.

**Keyboard:** Ctrl/Cmd+K opens the palette, Ctrl/Cmd+N opens a new note, and Ctrl/Cmd+Shift+N
records. These work while the app has focus.

**Global (system-wide) hotkey: not added.** The usual package, `hotkey_manager`, had its last release
in May 2024, so it fails the maintenance rule. The in-app shortcuts cover a focused window. A
system-wide hotkey is left to the OS: GNOME/KDE custom shortcuts, macOS Shortcuts or a Windows
shortcut key can launch the app with `--record`, which the desktop builds could accept later.

**Background work, Android:** `workmanager` 0.10.10 (September 2026, maintained; the brief names
it). An hourly periodic task, only when online, runs one pass: queue due reflections, sync, file
waiting captures, push, and post reflection notifications. The pass is a plain function
(`backgroundPass`), tested with fakes.

Two supporting changes make it safe:
* Android runs the task in the app's process, so `LibraryHandle.open` now joins a session already
  open on the same library instead of opening a second one. The queue and commit lock stay single.
* `Session::run_jobs` lets one caller drain the queue at a time. A concurrent call returns at once,
  because `claim` treats a `running` job as re-runnable after a crash.

The "heavy day" help flag is now a yes/no in device preferences, so a reflection that ran in the
background still brings up the card. Nothing about the day is stored with it.

iOS background refresh (BGTaskScheduler) needs capabilities and an identifier in Xcode. It is
documented, not done.
