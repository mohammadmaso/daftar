import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'library_api.dart';

/// Git credentials in platform secure storage (Keychain / Keystore / libsecret / DPAPI).
/// Never written to the repository, logs or SQLite.
abstract class CredentialStore {
  Future<Auth> gitAuth();
  Future<void> saveGitAuth(Auth auth);
  Future<void> clear();
}

class SecureCredentialStore implements CredentialStore {
  SecureCredentialStore([FlutterSecureStorage? storage])
    : _s = storage ?? const FlutterSecureStorage();

  final FlutterSecureStorage _s;
  static const _key = 'git.auth.default';

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
}

const noAuth = Auth(kind: AuthKind.none, secret: '');

class MemoryCredentialStore implements CredentialStore {
  Auth _auth = noAuth;
  @override
  Future<Auth> gitAuth() async => _auth;
  @override
  Future<void> saveGitAuth(Auth auth) async => _auth = auth;
  @override
  Future<void> clear() async => _auth = noAuth;
}
