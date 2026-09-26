import 'package:daftar/core/file_import.dart';
import 'package:daftar/core/global_hotkey.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

/// The compact recorder window size on Linux and macOS (desktop_shell.cc, MainFlutterWindow.swift).
const mini = Size(420, 150);

Future<(FakeHotkey, FakeRecorder, FakeLibrary)> _start(
  WidgetTester tester, {
  bool permission = true,
}) async {
  final hotkey = FakeHotkey(hasMini: true)
    ..onWindow = (on) => tester.view.physicalSize = (on ? mini : desktop) * 2;
  final recorder = FakeRecorder()..permission = permission;
  final lib = FakeLibrary();
  await pumpApp(
    tester,
    size: desktop,
    hotkey: hotkey,
    recorder: recorder,
    setup: FakeSetup(library: lib),
    location: '/wiki',
  );
  hotkey.press();
  await tester.pumpAndSettle();
  return (hotkey, recorder, lib);
}

void main() {
  testWidgets('the shortcut opens the compact recorder and records at once', (
    tester,
  ) async {
    final (hotkey, recorder, lib) = await _start(tester);
    expect(hotkey.mini, isTrue);
    expect(recorder.recording, isTrue);
    expect(hotkey.recording, [true]);
    expect(find.text('Enter saves · Esc discards'), findsOneWidget);
    expect(find.text('Wiki'), findsNothing, reason: 'no navigation chrome');

    await tester.pump(const Duration(seconds: 2));
    expect(find.text('0:02'), findsOneWidget);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(lib.captures.single.kind.name, 'voice');
    expect(hotkey.left, [true], reason: 'the window comes back, opened');
    expect(hotkey.recording, [true, false]);
    expect(find.text('Saved'), findsWidgets);
    expect(find.text('Today'), findsWidgets);
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('pressing the shortcut again saves', (tester) async {
    final (hotkey, _, lib) = await _start(tester);
    await tester.pump(const Duration(seconds: 1));
    hotkey.press();
    await tester.pumpAndSettle();
    expect(lib.captures, hasLength(1));
    expect(hotkey.left, [true]);
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('Esc discards and puts the window back as it was', (
    tester,
  ) async {
    final (hotkey, recorder, lib) = await _start(tester);
    await tester.pump(const Duration(seconds: 1));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(recorder.cancelled, isTrue);
    expect(lib.captures, isEmpty);
    expect(hotkey.left, [false]);
  });

  testWidgets('without the microphone it says so and records nothing', (
    tester,
  ) async {
    final (hotkey, recorder, lib) = await _start(tester, permission: false);
    expect(find.textContaining('Microphone access is off'), findsOneWidget);
    expect(recorder.recording, isFalse);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(hotkey.left, [false]);
    expect(lib.captures, isEmpty);
  });

  testWidgets('the tray menu is in the app language and imports files', (
    tester,
  ) async {
    final hotkey = FakeHotkey(hasMini: true);
    final imports = FakeFileImports()
      ..next = '/tmp/a.md'
      ..previews['/tmp/a.md'] = ImportPreview(
        kind: ImportKind.markdown,
        name: 'a.md',
        text: 'hello',
        totalChars: 5,
        truncated: false,
        bytes: BigInt.from(5),
      )
      ..previews['/tmp/b.md'] = ImportPreview(
        kind: ImportKind.markdown,
        name: 'b.md',
        text: 'opened with',
        totalChars: 11,
        truncated: false,
        bytes: BigInt.from(11),
      );
    await pumpApp(tester, size: desktop, hotkey: hotkey, imports: imports);
    expect(hotkey.labels, {
      'record': 'Record a voice note',
      'import': 'Import a file…',
      'open': 'Open Daftar',
      'quit': 'Quit',
    });

    hotkey.trayImport();
    await tester.pumpAndSettle();
    expect(find.text('a.md'), findsOneWidget);

    hotkey.trayImport('/tmp/b.md');
    await tester.pumpAndSettle();
    expect(find.text('opened with'), findsOneWidget);
  });

  testWidgets('Settings adds the shortcut to GNOME where apps cannot', (
    tester,
  ) async {
    final hotkey = FakeHotkey()
      ..status = const ShortcutStatus(
        keys: 'Ctrl+Alt+Shift+N',
        state: ShortcutState.unavailable,
        canInstall: true,
      );
    await pumpApp(tester, size: desktop, hotkey: hotkey, location: '/settings');
    await scrollTo(tester, find.text('Add it to GNOME keyboard shortcuts'));
    expect(
      find.text("This desktop doesn't let apps add a shortcut"),
      findsOneWidget,
    );
    await tester.tap(find.text('Add it to GNOME keyboard shortcuts'));
    await tester.pumpAndSettle();
    expect(hotkey.installed, ['Record a voice note']);
    expect(
      find.text("On: set in your desktop's keyboard shortcuts"),
      findsOneWidget,
    );
    expect(find.text('Add it to GNOME keyboard shortcuts'), findsNothing);
    await tester.pump(const Duration(seconds: 3));
  });
}
