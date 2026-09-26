import 'dart:convert';

import 'package:flutter/foundation.dart';
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

/// True where API keys and MCP credentials are iCloud Keychain items, so an iPhone and a Mac
/// signed in to the same Apple ID share them (ADR-0027).
bool get secretsFollowAppleId =>
    defaultTargetPlatform == TargetPlatform.iOS ||
    defaultTargetPlatform == TargetPlatform.macOS;

class SecureCredentialStore implements CredentialStore {
  SecureCredentialStore([
    FlutterSecureStorage? local,
    FlutterSecureStorage? shared,
  ]) : _s =
           local ??
           const FlutterSecureStorage(iOptions: _localIos, mOptions: _localMac),
       _shared =
           shared ??
           const FlutterSecureStorage(
             iOptions: _sharedIos,
             mOptions: _sharedMac,
           );

  // `first_unlock` lets the background pass read credentials while the phone is locked.
  static const _localIos = IOSOptions(
    accessibility: KeychainAccessibility.first_unlock,
  );
  static const _localMac = MacOsOptions(
    accessibility: KeychainAccessibility.first_unlock,
  );
  static const _sharedIos = IOSOptions(
    accessibility: KeychainAccessibility.first_unlock,
    synchronizable: true,
  );
  static const _sharedMac = MacOsOptions(
    accessibility: KeychainAccessibility.first_unlock,
    synchronizable: true,
  );

  /// Repository credentials: this device only (an SSH key is generated per device).
  final FlutterSecureStorage _s;

  /// API keys and MCP credentials: synced through iCloud Keychain on Apple platforms. The
  /// Apple options are ignored elsewhere, so there it behaves like [_s].
  final FlutterSecureStorage _shared;

  static const _key = 'git.auth.default';
  static String _apiKey(String providerId) => 'ai.key.$providerId';
  static String _mcpKey(String serverId) => 'mcp.secrets.$serverId';

  /// Reads a shared item, moving one saved before sharing existed into iCloud Keychain.
  Future<String?> _readShared(String key) async {
    final synced = await _shared.read(key: key);
    if (synced != null || !secretsFollowAppleId) return synced;
    final old = await _s.read(key: key);
    // Writing the synchronizable item replaces the device-only one.
    if (old != null) await _shared.write(key: key, value: old);
    return old;
  }

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
  Future<String?> apiKey(String providerId) => _readShared(_apiKey(providerId));

  @override
  Future<String?> mcpSecrets(String serverId) => _readShared(_mcpKey(serverId));

  @override
  Future<void> saveMcpSecrets(String serverId, String json) =>
      _shared.write(key: _mcpKey(serverId), value: json);

  // Deleting removes the synced and the device-only item alike.
  @override
  Future<void> deleteMcpSecrets(String serverId) =>
      _shared.delete(key: _mcpKey(serverId));

  @override
  Future<void> saveApiKey(String providerId, String key) =>
      _shared.write(key: _apiKey(providerId), value: key);

  @override
  Future<void> deleteApiKey(String providerId) =>
      _shared.delete(key: _apiKey(providerId));
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
