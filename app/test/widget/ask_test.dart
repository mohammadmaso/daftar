import 'package:daftar/core/library_api.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';
import '../wiki_fixtures.dart';

void main() {
  testWidgets(
    'answers stream in, cite pages that open, and can be saved to the wiki',
    (tester) async {
      final lib = wikiLibrary();
      await pumpApp(tester, setup: FakeSetup(library: lib));
      await tester.tap(find.text('Ask').last);
      await tester.pumpAndSettle();
      expect(
        find.text('Ask anything about what you have captured.'),
        findsOneWidget,
      );

      await tester.enterText(find.byType(EditableText), 'Who is Sara?');
      await tester.pump();
      await tester.tap(find.bySemanticsLabel('Send'));
      await tester.pumpAndSettle();
      expect(lib.questions.single.$1, 'Who is Sara?');
      expect(find.text('Who is Sara?'), findsOneWidget);
      expect(find.textContaining('Sara is your cousin'), findsOneWidget);

      await tester.tap(find.text('Save to wiki'));
      await tester.pumpAndSettle();
      expect(lib.savedAnswers.single.$1, 'Who is Sara?');
      await tester.pump(const Duration(seconds: 3));

      await tester.tapOnText(find.textRange.ofSubstring('Sara').last);
      await tester.pumpAndSettle();
      expect(
        find.text('My cousin. Lives in Tehran.'),
        findsOneWidget,
        reason: 'citation opened the page',
      );
    },
  );

  testWidgets('follow-ups carry history; scopes start a new conversation', (
    tester,
  ) async {
    final lib = wikiLibrary();
    lib.addPage(
      'vaults/stories/glass-city/characters/omid.md',
      'Omid',
      'امید',
      'Away at the harbour.',
      kind: 'character',
    );
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      location: '/ask',
    );
    for (final q in ['First', 'Second']) {
      await tester.enterText(find.byType(EditableText), q);
      await tester.pump();
      await tester.tap(find.bySemanticsLabel('Send'));
      await tester.pumpAndSettle();
    }
    expect(lib.questions.last.$3.single.question, 'First');

    await tester.ensureVisible(find.text('Story: glass-city'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Story: glass-city'));
    await tester.pumpAndSettle();
    expect(
      find.text('First'),
      findsNothing,
      reason: 'a story is its own conversation',
    );
    await tester.enterText(find.byType(EditableText), 'Does Omid know?');
    await tester.pump();
    await tester.tap(find.bySemanticsLabel('Send'));
    await tester.pumpAndSettle();
    expect(lib.questions.last.$2.kind, AskScopeKind.story);
    expect(lib.questions.last.$2.id, 'glass-city');
    expect(lib.questions.last.$3, isEmpty);
    await tester.tap(find.text('Save as draft'));
    await tester.pumpAndSettle();
    expect(lib.drafts.single.$1, 'glass-city');
    await tester.pump(const Duration(seconds: 3));
  });

  testWidgets(
    'a hard moment shows the Talk to someone card; failures are one sentence',
    (tester) async {
      final lib = wikiLibrary()
        ..nextAnswer = AskAnswer(
          text: 'I am here with you.',
          citations: const [],
          needsHelp: true,
          model: 'm',
          inputTokens: BigInt.zero,
          outputTokens: BigInt.zero,
        );
      await pumpApp(
        tester,
        setup: FakeSetup(library: lib),
        location: '/ask',
        prefs: prefsFor(language: 'fa', theme: 'light'),
      );
      await tester.enterText(find.byType(EditableText), 'دیگه نمی‌کشم');
      await tester.pump();
      await tester.tap(find.bySemanticsLabel('بفرست'));
      await tester.pumpAndSettle();
      expect(find.text('با کسی حرف بزن'), findsOneWidget);
      expect(
        find.textContaining('123'),
        findsOneWidget,
        reason: 'Iran defaults in Persian',
      );

      lib.nextFailure = 'OpenRouter rejected the API key.';
      await tester.enterText(find.byType(EditableText).last, 'again');
      await tester.pump();
      await tester.tap(find.bySemanticsLabel('بفرست'));
      await tester.pumpAndSettle();
      expect(find.text('OpenRouter rejected the API key.'), findsOneWidget);
    },
  );
}
