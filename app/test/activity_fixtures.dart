import 'package:daftar/core/library_api.dart';

import 'fakes.dart';

Operation op(String id, OpKind kind, {List<String> vaults = const ['life', 'health'], bool undone = false, String? note}) => Operation(
      opId: id,
      kind: kind,
      summary: 'Filed your voice note to Life and Health: updated Journal · 23 Sep, Sara; 1 claim',
      startedAt: '2026-09-23T10:31:00+03:30',
      device: 'pixel-8',
      vaults: vaults,
      pagesCreated: 1,
      pagesUpdated: 3,
      claimsAdded: 1,
      sources: const ['01RAW'],
      undone: undone,
      note: note,
      models: const ['p1/claude-sonnet-5', 'ingest v1'],
      inputTokens: BigInt.from(12400),
      outputTokens: BigInt.from(900),
      costUsd: 0.051,
      route: const [
        RouteTarget(vault: 'life', reason: 'a personal day note', confidence: 0.92),
        RouteTarget(vault: 'health', reason: 'mentions a headache', confidence: 0.71),
      ],
    );

FakeLibrary activityLibrary() {
  final lib = FakeLibrary();
  lib.ops.addAll([op('01OPB', OpKind.ingest), op('01OPA', OpKind.ingest, vaults: const ['work'], undone: true)]);
  lib.diffs['01OPB'] = const [
    PageDiff(
      path: 'vaults/life/people/sara.md',
      change: PageChange.modified,
      lines: [
        DiffLine(kind: DiffLineKind.context, text: '## Timeline'),
        DiffLine(kind: DiffLineKind.removed, text: '- Lives in Tehran'),
        DiffLine(kind: DiffLineKind.added, text: '- Lives in Shiraz since September'),
      ],
    ),
  ];
  lib.cards.addAll(const [
    ReviewCardDto(
      id: 'card-1',
      kind: CardKind.claim,
      createdAt: '2026-09-23T10:31:00+03:30',
      opId: '01OPB',
      page: 'vaults/health/symptoms-log.md',
      claim: ClaimInfo(id: 'c-1', text: 'Headaches follow short nights', status: 'proposed', confidence: 'low', sources: []),
      vaults: [],
    ),
    ReviewCardDto(
      id: 'card-2',
      kind: CardKind.routing,
      createdAt: '2026-09-23T10:32:00+03:30',
      opId: '01OPB',
      vaults: ['health'],
      rawId: '01RAW',
    ),
  ]);
  return lib;
}
