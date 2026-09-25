import 'dart:async';

import 'package:flutter/material.dart' show Material;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/identity.dart';
import '../../core/app_shortcuts.dart';
import '../../core/incoming_shares.dart';
import '../../core/job_runner.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../capture/capture_request.dart';
import '../palette/command_palette.dart';

/// Wide screens get a quiet navigation rail; phones get the destination full-screen.
const double kWideLayout = 900;

/// Owns app-lifetime behaviour for an open library: sync on foreground and periodically (§5.2),
/// and the AI job runner while the app is in the foreground.
class HomeShell extends ConsumerStatefulWidget {
  const HomeShell({super.key, required this.location, required this.child});
  final String location;
  final Widget child;

  @override
  ConsumerState<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends ConsumerState<HomeShell> {
  late final AppLifecycleListener _life;
  late final SyncController _sync;
  late final JobRunner _jobs;
  StreamSubscription<void>? _shares;

  @override
  void initState() {
    super.initState();
    _sync = ref.read(syncControllerProvider.notifier);
    _jobs = ref.read(jobRunnerProvider.notifier);
    _life = AppLifecycleListener(
      onResume: () {
        _refreshIndex();
        _sync.syncNow();
        _sync.startPeriodic();
        _jobs.resume();
      },
      onPause: () {
        _sync.stopPeriodic();
        _jobs.pause();
      },
    );
    _shares = ref.read(incomingSharesProvider).arrived.listen((_) {
      _fileShares();
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _sync.syncNow();
      _sync.startPeriodic();
      _jobs.resume();
      _fileShares();
    });
  }

  /// Whatever was shared into the app becomes raw captures, filed like any other (§8.1).
  Future<void> _fileShares() async {
    final items = await ref.read(incomingSharesProvider).take();
    if (items.isEmpty) return;
    final lib = await ref.read(libraryProvider.future);
    if (lib == null) return;
    for (final item in items) {
      if (item.text case final t?) await lib.captureText(t);
      if (item.image case final b?) await lib.capturePhoto(b);
    }
    ref.read(revisionProvider.notifier).bump();
    _sync.changed();
    _jobs.kick();
    if (!mounted) return;
    context.go('/');
    showNote(context, L10n.of(context).sharedSaved(items.length));
  }

  /// Files may have changed outside the app (Obsidian, another editor) while it was away.
  Future<void> _refreshIndex() async {
    final lib = await ref.read(libraryProvider.future);
    if (lib == null) return;
    if (await lib.refreshIndex() > 0 && mounted) {
      ref.read(revisionProvider.notifier).bump();
    }
  }

  @override
  void dispose() {
    _sync.stopPeriodic();
    _jobs.pause();
    _shares?.cancel();
    _life.dispose();
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // Re-set on language change so the app-icon menu speaks the app's language.
    final l = L10n.of(context);
    ref.read(appShortcutsProvider).set({
      AppShortcut.record: l.recordVoiceNote,
      AppShortcut.note: l.paletteNewNote,
      AppShortcut.ask: l.askTitle,
    }, _onShortcut);
  }

  void _onShortcut(AppShortcut s) {
    if (!mounted) return;
    switch (s) {
      case AppShortcut.record:
        _capture(CaptureRequest.record);
      case AppShortcut.note:
        _capture(CaptureRequest.note);
      case AppShortcut.ask:
        context.go('/ask');
    }
  }

  void _capture(CaptureRequest r) {
    context.go('/');
    ref.read(captureRequestProvider.notifier).request(r);
  }

  /// Keyboard shortcuts (§8.2): ⌘ on Apple platforms, Ctrl elsewhere.
  Map<ShortcutActivator, VoidCallback> get _shortcuts {
    final mac = usesCommandKey;
    SingleActivator key(LogicalKeyboardKey k, {bool shift = false}) =>
        SingleActivator(k, meta: mac, control: !mac, shift: shift);
    return {
      key(LogicalKeyboardKey.keyK): () => showCommandPalette(context),
      key(LogicalKeyboardKey.keyN): () => _capture(CaptureRequest.note),
      key(LogicalKeyboardKey.keyN, shift: true): () =>
          _capture(CaptureRequest.record),
    };
  }

  @override
  Widget build(BuildContext context) => CallbackShortcuts(
    bindings: _shortcuts,
    child: Focus(autofocus: true, child: _layout(context)),
  );

  Widget _layout(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width >= kWideLayout;
    final l = L10n.of(context);
    final p = context.palette;
    final locale = Localizations.localeOf(context);
    final destinations = [
      (DIcons.today, l.todayTitle, '/'),
      (DIcons.wiki, l.wikiTitle, '/wiki'),
      (DIcons.ask, l.askTitle, '/ask'),
    ];
    bool isAt(String path) =>
        path == '/' ? widget.location == '/' : widget.location.startsWith(path);
    if (!wide) {
      final showBar =
          widget.location == '/' ||
          widget.location == '/wiki' ||
          widget.location == '/ask' ||
          widget.location == '/wiki/page';
      if (!showBar) return widget.child;
      return Column(
        children: [
          Expanded(child: widget.child),
          _BottomBar(
            items: [
              for (final (icon, label, path) in destinations)
                (icon, label, isAt(path), () => context.go(path)),
            ],
          ),
        ],
      );
    }
    final items = [
      ...destinations,
      (DIcons.review, l.reviewTitle, '/review'),
      (DIcons.activity, l.activityTitle, '/activity'),
      (DIcons.device, l.settingsTitle, '/settings'),
    ];
    return Material(
      color: p.paper,
      child: Row(
        children: [
          Container(
            width: 220,
            decoration: BoxDecoration(
              color: p.raised,
              border: BorderDirectional(end: BorderSide(color: p.hairline)),
            ),
            child: SafeArea(
              child: Padding(
                padding: const EdgeInsets.all(Space.x3),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Padding(
                      padding: const EdgeInsets.fromLTRB(
                        Space.x2,
                        Space.x2,
                        Space.x2,
                        Space.x6,
                      ),
                      child: Text(
                        AppIdentity.name(locale),
                        style: context.type.title.copyWith(color: p.accent),
                      ),
                    ),
                    for (final (icon, label, path) in items)
                      _RailItem(
                        icon: icon,
                        label: label,
                        selected: isAt(path),
                        onTap: () => context.go(path),
                      ),
                    const Spacer(),
                    _RailItem(
                      icon: DIcons.search,
                      label: l.commandPalette,
                      hint: usesCommandKey ? '⌘K' : 'Ctrl+K',
                      selected: false,
                      onTap: () => showCommandPalette(context),
                    ),
                  ],
                ),
              ),
            ),
          ),
          Expanded(child: widget.child),
        ],
      ),
    );
  }
}

/// Mobile destinations (§8.1): quiet labels, one accent for the current one.
class _BottomBar extends StatelessWidget {
  const _BottomBar({required this.items});
  final List<(DIcons, String, bool, VoidCallback)> items;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Material(
      color: p.raised,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(
            top: BorderSide(color: p.hairline, width: Stroke.hairline),
          ),
        ),
        child: SafeArea(
          top: false,
          child: Row(
            children: [
              for (final (icon, label, selected, onTap) in items)
                Expanded(
                  child: Pressable(
                    onPressed: onTap,
                    selected: selected,
                    semanticLabel: label,
                    radius: 0,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(vertical: Space.x2),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          DIcon(
                            icon,
                            size: 22,
                            color: selected ? p.accent : p.inkMuted,
                          ),
                          const SizedBox(height: 2),
                          Text(
                            label,
                            style: context.type.caption.copyWith(
                              color: selected ? p.accent : p.inkMuted,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _RailItem extends StatelessWidget {
  const _RailItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
    this.hint,
  });
  final DIcons icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  /// A keyboard shortcut shown at the end of the row.
  final String? hint;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Pressable(
      onPressed: onTap,
      selected: selected,
      radius: Radii.small,
      child: Container(
        padding: const EdgeInsets.symmetric(
          horizontal: Space.x3,
          vertical: Space.x2 + 2,
        ),
        decoration: BoxDecoration(
          color: selected ? p.accentSoft : null,
          borderRadius: BorderRadius.circular(Radii.small),
        ),
        child: Row(
          children: [
            DIcon(icon, size: 20, color: selected ? p.accent : p.inkMuted),
            const SizedBox(width: Space.x3),
            Expanded(
              child: Text(
                label,
                style: context.type.label.copyWith(
                  color: selected ? p.accent : p.ink,
                ),
              ),
            ),
            if (hint != null)
              Text(
                hint!,
                textDirection: TextDirection.ltr,
                style: context.type.caption.copyWith(color: p.inkMuted),
              ),
          ],
        ),
      ),
    );
  }
}
