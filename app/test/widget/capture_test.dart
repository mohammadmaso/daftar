import 'package:daftar/core/library_api.dart';
import 'package:daftar/features/capture/capture_bar.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

void main() {
  testWidgets('first run shows onboarding; local start opens Today', (
    tester,
  ) async {
    final setup = FakeSetup();
    await pumpApp(tester, setup: setup);
    expect(find.text('A notebook that files itself.'), findsOneWidget);

    await tester.tap(find.text('Start on this device for now'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'Test phone');
    await tester.tap(find.text('Continue'));
    await tester.pumpAndSettle();

    expect(setup.deviceName, 'Test phone');
    expect(find.text('Today'), findsOneWidget);
    expect(find.text('On this device only'), findsOneWidget);
  });

  testWidgets('connect with HTTPS token clones and stores credentials', (
    tester,
  ) async {
    final setup = FakeSetup();
    await pumpApp(tester, setup: setup);
    await tester.tap(find.text('Connect a private repository'));
    await tester.pumpAndSettle();

    final continueButton = find.text('Continue');
    await tester.enterText(
      find.byType(EditableText).at(0),
      'https://github.com/me/notes.git',
    );
    await tester.enterText(find.byType(EditableText).at(1), 'github_pat_x');
    await tester.pumpAndSettle();
    await tester.tap(continueButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Continue'));
    await tester.pumpAndSettle();

    expect(setup.clonedUrl, 'https://github.com/me/notes.git');
    expect(setup.clonedAuth!.kind, AuthKind.token);
    expect(setup.clonedAuth!.secret, 'github_pat_x');
    expect(find.text('Today'), findsOneWidget);
  });

  testWidgets('SSH mode generates a key and shows the public half', (
    tester,
  ) async {
    await pumpApp(tester, setup: FakeSetup());
    await tester.tap(find.text('Connect a private repository'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('SSH key'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Create a key for this device'));
    await tester.pumpAndSettle();
    expect(find.textContaining('ssh-ed25519 AAAA'), findsOneWidget);
    expect(
      find.textContaining('PRIVATE KEY'),
      findsNothing,
      reason: 'private key is never shown',
    );
  });

  testWidgets('tap the capture button to type a note', (tester) async {
    final lib = FakeLibrary();
    await pumpApp(tester, setup: FakeSetup(library: lib));
    await tester.tap(find.bySemanticsLabel('Hold to record, tap to type'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'Sara called');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(lib.texts, ['Sara called']);
    expect(find.text('Sara called'), findsOneWidget);
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('hold to record, release to save', (tester) async {
    final lib = FakeLibrary();
    final rec = FakeRecorder();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      recorder: rec,
    );
    final button = find.bySemanticsLabel('Hold to record, tap to type');
    final g = await tester.startGesture(tester.getCenter(button));
    await tester.pump(const Duration(milliseconds: 600));
    expect(rec.recording, isTrue);
    expect(find.text('Slide to cancel'), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 800));
    await g.up();
    await tester.pumpAndSettle();
    expect(lib.captures.single.kind, RawKind.voice);
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('slide toward the start cancels the recording', (tester) async {
    final lib = FakeLibrary();
    final rec = FakeRecorder();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      recorder: rec,
    );
    final button = find.bySemanticsLabel('Hold to record, tap to type');
    final g = await tester.startGesture(tester.getCenter(button));
    await tester.pump(const Duration(milliseconds: 600));
    await g.moveBy(const Offset(-60, 0));
    await tester.pump();
    await g.moveBy(const Offset(-60, 0));
    await tester.pump();
    await g.up();
    await tester.pumpAndSettle();
    expect(rec.cancelled, isTrue);
    expect(lib.captures, isEmpty);
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('pinned vault applies to the next capture only', (tester) async {
    final lib = FakeLibrary();
    final c = await pumpApp(tester, setup: FakeSetup(library: lib));
    c.read(pinnedVaultProvider.notifier).set('health');
    await tester.pumpAndSettle();
    expect(find.text('Health'), findsOneWidget);
    await tester.tap(find.bySemanticsLabel('Hold to record, tap to type'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'headache');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(lib.captures.single.vaultHint, 'health');
    expect(c.read(pinnedVaultProvider), isNull);
    await tester.pump(const Duration(seconds: 3));
  });
}
