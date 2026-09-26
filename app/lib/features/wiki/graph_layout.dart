import 'dart:math' as math;
import 'dart:typed_data';
import 'dart:ui' show Offset, Rect;

/// Force-directed layout for the Graph view, in the manner of d3-force: springs along links,
/// many-body repulsion (Barnes–Hut, so thousands of pages stay smooth) and a weak pull towards
/// the centre that keeps separate clusters and unlinked pages in view.
///
/// Deterministic: the same graph always settles in the same place. [alpha] cools on every
/// [tick]; the layout is [settled] after about 300 ticks unless something reheats it.
class GraphLayout {
  GraphLayout({
    required this.count,
    required List<(int, int)> edges,
    Map<int, Offset> seed = const {},
  }) : x = Float64List(count),
       y = Float64List(count),
       vx = Float64List(count),
       vy = Float64List(count),
       _a = Int32List.fromList([for (final e in edges) e.$1]),
       _b = Int32List.fromList([for (final e in edges) e.$2]),
       _degree = Int32List(count) {
    for (final (a, b) in edges) {
      _degree[a]++;
      _degree[b]++;
    }
    // Phyllotaxis start, as d3 does: evenly spread, no two nodes on the same spot.
    final angle = math.pi * (3 - math.sqrt(5));
    for (var i = 0; i < count; i++) {
      final s = seed[i];
      if (s != null) {
        x[i] = s.dx;
        y[i] = s.dy;
        continue;
      }
      final r = 10 * math.sqrt(0.5 + i);
      x[i] = r * math.cos(i * angle);
      y[i] = r * math.sin(i * angle);
    }
    // Carried-over positions only need a gentle settle.
    if (seed.isNotEmpty) alpha = 0.5;
  }

  final int count;
  final Float64List x, y, vx, vy;
  final Int32List _a, _b, _degree;

  double alpha = 1;
  static const alphaMin = 0.002;
  static final _alphaDecay = 1 - math.pow(alphaMin, 1 / 300);
  static const _velocityDecay = 0.6;
  static const linkDistance = 40.0;
  static const _charge = -150.0;
  static const _gravity = 0.03;
  static const _theta2 = 0.81;

  int? _pinned;
  double _px = 0, _py = 0;

  bool get settled => alpha < alphaMin;

  Offset position(int i) => Offset(x[i], y[i]);

  int degree(int i) => _degree[i];

  /// Holds node [i] at [at] (while it is dragged) and warms the layout so its neighbours follow.
  void pin(int i, Offset at) {
    _pinned = i;
    _px = at.dx;
    _py = at.dy;
    reheat(0.3);
  }

  void unpin() => _pinned = null;

  void reheat([double to = 0.3]) => alpha = math.max(alpha, to);

  /// Runs ticks until settled or [budget] runs out. Returns whether it settled.
  bool settle({Duration budget = const Duration(milliseconds: 150)}) {
    final sw = Stopwatch()..start();
    while (!settled && sw.elapsed < budget) {
      tick();
    }
    return settled;
  }

  void tick() {
    alpha += (0 - alpha) * _alphaDecay;
    _links();
    _repel();
    for (var i = 0; i < count; i++) {
      vx[i] -= x[i] * _gravity * alpha;
      vy[i] -= y[i] * _gravity * alpha;
    }
    for (var i = 0; i < count; i++) {
      if (i == _pinned) {
        x[i] = _px;
        y[i] = _py;
        vx[i] = 0;
        vy[i] = 0;
        continue;
      }
      vx[i] *= 1 - _velocityDecay;
      vy[i] *= 1 - _velocityDecay;
      x[i] += vx[i];
      y[i] += vy[i];
    }
  }

  void _links() {
    for (var k = 0; k < _a.length; k++) {
      final a = _a[k], b = _b[k];
      var dx = x[b] + vx[b] - x[a] - vx[a];
      var dy = y[b] + vy[b] - y[a] - vy[a];
      var l = math.sqrt(dx * dx + dy * dy);
      if (l == 0) {
        dx = 1e-6 * (k + 1);
        l = dx.abs();
      }
      final strength = 1 / math.min(_degree[a], _degree[b]);
      l = (l - linkDistance) / l * alpha * strength;
      dx *= l;
      dy *= l;
      final bias = _degree[a] / (_degree[a] + _degree[b]);
      vx[b] -= dx * bias;
      vy[b] -= dy * bias;
      vx[a] += dx * (1 - bias);
      vy[a] += dy * (1 - bias);
    }
  }

  void _repel() {
    if (count < 2) return;
    var x0 = x[0], y0 = y[0], x1 = x[0], y1 = y[0];
    for (var i = 1; i < count; i++) {
      x0 = math.min(x0, x[i]);
      y0 = math.min(y0, y[i]);
      x1 = math.max(x1, x[i]);
      y1 = math.max(y1, y[i]);
    }
    final root = _Quad(x0, y0, math.max(x1 - x0, y1 - y0) + 1);
    for (var i = 0; i < count; i++) {
      root.insert(i, x, y, 0);
    }
    for (var i = 0; i < count; i++) {
      _push(i, root);
    }
  }

  void _push(int i, _Quad q) {
    if (q.mass == 0) return;
    final dx = q.cx - x[i], dy = q.cy - y[i];
    var l2 = dx * dx + dy * dy;
    final leaf = q.kids == null;
    if (leaf || q.size * q.size / _theta2 < l2) {
      final mass = leaf && q.point == i ? q.mass - 1 : q.mass;
      if (mass <= 0 || l2 == 0) return;
      if (l2 < 1) l2 = 1;
      final f = _charge * alpha * mass / l2;
      vx[i] += dx * f;
      vy[i] += dy * f;
      return;
    }
    for (final k in q.kids!) {
      if (k != null) _push(i, k);
    }
  }

  /// Bounds of [only] (or every node), padded by [pad].
  Rect bounds(Iterable<int> only, {double pad = 0}) {
    final it = only.iterator;
    if (!it.moveNext()) return Rect.zero;
    var x0 = x[it.current], y0 = y[it.current], x1 = x0, y1 = y0;
    while (it.moveNext()) {
      final i = it.current;
      x0 = math.min(x0, x[i]);
      y0 = math.min(y0, y[i]);
      x1 = math.max(x1, x[i]);
      y1 = math.max(y1, y[i]);
    }
    return Rect.fromLTRB(x0 - pad, y0 - pad, x1 + pad, y1 + pad);
  }
}

/// A Barnes–Hut quadtree cell: its centre of mass and how many nodes it holds.
class _Quad {
  _Quad(this.x0, this.y0, this.size);
  final double x0, y0, size;
  double mass = 0, cx = 0, cy = 0;

  /// The node in a leaf holding one (coincident extras only add mass).
  int point = -1;
  List<_Quad?>? kids;

  void insert(int i, Float64List xs, Float64List ys, int depth) {
    final px = xs[i], py = ys[i];
    cx = (cx * mass + px) / (mass + 1);
    cy = (cy * mass + py) / (mass + 1);
    mass++;
    if (kids == null) {
      if (point < 0 && mass == 1) {
        point = i;
        return;
      }
      if (depth > 24) return;
      kids = List.filled(4, null);
      final p = point;
      point = -1;
      if (p >= 0) _child(xs[p], ys[p]).insert(p, xs, ys, depth + 1);
    }
    _child(px, py).insert(i, xs, ys, depth + 1);
  }

  _Quad _child(double px, double py) {
    final h = size / 2;
    final right = px >= x0 + h, bottom = py >= y0 + h;
    final k = (bottom ? 2 : 0) + (right ? 1 : 0);
    return kids![k] ??= _Quad(right ? x0 + h : x0, bottom ? y0 + h : y0, h);
  }
}
