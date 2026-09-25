import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart' show SelectionArea;
import 'package:flutter/widgets.dart';
import 'package:markdown/markdown.dart' as md;

import '../../core/bidi.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// `[[target|label]]` and `![[target]]` (Obsidian wikilinks and embeds).
class WikiLinkSyntax extends md.InlineSyntax {
  WikiLinkSyntax() : super(r'(!?)\[\[([^\[\]\n|]+?)(?:\|([^\[\]\n]+?))?\]\]');

  @override
  bool onMatch(md.InlineParser parser, Match match) {
    final target = match[2]!.trim();
    final label = match[3]?.trim() ?? displayName(target);
    final el = md.Element.text(
      match[1] == '!' ? 'wikiembed' : 'wikilink',
      label,
    );
    el.attributes['target'] = target;
    parser.addNode(el);
    return true;
  }

  /// `vaults/life/people/sara#Timeline` → `sara`.
  static String displayName(String target) {
    final noFrag = target.split(RegExp(r'[#^]')).first;
    return noFrag.split('/').last;
  }
}

/// Dataview inline fields: `(status:: proposed)`, `(src:: [[raw/…|voice · 3 Mar]])`.
class InlineFieldSyntax extends md.InlineSyntax {
  InlineFieldSyntax()
    : super(r'\(([A-Za-z_][\w-]*)::\s*((?:[^()\[\]]|\[\[[^\]]*\]\])*)\)');

  @override
  bool onMatch(md.InlineParser parser, Match match) {
    final value = match[2]!.trim();
    final children = md.InlineParser(value, parser.document).parse();
    final el = md.Element('field', children);
    el.attributes['key'] = match[1]!;
    el.attributes['value'] = value;
    parser.addNode(el);
    return true;
  }
}

/// Block ids at the end of a line: ` ^c-01JAB…` (kept in the file, hidden when reading).
class BlockIdSyntax extends md.InlineSyntax {
  BlockIdSyntax() : super(r'\s\^([A-Za-z0-9-]+)\s*$');

  @override
  bool onMatch(md.InlineParser parser, Match match) {
    final el = md.Element.empty('blockid');
    el.attributes['id'] = match[1]!;
    parser.addNode(el);
    return true;
  }
}

List<md.Node> parseMarkdown(String text) {
  final doc = md.Document(
    extensionSet: md.ExtensionSet.gitHubFlavored,
    inlineSyntaxes: [WikiLinkSyntax(), InlineFieldSyntax(), BlockIdSyntax()],
    encodeHtml: false,
  );
  return doc.parseLines(text.replaceAll('\r\n', '\n').split('\n'));
}

/// Plain text of a subtree (for direction detection and semantics).
String plainText(md.Node n) => switch (n) {
  md.Text() => n.text,
  md.Element(tag: 'blockid') => '',
  md.Element(tag: 'field') => '',
  md.Element() => (n.children ?? const []).map(plainText).join(),
  _ => n.textContent,
};

typedef LinkTap = void Function(String target);

/// Renders a page body (§8.3): headings, wikilinks, callouts, tables, task lists, code, images,
/// footnotes, inline fields as chips, claim lines with a status pill. Direction is decided per
/// paragraph from its first strong character, independent of the UI language.
class MarkdownView extends StatelessWidget {
  const MarkdownView({
    super.key,
    required this.text,
    required this.onLink,
    this.imageRoot,
    this.selectable = true,
  });

  final String text;
  final LinkTap onLink;

  /// Library root for `raw/assets/…` images.
  final String? imageRoot;
  final bool selectable;

  @override
  Widget build(BuildContext context) {
    final nodes = parseMarkdown(text);
    final r = _Renderer(context, onLink, imageRoot);
    final blocks = r.blocks(nodes);
    final column = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: blocks,
    );
    return selectable ? SelectionArea(child: column) : column;
  }
}

class _Renderer {
  _Renderer(this.context, this.onLink, this.imageRoot);

  final BuildContext context;
  final LinkTap onLink;
  final String? imageRoot;

  Palette get p => context.palette;
  TypeScale get t => context.type;
  L10n get l => L10n.of(context);

  List<Widget> blocks(List<md.Node> nodes, {int depth = 0}) {
    final out = <Widget>[];
    for (final n in nodes) {
      final w = block(n, depth: depth);
      if (w != null) out.add(w);
    }
    return out;
  }

  Widget _gap(Widget child, {double below = Space.x3}) => Padding(
    padding: EdgeInsetsDirectional.only(bottom: below),
    child: child,
  );

  Widget? block(md.Node n, {int depth = 0}) {
    if (n is md.Text) {
      final s = n.text.trim();
      if (s.isEmpty) return null;
      return _gap(_para([n], t.body));
    }
    if (n is! md.Element) return null;
    final kids = n.children ?? const <md.Node>[];
    switch (n.tag) {
      case 'h1' || 'h2' || 'h3' || 'h4' || 'h5' || 'h6':
        final level = int.parse(n.tag.substring(1));
        final style = switch (level) {
          1 => t.title,
          2 => t.heading,
          _ => t.bodyStrong,
        };
        return Padding(
          padding: EdgeInsetsDirectional.only(
            top: level <= 2 ? Space.x6 : Space.x4,
            bottom: Space.x2,
          ),
          child: Semantics(header: true, child: _para(kids, style)),
        );
      case 'p':
        return _gap(_para(kids, t.body));
      case 'hr':
        return const Padding(
          padding: EdgeInsetsDirectional.symmetric(vertical: Space.x4),
          child: DHairline(),
        );
      case 'blockquote':
        return _gap(_quote(n));
      case 'pre':
        return _gap(_code(n));
      case 'ul' || 'ol':
        return _gap(
          _list(n, ordered: n.tag == 'ol', depth: depth),
          below: Space.x2,
        );
      case 'table':
        return _gap(_table(n));
      case 'section':
        // Footnotes.
        return Padding(
          padding: const EdgeInsetsDirectional.only(top: Space.x6),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const DHairline(),
              const SizedBox(height: Space.x3),
              DefaultTextStyle.merge(
                style: t.small.copyWith(color: p.inkMuted),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: blocks(kids, depth: depth),
                ),
              ),
            ],
          ),
        );
      default:
        final inner = blocks(kids, depth: depth);
        if (inner.isEmpty) return _gap(_para([n], t.body));
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: inner,
        );
    }
  }

  // ─────────────── inline ───────────────

  Widget _para(List<md.Node> nodes, TextStyle style, {Widget? trailing}) {
    final plain = nodes.map(plainText).join();
    final dir = directionOf(plain, fallback: Directionality.of(context));
    final spans = <InlineSpan>[];
    for (final n in nodes) {
      _inline(n, style.copyWith(color: style.color ?? p.ink), spans);
    }
    if (trailing != null) {
      spans.add(const TextSpan(text: ' '));
      spans.add(
        WidgetSpan(alignment: PlaceholderAlignment.middle, child: trailing),
      );
    }
    return Text.rich(
      TextSpan(children: spans),
      textDirection: dir,
      textAlign: TextAlign.start,
    );
  }

  void _inline(md.Node n, TextStyle style, List<InlineSpan> out) {
    if (n is md.Text) {
      out.add(TextSpan(text: n.text, style: style));
      return;
    }
    if (n is! md.Element) return;
    final kids = n.children ?? const <md.Node>[];
    void all(TextStyle s) {
      for (final c in kids) {
        _inline(c, s, out);
      }
    }

    switch (n.tag) {
      case 'strong':
        all(style.copyWith(fontWeight: FontWeight.w600));
      case 'em':
        all(style.copyWith(fontStyle: FontStyle.italic));
      case 'del':
        all(style.copyWith(decoration: TextDecoration.lineThrough));
      case 'code':
        out.add(
          TextSpan(
            text: n.textContent,
            style: TypeScale.mono.copyWith(
              fontSize: (style.fontSize ?? 16) * 0.9,
              color: p.ink,
              backgroundColor: p.sunken,
            ),
          ),
        );
      case 'br':
        out.add(const TextSpan(text: '\n'));
      case 'wikilink' || 'a':
        final target = n.tag == 'a'
            ? (n.attributes['href'] ?? '')
            : n.attributes['target']!;
        out.add(
          TextSpan(
            text: n.textContent,
            style: style.copyWith(
              color: p.accent,
              decoration: TextDecoration.underline,
              decorationColor: p.accent.withValues(alpha: 0.35),
            ),
            recognizer: TapGestureRecognizer()..onTap = () => onLink(target),
            semanticsLabel: n.textContent,
          ),
        );
      case 'wikiembed':
        final target = n.attributes['target']!;
        final img = _image(target);
        if (img != null) {
          out.add(WidgetSpan(child: img));
        } else {
          _inline(
            md.Element.text('wikilink', n.textContent)
              ..attributes['target'] = target,
            style,
            out,
          );
        }
      case 'img':
        final img = _image(n.attributes['src'] ?? '');
        if (img != null) out.add(WidgetSpan(child: img));
      case 'blockid':
        break;
      case 'field':
        final key = n.attributes['key']!;
        if (key == 'status') break; // shown as the claim pill
        out.add(
          WidgetSpan(
            alignment: PlaceholderAlignment.middle,
            child: _FieldChip(
              label: key == 'src' ? null : _fieldLabel(key),
              child: Text.rich(
                TextSpan(
                  children: [
                    for (final c in kids)
                      ..._spansOf(c, t.caption.copyWith(color: p.inkMuted)),
                  ],
                ),
              ),
            ),
          ),
        );
      case 'input':
        final checked = n.attributes['checked'] != null;
        out.add(
          WidgetSpan(
            alignment: PlaceholderAlignment.middle,
            child: Padding(
              padding: const EdgeInsetsDirectional.only(end: Space.x2),
              child: _Checkbox(checked: checked),
            ),
          ),
        );
      case 'sup':
        all(style.copyWith(fontSize: (style.fontSize ?? 16) * 0.7));
      default:
        all(style);
    }
  }

  List<InlineSpan> _spansOf(md.Node n, TextStyle s) {
    final out = <InlineSpan>[];
    _inline(n, s, out);
    return out;
  }

  String _fieldLabel(String key) => switch (key) {
    'confidence' => l.fieldConfidence,
    'superseded_by' => l.fieldSupersededBy,
    _ => key,
  };

  Widget? _image(String src) {
    final lower = src.toLowerCase();
    final isImage = [
      '.jpg',
      '.jpeg',
      '.png',
      '.webp',
      '.gif',
    ].any(lower.endsWith);
    if (!isImage ||
        imageRoot == null ||
        src.contains('..') ||
        src.startsWith('/')) {
      return null;
    }
    final file = File(
      '$imageRoot${Platform.pathSeparator}${src.replaceAll('/', Platform.pathSeparator)}',
    );
    return Padding(
      padding: const EdgeInsetsDirectional.symmetric(vertical: Space.x2),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(Radii.small),
        child: Image.file(
          file,
          fit: BoxFit.contain,
          cacheWidth: 1200,
          errorBuilder: (_, _, _) =>
              Text(src, style: t.small.copyWith(color: p.inkMuted)),
        ),
      ),
    );
  }

  // ─────────────── blocks ───────────────

  Widget _quote(md.Element n) {
    final kids = n.children ?? const <md.Node>[];
    // Obsidian callout: first paragraph starts with `[!kind] title`.
    final first = kids.isNotEmpty ? kids.first : null;
    final firstText = first is md.Element ? first.textContent : '';
    final m = RegExp(r'^\[!(\w+)\][+-]?\s*(.*)').firstMatch(firstText);
    if (m != null) {
      final kind = m[1]!.toLowerCase();
      final firstLineTitle = m[2]!.split('\n').first.trim();
      final restOfFirst = firstText.split('\n').skip(1).join('\n').trim();
      final color = switch (kind) {
        'conflict' || 'warning' || 'danger' || 'caution' => p.critical,
        'question' || 'todo' || 'tip' => p.pending,
        _ => p.accent,
      };
      final title = firstLineTitle.isNotEmpty
          ? firstLineTitle
          : _calloutTitle(kind);
      return Semantics(
        container: true,
        label: title,
        child: Container(
          padding: const EdgeInsetsDirectional.fromSTEB(
            Space.x4,
            Space.x3,
            Space.x4,
            Space.x3,
          ),
          decoration: BoxDecoration(
            color: color.withValues(alpha: 0.06),
            borderRadius: BorderRadius.circular(Radii.small),
            border: BorderDirectional(
              start: BorderSide(color: color, width: 3),
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                title,
                style: t.label.copyWith(color: color),
                textDirection: directionOf(
                  title,
                  fallback: Directionality.of(context),
                ),
              ),
              if (restOfFirst.isNotEmpty) ...[
                const SizedBox(height: Space.x1),
                _para([md.Text(restOfFirst)], t.body),
              ],
              ...blocks(kids.skip(1).toList()),
            ],
          ),
        ),
      );
    }
    return Container(
      padding: const EdgeInsetsDirectional.only(start: Space.x4),
      decoration: BoxDecoration(
        border: BorderDirectional(
          start: BorderSide(color: p.hairline, width: 3),
        ),
      ),
      child: DefaultTextStyle.merge(
        style: TextStyle(color: p.inkMuted),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: blocks(kids),
        ),
      ),
    );
  }

  String _calloutTitle(String kind) => switch (kind) {
    'conflict' => l.calloutConflict,
    'warning' || 'caution' || 'danger' => l.calloutWarning,
    _ => l.calloutNote,
  };

  Widget _code(md.Element pre) {
    final codeEl = (pre.children ?? const [])
        .whereType<md.Element>()
        .firstOrNull;
    final source = (codeEl?.textContent ?? pre.textContent).replaceAll(
      RegExp(r'\n$'),
      '',
    );
    final lang = (codeEl?.attributes['class'] ?? '').replaceFirst(
      'language-',
      '',
    );
    // Code is always left-to-right (and scrolls from its start), even inside a Persian page.
    return Directionality(
      textDirection: TextDirection.ltr,
      child: Container(
        decoration: BoxDecoration(
          color: p.sunken,
          borderRadius: BorderRadius.circular(Radii.small),
          border: Border.all(color: p.hairline, width: Stroke.hairline),
        ),
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.all(Space.x3),
          child: Directionality(
            // Code is always left-to-right, even inside a Persian page.
            textDirection: TextDirection.ltr,
            child: Text.rich(
              TextSpan(children: highlight(source, lang, p)),
              style: TypeScale.mono.copyWith(
                color: p.ink,
                fontSize: 13.5,
                height: 1.5,
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _list(md.Element n, {required bool ordered, required int depth}) {
    final items = (n.children ?? const [])
        .whereType<md.Element>()
        .where((e) => e.tag == 'li')
        .toList();
    final start = int.tryParse(n.attributes['start'] ?? '') ?? 1;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var i = 0; i < items.length; i++)
          _item(items[i], ordered ? '${start + i}.' : '•', depth),
      ],
    );
  }

  Widget _item(md.Element li, String marker, int depth) {
    final kids = li.children ?? const <md.Node>[];
    final inline = <md.Node>[];
    final nested = <md.Node>[];
    for (final k in kids) {
      if (k is md.Element &&
          ['ul', 'ol', 'p', 'pre', 'blockquote', 'table'].contains(k.tag)) {
        if (k.tag == 'p' && nested.isEmpty && inline.isEmpty) {
          inline.addAll(k.children ?? const []);
        } else {
          nested.add(k);
        }
      } else {
        inline.add(k);
      }
    }
    final plain = inline.map(plainText).join();
    final dir = directionOf(plain, fallback: Directionality.of(context));
    final status = _claimStatus(inline);
    final task = inline.whereType<md.Element>().any((e) => e.tag == 'input');
    return Directionality(
      textDirection: dir,
      child: Padding(
        padding: EdgeInsetsDirectional.only(
          start: depth * Space.x4,
          bottom: Space.x1 + 2,
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (!task)
              SizedBox(
                width: Space.x6,
                child: Text(
                  marker,
                  style: t.body.copyWith(color: p.inkMuted),
                  textAlign: TextAlign.center,
                ),
              ),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _para(
                    inline,
                    status == ClaimStatus.superseded
                        ? t.body.copyWith(color: p.inkMuted)
                        : t.body,
                    trailing: status == null
                        ? null
                        : DStatusPill(
                            status: status,
                            label: _statusLabel(status),
                          ),
                  ),
                  ...blocks(nested, depth: depth + 1),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  ClaimStatus? _claimStatus(List<md.Node> inline) {
    for (final n in inline) {
      if (n is md.Element &&
          n.tag == 'field' &&
          n.attributes['key'] == 'status') {
        return switch (n.attributes['value']) {
          'confirmed' => ClaimStatus.confirmed,
          'proposed' => ClaimStatus.proposed,
          'superseded' => ClaimStatus.superseded,
          _ => null,
        };
      }
    }
    return null;
  }

  String _statusLabel(ClaimStatus s) => switch (s) {
    ClaimStatus.confirmed => l.statusConfirmed,
    ClaimStatus.proposed => l.statusProposed,
    ClaimStatus.superseded => l.statusSuperseded,
  };

  Widget _table(md.Element table) {
    final rows = <md.Element>[];
    void collect(md.Element e) {
      for (final c in (e.children ?? const []).whereType<md.Element>()) {
        if (c.tag == 'tr') {
          rows.add(c);
        } else {
          collect(c);
        }
      }
    }

    collect(table);
    if (rows.isEmpty) return const SizedBox.shrink();
    final cols = rows
        .map((r) => (r.children ?? const []).whereType<md.Element>().length)
        .reduce((a, b) => a > b ? a : b);
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(color: p.hairline, width: Stroke.hairline),
          borderRadius: BorderRadius.circular(Radii.small),
        ),
        child: Table(
          defaultColumnWidth: const IntrinsicColumnWidth(),
          border: TableBorder(
            horizontalInside: BorderSide(
              color: p.hairline,
              width: Stroke.hairline,
            ),
            verticalInside: BorderSide(
              color: p.hairline,
              width: Stroke.hairline,
            ),
          ),
          children: [
            for (final r in rows)
              TableRow(
                decoration: BoxDecoration(
                  color:
                      (r.children ?? const []).whereType<md.Element>().any(
                        (c) => c.tag == 'th',
                      )
                      ? p.sunken
                      : null,
                ),
                children: [
                  for (var i = 0; i < cols; i++)
                    Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: Space.x3,
                        vertical: Space.x2,
                      ),
                      child: ConstrainedBox(
                        constraints: const BoxConstraints(maxWidth: 320),
                        child: () {
                          final cells = (r.children ?? const [])
                              .whereType<md.Element>()
                              .toList();
                          if (i >= cells.length) return const SizedBox.shrink();
                          final c = cells[i];
                          return _para(
                            c.children ?? const [],
                            c.tag == 'th' ? t.label : t.small,
                          );
                        }(),
                      ),
                    ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}

class _FieldChip extends StatelessWidget {
  const _FieldChip({required this.child, this.label});
  final Widget child;
  final String? label;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      margin: const EdgeInsetsDirectional.only(end: Space.x1),
      padding: const EdgeInsets.symmetric(horizontal: Space.x2, vertical: 1),
      decoration: BoxDecoration(
        color: p.sunken,
        borderRadius: BorderRadius.circular(Radii.pill),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (label != null) ...[
            Text(
              label!,
              style: context.type.caption.copyWith(color: p.inkMuted),
            ),
            const SizedBox(width: Space.x1),
          ],
          Flexible(child: child),
        ],
      ),
    );
  }
}

class _Checkbox extends StatelessWidget {
  const _Checkbox({required this.checked});
  final bool checked;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      width: 16,
      height: 16,
      decoration: BoxDecoration(
        color: checked ? p.accent : null,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: checked ? p.accent : p.inkMuted, width: 1.5),
      ),
      child: checked ? DIcon(DIcons.check, size: 12, color: p.onAccent) : null,
    );
  }
}

const _keywords = {
  'fn',
  'let',
  'mut',
  'pub',
  'use',
  'impl',
  'struct',
  'enum',
  'match',
  'if',
  'else',
  'for',
  'while',
  'return',
  'async',
  'await',
  'const',
  'final',
  'var',
  'class',
  'import',
  'def',
  'function',
  'true',
  'false',
  'null',
  'None',
  'self',
  'this',
  'new',
  'in',
  'of',
  'from',
  'export',
  'type',
  'interface',
};

/// A small, language-agnostic highlighter: comments, strings, numbers and common keywords.
List<TextSpan> highlight(String src, String lang, Palette p) {
  final out = <TextSpan>[];
  final re = RegExp(
    r'(//[^\n]*|#[^\n]*|/\*[\s\S]*?\*/)|("(?:[^"\\\n]|\\.)*"|'
    r"'(?:[^'\\\n]|\\.)*')"
    r'|(\b\d+(?:\.\d+)?\b)|(\b[A-Za-z_]\w*\b)',
  );
  var last = 0;
  final hashComments = [
    'py',
    'python',
    'sh',
    'bash',
    'yaml',
    'toml',
    'rb',
    'ruby',
  ].contains(lang);
  for (final m in re.allMatches(src)) {
    if (m.start > last) out.add(TextSpan(text: src.substring(last, m.start)));
    final s = m[0]!;
    if (m[1] != null && (hashComments || !s.startsWith('#'))) {
      out.add(
        TextSpan(
          text: s,
          style: TextStyle(color: p.inkMuted, fontStyle: FontStyle.italic),
        ),
      );
    } else if (m[2] != null) {
      out.add(
        TextSpan(
          text: s,
          style: TextStyle(color: p.positive),
        ),
      );
    } else if (m[3] != null) {
      out.add(
        TextSpan(
          text: s,
          style: TextStyle(color: p.pending),
        ),
      );
    } else if (m[4] != null && _keywords.contains(s)) {
      out.add(
        TextSpan(
          text: s,
          style: TextStyle(color: p.accent, fontWeight: FontWeight.w500),
        ),
      );
    } else {
      out.add(TextSpan(text: s));
    }
    last = m.end;
  }
  if (last < src.length) out.add(TextSpan(text: src.substring(last)));
  return out;
}
