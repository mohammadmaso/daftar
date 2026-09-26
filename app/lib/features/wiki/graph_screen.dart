import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart' show Material, Theme;
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/bidi.dart';
import '../../core/errors.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import 'graph_layout.dart';
import 'wiki_state.dart';

/// The Graph view: every page as a node and every wikilink as an edge, laid out by forces.
///
/// Only what helps to find your way: pan and zoom, fit to screen, drag a node, tap to see a
/// page's neighbours (tap again to open it), find a page by name, colour and filter by vault,
/// and hide unlinked pages. [focus] selects and centres a page when the view opens.
class GraphScreen extends ConsumerStatefulWidget {
  const GraphScreen({super.key, this.focus});
  final String? focus;

  @override
  ConsumerState<GraphScreen> createState() => _GraphScreenState();
}

class _GraphScreenState extends ConsumerState<GraphScreen>
    with TickerProviderStateMixin {
  static const _minScale = 0.08;
  static const _maxScale = 4.0;

  final _query = TextEditingController();
  final _repaint = ValueNotifier(0);
  late final Ticker _ticker = createTicker((_) => _tick());
  late final AnimationController _camera = AnimationController(
    vsync: this,
    duration: Motion.slow,
  )..addListener(_moveCamera);

  WikiGraph? _data;
  String? _vault;
  bool _unlinked = true;

  /// Data indexes of the nodes on screen, and the layout over them (layout index = position in
  /// [_shown]).
  List<int> _shown = const [];
  List<String> _paths = const [];
  Map<String, int> _slot = const {};
  List<List<int>> _adjacent = const [];
  List<(int, int)> _edges = const [];
  GraphLayout? _layout;

  int? _selected;
  int? _hover;
  int? _dragging;
  bool _touched = false;
  String? _pendingFocus;

  Size _size = Size.zero;
  double _scale = 1;
  Offset _pan = Offset.zero;
  double _gestureScale = 1;
  Offset _gesturePan = Offset.zero;
  Offset _gestureFocal = Offset.zero;
  (double, Offset, double, Offset)? _flight;

  @override
  void initState() {
    super.initState();
    _pendingFocus = widget.focus;
    ref.listenManual<AsyncValue<WikiGraph>>(wikiGraphProvider, (_, next) {
      final g = next.value;
      if (g != null && !identical(g, _data)) {
        _data = g;
        _rebuild();
        if (mounted) setState(() {});
      }
    }, fireImmediately: true);
  }

  @override
  void dispose() {
    _ticker.dispose();
    _camera.dispose();
    _query.dispose();
    _repaint.dispose();
    super.dispose();
  }

  // ---- Graph and layout -------------------------------------------------------------------

  /// Rebuilds the shown subgraph after the data or a filter changed, keeping every node that
  /// stays where it was.
  void _rebuild() {
    final g = _data!;
    final inVault = [
      for (var i = 0; i < g.nodes.length; i++)
        _vault == null || g.nodes[i].page.vault == _vault,
    ];
    final degree = List.filled(g.nodes.length, 0);
    for (var k = 0; k < g.edgeFrom.length; k++) {
      final a = g.edgeFrom[k], b = g.edgeTo[k];
      if (inVault[a] && inVault[b]) {
        degree[a]++;
        degree[b]++;
      }
    }
    final shown = [
      for (var i = 0; i < g.nodes.length; i++)
        if (inVault[i] && (_unlinked || degree[i] > 0)) i,
    ];
    final at = {for (var s = 0; s < shown.length; s++) shown[s]: s};
    final edges = <(int, int)>[];
    final adjacent = [for (final _ in shown) <int>[]];
    for (var k = 0; k < g.edgeFrom.length; k++) {
      final a = at[g.edgeFrom[k]], b = at[g.edgeTo[k]];
      if (a == null || b == null) continue;
      edges.add((a, b));
      adjacent[a].add(b);
      adjacent[b].add(a);
    }
    final old = _layout;
    final seed = <int, Offset>{};
    if (old != null) {
      for (var s = 0; s < shown.length; s++) {
        final was = _slot[g.nodes[shown[s]].page.path];
        if (was != null && was < old.count) seed[s] = old.position(was);
      }
    }
    final selected = _selected;
    final selectedPath = selected != null && selected < _paths.length
        ? _paths[selected]
        : null;
    _shown = shown;
    _paths = [for (final i in shown) g.nodes[i].page.path];
    _slot = {for (var s = 0; s < shown.length; s++) _paths[s]: s};
    _edges = edges;
    _adjacent = adjacent;
    _layout = GraphLayout(count: shown.length, edges: edges, seed: seed);
    _selected = selectedPath == null ? null : _slot[selectedPath];
    _hover = null;
    _dragging = null;
    if (seed.isEmpty) {
      _layout!.settle();
      _touched = false;
    }
    if (_size != Size.zero) _afterLayout();
    if (!_layout!.settled) _startTicker();
  }

  WikiGraphNode _node(int slot) => _data!.nodes[_shown[slot]];

  void _startTicker() {
    if (!_ticker.isActive) _ticker.start();
  }

  void _tick() {
    final layout = _layout;
    if (layout == null || layout.settled) {
      _ticker.stop();
      if (!_touched) _fit(animate: false);
      return;
    }
    // With reduced motion, jump to the settled layout instead of animating into it.
    if (MediaQuery.maybeDisableAnimationsOf(context) ?? false) {
      layout.settle(budget: const Duration(milliseconds: 400));
    } else {
      layout.tick();
    }
    if (!_touched && _dragging == null) _fit(animate: false);
    _repaint.value++;
  }

  /// First sizing, or a new layout: frame it, or centre the page asked for. Runs during build
  /// and layout, so it only sets fields and asks for one more frame.
  void _afterLayout() {
    final focus = _pendingFocus;
    final slot = focus == null ? null : _slot[focus];
    if (slot != null) {
      _pendingFocus = null;
      _selected = slot;
      _touched = true;
      final scale = math.max(_scale, 1.2);
      _fly(scale, -_layout!.position(slot) * scale, animate: false);
      SchedulerBinding.instance.addPostFrameCallback((_) {
        if (mounted) setState(() {});
      });
      return;
    }
    if (!_touched) _fit(animate: false);
  }

  // ---- Camera -----------------------------------------------------------------------------

  Offset _toScreen(Offset w) => _size.center(Offset.zero) + _pan + w * _scale;

  Offset _toWorld(Offset s) => (s - _size.center(Offset.zero) - _pan) / _scale;

  void _fly(double scale, Offset pan, {bool animate = true}) {
    if (!animate || (MediaQuery.maybeDisableAnimationsOf(context) ?? false)) {
      _camera.stop();
      _scale = scale;
      _pan = pan;
      _repaint.value++;
      return;
    }
    _flight = (_scale, _pan, scale, pan);
    _camera.forward(from: 0);
  }

  void _moveCamera() {
    final f = _flight;
    if (f == null) return;
    final t = Motion.ease.transform(_camera.value);
    _scale = f.$1 + (f.$3 - f.$1) * t;
    _pan = Offset.lerp(f.$2, f.$4, t)!;
    _repaint.value++;
  }

  void _fit({bool animate = true}) {
    final layout = _layout;
    if (layout == null || _size.isEmpty) return;
    if (layout.count == 0) {
      _fly(1, Offset.zero, animate: animate);
      return;
    }
    final b = layout.bounds(
      Iterable<int>.generate(layout.count),
      pad: GraphLayout.linkDistance,
    );
    final scale = math
        .min(_size.width / b.width, _size.height / b.height)
        .clamp(_minScale, 1.6);
    _fly(scale, -b.center * scale, animate: animate);
  }

  void _zoomAt(Offset focal, double factor) {
    _camera.stop();
    final world = _toWorld(focal);
    _scale = (_scale * factor).clamp(_minScale, _maxScale);
    _pan = focal - _size.center(Offset.zero) - world * _scale;
    _touched = true;
    _repaint.value++;
  }

  // ---- Selection and gestures -------------------------------------------------------------

  int? _hit(Offset screen) {
    final layout = _layout;
    if (layout == null) return null;
    final w = _toWorld(screen);
    final slop = 10 / _scale;
    int? best;
    var bestD = double.infinity;
    for (var i = 0; i < layout.count; i++) {
      final d = (layout.position(i) - w).distance;
      if (d <= _radius(i) + slop && d < bestD) {
        best = i;
        bestD = d;
      }
    }
    return best;
  }

  double _radius(int slot) =>
      math.min(3.5 + math.sqrt(_layout!.degree(slot)) * 1.8, 16);

  void _select(int? slot, {bool center = false}) {
    setState(() => _selected = slot);
    if (slot != null && center) {
      _touched = true;
      final scale = math.max(_scale, 1.2);
      _fly(scale, -_layout!.position(slot) * scale);
    }
  }

  void _open(int slot) => context.push(pageRoute(_node(slot).page.path));

  void _onTap(TapUpDetails d) {
    final hit = _hit(d.localPosition);
    if (hit != null && hit == _selected) {
      _open(hit);
    } else {
      _select(hit);
    }
  }

  void _onScaleStart(ScaleStartDetails d) {
    _camera.stop();
    final hit = d.pointerCount == 1 ? _hit(d.localFocalPoint) : null;
    if (hit != null) {
      _dragging = hit;
      _layout!.pin(hit, _toWorld(d.localFocalPoint));
      _startTicker();
      return;
    }
    _gestureScale = _scale;
    _gesturePan = _pan;
    _gestureFocal = d.localFocalPoint;
  }

  void _onScaleUpdate(ScaleUpdateDetails d) {
    _touched = true;
    if (_dragging != null) {
      _layout!.pin(_dragging!, _toWorld(d.localFocalPoint));
      _startTicker();
      return;
    }
    final world =
        (_gestureFocal - _size.center(Offset.zero) - _gesturePan) /
        _gestureScale;
    _scale = (_gestureScale * d.scale).clamp(_minScale, _maxScale);
    _pan = d.localFocalPoint - _size.center(Offset.zero) - world * _scale;
    _repaint.value++;
  }

  void _onScaleEnd(ScaleEndDetails d) {
    if (_dragging != null) {
      _layout!.unpin();
      _dragging = null;
    }
  }

  void _onHover(PointerHoverEvent e) {
    final hit = _hit(e.localPosition);
    if (hit != _hover) setState(() => _hover = hit);
  }

  void _setVault(String? v) {
    if (v == _vault) return;
    setState(() {
      _vault = v;
      _touched = false;
      _rebuild();
    });
  }

  void _setUnlinked(bool v) => setState(() {
    _unlinked = v;
    _rebuild();
  });

  List<int> _matches(String q) {
    final needle = q.trim().toLowerCase();
    if (needle.isEmpty) return const [];
    final out = <int>[];
    for (var s = 0; s < _shown.length; s++) {
      final pg = _node(s).page;
      if (pg.titleEn.toLowerCase().contains(needle) ||
          pg.titleFa.contains(needle) ||
          pg.path.split('/').last.toLowerCase().contains(needle)) {
        out.add(s);
      }
    }
    out.sort((a, b) => _layout!.degree(b).compareTo(_layout!.degree(a)));
    return out;
  }

  // ---- Build ------------------------------------------------------------------------------

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final vaults = ref.watch(vaultsProvider).value ?? const <Vault>[];
    final graph = ref.watch(wikiGraphProvider);
    final hues = Theme.of(context).brightness == Brightness.dark
        ? GraphHues.dark
        : GraphHues.light;
    Color hue(String vault) {
      final i = vaults.indexWhere((v) => v.id == vault);
      return i < 0 ? p.inkMuted : hues[i % hues.length];
    }

    String title(int slot) {
      final pg = _node(slot).page;
      return pageTitle(lang, pg.titleEn, pg.titleFa, pg.path);
    }

    final matches = _matches(_query.text);
    final layout = _layout;

    Widget canvas() => LayoutBuilder(
      builder: (context, box) {
        final size = box.biggest;
        if (size != _size) {
          final first = _size == Size.zero;
          _size = size;
          if (first) {
            _afterLayout();
          } else if (!_touched) {
            _fit(animate: false);
          }
        }
        return MouseRegion(
          cursor: _hover != null
              ? SystemMouseCursors.click
              : SystemMouseCursors.basic,
          onHover: _onHover,
          onExit: (_) => setState(() => _hover = null),
          child: Listener(
            onPointerSignal: (e) {
              if (e is PointerScrollEvent) {
                _zoomAt(e.localPosition, math.exp(-e.scrollDelta.dy / 400));
              }
            },
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTapUp: _onTap,
              onScaleStart: _onScaleStart,
              onScaleUpdate: _onScaleUpdate,
              onScaleEnd: _onScaleEnd,
              child: Semantics(
                label: l.graphSummary(
                  l.pagesCount(_shown.length),
                  l.linksCount(_edges.length),
                ),
                child: CustomPaint(
                  size: size,
                  painter: _GraphPainter(
                    repaint: _repaint,
                    state: this,
                    palette: p,
                    color: [
                      for (var s = 0; s < _shown.length; s++)
                        hue(_node(s).page.vault),
                    ],
                    labels: [for (var s = 0; s < _shown.length; s++) title(s)],
                    labelStyle: context.type.caption,
                    matches: matches.toSet(),
                    searching: _query.text.trim().isNotEmpty,
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );

    final Widget body;
    if (graph.hasError && _data == null) {
      body = Center(
        child: Padding(
          padding: const EdgeInsets.all(Space.x6),
          child: Text(
            humanError(graph.error!),
            textAlign: TextAlign.center,
            style: context.type.body.copyWith(color: p.critical),
          ),
        ),
      );
    } else if (_data == null || layout == null) {
      body = const SizedBox.expand();
    } else if (_data!.nodes.isEmpty) {
      body = Center(
        child: Padding(
          padding: const EdgeInsets.all(Space.x6),
          child: Text(
            l.graphEmpty,
            textAlign: TextAlign.center,
            style: context.type.body.copyWith(color: p.inkMuted),
          ),
        ),
      );
    } else {
      body = canvas();
    }

    final selected = _selected;
    return Material(
      color: p.paper,
      child: CallbackShortcuts(
        bindings: {
          const SingleActivator(LogicalKeyboardKey.escape): () {
            if (_selected != null) _select(null);
          },
        },
        child: Focus(
          autofocus: true,
          child: Stack(
            children: [
              Positioned.fill(child: body),
              PositionedDirectional(
                start: 0,
                end: 0,
                top: 0,
                child: SafeArea(
                  bottom: false,
                  child: Padding(
                    padding: const EdgeInsetsDirectional.fromSTEB(
                      Space.x2,
                      Space.x2,
                      Space.x2,
                      0,
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Row(
                          children: [
                            if (context.canPop())
                              DIconButton(
                                icon: DIcons.back,
                                semanticLabel: l.back,
                                onPressed: () => context.pop(),
                              ),
                            const SizedBox(width: Space.x1),
                            Expanded(
                              child: Semantics(
                                header: true,
                                child: Text(
                                  l.graphTitle,
                                  style: context.type.title,
                                ),
                              ),
                            ),
                            DIconButton(
                              icon: DIcons.fit,
                              semanticLabel: l.graphFit,
                              color: p.inkMuted,
                              onPressed: () {
                                _touched = false;
                                _fit();
                              },
                            ),
                          ],
                        ),
                        Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: Space.x2,
                          ),
                          child: Align(
                            alignment: AlignmentDirectional.centerStart,
                            child: ConstrainedBox(
                              constraints: const BoxConstraints(maxWidth: 420),
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  DTextField(
                                    controller: _query,
                                    hint: l.graphFind,
                                    leading: DIcons.search,
                                    onChanged: (_) => setState(() {}),
                                    onSubmitted: (_) {
                                      if (matches.isNotEmpty) {
                                        _pick(matches.first);
                                      }
                                    },
                                  ),
                                  if (matches.isNotEmpty)
                                    Padding(
                                      padding: const EdgeInsets.only(
                                        top: Space.x1,
                                      ),
                                      child: DSurface(
                                        child: Column(
                                          mainAxisSize: MainAxisSize.min,
                                          crossAxisAlignment:
                                              CrossAxisAlignment.stretch,
                                          children: [
                                            for (final s in matches.take(6))
                                              DListRow(
                                                title: title(s),
                                                subtitle: l.linksCount(
                                                  layout!.degree(s),
                                                ),
                                                onTap: () => _pick(s),
                                              ),
                                          ],
                                        ),
                                      ),
                                    ),
                                ],
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(height: Space.x1),
                        SingleChildScrollView(
                          scrollDirection: Axis.horizontal,
                          padding: const EdgeInsets.symmetric(
                            horizontal: Space.x2,
                          ),
                          child: Row(
                            children: [
                              DChip(
                                label: l.graphUnlinked,
                                selected: _unlinked,
                                onTap: () => _setUnlinked(!_unlinked),
                              ),
                              const SizedBox(width: Space.x4),
                              DChip(
                                label: l.allVaults,
                                selected: _vault == null,
                                onTap: () => _setVault(null),
                              ),
                              for (final v in vaults) ...[
                                const SizedBox(width: Space.x2),
                                DChip(
                                  label: lang == 'fa' ? v.titleFa : v.titleEn,
                                  dot: hue(v.id),
                                  selected: _vault == v.id,
                                  onTap: () =>
                                      _setVault(_vault == v.id ? null : v.id),
                                ),
                              ],
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              PositionedDirectional(
                start: 0,
                end: 0,
                bottom: 0,
                child: SafeArea(
                  top: false,
                  child: Padding(
                    padding: const EdgeInsets.all(Space.x4),
                    child: selected != null && selected < _shown.length
                        ? Align(
                            alignment: AlignmentDirectional.bottomStart,
                            child: ConstrainedBox(
                              constraints: const BoxConstraints(maxWidth: 420),
                              child: _SelectedCard(
                                title: title(selected),
                                summary: _node(selected).page.summary,
                                vault: hue(_node(selected).page.vault),
                                links: layout!.degree(selected),
                                onOpen: () => _open(selected),
                                onClose: () => _select(null),
                              ),
                            ),
                          )
                        : (_data != null && _data!.nodes.isNotEmpty)
                        ? Text(
                            l.graphSummary(
                              l.pagesCount(_shown.length),
                              l.linksCount(_edges.length),
                            ),
                            style: context.type.caption.copyWith(
                              color: p.inkMuted,
                            ),
                          )
                        : const SizedBox.shrink(),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _pick(int slot) {
    _query.clear();
    FocusManager.instance.primaryFocus?.unfocus();
    _select(slot, center: true);
  }
}

class _SelectedCard extends StatelessWidget {
  const _SelectedCard({
    required this.title,
    required this.summary,
    required this.vault,
    required this.links,
    required this.onOpen,
    required this.onClose,
  });
  final String title;
  final String summary;
  final Color vault;
  final int links;
  final VoidCallback onOpen;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    return DSurface(
      padding: const EdgeInsetsDirectional.fromSTEB(
        Space.x4,
        Space.x3,
        Space.x2,
        Space.x3,
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 7),
            child: Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(color: vault, shape: BoxShape.circle),
            ),
          ),
          const SizedBox(width: Space.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  textDirection: directionOf(
                    title,
                    fallback: Directionality.of(context),
                  ),
                  style: context.type.heading,
                ),
                Text(
                  '${l.linksCount(links)} · ${l.graphOpenHint}',
                  style: context.type.caption.copyWith(color: p.inkMuted),
                ),
                if (summary.trim().isNotEmpty) ...[
                  const SizedBox(height: Space.x1),
                  Text(
                    summary,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    textDirection: directionOf(
                      summary,
                      fallback: Directionality.of(context),
                    ),
                    style: context.type.small.copyWith(color: p.inkMuted),
                  ),
                ],
                const SizedBox(height: Space.x3),
                DButton(label: l.openPage, onPressed: onOpen),
              ],
            ),
          ),
          DIconButton(
            icon: DIcons.close,
            semanticLabel: l.close,
            color: p.inkMuted,
            onPressed: onClose,
          ),
        ],
      ),
    );
  }
}

/// Paints edges and nodes in world space and labels in screen space (so text stays legible at
/// any zoom). Labels fade in as you zoom; the selected or hovered page, its neighbours and search
/// matches are always labelled, and everything else dims around them.
class _GraphPainter extends CustomPainter {
  _GraphPainter({
    required Listenable repaint,
    required this.state,
    required this.palette,
    required this.color,
    required this.labels,
    required this.labelStyle,
    required this.matches,
    required this.searching,
  }) : super(repaint: repaint);

  final _GraphScreenState state;
  final Palette palette;
  final List<Color> color;
  final List<String> labels;
  final TextStyle labelStyle;
  final Set<int> matches;
  final bool searching;
  final Map<int, TextPainter> _ink = {}, _muted = {}, _halo = {};

  TextPainter _label(int i, TextStyle style) => TextPainter(
    text: TextSpan(text: labels[i], style: style),
    textDirection: directionOf(labels[i]),
    maxLines: 1,
    ellipsis: '…',
  )..layout(maxWidth: 160);

  @override
  void paint(Canvas canvas, Size size) {
    final layout = state._layout;
    if (layout == null || layout.count != labels.length) return;
    final scale = state._scale;
    final active = state._selected ?? state._hover;
    final near = <int>{
      if (active != null && active < layout.count) ...[
        active,
        ...state._adjacent[active],
      ],
    };
    bool lit(int i) => near.isNotEmpty
        ? near.contains(i)
        : (!searching || matches.contains(i));
    final view = Offset.zero & size;

    canvas.save();
    canvas.translate(
      size.width / 2 + state._pan.dx,
      size.height / 2 + state._pan.dy,
    );
    canvas.scale(scale);

    final faint = Path(), bright = Path(), hot = Path();
    for (final (a, b) in state._edges) {
      final pa = layout.position(a), pb = layout.position(b);
      final path = active != null && (a == active || b == active)
          ? hot
          : (lit(a) && lit(b) ? bright : faint);
      path
        ..moveTo(pa.dx, pa.dy)
        ..lineTo(pb.dx, pb.dy);
    }
    final dimming = near.isNotEmpty || searching;
    final edge = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = Stroke.hairline / scale;
    canvas.drawPath(
      faint,
      edge..color = palette.inkFaint.withValues(alpha: dimming ? 0.15 : 0.45),
    );
    canvas.drawPath(bright, edge..color = palette.inkFaint);
    canvas.drawPath(
      hot,
      edge
        ..color = palette.accent
        ..strokeWidth = Stroke.focus / scale,
    );

    final fill = Paint();
    final ring = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = Stroke.focus / scale
      ..color = palette.accent;
    for (var i = 0; i < layout.count; i++) {
      final o = layout.position(i);
      final r = state._radius(i);
      fill.color = lit(i) ? color[i] : color[i].withValues(alpha: 0.18);
      canvas.drawCircle(o, r, fill);
      if (i == active || (searching && matches.contains(i))) {
        canvas.drawCircle(o, r + 3 / scale, ring);
      }
    }
    canvas.restore();

    // Labels: all of them once zoomed in, the well-linked ones a little earlier.
    final fade = ((scale - 0.9) / 0.5).clamp(0.0, 1.0);
    for (var i = 0; i < layout.count; i++) {
      final always = near.contains(i) || (searching && matches.contains(i));
      final hub = ((scale * (1 + layout.degree(i) / 6) - 0.9) / 0.5).clamp(
        0.0,
        1.0,
      );
      final alpha = always ? 1.0 : (dimming ? 0.0 : math.max(fade, hub));
      if (alpha <= 0.02) continue;
      final at = state._toScreen(layout.position(i));
      if (!view.inflate(80).contains(at)) continue;
      final strong = near.contains(i);
      final tp = (strong ? _ink : _muted)[i] ??= _label(
        i,
        labelStyle.copyWith(color: strong ? palette.ink : palette.inkMuted),
      );
      // A halo of paper keeps labels readable over edges.
      final halo = _halo[i] ??= _label(
        i,
        labelStyle.copyWith(
          foreground: Paint()
            ..style = PaintingStyle.stroke
            ..strokeWidth = 3
            ..color = palette.paper,
        ),
      );
      final o = Offset(
        at.dx - tp.width / 2,
        at.dy + state._radius(i) * scale + 3,
      );
      final fading = alpha < 1;
      if (fading) {
        canvas.saveLayer(
          (o & tp.size).inflate(3),
          Paint()..color = Color.fromRGBO(0, 0, 0, alpha),
        );
      }
      halo.paint(canvas, o);
      tp.paint(canvas, o);
      if (fading) canvas.restore();
    }
  }

  @override
  bool shouldRepaint(_GraphPainter old) => true;
}
