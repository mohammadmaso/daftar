import 'dart:io';

import 'package:flutter/widgets.dart';

import '../../core/bidi.dart';
import '../../core/dates.dart';
import '../../core/library_api.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// One capture in the Today timeline: time, content, and where it is in its life.
class CaptureItem extends StatelessWidget {
  const CaptureItem({super.key, required this.capture, this.onTap});

  final Capture capture;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final at = DateTime.tryParse(capture.capturedAt)?.toLocal();
    final text = switch ((capture.kind, capture.text.isEmpty)) {
      (RawKind.voice, true) => l.voicePending,
      (RawKind.photo, true) => l.photoPending,
      _ => capture.text,
    };
    final placeholder = capture.text.isEmpty;

    final content = Padding(
      padding: const EdgeInsets.symmetric(
        vertical: Space.x3,
        horizontal: Space.x1,
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 52,
            child: Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Text(
                at == null ? '' : clockTime(at, lang),
                style: context.type.caption.copyWith(color: p.inkMuted),
              ),
            ),
          ),
          Padding(
            padding: const EdgeInsets.only(top: 1),
            child: DIcon(_icon(capture.kind), size: 18, color: p.inkMuted),
          ),
          const SizedBox(width: Space.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                if (capture.images.isNotEmpty) ...[
                  ClipRRect(
                    borderRadius: BorderRadius.circular(Radii.small),
                    child: AspectRatio(
                      aspectRatio: 4 / 3,
                      child: Image.file(
                        File(capture.images.first),
                        fit: BoxFit.cover,
                        cacheWidth: 800,
                        errorBuilder: (_, _, _) => ColoredBox(color: p.sunken),
                      ),
                    ),
                  ),
                  const SizedBox(height: Space.x2),
                ],
                Text(
                  text,
                  maxLines: 6,
                  overflow: TextOverflow.ellipsis,
                  textDirection: placeholder ? null : directionOf(text),
                  style: (placeholder ? context.type.small : context.type.body)
                      .copyWith(color: placeholder ? p.inkMuted : p.ink),
                ),
                const SizedBox(height: Space.x1),
                _StageLine(capture: capture),
              ],
            ),
          ),
        ],
      ),
    );
    if (onTap == null) return MergeSemantics(child: content);
    return Pressable(onPressed: onTap, radius: Radii.medium, child: content);
  }

  static DIcons _icon(RawKind k) => switch (k) {
    RawKind.voice || RawKind.voiceConversation => DIcons.mic,
    RawKind.photo => DIcons.camera,
    RawKind.chatAnswer => DIcons.ask,
    _ => DIcons.today,
  };
}

class _StageLine extends StatelessWidget {
  const _StageLine({required this.capture});
  final Capture capture;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final (label, color) = switch (capture.stage) {
      Stage.saved => (l.stageSaved, p.inkMuted),
      Stage.working => (l.stageWorking, p.accent),
      Stage.failed => (l.stageFailed, p.critical),
      Stage.filed => (l.stageFiled, p.positive),
      Stage.excluded => (l.stageExcluded, p.inkMuted),
    };
    return Row(
      children: [
        Container(
          width: 6,
          height: 6,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
        const SizedBox(width: Space.x2),
        Flexible(
          child: Text(
            [label, if (capture.problem != null) capture.problem!].join(' · '),
            style: context.type.caption.copyWith(color: color),
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }
}
