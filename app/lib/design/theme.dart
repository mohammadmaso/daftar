import 'package:flutter/material.dart';

import 'tokens.dart';

/// Resolved design system for the current brightness and UI script.
@immutable
class DaftarTheme extends ThemeExtension<DaftarTheme> {
  const DaftarTheme({required this.palette, required this.typeScale});

  final Palette palette;
  final TypeScale typeScale;

  static DaftarTheme of(BuildContext context) =>
      Theme.of(context).extension<DaftarTheme>()!;

  @override
  DaftarTheme copyWith({Palette? palette, TypeScale? typeScale}) => DaftarTheme(
    palette: palette ?? this.palette,
    typeScale: typeScale ?? this.typeScale,
  );

  // Palettes swap discretely; the app-level AnimatedTheme cross-fades the surfaces.
  @override
  DaftarTheme lerp(DaftarTheme? other, double t) =>
      other == null || t < 0.5 ? this : other;
}

extension DaftarThemeX on BuildContext {
  DaftarTheme get dt => DaftarTheme.of(this);
  Palette get palette => DaftarTheme.of(this).palette;
  TypeScale get type => DaftarTheme.of(this).typeScale;
}

/// Builds the Flutter [ThemeData] only so that stock widgets we still rely on (text selection,
/// scroll physics, focus) pick up our tokens. All visible components are our own.
ThemeData buildTheme(
  Brightness brightness,
  Script script, {
  LatinFont latinFont = LatinFont.sans,
  PersianFont persianFont = PersianFont.vazirmatn,
}) {
  final p = brightness == Brightness.dark ? Palette.dark : Palette.light;
  final t = TypeScale(script, latinFont: latinFont, persianFont: persianFont);
  final scheme = ColorScheme(
    brightness: brightness,
    primary: p.accent,
    onPrimary: p.onAccent,
    secondary: p.accent,
    onSecondary: p.onAccent,
    error: p.critical,
    onError: p.paper,
    surface: p.paper,
    onSurface: p.ink,
    outline: p.hairline,
  );
  return ThemeData(
    brightness: brightness,
    colorScheme: scheme,
    scaffoldBackgroundColor: p.paper,
    canvasColor: p.paper,
    fontFamily: t.family,
    fontFamilyFallback: t.fallback,
    splashFactory: NoSplash.splashFactory,
    highlightColor: Colors.transparent,
    hoverColor: p.sunken,
    focusColor: p.accentSoft,
    textSelectionTheme: TextSelectionThemeData(
      cursorColor: p.accent,
      selectionColor: p.accent.withValues(alpha: 0.22),
      selectionHandleColor: p.accent,
    ),
    textTheme: TextTheme(
      displaySmall: t.display,
      titleLarge: t.title,
      titleMedium: t.heading,
      bodyLarge: t.body,
      bodyMedium: t.body,
      bodySmall: t.small,
      labelLarge: t.label,
      labelSmall: t.caption,
    ).apply(bodyColor: p.ink, displayColor: p.ink),
    extensions: [DaftarTheme(palette: p, typeScale: t)],
  );
}
