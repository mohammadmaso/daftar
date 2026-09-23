import 'package:flutter/widgets.dart';

import '../icons.dart';
import '../theme.dart';
import '../tokens.dart';
import 'pressable.dart';

enum DButtonVariant { primary, secondary, quiet }

class DButton extends StatelessWidget {
  const DButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.variant = DButtonVariant.primary,
    this.icon,
  });

  final String label;
  final VoidCallback? onPressed;
  final DButtonVariant variant;
  final DIcons? icon;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final (bg, fg, border) = switch (variant) {
      DButtonVariant.primary => (p.accent, p.onAccent, p.accent),
      DButtonVariant.secondary => (p.raised, p.ink, p.hairline),
      DButtonVariant.quiet => (null, p.accent, null),
    };
    return Pressable(
      onPressed: onPressed,
      hoverColor: variant == DButtonVariant.primary ? null : p.sunken,
      child: Container(
        padding: const EdgeInsets.symmetric(
          horizontal: Space.x4,
          vertical: Space.x3 - 1,
        ),
        decoration: BoxDecoration(
          color: bg,
          borderRadius: BorderRadius.circular(Radii.medium),
          border: border == null
              ? null
              : Border.all(color: border, width: Stroke.hairline),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            if (icon != null) ...[
              DIcon(icon!, size: 18, color: fg),
              const SizedBox(width: Space.x2),
            ],
            Flexible(
              child: Text(
                label,
                style: context.type.label.copyWith(color: fg),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class DIconButton extends StatelessWidget {
  const DIconButton({
    super.key,
    required this.icon,
    required this.onPressed,
    required this.semanticLabel,
    this.color,
  });

  final DIcons icon;
  final VoidCallback? onPressed;
  final String semanticLabel;
  final Color? color;

  @override
  Widget build(BuildContext context) => Pressable(
    onPressed: onPressed,
    semanticLabel: semanticLabel,
    child: Center(
      widthFactor: 1,
      heightFactor: 1,
      child: Padding(
        padding: const EdgeInsets.all(Space.x2 + 2),
        child: DIcon(icon, size: 22, color: color ?? context.palette.ink),
      ),
    ),
  );
}
