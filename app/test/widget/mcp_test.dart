import 'dart:convert';

import 'package:daftar/core/credentials.dart';
import 'package:daftar/core/library_api.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../fakes.dart';
import '../helpers.dart';

McpServer server(String id, McpAuthKind auth) => McpServer(
  id: id,
  name: 'Notes',
  transport: McpTransportKind.streamableHttp,
  target: 'https://mcp.example.com/mcp',
  args: const [],
  envNames: const [],
  auth: auth,
  authNames: const [],
  clientId: '',
  scopes: const [],
  policy: McpPolicy.autoReadOnly,
  enabled: true,
);

void main() {
  testWidgets(
    'add a server with a token: config syncs, the token stays on this device',
    (tester) async {
      final lib = FakeLibrary();
      final creds = MemoryCredentialStore();
      await pumpApp(
        tester,
        setup: FakeSetup(library: lib),
        credentials: creds,
        location: '/settings',
      );
      await scrollTo(tester, find.text('Add server'));
      await tester.tap(find.text('Add server'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(EditableText).at(0), 'Notes');
      await tester.enterText(
        find.byType(EditableText).at(1),
        'https://mcp.example.com/mcp',
      );
      await tester.tap(find.text('Token'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(EditableText).at(2), 'secret-1');
      await tester.ensureVisible(find.text('Save'));
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      final s = lib.mcp.single;
      expect(s.auth, McpAuthKind.bearer);
      expect(s.target, 'https://mcp.example.com/mcp');
      expect(jsonDecode(creds.mcp[s.id]!)['token'], 'secret-1');
      await scrollTo(tester, find.text('Notes'));
      expect(find.text('Connect on this device'), findsOneWidget);
    },
  );

  testWidgets(
    'OAuth sign-in: desktop waits on the loopback, phones take the redirect',
    (tester) async {
      for (final loopback in [true, false]) {
        final lib = FakeLibrary()..mcp.add(server('notes', McpAuthKind.oAuth));
        final creds = MemoryCredentialStore();
        final browser = FakeBrowser(usesLoopback: loopback);
        await pumpApp(
          tester,
          setup: FakeSetup(library: lib),
          credentials: creds,
          browser: browser,
          location: '/settings',
        );
        await scrollTo(tester, find.text('Notes'));
        await tester.tap(find.text('Notes'));
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.text('Sign in'));
        await tester.tap(find.text('Sign in'));
        await tester.pumpAndSettle();
        expect(browser.opened.single, startsWith('https://auth.example.com/'));
        expect(
          lib.oauth.first,
          loopback
              ? 'begin:notes:loopback'
              : 'begin:notes:daftar://oauth/callback',
        );
        expect(
          lib.oauth.last,
          loopback
              ? 'wait:flow-1'
              : startsWith('complete:flow-1:daftar://oauth/callback?code=abc'),
        );
        expect(jsonDecode(creds.mcp['notes']!)['oauth'], isNotNull);
        expect(find.text('Connected on this device.'), findsOneWidget);
        await tester.pump(const Duration(seconds: 3));
      }
    },
  );

  testWidgets('Ask sends enabled servers and asks before a tool runs', (
    tester,
  ) async {
    final lib = FakeLibrary()
      ..mcp.add(server('notes', McpAuthKind.none))
      ..nextApprovals = const [
        ToolApproval(
          requestId: 'r1',
          serverName: 'Notes',
          tool: 'delete_note',
          arguments: '{"id": "7"}',
          readOnly: false,
        ),
      ];
    final api = FakeProviderApi();
    await pumpApp(
      tester,
      setup: FakeSetup(library: lib),
      providerApi: api,
      location: '/ask',
    );
    await tester.enterText(find.byType(EditableText), 'Delete note 7');
    await tester.pump();
    await tester.tap(find.bySemanticsLabel('Send'));
    await tester.pumpAndSettle();
    expect(lib.lastMcp.single.serverId, 'notes');
    expect(find.text('Notes wants to run delete_note'), findsOneWidget);
    await tester.tap(find.text("Don't allow"));
    await tester.pumpAndSettle();
    expect(api.approvals.single, ('r1', false));
    expect(find.text('Notes wants to run delete_note'), findsNothing);
  });
}
