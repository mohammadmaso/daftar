import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import '../activity_fixtures.dart';
import '../fakes.dart';
import '../helpers.dart';
import '../wiki_fixtures.dart';

/// Accessibility pass (§11, M9): every main screen, in both languages, meets Flutter's
/// guidelines for tap-target size (44 pt, brief §11; Material's 48 dp is not the bar) and labelled
/// targets, and lays out at 200 % text.
///
/// Contrast is checked on the tokens instead (test/design: every text/surface pair meets WCAG AA)
/// plus the rule below that faint ink is never text. Flutter's pixel-based `textContrastGuideline`
/// samples antialiased small glyphs, merged nodes spanning several surfaces and nested scroll views,
/// and gives different answers on macOS and Linux for the same screen.
void main() {
  const journal = 'vaults/life/journal/2026/2026-09-23.md';
  final screens = <String, (String, FakeLibrary Function())>{
    'today': ('/', FakeLibrary.new),
    'wiki': ('/wiki', wikiLibrary),
    'page': (
      '/wiki/page?path=${Uri.encodeQueryComponent(journal)}',
      wikiLibrary,
    ),
    'ask': ('/ask', FakeLibrary.new),
    'review': ('/review', activityLibrary),
    'activity': ('/activity', activityLibrary),
    'operation': ('/activity/op?id=01OPB', activityLibrary),
    'settings': ('/settings', FakeLibrary.new),
  };

  for (final lang in ['en', 'fa']) {
    for (final MapEntry(key: name, value: (location, library))
        in screens.entries) {
      for (final theme in ['light', 'dark']) {
        testWidgets('$name meets the guidelines ($lang, $theme)', (
          tester,
        ) async {
          final semantics = tester.ensureSemantics();
          await pumpApp(
            tester,
            prefs: prefsFor(language: lang, theme: theme),
            setup: FakeSetup(library: library()),
            location: location,
          );
          await expectLater(tester, meetsGuideline(iOSTapTargetGuideline));
          await expectLater(tester, meetsGuideline(labeledTapTargetGuideline));
          semantics.dispose();
        });
      }

      testWidgets('$name lays out at 200 % text ($lang)', (tester) async {
        tester.platformDispatcher.textScaleFactorTestValue = 2.0;
        await pumpApp(
          tester,
          prefs: prefsFor(language: lang, theme: 'dark'),
          setup: FakeSetup(library: library()),
          location: location,
        );
        expect(tester.takeException(), isNull);
      });
    }
  }

  test('faint ink is never used for text', () {
    final text = RegExp(
      r'(type\.\w+|TextStyle)\s*\.?\s*(copyWith)?\(\s*color:\s*\w+\.inkFaint',
    );
    final offenders = [
      for (final f in Directory('lib').listSync(recursive: true))
        if (f is File && f.path.endsWith('.dart'))
          for (final m in text.allMatches(f.readAsStringSync()))
            '${f.path}: ${m[0]}',
    ];
    expect(offenders, isEmpty, reason: 'inkFaint is decorative only');
  });
}
