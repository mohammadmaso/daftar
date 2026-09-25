import 'package:flutter/material.dart' show showModalBottomSheet;
import 'package:flutter/widgets.dart';

import '../theme.dart';
import '../tokens.dart';
import 'buttons.dart';
import 'layout.dart';

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

/// Asks for a line or two of text in a sheet; returns it trimmed, or `null` when dismissed.
/// The sheet owns its controller, so nothing is disposed while it is still animating away.
Future<String?> showTextPrompt(
  BuildContext context, {
  required String title,
  required String action,
  String? hint,
  String initial = '',
}) => showDSheet<String>(
  context,
  builder: (_) =>
      _TextPrompt(title: title, action: action, hint: hint, initial: initial),
);

class _TextPrompt extends StatefulWidget {
  const _TextPrompt({
    required this.title,
    required this.action,
    this.hint,
    this.initial = '',
  });
  final String title;
  final String action;
  final String? hint;
  final String initial;

  @override
  State<_TextPrompt> createState() => _TextPromptState();
}

class _TextPromptState extends State<_TextPrompt> {
  late final _c = TextEditingController(text: widget.initial);

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(Space.x4),
    child: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(widget.title, style: context.type.title),
        const SizedBox(height: Space.x4),
        DTextField(
          controller: _c,
          hint: widget.hint,
          autofocus: true,
          maxLines: 4,
          minLines: 2,
        ),
        const SizedBox(height: Space.x4),
        ListenableBuilder(
          listenable: _c,
          builder: (context, _) => DButton(
            label: widget.action,
            onPressed: _c.text.trim().isEmpty
                ? null
                : () => Navigator.of(context).pop(_c.text.trim()),
          ),
        ),
      ],
    ),
  );
}
