import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// The system-wide "record" shortcut (§8.2): ⌥⇧⌘N on macOS, Ctrl+Alt+Shift+N on Windows. The
/// runners register it natively (MainFlutterWindow.swift, flutter_window.cpp) and bring the window
/// forward; this only reports the press. Linux has no portable global shortcut (Wayland), so it
/// never fires there.
abstract class GlobalHotkey {
  Stream<void> get record;
}

class PlatformGlobalHotkey implements GlobalHotkey {
  PlatformGlobalHotkey() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'record') _record.add(null);
    });
  }

  static const _channel = MethodChannel('daftar/hotkey');
  final _record = StreamController<void>.broadcast();

  @override
  Stream<void> get record => _record.stream;
}

final globalHotkeyProvider = Provider<GlobalHotkey>(
  (ref) => PlatformGlobalHotkey(),
);
