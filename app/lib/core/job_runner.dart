import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'credentials.dart';
import 'errors.dart';
import 'library_api.dart';
import 'library_state.dart';

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

final jobRunnerProvider = NotifierProvider<JobRunner, JobRunnerView>(
  JobRunner.new,
);

/// Runs the AI job queue (transcribe → describe → ingest) while the app is in the foreground:
/// after a capture, after a sync brought something in, when settings change, on resume and on a
/// slow timer while jobs are still pending (retries with backoff live in the core queue).
class JobRunner extends Notifier<JobRunnerView> {
  static const retryEvery = Duration(minutes: 1);

  Timer? _timer;
  Future<void>? _running;
  bool _again = false;
  bool _active = true;

  @override
  JobRunnerView build() {
    ref.onDispose(() => _timer?.cancel());
    return const JobRunnerView();
  }

  void pause() {
    _active = false;
    _timer?.cancel();
  }

  void resume() {
    _active = true;
    kick();
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
}
