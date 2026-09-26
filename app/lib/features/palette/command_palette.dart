import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart' show Material;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/bidi.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../capture/capture_request.dart';

/// Ctrl/Cmd+K (§8.2): search pages, open places, capture and ask, all from the keyboard.
Future<void> showCommandPalette(BuildContext context) {
  final p = context.palette;
  return showGeneralDialog<void>(
    context: context,
    barrierDismissible: true,
    barrierLabel: L10n.of(context).close,
    barrierColor: p.ink.withValues(alpha: 0.18),
    transitionDuration: motion(context, Motion.quick),
    pageBuilder: (_, _, _) => CommandPalette(opener: context),
    transitionBuilder: (context, a, _, child) =>
        FadeTransition(opacity: a, child: child),
  );
}

/// True on Apple platforms, where the palette shortcut uses ⌘ rather than Ctrl.
bool get usesCommandKey =>
    defaultTargetPlatform == TargetPlatform.macOS ||
    defaultTargetPlatform == TargetPlatform.iOS;

@immutable
class PaletteEntry {
  const PaletteEntry(this.icon, this.title, this.run, {this.subtitle});
  final DIcons icon;
  final String title;
  final String? subtitle;
  final void Function(BuildContext context, ProviderContainer app) run;
}

class CommandPalette extends ConsumerStatefulWidget {
  const CommandPalette({super.key, required this.opener});

  /// The page the palette was opened from; commands act there once the palette is closed.
  final BuildContext opener;

  @override
  ConsumerState<CommandPalette> createState() => _CommandPaletteState();
}

class _CommandPaletteState extends ConsumerState<CommandPalette> {
  final _query = TextEditingController();
  List<SearchHit> _hits = const [];
  int _selected = 0;
  int _searchSeq = 0;

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  Future<void> _search(String q) async {
    final seq = ++_searchSeq;
    setState(() => _selected = 0);
    if (q.trim().isEmpty) {
      setState(() => _hits = const []);
      return;
    }
    final lib = await ref.read(libraryProvider.future);
    final hits = await lib?.search(q.trim(), limit: 8) ?? const <SearchHit>[];
    if (mounted && seq == _searchSeq) setState(() => _hits = hits);
  }

  /// Everything the palette can do, filtered by the query.
  List<PaletteEntry> _entries(L10n l, String lang) {
    final q = _query.text.trim();
    void capture(BuildContext c, ProviderContainer app, CaptureRequest r) {
      c.go('/');
      app.read(captureRequestProvider.notifier).request(r);
    }

    final places = [
      (DIcons.today, l.todayTitle, '/'),
      (DIcons.wiki, l.wikiTitle, '/wiki'),
      (DIcons.ask, l.askTitle, '/ask'),
      (DIcons.review, l.reviewTitle, '/review'),
      (DIcons.activity, l.activityTitle, '/activity'),
      (DIcons.device, l.settingsTitle, '/settings'),
    ];
    final actions = [
      PaletteEntry(
        DIcons.edit,
        l.paletteNewNote,
        (c, app) => capture(c, app, CaptureRequest.note),
      ),
      PaletteEntry(
        DIcons.mic,
        l.recordVoiceNote,
        (c, app) => capture(c, app, CaptureRequest.record),
      ),
      PaletteEntry(
        DIcons.camera,
        l.paletteTakePhoto,
        (c, app) => capture(c, app, CaptureRequest.photo),
      ),
      PaletteEntry(DIcons.speaker, l.talkMode, (c, _) => c.push('/voice')),
      for (final (icon, name, path) in places)
        PaletteEntry(icon, l.paletteGoTo(name), (c, _) => c.go(path)),
    ];
    final needle = q.toLowerCase();
    return [
      if (q.isNotEmpty) ...[
        PaletteEntry(
          DIcons.ask,
          l.paletteAsk(q),
          (c, _) =>
              c.go(Uri(path: '/ask', queryParameters: {'q': q}).toString()),
        ),
        PaletteEntry(DIcons.plus, l.paletteSaveNote(q), (c, app) async {
          final lib = await app.read(libraryProvider.future);
          if (lib == null) return;
          await lib.captureText(q);
          app.read(revisionProvider.notifier).bump();
          app.read(syncControllerProvider.notifier).changed();
          app.read(jobRunnerProvider.notifier).kick();
          if (c.mounted) showNote(c, l.saved);
        }),
      ],
      for (final h in _hits)
        PaletteEntry(
          DIcons.wiki,
          lang == 'fa' && h.page.titleFa.isNotEmpty
              ? h.page.titleFa
              : h.page.titleEn,
          (c, _) => c.push(
            Uri(
              path: '/wiki/page',
              queryParameters: {'path': h.page.path},
            ).toString(),
          ),
          subtitle: h.page.path,
        ),
      for (final a in actions)
        if (needle.isEmpty || a.title.toLowerCase().contains(needle)) a,
    ];
  }

  void _run(PaletteEntry e) {
    // The palette's ref ends with it; commands use the app's container instead.
    final app = ProviderScope.containerOf(widget.opener, listen: false);
    Navigator.of(context).pop();
    if (widget.opener.mounted) e.run(widget.opener, app);
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final entries = _entries(l, lang);
    final selected = entries.isEmpty
        ? -1
        : _selected.clamp(0, entries.length - 1);
    void move(int by) {
      if (entries.isEmpty) return;
      // Read the live index: two key presses can arrive before a rebuild.
      setState(() {
        final at = _selected.clamp(0, entries.length - 1);
        _selected = (at + by) % entries.length;
      });
    }

    return SafeArea(
      child: Align(
        alignment: const AlignmentDirectional(0, -0.6),
        child: Padding(
          padding: const EdgeInsets.all(Space.x4),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560, maxHeight: 520),
            child: Semantics(
              scopesRoute: true,
              namesRoute: true,
              explicitChildNodes: true,
              label: l.commandPalette,
              // Material only as the text field's required ancestor; it draws nothing itself.
              child: Material(
                color: p.raised,
                clipBehavior: Clip.antiAlias,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(Radii.large),
                  side: BorderSide(color: p.hairline, width: Stroke.hairline),
                ),
                child: CallbackShortcuts(
                  bindings: {
                    const SingleActivator(LogicalKeyboardKey.arrowDown): () =>
                        move(1),
                    const SingleActivator(LogicalKeyboardKey.arrowUp): () =>
                        move(-1),
                  },
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Padding(
                        padding: const EdgeInsets.all(Space.x3),
                        child: DTextField(
                          controller: _query,
                          hint: l.paletteHint,
                          leading: DIcons.search,
                          autofocus: true,
                          onChanged: _search,
                          onSubmitted: (_) {
                            // Live state, not this build's: keys may not have rebuilt yet.
                            final now = _entries(l, lang);
                            if (now.isEmpty) return;
                            _run(now[_selected.clamp(0, now.length - 1)]);
                          },
                        ),
                      ),
                      const DHairline(),
                      Flexible(
                        child: entries.isEmpty
                            ? Padding(
                                padding: const EdgeInsets.all(Space.x4),
                                child: Text(
                                  l.paletteNoMatch,
                                  style: context.type.small.copyWith(
                                    color: p.inkMuted,
                                  ),
                                ),
                              )
                            : ListView.builder(
                                shrinkWrap: true,
                                padding: const EdgeInsets.symmetric(
                                  vertical: Space.x1,
                                ),
                                itemCount: entries.length,
                                itemBuilder: (_, i) => _Row(
                                  entry: entries[i],
                                  selected: i == selected,
                                  onTap: () => _run(entries[i]),
                                ),
                              ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
    required this.entry,
    required this.selected,
    required this.onTap,
  });
  final PaletteEntry entry;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Pressable(
      onPressed: onTap,
      selected: selected,
      radius: 0,
      child: Container(
        color: selected ? p.accentSoft : null,
        padding: const EdgeInsets.symmetric(
          horizontal: Space.x4,
          vertical: Space.x2 + 2,
        ),
        child: Row(
          children: [
            DIcon(
              entry.icon,
              size: 18,
              color: selected ? p.accent : p.inkMuted,
            ),
            const SizedBox(width: Space.x3),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    entry.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    textDirection: directionOf(entry.title),
                    style: context.type.body.copyWith(
                      color: selected ? p.accent : p.ink,
                    ),
                  ),
                  if (entry.subtitle != null)
                    Text(
                      entry.subtitle!,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      textDirection: TextDirection.ltr,
                      style: context.type.caption.copyWith(color: p.inkMuted),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
