import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/bidi.dart';
import '../../core/errors.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../activity/activity_screen.dart' show vaultNames;
import '../wiki/markdown_view.dart';
import '../wiki/wiki_state.dart';

final reviewCardsProvider = FutureProvider<List<ReviewCardDto>>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib?.reviewCards() ?? const [];
});

/// The Review stack (§8.5): one card at a time; swipe right to confirm or keep, left to reject or
/// dismiss. Buttons do the same for keyboards and screen readers.
class ReviewScreen extends ConsumerStatefulWidget {
  const ReviewScreen({super.key, this.embedded = false});
  final bool embedded;

  @override
  ConsumerState<ReviewScreen> createState() => _ReviewScreenState();
}

class _ReviewScreenState extends ConsumerState<ReviewScreen> {
  double _dx = 0;
  bool _busy = false;
  final _done = <String>{};

  Future<void> _resolve(
    ReviewCardDto c,
    ReviewAction a, {
    String? edited,
  }) async {
    if (_busy) return;
    setState(() => _busy = true);
    HapticFeedback.selectionClick();
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      await lib.resolveReview(c.id, a, editedText: edited);
      _done.add(c.id);
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
    } catch (e) {
      if (mounted) showNote(context, humanError(e));
    }
    if (mounted) {
      setState(() {
        _busy = false;
        _dx = 0;
      });
    }
  }

  Future<void> _move(ReviewCardDto c, String vault) async {
    if (c.opId == null) return;
    setState(() => _busy = true);
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      await lib.moveToVault(c.opId!, vault);
      await lib.resolveReview(c.id, ReviewAction.dismiss);
      _done.add(c.id);
      ref.read(revisionProvider.notifier).bump();
      await ref.read(jobRunnerProvider.notifier).kick();
    } catch (e) {
      if (mounted) showNote(context, humanError(e));
    }
    if (mounted) setState(() => _busy = false);
  }

  Future<void> _edit(ReviewCardDto c) async {
    final l = L10n.of(context);
    final text = await showTextPrompt(
      context,
      title: l.editPage,
      action: l.confirm,
      initial: c.claim?.text ?? '',
    );
    if (text != null) await _resolve(c, ReviewAction.confirm, edited: text);
  }

  /// What a swipe does for this card: (right, left).
  (ReviewAction, ReviewAction) _swipes(ReviewCardDto c) => switch (c.kind) {
    CardKind.claim => (ReviewAction.confirm, ReviewAction.reject),
    _ => (ReviewAction.dismiss, ReviewAction.dismiss),
  };

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final cards =
        (ref.watch(reviewCardsProvider).value ?? const <ReviewCardDto>[])
            .where((c) => !_done.contains(c.id))
            .toList();
    final width = MediaQuery.sizeOf(context).width;
    return DPage(
      title: l.reviewTitle,
      onBack: widget.embedded || !context.canPop() ? null : () => context.pop(),
      backLabel: l.back,
      children: [
        if (cards.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: Space.x12),
            child: Column(
              children: [
                DIcon(DIcons.check, size: 40, color: p.positive),
                const SizedBox(height: Space.x3),
                Text(
                  l.reviewEmpty,
                  style: context.type.body.copyWith(color: p.inkMuted),
                ),
              ],
            ),
          )
        else ...[
          Text(
            l.toReview(cards.length),
            style: context.type.caption.copyWith(color: p.inkMuted),
          ),
          Builder(
            builder: (context) {
              final c = cards.first;
              final (right, left) = _swipes(c);
              final rtl = Directionality.of(context) == TextDirection.rtl;
              // "Right" means forward in the reading direction.
              final forward = rtl ? -_dx : _dx;
              return GestureDetector(
                onHorizontalDragUpdate: _busy
                    ? null
                    : (d) => setState(() => _dx += d.delta.dx),
                onHorizontalDragEnd: _busy
                    ? null
                    : (d) {
                        final threshold = width * 0.28;
                        final f = rtl ? -_dx : _dx;
                        if (f > threshold) {
                          _resolve(c, right);
                        } else if (f < -threshold) {
                          _resolve(c, left);
                        } else {
                          setState(() => _dx = 0);
                        }
                      },
                child: AnimatedContainer(
                  duration: _dx == 0
                      ? motion(context, Motion.standard)
                      : Duration.zero,
                  curve: Motion.ease,
                  transform: Matrix4.translationValues(_dx, 0, 0)
                    ..rotateZ(_dx / 2400),
                  child: _Card(
                    card: c,
                    tint: forward > 40
                        ? p.positive
                        : (forward < -40 ? p.critical : null),
                    onConfirm: () => _resolve(c, ReviewAction.confirm),
                    onReject: () => _resolve(c, ReviewAction.reject),
                    onEdit: () => _edit(c),
                    onDismiss: () => _resolve(c, ReviewAction.dismiss),
                    onMove: (v) => _move(c, v),
                  ),
                ),
              );
            },
          ),
          if (cards.first.kind == CardKind.claim)
            Text(
              l.swipeHint,
              textAlign: TextAlign.center,
              style: context.type.caption.copyWith(color: p.inkFaint),
            ),
        ],
      ],
    );
  }
}

class _Card extends ConsumerWidget {
  const _Card({
    required this.card,
    required this.tint,
    required this.onConfirm,
    required this.onReject,
    required this.onEdit,
    required this.onDismiss,
    required this.onMove,
  });

  final ReviewCardDto card;
  final Color? tint;
  final VoidCallback onConfirm;
  final VoidCallback onReject;
  final VoidCallback onEdit;
  final VoidCallback onDismiss;
  final ValueChanged<String> onMove;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final vaults = ref.watch(vaultsProvider).value ?? const <Vault>[];
    Widget body(String text, {TextStyle? style}) => Text(
      text,
      textDirection: directionOf(text, fallback: Directionality.of(context)),
      style: style ?? context.type.heading,
    );
    final (title, content, actions) = switch (card.kind) {
      CardKind.claim => (
        l.claimQuestion,
        <Widget>[
          body(card.claim?.text ?? card.text ?? ''),
          if (card.claim?.confidence case final conf?)
            Text(
              '${l.fieldConfidence}: $conf',
              style: context.type.caption.copyWith(color: p.inkMuted),
            ),
          if (card.replaces != null) ...[
            const SizedBox(height: Space.x2),
            body(
              l.replacesClaim(card.replaces!.text),
              style: context.type.small.copyWith(color: p.inkMuted),
            ),
          ],
          if (card.page case final page?)
            Pressable(
              onPressed: () => context.push(pageRoute(page)),
              radius: Radii.small,
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: Space.x2),
                child: Text(
                  page,
                  textDirection: TextDirection.ltr,
                  style: context.type.caption.copyWith(color: p.accent),
                ),
              ),
            ),
        ],
        <Widget>[
          DButton(label: l.confirm, icon: DIcons.check, onPressed: onConfirm),
          DButton(
            label: l.editPage,
            variant: DButtonVariant.secondary,
            onPressed: onEdit,
          ),
          DButton(
            label: l.reject,
            variant: DButtonVariant.quiet,
            onPressed: onReject,
          ),
        ],
      ),
      CardKind.routing => (
        l.routingQuestion(vaultNames(card.vaults, vaults, lang)),
        <Widget>[
          Wrap(
            spacing: Space.x2,
            runSpacing: Space.x2,
            children: [
              for (final v in vaults.where((v) => !card.vaults.contains(v.id)))
                DChip(
                  label: lang == 'fa' ? v.titleFa : v.titleEn,
                  selected: false,
                  onTap: () => onMove(v.id),
                ),
            ],
          ),
        ],
        <Widget>[
          DButton(
            label: l.keepFiling,
            icon: DIcons.check,
            onPressed: onDismiss,
          ),
        ],
      ),
      CardKind.syncConflict => (
        l.conflictCard,
        <Widget>[
          if (card.page case final page?)
            Pressable(
              onPressed: () => context.push(pageRoute(page)),
              radius: Radii.small,
              child: Text(
                page,
                textDirection: TextDirection.ltr,
                style: context.type.small.copyWith(color: p.accent),
              ),
            ),
          const SizedBox(height: Space.x2),
          Text(
            l.conflictHint,
            style: context.type.small.copyWith(color: p.inkMuted),
          ),
        ],
        <Widget>[DButton(label: l.resolved, onPressed: onDismiss)],
      ),
      CardKind.lint => (
        l.lintCard,
        <Widget>[body(card.text ?? '', style: context.type.body)],
        <Widget>[
          DButton(
            label: l.dismiss,
            variant: DButtonVariant.secondary,
            onPressed: onDismiss,
          ),
        ],
      ),
      CardKind.schema => (
        l.schemaCard,
        <Widget>[
          MarkdownView(
            text: card.text ?? '',
            onLink: (_) {},
            selectable: false,
          ),
        ],
        <Widget>[
          DButton(
            label: l.dismiss,
            variant: DButtonVariant.secondary,
            onPressed: onDismiss,
          ),
        ],
      ),
      CardKind.question || CardKind.humanEdit => (
        l.questionCard,
        <Widget>[body(card.text ?? '', style: context.type.body)],
        <Widget>[
          DButton(
            label: l.dismiss,
            variant: DButtonVariant.secondary,
            onPressed: onDismiss,
          ),
        ],
      ),
    };
    return Container(
      padding: const EdgeInsets.all(Space.x5),
      decoration: BoxDecoration(
        color: p.raised,
        borderRadius: BorderRadius.circular(Radii.large),
        border: Border.all(
          color: tint ?? p.hairline,
          width: tint == null ? Stroke.hairline : 2,
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(title, style: context.type.caption.copyWith(color: p.inkMuted)),
          const SizedBox(height: Space.x3),
          ...content,
          const SizedBox(height: Space.x5),
          Wrap(spacing: Space.x2, runSpacing: Space.x2, children: actions),
        ],
      ),
    );
  }
}
