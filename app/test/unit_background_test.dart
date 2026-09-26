import 'package:daftar/core/background.dart';
import 'package:daftar/core/credentials.dart';
import 'package:daftar/core/job_runner.dart';
import 'package:daftar/core/library_api.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'fakes.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<SharedPreferences> prefs(Map<String, Object> values) async {
    SharedPreferences.setMockInitialValues(values);
    return SharedPreferences.getInstance();
  }

  test('a background pass syncs, files, reflects and notifies', () async {
    final lib = FakeLibrary()
      ..nextRun = const RunSummary(
        jobs: [JobOutcome(kind: JobKindDto.ingest, state: JobStateDto.done)],
        pending: false,
      )
      ..signals = const ReflectSignals(
        notifications: ['Your daily reflection is ready.'],
        needsHelp: true,
      );
    final notes = FakeNotifications();
    final p = await prefs({'appearance.language': 'fa'});
    final done = await backgroundPass(
      setup: FakeSetup(library: lib),
      root: '/tmp/x',
      credentials: MemoryCredentialStore(),
      notifications: notes,
      prefs: p,
    );
    expect(done, isTrue);
    expect(lib.scheduled, 1);
    expect(lib.runs, 1);
    expect(lib.syncs, 2, reason: 'pull first, push what was filed');
    expect(notes.shown, ['Your daily reflection is ready.']);
    expect(p.getBool(helpPendingKey), isTrue, reason: 'card on next open');
  });

  test('nothing to do on a device that is not set up', () async {
    expect(
      await backgroundPass(
        setup: FakeSetup(),
        root: '/tmp/x',
        credentials: MemoryCredentialStore(),
        notifications: FakeNotifications(),
        prefs: await prefs({}),
      ),
      isTrue,
    );
  });
}
