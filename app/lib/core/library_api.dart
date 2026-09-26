import 'dart:typed_data';

import '../src/rust/api/ai.dart' as ai;
import '../src/rust/api/ask.dart' as ak;
import '../src/rust/api/audit.dart' as au;
import '../src/rust/api/library.dart' as rs;
import '../src/rust/api/mcp.dart' as mc;
import '../src/rust/api/reflect.dart' as rf;
import '../src/rust/api/voice.dart' as vo;
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
export '../src/rust/api/ask.dart'
    show
        AnswerCitation,
        AskAnswer,
        AskEvent,
        AskEventKind,
        AskImage,
        AskScopeDto,
        AskScopeKind,
        AskTurn,
        Helpline,
        McpSecret,
        ToolApproval;
export '../src/rust/api/audit.dart'
    show
        CardKind,
        ClaimInfo,
        DiffLine,
        DiffLineKind,
        OpKind,
        Operation,
        PageChange,
        PageDiff,
        ReviewAction,
        ReviewCardDto,
        RouteTarget,
        UndoResult;
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
export '../src/rust/api/mcp.dart'
    show
        McpAuthKind,
        McpPolicy,
        McpServer,
        McpStatus,
        McpStatusKind,
        McpToolInfo,
        McpTransportKind,
        OAuthStart;
export '../src/rust/api/reflect.dart'
    show LintSummary, ReflectPrefs, ReflectSignals;
export '../src/rust/api/voice.dart'
    show VoiceEventDto, VoiceEventKind, VoiceOptions, VoiceStateDto;
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
        WikiGraph,
        WikiGraphNode,
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
  Future<wk.WikiGraph> wikiGraph();
  Future<wk.WikiPage> page(String path);
  Future<String?> resolveLink(String target);
  Future<wk.SaveResult> savePage(String path, String baseHash, String text);
  Future<int> refreshIndex();
  Future<int> rebuildIndex();

  /// The library folder (for images under `raw/assets/`).
  Future<String> root();

  // Activity and Review (§7, §8.5).
  Future<List<au.Operation>> activity({int limit, String? before});
  Future<au.Operation> operation(String opId);
  Future<List<au.PageDiff>> operationDiff(String opId);
  Future<au.UndoResult> undo(String opId);
  Future<au.UndoResult> moveToVault(String opId, String vault);
  Future<au.UndoResult> rerunWithNote(String opId, String note);
  Future<void> includeCapture(String rawId);
  Future<List<au.ReviewCardDto>> reviewCards();
  Future<void> resolveReview(
    String cardId,
    au.ReviewAction action, {
    String? editedText,
  });

  // Ask (§4.3).
  Stream<ak.AskEvent> ask(
    List<ak.AskTurn> history,
    String question,
    ak.AskScopeDto scope,
    List<ai.ApiKey> keys, {
    ak.AskImage? image,
    List<ak.McpSecret> mcp = const [],
  });
  Future<String> saveAnswer(
    String question,
    String answer,
    ak.AskScopeDto scope,
  );
  Future<String> saveDraft(String story, String title, String text);

  // Voice mode (§8.4).
  Future<VoiceConversation> startVoice(
    vo.VoiceOptions options,
    List<ai.ApiKey> keys,
  );

  // MCP servers (§10).
  Future<List<mc.McpServer>> mcpServers();
  Future<String> saveMcpServer(mc.McpServer server);
  Future<void> removeMcpServer(String id);
  Future<mc.McpStatus> mcpCheck(String id, String secretsJson);
  Future<mc.OAuthStart> mcpOauthBegin(
    String id,
    String secretsJson, {
    String? redirectUri,
  });
  Future<String> mcpOauthWait(String flowId);
  Future<String> mcpOauthComplete(String flowId, String callbackUrl);

  // Reflect and lint (§4.6, §6.5).
  Future<rf.ReflectPrefs> reflectPrefs();
  Future<void> setReflectPrefs(rf.ReflectPrefs prefs);

  /// Queues reflections and lint that are due now; returns how many jobs were added.
  Future<int> scheduleDue();

  /// Notifications and the crisis flag left by the jobs that just ran.
  Future<rf.ReflectSignals> takeReflectSignals();
  Future<rf.LintSummary> lintNow();
}

/// A running voice conversation: microphone PCM in, events (captions, audio, state) out.
abstract class VoiceConversation {
  Stream<vo.VoiceEventDto> get events;
  Future<void> feed(Int16List pcm);
  Future<void> playbackFinished();
  Future<void> setMuted(bool muted);

  /// Ends it; returns the id of the saved transcript capture, if any.
  Future<String?> end();
}

class _RustVoice implements VoiceConversation {
  _RustVoice(this._h) : events = _h.events().asBroadcastStream();
  final vo.VoiceHandle _h;

  @override
  final Stream<vo.VoiceEventDto> events;

  @override
  Future<void> feed(Int16List pcm) => _h.feed(pcm: pcm);

  @override
  Future<void> playbackFinished() => _h.playbackFinished();

  @override
  Future<void> setMuted(bool muted) => _h.setMuted(muted: muted);

  @override
  Future<String?> end() => _h.end();
}

/// Provider calls that need no open library.
abstract class ProviderApi {
  Future<List<String>> listModels(ai.AiProvider provider, String? apiKey);
  String defaultBaseUrl(ai.ProviderKindDto kind);
  ai.CapabilityWarning? capabilityWarning(ai.ModelRole role, String model);
  List<ak.Helpline> helplines(String country);
  bool get stdioSupported;
  void answerToolApproval(String requestId, bool allowed);
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

  @override
  List<ak.Helpline> helplines(String country) => ak.helplines(country: country);

  @override
  bool get stdioSupported => mc.mcpStdioSupported();

  @override
  void answerToolApproval(String requestId, bool allowed) =>
      mc.answerToolApproval(requestId: requestId, allowed: allowed);
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
  Future<wk.WikiGraph> wikiGraph() => _h.wikiGraph();

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

  @override
  Future<List<au.Operation>> activity({int limit = 50, String? before}) =>
      _h.activity(limit: limit, before: before);

  @override
  Future<au.Operation> operation(String opId) => _h.operation(opId: opId);

  @override
  Future<List<au.PageDiff>> operationDiff(String opId) =>
      _h.operationDiff(opId: opId);

  @override
  Future<au.UndoResult> undo(String opId) => _h.undo(opId: opId);

  @override
  Future<au.UndoResult> moveToVault(String opId, String vault) =>
      _h.moveToVault(opId: opId, vault: vault);

  @override
  Future<au.UndoResult> rerunWithNote(String opId, String note) =>
      _h.rerunWithNote(opId: opId, note: note);

  @override
  Future<void> includeCapture(String rawId) => _h.includeCapture(rawId: rawId);

  @override
  Future<List<au.ReviewCardDto>> reviewCards() => _h.reviewCards();

  @override
  Future<void> resolveReview(
    String cardId,
    au.ReviewAction action, {
    String? editedText,
  }) =>
      _h.resolveReview(cardId: cardId, action: action, editedText: editedText);

  @override
  Stream<ak.AskEvent> ask(
    List<ak.AskTurn> history,
    String question,
    ak.AskScopeDto scope,
    List<ai.ApiKey> keys, {
    ak.AskImage? image,
    List<ak.McpSecret> mcp = const [],
  }) => _h.ask(
    history: history,
    question: question,
    image: image,
    scopeDto: scope,
    apiKeys: keys,
    mcpSecrets: mcp,
  );

  @override
  Future<String> saveAnswer(
    String question,
    String answer,
    ak.AskScopeDto scope,
  ) => _h.saveAnswer(question: question, answer: answer, scopeDto: scope);

  @override
  Future<String> saveDraft(String story, String title, String text) =>
      _h.saveDraft(story: story, title: title, text: text);

  @override
  Future<VoiceConversation> startVoice(
    vo.VoiceOptions options,
    List<ai.ApiKey> keys,
  ) async => _RustVoice(await _h.startVoice(options: options, apiKeys: keys));

  @override
  Future<List<mc.McpServer>> mcpServers() => _h.mcpServers();

  @override
  Future<String> saveMcpServer(mc.McpServer server) =>
      _h.saveMcpServer(server: server);

  @override
  Future<void> removeMcpServer(String id) => _h.removeMcpServer(id: id);

  @override
  Future<mc.McpStatus> mcpCheck(String id, String secretsJson) =>
      _h.mcpCheck(id: id, secretsJson: secretsJson);

  @override
  Future<mc.OAuthStart> mcpOauthBegin(
    String id,
    String secretsJson, {
    String? redirectUri,
  }) => _h.mcpOauthBegin(
    id: id,
    secretsJson: secretsJson,
    redirectUri: redirectUri,
  );

  @override
  Future<String> mcpOauthWait(String flowId) => _h.mcpOauthWait(flowId: flowId);

  @override
  Future<String> mcpOauthComplete(String flowId, String callbackUrl) =>
      _h.mcpOauthComplete(flowId: flowId, callbackUrl: callbackUrl);

  @override
  Future<rf.ReflectPrefs> reflectPrefs() => _h.reflectPrefs();

  @override
  Future<void> setReflectPrefs(rf.ReflectPrefs prefs) =>
      _h.setReflectPrefs(prefs: prefs);

  @override
  Future<int> scheduleDue() => _h.scheduleDue();

  @override
  Future<rf.ReflectSignals> takeReflectSignals() => _h.takeReflectSignals();

  @override
  Future<rf.LintSummary> lintNow() => _h.lintNow();
}
