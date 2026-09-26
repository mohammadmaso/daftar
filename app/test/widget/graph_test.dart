import 'package:daftar/features/wiki/graph_layout.dart';
import 'package:daftar/features/wiki/graph_screen.dart';
import 'package:daftar/features/wiki/markdown_view.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';
import '../wiki_fixtures.dart';

void main() {
  testWidgets('the Wiki tab opens the graph of every page', (tester) async {
    await pumpApp(tester, setup: FakeSetup(library: wikiLibrary()));
    await tester.tap(find.text('Wiki').last);
    await tester.pumpAndSettle();
    await tester.tap(find.bySemanticsLabel('Open the graph'));
    await tester.pumpAndSettle();
    expect(find.byType(GraphScreen), findsOneWidget);
    expect(find.text('3 pages · 1 link'), findsOneWidget);

    await tester.tap(find.text('Unlinked pages'));
    await tester.pumpAndSettle();
    expect(
      find.text('2 pages · 1 link'),
      findsOneWidget,
      reason: 'the health profile links to nothing and is hidden',
    );
  });

  testWidgets('finding a page selects it and Open shows it', (tester) async {
    await pumpApp(
      tester,
      setup: FakeSetup(library: wikiLibrary()),
      location: '/wiki/graph',
    );
    await tester.enterText(find.byType(EditableText), 'sar');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sara'));
    await tester.pumpAndSettle();
    expect(find.text('1 link · Tap again to open'), findsOneWidget);
    expect(find.text('Sara summary'), findsOneWidget);

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.byType(MarkdownBlock).first, findsOneWidget);
    expect(find.text('person'), findsOneWidget, reason: 'the page reader');
  });

  testWidgets(
    'a vault filter narrows the graph; tap selects, tap again opens',
    (tester) async {
      await pumpApp(
        tester,
        setup: FakeSetup(library: wikiLibrary()),
        location: '/wiki/graph',
      );
      await tester.tap(find.text('Health'));
      await tester.pumpAndSettle();
      expect(find.text('1 page · No links'), findsOneWidget);

      // One node, framed in the middle of the canvas.
      final centre = tester.getCenter(find.byType(GraphScreen));
      await tester.tapAt(centre);
      await tester.pumpAndSettle();
      expect(find.text('No links · Tap again to open'), findsOneWidget);
      await tester.tapAt(centre);
      await tester.pumpAndSettle();
      expect(find.text('profile'), findsOneWidget, reason: 'the page reader');
    },
  );

  testWidgets('opening from a page centres and selects it', (tester) async {
    await pumpApp(
      tester,
      setup: FakeSetup(library: wikiLibrary()),
      location:
          '/wiki/page?path=${Uri.encodeQueryComponent('vaults/life/people/sara.md')}',
    );
    await tester.tap(find.bySemanticsLabel('Nearby pages'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Open the graph'));
    await tester.pumpAndSettle();
    expect(find.byType(GraphScreen), findsOneWidget);
    expect(find.text('1 link · Tap again to open'), findsOneWidget);
    expect(find.text('Sara summary'), findsOneWidget);
  });

  test('the layout settles, deterministically, with linked pages close', () {
    // Two triangles joined by one bridge, and a page on its own.
    final edges = [(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)];
    List<Offset> run() {
      final g = GraphLayout(count: 7, edges: edges);
      while (!g.settled) {
        g.tick();
      }
      return [for (var i = 0; i < 7; i++) g.position(i)];
    }

    final a = run();
    expect(run(), a, reason: 'same graph, same picture');
    for (final o in a) {
      expect(o.dx.isFinite && o.dy.isFinite, isTrue);
    }
    double d(int i, int j) => (a[i] - a[j]).distance;
    expect(d(0, 1), lessThan(d(0, 4)), reason: 'clusters stay together');
    expect(d(0, 1), greaterThan(10), reason: 'nodes do not collapse');
    expect(d(6, 0), lessThan(400), reason: 'unlinked pages stay in view');
  });

  test('a dragged node stays under the finger', () {
    final g = GraphLayout(count: 3, edges: [(0, 1), (1, 2)]);
    g.settle();
    g.pin(1, const Offset(300, 300));
    for (var i = 0; i < 30; i++) {
      g.tick();
    }
    expect(g.position(1), const Offset(300, 300));
    expect((g.position(0) - g.position(1)).distance, lessThan(120));
    g.unpin();
  });
}
