import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import 'credentials.dart';
import 'errors.dart';
import 'job_runner.dart';
import 'library_api.dart';

/// Wall clock; overridden in tests for deterministic goldens.
final clockProvider = Provider<DateTime Function()>((ref) => DateTime.now);

/// Where this device keeps its checkout of the (single, for now) library.
final libraryRootProvider = FutureProvider<String>(
  (ref) => defaultLibraryRoot(),
);

Future<String> defaultLibraryRoot() async {
  final base = await getApplicationSupportDirectory();
  return '${base.path}${Platform.pathSeparator}libraries${Platform.pathSeparator}default';
}

final setupApiProvider = Provider<SetupApi>((ref) => const RustSetupApi());

final credentialStoreProvider = Provider<CredentialStore>(
  (ref) => SecureCredentialStore(),
);

String currentPlatform() => kIsWeb ? 'web' : Platform.operatingSystem;

/// The open library, or `null` when this device has not been set up yet.
final libraryProvider = AsyncNotifierProvider<LibraryNotifier, LibraryApi?>(
  LibraryNotifier.new,
);

class LibraryNotifier extends AsyncNotifier<LibraryApi?> {
  @override
  Future<LibraryApi?> build() async {
    final root = await ref.watch(libraryRootProvider.future);
    final setup = ref.watch(setupApiProvider);
    if (!setup.ready(root)) return null;
    return setup.open(root);
  }

  /// Called by onboarding once the library exists on disk.
  Future<void> reload() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(build);
  }
}

/// Bumped whenever repository content may have changed (capture, sync) so views re-read.
final revisionProvider = NotifierProvider<Revision, int>(Revision.new);

class Revision extends Notifier<int> {
  @override
  int build() => 0;
  void bump() => state++;
}

DateTime dateOnly(DateTime d) => DateTime(d.year, d.month, d.day);

final dayCapturesProvider = FutureProvider.family<List<Capture>, DateTime>((
  ref,
  day,
) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  if (lib == null) return const [];
  return lib.day(day);
});

final vaultsProvider = FutureProvider<List<Vault>>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib == null ? const [] : lib.vaults();
});

// ─────────────────────────── sync ───────────────────────────

enum SyncIndicator {
  synced,
  localChanges,
  syncing,
  offline,
  needsAttention,
  noRemote,
}

@immutable
class SyncView {
  const SyncView({
    this.indicator = SyncIndicator.synced,
    this.pending = 0,
    this.message,
    this.lastSynced,
  });

  final SyncIndicator indicator;

  /// Local changes not yet on the remote.
  final int pending;
  final String? message;
  final DateTime? lastSynced;

  SyncView copyWith({
    SyncIndicator? indicator,
    int? pending,
    String? message,
    DateTime? lastSynced,
  }) => SyncView(
    indicator: indicator ?? this.indicator,
    pending: pending ?? this.pending,
    message: message,
    lastSynced: lastSynced ?? this.lastSynced,
  );
}

final syncControllerProvider = NotifierProvider<SyncController, SyncView>(
  SyncController.new,
);

/// Opportunistic sync (§5.2): on foreground, after changes (debounced), periodically while the app
/// is in the foreground, and on demand. Being offline is normal and never alarming.
class SyncController extends Notifier<SyncView> {
  static const debounce = Duration(seconds: 10);
  static const period = Duration(minutes: 5);

  Timer? _debounce;
  Timer? _periodic;
  Future<void>? _running;

  @override
  SyncView build() {
    ref.onDispose(() {
      _debounce?.cancel();
      _periodic?.cancel();
    });
    return const SyncView();
  }

  void startPeriodic() {
    _periodic?.cancel();
    _periodic = Timer.periodic(period, (_) => syncNow());
  }

  void stopPeriodic() {
    _periodic?.cancel();
    _debounce?.cancel();
  }

  /// Something changed locally; sync soon.
  void changed() {
    state = state.copyWith(
      indicator: state.indicator == SyncIndicator.noRemote
          ? SyncIndicator.noRemote
          : SyncIndicator.localChanges,
      pending: state.pending + 1,
    );
    _debounce?.cancel();
    _debounce = Timer(debounce, syncNow);
  }

  Future<void> syncNow() =>
      _running ??= _run().whenComplete(() => _running = null);

  Future<void> _run() async {
    final lib = await ref.read(libraryProvider.future);
    if (lib == null) return;
    final status = await lib.status();
    if (!status.hasRemote) {
      state = SyncView(
        indicator: SyncIndicator.noRemote,
        pending: status.unpushed,
      );
      return;
    }
    state = state.copyWith(indicator: SyncIndicator.syncing);
    try {
      final auth = await ref.read(credentialStoreProvider).gitAuth();
      final r = await lib.sync(auth);
      final after = await lib.status();
      state = SyncView(
        indicator: switch (r.state) {
          SyncState.synced => SyncIndicator.synced,
          SyncState.localChanges => SyncIndicator.localChanges,
          SyncState.offline => SyncIndicator.offline,
          SyncState.needsAttention => SyncIndicator.needsAttention,
        },
        pending: after.unpushed,
        message: r.message,
        lastSynced: r.state == SyncState.synced
            ? DateTime.now()
            : state.lastSynced,
      );
      if (r.pulled > 0 || r.changedPaths.isNotEmpty) {
        ref.read(revisionProvider.notifier).bump();
      }
      // Pulled captures from other devices, or AI ops dropped for replay (§5.4), need filing.
      if (r.pulled > 0 || r.replays > 0) {
        unawaited(ref.read(jobRunnerProvider.notifier).kick());
      }
    } catch (e) {
      state = SyncView(
        indicator: SyncIndicator.needsAttention,
        pending: state.pending,
        message: humanError(e),
        lastSynced: state.lastSynced,
      );
    }
  }
}
