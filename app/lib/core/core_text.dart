import '../l10n/app_localizations.dart';
import 'library_api.dart' show ModelRole;

/// The core speaks English (it has no UI language). Its user-facing sentences are translated here
/// into the app's language, so the ARB files stay the only source of UI text. A sentence without a
/// pattern is shown as the core wrote it.
///
/// The app root keeps [coreStrings] current; it is null in pure unit tests, which then see the
/// core's own words.
L10n? coreStrings;

/// Role ids as the core writes them.
String _role(L10n l, String id) {
  final r = ModelRole.values.where((v) => v.name == id).firstOrNull;
  return switch (r) {
    ModelRole.router => l.roleRouter,
    ModelRole.ingest => l.roleIngest,
    ModelRole.chat => l.roleChat,
    ModelRole.voice => l.roleVoice,
    ModelRole.vision => l.roleVision,
    ModelRole.reflect => l.roleReflect,
    ModelRole.lint => l.roleLint,
    ModelRole.stt => l.roleStt,
    ModelRole.tts => l.roleTts,
    ModelRole.embedding => l.roleEmbedding,
    null => id,
  };
}

typedef _Rule = (RegExp, String Function(L10n, RegExpMatch));

final List<_Rule> _rules = [
  (
    RegExp(r'^(.+) rejected the API key\.$'),
    (l, m) => l.coreKeyRejected(m[1]!),
  ),
  (
    RegExp(r'^(.+) is rate-limiting requests; will retry\.$'),
    (l, m) => l.coreRateLimited(m[1]!),
  ),
  (
    RegExp(r"^(.+) doesn't know this model or endpoint\.$"),
    (l, m) => l.coreUnknownModel(m[1]!),
  ),
  (RegExp(r'^(.+) refused the request\.$'), (l, m) => l.coreRefused(m[1]!)),
  (
    RegExp(r'^(.+) had a server error; will retry\.$'),
    (l, m) => l.coreServerError(m[1]!),
  ),
  (
    RegExp(r'^(.+) answered with status (\d+)\.$'),
    (l, m) => l.coreStatus(m[1]!, m[2]!),
  ),
  (RegExp(r'^(.+) took too long to answer\.$'), (l, m) => l.coreTimeout(m[1]!)),
  (RegExp(r"^Couldn't reach (.+)\.$"), (l, m) => l.coreUnreachable(m[1]!)),
  (
    RegExp(r"^(.+) sent a response that couldn't be read\.$"),
    (l, m) => l.coreBadResponse(m[1]!),
  ),
  (
    RegExp(r'^Connection to (.+) failed\.$'),
    (l, m) => l.coreUnreachable(m[1]!),
  ),
  (
    RegExp(
      r'^No model is set up for (\w+)\. Choose one in Settings › Models\.$',
    ),
    (l, m) => l.coreNoModel(_role(l, m[1]!)),
  ),
  (RegExp(r'^network unavailable$'), (l, _) => l.syncOffline),
  (RegExp(r'^authentication failed: '), (l, _) => l.coreAuthFailed),
  (RegExp(r'^The remote rejected the credentials'), (l, _) => l.coreAuthFailed),
  (
    RegExp(r'^The remote kept changing while pushing'),
    (l, _) => l.coreRemoteBusy,
  ),
  (RegExp(r'^The remote refused the push'), (l, _) => l.corePushRefused),
  (
    RegExp(r'^The recording is no longer on this device\.$'),
    (l, _) => l.coreRecordingGone,
  ),
  (RegExp(r'^The capture no longer exists\.$'), (l, _) => l.coreCaptureGone),
  (
    RegExp(r'^This page changed while you were editing\.'),
    (l, _) => l.coreEditConflict,
  ),
  (
    RegExp(r'^This page contains what looks like a'),
    (l, _) => l.coreSecretInPage,
  ),
  (RegExp(r'^MCP servers must use https'), (l, _) => l.coreMcpHttps),
  (RegExp(r'^The server address is not a URL\.$'), (l, _) => l.coreMcpBadUrl),
];

/// [raw] in the app's language, when the core sentence is known.
String coreText(String raw, [L10n? strings]) {
  final l = strings ?? coreStrings;
  if (l == null) return raw;
  final text = raw.trim();
  for (final (re, make) in _rules) {
    final m = re.firstMatch(text);
    if (m != null) return make(l, m);
  }
  return raw;
}
