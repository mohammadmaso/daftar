import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Desktop integration (§8.2, ADR-0024, ADR-0027) over the `daftar/hotkey` channel:
///
/// * the system-wide record shortcut: ⌥⇧⌘N on macOS, Ctrl+Alt+Shift+N on Windows and Linux (an
///   X11 grab, or the desktop portal on Wayland), plus `daftar --record` on Linux;
/// * the tray / menu-bar icon (Linux, macOS) with Record, Import a file, Open and Quit;
/// * the compact recorder: on Linux and macOS the window shrinks to a small floating panel while
///   a shortcut recording runs, then comes back (Windows keeps recording in the full window).
///
/// The runners register everything natively and report presses here.
abstract class GlobalHotkey {
  /// The shortcut, the tray's Record item or `daftar --record`.
  Stream<void> get record;

  /// The tray's "Import a file" (null: pick one) or a file opened with the app (its path).
  Stream<String?> get import;

  /// Shrinks the window into the compact recorder. False where the platform has none.
  Future<bool> enterMiniRecorder();

  /// Restores the window; with [open] it is shown and focused, otherwise it goes back to how it
  /// was (hidden, if it was).
  Future<void> leaveMiniRecorder({required bool open});

  /// Shows that a recording is running on the tray / menu-bar icon.
  Future<void> setRecording(bool on);

  /// Tray menu labels in the app's language: `record`, `import`, `open`, `quit`.
  Future<void> setLabels(Map<String, String> labels);

  /// Whether the system-wide shortcut works here; null off the desktop.
  Future<ShortcutStatus?> shortcutStatus();

  /// Adds the shortcut to the desktop's own keyboard shortcuts (GNOME without the portal).
  Future<bool> installShortcut(String name);
}

enum ShortcutState {
  /// The shortcut works from any app.
  active,

  /// Another program holds the same keys.
  taken,

  /// This desktop offers no way for an app to register one.
  unavailable,
}

@immutable
class ShortcutStatus {
  const ShortcutStatus({
    required this.keys,
    required this.state,
    this.canInstall = false,
    this.viaDesktopSettings = false,
  });

  /// The keys as the platform writes them.
  final String keys;
  final ShortcutState state;

  /// The shortcut can be added to the desktop's keyboard settings (GNOME).
  final bool canInstall;

  /// It works because it is in the desktop's keyboard settings.
  final bool viaDesktopSettings;
}

class PlatformGlobalHotkey implements GlobalHotkey {
  PlatformGlobalHotkey() {
    if (!_desktop) return;
    _channel.setMethodCallHandler((call) async {
      switch (call.method) {
        case 'record':
          _record.add(null);
        case 'import':
          _import.add(call.arguments as String?);
      }
    });
    // Anything the runner received before Dart listened (a cold start) arrives after this.
    unawaited(_call('ready'));
  }

  static const _channel = MethodChannel('daftar/hotkey');
  static bool get _desktop =>
      !kIsWeb && (Platform.isLinux || Platform.isMacOS || Platform.isWindows);
  final _record = StreamController<void>.broadcast();
  final _import = StreamController<String?>.broadcast();

  /// Runners implement what their platform supports; the rest is a quiet no.
  Future<T?> _call<T>(String method, [Object? args]) async {
    if (!_desktop) return null;
    try {
      return await _channel.invokeMethod<T>(method, args);
    } on MissingPluginException {
      return null;
    } on PlatformException {
      return null;
    }
  }

  @override
  Stream<void> get record => _record.stream;

  @override
  Stream<String?> get import => _import.stream;

  @override
  Future<bool> enterMiniRecorder() async =>
      await _call<bool>('miniRecorder', true) ?? false;

  @override
  Future<void> leaveMiniRecorder({required bool open}) =>
      _call<void>('leaveMini', {'open': open});

  @override
  Future<void> setRecording(bool on) => _call<void>('recording', on);

  @override
  Future<ShortcutStatus?> shortcutStatus() async {
    if (!_desktop) return null;
    if (Platform.isMacOS) {
      return const ShortcutStatus(keys: '⌥⇧⌘N', state: ShortcutState.active);
    }
    const keys = 'Ctrl+Alt+Shift+N';
    if (Platform.isWindows) {
      return const ShortcutStatus(keys: keys, state: ShortcutState.active);
    }
    final m = await _call<Map<Object?, Object?>>('shortcut');
    if (m == null) return null;
    final kind = m['kind'];
    final installed = m['installed'] == true;
    final canInstall = m['canInstall'] == true;
    return ShortcutStatus(
      keys: keys,
      state: kind == 'x11' || kind == 'portal' || installed
          ? ShortcutState.active
          : kind == 'taken'
          ? ShortcutState.taken
          : ShortcutState.unavailable,
      canInstall: canInstall && !installed && kind != 'x11' && kind != 'portal',
      viaDesktopSettings: installed && kind != 'x11' && kind != 'portal',
    );
  }

  @override
  Future<bool> installShortcut(String name) async =>
      await _call<bool>('installShortcut', name) ?? false;

  Map<String, String>? _labels;

  @override
  Future<void> setLabels(Map<String, String> labels) async {
    if (mapEquals(labels, _labels)) return;
    _labels = Map.of(labels);
    await _call<void>('labels', labels);
  }
}

final globalHotkeyProvider = Provider<GlobalHotkey>(
  (ref) => PlatformGlobalHotkey(),
);
