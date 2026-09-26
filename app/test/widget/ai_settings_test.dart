import 'package:daftar/core/credentials.dart';
import 'package:daftar/core/job_runner.dart';
import 'package:daftar/core/library_api.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

Future<void> _scrollTo(WidgetTester tester, String text) async {
  await tester.scrollUntilVisible(
    find.text(text),
    200,
    scrollable: find.byType(Scrollable).first,
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('adding a provider keeps the key in secure storage only', (
    tester,
  ) async {
    final lib = FakeLibrary();
    final creds = MemoryCredentialStore();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      credentials: creds,
      location: '/settings',
    );

    await _scrollTo(tester, 'Add provider');
    await tester.tap(find.text('Add provider'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText).at(0), 'OpenRouter');
    await tester.enterText(
      find.byType(EditableText).at(1),
      'https://openrouter.ai/api/v1',
    );
    await tester.enterText(find.byType(EditableText).at(2), 'sk-or-test');
    await tester.tap(find.text('Check'));
    await tester.pumpAndSettle();
    expect(find.text('Connected · 3 models'), findsOneWidget);
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(lib.providers.single.name, 'OpenRouter');
    expect(lib.providers.single.kind, ProviderKindDto.openaiCompatible);
    expect(creds.keys, {'p1': 'sk-or-test'});
    expect(find.text('OpenAI-compatible · openrouter.ai'), findsOneWidget);
  });

  testWidgets(
    'a role picks a model from /models and the Test button makes a real call',
    (tester) async {
      final lib = FakeLibrary();
      await lib.saveProvider(
        const AiProvider(
          id: 'p1',
          name: 'OpenRouter',
          kind: ProviderKindDto.openaiCompatible,
          baseUrl: '',
          headers: [],
          timeoutS: 60,
        ),
      );
      final creds = MemoryCredentialStore()..keys['p1'] = 'sk';
      await pumpApp(
        tester,
        setup: FakeSetup(library: lib),
        credentials: creds,
        location: '/settings',
      );

      await _scrollTo(tester, 'Models');
      await _scrollTo(tester, 'Chat');
      expect(find.text('Not set'), findsWidgets);
      await tester.tap(find.text('Chat'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('gpt-5-mini'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Test'));
      await tester.pumpAndSettle();

      expect(lib.roles[ModelRole.chat], ('p1', 'gpt-5-mini'));
      expect(lib.probed.single.$1, ModelRole.chat);
      expect(lib.probed.single.$2.single.key, 'sk');
      expect(find.text('Works · 840 ms · OK'), findsOneWidget);

      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(find.text('OpenRouter · gpt-5-mini'), findsOneWidget);
      expect(
        find.text('Uses Chat'),
        findsWidgets,
        reason: 'routing and filing fall back to Chat',
      );
    },
  );

  testWidgets('a failing test shows one human sentence', (tester) async {
    final lib = FakeLibrary()
      ..nextProbe = const ProbeOutcome(
        ok: false,
        latencyMs: 120,
        detail: 'OpenRouter rejected the API key.',
      );
    await lib.saveProvider(
      const AiProvider(
        id: 'p1',
        name: 'OpenRouter',
        kind: ProviderKindDto.anthropic,
        baseUrl: '',
        headers: [],
        timeoutS: 60,
      ),
    );
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      location: '/settings',
    );
    await _scrollTo(tester, 'Photos');
    await tester.tap(find.text('Photos'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText).first, 'whisper-1');
    await tester.pumpAndSettle();
    expect(find.text("This model probably can't read images."), findsOneWidget);
    await tester.tap(find.text('Test'));
    await tester.pumpAndSettle();
    expect(find.text('OpenRouter rejected the API key.'), findsOneWidget);
  });

  testWidgets(
    'filed captures say where they went; failed ones can be retried',
    (tester) async {
      final lib = FakeLibrary(
        captures: [
          const Capture(
            id: 'A',
            kind: RawKind.voice,
            capturedAt: '2026-09-23T14:05:00+03:30',
            device: 'pixel-8',
            text: 'سر درد داشتم',
            images: [],
            stage: Stage.filed,
            filing: Filing(
              opId: 'op',
              vaults: ['life', 'health'],
              pagesCreated: 1,
              pagesUpdated: 3,
              claimsToReview: 1,
              toReview: 1,
            ),
          ),
          const Capture(
            id: 'B',
            kind: RawKind.text,
            capturedAt: '2026-09-23T14:06:00+03:30',
            device: 'pixel-8',
            text: 'note',
            images: [],
            stage: Stage.failed,
            problem: 'OpenRouter rejected the API key.',
          ),
        ],
      );
      await pumpApp(tester, setup: FakeSetup(library: lib));
      expect(
        find.text(
          'Filed to Life · Health — 4 pages updated, 1 claim to review',
        ),
        findsOneWidget,
      );
      await tester.tap(find.text('Retry'));
      await tester.pumpAndSettle();
      expect(lib.retried, ['B']);
    },
  );

  testWidgets('Persian filing line', (tester) async {
    final lib = FakeLibrary(
      captures: [
        const Capture(
          id: 'A',
          kind: RawKind.voice,
          capturedAt: '2026-09-23T14:05:00+03:30',
          device: 'pixel-8',
          text: 'سر درد داشتم',
          images: [],
          stage: Stage.filed,
          filing: Filing(
            opId: 'op',
            vaults: ['life', 'health'],
            pagesCreated: 0,
            pagesUpdated: 2,
            claimsToReview: 1,
            toReview: 1,
          ),
        ),
      ],
    );
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      prefs: prefsFor(language: 'fa', theme: 'light'),
    );
    expect(
      find.text(
        'بایگانی شد در زندگی · سلامت — ۲ صفحه به‌روز شد، ۱ ادعا برای بازبینی',
      ),
      findsOneWidget,
    );
  });

  testWidgets(
    'the job runner runs after a capture and reports a missing model calmly',
    (tester) async {
      final lib = FakeLibrary()
        ..nextRun = const RunSummary(
          jobs: [
            JobOutcome(
              kind: JobKindDto.transcribe,
              state: JobStateDto.queued,
              waitingFor: ModelRole.stt,
            ),
          ],
          pending: true,
        );
      final c = await pumpApp(tester, setup: FakeSetup(library: lib));
      final before = lib.runs;
      await tester.tap(find.bySemanticsLabel('Hold to record, tap to type'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(EditableText), 'hello');
      await tester.pumpAndSettle();
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 3));
      expect(lib.runs, greaterThan(before));
      expect(c.read(jobRunnerProvider).waitingFor, ModelRole.stt);
      expect(
        find.text(
          'Filing waits for a Speech to text model. Set it up in Settings.',
        ),
        findsOneWidget,
      );
    },
  );
}
