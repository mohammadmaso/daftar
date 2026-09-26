import 'package:daftar/core/file_import.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

ImportPreview _pdf({bool truncated = false}) => ImportPreview(
  kind: ImportKind.pdf,
  name: 'report.pdf',
  text: 'Quarterly plan\n\nSara leads the move to the new office.',
  totalChars: truncated ? 120000 : 52,
  truncated: truncated,
  pages: 3,
  bytes: BigInt.from(2 * 1024 * 1024),
);

void main() {
  testWidgets('the capture bar imports a document after a preview', (
    tester,
  ) async {
    final lib = FakeLibrary();
    final imports = FakeFileImports()
      ..next = '/tmp/report.pdf'
      ..previews['/tmp/report.pdf'] = _pdf(truncated: true);
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      imports: imports,
    );

    await tester.tap(find.bySemanticsLabel('Import a file'));
    await tester.pumpAndSettle();

    expect(find.text('report.pdf'), findsOneWidget);
    expect(find.textContaining('PDF, 3 pages'), findsOneWidget);
    expect(find.textContaining('only the first 60,000 of 120,000'), findsOne);
    expect(find.text('Quarterly plan'), findsOneWidget);
    expect(find.text('Sara leads the move to the new office.'), findsOneWidget);
    expect(lib.imports, isEmpty, reason: 'nothing is saved before File it');

    await tester.tap(find.text('File it'));
    await tester.pumpAndSettle();
    expect(lib.imports, [
      (
        'Quarterly plan\n\nSara leads the move to the new office.',
        'report.pdf',
        null,
      ),
    ]);
    expect(find.text('Imported report.pdf'), findsOneWidget);
    expect(find.text('Today'), findsWidgets);

    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('the text can be edited before filing', (tester) async {
    final lib = FakeLibrary();
    final imports = FakeFileImports()..previews['/tmp/report.pdf'] = _pdf();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      imports: imports,
      location: '/import?path=%2Ftmp%2Freport.pdf',
    );
    await tester.tap(find.text('Edit text'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'Only this part');
    await tester.tap(find.text('Done editing'));
    await tester.pumpAndSettle();
    expect(find.text('Only this part'), findsOneWidget);
    await tester.tap(find.text('File it'));
    await tester.pumpAndSettle();
    expect(lib.imports.single.$1, 'Only this part');

    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('a recording plays, then is transcribed and filed', (
    tester,
  ) async {
    final lib = FakeLibrary();
    final player = FakeFilePlayer();
    final imports = FakeFileImports()
      ..previews['/tmp/memo.mp3'] = ImportPreview(
        kind: ImportKind.audio,
        name: 'memo.mp3',
        text: '',
        totalChars: 0,
        truncated: false,
        bytes: BigInt.from(1024 * 1024),
      );
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      imports: imports,
      filePlayer: player,
      location: '/import?path=%2Ftmp%2Fmemo.mp3',
    );
    expect(find.textContaining('Recording, 1 MB'), findsOneWidget);
    await tester.tap(find.text('Play'));
    await tester.pumpAndSettle();
    expect(player.played, ['/tmp/memo.mp3']);
    expect(find.text('Stop'), findsOneWidget);

    await tester.tap(find.text('Transcribe and file'));
    await tester.pumpAndSettle();
    expect(lib.audioFiles, ['/tmp/memo.mp3']);
    expect(find.text('Imported memo.mp3'), findsOneWidget);

    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('an unreadable file says why and files nothing', (tester) async {
    final lib = FakeLibrary();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      imports: FakeFileImports(),
      location: '/import?path=%2Ftmp%2Fscan.pdf',
    );
    expect(find.textContaining('This PDF has no text in it'), findsOneWidget);
    expect(find.text('File it'), findsNothing);
    await tester.tap(find.text('Back').last);
    await tester.pumpAndSettle();
    expect(lib.imports, isEmpty);
  });

  testWidgets('Ctrl+O and the palette open the picker', (tester) async {
    final imports = FakeFileImports()
      ..next = '/tmp/report.pdf'
      ..previews['/tmp/report.pdf'] = _pdf();
    await pumpApp(tester, size: desktop, imports: imports);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyO);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();
    expect(find.text('report.pdf'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(find.text('report.pdf'), findsNothing);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Import a file'));
    await tester.pumpAndSettle();
    expect(find.text('report.pdf'), findsOneWidget);
  });
}
