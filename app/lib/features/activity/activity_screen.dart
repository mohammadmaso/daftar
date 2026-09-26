import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart' show NumberFormat;

import '../../core/bidi.dart';
import '../../core/dates.dart';
import '../../core/errors.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../wiki/markdown_view.dart';
import '../wiki/wiki_state.dart';

final activityProvider = FutureProvider<List<Operation>>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib?.activity(limit: 100) ?? const [];
});

final operationProvider =
    FutureProvider.family<(Operation, List<PageDiff>), String>((ref, id) async {
      ref.watch(revisionProvider);
      final lib = await ref.watch(libraryProvider.future);
      return (await lib!.operation(id), await lib.operationDiff(id));
    });

String vaultNames(List<String> ids, List<Vault> vaults, String lang) => ids
    .map((id) {
      final v = vaults.where((x) => x.id == id).firstOrNull;
      return v == null ? id : (lang == 'fa' ? v.titleFa : v.titleEn);
    })
    .join(' · ');

/// One line in the UI language for what an op did (§7 "a human summary").
String opHeadline(L10n l, Operation o, List<Vault> vaults, String lang) =>
    switch (o.kind) {
      OpKind.ingest when o.vaults.isEmpty => l.opNothingFiled,
      OpKind.ingest => l.filedTo(vaultNames(o.vaults, vaults, lang)),
      OpKind.undo => l.opUndo,
      OpKind.compensate => l.opCompensate,
      OpKind.review => l.opReview,
      OpKind.saveAnswer => l.opSaveAnswer,
      OpKind.lint => l.opLint,
      OpKind.reflect => l.opReflect,
    };

DIcons opIcon(OpKind k) => switch (k) {
  OpKind.ingest || OpKind.saveAnswer => DIcons.wiki,
  OpKind.undo || OpKind.compensate => DIcons.undo,
  OpKind.review => DIcons.review,
  OpKind.lint => DIcons.check,
  OpKind.reflect => DIcons.today,
};

/// The Activity screen (§7): every AI operation, newest first.
class ActivityScreen extends ConsumerWidget {
  const ActivityScreen({super.key, this.embedded = false});
  final bool embedded;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final vaults = ref.watch(vaultsProvider).value ?? const <Vault>[];
    final ops = ref.watch(activityProvider);
    return DPage(
      title: l.activityTitle,
      onBack: embedded || !context.canPop() ? null : () => context.pop(),
      backLabel: l.back,
      children: [
        ops.when(
          loading: () => const SizedBox.shrink(),
          error: (e, _) => Text(
            humanError(e),
            style: context.type.body.copyWith(color: p.critical),
          ),
          data: (list) => list.isEmpty
              ? Text(
                  l.noActivity,
                  style: context.type.body.copyWith(color: p.inkMuted),
                )
              : DSection(
                  children: [
                    for (final o in list)
                      DListRow(
                        leading: DIcon(
                          opIcon(o.kind),
                          size: 20,
                          color: o.undone ? p.inkFaint : p.inkMuted,
                        ),
                        title: opHeadline(l, o, vaults, lang),
                        subtitle: [
                          if (DateTime.tryParse(o.startedAt)?.toLocal()
                              case final at?)
                            '${shortDate(at, lang)} ${clockTime(at, lang)}',
                          if (o.pagesCreated + o.pagesUpdated > 0)
                            l.pagesUpdated(o.pagesCreated + o.pagesUpdated),
                          if (o.undone) l.undoneTag,
                        ].join(' · '),
                        chevron: true,
                        onTap: () => context.push('/activity/op?id=${o.opId}'),
                      ),
                  ],
                ),
        ),
      ],
    );
  }
}

/// One operation (§7): summary, why the vault was chosen, per-page diffs, and the corrections.
class OperationScreen extends ConsumerStatefulWidget {
  const OperationScreen({super.key, required this.opId});
  final String opId;

  @override
  ConsumerState<OperationScreen> createState() => _OperationScreenState();
}

class _OperationScreenState extends ConsumerState<OperationScreen> {
  bool _raw = false;
  bool _busy = false;

  Future<void> _run(
    Future<UndoResult> Function(LibraryApi) action, {
    bool refile = false,
  }) async {
    final l = L10n.of(context);
    setState(() => _busy = true);
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      final r = await action(lib);
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      await ref.read(jobRunnerProvider.notifier).kick();
      if (mounted) {
        showNote(context, switch (r) {
          UndoResult.queued => l.undoQueued,
          UndoResult.done when refile => l.refiling,
          UndoResult.done => l.undoneNote,
        });
      }
    } catch (e) {
      if (mounted) showNote(context, humanError(e));
    }
    if (mounted) setState(() => _busy = false);
  }

  Future<void> _move(Operation o) async {
    final l = L10n.of(context);
    final lang = Localizations.localeOf(context).languageCode;
    final vaults = ref.read(vaultsProvider).value ?? const <Vault>[];
    final vault = await showDSheet<String>(
      context,
      builder: (sheet) => Padding(
        padding: const EdgeInsets.all(Space.x4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(l.moveToVault, style: sheet.type.title),
            const SizedBox(height: Space.x4),
            Wrap(
              spacing: Space.x2,
              runSpacing: Space.x2,
              children: [
                for (final v in vaults.where((v) => !o.vaults.contains(v.id)))
                  DChip(
                    label: lang == 'fa' ? v.titleFa : v.titleEn,
                    selected: false,
                    onTap: () => Navigator.of(sheet).pop(v.id),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
    if (vault != null) {
      await _run((lib) => lib.moveToVault(o.opId, vault), refile: true);
    }
  }

  Future<void> _rerun(Operation o) async {
    final l = L10n.of(context);
    final note = await showTextPrompt(
      context,
      title: l.rerunWithNote,
      hint: l.rerunHint,
      action: l.continueAction,
    );
    if (note != null) {
      await _run((lib) => lib.rerunWithNote(o.opId, note), refile: true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final vaults = ref.watch(vaultsProvider).value ?? const <Vault>[];
    final data = ref.watch(operationProvider(widget.opId));
    return data.when(
      loading: () => const SizedBox.shrink(),
      error: (e, _) => DPage(
        title: l.activityTitle,
        onBack: context.canPop() ? () => context.pop() : null,
        backLabel: l.back,
        children: [
          Text(
            humanError(e),
            style: context.type.body.copyWith(color: p.critical),
          ),
        ],
      ),
      data: (d) {
        final (o, diffs) = d;
        final pct = NumberFormat.percentPattern(lang);
        final n = NumberFormat.decimalPattern(lang);
        final canUndo = !o.undone;
        return DPage(
          title: opHeadline(l, o, vaults, lang),
          onBack: context.canPop() ? () => context.pop() : null,
          backLabel: l.back,
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // The ledger summary is written in English; the headline above is localized.
                if (o.summary.isNotEmpty && lang == 'en')
                  Text(
                    o.summary,
                    textDirection: directionOf(o.summary),
                    style: context.type.body.copyWith(color: p.inkMuted),
                  ),
                if (o.note != null)
                  Padding(
                    padding: const EdgeInsets.only(top: Space.x2),
                    child: Text(
                      '“${o.note}”',
                      textDirection: directionOf(o.note!),
                      style: context.type.body,
                    ),
                  ),
                const SizedBox(height: Space.x2),
                Text(
                  [
                    ...o.models.where((m) => !m.contains(' v')),
                    l.usageTokens(
                      n.format((o.inputTokens + o.outputTokens).toInt()),
                    ),
                    if (o.costUsd != null)
                      l.costApprox(
                        NumberFormat.simpleCurrency(
                          name: 'USD',
                        ).format(o.costUsd),
                      ),
                  ].join(' · '),
                  style: context.type.caption.copyWith(color: p.inkMuted),
                ),
              ],
            ),
            if (o.route.isNotEmpty)
              DSection(
                title: l.whyHere,
                children: [
                  for (final t in o.route)
                    DListRow(
                      title: vaultNames([t.vault], vaults, lang),
                      subtitle: t.reason,
                      trailing: Text(
                        l.confidencePct(pct.format(t.confidence)),
                        style: context.type.caption.copyWith(color: p.inkMuted),
                      ),
                    ),
                ],
              ),
            if (canUndo && o.kind != OpKind.lint && o.kind != OpKind.reflect)
              Wrap(
                spacing: Space.x2,
                runSpacing: Space.x2,
                children: [
                  DButton(
                    label: o.kind == OpKind.undo || o.kind == OpKind.compensate
                        ? l.undoUndo
                        : l.undoAction,
                    icon: DIcons.undo,
                    onPressed: _busy
                        ? null
                        : () => _run((lib) => lib.undo(o.opId)),
                  ),
                  if (o.kind == OpKind.ingest) ...[
                    DButton(
                      label: l.moveToVault,
                      variant: DButtonVariant.secondary,
                      onPressed: _busy ? null : () => _move(o),
                    ),
                    DButton(
                      label: l.rerunWithNote,
                      variant: DButtonVariant.secondary,
                      onPressed: _busy ? null : () => _rerun(o),
                    ),
                    DButton(
                      label: l.excludeSource,
                      variant: DButtonVariant.quiet,
                      onPressed: _busy
                          ? null
                          : () => _run((lib) => lib.undo(o.opId)),
                    ),
                  ],
                ],
              ),
            if (diffs.isNotEmpty)
              Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          l.changes,
                          style: context.type.caption.copyWith(
                            color: p.inkMuted,
                          ),
                        ),
                      ),
                      Text(
                        l.rawDiff,
                        style: context.type.caption.copyWith(color: p.inkMuted),
                      ),
                      const SizedBox(width: Space.x2),
                      DSwitch(
                        value: _raw,
                        onChanged: (v) => setState(() => _raw = v),
                        semanticLabel: l.rawDiff,
                      ),
                    ],
                  ),
                  const SizedBox(height: Space.x2),
                  for (final f in diffs) ...[
                    _DiffView(diff: f, raw: _raw),
                    const SizedBox(height: Space.x4),
                  ],
                ],
              ),
          ],
        );
      },
    );
  }
}

/// A page's changes: rendered (added lines tinted, removed lines struck through) or raw.
class _DiffView extends ConsumerWidget {
  const _DiffView({required this.diff, required this.raw});
  final PageDiff diff;
  final bool raw;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final lines = diff.lines
        .where(
          (x) =>
              !(x.text.startsWith('---') ||
                  x.text.startsWith('updated:') ||
                  x.text.startsWith('sources:')),
        )
        .toList();
    return DSurface(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Pressable(
            onPressed: diff.change == PageChange.deleted
                ? null
                : () => context.push(pageRoute(diff.path)),
            radius: 0,
            child: Padding(
              padding: const EdgeInsets.all(Space.x3),
              child: Text(
                diff.path,
                textDirection: TextDirection.ltr,
                style: context.type.caption.copyWith(color: p.accent),
              ),
            ),
          ),
          const DHairline(),
          for (final line in lines)
            switch (line.kind) {
              DiffLineKind.gap => Padding(
                padding: const EdgeInsets.symmetric(vertical: Space.x1),
                child: Center(
                  child: Text(
                    '⋯',
                    style: context.type.caption.copyWith(color: p.inkMuted),
                  ),
                ),
              ),
              _ when raw => Container(
                color: switch (line.kind) {
                  DiffLineKind.added => p.positive.withValues(alpha: 0.10),
                  DiffLineKind.removed => p.critical.withValues(alpha: 0.10),
                  _ => null,
                },
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.x3,
                  vertical: 1,
                ),
                child: Text(
                  '${switch (line.kind) {
                    DiffLineKind.added => '+',
                    DiffLineKind.removed => '-',
                    _ => ' ',
                  }} ${line.text}',
                  textDirection: TextDirection.ltr,
                  style: TypeScale.mono.copyWith(fontSize: 12.5, color: p.ink),
                ),
              ),
              DiffLineKind.context => const SizedBox.shrink(),
              _ => Container(
                padding: const EdgeInsetsDirectional.fromSTEB(
                  Space.x3,
                  Space.x1,
                  Space.x3,
                  Space.x1,
                ),
                decoration: BoxDecoration(
                  color: line.kind == DiffLineKind.added
                      ? p.positive.withValues(alpha: 0.08)
                      : null,
                  border: BorderDirectional(
                    start: BorderSide(
                      color: line.kind == DiffLineKind.added
                          ? p.positive
                          : p.critical,
                      width: 2,
                    ),
                  ),
                ),
                child: line.text.trim().isEmpty
                    ? const SizedBox(height: Space.x2)
                    : Opacity(
                        // Removed text is shown faded beside its red rule.
                        opacity: line.kind == DiffLineKind.removed ? 0.5 : 1,
                        child: MarkdownView(
                          text: line.text,
                          onLink: (_) {},
                          selectable: false,
                        ),
                      ),
              ),
            },
          if (lines.isEmpty)
            Padding(
              padding: const EdgeInsets.all(Space.x3),
              child: Text(
                l.changes,
                style: context.type.small.copyWith(color: p.inkMuted),
              ),
            ),
        ],
      ),
    );
  }
}
