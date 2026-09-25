import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/library_api.dart';
import '../../core/library_state.dart';

/// The vault the Wiki tab is filtered to; `null` = all vaults.
final wikiVaultProvider = NotifierProvider<WikiVault, String?>(WikiVault.new);

class WikiVault extends Notifier<String?> {
  @override
  String? build() => null;
  void set(String? v) => state = v;
}

final wikiQueryProvider = NotifierProvider<WikiQuery, String>(WikiQuery.new);

class WikiQuery extends Notifier<String> {
  @override
  String build() => '';
  void set(String q) => state = q;
}

final searchResultsProvider = FutureProvider<List<SearchHit>>((ref) async {
  ref.watch(revisionProvider);
  final q = ref.watch(wikiQueryProvider).trim();
  final vault = ref.watch(wikiVaultProvider);
  final lib = await ref.watch(libraryProvider.future);
  if (lib == null || q.isEmpty) return const [];
  return lib.search(q, vaults: vault == null ? const [] : [vault], limit: 40);
});

final recentPagesProvider = FutureProvider<List<PageSummary>>((ref) async {
  ref.watch(revisionProvider);
  final vault = ref.watch(wikiVaultProvider);
  final lib = await ref.watch(libraryProvider.future);
  if (lib == null) return const [];
  return lib.recentPages(vault: vault, limit: 30);
});

/// Pages everyone keeps coming back to (§8.1): the profiles and concerns, when they exist.
const pinnedCandidates = [
  'vaults/health/profile.md',
  'vaults/mind/profile.md',
  'vaults/life/profile.md',
];

final pinnedPagesProvider = FutureProvider<List<WikiPage>>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  if (lib == null) return const [];
  final out = <WikiPage>[];
  for (final p in pinnedCandidates) {
    try {
      out.add(await lib.page(p));
    } catch (_) {
      // Not created yet.
    }
  }
  return out;
});

final pageProvider = FutureProvider.family<WikiPage, String>((ref, path) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib!.page(path);
});

final listingProvider = FutureProvider.family<Listing, String>((
  ref,
  dir,
) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib!.listDir(dir);
});

final graphProvider = FutureProvider.family<LocalGraph, (String, int)>((
  ref,
  key,
) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib!.localGraph(key.$1, depth: key.$2);
});

final libraryRootPathProvider = FutureProvider<String?>((ref) async {
  final lib = await ref.watch(libraryProvider.future);
  return lib?.root();
});

String pageTitle(String lang, String en, String fa, String path) {
  final primary = lang == 'fa' ? fa : en;
  final secondary = lang == 'fa' ? en : fa;
  if (primary.trim().isNotEmpty) return primary;
  if (secondary.trim().isNotEmpty) return secondary;
  return path.split('/').last.replaceAll('.md', '');
}

String pageRoute(String path) =>
    '/wiki/page?path=${Uri.encodeQueryComponent(path)}';

/// Opens a wikilink target (§8.3): resolved like Obsidian; missing pages say so calmly.
Future<void> openLink(
  BuildContext context,
  WidgetRef ref,
  String target, {
  required String missing,
  required void Function(String) note,
}) async {
  if (target.startsWith('http://') || target.startsWith('https://')) return;
  final lib = await ref.read(libraryProvider.future);
  final resolved = await lib?.resolveLink(
    target.split('#').first.split('^').first,
  );
  if (!context.mounted) return;
  if (resolved == null) {
    note(missing);
    return;
  }
  context.push(pageRoute(resolved));
}
