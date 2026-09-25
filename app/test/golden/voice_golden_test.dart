import 'package:daftar/app/app.dart';
import 'package:daftar/core/library_api.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

void main() {
  for (final theme in ['light', 'dark']) {
    for (final lang in ['en', 'fa']) {
      final tag = '${theme}_$lang';
      testWidgets('voice · $tag · phone', (tester) async {
        final lib = FakeLibrary();
        await pumpApp(
          tester,
          prefs: prefsFor(language: lang, theme: theme),
          setup: FakeSetup(library: lib),
          location: '/ask',
        );
        await tester.tap(
          find.bySemanticsLabel(lang == 'fa' ? 'گفت‌وگو' : 'Talk'),
        );
        await tester.pump(const Duration(milliseconds: 500));
        final v = lib.voice!;
        v.emit(
          VoiceEventKind.userCaption,
          text: lang == 'fa'
              ? 'این هفته بیشتر نگران چی بودم؟'
              : 'What was I most worried about this week?',
        );
        v.emit(VoiceEventKind.state, state: VoiceStateDto.speaking);
        v.emit(
          VoiceEventKind.assistantCaption,
          text: lang == 'fa'
              ? 'بیشتر نگران سفر اصفهان و خواب کم بودی.'
              : 'Mostly the Isfahan trip, and short nights.',
        );
        await tester.pump(const Duration(milliseconds: 300));
        await expectLater(
          find.byType(DaftarApp),
          matchesGoldenFile('goldens/voice_${tag}_phone.png'),
        );
        await tester.tap(
          find.bySemanticsLabel(lang == 'fa' ? 'پایان' : 'End').first,
        );
        for (var i = 0; i < 4; i++) {
          await tester.pump(const Duration(seconds: 1));
        }
      });
    }
  }
}
