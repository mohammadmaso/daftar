import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/bidi.dart';
import '../../core/dates.dart';
import '../../core/errors.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import 'local_graph.dart';
import 'markdown_view.dart';
import 'wiki_state.dart';

/// The page reader (§8.3): a compact property header, the rendered body, and backlinks.
class PageScreen extends ConsumerWidget {
  const PageScreen({super.key, required this.path, this.embedded = false});

  final String path;

  /// In the desktop layout the reader is the centre pane, not a pushed page.
  final bool embedded;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final page = ref.watch(pageProvider(path));
    final root = ref.watch(libraryRootPathProvider).value;

    void open(String target) => openLink(
      context,
      ref,
      target,
      missing: l.pageMissing,
      note: (m) => showNote(context, m),
    );

    return page.when(
      loading: () => const SizedBox.shrink(),
      error: (e, _) => DPage(
        title: l.wikiTitle,
        onBack: embedded || !context.canPop() ? null : () => context.pop(),
        backLabel: l.back,
        children: [
          Text(
            humanError(e),
            style: context.type.body.copyWith(color: p.critical),
          ),
        ],
      ),
      data: (pg) {
        final title = pageTitle(lang, pg.titleEn, pg.titleFa, pg.path);
        final isRaw = pg.path.startsWith('raw/');
        return DPage(
          title: title,
          onBack: embedded || !context.canPop() ? null : () => context.pop(),
          backLabel: l.back,
          trailing: isRaw
              ? null
              : Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    DIconButton(
                      icon: DIcons.graph,
                      semanticLabel: l.localGraph,
                      color: p.inkMuted,
                      onPressed: () => showDSheet<void>(
                        context,
                        builder: (sheet) => LocalGraphView(
                          path: pg.path,
                          onOpen: (to) {
                            Navigator.of(sheet).pop();
                            context.push(pageRoute(to));
                          },
                        ),
                      ),
                    ),
                    DIconButton(
                      icon: DIcons.edit,
                      semanticLabel: l.editPage,
                      color: p.inkMuted,
                      onPressed: () => context.push(
                        '/wiki/edit?path=${Uri.encodeQueryComponent(pg.path)}',
                      ),
                    ),
                  ],
                ),
          children: [
            _Properties(page: pg),
            MarkdownView(text: pg.body, onLink: open, imageRoot: root),
            if (!isRaw) _Backlinks(page: pg),
          ],
        );
      },
    );
  }
}

class _Properties extends ConsumerWidget {
  const _Properties({required this.page});
  final WikiPage page;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final vaults = ref.watch(vaultsProvider).value ?? const <Vault>[];
    final v = vaults.where((x) => x.id == page.vault).firstOrNull;
    final updated = DateTime.tryParse(page.updated);
    final parts = [
      if (page.kind.isNotEmpty) page.kind,
      if (v != null) (lang == 'fa' ? v.titleFa : v.titleEn),
      if (updated != null) l.updatedOn(shortDate(updated, lang)),
      if (page.sourceCount > 0) l.sourcesCount(page.sourceCount),
    ];
    final other = lang == 'fa' ? page.titleEn : page.titleFa;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (other.trim().isNotEmpty)
          Text(
            other,
            textDirection: directionOf(other),
            style: context.type.body.copyWith(color: p.inkMuted),
          ),
        const SizedBox(height: Space.x2),
        Wrap(
          spacing: Space.x2,
          runSpacing: Space.x2,
          children: [
            for (final t in parts)
              Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.x2,
                  vertical: 2,
                ),
                decoration: BoxDecoration(
                  color: p.sunken,
                  borderRadius: BorderRadius.circular(Radii.pill),
                ),
                child: Text(
                  t,
                  style: context.type.caption.copyWith(color: p.inkMuted),
                ),
              ),
          ],
        ),
      ],
    );
  }
}

class _Backlinks extends StatelessWidget {
  const _Backlinks({required this.page});
  final WikiPage page;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    if (page.backlinks.isEmpty) {
      return DSection(
        title: l.backlinks,
        children: [
          Padding(
            padding: const EdgeInsets.all(Space.x4),
            child: Text(
              l.noBacklinks,
              style: context.type.small.copyWith(color: p.inkMuted),
            ),
          ),
        ],
      );
    }
    return DSection(
      title: l.backlinks,
      children: [
        for (final b in page.backlinks)
          DListRow(
            title: pageTitle(lang, b.titleEn, b.titleFa, b.path),
            subtitle: b.summary.isEmpty ? null : b.summary,
            chevron: true,
            onTap: () => context.push(pageRoute(b.path)),
          ),
      ],
    );
  }
}
