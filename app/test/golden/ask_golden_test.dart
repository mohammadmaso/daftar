import 'package:daftar/app/app.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';
import '../wiki_fixtures.dart';

void main() {
  for (final theme in ['light', 'dark']) {
    for (final lang in ['en', 'fa']) {
      final tag = '${theme}_$lang';
      testWidgets('ask · $tag · phone', (tester) async {
        final lib = wikiLibrary();
        await pumpApp(
          tester,
          prefs: prefsFor(language: lang, theme: theme),
          setup: FakeSetup(library: lib),
          location: '/ask',
        );
        await tester.enterText(
          find.byType(EditableText),
          lang == 'fa' ? 'سارا کیست؟' : 'Who is Sara?',
        );
        await tester.pump();
        await tester.tap(
          find.bySemanticsLabel(lang == 'fa' ? 'بفرست' : 'Send'),
        );
        await tester.pumpAndSettle();
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/ask_${tag}_phone.png'),
        );
      });
    }
  }
}
