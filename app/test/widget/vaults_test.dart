import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

void main() {
  Future<FakeLibrary> open(WidgetTester tester) async {
    final lib = FakeLibrary();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      location: '/settings',
    );
    return lib;
  }

  Future<void> tapRow(WidgetTester tester, String text) async {
    await scrollTo(tester, find.text(text));
    await tester.tap(find.text(text));
    await tester.pumpAndSettle();
  }

  Future<void> press(WidgetTester tester, String label) async {
    await tester.ensureVisible(find.text(label));
    await tester.tap(find.text(label));
    await tester.pumpAndSettle();
  }

  testWidgets('add a vault with a name and what belongs in it', (tester) async {
    final lib = await open(tester);
    await tapRow(tester, 'Add vault');
    await press(tester, 'Save');
    expect(find.text('Give the vault a name.'), findsOneWidget);

    await tester.enterText(find.byType(EditableText).at(0), 'Travel');
    await tester.enterText(find.byType(EditableText).at(1), 'سفر');
    await press(tester, 'Save');
    expect(find.text('Say what belongs in this vault.'), findsOneWidget);

    await tester.enterText(
      find.byType(EditableText).at(2),
      'Trips, visas, packing lists.',
    );
    await press(tester, 'Save');
    final v = lib.vaultList.last;
    expect(
      (v.id, v.titleFa, v.purpose),
      ('travel', 'سفر', 'Trips, visas, packing lists.'),
    );
    expect((await lib.vaults()).map((v) => v.id), contains('travel'));
    await scrollTo(tester, find.text('Travel'));
    expect(find.text('Empty · Trips, visas, packing lists.'), findsOneWidget);
  });

  testWidgets('Life can be renamed but not archived or removed', (
    tester,
  ) async {
    final lib = await open(tester);
    await tapRow(tester, 'Life');
    expect(find.text('Archive'), findsNothing);
    expect(find.text('Remove'), findsNothing);
    await tester.enterText(find.byType(EditableText).at(0), 'Daily');
    await press(tester, 'Save');
    expect(lib.vaultList.first.titleEn, 'Daily');
  });

  testWidgets('a vault with pages is archived, an empty one removed', (
    tester,
  ) async {
    final lib = await open(tester);
    await tapRow(tester, 'Health');
    expect(find.text('Remove'), findsNothing);
    await press(tester, 'Archive');
    expect(lib.vaultList.firstWhere((v) => v.id == 'health').archived, isTrue);
    expect((await lib.vaults()).map((v) => v.id), isNot(contains('health')));
    await scrollTo(tester, find.text('Archived'));

    await tapRow(tester, 'Mind');
    await press(tester, 'Remove');
    expect(lib.vaultList.map((v) => v.id), isNot(contains('mind')));
  });
}
