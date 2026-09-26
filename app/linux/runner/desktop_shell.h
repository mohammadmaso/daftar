#ifndef RUNNER_DESKTOP_SHELL_H_
#define RUNNER_DESKTOP_SHELL_H_

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>

// Desktop integration for Linux (ADR-0028), over the `daftar/hotkey` channel:
//
// * a tray icon (StatusNotifierItem through libayatana-appindicator, loaded at
//   run time so the app still starts where it is missing) with Record, Import,
//   Open and Quit;
// * the system-wide record shortcut Ctrl+Alt+Shift+N: an X11 key grab, or the
//   xdg-desktop-portal GlobalShortcuts API on Wayland;
// * the compact recorder: the window shrinks to a small always-on-top panel
//   while a hotkey recording runs, then comes back.
//
// Native → Dart: "record", "import" (argument: a path, or null to pick one).
// Dart → native: "ready", "miniRecorder" (bool on; returns true), "leaveMini"
// ({open: bool}), "recording" (bool), "labels" (map of menu labels),
// "shortcut" (returns {kind, canInstall, installed}) and "installShortcut"
// (name; adds a GNOME custom keybinding running `daftar --record`).
typedef struct _DesktopShell DesktopShell;

DesktopShell* desktop_shell_new(GtkApplication* app, GtkWindow* window,
                                FlView* view);

// Sends "record" to Dart (queued until Dart is ready).
void desktop_shell_record(DesktopShell* self);

// Sends "import" with `path` (nullptr: let the user pick) to Dart.
void desktop_shell_import(DesktopShell* self, const gchar* path);

// Shows and focuses the main window.
void desktop_shell_present(DesktopShell* self);

// Whether a tray icon is showing, so closing the window can hide it instead.
gboolean desktop_shell_has_tray(DesktopShell* self);

void desktop_shell_free(DesktopShell* self);

#endif  // RUNNER_DESKTOP_SHELL_H_
