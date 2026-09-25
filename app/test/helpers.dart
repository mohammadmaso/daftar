import 'package:daftar/app/app.dart';
import 'package:daftar/app/appearance.dart';
import 'package:daftar/app/core.dart';
import 'package:daftar/core/credentials.dart';
import 'package:daftar/core/job_runner.dart';
import 'package:daftar/core/library_state.dart';
import 'package:daftar/core/recorder.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'fakes.dart';

const fakeCoreInfo = CoreInfo(
  appName: 'Daftar',
  version: '0.1.0',
  repoSchemaVersion: 1,
  target: 'linux-x86_64',
);

const phone = Size(390, 844);
const desktop = Size(1280, 800);

/// Fixed "now" so goldens are stable: Wed 23 Sep 2026, 14:05.
final fixedNow = DateTime(2026, 9, 23, 14, 5);

/// Pumps the real app with deterministic preferences, fake core/library and a fixed surface.
Future<ProviderContainer> pumpApp(
  WidgetTester tester, {
  Map<String, Object> prefs = const {},
  Size size = phone,
  Brightness platformBrightness = Brightness.light,
  FakeSetup? setup,
  FakeRecorder? recorder,
  FakeProviderApi? providerApi,
  MemoryCredentialStore? credentials,
  String? location,
}) async {
  SharedPreferences.setMockInitialValues(prefs);
  final sp = await SharedPreferences.getInstance();
  tester.view.physicalSize = size * 2;
  tester.view.devicePixelRatio = 2;
  tester.platformDispatcher.platformBrightnessTestValue = platformBrightness;
  tester.platformDispatcher.localesTestValue = const [Locale('en')];
  addTearDown(tester.view.reset);
  addTearDown(tester.platformDispatcher.clearAllTestValues);

  final container = ProviderContainer(
    overrides: [
      sharedPreferencesProvider.overrideWithValue(sp),
      coreInfoProvider.overrideWithValue(fakeCoreInfo),
      libraryRootProvider.overrideWith((ref) async => '/tmp/daftar-test'),
      setupApiProvider.overrideWithValue(
        setup ?? FakeSetup(library: FakeLibrary()),
      ),
      credentialStoreProvider.overrideWithValue(
        credentials ?? MemoryCredentialStore(),
      ),
      providerApiProvider.overrideWithValue(providerApi ?? FakeProviderApi()),
      voiceRecorderProvider.overrideWithValue(recorder ?? FakeRecorder()),
      clockProvider.overrideWithValue(() => fixedNow),
    ],
  );
  addTearDown(container.dispose);
  await tester.pumpWidget(
    UncontrolledProviderScope(container: container, child: const DaftarApp()),
  );
  await tester.pumpAndSettle();
  if (location != null) {
    container.read(routerProvider).go(location);
    await tester.pumpAndSettle();
  }
  return container;
}

Map<String, Object> prefsFor({
  required String language,
  required String theme,
}) => {'appearance.language': language, 'appearance.theme': theme};

/// Scrolls the first scrollable until [finder] is built and visible, like a user would.
Future<void> scrollTo(WidgetTester tester, Finder finder) async {
  await tester.scrollUntilVisible(
    finder,
    200,
    scrollable: find.byType(Scrollable).first,
  );
  await tester.pumpAndSettle();
}

/// Scrolls back up until [finder] is built and visible.
Future<void> scrollUp(WidgetTester tester, Finder finder) async {
  await tester.scrollUntilVisible(
    finder,
    -200,
    scrollable: find.byType(Scrollable).first,
  );
  await tester.pumpAndSettle();
}
