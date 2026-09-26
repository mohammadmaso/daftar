import 'dart:async';
import 'dart:typed_data';

import 'package:daftar/core/app_shortcuts.dart';
import 'package:daftar/core/file_import.dart';
import 'package:daftar/core/global_hotkey.dart';
import 'package:daftar/core/incoming_shares.dart';
import 'package:daftar/core/library_api.dart';
import 'package:daftar/core/notifications.dart';
import 'package:daftar/core/oauth_browser.dart';
import 'package:daftar/core/recorder.dart';
import 'package:daftar/core/voice_io.dart';

class FakeLibrary implements LibraryApi {
  FakeLibrary({List<Capture>? captures, this.hasRemote = true})
    : captures = captures ?? [];

  final List<Capture> captures;
  bool hasRemote;
  int syncs = 0;
  final texts = <String>[];

  Capture _add(
    RawKind kind,
    String text,
    String? vault, {
    List<String> images = const [],
  }) {
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
  Future<String> capturePhoto(
    Uint8List bytes, {
    String? note,
    String? vault,
  }) async => _add(RawKind.photo, '', vault).id;

  @override
  Future<String> captureVoice(String audioPath, {String? vault}) async =>
      _add(RawKind.voice, '', vault).id;

  final imports = <(String, String, String?)>[];
  final audioFiles = <String>[];

  @override
  Future<String> captureImport(
    String text,
    String fileName, {
    String? vault,
  }) async {
    imports.add((text, fileName, vault));
    return _add(RawKind.import_, text, vault).id;
  }

  @override
  Future<String> captureAudioFile(String path, {String? vault}) async {
    audioFiles.add(path);
    return _add(RawKind.voice, '', vault).id;
  }

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
    return const SyncResult(
      state: SyncState.synced,
      pulled: 0,
      pushed: 0,
      conflicts: [],
      replays: 0,
      changedPaths: [],
    );
  }

  @override
  Future<void> setRemote(String url) async => hasRemote = true;

  // ── AI settings: an in-memory stand-in for `.daftar/config.json` ──
  final providers = <AiProvider>[];
  final roles = <ModelRole, (String, String)>{};
  RunSummary nextRun = const RunSummary(jobs: [], pending: false);
  ProbeOutcome nextProbe = const ProbeOutcome(
    ok: true,
    latencyMs: 840,
    detail: 'OK',
  );
  final probed = <(ModelRole, List<ApiKey>)>[];
  final retried = <String>[];
  int runs = 0;

  static ModelRole? _fallback(ModelRole r) => switch (r) {
    ModelRole.router ||
    ModelRole.ingest ||
    ModelRole.voice ||
    ModelRole.vision ||
    ModelRole.reflect ||
    ModelRole.lint => ModelRole.chat,
    _ => null,
  };

  @override
  Future<AiSettings> aiSettings() async => AiSettings(
    providers: List.of(providers),
    roles: [
      for (final r in ModelRole.values)
        () {
          ModelRole? used = r;
          while (used != null && !roles.containsKey(used)) {
            used = _fallback(used);
          }
          final v = used == null ? null : roles[used];
          return RoleSetting(
            role: r,
            providerId: v?.$1,
            model: v?.$2,
            inheritedFrom: used == r ? null : used,
          );
        }(),
    ],
  );

  @override
  Future<String> saveProvider(AiProvider p) async {
    final id = p.id.isEmpty ? 'p${providers.length + 1}' : p.id;
    final saved = AiProvider(
      id: id,
      name: p.name,
      kind: p.kind,
      baseUrl: p.baseUrl,
      headers: p.headers,
      timeoutS: p.timeoutS,
    );
    final i = providers.indexWhere((x) => x.id == id);
    i < 0 ? providers.add(saved) : providers[i] = saved;
    return id;
  }

  @override
  Future<void> removeProvider(String id) async {
    providers.removeWhere((p) => p.id == id);
    roles.removeWhere((_, v) => v.$1 == id);
  }

  @override
  Future<void> setRole(ModelRole role, String providerId, String model) async =>
      roles[role] = (providerId, model);

  @override
  Future<void> clearRole(ModelRole role) async => roles.remove(role);

  @override
  Future<ProbeOutcome> testRole(ModelRole role, List<ApiKey> keys) async {
    probed.add((role, keys));
    return nextProbe;
  }

  @override
  Future<RunSummary> runJobs(List<ApiKey> keys) async {
    runs++;
    return nextRun;
  }

  @override
  Future<void> retryCapture(String id) async => retried.add(id);

  // ── Wiki: pages keyed by path ──
  final wiki = <String, WikiPage>{};
  final saved = <(String, String)>[];

  void addPage(
    String path,
    String titleEn,
    String titleFa,
    String body, {
    String kind = 'topic',
    List<String> backlinks = const [],
  }) {
    final vault = path.split('/')[1];
    wiki[path] = WikiPage(
      path: path,
      text: '---\ntype: $kind\n---\n\n$body\n',
      body: body,
      hash: 'h${body.hashCode}',
      kind: kind,
      vault: vault,
      titleEn: titleEn,
      titleFa: titleFa,
      aliases: const [],
      summary: '$titleEn summary',
      updated: '2026-09-23',
      status: 'active',
      sourceCount: 2,
      backlinks: [for (final b in backlinks) _summary(wiki[b]!)],
    );
  }

  PageSummary _summary(WikiPage p) => PageSummary(
    path: p.path,
    vault: p.vault,
    kind: p.kind,
    titleEn: p.titleEn,
    titleFa: p.titleFa,
    summary: p.summary,
    updated: p.updated,
  );

  @override
  Future<List<SearchHit>> search(
    String query, {
    List<String> vaults = const [],
    int limit = 30,
  }) async => [
    for (final p in wiki.values)
      if ((vaults.isEmpty || vaults.contains(p.vault)) &&
          (p.body.toLowerCase().contains(query.toLowerCase()) ||
              p.titleEn.toLowerCase().contains(query.toLowerCase()) ||
              p.titleFa.contains(query)))
        SearchHit(page: _summary(p), snippet: p.body.split('\n').first),
  ];

  @override
  Future<List<PageSummary>> recentPages({
    String? vault,
    int limit = 30,
  }) async => [
    for (final p in wiki.values)
      if (vault == null || p.vault == vault) _summary(p),
  ];

  @override
  Future<Listing> listDir(String dir) async {
    final prefix = '$dir/';
    final folders = <String, int>{};
    final pages = <PageSummary>[];
    for (final p in wiki.values.where((p) => p.path.startsWith(prefix))) {
      final rest = p.path.substring(prefix.length);
      final i = rest.indexOf('/');
      if (i < 0) {
        pages.add(_summary(p));
      } else {
        folders['$prefix${rest.substring(0, i)}'] =
            (folders['$prefix${rest.substring(0, i)}'] ?? 0) + 1;
      }
    }
    return Listing(
      folders: [
        for (final e in folders.entries)
          FolderEntry(path: e.key, pages: e.value),
      ],
      pages: pages,
    );
  }

  @override
  Future<LocalGraph> localGraph(String path, {int depth = 1}) async {
    final p = wiki[path]!;
    return LocalGraph(
      nodes: [
        GraphNode(page: _summary(p), depth: 0),
        for (final b in p.backlinks) GraphNode(page: b, depth: 1),
      ],
      edges: [for (final b in p.backlinks) GraphEdge(from: b.path, to: path)],
    );
  }

  @override
  Future<WikiGraph> wikiGraph() async {
    final paths = wiki.keys.where((p) => !p.startsWith('raw/')).toList()
      ..sort();
    final at = {for (var i = 0; i < paths.length; i++) paths[i]: i};
    final from = <int>[], to = <int>[];
    final links = List.filled(paths.length, 0);
    for (final p in paths) {
      for (final b in wiki[p]!.backlinks) {
        final a = at[b.path];
        if (a == null) continue;
        from.add(a);
        to.add(at[p]!);
        links[a]++;
        links[at[p]!]++;
      }
    }
    return WikiGraph(
      nodes: [
        for (var i = 0; i < paths.length; i++)
          WikiGraphNode(page: _summary(wiki[paths[i]]!), links: links[i]),
      ],
      edgeFrom: Uint32List.fromList(from),
      edgeTo: Uint32List.fromList(to),
    );
  }

  @override
  Future<WikiPage> page(String path) async =>
      wiki[path] ?? (throw StateError('no page $path'));

  @override
  Future<String?> resolveLink(String target) async {
    final t = target.split('#').first;
    for (final p in wiki.keys) {
      if (p == '$t.md' || p.endsWith('/$t.md')) return p;
    }
    return null;
  }

  @override
  Future<SaveResult> savePage(String path, String baseHash, String text) async {
    final p = wiki[path]!;
    if (baseHash != p.hash) {
      throw StateError('This page changed while you were editing.');
    }
    saved.add((path, text));
    final body = text.contains('\n---\n')
        ? text.split('\n---\n').last.trim()
        : text;
    addPage(path, p.titleEn, p.titleFa, body, kind: p.kind);
    return SaveResult(hash: wiki[path]!.hash, committed: true);
  }

  @override
  Future<int> refreshIndex() async => 0;

  @override
  Future<int> rebuildIndex() async => wiki.length;

  @override
  Future<String> root() async => '/data/daftar/libraries/default';

  // ── Activity and Review ──
  final ops = <Operation>[];
  final diffs = <String, List<PageDiff>>{};
  final cards = <ReviewCardDto>[];
  final undone = <String>[];
  final moved = <(String, String)>[];
  final reruns = <(String, String)>[];
  final resolved = <(String, ReviewAction, String?)>[];
  UndoResult nextUndo = UndoResult.done;

  @override
  Future<List<Operation>> activity({int limit = 50, String? before}) async =>
      List.of(ops);

  @override
  Future<Operation> operation(String opId) async =>
      ops.firstWhere((o) => o.opId == opId);

  @override
  Future<List<PageDiff>> operationDiff(String opId) async =>
      diffs[opId] ?? const [];

  @override
  Future<UndoResult> undo(String opId) async {
    undone.add(opId);
    return nextUndo;
  }

  @override
  Future<UndoResult> moveToVault(String opId, String vault) async {
    moved.add((opId, vault));
    return nextUndo;
  }

  @override
  Future<UndoResult> rerunWithNote(String opId, String note) async {
    reruns.add((opId, note));
    return nextUndo;
  }

  @override
  Future<void> includeCapture(String rawId) async {}

  @override
  Future<List<ReviewCardDto>> reviewCards() async => List.of(cards);

  @override
  Future<void> resolveReview(
    String cardId,
    ReviewAction action, {
    String? editedText,
  }) async {
    resolved.add((cardId, action, editedText));
    cards.removeWhere((c) => c.id == cardId);
  }

  // ── Ask ──
  final questions = <(String, AskScopeDto, List<AskTurn>, bool)>[];
  List<String> nextDeltas = const [
    'Sara is your ',
    'cousin ([[vaults/life/people/sara|Sara]]).',
  ];
  AskAnswer nextAnswer = AskAnswer(
    text: 'Sara is your cousin ([[vaults/life/people/sara|Sara]]).',
    citations: const [
      AnswerCitation(
        target: 'vaults/life/people/sara',
        label: 'Sara',
        path: 'vaults/life/people/sara.md',
      ),
    ],
    needsHelp: false,
    model: 'p1/claude-sonnet-5',
    inputTokens: BigInt.zero,
    outputTokens: BigInt.zero,
  );
  String? nextFailure;
  final savedAnswers = <(String, String)>[];
  final drafts = <(String, String, String)>[];

  @override
  Stream<AskEvent> ask(
    List<AskTurn> history,
    String question,
    AskScopeDto scope,
    List<ApiKey> keys, {
    AskImage? image,
    List<McpSecret> mcp = const [],
  }) async* {
    lastMcp = mcp;
    for (final a in nextApprovals) {
      yield AskEvent(kind: AskEventKind.approval, approval: a);
    }
    questions.add((question, scope, history, image != null));
    for (final d in nextDeltas) {
      yield AskEvent(kind: AskEventKind.delta, text: d);
    }
    if (nextFailure != null) {
      yield AskEvent(kind: AskEventKind.failed, text: nextFailure);
    } else {
      yield AskEvent(kind: AskEventKind.done, answer: nextAnswer);
    }
  }

  @override
  Future<String> saveAnswer(
    String question,
    String answer,
    AskScopeDto scope,
  ) async {
    savedAnswers.add((question, answer));
    return '01ANSWER';
  }

  @override
  Future<String> saveDraft(String story, String title, String text) async {
    drafts.add((story, title, text));
    return 'vaults/stories/$story/drafts/x.md';
  }

  // ── MCP ──
  final mcp = <McpServer>[];
  List<McpSecret> lastMcp = const [];
  List<ToolApproval> nextApprovals = const [];
  McpStatus nextStatus = const McpStatus(
    kind: McpStatusKind.needsAuth,
    tools: [],
  );
  final checks = <(String, String)>[];
  final oauth = <String>[];

  @override
  Future<List<McpServer>> mcpServers() async => List.of(mcp);

  @override
  Future<String> saveMcpServer(McpServer server) async {
    final id = server.id.isEmpty
        ? server.name.toLowerCase().replaceAll(' ', '-')
        : server.id;
    mcp.removeWhere((m) => m.id == id);
    mcp.add(
      McpServer(
        id: id,
        name: server.name,
        transport: server.transport,
        target: server.target,
        args: server.args,
        envNames: server.envNames,
        auth: server.auth,
        authNames: server.authNames,
        clientId: server.clientId,
        scopes: server.scopes,
        policy: server.policy,
        enabled: server.enabled,
      ),
    );
    return id;
  }

  @override
  Future<void> removeMcpServer(String id) async =>
      mcp.removeWhere((m) => m.id == id);

  @override
  Future<McpStatus> mcpCheck(String id, String secretsJson) async {
    checks.add((id, secretsJson));
    return nextStatus;
  }

  @override
  Future<OAuthStart> mcpOauthBegin(
    String id,
    String secretsJson, {
    String? redirectUri,
  }) async {
    oauth.add('begin:$id:${redirectUri ?? 'loopback'}');
    return const OAuthStart(
      flowId: 'flow-1',
      authUrl: 'https://auth.example.com/authorize?x=1',
    );
  }

  @override
  Future<String> mcpOauthWait(String flowId) async {
    oauth.add('wait:$flowId');
    return '{"oauth":{"client_id":"c1"}}';
  }

  @override
  Future<String> mcpOauthComplete(String flowId, String callbackUrl) async {
    oauth.add('complete:$flowId:$callbackUrl');
    return '{"oauth":{"client_id":"c1"}}';
  }

  // ── Reflect ──
  ReflectPrefs prefs = const ReflectPrefs(
    daily: true,
    dailyTime: '21:30',
    weekly: true,
    weeklyDay: 5,
    notifications: true,
    helplineCountry: '',
  );
  int scheduled = 0;
  int lints = 0;
  ReflectSignals signals = const ReflectSignals(
    notifications: [],
    needsHelp: false,
  );

  @override
  Future<ReflectPrefs> reflectPrefs() async => prefs;

  @override
  Future<void> setReflectPrefs(ReflectPrefs p) async => prefs = p;

  @override
  Future<int> scheduleDue() async {
    scheduled++;
    return 0;
  }

  @override
  Future<ReflectSignals> takeReflectSignals() async {
    final s = signals;
    signals = const ReflectSignals(notifications: [], needsHelp: false);
    return s;
  }

  @override
  Future<LintSummary> lintNow() async {
    lints++;
    return const LintSummary(findings: 3, newCards: 1);
  }

  // ── Voice ──
  FakeVoice? voice;

  @override
  Future<VoiceConversation> startVoice(
    VoiceOptions options,
    List<ApiKey> keys,
  ) async => voice = FakeVoice();

  // ── Vaults ──
  List<VaultSettings> vaultList = [
    _vault('life', 'Life', 'زندگی', 'Journal, people, places.', pages: 3),
    _vault('health', 'Health', 'سلامت', 'Medical profile.', pages: 1),
    _vault('mind', 'Mind', 'ذهن', 'Moods, patterns, values.'),
    _vault('work', 'Work', 'کار', 'Projects, learning.'),
    _vault('stories', 'Stories', 'داستان‌ها', 'Fiction.', fiction: true),
  ];

  static VaultSettings _vault(
    String id,
    String en,
    String fa,
    String purpose, {
    bool archived = false,
    bool fiction = false,
    int pages = 0,
  }) => VaultSettings(
    id: id,
    titleEn: en,
    titleFa: fa,
    purpose: purpose,
    archived: archived,
    fiction: fiction,
    builtin: id == 'life' || id == 'stories',
    pages: pages,
  );

  @override
  Future<List<Vault>> vaults() async => [
    for (final v in vaultList)
      if (!v.archived)
        Vault(
          id: v.id,
          titleEn: v.titleEn,
          titleFa: v.titleFa,
          fiction: v.fiction,
        ),
  ];

  @override
  Future<List<VaultSettings>> vaultSettings() async => vaultList;

  @override
  Future<String> addVault({
    required String titleEn,
    required String titleFa,
    required String purpose,
  }) async {
    final id = titleEn.toLowerCase().replaceAll(RegExp('[^a-z0-9]+'), '-');
    vaultList = [...vaultList, _vault(id, titleEn, titleFa, purpose)];
    return id;
  }

  @override
  Future<void> editVault(
    String id, {
    required String titleEn,
    required String titleFa,
    required String purpose,
  }) async {
    vaultList = [
      for (final v in vaultList)
        v.id == id
            ? _vault(
                id,
                titleEn,
                titleFa,
                purpose,
                archived: v.archived,
                fiction: v.fiction,
                pages: v.pages,
              )
            : v,
    ];
  }

  @override
  Future<void> setVaultArchived(String id, bool archived) async {
    vaultList = [
      for (final v in vaultList)
        v.id == id
            ? _vault(
                id,
                v.titleEn,
                v.titleFa,
                v.purpose,
                archived: archived,
                fiction: v.fiction,
                pages: v.pages,
              )
            : v,
    ];
  }

  @override
  Future<void> removeVault(String id) async {
    vaultList = vaultList.where((v) => v.id != id).toList();
  }
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
  Future<void> initLocal(
    String root, {
    required String deviceName,
    required String platform,
  }) async {
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
    privateOpenssh:
        '-----BEGIN OPENSSH PRIVATE KEY-----\nfake\n-----END OPENSSH PRIVATE KEY-----\n',
    publicOpenssh:
        'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFakeKeyForTestsOnly $comment',
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

class FakeProviderApi implements ProviderApi {
  List<String> models = const ['gpt-5-mini', 'gpt-5', 'whisper-1'];
  Object? error;

  @override
  Future<List<String>> listModels(AiProvider provider, String? apiKey) async {
    if (error != null) throw error!;
    return models;
  }

  @override
  String defaultBaseUrl(ProviderKindDto kind) => switch (kind) {
    ProviderKindDto.openaiCompatible => 'https://api.openai.com/v1',
    ProviderKindDto.anthropic => 'https://api.anthropic.com/v1',
    ProviderKindDto.gemini =>
      'https://generativelanguage.googleapis.com/v1beta',
  };

  final approvals = <(String, bool)>[];

  @override
  bool get stdioSupported => true;

  @override
  void answerToolApproval(String requestId, bool allowed) =>
      approvals.add((requestId, allowed));

  @override
  List<Helpline> helplines(String country) => [
    if (country == 'IR')
      const Helpline(
        nameEn: 'Social emergency',
        nameFa: 'اورژانس اجتماعی',
        phone: '123',
        url: '',
      ),
    const Helpline(
      nameEn: 'Find a helpline in your country',
      nameFa: 'یافتن خط کمک',
      phone: '',
      url: 'https://findahelpline.com',
    ),
  ];

  @override
  CapabilityWarning? capabilityWarning(ModelRole role, String model) =>
      role == ModelRole.vision && model.contains('whisper')
      ? CapabilityWarning.noVision
      : null;
}

class FakeVoice implements VoiceConversation {
  final _events = StreamController<VoiceEventDto>.broadcast();
  final fed = <int>[];
  bool muted = false;
  bool ended = false;
  int playbackDone = 0;

  void emit(
    VoiceEventKind kind, {
    VoiceStateDto? state,
    String? text,
    Uint8List? bytes,
  }) => _events.add(
    VoiceEventDto(
      kind: kind,
      state: state,
      text: text,
      seq: BigInt.zero,
      lang: 'en',
      bytes: bytes,
    ),
  );

  @override
  Stream<VoiceEventDto> get events => _events.stream;

  @override
  Future<void> feed(Int16List pcm) async => fed.add(pcm.length);

  @override
  Future<void> playbackFinished() async => playbackDone++;

  @override
  Future<void> setMuted(bool m) async => muted = m;

  @override
  Future<String?> end() async {
    ended = true;
    await _events.close();
    return '01TRANSCRIPT';
  }
}

class FakeMic implements VoiceMic {
  final controller = StreamController<Uint8List>.broadcast();
  bool permission = true;
  bool running = false;

  @override
  Future<bool> hasPermission() async => permission;

  @override
  Future<Stream<Uint8List>> start({int sampleRate = 16000}) async {
    running = true;
    return controller.stream;
  }

  @override
  Future<void> stop() async => running = false;
}

class FakePlayer implements VoicePlayer {
  final played = <Uint8List>[];
  int stops = 0;
  final _idle = StreamController<void>.broadcast();

  void finish() => _idle.add(null);

  @override
  void enqueue(Uint8List bytes) => played.add(bytes);

  @override
  Future<void> stop() async => stops++;

  @override
  Stream<void> get idle => _idle.stream;

  @override
  Future<void> dispose() => _idle.close();
}

class FakeAwake implements ScreenAwake {
  bool on = false;
  VoiceNotice? notice;
  @override
  Future<void> set(bool v, {VoiceNotice? notice}) async {
    on = v;
    this.notice = v ? notice : null;
  }
}

class FakeBrowser implements OAuthBrowser {
  FakeBrowser({this.usesLoopback = true});
  @override
  final bool usesLoopback;
  final opened = <String>[];

  @override
  String get mobileRedirect => 'daftar://oauth/callback';

  @override
  Future<String?> open(String url) async {
    opened.add(url);
    return usesLoopback ? null : 'daftar://oauth/callback?code=abc&state=s';
  }
}

class FakeNotifications implements SystemNotifications {
  final shown = <String>[];
  var asked = 0;
  var granted = true;

  @override
  Future<bool> requestPermission({required String openLabel}) async {
    asked++;
    return granted;
  }

  @override
  Future<void> show(
    String title,
    String body, {
    required String channel,
    required String openLabel,
  }) async => shown.add(body);
}

class FakeAppShortcuts implements AppShortcuts {
  Map<AppShortcut, String> titles = const {};
  void Function(AppShortcut)? _handler;

  /// Simulates a long press on the app icon and picking [s].
  void launch(AppShortcut s) => _handler!(s);

  @override
  Future<void> set(
    Map<AppShortcut, String> titles,
    void Function(AppShortcut) onAction,
  ) async {
    this.titles = titles;
    _handler = onAction;
  }
}

class FakeShares implements IncomingShares {
  final pending = <SharedItem>[];
  final _arrived = StreamController<void>.broadcast();

  void share(SharedItem item) {
    pending.add(item);
    _arrived.add(null);
  }

  @override
  Stream<void> get arrived => _arrived.stream;

  @override
  Future<List<SharedItem>> take() async {
    final items = List.of(pending);
    pending.clear();
    return items;
  }
}

class FakeHotkey implements GlobalHotkey {
  FakeHotkey({this.hasMini = false});

  /// Whether this "platform" has the compact recorder (Linux, macOS).
  final bool hasMini;
  bool mini = false;
  final left = <bool>[];
  final recording = <bool>[];
  Map<String, String> labels = const {};

  /// Called when the "window" shrinks (true) or comes back (false), so a test can resize.
  void Function(bool mini)? onWindow;

  final _record = StreamController<void>.broadcast();
  final _import = StreamController<String?>.broadcast();
  void press() => _record.add(null);
  void trayImport([String? path]) => _import.add(path);

  @override
  Stream<void> get record => _record.stream;

  @override
  Stream<String?> get import => _import.stream;

  @override
  Future<bool> enterMiniRecorder() async {
    mini = hasMini;
    if (mini) onWindow?.call(true);
    return mini;
  }

  @override
  Future<void> leaveMiniRecorder({required bool open}) async {
    mini = false;
    left.add(open);
    onWindow?.call(false);
  }

  @override
  Future<void> setRecording(bool on) async => recording.add(on);

  @override
  Future<void> setLabels(Map<String, String> labels) async =>
      this.labels = labels;

  ShortcutStatus? status;
  final installed = <String>[];

  @override
  Future<ShortcutStatus?> shortcutStatus() async => status;

  @override
  Future<bool> installShortcut(String name) async {
    installed.add(name);
    status = ShortcutStatus(
      keys: status?.keys ?? 'Ctrl+Alt+Shift+N',
      state: ShortcutState.active,
      viaDesktopSettings: true,
    );
    return true;
  }
}

/// Picks [next] (null = cancelled) and returns [previews] by path; a missing path throws [error].
class FakeFileImports implements FileImports {
  String? next;
  final previews = <String, ImportPreview>{};
  Object error = Exception(
    'This PDF has no text in it; scanned pages can be added as photos.',
  );

  @override
  Future<String?> pick() async => next;

  @override
  Future<ImportPreview> read(String path) async =>
      previews[path] ?? (throw error);

  @override
  int get charLimit => 60000;
}

class FakeFilePlayer implements FilePlayer {
  final played = <String>[];
  final _playing = StreamController<bool>.broadcast();

  @override
  Future<void> play(String path) async {
    played.add(path);
    _playing.add(true);
  }

  @override
  Future<void> stop() async => _playing.add(false);

  @override
  Stream<bool> get playing => _playing.stream;

  @override
  Future<void> dispose() async {}
}
