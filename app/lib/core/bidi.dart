import 'package:flutter/widgets.dart';

/// Paragraph direction from the first strong character (Unicode bidi rule P2/P3),
/// independent of the UI language (brief §8.3).
TextDirection directionOf(
  String text, {
  TextDirection fallback = TextDirection.ltr,
}) {
  for (final r in text.runes) {
    if (_isRtl(r)) return TextDirection.rtl;
    if (_isLtrStrong(r)) return TextDirection.ltr;
  }
  return fallback;
}

bool _isRtl(int r) =>
    (r >= 0x0590 && r <= 0x08FF) ||
    (r >= 0xFB1D && r <= 0xFDFF) ||
    (r >= 0xFE70 && r <= 0xFEFF);

bool _isLtrStrong(int r) =>
    (r >= 0x41 && r <= 0x5A) ||
    (r >= 0x61 && r <= 0x7A) ||
    (r >= 0xC0 && r <= 0x24F) ||
    (r >= 0x370 && r <= 0x3FF) ||
    (r >= 0x400 && r <= 0x4FF);
