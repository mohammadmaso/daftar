import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../design/tokens.dart';

enum LanguagePref { system, en, fa }

enum ThemePref { system, light, dark }

/// Multiplier applied on top of the OS text scale.
enum TextSizePref {
  small(0.9),
  normal(1.0),
  large(1.15),
  larger(1.3);

  const TextSizePref(this.factor);
  final double factor;
}

@immutable
class Appearance {
  const Appearance({
    this.language = LanguagePref.system,
    this.theme = ThemePref.system,
    this.textSize = TextSizePref.normal,
    this.latinFont = LatinFont.sans,
    this.persianFont = PersianFont.vazirmatn,
  });

  final LanguagePref language;
  final ThemePref theme;
  final TextSizePref textSize;

  /// Reading face for English and other Latin text.
  final LatinFont latinFont;

  /// Reading face for Persian text.
  final PersianFont persianFont;

  Appearance copyWith({
    LanguagePref? language,
    ThemePref? theme,
    TextSizePref? textSize,
    LatinFont? latinFont,
    PersianFont? persianFont,
  }) => Appearance(
    language: language ?? this.language,
    theme: theme ?? this.theme,
    textSize: textSize ?? this.textSize,
    latinFont: latinFont ?? this.latinFont,
    persianFont: persianFont ?? this.persianFont,
  );

  /// Null means "follow the system locale".
  Locale? get locale => switch (language) {
    LanguagePref.system => null,
    LanguagePref.en => const Locale('en'),
    LanguagePref.fa => const Locale('fa'),
  };
}

const supportedLocales = [Locale('en'), Locale('fa')];

Script scriptFor(Locale locale) =>
    locale.languageCode == 'fa' ? Script.persian : Script.latin;

/// Appearance is per device (a phone and a laptop may want different themes), so it lives in
/// local preferences rather than the synced `.daftar/config.json`.
final sharedPreferencesProvider = Provider<SharedPreferences>(
  (ref) => throw StateError('sharedPreferencesProvider must be overridden'),
);

final appearanceProvider = NotifierProvider<AppearanceNotifier, Appearance>(
  AppearanceNotifier.new,
);

class AppearanceNotifier extends Notifier<Appearance> {
  static const _kLanguage = 'appearance.language';
  static const _kTheme = 'appearance.theme';
  static const _kTextSize = 'appearance.textSize';
  static const _kLatinFont = 'appearance.latinFont';
  static const _kPersianFont = 'appearance.persianFont';

  SharedPreferences get _prefs => ref.read(sharedPreferencesProvider);

  @override
  Appearance build() {
    T pick<T extends Enum>(List<T> values, String key, T fallback) {
      final name = _prefs.getString(key);
      return values.where((v) => v.name == name).firstOrNull ?? fallback;
    }

    return Appearance(
      language: pick(LanguagePref.values, _kLanguage, LanguagePref.system),
      theme: pick(ThemePref.values, _kTheme, ThemePref.system),
      textSize: pick(TextSizePref.values, _kTextSize, TextSizePref.normal),
      latinFont: pick(LatinFont.values, _kLatinFont, LatinFont.sans),
      persianFont: pick(
        PersianFont.values,
        _kPersianFont,
        PersianFont.vazirmatn,
      ),
    );
  }

  void setLanguage(LanguagePref v) {
    state = state.copyWith(language: v);
    _prefs.setString(_kLanguage, v.name);
  }

  void setTheme(ThemePref v) {
    state = state.copyWith(theme: v);
    _prefs.setString(_kTheme, v.name);
  }

  void setTextSize(TextSizePref v) {
    state = state.copyWith(textSize: v);
    _prefs.setString(_kTextSize, v.name);
  }

  void setLatinFont(LatinFont v) {
    state = state.copyWith(latinFont: v);
    _prefs.setString(_kLatinFont, v.name);
  }

  void setPersianFont(PersianFont v) {
    state = state.copyWith(persianFont: v);
    _prefs.setString(_kPersianFont, v.name);
  }
}
