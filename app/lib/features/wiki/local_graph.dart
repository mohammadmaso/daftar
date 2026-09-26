import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/library_api.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import 'wiki_state.dart';

/// A small local graph (§8.3): the page in the middle, depth-1 neighbours on an inner ring,
/// depth-2 on an outer ring. Minimal and animated; tapping a node opens it.
class LocalGraphView extends ConsumerStatefulWidget {
  const LocalGraphView({
    super.key,
    required this.path,
    required this.onOpen,
    this.onOpenGraph,
  });
  final String path;
  final ValueChanged<String> onOpen;

  /// Opens the whole-wiki graph centred on this page.
  final VoidCallback? onOpenGraph;

  @override
  ConsumerState<LocalGraphView> createState() => _LocalGraphViewState();
}

class _LocalGraphViewState extends ConsumerState<LocalGraphView> {
  int _depth = 1;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final graph = ref.watch(graphProvider((widget.path, _depth)));
    return Padding(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(child: Text(l.localGraph, style: context.type.title)),
              SizedBox(
                width: 120,
                child: DSegmented<int>(
                  value: _depth,
                  onChanged: (d) => setState(() => _depth = d),
                  segments: const [DSegment(1, '1'), DSegment(2, '2')],
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.x4),
          SizedBox(
            height: 320,
            child: graph.when(
              loading: () => const SizedBox.shrink(),
              error: (e, _) => Center(
                child: Text(
                  '$e',
                  style: context.type.small.copyWith(color: p.critical),
                ),
              ),
              data: (g) => _Graph(
                graph: g,
                label: (n) => pageTitle(
                  lang,
                  n.page.titleEn,
                  n.page.titleFa,
                  n.page.path,
                ),
                onOpen: widget.onOpen,
              ),
            ),
          ),
          if (widget.onOpenGraph != null) ...[
            const SizedBox(height: Space.x3),
            DButton(
              label: l.openGraph,
              icon: DIcons.graph,
              variant: DButtonVariant.secondary,
              onPressed: widget.onOpenGraph,
            ),
          ],
        ],
      ),
    );
  }
}

class _Graph extends StatelessWidget {
  const _Graph({
    required this.graph,
    required this.label,
    required this.onOpen,
  });
  final LocalGraph graph;
  final String Function(GraphNode) label;
  final ValueChanged<String> onOpen;

  Map<String, Offset> _layout(Size size) {
    final c = size.center(Offset.zero);
    final r1 = math.min(size.width, size.height) * 0.28;
    final r2 = math.min(size.width, size.height) * 0.45;
    final out = <String, Offset>{};
    for (final depth in [0, 1, 2]) {
      final ring = graph.nodes.where((n) => n.depth == depth).toList();
      for (var i = 0; i < ring.length; i++) {
        if (depth == 0) {
          out[ring[i].page.path] = c;
          continue;
        }
        final a =
            -math.pi / 2 +
            2 * math.pi * i / ring.length +
            (depth == 2 ? math.pi / ring.length : 0);
        final r = depth == 1 ? r1 : r2;
        out[ring[i].page.path] = c + Offset(math.cos(a) * r, math.sin(a) * r);
      }
    }
    return out;
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return LayoutBuilder(
      builder: (context, box) {
        final size = Size(box.maxWidth, box.maxHeight);
        final pos = _layout(size);
        return TweenAnimationBuilder<double>(
          tween: Tween(begin: 0, end: 1),
          duration: motion(context, Motion.slow),
          curve: Motion.ease,
          builder: (context, t, _) => Stack(
            children: [
              Positioned.fill(
                child: CustomPaint(
                  painter: _Edges(
                    [
                      for (final e in graph.edges)
                        if (pos[e.from] != null && pos[e.to] != null)
                          (pos[e.from]!, pos[e.to]!),
                    ],
                    p.hairline,
                    t,
                    size.center(Offset.zero),
                  ),
                ),
              ),
              for (final n in graph.nodes)
                if (pos[n.page.path] case final o?)
                  Positioned(
                    left:
                        size.center(Offset.zero).dx +
                        (o.dx - size.center(Offset.zero).dx) * t -
                        60,
                    top:
                        size.center(Offset.zero).dy +
                        (o.dy - size.center(Offset.zero).dy) * t -
                        14,
                    width: 120,
                    child: Opacity(
                      opacity: t,
                      child: Pressable(
                        onPressed: n.depth == 0
                            ? null
                            : () => onOpen(n.page.path),
                        semanticLabel: label(n),
                        radius: Radii.pill,
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: Space.x2,
                            vertical: Space.x1,
                          ),
                          decoration: BoxDecoration(
                            color: n.depth == 0 ? p.accent : p.raised,
                            borderRadius: BorderRadius.circular(Radii.pill),
                            border: Border.all(
                              color: n.depth == 0 ? p.accent : p.hairline,
                            ),
                          ),
                          child: Text(
                            label(n),
                            textAlign: TextAlign.center,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.type.caption.copyWith(
                              color: n.depth == 0
                                  ? p.onAccent
                                  : (n.depth == 1 ? p.ink : p.inkMuted),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
            ],
          ),
        );
      },
    );
  }
}

class _Edges extends CustomPainter {
  _Edges(this.lines, this.color, this.t, this.center);
  final List<(Offset, Offset)> lines;
  final Color color;
  final double t;
  final Offset center;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = Stroke.hairline * 1.5;
    Offset at(Offset o) => center + (o - center) * t;
    for (final (a, b) in lines) {
      canvas.drawLine(at(a), at(b), paint);
    }
  }

  @override
  bool shouldRepaint(_Edges old) =>
      old.t != t || old.lines != lines || old.color != color;
}
