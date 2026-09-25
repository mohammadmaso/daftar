import 'package:daftar/features/wiki/markdown_view.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';
import '../wiki_fixtures.dart';

const journal = 'vaults/life/journal/2026/2026-09-23.md';

void main() {
  testWidgets('the Wiki tab searches and opens pages', (tester) async {
    final lib = wikiLibrary();
    await pumpApp(tester, setup: FakeSetup(library: lib));
    await tester.tap(find.text('Wiki').last);
    await tester.pumpAndSettle();
    expect(find.text('Recently updated'), findsOneWidget);
    expect(find.text('Health profile'), findsWidgets, reason: 'pinned profile');

    await tester.enterText(find.byType(EditableText), 'postgres');
    await tester.pump(const Duration(milliseconds: 200));
    await tester.pumpAndSettle();
    expect(find.text('Journal · 23 Sep'), findsOneWidget);
    expect(find.text('Sara'), findsNothing);

    await tester.tap(find.text('Journal · 23 Sep'));
    await tester.pumpAndSettle();
    expect(find.byType(MarkdownView), findsOneWidget);
    expect(
      find.text('journal-day'),
      findsOneWidget,
      reason: 'property header, not raw YAML',
    );
    expect(
      find.text('Edited on two devices'),
      findsNothing,
      reason: 'a titled callout shows its own title',
    );
    expect(find.text('From laptop'), findsOneWidget);
  });

  testWidgets(
    'wikilinks open pages; missing pages say so; backlinks list linkers',
    (tester) async {
      final lib = wikiLibrary();
      await pumpApp(
        tester,
        setup: FakeSetup(library: lib),
        location: '/wiki/page?path=${Uri.encodeQueryComponent(journal)}',
      );
      await scrollTo(tester, find.text('Linked from'));
      expect(find.text('Sara'), findsOneWidget, reason: 'backlink row');

      final rich = find.byWidgetPredicate(
        (w) => w is RichText && w.text.toPlainText().contains('nowhere'),
      );
      await scrollTo(tester, rich);
      await tester.tapOnText(find.textRange.ofSubstring('nowhere'));
      await tester.pump();
      expect(find.text('This page does not exist yet.'), findsOneWidget);
      await tester.pump(const Duration(seconds: 3));

      await tester.tapOnText(find.textRange.ofSubstring('Health profile'));
      await tester.pumpAndSettle();
      expect(find.text('proposed'), findsOneWidget);
      expect(find.text('confirmed'), findsOneWidget);
      expect(find.text('superseded'), findsOneWidget);
      expect(
        find.textContaining('(status::'),
        findsNothing,
        reason: 'inline fields are chips, not raw text',
      );
    },
  );

  testWidgets('editing saves one human edit and reports a stale page', (
    tester,
  ) async {
    final lib = wikiLibrary();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      location:
          '/wiki/page?path=${Uri.encodeQueryComponent('vaults/life/people/sara.md')}',
    );
    await tester.tap(find.bySemanticsLabel('Edit'));
    await tester.pumpAndSettle();
    final field = find.byType(EditableText);
    final text = tester.widget<EditableText>(field).controller.text;
    await tester.enterText(
      field,
      text.replaceFirst('Lives in Tehran.', 'Lives in Shiraz.'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(lib.saved.single.$2, contains('Lives in Shiraz.'));
    expect(
      find.textContaining('Lives in Shiraz.'),
      findsOneWidget,
      reason: 'back on the reader, updated',
    );

    await tester.tap(find.bySemanticsLabel('Edit'));
    await tester.pumpAndSettle();
    lib.addPage(
      'vaults/life/people/sara.md',
      'Sara',
      'سارا',
      'Changed on another device.',
      kind: 'person',
    );
    await tester.enterText(find.byType(EditableText), 'mine');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(
      find.textContaining('changed while you were editing'),
      findsOneWidget,
    );
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets('the local graph shows neighbours', (tester) async {
    final lib = wikiLibrary();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      location: '/wiki/page?path=${Uri.encodeQueryComponent(journal)}',
    );
    await tester.tap(find.bySemanticsLabel('Nearby pages'));
    await tester.pumpAndSettle();
    expect(find.text('Nearby pages'), findsWidgets);
    expect(find.text('Sara'), findsWidgets);
  });
}
