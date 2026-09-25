import 'package:flutter/material.dart' show Material, RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/identity.dart';
import '../../core/dates.dart';
import '../../core/job_runner.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../ask/ask_screen.dart' show TalkToSomeoneCard;
import '../review/review_screen.dart' show reviewCardsProvider;
import '../settings/ai_settings.dart' show roleName;
import '../shell/home_shell.dart' show kWideLayout;
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
                  padding: const EdgeInsetsDirectional.fromSTEB(
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
                            const _FilingWaits(),
                            const _ReviewAndActivity(),
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
                if (ref.watch(helpCardProvider))
                  Padding(
                    padding: const EdgeInsetsDirectional.fromSTEB(
                      Space.x3,
                      Space.x2,
                      Space.x3,
                      Space.x2,
                    ),
                    child: TalkToSomeoneCard(
                      onClose: ref.read(helpCardProvider.notifier).close,
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
                              itemBuilder: (_, i) {
                                final c = items[items.length - 1 - i];
                                final op = c.filing?.opId;
                                return CaptureItem(
                                  capture: c,
                                  onTap: op == null
                                      ? null
                                      : () =>
                                            context.push('/activity/op?id=$op'),
                                );
                              },
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

/// §8.1: Review and Activity are reached from a small badge on Today.
class _ReviewAndActivity extends ConsumerWidget {
  const _ReviewAndActivity();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final n = ref.watch(reviewCardsProvider).value?.length ?? 0;
    final wide = MediaQuery.sizeOf(context).width >= kWideLayout;
    if (wide) return const SizedBox.shrink(); // the rail has both
    return Padding(
      padding: const EdgeInsets.only(top: Space.x1),
      child: Wrap(
        spacing: Space.x2,
        children: [
          if (n > 0)
            Pressable(
              onPressed: () => context.push('/review'),
              radius: Radii.pill,
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.x2,
                  vertical: 2,
                ),
                decoration: BoxDecoration(
                  color: p.accentSoft,
                  borderRadius: BorderRadius.circular(Radii.pill),
                ),
                child: Text(
                  l.toReview(n),
                  style: context.type.caption.copyWith(color: p.accent),
                ),
              ),
            ),
          Pressable(
            onPressed: () => context.push('/activity'),
            radius: Radii.pill,
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: Space.x1,
                vertical: 2,
              ),
              child: Text(
                l.activityTitle,
                style: context.type.caption.copyWith(color: p.inkMuted),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Shown when filing is paused because a model role is not set up (never an error).
class _FilingWaits extends ConsumerWidget {
  const _FilingWaits();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final role = ref.watch(jobRunnerProvider).waitingFor;
    if (role == null) return const SizedBox.shrink();
    final l = L10n.of(context);
    return Pressable(
      onPressed: () => context.push('/settings'),
      radius: Radii.small,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: Space.x1),
        child: Text(
          l.filingWaits(roleName(l, role)),
          style: context.type.caption.copyWith(color: context.palette.pending),
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
