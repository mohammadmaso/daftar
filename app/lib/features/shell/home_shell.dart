import 'package:flutter/material.dart' show Material;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/identity.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// Wide screens get a quiet navigation rail; phones get the destination full-screen.
const double kWideLayout = 900;

/// Owns app-lifetime behaviour for an open library: sync on foreground and periodically (§5.2).
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

  @override
  void initState() {
    super.initState();
    _sync = ref.read(syncControllerProvider.notifier);
    _life = AppLifecycleListener(
      onResume: () {
        _sync.syncNow();
        _sync.startPeriodic();
      },
      onPause: () => _sync.stopPeriodic(),
    );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _sync.syncNow();
      _sync.startPeriodic();
    });
  }

  @override
  void dispose() {
    _sync.stopPeriodic();
    _life.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width >= kWideLayout;
    if (!wide) return widget.child;
    final l = L10n.of(context);
    final p = context.palette;
    final locale = Localizations.localeOf(context);
    final items = [
      (DIcons.today, l.todayTitle, '/'),
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
                        selected: path == '/'
                            ? widget.location == '/'
                            : widget.location.startsWith(path),
                        onTap: () => context.go(path),
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

class _RailItem extends StatelessWidget {
  const _RailItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });
  final DIcons icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

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
            Text(
              label,
              style: context.type.label.copyWith(
                color: selected ? p.accent : p.ink,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
