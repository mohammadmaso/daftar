import 'dart:async';
import 'dart:typed_data';

import 'package:daftar/core/library_api.dart';
import 'package:daftar/core/recorder.dart';

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

  @override
  Future<List<Vault>> vaults() async => const [
    Vault(id: 'life', titleEn: 'Life', titleFa: 'زندگی', fiction: false),
    Vault(id: 'health', titleEn: 'Health', titleFa: 'سلامت', fiction: false),
    Vault(id: 'mind', titleEn: 'Mind', titleFa: 'ذهن', fiction: false),
    Vault(id: 'work', titleEn: 'Work', titleFa: 'کار', fiction: false),
    Vault(
      id: 'stories',
      titleEn: 'Stories',
      titleFa: 'داستان‌ها',
      fiction: true,
    ),
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

  @override
  CapabilityWarning? capabilityWarning(ModelRole role, String model) =>
      role == ModelRole.vision && model.contains('whisper')
      ? CapabilityWarning.noVision
      : null;
}
