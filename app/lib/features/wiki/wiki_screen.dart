import 'dart:async';

import 'package:flutter/material.dart' show Material;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/bidi.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../shell/home_shell.dart' show kWideLayout;
import 'page_screen.dart';
import 'wiki_state.dart';

/// The Wiki tab (§8.1): vault chips, instant bilingual search, pinned and recent pages. On wide
/// screens (§8.2) the page tree sits beside the reader.
class WikiScreen extends ConsumerWidget {
  const WikiScreen({super.key, this.selected});

  /// Page shown in the centre pane on wide screens.
  final String? selected;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final wide = MediaQuery.sizeOf(context).width >= kWideLayout;
    final p = context.palette;
    if (!wide) return const _WikiHome();
    return Material(
      color: p.paper,
      child: Row(
        children: [
          SizedBox(
            width: 320,
            child: DecoratedBox(
              decoration: BoxDecoration(
                border: BorderDirectional(end: BorderSide(color: p.hairline)),
              ),
              child: _WikiHome(
                onOpen: (path) =>
                    context.go('/wiki?path=${Uri.encodeQueryComponent(path)}'),
                showTree: true,
              ),
            ),
          ),
          Expanded(
            child: selected == null
                ? Center(child: DIcon(DIcons.wiki, size: 48, color: p.inkFaint))
                : PageScreen(
                    key: ValueKey(selected),
                    path: selected!,
                    embedded: true,
                  ),
          ),
        ],
      ),
    );
  }
}

class _WikiHome extends ConsumerStatefulWidget {
  const _WikiHome({this.onOpen, this.showTree = false});
  final ValueChanged<String>? onOpen;
  final bool showTree;

  @override
  ConsumerState<_WikiHome> createState() => _WikiHomeState();
}

class _WikiHomeState extends ConsumerState<_WikiHome> {
  final _query = TextEditingController();
  Timer? _debounce;

  @override
  void initState() {
    super.initState();
    _query.text = ref.read(wikiQueryProvider);
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _query.dispose();
    super.dispose();
  }

  void _open(String path) {
    if (widget.onOpen != null) {
      widget.onOpen!(path);
    } else {
      context.push(pageRoute(path));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final vault = ref.watch(wikiVaultProvider);
    final vaults = ref.watch(vaultsProvider).value ?? const <Vault>[];
    final q = ref.watch(wikiQueryProvider).trim();

    Widget pageRow(PageSummary s, {String? snippet}) => DListRow(
      title: pageTitle(lang, s.titleEn, s.titleFa, s.path),
      subtitle: (snippet?.isNotEmpty ?? false)
          ? snippet
          : (s.summary.isEmpty ? null : s.summary),
      onTap: () => _open(s.path),
      chevron: true,
    );

    final body = <Widget>[];
    if (q.isNotEmpty) {
      final hits = ref.watch(searchResultsProvider);
      body.add(
        hits.when(
          loading: () => const SizedBox.shrink(),
          error: (e, _) =>
              Text('$e', style: context.type.small.copyWith(color: p.critical)),
          data: (hs) => hs.isEmpty
              ? Padding(
                  padding: const EdgeInsets.all(Space.x4),
                  child: Text(
                    l.noResults(q),
                    style: context.type.body.copyWith(color: p.inkMuted),
                  ),
                )
              : DSection(
                  children: [
                    for (final h in hs) pageRow(h.page, snippet: h.snippet),
                  ],
                ),
        ),
      );
    } else {
      final pinned = ref.watch(pinnedPagesProvider).value ?? const [];
      if (pinned.isNotEmpty && vault == null) {
        body.add(
          DSection(
            title: l.pinnedPages,
            children: [
              for (final pg in pinned)
                DListRow(
                  title: pageTitle(lang, pg.titleEn, pg.titleFa, pg.path),
                  onTap: () => _open(pg.path),
                  chevron: true,
                ),
            ],
          ),
        );
      }
      if (widget.showTree) {
        final roots = vault == null
            ? [for (final v in vaults) v]
            : vaults.where((v) => v.id == vault).toList();
        body.add(
          DSection(
            title: l.pages,
            children: [
              for (final v in roots)
                _Folder(
                  dir: 'vaults/${v.id}',
                  label: lang == 'fa' ? v.titleFa : v.titleEn,
                  depth: 0,
                  onOpen: _open,
                ),
            ],
          ),
        );
      }
      final recent = ref.watch(recentPagesProvider).value ?? const [];
      body.add(
        recent.isEmpty
            ? Padding(
                padding: const EdgeInsets.all(Space.x4),
                child: Text(
                  l.emptyWiki,
                  style: context.type.body.copyWith(color: p.inkMuted),
                ),
              )
            : DSection(
                title: l.recentPages,
                children: [for (final s in recent.take(20)) pageRow(s)],
              ),
      );
    }

    return Material(
      color: p.paper,
      child: SafeArea(
        bottom: false,
        child: CustomScrollView(
          slivers: [
            SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsetsDirectional.fromSTEB(
                  Space.gutter,
                  Space.x4,
                  Space.gutter,
                  Space.x3,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Semantics(
                      header: true,
                      child: Text(l.wikiTitle, style: context.type.display),
                    ),
                    const SizedBox(height: Space.x3),
                    DTextField(
                      controller: _query,
                      hint: l.searchWiki,
                      leading: DIcons.search,
                      onChanged: (v) {
                        _debounce?.cancel();
                        _debounce = Timer(
                          const Duration(milliseconds: 120),
                          () {
                            ref.read(wikiQueryProvider.notifier).set(v);
                          },
                        );
                      },
                    ),
                    const SizedBox(height: Space.x2),
                    SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: Row(
                        children: [
                          DChip(
                            label: l.allVaults,
                            selected: vault == null,
                            onTap: () =>
                                ref.read(wikiVaultProvider.notifier).set(null),
                          ),
                          for (final v in vaults) ...[
                            const SizedBox(width: Space.x2),
                            DChip(
                              label: lang == 'fa' ? v.titleFa : v.titleEn,
                              selected: vault == v.id,
                              onTap: () => ref
                                  .read(wikiVaultProvider.notifier)
                                  .set(vault == v.id ? null : v.id),
                            ),
                          ],
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
            SliverList.separated(
              itemCount: body.length,
              separatorBuilder: (_, _) => const SizedBox(height: Space.x6),
              itemBuilder: (_, i) => Padding(
                padding: const EdgeInsets.symmetric(horizontal: Space.gutter),
                child: body[i],
              ),
            ),
            const SliverToBoxAdapter(child: SizedBox(height: Space.x12)),
          ],
        ),
      ),
    );
  }
}

/// One folder of the page tree, loaded when expanded (§12: the tree loads lazily).
class _Folder extends ConsumerStatefulWidget {
  const _Folder({
    required this.dir,
    required this.label,
    required this.depth,
    required this.onOpen,
  });
  final String dir;
  final String label;
  final int depth;
  final ValueChanged<String> onOpen;

  @override
  ConsumerState<_Folder> createState() => _FolderState();
}

class _FolderState extends ConsumerState<_Folder> {
  bool _open = false;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final indent = Space.x4 * widget.depth;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Pressable(
          onPressed: () => setState(() => _open = !_open),
          radius: 0,
          child: Padding(
            padding: EdgeInsetsDirectional.fromSTEB(
              Space.x3 + indent,
              Space.x2 + 2,
              Space.x3,
              Space.x2 + 2,
            ),
            child: Row(
              children: [
                AnimatedRotation(
                  turns: _open ? 0.25 : 0,
                  duration: motion(context, Motion.quick),
                  child: DIcon(
                    DIcons.chevronForward,
                    size: 16,
                    color: p.inkMuted,
                  ),
                ),
                const SizedBox(width: Space.x2),
                Expanded(child: Text(widget.label, style: context.type.label)),
              ],
            ),
          ),
        ),
        if (_open)
          ref
              .watch(listingProvider(widget.dir))
              .when(
                loading: () => const SizedBox.shrink(),
                error: (_, _) => const SizedBox.shrink(),
                data: (listing) => Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    for (final f in listing.folders)
                      _Folder(
                        dir: f.path,
                        label: f.path.split('/').last,
                        depth: widget.depth + 1,
                        onOpen: widget.onOpen,
                      ),
                    for (final s in listing.pages)
                      Pressable(
                        onPressed: () => widget.onOpen(s.path),
                        radius: 0,
                        child: Padding(
                          padding: EdgeInsetsDirectional.fromSTEB(
                            Space.x3 + indent + Space.x6,
                            Space.x2,
                            Space.x3,
                            Space.x2,
                          ),
                          child: Builder(
                            builder: (context) {
                              final t = pageTitle(
                                lang,
                                s.titleEn,
                                s.titleFa,
                                s.path,
                              );
                              return Text(
                                t,
                                textDirection: directionOf(
                                  t,
                                  fallback: Directionality.of(context),
                                ),
                                style: context.type.small.copyWith(
                                  color: p.ink,
                                ),
                                overflow: TextOverflow.ellipsis,
                              );
                            },
                          ),
                        ),
                      ),
                  ],
                ),
              ),
      ],
    );
  }
}
