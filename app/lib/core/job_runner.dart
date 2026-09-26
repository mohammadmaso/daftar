import 'dart:async';

import 'dart:ui' show Locale, PlatformDispatcher;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart' show basicLocaleListResolution;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../app/appearance.dart';
import '../app/identity.dart';
import '../l10n/app_localizations.dart';
import 'credentials.dart';
import 'errors.dart';
import 'library_api.dart';
import 'library_state.dart';
import 'notifications.dart';

/// Provider settings for the open library; re-read whenever repository content may have changed.
final aiSettingsProvider = FutureProvider<AiSettings?>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib?.aiSettings();
});

final providerApiProvider = Provider<ProviderApi>(
  (ref) => const RustProviderApi(),
);

@immutable
class JobRunnerView {
  const JobRunnerView({this.running = false, this.waitingFor, this.error});

  final bool running;

  /// Filing is paused until a model is set up for this role.
  final ModelRole? waitingFor;

  /// Last unexpected failure of the runner itself (not of a single job).
  final String? error;
}

/// A reflection saw signs of a heavy day (§4.7): Today shows the Talk to someone card until the
/// person closes it. Only this yes/no is kept, in device preferences, so a reflection that ran in
/// the background still brings the card up; nothing about the day itself is stored.
final helpCardProvider = NotifierProvider<HelpCard, bool>(HelpCard.new);

const helpPendingKey = 'reflect.helpPending';

class HelpCard extends Notifier<bool> {
  SharedPreferences get _prefs => ref.read(sharedPreferencesProvider);

  @override
  bool build() => _prefs.getBool(helpPendingKey) ?? false;

  void show() {
    state = true;
    _prefs.setBool(helpPendingKey, true);
  }

  void close() {
    state = false;
    _prefs.remove(helpPendingKey);
  }

  /// Picks up a flag set by the background task, which writes through its own preferences.
  Future<void> reload() async {
    await _prefs.reload();
    state = _prefs.getBool(helpPendingKey) ?? false;
  }
}

/// The strings the app would show right now, for work that runs without a widget context.
L10n currentL10n(Ref ref) => l10nFor(ref.read(appearanceProvider).locale);

/// Strings for the chosen language, or the system's when [chosen] is null.
L10n l10nFor(Locale? chosen) => lookupL10n(
  chosen ??
      basicLocaleListResolution(
        PlatformDispatcher.instance.locales,
        supportedLocales,
      ),
);

final jobRunnerProvider = NotifierProvider<JobRunner, JobRunnerView>(
  JobRunner.new,
);

/// Runs the AI job queue (transcribe → describe → ingest) while the app is in the foreground:
/// after a capture, after a sync brought something in, when settings change, on resume and on a
/// slow timer while jobs are still pending (retries with backoff live in the core queue).
/// Reflections and lint are queued when due (§4.6): on open, on resume and every few minutes while
/// the app stays open, so a 21:30 reflection happens even if the app was left open all evening.
class JobRunner extends Notifier<JobRunnerView> {
  static const retryEvery = Duration(minutes: 1);
  static const dueEvery = Duration(minutes: 10);

  Timer? _timer;
  Timer? _due;
  Future<void>? _running;
  bool _again = false;
  bool _active = true;

  @override
  JobRunnerView build() {
    ref.onDispose(() {
      _timer?.cancel();
      _due?.cancel();
    });
    return const JobRunnerView();
  }

  void pause() {
    _active = false;
    _timer?.cancel();
    _due?.cancel();
  }

  /// Starts (or restarts after a pause): queues what is due, then runs the queue.
  Future<void> resume() async {
    _active = true;
    _due?.cancel();
    _due = Timer.periodic(dueEvery, (_) => _scheduleDue());
    await _scheduleDue();
  }

  Future<void> _scheduleDue() async {
    try {
      final lib = await ref.read(libraryProvider.future);
      await lib?.scheduleDue();
    } catch (e) {
      debugPrint('scheduling reflections failed: ${humanError(e)}');
    }
    if (_active) await kick();
  }

  /// Asks for a run soon. Concurrent requests collapse into one follow-up run.
  Future<void> kick() {
    if (_running != null) {
      _again = true;
      return _running!;
    }
    return _running = _loop().whenComplete(() => _running = null);
  }

  Future<void> _loop() async {
    do {
      _again = false;
      await _once();
    } while (_again && _active);
  }

  Future<void> _once() async {
    final lib = await ref.read(libraryProvider.future);
    if (lib == null || !_active) return;
    _timer?.cancel();
    state = JobRunnerView(running: true, waitingFor: state.waitingFor);
    try {
      final settings = await lib.aiSettings();
      final keys = await ref
          .read(credentialStoreProvider)
          .apiKeys(settings.providers.map((p) => p.id));
      final summary = await lib.runJobs(keys);
      final waiting = summary.jobs
          .map((j) => j.waitingFor)
          .whereType<ModelRole>()
          .firstOrNull;
      state = JobRunnerView(waitingFor: waiting);
      if (summary.jobs.isNotEmpty) {
        ref.read(revisionProvider.notifier).bump();
        await _signals(lib);
      }
      if (summary.jobs.any((j) => j.state == JobStateDto.done)) {
        ref.read(syncControllerProvider.notifier).changed();
      }
      if (summary.pending && waiting == null && _active) {
        _timer = Timer(retryEvery, kick);
      }
    } catch (e) {
      state = JobRunnerView(waitingFor: state.waitingFor, error: humanError(e));
    }
  }

  /// Posts what the reflections left: neutral notifications and, if needed, the help card.
  Future<void> _signals(LibraryApi lib) async {
    final signals = await lib.takeReflectSignals();
    if (signals.needsHelp) ref.read(helpCardProvider.notifier).show();
    await showReflectNotes(
      ref.read(systemNotificationsProvider),
      currentL10n(ref),
      signals.notifications,
    );
  }
}

/// Shows reflection notifications, titled with the app's name. A failure only loses the notice;
/// the reflection itself is already filed.
Future<void> showReflectNotes(
  SystemNotifications notes,
  L10n l,
  List<String> bodies,
) async {
  final title = AppIdentity.name(Locale(l.localeName));
  for (final body in bodies) {
    try {
      await notes.show(
        title,
        body,
        channel: l.reflectChannel,
        openLabel: l.open,
      );
    } catch (e) {
      debugPrint('notification failed: ${humanError(e)}');
    }
  }
}
