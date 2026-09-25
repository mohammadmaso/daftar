import 'package:daftar/core/library_api.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../activity_fixtures.dart';
import '../fakes.dart';
import '../helpers.dart';

void main() {
  testWidgets('activity lists ops and an op can be undone, moved or re-run', (tester) async {
    final lib = activityLibrary();
    await pumpApp(tester, setup: FakeSetup(library: lib));
    await tester.tap(find.text('Activity'));
    await tester.pumpAndSettle();
    expect(find.text('Filed to Life · Health'), findsOneWidget);
    expect(find.textContaining('undone'), findsOneWidget);

    await tester.tap(find.text('Filed to Life · Health'));
    await tester.pumpAndSettle();
    expect(find.text('Why here'), findsOneWidget);
    expect(find.text('a personal day note'), findsOneWidget);
    expect(find.text('92% sure'), findsOneWidget);
    await scrollTo(tester, find.text('Changes'));
    expect(find.textContaining('Lives in Shiraz'), findsOneWidget);

    await scrollTo(tester, find.text('Undo'));
    await tester.tap(find.text('Undo'));
    await tester.pumpAndSettle();
    expect(lib.undone, ['01OPB']);
    expect(find.text('Undone.'), findsOneWidget);
    await tester.pump(const Duration(seconds: 3));

    await tester.tap(find.text('Move to vault…'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Work'));
    await tester.pumpAndSettle();
    expect(lib.moved, [('01OPB', 'work')]);
    await tester.pump(const Duration(seconds: 3));

    lib.nextUndo = UndoResult.queued;
    await tester.tap(find.text('Re-run with a note…'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'Sara is my cousin, not my colleague');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Continue'));
    await tester.pumpAndSettle();
    expect(lib.reruns.single.$2, 'Sara is my cousin, not my colleague');
    expect(find.textContaining('undone carefully'), findsOneWidget);
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('review: swipe right confirms a claim; routing can be kept', (tester) async {
    final lib = activityLibrary();
    await pumpApp(tester, setup: FakeSetup(library: lib));
    expect(find.text('2 to review'), findsOneWidget);
    await tester.tap(find.text('2 to review'));
    await tester.pumpAndSettle();
    expect(find.text('Headaches follow short nights'), findsOneWidget);

    await tester.drag(find.text('Headaches follow short nights'), const Offset(300, 0));
    await tester.pumpAndSettle();
    expect(lib.resolved.single, ('card-1', ReviewAction.confirm, null));
    expect(find.text('Filed to Health — right?'), findsOneWidget);

    await tester.tap(find.text('Right'));
    await tester.pumpAndSettle();
    expect(lib.resolved.last.$2, ReviewAction.dismiss);
    expect(find.text('Nothing to review.'), findsOneWidget);
  });

  testWidgets('review: edit a claim before confirming; swipe left rejects in Persian too', (tester) async {
    final lib = activityLibrary();
    await pumpApp(tester, setup: FakeSetup(library: lib), location: '/review');
    await tester.tap(find.text('Edit'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'Headaches often follow short nights');
    await tester.tap(find.text('Confirm').last);
    await tester.pumpAndSettle();
    expect(lib.resolved.single, ('card-1', ReviewAction.confirm, 'Headaches often follow short nights'));

    final fa = activityLibrary();
    await pumpApp(tester, setup: FakeSetup(library: fa), location: '/review', prefs: prefsFor(language: 'fa', theme: 'light'));
    // In RTL, "forward" is to the left: dragging right rejects.
    await tester.drag(find.text('Headaches follow short nights'), const Offset(300, 0));
    await tester.pumpAndSettle();
    expect(fa.resolved.single.$2, ReviewAction.reject);
  });
}
