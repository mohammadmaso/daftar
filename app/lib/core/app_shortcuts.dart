import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:quick_actions/quick_actions.dart';

/// What a long press on the app icon offers (§8.1): "Record" opens straight into recording.
enum AppShortcut { record, note, ask }

/// App-icon quick actions on Android and iOS; a no-op elsewhere.
abstract class AppShortcuts {
  /// Sets the localized items and the handler. Safe to call again when the language changes.
  Future<void> set(
    Map<AppShortcut, String> titles,
    void Function(AppShortcut) onAction,
  );
}

class PlatformAppShortcuts implements AppShortcuts {
  final _plugin = const QuickActions();
  bool _initialized = false;

  static bool get supported =>
      !kIsWeb && (Platform.isAndroid || Platform.isIOS);

  @override
  Future<void> set(
    Map<AppShortcut, String> titles,
    void Function(AppShortcut) onAction,
  ) async {
    if (!supported) return;
    if (!_initialized) {
      _initialized = true;
      await _plugin.initialize((type) {
        final s = AppShortcut.values.where((v) => v.name == type).firstOrNull;
        if (s != null) onAction(s);
      });
    }
    await _plugin.setShortcutItems([
      for (final e in titles.entries)
        ShortcutItem(type: e.key.name, localizedTitle: e.value),
    ]);
  }
}

final appShortcutsProvider = Provider<AppShortcuts>(
  (ref) => PlatformAppShortcuts(),
);
