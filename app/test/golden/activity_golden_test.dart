import 'package:daftar/app/app.dart';
import 'package:flutter_test/flutter_test.dart';

import '../activity_fixtures.dart';
import '../fakes.dart';
import '../helpers.dart';

/// Activity detail and the Review stack in every theme × language (§11).
void main() {
  for (final theme in ['light', 'dark']) {
    for (final lang in ['en', 'fa']) {
      final tag = '${theme}_$lang';
      final prefs = prefsFor(language: lang, theme: theme);

      testWidgets('operation · $tag · phone', (tester) async {
        await pumpApp(tester, prefs: prefs, setup: FakeSetup(library: activityLibrary()), location: '/activity/op?id=01OPB');
        await expectLater(find.byType(DaftarApp), matchesGoldenFile('goldens/operation_${tag}_phone.png'));
      });

      testWidgets('review · $tag · phone', (tester) async {
        await pumpApp(tester, prefs: prefs, setup: FakeSetup(library: activityLibrary()), location: '/review');
        await expectLater(find.byType(DaftarApp), matchesGoldenFile('goldens/review_${tag}_phone.png'));
      });
    }
  }
}
