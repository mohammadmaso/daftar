import 'package:flutter/material.dart'
    show InputBorder, InputDecoration, Material, TextField;
import 'package:flutter/widgets.dart';

import '../icons.dart';
import '../theme.dart';
import '../tokens.dart';
import 'buttons.dart';
import 'pressable.dart';

class DHairline extends StatelessWidget {
  const DHairline({super.key, this.indent = 0});
  final double indent;

  @override
  Widget build(BuildContext context) => Padding(
    padding: EdgeInsetsDirectional.only(start: indent),
    child: SizedBox(
      height: Stroke.hairline,
      child: ColoredBox(color: context.palette.hairline),
    ),
  );
}

class DSurface extends StatelessWidget {
  const DSurface({super.key, required this.child, this.padding});
  final Widget child;
  final EdgeInsetsGeometry? padding;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      padding: padding,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: p.raised,
        borderRadius: BorderRadius.circular(Radii.large),
        border: Border.all(color: p.hairline, width: Stroke.hairline),
      ),
      child: child,
    );
  }
}

/// Titled group of rows separated by hairlines.
class DSection extends StatelessWidget {
  const DSection({super.key, this.title, required this.children, this.footer});
  final String? title;
  final List<Widget> children;
  final String? footer;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (title != null)
          Padding(
            padding: const EdgeInsetsDirectional.only(
              start: Space.x1,
              bottom: Space.x2,
            ),
            child: Semantics(
              header: true,
              child: Text(
                title!,
                style: context.type.caption.copyWith(color: p.inkMuted),
              ),
            ),
          ),
        DSurface(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (var i = 0; i < children.length; i++) ...[
                if (i > 0) const DHairline(indent: Space.x4),
                children[i],
              ],
            ],
          ),
        ),
        if (footer != null)
          Padding(
            padding: const EdgeInsetsDirectional.only(
              start: Space.x1,
              top: Space.x2,
            ),
            child: Text(
              footer!,
              style: context.type.small.copyWith(color: p.inkMuted),
            ),
          ),
      ],
    );
  }
}

class DListRow extends StatelessWidget {
  const DListRow({
    super.key,
    required this.title,
    this.subtitle,
    this.leading,
    this.trailing,
    this.onTap,
    this.chevron = false,
  });

  final String title;
  final String? subtitle;
  final Widget? leading;
  final Widget? trailing;
  final VoidCallback? onTap;
  final bool chevron;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final row = Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.x4,
        vertical: Space.x3,
      ),
      child: Row(
        children: [
          if (leading != null) ...[leading!, const SizedBox(width: Space.x3)],
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(title, style: context.type.body.copyWith(color: p.ink)),
                if (subtitle != null)
                  Text(
                    subtitle!,
                    style: context.type.small.copyWith(color: p.inkMuted),
                  ),
              ],
            ),
          ),
          if (trailing != null) ...[const SizedBox(width: Space.x3), trailing!],
          if (chevron)
            DIcon(DIcons.chevronForward, size: 18, color: p.inkMuted),
        ],
      ),
    );
    if (onTap == null) return MergeSemantics(child: row);
    return Pressable(onPressed: onTap, radius: 0, child: row);
  }
}

class DTextField extends StatelessWidget {
  const DTextField({
    super.key,
    this.controller,
    this.hint,
    this.onChanged,
    this.leading,
    this.autofocus = false,
    this.obscure = false,
    this.forceLtr = false,
    this.mono = false,
    this.maxLines = 1,
    this.minLines,
    this.keyboardType,
    this.onSubmitted,
  });

  final TextEditingController? controller;
  final String? hint;
  final ValueChanged<String>? onChanged;
  final DIcons? leading;
  final bool autofocus;
  final bool obscure;

  /// For URLs, tokens and other technical input that must stay LTR inside Persian UI.
  final bool forceLtr;
  final bool mono;
  final int? maxLines;
  final int? minLines;
  final TextInputType? keyboardType;
  final ValueChanged<String>? onSubmitted;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      constraints: const BoxConstraints(minHeight: kMinTarget),
      padding: const EdgeInsets.symmetric(horizontal: Space.x3),
      decoration: BoxDecoration(
        color: p.sunken,
        borderRadius: BorderRadius.circular(Radii.medium),
        border: Border.all(color: p.hairline, width: Stroke.hairline),
      ),
      child: Row(
        children: [
          if (leading != null) ...[
            DIcon(leading!, size: 18, color: p.inkMuted),
            const SizedBox(width: Space.x2),
          ],
          Expanded(
            // The padding lives inside the field so the whole box takes the tap (≥ 44 pt).
            child: TextField(
              controller: controller,
              onChanged: onChanged,
              onSubmitted: onSubmitted,
              autofocus: autofocus,
              obscureText: obscure,
              autocorrect: !forceLtr && !obscure,
              enableSuggestions: !forceLtr && !obscure,
              keyboardType: keyboardType,
              maxLines: obscure ? 1 : maxLines,
              minLines: minLines,
              textDirection: forceLtr ? TextDirection.ltr : null,
              style: (mono ? TypeScale.mono : context.type.body).copyWith(
                color: p.ink,
              ),
              decoration: InputDecoration(
                isCollapsed: true,
                border: InputBorder.none,
                contentPadding: const EdgeInsets.symmetric(
                  vertical: Space.x3 - 2,
                ),
                hintText: hint,
                hintStyle: (mono ? TypeScale.mono : context.type.body).copyWith(
                  color: p.inkMuted,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Page scaffold: paper background, a quiet header, content constrained to a readable measure.
class DPage extends StatelessWidget {
  const DPage({
    super.key,
    required this.title,
    required this.children,
    this.onBack,
    this.backLabel,
    this.trailing,
  });

  final String title;
  final List<Widget> children;
  final VoidCallback? onBack;
  final String? backLabel;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Material(
      color: p.paper,
      child: SafeArea(
        child: CustomScrollView(
          slivers: [
            SliverToBoxAdapter(
              child: _Measure(
                child: Padding(
                  padding: const EdgeInsets.only(
                    top: Space.x4,
                    bottom: Space.x6,
                  ),
                  child: Row(
                    children: [
                      if (onBack != null) ...[
                        DIconButton(
                          icon: DIcons.back,
                          onPressed: onBack,
                          semanticLabel: backLabel ?? '',
                        ),
                        const SizedBox(width: Space.x1),
                      ],
                      Expanded(
                        child: Semantics(
                          header: true,
                          child: Text(
                            title,
                            style: context.type.display.copyWith(color: p.ink),
                          ),
                        ),
                      ),
                      ?trailing,
                    ],
                  ),
                ),
              ),
            ),
            SliverList.separated(
              itemCount: children.length,
              separatorBuilder: (_, _) => const SizedBox(height: Space.x8),
              itemBuilder: (_, i) => _Measure(child: children[i]),
            ),
            const SliverToBoxAdapter(child: SizedBox(height: Space.x12)),
          ],
        ),
      ),
    );
  }
}

class _Measure extends StatelessWidget {
  const _Measure({required this.child});
  final Widget child;

  @override
  Widget build(BuildContext context) => Align(
    alignment: Alignment.topCenter,
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: Space.measure),
      // Fill the measure so narrow children line up with the start edge, not the centre.
      child: SizedBox(
        width: double.infinity,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: Space.gutter),
          child: Align(alignment: AlignmentDirectional.topStart, child: child),
        ),
      ),
    ),
  );
}
