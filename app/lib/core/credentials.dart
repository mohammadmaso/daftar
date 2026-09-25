import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'library_api.dart';

/// Git credentials and AI provider keys in platform secure storage (Keychain / Keystore /
/// libsecret / DPAPI). Never written to the repository, logs or SQLite.
abstract class CredentialStore {
  Future<Auth> gitAuth();
  Future<void> saveGitAuth(Auth auth);
  Future<void> clear();

  Future<String?> apiKey(String providerId);

  /// This device's credentials for one MCP server, as JSON (§10).
  Future<String?> mcpSecrets(String serverId);
  Future<void> saveMcpSecrets(String serverId, String json);
  Future<void> deleteMcpSecrets(String serverId);
  Future<void> saveApiKey(String providerId, String key);
  Future<void> deleteApiKey(String providerId);
}

extension ApiKeys on CredentialStore {
  /// Credentials for the given MCP servers that this device has.
  Future<List<McpSecret>> mcpSecretsFor(Iterable<String> serverIds) async => [
    for (final id in serverIds)
      if (await mcpSecrets(id) case final j?)
        McpSecret(serverId: id, secretsJson: j),
  ];

  /// Keys for the given providers, skipping those without one.
  Future<List<ApiKey>> apiKeys(Iterable<String> providerIds) async => [
    for (final id in providerIds)
      if (await apiKey(id) case final k? when k.isNotEmpty)
        ApiKey(providerId: id, key: k),
  ];
}

class SecureCredentialStore implements CredentialStore {
  SecureCredentialStore([FlutterSecureStorage? storage])
    : _s = storage ?? const FlutterSecureStorage();

  final FlutterSecureStorage _s;
  static const _key = 'git.auth.default';
  static String _apiKey(String providerId) => 'ai.key.$providerId';

  @override
  Future<Auth> gitAuth() async {
    final raw = await _s.read(key: _key);
    if (raw == null) return noAuth;
    final m = jsonDecode(raw) as Map<String, dynamic>;
    return Auth(
      kind: AuthKind.values.byName(m['kind'] as String),
      username: m['username'] as String?,
      secret: m['secret'] as String? ?? '',
      passphrase: m['passphrase'] as String?,
    );
  }

  @override
  Future<void> saveGitAuth(Auth auth) => _s.write(
    key: _key,
    value: jsonEncode({
      'kind': auth.kind.name,
      'username': auth.username,
      'secret': auth.secret,
      'passphrase': auth.passphrase,
    }),
  );

  @override
  Future<void> clear() => _s.delete(key: _key);

  @override
  Future<String?> apiKey(String providerId) =>
      _s.read(key: _apiKey(providerId));

  @override
  Future<String?> mcpSecrets(String serverId) =>
      _s.read(key: 'mcp.secrets.$serverId');

  @override
  Future<void> saveMcpSecrets(String serverId, String json) =>
      _s.write(key: 'mcp.secrets.$serverId', value: json);

  @override
  Future<void> deleteMcpSecrets(String serverId) =>
      _s.delete(key: 'mcp.secrets.$serverId');

  @override
  Future<void> saveApiKey(String providerId, String key) =>
      _s.write(key: _apiKey(providerId), value: key);

  @override
  Future<void> deleteApiKey(String providerId) =>
      _s.delete(key: _apiKey(providerId));
}

const noAuth = Auth(kind: AuthKind.none, secret: '');

class MemoryCredentialStore implements CredentialStore {
  Auth _auth = noAuth;
  final keys = <String, String>{};
  @override
  Future<Auth> gitAuth() async => _auth;
  @override
  Future<void> saveGitAuth(Auth auth) async => _auth = auth;
  @override
  Future<void> clear() async => _auth = noAuth;
  @override
  Future<String?> apiKey(String providerId) async => keys[providerId];
  final mcp = <String, String>{};
  @override
  Future<String?> mcpSecrets(String serverId) async => mcp[serverId];
  @override
  Future<void> saveMcpSecrets(String serverId, String json) async =>
      mcp[serverId] = json;
  @override
  Future<void> deleteMcpSecrets(String serverId) async => mcp.remove(serverId);
  @override
  Future<void> saveApiKey(String providerId, String key) async =>
      keys[providerId] = key;
  @override
  Future<void> deleteApiKey(String providerId) async => keys.remove(providerId);
}
