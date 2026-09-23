import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../theme.dart';
import '../tokens.dart';

/// Respects the platform "reduce motion" setting.
Duration motion(BuildContext context, Duration d) =>
    MediaQuery.maybeDisableAnimationsOf(context) ?? false ? Duration.zero : d;

/// Shared interaction base for every tappable component: keyboard activation, focus ring,
/// hover tint, press feedback, semantics and a ≥44 pt hit target.
class Pressable extends StatefulWidget {
  const Pressable({
    super.key,
    required this.child,
    required this.onPressed,
    this.semanticLabel,
    this.selected,
    this.toggled,
    this.radius = Radii.medium,
    this.haptic = false,
    this.hoverColor,
    this.minSize = const Size(kMinTarget, kMinTarget),
  });

  final Widget child;
  final VoidCallback? onPressed;
  final String? semanticLabel;
  final bool? selected;
  final bool? toggled;
  final double radius;
  final bool haptic;
  final Color? hoverColor;
  final Size minSize;

  @override
  State<Pressable> createState() => _PressableState();
}

class _PressableState extends State<Pressable> {
  bool _hover = false;
  bool _focus = false;
  bool _down = false;

  bool get _enabled => widget.onPressed != null;

  void _activate() {
    if (!_enabled) return;
    if (widget.haptic) HapticFeedback.selectionClick();
    widget.onPressed!();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final d = motion(context, Motion.quick);
    return Semantics(
      button: true,
      enabled: _enabled,
      selected: widget.selected,
      toggled: widget.toggled,
      label: widget.semanticLabel,
      child: FocusableActionDetector(
        enabled: _enabled,
        mouseCursor: _enabled
            ? SystemMouseCursors.click
            : SystemMouseCursors.basic,
        onShowHoverHighlight: (v) => setState(() => _hover = v),
        onShowFocusHighlight: (v) => setState(() => _focus = v),
        actions: {
          ActivateIntent: CallbackAction<ActivateIntent>(
            onInvoke: (_) => _activate(),
          ),
        },
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: _enabled ? (_) => setState(() => _down = true) : null,
          onTapCancel: () => setState(() => _down = false),
          onTapUp: _enabled ? (_) => setState(() => _down = false) : null,
          onTap: _enabled ? _activate : null,
          child: ConstrainedBox(
            constraints: BoxConstraints(
              minWidth: widget.minSize.width,
              minHeight: widget.minSize.height,
            ),
            child: AnimatedOpacity(
              duration: d,
              curve: Motion.ease,
              opacity: !_enabled ? 0.45 : (_down ? 0.72 : 1),
              child: AnimatedContainer(
                duration: d,
                curve: Motion.ease,
                decoration: BoxDecoration(
                  color: _hover ? (widget.hoverColor ?? p.sunken) : null,
                  borderRadius: BorderRadius.circular(widget.radius),
                  border: Border.all(
                    color: _focus ? p.accent : const Color(0x00000000),
                    width: Stroke.focus,
                  ),
                ),
                child: widget.child,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
