import 'dart:io';

import 'package:daftar/app/app.dart';
import 'package:daftar/app/appearance.dart';
import 'package:daftar/core/credentials.dart';
import 'package:daftar/core/library_api.dart';
import 'package:daftar/core/library_state.dart';
import 'package:daftar/src/rust/frb_generated.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

Future<void> pumpUntil(WidgetTester tester, Finder f, {Duration timeout = const Duration(seconds: 20)}) async {
  final end = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(end)) {
    await tester.pump(const Duration(milliseconds: 100));
    if (f.evaluate().isNotEmpty) return;
  }
  final errors = find.textContaining('rror').evaluate().map((e) => (e.widget as Text).data).join('; ');
  fail('timed out waiting for $f. Visible errors: $errors');
}

/// End-to-end on a real device/desktop with the real Rust core (no fakes).
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init());

  testWidgets('local start → text capture lands in the repository → syncs to a second device', (tester) async {
    final tmp = await Directory.systemTemp.createTemp('daftar-it-');
    addTearDown(() => tmp.delete(recursive: true));
    final remote = '${tmp.path}/remote.git';
    await Process.run('git', ['init', '--bare', '-b', 'main', remote]);

    SharedPreferences.setMockInitialValues({'appearance.language': 'en'});
    final prefs = await SharedPreferences.getInstance();
    final phoneRoot = '${tmp.path}/phone';
    final container = ProviderContainer(overrides: [
      sharedPreferencesProvider.overrideWithValue(prefs),
      libraryRootProvider.overrideWith((ref) async => phoneRoot),
      credentialStoreProvider.overrideWithValue(MemoryCredentialStore()),
    ]);
    addTearDown(container.dispose);
    await tester.pumpWidget(UncontrolledProviderScope(container: container, child: const DaftarApp()));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Start on this device for now'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'IT phone');
    await tester.tap(find.text('Continue'));
    await pumpUntil(tester, find.text('Today'));

    await tester.tap(find.bySemanticsLabel('Hold to record, tap to type'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(EditableText), 'جلسه با Sara خوب بود');
    await tester.pumpAndSettle();
    final sw = Stopwatch()..start();
    await tester.tap(find.text('Save'));
    await pumpUntil(tester, find.text('جلسه با Sara خوب بود'));
    sw.stop();

    final rawFiles = Directory('$phoneRoot/raw').listSync(recursive: true).whereType<File>().where((f) => f.path.endsWith('.md')).toList();
    expect(rawFiles, hasLength(1));
    final doc = rawFiles.single.readAsStringSync();
    expect(doc, contains('kind: text'));
    expect(doc, contains('device: "it-phone"'));
    expect(doc, contains('lang: ["fa", "en"]'));

    // Connect the local library to a remote and sync through the real core.
    final lib = await container.read(libraryProvider.future);
    await lib!.setRemote(remote);
    final r = await lib.sync(noAuth);
    expect(r.state, SyncState.synced);

    const setup = RustSetupApi();
    final laptopRoot = '${tmp.path}/laptop';
    await setup.clone(url: remote, root: laptopRoot, auth: noAuth, branch: 'main', deviceName: 'IT laptop', platform: 'linux');
    final laptop = await setup.open(laptopRoot);
    final today = await laptop.day(DateTime.now());
    expect(today.map((c) => c.text), contains('جلسه با Sara خوب بود'));
    debugPrint('capture UI round trip: ${sw.elapsedMilliseconds} ms');
  });
}
