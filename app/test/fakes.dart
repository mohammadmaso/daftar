import 'dart:async';
import 'dart:typed_data';

import 'package:daftar/core/library_api.dart';
import 'package:daftar/core/recorder.dart';

class FakeLibrary implements LibraryApi {
  FakeLibrary({List<Capture>? captures, this.hasRemote = true}) : captures = captures ?? [];

  final List<Capture> captures;
  bool hasRemote;
  int syncs = 0;
  final texts = <String>[];

  Capture _add(RawKind kind, String text, String? vault, {List<String> images = const []}) {
    final c = Capture(
      id: '01K${captures.length.toString().padLeft(23, '0')}',
      kind: kind,
      capturedAt: '2026-09-23T14:05:00+03:30',
      device: 'pixel-8',
      text: text,
      vaultHint: vault,
      images: images,
      stage: Stage.saved,
    );
    captures.add(c);
    return c;
  }

  @override
  Future<List<Capture>> day(DateTime date) async => List.of(captures);

  @override
  Future<String> captureText(String text, {String? vault}) async {
    texts.add(text);
    return _add(RawKind.text, text, vault).id;
  }

  @override
  Future<String> capturePhoto(Uint8List bytes, {String? note, String? vault}) async =>
      _add(RawKind.photo, '', vault).id;

  @override
  Future<String> captureVoice(String audioPath, {String? vault}) async => _add(RawKind.voice, '', vault).id;

  @override
  Future<bool> discard(String id) async => true;

  @override
  Future<RepoStatus> status() async => RepoStatus(
        uncommitted: 0,
        unpushed: 0,
        hasRemote: hasRemote,
        branch: 'main',
        remoteUrl: hasRemote ? 'https://github.com/me/notes.git' : null,
        deviceId: 'pixel-8',
        deviceName: 'Pixel 8',
        root: '/data/daftar/libraries/default',
      );

  @override
  Future<SyncResult> sync(Auth auth) async {
    syncs++;
    return const SyncResult(state: SyncState.synced, pulled: 0, pushed: 0, conflicts: [], replays: 0, changedPaths: []);
  }

  @override
  Future<void> setRemote(String url) async => hasRemote = true;

  @override
  Future<List<Vault>> vaults() async => const [
        Vault(id: 'life', titleEn: 'Life', titleFa: 'زندگی', fiction: false),
        Vault(id: 'health', titleEn: 'Health', titleFa: 'سلامت', fiction: false),
        Vault(id: 'mind', titleEn: 'Mind', titleFa: 'ذهن', fiction: false),
        Vault(id: 'work', titleEn: 'Work', titleFa: 'کار', fiction: false),
        Vault(id: 'stories', titleEn: 'Stories', titleFa: 'داستان‌ها', fiction: true),
      ];
}

class FakeSetup implements SetupApi {
  FakeSetup({this.library});

  /// `null` means this device is not set up yet.
  FakeLibrary? library;
  String? clonedUrl;
  Auth? clonedAuth;
  String? deviceName;

  @override
  bool ready(String root) => library != null;

  @override
  Future<void> initLocal(String root, {required String deviceName, required String platform}) async {
    this.deviceName = deviceName;
    library = FakeLibrary(hasRemote: false);
  }

  @override
  Future<void> clone({
    required String url,
    required String root,
    required Auth auth,
    required String branch,
    required String deviceName,
    required String platform,
  }) async {
    clonedUrl = url;
    clonedAuth = auth;
    this.deviceName = deviceName;
    library = FakeLibrary();
  }

  @override
  Future<SshKeyPair> generateSshKey(String comment) async => SshKeyPair(
        privateOpenssh: '-----BEGIN OPENSSH PRIVATE KEY-----\nfake\n-----END OPENSSH PRIVATE KEY-----\n',
        publicOpenssh: 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFakeKeyForTestsOnly $comment',
      );

  @override
  Future<LibraryApi> open(String root) async => library!;
}

class FakeRecorder implements VoiceRecorder {
  bool permission = true;
  bool recording = false;
  bool cancelled = false;
  final _levels = StreamController<double>.broadcast();

  @override
  Future<bool> hasPermission() async => permission;
  @override
  Future<void> start() async => recording = true;
  @override
  Future<String?> stop() async {
    recording = false;
    return '/tmp/fake.m4a';
  }

  @override
  Future<void> cancel() async {
    recording = false;
    cancelled = true;
  }

  @override
  Stream<double> get levels => _levels.stream;
  @override
  Future<void> dispose() => _levels.close();
}
