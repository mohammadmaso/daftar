/// Compile-time feature gates. A feature ships only when its milestone's acceptance passes;
/// until then it is compiled out of release builds (brief §0.5).
abstract final class Features {
  /// Developer preview: exposes the design-system gallery. `--dart-define=DAFTAR_PREVIEW=true`.
  static const preview = bool.fromEnvironment('DAFTAR_PREVIEW');
}
