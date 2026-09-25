import 'package:daftar/features/wiki/markdown_view.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

/// §12: long pages scroll smoothly because only what is on screen is built and laid out.
void main() {
  const path = 'vaults/life/journal/2026/long.md';
  final body = [
    for (var i = 0; i < 3000; i++) 'Paragraph $i — ${'متن فارسی ' * 4}',
  ].join('\n\n');

  testWidgets('a 3,000-block page builds only what is visible', (tester) async {
    final lib = FakeLibrary()..addPage(path, 'Long', 'بلند', body);
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      location: '/wiki/page?path=${Uri.encodeQueryComponent(path)}',
    );
    final built = find.byType(MarkdownBlock).evaluate().length;
    expect(built, lessThan(60), reason: '$built of 3000 blocks built');
    expect(find.textContaining('Paragraph 2999'), findsNothing);

    await tester.fling(
      find.byType(MarkdownBlock).first,
      const Offset(0, -3000),
      8000,
    );
    await tester.pumpAndSettle();
    expect(find.byType(MarkdownBlock).evaluate().length, lessThan(60));
  });

  test('a body is parsed once while it stays the same', () {
    expect(identical(parseMarkdown(body), parseMarkdown(body)), isTrue);
  });
}
