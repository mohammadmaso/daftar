import 'package:daftar/app/app.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';
import '../wiki_fixtures.dart';

/// Scenario 8 (§13): bidi rendering — mixed Persian/English paragraphs, lists, tables and code
/// inside an RTL page — in every theme × language, plus the desktop panes.
void main() {
  const journal = 'vaults/life/journal/2026/2026-09-23.md';
  for (final theme in ['light', 'dark']) {
    for (final lang in ['en', 'fa']) {
      final tag = '${theme}_$lang';
      final prefs = prefsFor(language: lang, theme: theme);

      testWidgets('page · bidi · $tag · phone', (tester) async {
        await pumpApp(
          tester,
          prefs: prefs,
          setup: FakeSetup(library: wikiLibrary()),
          location: '/wiki/page?path=${Uri.encodeQueryComponent(journal)}',
          size: const Size(390, 1500),
        );
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/page_bidi_${tag}_phone.png'),
        );
      });

      testWidgets('page · claims · $tag · phone', (tester) async {
        await pumpApp(
          tester,
          prefs: prefs,
          setup: FakeSetup(library: wikiLibrary()),
          location:
              '/wiki/page?path=${Uri.encodeQueryComponent('vaults/health/profile.md')}',
        );
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/page_claims_${tag}_phone.png'),
        );
      });

      testWidgets('wiki · $tag · phone', (tester) async {
        await pumpApp(
          tester,
          prefs: prefs,
          setup: FakeSetup(library: wikiLibrary()),
          location: '/wiki',
        );
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/wiki_${tag}_phone.png'),
        );
      });

      testWidgets('wiki · $tag · desktop', (tester) async {
        await pumpApp(
          tester,
          prefs: prefs,
          size: desktop,
          setup: FakeSetup(library: wikiLibrary()),
          location: '/wiki?path=${Uri.encodeQueryComponent(journal)}',
        );
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/wiki_${tag}_desktop.png'),
        );
      });
    }
  }
}
