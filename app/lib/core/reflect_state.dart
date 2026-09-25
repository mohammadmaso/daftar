import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'library_api.dart';
import 'library_state.dart';

/// Settings › Reflect, from the synced `.daftar/config.json`.
final reflectPrefsProvider = FutureProvider<ReflectPrefs?>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib?.reflectPrefs();
});

/// The helpline country chosen in Settings › Reflect; null when none was chosen.
final helplineCountryProvider = FutureProvider<String?>((ref) async {
  final c = (await ref.watch(reflectPrefsProvider.future))?.helplineCountry;
  return c == null || c.isEmpty ? null : c;
});
