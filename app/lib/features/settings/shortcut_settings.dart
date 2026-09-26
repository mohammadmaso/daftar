import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/global_hotkey.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

final shortcutStatusProvider = FutureProvider.autoDispose<ShortcutStatus?>(
  (ref) => ref.watch(globalHotkeyProvider).shortcutStatus(),
);

/// Settings › Record from anywhere (desktop only): whether the system-wide shortcut works here,
/// and on GNOME without a shortcut API, a way to add it to GNOME's keyboard shortcuts.
class ShortcutSection extends ConsumerWidget {
  const ShortcutSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final status = ref.watch(shortcutStatusProvider).value;
    if (status == null) return const SizedBox.shrink();
    final (label, color) = switch (status.state) {
      ShortcutState.active when status.viaDesktopSettings => (
        l.shortcutOnDesktop,
        p.ink,
      ),
      ShortcutState.active => (l.shortcutOn, p.ink),
      ShortcutState.taken => (l.shortcutTaken, p.inkMuted),
      ShortcutState.unavailable => (l.shortcutUnavailable, p.inkMuted),
    };
    final linux = !kIsWeb && Platform.isLinux;
    return DSection(
      title: l.shortcutTitle,
      footer: linux ? l.shortcutFooterLinux : l.shortcutFooter,
      children: [
        DListRow(
          title: status.keys,
          subtitle: label,
          trailing: DIcon(
            status.state == ShortcutState.active ? DIcons.check : DIcons.close,
            size: 18,
            color: color,
          ),
        ),
        if (status.canInstall)
          DListRow(
            title: l.shortcutAddGnome,
            chevron: true,
            onTap: () async {
              final ok = await ref
                  .read(globalHotkeyProvider)
                  .installShortcut(l.recordVoiceNote);
              ref.invalidate(shortcutStatusProvider);
              if (context.mounted) {
                showNote(
                  context,
                  ok ? l.shortcutAdded(status.keys) : l.shortcutAddFailed,
                );
              }
            },
          ),
      ],
    );
  }
}
