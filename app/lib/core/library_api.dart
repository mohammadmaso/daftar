import 'dart:typed_data';

import '../src/rust/api/library.dart' as rs;

export '../src/rust/api/library.dart'
    show
        Auth,
        AuthKind,
        Capture,
        RawKind,
        RepoStatus,
        SshKeyPair,
        Stage,
        SyncResult,
        SyncState,
        Vault;

/// The app's view of one open library. Implemented over the Rust core; faked in widget tests.
abstract class LibraryApi {
  Future<List<rs.Capture>> day(DateTime date);
  Future<String> captureText(String text, {String? vault});
  Future<String> capturePhoto(Uint8List bytes, {String? note, String? vault});
  Future<String> captureVoice(String audioPath, {String? vault});
  Future<bool> discard(String id);
  Future<rs.RepoStatus> status();
  Future<rs.SyncResult> sync(rs.Auth auth);
  Future<void> setRemote(String url);
  Future<List<rs.Vault>> vaults();
}

/// Library creation and lookup (before a library is open).
abstract class SetupApi {
  bool ready(String root);
  Future<void> initLocal(
    String root, {
    required String deviceName,
    required String platform,
  });
  Future<void> clone({
    required String url,
    required String root,
    required rs.Auth auth,
    required String branch,
    required String deviceName,
    required String platform,
  });
  Future<rs.SshKeyPair> generateSshKey(String comment);
  Future<LibraryApi> open(String root);
}

class RustSetupApi implements SetupApi {
  const RustSetupApi();

  @override
  bool ready(String root) => rs.libraryReady(root: root);

  @override
  Future<void> initLocal(
    String root, {
    required String deviceName,
    required String platform,
  }) => rs.initLibrary(root: root, deviceName: deviceName, platform: platform);

  @override
  Future<void> clone({
    required String url,
    required String root,
    required rs.Auth auth,
    required String branch,
    required String deviceName,
    required String platform,
  }) => rs.cloneLibrary(
    url: url,
    root: root,
    auth: auth,
    branch: branch,
    deviceName: deviceName,
    platform: platform,
  );

  @override
  Future<rs.SshKeyPair> generateSshKey(String comment) =>
      rs.generateSshKey(comment: comment);

  @override
  Future<LibraryApi> open(String root) async =>
      RustLibraryApi(await rs.LibraryHandle.open(root: root));
}

class RustLibraryApi implements LibraryApi {
  RustLibraryApi(this._h);
  final rs.LibraryHandle _h;

  @override
  Future<List<rs.Capture>> day(DateTime d) =>
      _h.day(year: d.year, month: d.month, day: d.day);

  @override
  Future<String> captureText(String text, {String? vault}) =>
      _h.captureText(text: text, vaultHint: vault);

  @override
  Future<String> capturePhoto(Uint8List bytes, {String? note, String? vault}) =>
      _h.capturePhoto(bytes: bytes, note: note, vaultHint: vault);

  @override
  Future<String> captureVoice(String audioPath, {String? vault}) =>
      _h.captureVoice(audioPath: audioPath, vaultHint: vault);

  @override
  Future<bool> discard(String id) => _h.discard(id: id);

  @override
  Future<rs.RepoStatus> status() => _h.status();

  @override
  Future<rs.SyncResult> sync(rs.Auth auth) => _h.sync_(auth: auth);

  @override
  Future<void> setRemote(String url) => _h.setRemote(url: url);

  @override
  Future<List<rs.Vault>> vaults() => _h.vaults();
}
