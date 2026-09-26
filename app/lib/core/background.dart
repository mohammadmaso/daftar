import 'dart:io';
import 'dart:ui' show Locale;

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:workmanager/workmanager.dart';

import '../app/appearance.dart';
import '../src/rust/frb_generated.dart';
import 'credentials.dart';
import 'errors.dart';
import 'job_runner.dart';
import 'library_api.dart';
import 'library_state.dart';
import 'notifications.dart';

/// Background work on Android and iOS (brief §3: best effort): every hour or so, when online,
/// sync, file what is waiting, run due reflections and post their notifications. The app does the
/// same in the foreground; this only covers the time it is closed. iOS decides when a refresh
/// runs. Desktops sync while the app is open.
abstract final class Background {
  /// Also the iOS BGTaskScheduler identifier (Info.plist, AppDelegate.swift).
  static const task = 'dev.daftar.daftar.background';
  static const every = Duration(hours: 1);

  static bool get supported => !kIsWeb && (Platform.isAndroid || Platform.isIOS);

  static Future<void> register() async {
    if (!supported) return;
    await Workmanager().initialize(backgroundDispatcher);
    await Workmanager().registerPeriodicTask(
      task,
      task,
      frequency: every,
      constraints: Constraints(networkType: NetworkType.connected),
      existingWorkPolicy: ExistingPeriodicWorkPolicy.keep,
    );
  }
}

@pragma('vm:entry-point')
void backgroundDispatcher() {
  Workmanager().executeTask((_, _) async {
    try {
      await RustLib.init();
      final prefs = await SharedPreferences.getInstance();
      return await backgroundPass(
        setup: const RustSetupApi(),
        root: await defaultLibraryRoot(),
        credentials: SecureCredentialStore(),
        notifications: LocalSystemNotifications(),
        prefs: prefs,
      );
    } catch (e) {
      debugPrint('background pass failed: ${humanError(e)}');
      return false;
    }
  });
}

/// One background pass; returns false to ask the OS to retry later.
Future<bool> backgroundPass({
  required SetupApi setup,
  required String root,
  required CredentialStore credentials,
  required SystemNotifications notifications,
  required SharedPreferences prefs,
}) async {
  if (!setup.ready(root)) return true;
  final lib = await setup.open(root);
  await lib.scheduleDue();

  final remote = (await lib.status()).hasRemote;
  final auth = remote ? await credentials.gitAuth() : null;
  if (auth != null) await lib.sync(auth);

  final settings = await lib.aiSettings();
  final keys = await credentials.apiKeys(settings.providers.map((p) => p.id));
  final summary = await lib.runJobs(keys);
  if (auth != null && summary.jobs.any((j) => j.state == JobStateDto.done)) {
    await lib.sync(auth);
  }

  final signals = await lib.takeReflectSignals();
  if (signals.needsHelp) await prefs.setBool(helpPendingKey, true);
  final lang = prefs.getString('appearance.language');
  await showReflectNotes(
    notifications,
    l10nFor(
      LanguagePref.values
          .where((v) => v.name == lang)
          .map((v) => v.locale)
          .firstOrNull,
    ),
    signals.notifications,
  );
  return true;
}

extension on LanguagePref {
  Locale? get locale => switch (this) {
    LanguagePref.system => null,
    LanguagePref.en => const Locale('en'),
    LanguagePref.fa => const Locale('fa'),
  };
}
