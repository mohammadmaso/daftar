import 'dart:typed_data';

import '../src/rust/api/ai.dart' as ai;
import '../src/rust/api/library.dart' as rs;
import '../src/rust/api/wiki.dart' as wk;

export '../src/rust/api/ai.dart'
    show
        AiProvider,
        AiSettings,
        ApiKey,
        CapabilityWarning,
        HttpHeader,
        JobKindDto,
        JobOutcome,
        JobStateDto,
        ModelRole,
        ProbeOutcome,
        ProviderKindDto,
        RoleSetting,
        RunSummary;
export '../src/rust/api/library.dart'
    show
        Auth,
        AuthKind,
        Capture,
        Filing,
        RawKind,
        RepoStatus,
        SshKeyPair,
        Stage,
        SyncResult,
        SyncState,
        Vault;
export '../src/rust/api/wiki.dart'
    show
        FolderEntry,
        GraphEdge,
        GraphNode,
        Listing,
        LocalGraph,
        PageSummary,
        SaveResult,
        SearchHit,
        WikiPage;

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

  // AI providers and model roles (§9). Keys come from secure storage per call.
  Future<ai.AiSettings> aiSettings();
  Future<String> saveProvider(ai.AiProvider provider);
  Future<void> removeProvider(String id);
  Future<void> setRole(ai.ModelRole role, String providerId, String model);
  Future<void> clearRole(ai.ModelRole role);
  Future<ai.ProbeOutcome> testRole(ai.ModelRole role, List<ai.ApiKey> keys);
  Future<ai.RunSummary> runJobs(List<ai.ApiKey> keys);
  Future<void> retryCapture(String id);

  // Wiki (§8.1, §8.3).
  Future<List<wk.SearchHit>> search(
    String query, {
    List<String> vaults,
    int limit,
  });
  Future<List<wk.PageSummary>> recentPages({String? vault, int limit});
  Future<wk.Listing> listDir(String dir);
  Future<wk.LocalGraph> localGraph(String path, {int depth});
  Future<wk.WikiPage> page(String path);
  Future<String?> resolveLink(String target);
  Future<wk.SaveResult> savePage(String path, String baseHash, String text);
  Future<int> refreshIndex();
  Future<int> rebuildIndex();

  /// The library folder (for images under `raw/assets/`).
  Future<String> root();
}

/// Provider calls that need no open library.
abstract class ProviderApi {
  Future<List<String>> listModels(ai.AiProvider provider, String? apiKey);
  String defaultBaseUrl(ai.ProviderKindDto kind);
  ai.CapabilityWarning? capabilityWarning(ai.ModelRole role, String model);
}

class RustProviderApi implements ProviderApi {
  const RustProviderApi();

  @override
  Future<List<String>> listModels(ai.AiProvider provider, String? apiKey) =>
      ai.listModels(provider: provider, apiKey: apiKey);

  @override
  String defaultBaseUrl(ai.ProviderKindDto kind) =>
      ai.defaultBaseUrl(kind: kind);

  @override
  ai.CapabilityWarning? capabilityWarning(ai.ModelRole role, String model) =>
      ai.capabilityWarning(role: role, model: model);
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

  @override
  Future<ai.AiSettings> aiSettings() => _h.aiSettings();

  @override
  Future<String> saveProvider(ai.AiProvider provider) =>
      _h.saveProvider(provider: provider);

  @override
  Future<void> removeProvider(String id) => _h.removeProvider(id: id);

  @override
  Future<void> setRole(ai.ModelRole role, String providerId, String model) =>
      _h.setRole(role: role, providerId: providerId, model: model);

  @override
  Future<void> clearRole(ai.ModelRole role) => _h.clearRole(role: role);

  @override
  Future<ai.ProbeOutcome> testRole(ai.ModelRole role, List<ai.ApiKey> keys) =>
      _h.testRole(role: role, apiKeys: keys);

  @override
  Future<ai.RunSummary> runJobs(List<ai.ApiKey> keys) =>
      _h.runJobs(apiKeys: keys, online: true);

  @override
  Future<void> retryCapture(String id) => _h.retryCapture(rawId: id);

  @override
  Future<List<wk.SearchHit>> search(
    String query, {
    List<String> vaults = const [],
    int limit = 30,
  }) => _h.search(query: query, vaults: vaults, limit: limit);

  @override
  Future<List<wk.PageSummary>> recentPages({String? vault, int limit = 30}) =>
      _h.recentPages(vault: vault, limit: limit);

  @override
  Future<wk.Listing> listDir(String dir) => _h.listDir(dir: dir);

  @override
  Future<wk.LocalGraph> localGraph(String path, {int depth = 1}) =>
      _h.localGraph(path: path, depth: depth);

  @override
  Future<wk.WikiPage> page(String path) => _h.page(path: path);

  @override
  Future<String?> resolveLink(String target) => _h.resolveLink(target: target);

  @override
  Future<wk.SaveResult> savePage(String path, String baseHash, String text) =>
      _h.savePage(path: path, baseHash: baseHash, text: text);

  @override
  Future<int> refreshIndex() => _h.refreshIndex();

  @override
  Future<int> rebuildIndex() => _h.rebuildIndex();

  @override
  Future<String> root() async => (await _h.status()).root;
}
