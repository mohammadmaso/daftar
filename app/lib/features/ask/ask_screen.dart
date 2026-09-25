import 'package:flutter/material.dart' show Material, SelectableText;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:image_picker/image_picker.dart';

import '../../core/bidi.dart';
import '../../core/errors.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../wiki/markdown_view.dart';
import '../wiki/wiki_state.dart';
import 'ask_state.dart';

final storiesProvider = FutureProvider<List<String>>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  if (lib == null) return const [];
  try {
    final l = await lib.listDir('vaults/stories');
    return [for (final f in l.folders) f.path.split('/').last];
  } catch (_) {
    return const [];
  }
});

/// Ask (§8.1): a thread with streamed, cited answers; scope chips; photo input; Save to wiki; story
/// co-writer mode. The assistant has no name or persona; it simply answers.
class AskScreen extends ConsumerStatefulWidget {
  const AskScreen({super.key, this.reading, this.panel = false});

  /// Page open beside the panel on desktop (§8.2).
  final String? reading;

  /// Rendered as the desktop side panel rather than a full screen.
  final bool panel;

  @override
  ConsumerState<AskScreen> createState() => _AskScreenState();
}

class _AskScreenState extends ConsumerState<AskScreen> {
  final _input = TextEditingController();
  final _scroll = ScrollController();
  Uint8List? _image;

  @override
  void dispose() {
    _input.dispose();
    _scroll.dispose();
    super.dispose();
  }

  Future<void> _pick() async {
    final x = await ImagePicker().pickImage(
      source: ImageSource.gallery,
      maxWidth: 1600,
      imageQuality: 80,
    );
    if (x == null) return;
    final bytes = await x.readAsBytes();
    if (mounted) setState(() => _image = bytes);
  }

  Future<void> _send() async {
    final q = _input.text;
    if (q.trim().isEmpty) return;
    final image = _image;
    _input.clear();
    setState(() => _image = null);
    HapticFeedback.selectionClick();
    final f = ref
        .read(askProvider.notifier)
        .send(q, image: image, reading: widget.reading);
    WidgetsBinding.instance.addPostFrameCallback((_) => _toEnd());
    await f;
    _toEnd();
  }

  void _toEnd() {
    if (_scroll.hasClients) {
      _scroll.animateTo(
        _scroll.position.maxScrollExtent,
        duration: motion(context, Motion.standard),
        curve: Motion.ease,
      );
    }
  }

  Future<void> _save(AskTurnView t) async {
    final l = L10n.of(context);
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      await lib.saveAnswer(t.question, t.answer, ref.read(askProvider).scope);
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      await ref.read(jobRunnerProvider.notifier).kick();
      if (mounted) showNote(context, l.savedToWiki);
    } catch (e) {
      if (mounted) showNote(context, humanError(e));
    }
  }

  Future<void> _draft(AskTurnView t, String story) async {
    final l = L10n.of(context);
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      final title = t.question.split('\n').first;
      await lib.saveDraft(
        story,
        title.length > 60 ? title.substring(0, 60) : title,
        t.answer,
      );
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      if (mounted) showNote(context, l.draftSaved);
    } catch (e) {
      if (mounted) showNote(context, humanError(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final view = ref.watch(askProvider);
    final vaults = ref.watch(vaultsProvider).value ?? const <Vault>[];
    final stories = ref.watch(storiesProvider).value ?? const <String>[];
    final scope = view.scope;
    bool selected(AskScopeKind k, [String? id]) =>
        scope.kind == k && scope.id == id;

    final chips = [
      DChip(
        label: l.askScopeAll,
        selected: selected(AskScopeKind.all),
        onTap: () => ref
            .read(askProvider.notifier)
            .setScope(const AskScopeDto(kind: AskScopeKind.all)),
      ),
      for (final v in vaults.where((v) => !v.fiction))
        DChip(
          label: lang == 'fa' ? v.titleFa : v.titleEn,
          selected: selected(AskScopeKind.vault, v.id),
          onTap: () => ref
              .read(askProvider.notifier)
              .setScope(AskScopeDto(kind: AskScopeKind.vault, id: v.id)),
        ),
      for (final s in stories)
        DChip(
          label: l.storyScope(s),
          selected: selected(AskScopeKind.story, s),
          onTap: () => ref
              .read(askProvider.notifier)
              .setScope(AskScopeDto(kind: AskScopeKind.story, id: s)),
        ),
    ];

    return Material(
      color: widget.panel ? p.raised : p.paper,
      child: SafeArea(
        bottom: false,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsetsDirectional.fromSTEB(
                Space.gutter,
                Space.x4,
                Space.x2,
                Space.x2,
              ),
              child: Row(
                children: [
                  Expanded(
                    child: Semantics(
                      header: true,
                      child: Text(
                        l.askTitle,
                        style: widget.panel
                            ? context.type.title
                            : context.type.display,
                      ),
                    ),
                  ),
                  if (view.turns.isNotEmpty)
                    DIconButton(
                      icon: DIcons.plus,
                      semanticLabel: l.newConversation,
                      color: p.inkMuted,
                      onPressed: () => ref.read(askProvider.notifier).clear(),
                    ),
                  if (!widget.panel)
                    DIconButton(
                      icon: DIcons.speaker,
                      semanticLabel: l.talkMode,
                      color: p.accent,
                      onPressed: () => context.push('/voice'),
                    ),
                ],
              ),
            ),
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: Space.gutter),
              child: Row(
                children: [
                  for (final c in chips)
                    Padding(
                      padding: const EdgeInsetsDirectional.only(end: Space.x2),
                      child: c,
                    ),
                ],
              ),
            ),
            if (widget.reading != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(
                  Space.gutter,
                  Space.x2,
                  Space.gutter,
                  0,
                ),
                child: Text(
                  l.askReading(
                    widget.reading!.split('/').last.replaceAll('.md', ''),
                  ),
                  style: context.type.caption.copyWith(color: p.inkMuted),
                ),
              ),
            const SizedBox(height: Space.x2),
            const DHairline(),
            Expanded(
              child: view.turns.isEmpty
                  ? Center(
                      child: Padding(
                        padding: const EdgeInsets.all(Space.x8),
                        child: Text(
                          l.askEmpty,
                          textAlign: TextAlign.center,
                          style: context.type.body.copyWith(color: p.inkMuted),
                        ),
                      ),
                    )
                  : ListView.builder(
                      controller: _scroll,
                      padding: const EdgeInsets.fromLTRB(
                        Space.gutter,
                        Space.x4,
                        Space.gutter,
                        Space.x4,
                      ),
                      itemCount: view.turns.length,
                      itemBuilder: (context, i) => _Turn(
                        turn: view.turns[i],
                        story: scope.kind == AskScopeKind.story
                            ? scope.id
                            : null,
                        onSave: () => _save(view.turns[i]),
                        onDraft: (s) => _draft(view.turns[i], s),
                      ),
                    ),
            ),
            const DHairline(),
            SafeArea(
              top: false,
              child: Padding(
                padding: const EdgeInsets.all(Space.x3),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (_image != null)
                      Padding(
                        padding: const EdgeInsets.only(bottom: Space.x2),
                        child: Row(
                          children: [
                            ClipRRect(
                              borderRadius: BorderRadius.circular(Radii.small),
                              child: Image.memory(
                                _image!,
                                width: 56,
                                height: 56,
                                fit: BoxFit.cover,
                              ),
                            ),
                            DIconButton(
                              icon: DIcons.close,
                              semanticLabel: l.removePhoto,
                              color: p.inkMuted,
                              onPressed: () => setState(() => _image = null),
                            ),
                          ],
                        ),
                      ),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.end,
                      children: [
                        DIconButton(
                          icon: DIcons.image,
                          semanticLabel: l.attachPhoto,
                          color: p.inkMuted,
                          onPressed: view.busy ? null : _pick,
                        ),
                        Expanded(
                          child: DTextField(
                            controller: _input,
                            hint: l.askHint,
                            maxLines: 5,
                            minLines: 1,
                            onSubmitted: (_) => _send(),
                          ),
                        ),
                        ListenableBuilder(
                          listenable: _input,
                          builder: (context, _) => DIconButton(
                            icon: DIcons.send,
                            semanticLabel: l.send,
                            color: p.accent,
                            onPressed: view.busy || _input.text.trim().isEmpty
                                ? null
                                : _send,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _Turn extends ConsumerWidget {
  const _Turn({
    required this.turn,
    required this.story,
    required this.onSave,
    required this.onDraft,
  });
  final AskTurnView turn;
  final String? story;
  final VoidCallback onSave;
  final ValueChanged<String> onDraft;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.x6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // The question: set apart by weight and a quiet rule, not a chat bubble.
          Container(
            padding: const EdgeInsetsDirectional.only(start: Space.x3),
            decoration: BoxDecoration(
              border: BorderDirectional(
                start: BorderSide(color: p.accent, width: 2),
              ),
            ),
            child: Row(
              children: [
                if (turn.hadImage) ...[
                  DIcon(DIcons.image, size: 16, color: p.inkMuted),
                  const SizedBox(width: Space.x2),
                ],
                Expanded(
                  child: Text(
                    turn.question,
                    textDirection: directionOf(
                      turn.question,
                      fallback: Directionality.of(context),
                    ),
                    style: context.type.bodyStrong.copyWith(color: p.ink),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: Space.x3),
          if (turn.error != null)
            Text(
              turn.error!,
              style: context.type.small.copyWith(color: p.critical),
            )
          else if (turn.answer.isEmpty)
            Text(
              l.thinking,
              style: context.type.small.copyWith(color: p.inkMuted),
            )
          else
            MarkdownView(
              text: turn.answer,
              onLink: (t) => openLink(
                context,
                ref,
                t,
                missing: l.pageMissing,
                note: (m) => showNote(context, m),
              ),
            ),
          for (final a in turn.approvals)
            Padding(
              padding: const EdgeInsets.only(top: Space.x3),
              child: _ApprovalCard(approval: a),
            ),
          if (turn.needsHelp) ...[
            const SizedBox(height: Space.x3),
            const TalkToSomeoneCard(),
          ],
          if (turn.done && turn.error == null && turn.answer.isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(top: Space.x2),
              child: Wrap(
                spacing: Space.x2,
                children: [
                  DButton(
                    label: l.saveToWiki,
                    variant: DButtonVariant.quiet,
                    onPressed: onSave,
                  ),
                  if (story != null)
                    DButton(
                      label: l.saveDraft,
                      variant: DButtonVariant.quiet,
                      onPressed: () => onDraft(story!),
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

/// §10: an outside tool asks before it runs (unless its server's policy allows it).
class _ApprovalCard extends ConsumerWidget {
  const _ApprovalCard({required this.approval});
  final ToolApproval approval;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    return DSurface(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            l.toolApproval(approval.serverName, approval.tool),
            style: context.type.bodyStrong,
          ),
          if (approval.readOnly)
            Text(
              l.readOnlyTool,
              style: context.type.caption.copyWith(color: p.inkMuted),
            ),
          const SizedBox(height: Space.x2),
          Container(
            padding: const EdgeInsets.all(Space.x2),
            decoration: BoxDecoration(
              color: p.sunken,
              borderRadius: BorderRadius.circular(Radii.small),
            ),
            child: Text(
              approval.arguments,
              textDirection: TextDirection.ltr,
              style: TypeScale.mono.copyWith(fontSize: 12.5, color: p.ink),
            ),
          ),
          const SizedBox(height: Space.x3),
          Wrap(
            spacing: Space.x2,
            children: [
              DButton(
                label: l.allow,
                onPressed: () => ref
                    .read(askProvider.notifier)
                    .answerApproval(approval, true),
              ),
              DButton(
                label: l.deny,
                variant: DButtonVariant.secondary,
                onPressed: () => ref
                    .read(askProvider.notifier)
                    .answerApproval(approval, false),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// §4.7: warm, prominent, never flippant. Professional help and a crisis line.
class TalkToSomeoneCard extends ConsumerWidget {
  const TalkToSomeoneCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    final country =
        ref.watch(helplineCountryProvider).value ?? (lang == 'fa' ? 'IR' : '');
    final lines = ref.read(providerApiProvider).helplines(country);
    return Semantics(
      container: true,
      liveRegion: true,
      child: Container(
        padding: const EdgeInsets.all(Space.x4),
        decoration: BoxDecoration(
          color: p.accentSoft,
          borderRadius: BorderRadius.circular(Radii.large),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              l.talkToSomeone,
              style: context.type.heading.copyWith(color: p.accent),
            ),
            const SizedBox(height: Space.x2),
            Text(l.talkToSomeoneBody, style: context.type.body),
            const SizedBox(height: Space.x3),
            for (final h in lines)
              Padding(
                padding: const EdgeInsets.only(bottom: Space.x2),
                child: SelectableText.rich(
                  TextSpan(
                    children: [
                      TextSpan(
                        text: lang == 'fa' ? h.nameFa : h.nameEn,
                        style: context.type.bodyStrong,
                      ),
                      if (h.phone.isNotEmpty)
                        TextSpan(
                          text: '  ${h.phone}',
                          style: context.type.bodyStrong.copyWith(
                            color: p.accent,
                          ),
                        ),
                      if (h.url.isNotEmpty)
                        TextSpan(
                          text: '  ${h.url}',
                          style: context.type.small.copyWith(color: p.accent),
                        ),
                    ],
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// The helpline country chosen in Settings › Reflect (empty until M8 settings set it).
final helplineCountryProvider = FutureProvider<String?>((ref) async => null);
