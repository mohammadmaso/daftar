import 'package:daftar/core/credentials.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';

/// One keychain view: device-only or synchronizable items.
class _Store implements FlutterSecureStorage {
  final items = <String, String>{};

  @override
  Future<String?> read({
    required String key,
    dynamic iOptions,
    dynamic aOptions,
    dynamic lOptions,
    dynamic webOptions,
    dynamic mOptions,
    dynamic wOptions,
  }) async => items[key];

  @override
  Future<void> write({
    required String key,
    required String? value,
    dynamic iOptions,
    dynamic aOptions,
    dynamic lOptions,
    dynamic webOptions,
    dynamic mOptions,
    dynamic wOptions,
  }) async {
    if (value == null) {
      items.remove(key);
    } else {
      items[key] = value;
    }
  }

  @override
  Future<void> delete({
    required String key,
    dynamic iOptions,
    dynamic aOptions,
    dynamic lOptions,
    dynamic webOptions,
    dynamic mOptions,
    dynamic wOptions,
  }) async => items.remove(key);

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  tearDown(() => debugDefaultTargetPlatformOverride = null);

  test(
    'on Apple platforms a key saved before sharing moves into iCloud Keychain',
    () async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      final local = _Store()..items['ai.key.p1'] = 'sk-old';
      final shared = _Store();
      final store = SecureCredentialStore(local, shared);

      expect(await store.apiKey('p1'), 'sk-old');
      expect(shared.items['ai.key.p1'], 'sk-old');

      await store.saveMcpSecrets('m1', '{}');
      expect(shared.items['mcp.secrets.m1'], '{}');
      expect(local.items.containsKey('mcp.secrets.m1'), isFalse);
    },
  );

  test('repository credentials stay on the device', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    final local = _Store();
    final shared = _Store();
    final store = SecureCredentialStore(local, shared);

    await store.saveGitAuth(noAuth);
    expect(local.items.keys, ['git.auth.default']);
    expect(shared.items, isEmpty);
  });

  // Off Apple platforms both stores are the same storage, so there is nothing to move.
  test('elsewhere there is no migration step', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    final local = _Store()..items['ai.key.p1'] = 'sk-old';
    final shared = _Store();
    final store = SecureCredentialStore(local, shared);

    expect(await store.apiKey('p1'), isNull);
    expect(shared.items, isEmpty);
  });
}
