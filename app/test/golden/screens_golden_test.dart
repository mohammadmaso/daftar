import 'package:daftar/app/app.dart';
import 'package:daftar/core/library_api.dart';
import 'package:daftar/features/palette/command_palette.dart';
import 'package:daftar/features/settings/reflect_settings.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

Capture _c(
  String id,
  RawKind kind,
  String at,
  String text,
  Stage stage, {
  String? problem,
  Filing? filing,
}) => Capture(
  id: id,
  kind: kind,
  capturedAt: at,
  device: 'pixel-8',
  text: text,
  images: const [],
  stage: stage,
  problem: problem,
  filing: filing,
);

Filing _f(List<String> vaults, int pages, int claims) => Filing(
  opId: '01OP',
  vaults: vaults,
  pagesCreated: 1,
  pagesUpdated: pages - 1,
  claimsToReview: claims,
  toReview: claims,
);

List<Capture> _day() => [
  _c(
    '01',
    RawKind.voice,
    '2026-09-23T08:12:00',
    'بد خوابیدم، تا ظهر سردرد داشتم. شاید به‌خاطر قهوه‌ی دیروقت بود.',
    Stage.filed,
    filing: _f(['life', 'health'], 4, 1),
  ),
  _c(
    '02',
    RawKind.text,
    '2026-09-23T11:40:00',
    'Sara called about the trip to Isfahan — she wants to leave on Thursday.',
    Stage.filed,
    filing: _f(['life'], 2, 0),
  ),
  _c('03', RawKind.photo, '2026-09-23T12:30:00', '', Stage.saved),
  _c(
    '04',
    RawKind.text,
    '2026-09-23T13:55:00',
    'جلسه با Sara درباره‌ی project جدید خوب بود.',
    Stage.saved,
  ),
  _c('05', RawKind.voice, '2026-09-23T14:02:00', '', Stage.saved),
];

/// Key screens in every theme × language combination (brief §11).
void main() {
  for (final theme in ['light', 'dark']) {
    for (final lang in ['en', 'fa']) {
      final prefs = prefsFor(language: lang, theme: theme);
      final tag = '${theme}_$lang';

      testWidgets('settings · $tag · phone', (tester) async {
        await pumpApp(tester, prefs: prefs, location: '/settings');
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/settings_${tag}_phone.png'),
        );
      });

      testWidgets('today · $tag · phone', (tester) async {
        await pumpApp(
          tester,
          prefs: prefs,
          setup: FakeSetup(library: FakeLibrary(captures: _day())),
        );
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/today_${tag}_phone.png'),
        );
      });

      testWidgets('settings reflect · $tag · phone', (tester) async {
        await pumpApp(tester, prefs: prefs, location: '/settings');
        await scrollTo(tester, find.byType(ReflectSection));
        // The section's top at the top of the screen, so all of it shows.
        await Scrollable.ensureVisible(
          tester.element(find.byType(ReflectSection)),
        );
        await tester.pumpAndSettle();
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/settings_reflect_${tag}_phone.png'),
        );
      });

      testWidgets('today help card · $tag · phone', (tester) async {
        final lib = FakeLibrary(captures: _day())
          ..nextRun = const RunSummary(
            jobs: [JobOutcome(kind: JobKindDto.other, state: JobStateDto.done)],
            pending: false,
          )
          ..signals = const ReflectSignals(notifications: [], needsHelp: true);
        await pumpApp(
          tester,
          prefs: prefs,
          setup: FakeSetup(library: lib),
        );
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/today_help_${tag}_phone.png'),
        );
      });

      testWidgets('palette · $tag · desktop', (tester) async {
        final lib = FakeLibrary(captures: _day())
          ..addPage(
            'vaults/life/people/sara.md',
            'Sara',
            'سارا',
            'Cousin.',
            kind: 'person',
          );
        await pumpApp(
          tester,
          prefs: prefs,
          size: desktop,
          setup: FakeSetup(library: lib),
        );
        await tester.tap(find.text(lang == 'fa' ? 'فرمان‌ها' : 'Commands'));
        await tester.pumpAndSettle();
        await tester.enterText(
          find.descendant(
            of: find.byType(CommandPalette),
            matching: find.byType(EditableText),
          ),
          lang == 'fa' ? 'سارا' : 'Sara',
        );
        await tester.pumpAndSettle();
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/palette_${tag}_desktop.png'),
        );
      });

      testWidgets('today empty · $tag · phone', (tester) async {
        await pumpApp(tester, prefs: prefs);
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/today_empty_${tag}_phone.png'),
        );
      });

      testWidgets('today · $tag · desktop', (tester) async {
        await pumpApp(
          tester,
          prefs: prefs,
          size: desktop,
          setup: FakeSetup(library: FakeLibrary(captures: _day())),
        );
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/today_${tag}_desktop.png'),
        );
      });

      testWidgets('welcome · $tag · phone', (tester) async {
        await pumpApp(tester, prefs: prefs, setup: FakeSetup());
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/welcome_${tag}_phone.png'),
        );
      });
    }
  }
}
