import 'dart:typed_data';

import 'package:daftar/core/library_api.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

void main() {
  testWidgets(
    'voice mode streams the mic, plays answers, stops on barge-in, and ends cleanly',
    (tester) async {
      final lib = FakeLibrary();
      final mic = FakeMic();
      final player = FakePlayer();
      final awake = FakeAwake();
      await pumpApp(
        tester,
        setup: FakeSetup(library: lib),
        mic: mic,
        player: player,
        awake: awake,
        location: '/ask',
      );
      await tester.tap(find.bySemanticsLabel('Talk'));
      await tester.pump(const Duration(milliseconds: 500));
      await tester.pump(const Duration(milliseconds: 500));
      expect(mic.running, isTrue);
      expect(awake.on, isTrue, reason: 'screen stays awake');
      expect(find.text('Listening'), findsOneWidget);

      mic.controller.add(Uint8List(640));
      await tester.pump();
      expect(lib.voice!.fed, [320], reason: '16-bit PCM becomes samples');

      final v = lib.voice!;
      v.emit(VoiceEventKind.userCaption, text: 'Tell me about Sara');
      v.emit(VoiceEventKind.state, state: VoiceStateDto.speaking);
      v.emit(VoiceEventKind.assistantCaption, text: 'Sara is your cousin.');
      v.emit(VoiceEventKind.audio, bytes: Uint8List.fromList([1, 2, 3]));
      await tester.pump();
      expect(find.text('Speaking'), findsOneWidget);
      expect(find.text('Sara is your cousin.'), findsOneWidget);
      expect(player.played.single, [1, 2, 3]);

      v.emit(VoiceEventKind.stopPlayback);
      await tester.pump();
      expect(player.stops, 1, reason: 'barge-in cuts playback');

      player.finish();
      await tester.pump();
      expect(v.playbackDone, 1);

      await tester.tap(find.bySemanticsLabel('Mute').first);
      await tester.pump();
      expect(v.muted, isTrue);

      await tester.tap(find.bySemanticsLabel('End').first);
      for (var i = 0; i < 6; i++) {
        await tester.pump(const Duration(milliseconds: 200));
      }
      expect(v.ended, isTrue);
      expect(mic.running, isFalse);
      expect(awake.on, isFalse);
      expect(
        find.text('Conversation saved; it will be filed.'),
        findsOneWidget,
      );
      await tester.pump(const Duration(seconds: 3));
    },
  );

  testWidgets('without the microphone voice mode says so', (tester) async {
    await pumpApp(tester, mic: FakeMic()..permission = false, location: '/ask');
    await tester.tap(find.bySemanticsLabel('Talk'));
    await tester.pump(const Duration(milliseconds: 500));
    await tester.pump(const Duration(milliseconds: 500));
    expect(find.text('Voice mode needs the microphone.'), findsOneWidget);
  });
}
