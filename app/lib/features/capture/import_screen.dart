import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import '../../core/bidi.dart';
import '../../core/dates.dart';
import '../../core/errors.dart';
import '../../core/file_import.dart';
import '../../core/job_runner.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import 'capture_bar.dart';

/// Preview of a picked file before it becomes a capture (§4.1): the text read from a document, or
/// a recording to play. Nothing is saved until "File it".
class ImportScreen extends ConsumerStatefulWidget {
  const ImportScreen({super.key, required this.path});
  final String path;

  @override
  ConsumerState<ImportScreen> createState() => _ImportScreenState();
}

class _ImportScreenState extends ConsumerState<ImportScreen> {
  late final Future<ImportPreview> _preview = ref
      .read(fileImportsProvider)
      .read(widget.path);
  final _text = TextEditingController();
  bool _editing = false;
  bool _filing = false;

  // Recording preview.
  bool _playing = false;
  StreamSubscription<bool>? _playSub;

  String get _name => widget.path.split(RegExp(r'[/\\]')).last;

  @override
  void initState() {
    super.initState();
    _preview.then((p) {
      if (mounted) _text.text = p.text;
    }, onError: (_) {});
  }

  @override
  void dispose() {
    _playSub?.cancel();
    _text.dispose();
    super.dispose();
  }

  void _close() {
    if (context.canPop()) {
      context.pop();
    } else {
      context.go('/');
    }
  }

  Future<void> _togglePlay() async {
    final player = ref.read(filePlayerProvider);
    _playSub ??= player.playing.listen((v) {
      if (mounted) setState(() => _playing = v);
    });
    if (_playing) {
      await player.stop();
    } else {
      await player.play(widget.path);
    }
  }

  Future<void> _file(ImportPreview p) async {
    if (_filing) return;
    setState(() => _filing = true);
    final l = L10n.of(context);
    try {
      final lib = await ref.read(libraryProvider.future);
      if (lib == null) return;
      final vault = ref.read(pinnedVaultProvider.notifier).take();
      if (p.kind == ImportKind.audio) {
        await ref.read(filePlayerProvider).stop();
        await lib.captureAudioFile(widget.path, vault: vault);
      } else {
        await lib.captureImport(_text.text.trim(), p.name, vault: vault);
      }
      HapticFeedback.lightImpact();
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      ref.read(jobRunnerProvider.notifier).kick();
      if (!mounted) return;
      context.go('/');
      showNote(context, l.importSaved(p.name));
    } catch (e) {
      if (mounted) showNote(context, humanError(e));
    } finally {
      if (mounted) setState(() => _filing = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    return ColoredBox(
      color: p.paper,
      child: FutureBuilder<ImportPreview>(
        future: _preview,
        builder: (context, snap) {
          if (snap.hasError) {
            return DPage(
              title: _name,
              onBack: _close,
              backLabel: l.back,
              children: [
                Text(
                  humanError(snap.error!),
                  style: context.type.body.copyWith(color: p.ink),
                ),
                Align(
                  alignment: AlignmentDirectional.centerStart,
                  child: DButton(
                    label: l.back,
                    variant: DButtonVariant.secondary,
                    onPressed: _close,
                  ),
                ),
              ],
            );
          }
          final preview = snap.data;
          if (preview == null) {
            return DPage(
              title: _name,
              onBack: _close,
              backLabel: l.back,
              children: [
                Text(
                  l.importReading(_name),
                  style: context.type.body.copyWith(color: p.inkMuted),
                ),
              ],
            );
          }
          return Column(
            children: [
              Expanded(child: _body(context, preview)),
              _Actions(
                audio: preview.kind == ImportKind.audio,
                busy: _filing,
                onCancel: _close,
                onFile: () => _file(preview),
                canFile: preview.kind == ImportKind.audio
                    ? true
                    : _text.text.trim().isNotEmpty,
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _body(BuildContext context, ImportPreview preview) {
    final l = L10n.of(context);
    final p = context.palette;
    final lang = Localizations.localeOf(context).languageCode;
    String n(num v) {
      final s = NumberFormat.decimalPattern('en').format(v);
      return lang == 'fa' ? persianDigits(s) : s;
    }

    final mb = n(
      double.parse(
        (preview.bytes.toDouble() / (1024 * 1024)).toStringAsFixed(1),
      ),
    );
    final facts = [
      _kindLabel(l, preview.kind),
      if (preview.pages case final pages?)
        preview.kind == ImportKind.pptx
            ? l.importSlides(pages)
            : l.importPages(pages),
      if (preview.kind != ImportKind.audio)
        l.importChars(n(preview.totalChars)),
      l.importSizeMb(mb),
    ];
    final meta = <Widget>[
      Text(
        facts.join(l.listSeparator),
        style: context.type.small.copyWith(color: p.inkMuted),
      ),
      if (preview.truncated)
        _Notice(
          l.importTruncated(
            n(ref.read(fileImportsProvider).charLimit),
            n(preview.totalChars),
          ),
        ),
      if (preview.encoding case final enc?) _Notice(l.importEncoding(enc)),
    ];

    if (preview.kind == ImportKind.audio) {
      return DPage(
        title: preview.name,
        onBack: _close,
        backLabel: l.back,
        children: [
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              for (final m in meta) ...[m, const SizedBox(height: Space.x3)],
            ],
          ),
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: DButton(
              label: _playing ? l.importStopPlaying : l.importPlay,
              icon: _playing ? DIcons.stop : DIcons.speaker,
              variant: DButtonVariant.secondary,
              onPressed: _togglePlay,
            ),
          ),
          Text(
            l.importAudioNote,
            style: context.type.body.copyWith(color: p.inkMuted),
          ),
        ],
      );
    }

    final paragraphs = _editing
        ? const <String>[]
        : _text.text
              .split(RegExp(r'\n\s*\n'))
              .where((s) => s.trim().isNotEmpty)
              .toList();
    return DPage(
      title: preview.name,
      onBack: _close,
      backLabel: l.back,
      selectable: !_editing,
      blocks: _editing
          ? [
              DTextField(
                controller: _text,
                minLines: 12,
                maxLines: null,
                keyboardType: TextInputType.multiline,
                onChanged: (_) => setState(() {}),
              ),
            ]
          : [
              for (final para in paragraphs)
                Padding(
                  padding: const EdgeInsets.only(bottom: Space.x4),
                  child: Text(
                    para.trim(),
                    textDirection: directionOf(para),
                    style: context.type.body.copyWith(color: p.ink),
                  ),
                ),
            ],
      trailing: DButton(
        label: _editing ? l.importDoneEditing : l.importEdit,
        icon: _editing ? DIcons.check : DIcons.edit,
        variant: DButtonVariant.quiet,
        onPressed: () => setState(() => _editing = !_editing),
      ),
      children: [
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (final (i, m) in meta.indexed) ...[
              if (i > 0) const SizedBox(height: Space.x3),
              m,
            ],
          ],
        ),
      ],
    );
  }

  static String _kindLabel(L10n l, ImportKind k) => switch (k) {
    ImportKind.text => l.importKindText,
    ImportKind.markdown => l.importKindMarkdown,
    ImportKind.html => l.importKindHtml,
    ImportKind.rtf => l.importKindRtf,
    ImportKind.docx => l.importKindDocx,
    ImportKind.odt => l.importKindOdt,
    ImportKind.pptx => l.importKindPptx,
    ImportKind.epub => l.importKindEpub,
    ImportKind.pdf => l.importKindPdf,
    ImportKind.audio => l.importKindAudio,
  };
}

class _Notice extends StatelessWidget {
  const _Notice(this.text);
  final String text;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(Space.x3),
      decoration: BoxDecoration(
        color: p.sunken,
        borderRadius: BorderRadius.circular(Radii.medium),
        border: Border.all(color: p.hairline, width: Stroke.hairline),
      ),
      child: Text(text, style: context.type.small.copyWith(color: p.ink)),
    );
  }
}

/// Vault pin, Cancel and File it, pinned under the preview.
class _Actions extends ConsumerWidget {
  const _Actions({
    required this.audio,
    required this.busy,
    required this.canFile,
    required this.onCancel,
    required this.onFile,
  });

  final bool audio;
  final bool busy;
  final bool canFile;
  final VoidCallback onCancel;
  final VoidCallback onFile;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final pinned = ref.watch(pinnedVaultProvider);
    final vaults = ref.watch(vaultsProvider).value ?? const [];
    final lang = Localizations.localeOf(context).languageCode;
    final pinnedLabel = pinned == null
        ? l.vaultAuto
        : (vaults
                  .where((v) => v.id == pinned)
                  .map((v) => lang == 'fa' ? v.titleFa : v.titleEn)
                  .firstOrNull ??
              pinned);
    return Container(
      decoration: BoxDecoration(
        color: p.paper,
        border: Border(top: BorderSide(color: p.hairline)),
      ),
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsetsDirectional.fromSTEB(
            Space.x4,
            Space.x3,
            Space.x4,
            Space.x3,
          ),
          child: Row(
            children: [
              Flexible(
                flex: 0,
                child: DChip(
                  label: pinnedLabel,
                  selected: pinned != null,
                  onTap: () async {
                    final picked = await showDSheet<({String? id})>(
                      context,
                      builder: (_) => const VaultPickerSheet(),
                    );
                    if (picked != null) {
                      ref.read(pinnedVaultProvider.notifier).set(picked.id);
                    }
                  },
                ),
              ),
              const Spacer(),
              // On a phone the header's back button already cancels.
              if (MediaQuery.sizeOf(context).width >= 480) ...[
                DButton(
                  label: l.cancel,
                  variant: DButtonVariant.quiet,
                  onPressed: onCancel,
                ),
                const SizedBox(width: Space.x2),
              ],
              DButton(
                label: audio ? l.importTranscribe : l.importFileIt,
                icon: DIcons.check,
                onPressed: busy || !canFile ? null : onFile,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
