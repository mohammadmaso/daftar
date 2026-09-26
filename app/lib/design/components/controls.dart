import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../icons.dart';
import '../theme.dart';
import '../tokens.dart';
import 'pressable.dart';

class DChip extends StatelessWidget {
  const DChip({
    super.key,
    required this.label,
    required this.selected,
    required this.onTap,
    this.dot,
  });

  final String label;
  final bool selected;
  final VoidCallback? onTap;

  /// A small colour swatch before the label, for chips that double as a legend.
  final Color? dot;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Pressable(
      onPressed: onTap,
      selected: selected,
      haptic: true,
      radius: Radii.pill,
      minSize: const Size(kMinTarget, kMinTarget),
      child: Center(
        widthFactor: 1,
        child: AnimatedContainer(
          duration: motion(context, Motion.quick),
          curve: Motion.ease,
          padding: const EdgeInsets.symmetric(
            horizontal: Space.x3 + 2,
            vertical: Space.x1 + 2,
          ),
          decoration: BoxDecoration(
            color: selected ? p.accentSoft : null,
            borderRadius: BorderRadius.circular(Radii.pill),
            border: Border.all(
              color: selected ? p.accent : p.hairline,
              width: Stroke.hairline,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (dot != null) ...[
                Container(
                  width: 8,
                  height: 8,
                  decoration: BoxDecoration(color: dot, shape: BoxShape.circle),
                ),
                const SizedBox(width: Space.x2),
              ],
              Text(
                label,
                style: context.type.label.copyWith(
                  color: selected ? p.accent : p.inkMuted,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class DSegment<T> {
  const DSegment(this.value, this.label, {this.icon});
  final T value;
  final String label;
  final DIcons? icon;
}

/// Mutually exclusive choice with a sliding ink indicator.
class DSegmented<T> extends StatelessWidget {
  const DSegmented({
    super.key,
    required this.segments,
    required this.value,
    required this.onChanged,
  });

  final List<DSegment<T>> segments;
  final T value;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final index = segments.indexWhere((s) => s.value == value);
    return Container(
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: p.sunken,
        borderRadius: BorderRadius.circular(Radii.medium),
      ),
      child: LayoutBuilder(
        builder: (context, c) {
          final w = c.maxWidth / segments.length;
          return Stack(
            children: [
              AnimatedPositionedDirectional(
                duration: motion(context, Motion.standard),
                curve: Motion.ease,
                start: w * index,
                top: 0,
                bottom: 0,
                width: w,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: p.raised,
                    borderRadius: BorderRadius.circular(Radii.small),
                    border: Border.all(
                      color: p.hairline,
                      width: Stroke.hairline,
                    ),
                  ),
                ),
              ),
              Row(
                children: [
                  for (final s in segments)
                    Expanded(
                      child: Pressable(
                        onPressed: () {
                          if (s.value != value) {
                            HapticFeedback.selectionClick();
                            onChanged(s.value);
                          }
                        },
                        selected: s.value == value,
                        hoverColor: const Color(0x00000000),
                        radius: Radii.small,
                        child: Center(
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              if (s.icon != null) ...[
                                DIcon(
                                  s.icon!,
                                  size: 16,
                                  color: s.value == value ? p.ink : p.inkMuted,
                                ),
                                const SizedBox(width: Space.x1 + 2),
                              ],
                              Flexible(
                                child: Text(
                                  s.label,
                                  overflow: TextOverflow.ellipsis,
                                  style: context.type.label.copyWith(
                                    color: s.value == value
                                        ? p.ink
                                        : p.inkMuted,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ],
          );
        },
      ),
    );
  }
}

class DSwitch extends StatelessWidget {
  const DSwitch({
    super.key,
    required this.value,
    required this.onChanged,
    this.semanticLabel,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;
  final String? semanticLabel;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final d = motion(context, Motion.standard);
    return Pressable(
      onPressed: onChanged == null
          ? null
          : () {
              HapticFeedback.selectionClick();
              onChanged!(!value);
            },
      toggled: value,
      semanticLabel: semanticLabel,
      hoverColor: const Color(0x00000000),
      radius: Radii.pill,
      child: Center(
        widthFactor: 1,
        child: AnimatedContainer(
          duration: d,
          curve: Motion.ease,
          width: 42,
          height: 26,
          padding: const EdgeInsets.all(3),
          decoration: BoxDecoration(
            color: value ? p.accent : p.sunken,
            borderRadius: BorderRadius.circular(Radii.pill),
            border: Border.all(
              color: value ? p.accent : p.hairline,
              width: Stroke.hairline,
            ),
          ),
          child: AnimatedAlign(
            duration: d,
            curve: Motion.ease,
            alignment: value
                ? AlignmentDirectional.centerEnd
                : AlignmentDirectional.centerStart,
            child: Container(
              width: 18,
              height: 18,
              decoration: BoxDecoration(
                color: value ? p.onAccent : p.raised,
                shape: BoxShape.circle,
                border: value
                    ? null
                    : Border.all(color: p.hairline, width: Stroke.hairline),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

enum ClaimStatus { confirmed, proposed, superseded }

class DStatusPill extends StatelessWidget {
  const DStatusPill({super.key, required this.status, required this.label});

  final ClaimStatus status;
  final String label;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final c = switch (status) {
      ClaimStatus.confirmed => p.positive,
      ClaimStatus.proposed => p.pending,
      ClaimStatus.superseded => p.inkMuted,
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: Space.x2, vertical: 2),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(Radii.pill),
        border: Border.all(color: c.withValues(alpha: 0.5), width: 1),
      ),
      child: Text(
        label,
        style: context.type.caption.copyWith(
          color: c,
          decoration: status == ClaimStatus.superseded
              ? TextDecoration.lineThrough
              : null,
          decorationColor: c,
        ),
      ),
    );
  }
}

class DFace<T> {
  const DFace(this.value, this.label, this.family, {this.scale = 1});
  final T value;
  final String label;

  /// Font family the sample is set in.
  final String family;

  /// Evens out faces drawn small or large on their body.
  final double scale;
}

/// Picks a typeface: a row of specimen tiles, each showing [sample] in its own face.
class DFaceChoice<T> extends StatelessWidget {
  const DFaceChoice({
    super.key,
    required this.faces,
    required this.value,
    required this.onChanged,
    required this.sample,
  });

  final List<DFace<T>> faces;
  final T value;
  final ValueChanged<T> onChanged;
  final String sample;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (final (i, f) in faces.indexed) ...[
          if (i > 0) const SizedBox(width: Space.x2),
          Expanded(
            child: Pressable(
              onPressed: () {
                if (f.value != value) {
                  HapticFeedback.selectionClick();
                  onChanged(f.value);
                }
              },
              selected: f.value == value,
              semanticLabel: f.label,
              radius: Radii.medium,
              child: AnimatedContainer(
                duration: motion(context, Motion.quick),
                curve: Motion.ease,
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.x1,
                  vertical: Space.x3,
                ),
                decoration: BoxDecoration(
                  color: f.value == value ? p.accentSoft : p.raised,
                  borderRadius: BorderRadius.circular(Radii.medium),
                  border: Border.all(
                    color: f.value == value ? p.accent : p.hairline,
                    width: f.value == value ? 1.5 : Stroke.hairline,
                  ),
                ),
                child: ExcludeSemantics(
                  child: Column(
                    children: [
                      SizedBox(
                        height: 40,
                        child: FittedBox(
                          fit: BoxFit.scaleDown,
                          child: Text(
                            sample,
                            textScaler: TextScaler.noScaling,
                            style: TextStyle(
                              fontFamily: f.family,
                              fontSize: 28 * f.scale,
                              height: 1.3,
                              color: f.value == value ? p.accent : p.ink,
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(height: Space.x1),
                      Text(
                        f.label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: context.type.caption.copyWith(
                          color: f.value == value ? p.ink : p.inkMuted,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ],
    );
  }
}
