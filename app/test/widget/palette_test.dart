import 'package:daftar/core/app_shortcuts.dart';
import 'package:daftar/core/incoming_shares.dart';
import 'package:daftar/core/library_api.dart';
import 'package:daftar/features/palette/command_palette.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

/// Ctrl+K here: the test host is not an Apple platform.
Future<void> chord(
  WidgetTester tester,
  LogicalKeyboardKey key, {
  bool shift = false,
}) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyEvent(key);
  if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await tester.pumpAndSettle();
}

Finder get paletteField => find.descendant(
  of: find.byType(CommandPalette),
  matching: find.byType(EditableText),
);

void main() {
  quickActionTests();
  shareTests();
  widgetTests();

  testWidgets('Ctrl+K finds a page and opens it', (tester) async {
    final lib = FakeLibrary()
      ..addPage(
        'vaults/life/people/sara.md',
        'Sara',
        'سارا',
        'Cousin; lives in Isfahan.',
        kind: 'person',
      );
    await pumpApp(
      tester,
      size: desktop,
      setup: FakeSetup(library: lib),
    );
    await chord(tester, LogicalKeyboardKey.keyK);
    expect(find.text('Search, open, capture or ask'), findsOneWidget);

    await tester.enterText(paletteField, 'Sara');
    await tester.pumpAndSettle();
    expect(find.text('Ask: Sara'), findsOneWidget);
    expect(find.text('Save as a note: Sara'), findsOneWidget);
    expect(find.text('vaults/life/people/sara.md'), findsOneWidget);

    // Ask, Save, then the page: two steps down.
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(
      find.textContaining('lives in Isfahan', findRichText: true),
      findsOneWidget,
    );
  });

  testWidgets('the first command asks the question', (tester) async {
    final lib = FakeLibrary();
    await pumpApp(
      tester,
      size: desktop,
      setup: FakeSetup(library: lib),
    );
    await chord(tester, LogicalKeyboardKey.keyK);
    await tester.enterText(paletteField, 'When is the trip?');
    await tester.pumpAndSettle();
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(lib.questions.single.$1, 'When is the trip?');
  });

  testWidgets('a note can be saved straight from the palette', (tester) async {
    final lib = FakeLibrary();
    await pumpApp(
      tester,
      size: desktop,
      setup: FakeSetup(library: lib),
    );
    await chord(tester, LogicalKeyboardKey.keyK);
    await tester.enterText(paletteField, 'Call the dentist');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Save as a note: Call the dentist'));
    await tester.pumpAndSettle();
    expect(lib.texts, ['Call the dentist']);
    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
  });

  testWidgets('commands filter as you type; Record starts hands-free', (
    tester,
  ) async {
    final recorder = FakeRecorder();
    await pumpApp(tester, size: desktop, recorder: recorder, location: '/wiki');
    await chord(tester, LogicalKeyboardKey.keyK);
    await tester.enterText(paletteField, 'record');
    await tester.pumpAndSettle();
    expect(find.text('Go to Wiki'), findsNothing);
    await tester.tap(find.text('Record a voice note'));
    await tester.pump();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(recorder.recording, isTrue);
    expect(find.text('Save'), findsOneWidget, reason: 'locked: tap to save');
  });

  testWidgets('Ctrl+N opens a new note, Ctrl+Shift+N records', (tester) async {
    final recorder = FakeRecorder();
    await pumpApp(tester, size: desktop, recorder: recorder);
    await chord(tester, LogicalKeyboardKey.keyN);
    expect(find.byType(EditableText), findsOneWidget);
    await tester.tapAt(const Offset(10, 10));
    await tester.pumpAndSettle();

    await chord(tester, LogicalKeyboardKey.keyN, shift: true);
    expect(recorder.recording, isTrue);
  });
}

void quickActionTests() {
  testWidgets('the app-icon Record action opens straight into recording', (
    tester,
  ) async {
    final shortcuts = FakeAppShortcuts();
    final recorder = FakeRecorder();
    await pumpApp(
      tester,
      shortcuts: shortcuts,
      recorder: recorder,
      location: '/wiki',
    );
    expect(shortcuts.titles.values, ['Record a voice note', 'New note', 'Ask']);
    shortcuts.launch(AppShortcut.record);
    await tester.pump();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(recorder.recording, isTrue);
    expect(find.text('Save'), findsOneWidget);
  });
}

void shareTests() {
  testWidgets('text and photos shared into the app become captures', (
    tester,
  ) async {
    final lib = FakeLibrary();
    final shares = FakeShares()
      ..pending.add(const SharedItem.text('Shared at launch'));
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      shares: shares,
      location: '/wiki',
    );
    await tester.pumpAndSettle();
    expect(lib.texts, ['Shared at launch'], reason: 'a cold-start share');

    shares.share(SharedItem.image(Uint8List.fromList([1, 2, 3])));
    await tester.pumpAndSettle();
    expect(lib.captures.last.kind, RawKind.photo);
    expect(find.text('Saved what you shared'), findsWidgets);
    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
  });
}

void widgetTests() {
  testWidgets('the home-screen Record widget opens straight into recording', (
    tester,
  ) async {
    final recorder = FakeRecorder();
    final shares = FakeShares()..pending.add(const SharedItem.record());
    await pumpApp(tester, recorder: recorder, shares: shares);
    await tester.pump(const Duration(milliseconds: 300));
    expect(recorder.recording, isTrue);
  });
}
