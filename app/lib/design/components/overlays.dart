import 'package:flutter/material.dart' show showModalBottomSheet;
import 'package:flutter/widgets.dart';

import '../theme.dart';
import '../tokens.dart';

/// Paper sheet rising from the bottom, with a hairline top edge and a grab handle.
Future<T?> showDSheet<T>(
  BuildContext context, {
  required WidgetBuilder builder,
}) {
  final p = context.palette;
  return showModalBottomSheet<T>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: p.raised,
    barrierColor: p.ink.withValues(alpha: 0.18),
    elevation: 0,
    shape: RoundedRectangleBorder(
      borderRadius: const BorderRadius.vertical(
        top: Radius.circular(Radii.large),
      ),
      side: BorderSide(color: p.hairline),
    ),
    builder: (context) => Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: Space.x2),
          Container(
            width: 36,
            height: 4,
            decoration: BoxDecoration(
              color: p.hairline,
              borderRadius: BorderRadius.circular(Radii.pill),
            ),
          ),
          Flexible(child: builder(context)),
        ],
      ),
    ),
  );
}

/// A short, quiet confirmation line at the bottom of the screen.
void showNote(BuildContext context, String text) {
  final overlay = Overlay.maybeOf(context);
  if (overlay == null) return;
  late OverlayEntry entry;
  entry = OverlayEntry(
    builder: (context) => _Note(text: text, onDone: () => entry.remove()),
  );
  overlay.insert(entry);
}

class _Note extends StatefulWidget {
  const _Note({required this.text, required this.onDone});
  final String text;
  final VoidCallback onDone;

  @override
  State<_Note> createState() => _NoteState();
}

class _NoteState extends State<_Note> with SingleTickerProviderStateMixin {
  late final _c = AnimationController(vsync: this, duration: Motion.standard);

  @override
  void initState() {
    super.initState();
    _c.forward();
    Future<void>.delayed(const Duration(milliseconds: 1800), () async {
      if (!mounted) return;
      await _c.reverse();
      widget.onDone();
    });
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Positioned(
      left: Space.x4,
      right: Space.x4,
      bottom: MediaQuery.paddingOf(context).bottom + 120,
      child: IgnorePointer(
        child: FadeTransition(
          opacity: CurvedAnimation(parent: _c, curve: Motion.ease),
          child: Center(
            child: Semantics(
              liveRegion: true,
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.x4,
                  vertical: Space.x2 + 2,
                ),
                decoration: BoxDecoration(
                  color: p.ink,
                  borderRadius: BorderRadius.circular(Radii.pill),
                ),
                child: Text(
                  widget.text,
                  style: context.type.label.copyWith(color: p.paper),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
