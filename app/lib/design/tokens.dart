import 'package:flutter/animation.dart';
import 'package:flutter/painting.dart';

/// Design tokens: "calm paper, precise ink". See docs/design/style-guide.md.
/// Components read these through [DaftarTheme]; never hard-code values in feature code.
abstract final class Space {
  static const double x1 = 4;
  static const double x2 = 8;
  static const double x3 = 12;
  static const double x4 = 16;
  static const double x5 = 20;
  static const double x6 = 24;
  static const double x8 = 32;
  static const double x10 = 40;
  static const double x12 = 48;
  static const double x16 = 64;

  /// Horizontal page gutter on phones.
  static const double gutter = x4;

  /// Readable measure for prose and settings columns.
  static const double measure = 680;
}

abstract final class Radii {
  static const double small = 10;
  static const double medium = 12;
  static const double large = 14;
  static const double pill = 999;
}

abstract final class Stroke {
  static const double hairline = 1;
  static const double focus = 2;
}

abstract final class Motion {
  static const Duration quick = Duration(milliseconds: 150);
  static const Duration standard = Duration(milliseconds: 200);
  static const Duration slow = Duration(milliseconds: 250);
  static const Curve ease = Curves.easeOutCubic;
  static const Curve emphasized = Curves.easeOutBack;
}

/// Minimum interactive target (WCAG / platform guidance).
const double kMinTarget = 44;

/// Colour palette. One accent (ink teal); semantic colours are muted and used only for status.
final class Palette {
  const Palette({
    required this.paper,
    required this.raised,
    required this.sunken,
    required this.ink,
    required this.inkMuted,
    required this.inkFaint,
    required this.hairline,
    required this.accent,
    required this.onAccent,
    required this.accentSoft,
    required this.positive,
    required this.pending,
    required this.critical,
  });

  /// Page background.
  final Color paper;

  /// Cards, sheets, bars.
  final Color raised;

  /// Inputs, code blocks, pressed states.
  final Color sunken;

  /// Primary text.
  final Color ink;

  /// Secondary text (AA on paper).
  final Color inkMuted;

  /// Decorative only: placeholder glyphs, disabled icons. Never body text.
  final Color inkFaint;

  final Color hairline;
  final Color accent;
  final Color onAccent;
  final Color accentSoft;

  /// Status: confirmed / synced.
  final Color positive;

  /// Status: proposed / waiting.
  final Color pending;

  /// Status: failed / superseded / destructive.
  final Color critical;

  static const light = Palette(
    paper: Color(0xFFF7F4EE),
    raised: Color(0xFFFCFAF6),
    sunken: Color(0xFFEFEBE3),
    ink: Color(0xFF1D1C1A),
    inkMuted: Color(0xFF625D55),
    inkFaint: Color(0xFFA29C92),
    hairline: Color(0xFFE2DCD1),
    accent: Color(0xFF22505E),
    onAccent: Color(0xFFF7F4EE),
    accentSoft: Color(0xFFE2EAE9),
    positive: Color(0xFF3B6A4A),
    pending: Color(0xFF8A5A14),
    critical: Color(0xFF9C3A2E),
  );

  static const dark = Palette(
    paper: Color(0xFF1B1B1A),
    raised: Color(0xFF232321),
    sunken: Color(0xFF151514),
    ink: Color(0xFFECE8E1),
    inkMuted: Color(0xFFA7A298),
    inkFaint: Color(0xFF6E6A63),
    hairline: Color(0xFF34322F),
    accent: Color(0xFF86B6C2),
    onAccent: Color(0xFF12292F),
    accentSoft: Color(0xFF233237),
    positive: Color(0xFF8DBE9A),
    pending: Color(0xFFD9AE6A),
    critical: Color(0xFFE39A8E),
  );
}

enum Script { latin, persian }

/// Type scale tuned per script. Persian runs ~1.12× larger with airier leading.
final class TypeScale {
  const TypeScale._(this.script);

  final Script script;

  static const latin = TypeScale._(Script.latin);
  static const persian = TypeScale._(Script.persian);

  String get family => script == Script.persian ? 'Vazirmatn' : 'Inter';
  List<String> get fallback =>
      script == Script.persian ? const ['Inter'] : const ['Vazirmatn'];

  double get _k => script == Script.persian ? 1.12 : 1.0;
  double get _bodyLeading => script == Script.persian ? 1.8 : 1.55;
  double get _headLeading => script == Script.persian ? 1.5 : 1.25;

  TextStyle _s(double size, FontWeight w, double height, {double ls = 0}) =>
      TextStyle(
        fontFamily: family,
        fontFamilyFallback: fallback,
        fontSize: (size * _k).roundToDouble(),
        fontWeight: w,
        height: height,
        letterSpacing: script == Script.persian ? 0 : ls,
      );

  TextStyle get display => _s(30, FontWeight.w600, _headLeading, ls: -0.4);
  TextStyle get title => _s(22, FontWeight.w600, _headLeading, ls: -0.2);
  TextStyle get heading => _s(17, FontWeight.w600, _headLeading, ls: -0.1);
  TextStyle get body => _s(16, FontWeight.w400, _bodyLeading);
  TextStyle get bodyStrong => _s(16, FontWeight.w500, _bodyLeading);
  TextStyle get small => _s(14, FontWeight.w400, _bodyLeading);
  TextStyle get label => _s(14, FontWeight.w500, 1.3, ls: 0.1);
  TextStyle get caption => _s(12, FontWeight.w500, 1.4, ls: 0.3);

  static const mono = TextStyle(
    fontFamily: 'JetBrainsMono',
    fontFamilyFallback: ['Vazirmatn'],
    fontSize: 14,
    height: 1.55,
  );
}
