# ADR-0024: Share, quick actions, shortcuts and the command palette (M9)

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

**Keyboard:** Ctrl/Cmd+K opens the palette, Ctrl/Cmd+N opens a new note, and Ctrl/Cmd+Shift+N
records. These work while the app has focus.

**Global (system-wide) hotkey: not added.** The usual package, `hotkey_manager`, had its last release
in May 2024, so it fails the maintenance rule. The in-app shortcuts cover a focused window. A
system-wide hotkey is left to the OS: GNOME/KDE custom shortcuts, macOS Shortcuts or a Windows
shortcut key can launch the app with `--record`, which the desktop builds could accept later.
