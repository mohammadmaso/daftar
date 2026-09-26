import 'package:flutter/widgets.dart';

import '../theme.dart';
import '../tokens.dart';

/// The app icon as a vector: a ruled notebook page with a bookmark on an ink tile. Geometry
/// mirrors packaging/icon/make_icon.py (1024-unit grid) so the mark and the icon never drift.
class Logo extends StatelessWidget {
  const Logo({super.key, this.size = 36});
  final double size;

  @override
  Widget build(BuildContext context) => ExcludeSemantics(
    child: CustomPaint(size: Size.square(size), painter: const _LogoPainter()),
  );
}

class _LogoPainter extends CustomPainter {
  const _LogoPainter();

  @override
  void paint(Canvas canvas, Size size) {
    final u = size.width / 1024;
    Rect r(double l, double t, double rt, double b) =>
        Rect.fromLTRB(l * u, t * u, rt * u, b * u);
    final ink = Paint()..color = Brand.ink;
    final paper = Paint()..color = Brand.paper;

    canvas.drawRRect(
      RRect.fromRectAndRadius(Offset.zero & size, Radius.circular(230 * u)),
      ink,
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(r(292, 212, 732, 812), Radius.circular(36 * u)),
      paper,
    );
    // Rules stay at least a pixel thick so they survive at toolbar sizes.
    final half = (10 * u).clamp(0.5, double.infinity);
    for (final y in const [420.0, 520.0, 620.0]) {
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          Rect.fromLTRB(372 * u, y * u - half, 652 * u, y * u + half),
          Radius.circular(half),
        ),
        ink,
      );
    }
    canvas.drawPath(
      Path()
        ..moveTo(556 * u, 212 * u)
        ..lineTo(636 * u, 212 * u)
        ..lineTo(636 * u, 352 * u)
        ..lineTo(596 * u, 316 * u)
        ..lineTo(556 * u, 352 * u)
        ..close(),
      ink,
    );
  }

  @override
  bool shouldRepaint(_LogoPainter oldDelegate) => false;
}

/// The mark beside the product name, set in the current reading face. [latinFont] and
/// [persianFont] pin a face (the style guide shows both the sans and the serif lockups).
class Wordmark extends StatelessWidget {
  const Wordmark({
    super.key,
    required this.name,
    this.size = 40,
    this.latinFont,
    this.persianFont,
  });

  final String name;

  /// Height of the mark; the name is sized from it.
  final double size;
  final LatinFont? latinFont;
  final PersianFont? persianFont;

  @override
  Widget build(BuildContext context) {
    final t = context.type;
    final persian = name.runes.any((c) => c >= 0x0600 && c <= 0x06FF);
    final family = persian
        ? (persianFont ?? t.persianFont).family
        : (latinFont ?? t.latinFont).family;
    final serif = !persian && (latinFont ?? t.latinFont) != LatinFont.sans;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Logo(size: size),
        SizedBox(width: size * 0.32),
        Text(
          name,
          textScaler: TextScaler.noScaling,
          style: TextStyle(
            fontFamily: family,
            fontSize: size * (persian ? 0.66 : 0.62),
            fontWeight: persian ? FontWeight.w700 : FontWeight.w600,
            letterSpacing: persian ? 0 : size * (serif ? 0.0 : -0.012),
            height: 1.0,
            color: context.palette.ink,
          ),
        ),
      ],
    );
  }
}
