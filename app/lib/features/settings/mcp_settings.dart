import 'dart:convert';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/errors.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../core/oauth_browser.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

final mcpServersProvider = FutureProvider<List<McpServer>>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib?.mcpServers() ?? const [];
});

/// Connects once with this device's credentials to show status and tools (§10 server list).
final mcpStatusProvider = FutureProvider.family<McpStatus, String>((
  ref,
  id,
) async {
  ref.watch(revisionProvider);
  final lib = (await ref.watch(libraryProvider.future))!;
  final creds = ref.read(credentialStoreProvider);
  final status = await lib.mcpCheck(id, await creds.mcpSecrets(id) ?? '{}');
  if (status.updatedSecrets case final s?) await creds.saveMcpSecrets(id, s);
  return status;
});

/// Settings › Outside tools.
class McpSection extends ConsumerWidget {
  const McpSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final servers = ref.watch(mcpServersProvider).value ?? const <McpServer>[];
    return DSection(
      title: l.mcpServers,
      footer: l.mcpFooter,
      children: [
        for (final s in servers)
          DListRow(
            title: s.name,
            subtitle: _statusLine(l, ref.watch(mcpStatusProvider(s.id)), s),
            chevron: true,
            onTap: () => showDSheet<void>(
              context,
              builder: (_) => McpServerSheet(existing: s),
            ),
          ),
        DListRow(
          title: l.addMcpServer,
          leading: DIcon(DIcons.plus, size: 18, color: p.accent),
          onTap: () =>
              showDSheet<void>(context, builder: (_) => const McpServerSheet()),
        ),
      ],
    );
  }

  static String _statusLine(L10n l, AsyncValue<McpStatus> st, McpServer s) {
    if (!s.enabled) return l.mcpDisabled;
    return st.when(
      loading: () => l.mcpChecking,
      error: (e, _) => humanError(e),
      data: (x) => switch (x.kind) {
        McpStatusKind.connected => l.mcpConnected(x.tools.length),
        McpStatusKind.needsAuth => l.mcpNeedsAuth,
        McpStatusKind.desktopOnly => l.mcpDesktopOnly,
        McpStatusKind.disabled => l.mcpDisabled,
        McpStatusKind.error => x.message ?? '',
      },
    );
  }
}

List<String> _lines(String s) => [
  for (final x in s.split('\n'))
    if (x.trim().isNotEmpty) x.trim(),
];

class McpServerSheet extends ConsumerStatefulWidget {
  const McpServerSheet({super.key, this.existing});
  final McpServer? existing;

  @override
  ConsumerState<McpServerSheet> createState() => _McpServerSheetState();
}

class _McpServerSheetState extends ConsumerState<McpServerSheet> {
  late McpServer _s =
      widget.existing ??
      const McpServer(
        id: '',
        name: '',
        transport: McpTransportKind.streamableHttp,
        target: '',
        args: [],
        envNames: [],
        auth: McpAuthKind.none,
        authNames: [],
        clientId: '',
        scopes: [],
        policy: McpPolicy.autoReadOnly,
        enabled: true,
      );
  late final _name = TextEditingController(text: _s.name);
  late final _target = TextEditingController(text: _s.target);
  late final _args = TextEditingController(text: _s.args.join('\n'));
  late final _envNames = TextEditingController(text: _s.envNames.join('\n'));
  late final _authNames = TextEditingController(text: _s.authNames.join('\n'));
  late final _clientId = TextEditingController(text: _s.clientId);
  late final _scopes = TextEditingController(text: _s.scopes.join(' '));
  final _secret = TextEditingController();
  final _headerValues = TextEditingController();
  final _envValues = TextEditingController();
  final _clientSecret = TextEditingController();
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    for (final c in [
      _name,
      _target,
      _args,
      _envNames,
      _authNames,
      _clientId,
      _scopes,
      _secret,
      _headerValues,
      _envValues,
      _clientSecret,
    ]) {
      c.dispose();
    }
    super.dispose();
  }

  McpServer _draft() => McpServer(
    id: _s.id,
    name: _name.text.trim(),
    transport: _s.transport,
    target: _target.text.trim(),
    args: _lines(_args.text),
    envNames: _lines(_envNames.text),
    auth: _s.auth,
    authNames: _lines(_authNames.text),
    clientId: _clientId.text.trim(),
    scopes: _scopes.text
        .split(RegExp(r'\s+'))
        .where((x) => x.isNotEmpty)
        .toList(),
    policy: _s.policy,
    enabled: _s.enabled,
  );

  /// Merges what was typed into this device's stored credentials for the server.
  Future<String> _secrets(String id) async {
    final creds = ref.read(credentialStoreProvider);
    final m =
        jsonDecode(await creds.mcpSecrets(id) ?? '{}') as Map<String, dynamic>;
    if (_secret.text.trim().isNotEmpty) m['token'] = _secret.text.trim();
    if (_clientSecret.text.trim().isNotEmpty) {
      m['client_secret'] = _clientSecret.text.trim();
    }
    if (_headerValues.text.trim().isNotEmpty) {
      m['headers'] = {
        for (final line in _lines(_headerValues.text))
          if (line.contains(':'))
            line.substring(0, line.indexOf(':')).trim(): line
                .substring(line.indexOf(':') + 1)
                .trim(),
      };
    }
    if (_envValues.text.trim().isNotEmpty) {
      m['env'] = {
        for (final line in _lines(_envValues.text))
          if (line.contains('='))
            line.substring(0, line.indexOf('=')).trim(): line.substring(
              line.indexOf('=') + 1,
            ),
      };
    }
    final json = jsonEncode(m);
    await creds.saveMcpSecrets(id, json);
    return json;
  }

  Future<String?> _save() async {
    final l = L10n.of(context);
    if (_name.text.trim().isEmpty) {
      setState(() => _error = l.nameRequired);
      return null;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      final id = await lib.saveMcpServer(_draft());
      await _secrets(id);
      _s = McpServer(
        id: id,
        name: _draft().name,
        transport: _s.transport,
        target: _draft().target,
        args: _draft().args,
        envNames: _draft().envNames,
        auth: _s.auth,
        authNames: _draft().authNames,
        clientId: _draft().clientId,
        scopes: _draft().scopes,
        policy: _s.policy,
        enabled: _s.enabled,
      );
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      return id;
    } catch (e) {
      if (mounted) setState(() => _error = humanError(e));
      return null;
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _signIn() async {
    final l = L10n.of(context);
    final id = await _save();
    if (id == null) return;
    setState(() => _busy = true);
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      final browser = ref.read(oauthBrowserProvider);
      final creds = ref.read(credentialStoreProvider);
      final secrets = await creds.mcpSecrets(id) ?? '{}';
      final start = await lib.mcpOauthBegin(
        id,
        secrets,
        redirectUri: browser.usesLoopback ? null : browser.mobileRedirect,
      );
      final back = await browser.open(start.authUrl);
      final json = back == null
          ? await lib.mcpOauthWait(start.flowId)
          : await lib.mcpOauthComplete(start.flowId, back);
      await creds.saveMcpSecrets(id, json);
      ref.read(revisionProvider.notifier).bump();
      if (mounted) showNote(context, l.signedIn);
    } catch (e) {
      if (mounted) setState(() => _error = humanError(e));
    }
    if (mounted) setState(() => _busy = false);
  }

  Future<void> _remove() async {
    final lib = (await ref.read(libraryProvider.future))!;
    await lib.removeMcpServer(_s.id);
    await ref.read(credentialStoreProvider).deleteMcpSecrets(_s.id);
    ref.read(revisionProvider.notifier).bump();
    ref.read(syncControllerProvider.notifier).changed();
    if (mounted) Navigator.of(context).pop();
  }

  McpServer _with({
    McpTransportKind? transport,
    McpAuthKind? auth,
    McpPolicy? policy,
    bool? enabled,
  }) => McpServer(
    id: _s.id,
    name: _s.name,
    transport: transport ?? _s.transport,
    target: _s.target,
    args: _s.args,
    envNames: _s.envNames,
    auth: auth ?? _s.auth,
    authNames: _s.authNames,
    clientId: _s.clientId,
    scopes: _s.scopes,
    policy: policy ?? _s.policy,
    enabled: enabled ?? _s.enabled,
  );

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final stdio = ref.read(providerApiProvider).stdioSupported;
    Widget field(
      String label,
      TextEditingController c, {
      bool ltr = true,
      bool secret = false,
      int lines = 1,
    }) => Padding(
      padding: const EdgeInsets.only(bottom: Space.x3),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(label, style: context.type.caption.copyWith(color: p.inkMuted)),
          const SizedBox(height: Space.x1),
          DTextField(
            controller: c,
            forceLtr: ltr,
            obscure: secret,
            maxLines: lines,
            minLines: 1,
          ),
        ],
      ),
    );
    Widget chips<T>(
      String label,
      T value,
      List<(T, String)> options,
      ValueChanged<T> onPick,
    ) => Padding(
      padding: const EdgeInsets.only(bottom: Space.x3),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(label, style: context.type.caption.copyWith(color: p.inkMuted)),
          const SizedBox(height: Space.x1),
          Wrap(
            spacing: Space.x2,
            runSpacing: Space.x2,
            children: [
              for (final (v, name) in options)
                DChip(
                  label: name,
                  selected: v == value,
                  onTap: () => setState(() => onPick(v)),
                ),
            ],
          ),
        ],
      ),
    );
    final isStdio = _s.transport == McpTransportKind.stdio;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            widget.existing == null ? l.addMcpServer : _s.name,
            style: context.type.title,
          ),
          const SizedBox(height: Space.x4),
          field(l.providerName, _name, ltr: false),
          chips<McpTransportKind>('', _s.transport, [
            (McpTransportKind.streamableHttp, l.transportHttp),
            (McpTransportKind.sse, l.transportSse),
            (McpTransportKind.stdio, l.transportStdio),
          ], (v) => _s = _with(transport: v)),
          if (isStdio && !stdio)
            Padding(
              padding: const EdgeInsets.only(bottom: Space.x3),
              child: Text(
                l.stdioMobile,
                style: context.type.small.copyWith(color: p.pending),
              ),
            ),
          field(isStdio ? l.command : l.baseUrl, _target),
          if (isStdio) ...[
            field(l.arguments, _args, lines: 4),
            field(l.envNames, _envNames, lines: 3),
            field(l.envValues, _envValues, secret: false, lines: 3),
          ] else ...[
            chips<McpAuthKind>(l.authLabel, _s.auth, [
              (McpAuthKind.none, l.authNone),
              (McpAuthKind.oAuth, l.authOAuth),
              (McpAuthKind.bearer, l.authBearer),
              (McpAuthKind.apiKeyHeader, l.authHeader),
              (McpAuthKind.apiKeyQuery, l.authQuery),
              (McpAuthKind.headers, l.authHeaders),
              (McpAuthKind.clientCredentials, l.authClient),
            ], (v) => _s = _with(auth: v)),
            switch (_s.auth) {
              McpAuthKind.bearer => field(l.secretValue, _secret, secret: true),
              McpAuthKind.apiKeyHeader || McpAuthKind.apiKeyQuery => Column(
                children: [
                  field(l.authName, _authNames),
                  field(l.secretValue, _secret, secret: true),
                ],
              ),
              McpAuthKind.headers => Column(
                children: [
                  field(l.headerNames, _authNames, lines: 3),
                  field(l.headerValues, _headerValues, lines: 3),
                ],
              ),
              McpAuthKind.oAuth => Column(
                children: [
                  field(l.clientId, _clientId),
                  field(l.clientSecret, _clientSecret, secret: true),
                  field(l.scopesLabel, _scopes),
                ],
              ),
              McpAuthKind.clientCredentials => Column(
                children: [
                  field(l.clientId, _clientId),
                  field(l.clientSecret, _clientSecret, secret: true),
                  field(l.scopesLabel, _scopes),
                ],
              ),
              McpAuthKind.none => const SizedBox.shrink(),
            },
          ],
          chips<McpPolicy>(l.policyLabel, _s.policy, [
            (McpPolicy.askEveryTime, l.policyAsk),
            (McpPolicy.autoReadOnly, l.policyReadOnly),
            (McpPolicy.alwaysAllow, l.policyAllow),
          ], (v) => _s = _with(policy: v)),
          Row(
            children: [
              Expanded(child: Text(l.serverOn, style: context.type.body)),
              DSwitch(
                value: _s.enabled,
                onChanged: (v) => setState(() => _s = _with(enabled: v)),
                semanticLabel: l.serverOn,
              ),
            ],
          ),
          if (_error != null) ...[
            const SizedBox(height: Space.x3),
            Text(
              _error!,
              style: context.type.small.copyWith(color: p.critical),
            ),
          ],
          const SizedBox(height: Space.x6),
          DButton(
            label: l.save,
            onPressed: _busy
                ? null
                : () async {
                    if (await _save() != null && context.mounted) {
                      Navigator.of(context).pop();
                    }
                  },
          ),
          if (_s.auth == McpAuthKind.oAuth && !isStdio) ...[
            const SizedBox(height: Space.x2),
            DButton(
              label: l.signIn,
              variant: DButtonVariant.secondary,
              onPressed: _busy ? null : _signIn,
            ),
          ],
          if (widget.existing != null) ...[
            const SizedBox(height: Space.x2),
            DButton(
              label: l.remove,
              variant: DButtonVariant.quiet,
              onPressed: _busy ? null : _remove,
            ),
          ],
        ],
      ),
    );
  }
}
