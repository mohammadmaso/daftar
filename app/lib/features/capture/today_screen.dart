import 'package:flutter/material.dart' show Material, RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/identity.dart';
import '../../core/dates.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import 'capture_bar.dart';
import 'capture_item.dart';

class TodayScreen extends ConsumerWidget {
  const TodayScreen({super.key, this.showSettingsButton = true});

  final bool showSettingsButton;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final locale = Localizations.localeOf(context);
    final now = ref.watch(clockProvider)();
    final today = dateOnly(now);
    final captures = ref.watch(dayCapturesProvider(today));

    return Material(
      color: p.paper,
      child: SafeArea(
        bottom: false,
        child: Align(
          alignment: Alignment.topCenter,
          child: ConstrainedBox(
            constraints: const BoxConstraints(
              maxWidth: Space.measure + 2 * Space.x4,
            ),
            child: Column(
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(
                    Space.x4,
                    Space.x4,
                    Space.x2,
                    Space.x2,
                  ),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Semantics(
                              header: true,
                              child: Text(
                                l.todayTitle,
                                style: context.type.display,
                              ),
                            ),
                            Text(
                              longDate(now, locale.languageCode),
                              style: context.type.small.copyWith(
                                color: p.inkMuted,
                              ),
                            ),
                            const SizedBox(height: Space.x1),
                            const SyncBadge(),
                          ],
                        ),
                      ),
                      if (showSettingsButton)
                        Pressable(
                          onPressed: () => context.push('/settings'),
                          semanticLabel: l.settingsTitle,
                          radius: Radii.pill,
                          child: Padding(
                            padding: const EdgeInsets.all(Space.x1),
                            child: Monogram(name: AppIdentity.name(locale)),
                          ),
                        ),
                    ],
                  ),
                ),
                Expanded(
                  child: RefreshIndicator(
                    color: p.accent,
                    backgroundColor: p.raised,
                    onRefresh: () =>
                        ref.read(syncControllerProvider.notifier).syncNow(),
                    child: captures.when(
                      loading: () => const SizedBox.shrink(),
                      error: (e, _) => _Centered(
                        child: Text(
                          '$e',
                          style: context.type.small.copyWith(color: p.critical),
                        ),
                      ),
                      data: (items) => items.isEmpty
                          ? _Centered(
                              child: Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Text(
                                    l.emptyToday,
                                    style: context.type.body.copyWith(
                                      color: p.ink,
                                    ),
                                    textAlign: TextAlign.center,
                                  ),
                                  const SizedBox(height: Space.x1),
                                  Text(
                                    l.emptyTodayHint,
                                    style: context.type.small.copyWith(
                                      color: p.inkMuted,
                                    ),
                                    textAlign: TextAlign.center,
                                  ),
                                ],
                              ),
                            )
                          : ListView.separated(
                              padding: const EdgeInsets.symmetric(
                                horizontal: Space.x3,
                                vertical: Space.x2,
                              ),
                              // Newest first: what you just said is what you want to see.
                              itemCount: items.length,
                              separatorBuilder: (_, _) =>
                                  const DHairline(indent: 52 + Space.x1),
                              itemBuilder: (_, i) => CaptureItem(
                                capture: items[items.length - 1 - i],
                              ),
                            ),
                    ),
                  ),
                ),
                const DHairline(),
                const SafeArea(top: false, child: CaptureBar()),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Centered extends StatelessWidget {
  const _Centered({required this.child});
  final Widget child;

  // Scrollable so pull-to-refresh works on an empty day.
  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, c) => ListView(
      children: [
        SizedBox(
          height: c.maxHeight,
          child: Center(
            child: Padding(
              padding: const EdgeInsets.all(Space.x8),
              child: child,
            ),
          ),
        ),
      ],
    ),
  );
}

/// Tiny sync indicator (§5.2): calm when offline, specific when attention is needed.
class SyncBadge extends ConsumerWidget {
  const SyncBadge({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final s = ref.watch(syncControllerProvider);
    final (label, color) = switch (s.indicator) {
      SyncIndicator.synced => (l.syncSynced, p.inkMuted),
      SyncIndicator.localChanges => (l.syncLocal(s.pending), p.inkMuted),
      SyncIndicator.syncing => (l.syncSyncing, p.accent),
      SyncIndicator.offline => (l.syncOffline, p.inkMuted),
      SyncIndicator.needsAttention => (l.syncAttention, p.critical),
      SyncIndicator.noRemote => (l.syncNoRemote, p.inkMuted),
    };
    return Semantics(
      liveRegion: true,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: Space.x2),
          Text(label, style: context.type.caption.copyWith(color: color)),
        ],
      ),
    );
  }
}
