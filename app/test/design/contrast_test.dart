import 'dart:math' as math;

import 'package:daftar/design/tokens.dart';
import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

double _lum(Color c) {
  double ch(double v) =>
      v <= 0.03928 ? v / 12.92 : math.pow((v + 0.055) / 1.055, 2.4).toDouble();
  return 0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b);
}

double contrast(Color a, Color b) {
  final (hi, lo) = (math.max(_lum(a), _lum(b)), math.min(_lum(a), _lum(b)));
  return (hi + 0.05) / (lo + 0.05);
}

void main() {
  for (final (name, p) in [('light', Palette.light), ('dark', Palette.dark)]) {
    group('$name palette meets WCAG AA', () {
      final surfaces = {
        'paper': p.paper,
        'raised': p.raised,
        'sunken': p.sunken,
      };
      final texts = {
        'ink': p.ink,
        'inkMuted': p.inkMuted,
        'accent': p.accent,
        'positive': p.positive,
        'pending': p.pending,
        'critical': p.critical,
      };
      for (final s in surfaces.entries) {
        for (final t in texts.entries) {
          test('${t.key} on ${s.key}', () {
            expect(contrast(t.value, s.value), greaterThanOrEqualTo(4.5));
          });
        }
      }
      test('onAccent on accent', () {
        expect(contrast(p.onAccent, p.accent), greaterThanOrEqualTo(4.5));
      });
      test('accent on accentSoft', () {
        expect(contrast(p.accent, p.accentSoft), greaterThanOrEqualTo(4.5));
      });
    });
  }
}
