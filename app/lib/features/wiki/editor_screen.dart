import 'package:flutter/material.dart'
    show Material, TextField, InputDecoration;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/bidi.dart';
import '../../core/errors.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import 'wiki_state.dart';

/// Colours Markdown source as it is typed: frontmatter, headings, wikilinks, inline fields and
/// block ids, code. Text stays editable exactly as written.
class MarkdownSourceController extends TextEditingController {
  MarkdownSourceController({super.text, required this.palette});

  Palette palette;

  static final _token = RegExp(
    r'(^---$[\s\S]*?^---$)|(^#{1,6} .*$)|(!?\[\[[^\]\n]+\]\])|(\([A-Za-z_][\w-]*::[^()\n]*\))|(\s\^[A-Za-z0-9-]+$)|(`[^`\n]+`)|(^```[\s\S]*?^```$)',
    multiLine: true,
  );

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    final base = style ?? const TextStyle();
    if (withComposing && value.composing.isValid) {
      return super.buildTextSpan(
        context: context,
        style: style,
        withComposing: withComposing,
      );
    }
    final children = <TextSpan>[];
    var last = 0;
    for (final m in _token.allMatches(text)) {
      if (m.start > last) {
        children.add(TextSpan(text: text.substring(last, m.start)));
      }
      final s = m[0]!;
      final TextStyle st;
      if (m[1] != null) {
        st = TextStyle(color: palette.inkMuted);
      } else if (m[2] != null) {
        st = const TextStyle(fontWeight: FontWeight.w600);
      } else if (m[3] != null) {
        st = TextStyle(color: palette.accent);
      } else if (m[4] != null || m[5] != null) {
        st = TextStyle(color: palette.inkMuted);
      } else {
        st = TypeScale.mono.copyWith(
          color: palette.ink,
          fontSize: (base.fontSize ?? 16) * 0.9,
        );
      }
      children.add(TextSpan(text: s, style: st));
      last = m.end;
    }
    if (last < text.length) children.add(TextSpan(text: text.substring(last)));
    return TextSpan(style: base, children: children);
  }
}

class EditorScreen extends ConsumerStatefulWidget {
  const EditorScreen({super.key, required this.path});
  final String path;

  @override
  ConsumerState<EditorScreen> createState() => _EditorScreenState();
}

class _EditorScreenState extends ConsumerState<EditorScreen> {
  MarkdownSourceController? _c;
  String? _hash;
  String _original = '';
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    _c?.dispose();
    super.dispose();
  }

  bool get _dirty => _c != null && _c!.text != _original;

  Future<void> _save() async {
    final l = L10n.of(context);
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final lib = await ref.read(libraryProvider.future);
      final r = await lib!.savePage(widget.path, _hash!, _c!.text);
      _hash = r.hash;
      _original = _c!.text;
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      if (mounted) {
        showNote(context, l.saved);
        context.pop();
      }
    } catch (e) {
      if (mounted) setState(() => _error = humanError(e));
    }
    if (mounted) setState(() => _saving = false);
  }

  Future<void> _leave() async {
    if (!_dirty) {
      context.pop();
      return;
    }
    final l = L10n.of(context);
    final leave = await showDSheet<bool>(
      context,
      builder: (sheet) => Padding(
        padding: const EdgeInsets.all(Space.x4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(l.unsavedChanges, style: sheet.type.title),
            const SizedBox(height: Space.x4),
            DButton(
              label: l.keepEditing,
              onPressed: () => Navigator.of(sheet).pop(false),
            ),
            const SizedBox(height: Space.x2),
            DButton(
              label: l.discard,
              variant: DButtonVariant.quiet,
              onPressed: () => Navigator.of(sheet).pop(true),
            ),
          ],
        ),
      ),
    );
    if (leave == true && mounted) context.pop();
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final page = ref.watch(pageProvider(widget.path));
    return Material(
      color: p.paper,
      child: SafeArea(
        child: page.when(
          loading: () => const SizedBox.shrink(),
          error: (e, _) => Center(
            child: Text(
              humanError(e),
              style: context.type.body.copyWith(color: p.critical),
            ),
          ),
          data: (pg) {
            if (_c == null) {
              _c = MarkdownSourceController(text: pg.text, palette: p);
              _hash = pg.hash;
              _original = pg.text;
            }
            _c!.palette = p;
            final lang = Localizations.localeOf(context).languageCode;
            return PopScope(
              canPop: !_dirty,
              onPopInvokedWithResult: (didPop, _) {
                if (!didPop) _leave();
              },
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Padding(
                    padding: const EdgeInsetsDirectional.fromSTEB(
                      Space.x2,
                      Space.x2,
                      Space.x4,
                      Space.x2,
                    ),
                    child: Row(
                      children: [
                        DIconButton(
                          icon: DIcons.close,
                          semanticLabel: l.cancel,
                          onPressed: _leave,
                        ),
                        const SizedBox(width: Space.x2),
                        Expanded(
                          child: Text(
                            pageTitle(lang, pg.titleEn, pg.titleFa, pg.path),
                            style: context.type.heading,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        DButton(
                          label: l.save,
                          onPressed: _saving || !_dirty ? null : _save,
                        ),
                      ],
                    ),
                  ),
                  if (_error != null)
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: Space.x4),
                      child: Text(
                        _error!,
                        style: context.type.small.copyWith(color: p.critical),
                      ),
                    ),
                  const DHairline(),
                  Expanded(
                    child: Align(
                      alignment: AlignmentDirectional.topCenter,
                      child: ConstrainedBox(
                        constraints: const BoxConstraints(
                          maxWidth: Space.measure,
                        ),
                        child: ListenableBuilder(
                          listenable: _c!,
                          builder: (context, _) => TextField(
                            controller: _c,
                            maxLines: null,
                            expands: true,
                            keyboardType: TextInputType.multiline,
                            // The document's direction follows its first strong character.
                            textDirection: directionOf(
                              pg.body,
                              fallback: Directionality.of(context),
                            ),
                            style: context.type.body.copyWith(color: p.ink),
                            decoration:
                                const InputDecoration.collapsed(
                                  hintText: null,
                                ).copyWith(
                                  contentPadding: const EdgeInsets.all(
                                    Space.x4,
                                  ),
                                ),
                            onChanged: (_) => setState(() {}),
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            );
          },
        ),
      ),
    );
  }
}
