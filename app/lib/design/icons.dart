import 'dart:math' as math;

import 'package:flutter/widgets.dart';

import 'theme.dart';

/// Hand-drawn stroke icons on a 24-unit grid. Directional glyphs mirror in RTL;
/// media glyphs (mic, waveform, camera) never do.
enum DIcons {
  today(_today),
  wiki(_wiki),
  ask(_ask),
  mic(_mic),
  camera(_camera),
  search(_search),
  chevronForward(_chevron, directional: true),
  back(_back, directional: true),
  check(_check),
  close(_close),
  plus(_plus),
  sun(_sun),
  moon(_moon),
  device(_device),
  copy(_copy),
  edit(_edit),
  graph(_graph),
  review(_review),
  activity(_activity),
  undo(_undo, directional: true),
  speaker(_speaker),
  stop(_stop),
  image(_image),
  send(_send, directional: true);

  const DIcons(this._paint, {this.directional = false});

  final void Function(Canvas, Paint) _paint;
  final bool directional;
}

class DIcon extends StatelessWidget {
  const DIcon(
    this.icon, {
    super.key,
    this.size = 24,
    this.color,
    this.semanticLabel,
  });

  final DIcons icon;
  final double size;
  final Color? color;
  final String? semanticLabel;

  @override
  Widget build(BuildContext context) {
    final mirror =
        icon.directional && Directionality.of(context) == TextDirection.rtl;
    Widget glyph = CustomPaint(
      size: Size.square(size),
      painter: _IconPainter(icon, color ?? context.palette.ink),
    );
    if (mirror) {
      glyph = Transform.flip(flipX: true, child: glyph);
    }
    return Semantics(
      label: semanticLabel,
      excludeSemantics: semanticLabel == null,
      child: SizedBox.square(dimension: size, child: glyph),
    );
  }
}

class _IconPainter extends CustomPainter {
  _IconPainter(this.icon, this.color);

  final DIcons icon;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final s = size.width / 24;
    canvas.scale(s);
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.6
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round
      ..isAntiAlias = true;
    icon._paint(canvas, paint);
  }

  @override
  bool shouldRepaint(_IconPainter old) =>
      old.icon != icon || old.color != color;
}

RRect _rr(double l, double t, double r, double b, double rad) =>
    RRect.fromLTRBR(l, t, r, b, Radius.circular(rad));

// A page with a folded corner and today's line.
void _today(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(6, 3.5)
      ..lineTo(14.5, 3.5)
      ..lineTo(18.5, 7.5)
      ..lineTo(18.5, 20.5)
      ..lineTo(6, 20.5)
      ..close(),
    p,
  );
  c.drawPath(
    Path()
      ..moveTo(14.5, 3.5)
      ..lineTo(14.5, 7.5)
      ..lineTo(18.5, 7.5),
    p,
  );
  c.drawLine(const Offset(9, 12), const Offset(15.5, 12), p);
  c.drawLine(const Offset(9, 15.5), const Offset(13, 15.5), p);
}

// Open notebook.
void _wiki(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(12, 6.5)
      ..cubicTo(9.5, 5, 6.5, 4.8, 3.5, 5.5)
      ..lineTo(3.5, 18.5)
      ..cubicTo(6.5, 17.8, 9.5, 18, 12, 19.5)
      ..cubicTo(14.5, 18, 17.5, 17.8, 20.5, 18.5)
      ..lineTo(20.5, 5.5)
      ..cubicTo(17.5, 4.8, 14.5, 5, 12, 6.5)
      ..close(),
    p,
  );
  c.drawLine(const Offset(12, 6.5), const Offset(12, 19.5), p);
}

// A quiet question: a single ink stroke ending in a dot.
void _ask(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(8.5, 8.5)
      ..cubicTo(8.5, 6, 10, 4.5, 12, 4.5)
      ..cubicTo(14.2, 4.5, 15.5, 6, 15.5, 8)
      ..cubicTo(15.5, 10.5, 12, 11, 12, 14),
    p,
  );
  c.drawCircle(const Offset(12, 18.5), 0.6, p..style = PaintingStyle.fill);
  p.style = PaintingStyle.stroke;
}

void _mic(Canvas c, Paint p) {
  c.drawRRect(_rr(9, 3.5, 15, 14, 3), p);
  c.drawPath(
    Path()
      ..moveTo(6, 11)
      ..cubicTo(6, 14.5, 8.7, 17, 12, 17)
      ..cubicTo(15.3, 17, 18, 14.5, 18, 11),
    p,
  );
  c.drawLine(const Offset(12, 17), const Offset(12, 20.5), p);
}

void _camera(Canvas c, Paint p) {
  c.drawRRect(_rr(3.5, 7, 20.5, 19, 2.5), p);
  c.drawPath(
    Path()
      ..moveTo(8.5, 7)
      ..lineTo(10, 4.5)
      ..lineTo(14, 4.5)
      ..lineTo(15.5, 7),
    p,
  );
  c.drawCircle(const Offset(12, 13), 3.2, p);
}

void _search(Canvas c, Paint p) {
  c.drawCircle(const Offset(10.5, 10.5), 6, p);
  c.drawLine(const Offset(15, 15), const Offset(20, 20), p);
}

void _chevron(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(9.5, 6)
      ..lineTo(15.5, 12)
      ..lineTo(9.5, 18),
    p,
  );
}

void _back(Canvas c, Paint p) {
  c.drawLine(const Offset(19, 12), const Offset(5.5, 12), p);
  c.drawPath(
    Path()
      ..moveTo(11, 6)
      ..lineTo(5, 12)
      ..lineTo(11, 18),
    p,
  );
}

void _check(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(5, 12.5)
      ..lineTo(9.5, 17)
      ..lineTo(19, 7),
    p,
  );
}

void _close(Canvas c, Paint p) {
  c.drawLine(const Offset(6.5, 6.5), const Offset(17.5, 17.5), p);
  c.drawLine(const Offset(17.5, 6.5), const Offset(6.5, 17.5), p);
}

void _plus(Canvas c, Paint p) {
  c.drawLine(const Offset(12, 5), const Offset(12, 19), p);
  c.drawLine(const Offset(5, 12), const Offset(19, 12), p);
}

void _sun(Canvas c, Paint p) {
  c.drawCircle(const Offset(12, 12), 4, p);
  for (var i = 0; i < 8; i++) {
    final a = i * math.pi / 4;
    c.drawLine(
      Offset(12 + 7 * math.cos(a), 12 + 7 * math.sin(a)),
      Offset(12 + 9 * math.cos(a), 12 + 9 * math.sin(a)),
      p,
    );
  }
}

void _moon(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(19.5, 14.5)
      ..cubicTo(17, 19.5, 10, 20.5, 6.5, 16.5)
      ..cubicTo(3, 12.5, 4.5, 6, 9.5, 4.5)
      ..cubicTo(7.5, 9, 10.5, 15.5, 19.5, 14.5)
      ..close(),
    p,
  );
}

void _device(Canvas c, Paint p) {
  c.drawRRect(_rr(3.5, 5, 20.5, 15.5, 2), p);
  c.drawLine(const Offset(9, 19.5), const Offset(15, 19.5), p);
}

void _copy(Canvas c, Paint p) {
  c.drawRRect(_rr(8.5, 8.5, 19.5, 19.5, 2), p);
  c.drawPath(
    Path()
      ..moveTo(15.5, 8.5)
      ..lineTo(15.5, 6)
      ..arcToPoint(
        const Offset(13.5, 4.5),
        radius: const Radius.circular(2),
        clockwise: false,
      )
      ..lineTo(6.5, 4.5)
      ..arcToPoint(
        const Offset(4.5, 6.5),
        radius: const Radius.circular(2),
        clockwise: false,
      )
      ..lineTo(4.5, 13.5)
      ..arcToPoint(
        const Offset(6.5, 15.5),
        radius: const Radius.circular(2),
        clockwise: false,
      )
      ..lineTo(8.5, 15.5),
    p,
  );
}

// A pen nib drawing a line.
void _edit(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(15, 4.5)
      ..lineTo(19.5, 9)
      ..lineTo(9, 19.5)
      ..lineTo(4.5, 19.5)
      ..lineTo(4.5, 15)
      ..close(),
    p,
  );
  c.drawLine(const Offset(12.5, 7), const Offset(17, 11.5), p);
}

// Three linked nodes.
void _graph(Canvas c, Paint p) {
  c.drawCircle(const Offset(6.5, 7), 2.3, p);
  c.drawCircle(const Offset(17.5, 6), 2.3, p);
  c.drawCircle(const Offset(12, 17.5), 2.3, p);
  c.drawLine(const Offset(8.7, 7), const Offset(15.2, 6.2), p);
  c.drawLine(const Offset(7.6, 9), const Offset(10.9, 15.4), p);
  c.drawLine(const Offset(16.4, 8.1), const Offset(13.1, 15.4), p);
}

// A small stack of cards.
void _review(Canvas c, Paint p) {
  c.drawRRect(_rr(4.5, 7.5, 17.5, 19.5, 2), p);
  c.drawLine(const Offset(7, 4.5), const Offset(18.5, 4.5), p);
  c.drawLine(const Offset(19.5, 5.5), const Offset(19.5, 16.5), p);
  c.drawPath(
    Path()
      ..moveTo(8, 13.5)
      ..lineTo(10.2, 15.7)
      ..lineTo(14.2, 11.2),
    p,
  );
}

// A timeline: dots on a line.
void _activity(Canvas c, Paint p) {
  c.drawLine(const Offset(7, 4), const Offset(7, 20), p);
  for (final y in [7.0, 12.0, 17.0]) {
    c.drawCircle(Offset(7, y), 1.4, p..style = PaintingStyle.fill);
    p.style = PaintingStyle.stroke;
    c.drawLine(Offset(10.5, y), Offset(y == 12 ? 16 : 19, y), p);
  }
}

// An arrow turning back.
void _undo(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(9, 5.5)
      ..lineTo(4.5, 10)
      ..lineTo(9, 14.5),
    p,
  );
  c.drawPath(
    Path()
      ..moveTo(4.5, 10)
      ..lineTo(14, 10)
      ..cubicTo(17.5, 10, 19.5, 12.5, 19.5, 15)
      ..cubicTo(19.5, 17.5, 17.5, 19.5, 14.5, 19.5)
      ..lineTo(11, 19.5),
    p,
  );
}

// Sound coming out.
void _speaker(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(4.5, 9.5)
      ..lineTo(8, 9.5)
      ..lineTo(12.5, 5.5)
      ..lineTo(12.5, 18.5)
      ..lineTo(8, 14.5)
      ..lineTo(4.5, 14.5)
      ..close(),
    p,
  );
  c.drawArc(const Rect.fromLTRB(11, 8, 17, 16), -0.9, 1.8, false, p);
  c.drawArc(const Rect.fromLTRB(11, 5, 20.5, 19), -0.9, 1.8, false, p);
}

void _stop(Canvas c, Paint p) {
  c.drawRRect(_rr(6.5, 6.5, 17.5, 17.5, 2.5), p);
}

// A framed picture with a hill.
void _image(Canvas c, Paint p) {
  c.drawRRect(_rr(3.5, 5, 20.5, 19, 2), p);
  c.drawCircle(const Offset(9, 10), 1.6, p);
  c.drawPath(
    Path()
      ..moveTo(4, 17)
      ..lineTo(10, 12.5)
      ..lineTo(14, 15.5)
      ..lineTo(16.5, 13.5)
      ..lineTo(20, 16.5),
    p,
  );
}

// A paper plane stroke.
void _send(Canvas c, Paint p) {
  c.drawPath(
    Path()
      ..moveTo(4, 11.5)
      ..lineTo(20, 4.5)
      ..lineTo(14.5, 19.5)
      ..lineTo(11.5, 13)
      ..close(),
    p,
  );
  c.drawLine(const Offset(11.5, 13), const Offset(20, 4.5), p);
}
