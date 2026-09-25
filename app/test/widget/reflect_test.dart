import 'package:daftar/core/library_api.dart';
import 'package:daftar/design/design.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

Finder switchIn(String title) => find.descendant(
  of: find.ancestor(of: find.text(title), matching: find.byType(DListRow)),
  matching: find.byType(DSwitch),
);

void main() {
  testWidgets('Settings › Reflect saves to the synced config', (tester) async {
    final lib = FakeLibrary();
    final notes = FakeNotifications();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      notifications: notes,
      location: '/settings',
    );

    await scrollTo(tester, find.text('Daily reflection'));
    await tester.tap(switchIn('Daily reflection'));
    await tester.pumpAndSettle();
    expect(lib.prefs.daily, isFalse);
    expect(find.text('Time'), findsNothing, reason: 'no time without daily');

    await scrollTo(tester, find.text('Day'));
    await tester.tap(find.text('Day'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sunday'));
    await tester.pumpAndSettle();
    expect(lib.prefs.weeklyDay, 7);

    await scrollTo(tester, find.text('Notifications'));
    await tester.tap(switchIn('Notifications'));
    await tester.pumpAndSettle();
    expect(lib.prefs.notifications, isFalse);
    expect(notes.asked, 0, reason: 'turning off never asks the OS');
    notes.granted = false;
    await tester.tap(switchIn('Notifications'));
    await tester.pump();
    await tester.pump();
    expect(lib.prefs.notifications, isTrue);
    expect(notes.asked, 1);
    expect(
      find.text(
        'Notifications are turned off for this app in system settings.',
      ),
      findsOneWidget,
    );
    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();

    await scrollTo(tester, find.text('Helpline country'));
    await tester.tap(find.text('Helpline country'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Germany'));
    await tester.pumpAndSettle();
    expect(lib.prefs.helplineCountry, 'DE');
    expect(find.text('Germany'), findsOneWidget);
  });

  testWidgets('Check the wiki now reports what it found', (tester) async {
    final lib = FakeLibrary();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      location: '/settings',
    );
    await scrollTo(tester, find.text('Check the wiki now'));
    await tester.tap(find.text('Check the wiki now'));
    await tester.pump();
    await tester.pump();
    expect(lib.lints, 1);
    expect(find.text('3 things to look at · 1 new in Review'), findsOneWidget);
    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
  });

  testWidgets(
    'finished reflections post notifications and, after a heavy day, the help card',
    (tester) async {
      final lib = FakeLibrary()
        ..nextRun = const RunSummary(
          jobs: [JobOutcome(kind: JobKindDto.other, state: JobStateDto.done)],
          pending: false,
        )
        ..signals = const ReflectSignals(
          notifications: ['Your daily reflection is ready.'],
          needsHelp: true,
        );
      final notes = FakeNotifications();
      await pumpApp(
        tester,
        setup: FakeSetup(library: lib),
        notifications: notes,
      );
      expect(lib.scheduled, 1, reason: 'due work is queued on open');
      expect(notes.shown, ['Your daily reflection is ready.']);
      expect(find.text('Talk to someone'), findsOneWidget);

      await tester.tap(find.bySemanticsLabel('Close'));
      await tester.pumpAndSettle();
      expect(find.text('Talk to someone'), findsNothing);
    },
  );
}
