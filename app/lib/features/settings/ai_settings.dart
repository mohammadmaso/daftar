import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart' show NumberFormat;

import '../../core/credentials.dart';
import '../../core/errors.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// Settings › Providers and Settings › Models (§8.6, §9).
class AiSettingsSections extends ConsumerWidget {
  const AiSettingsSections({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(aiSettingsProvider).value;
    if (settings == null) return const SizedBox.shrink();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _ProvidersSection(settings: settings),
        const SizedBox(height: Space.x8),
        _ModelsSection(settings: settings),
      ],
    );
  }
}

String roleName(L10n l, ModelRole r) => switch (r) {
  ModelRole.router => l.roleRouter,
  ModelRole.ingest => l.roleIngest,
  ModelRole.chat => l.roleChat,
  ModelRole.voice => l.roleVoice,
  ModelRole.vision => l.roleVision,
  ModelRole.reflect => l.roleReflect,
  ModelRole.lint => l.roleLint,
  ModelRole.stt => l.roleStt,
  ModelRole.tts => l.roleTts,
  ModelRole.embedding => l.roleEmbedding,
};

String warningText(L10n l, CapabilityWarning w) => switch (w) {
  CapabilityWarning.noVision => l.warnNoVision,
  CapabilityWarning.notSpeechToText => l.warnNotStt,
  CapabilityWarning.notTextToSpeech => l.warnNotTts,
  CapabilityWarning.notEmbedding => l.warnNotEmbedding,
  CapabilityWarning.notConversational => l.warnNotConversational,
};

String kindName(L10n l, ProviderKindDto k) => switch (k) {
  ProviderKindDto.openaiCompatible => l.providerKindOpenai,
  ProviderKindDto.anthropic => l.providerKindAnthropic,
  ProviderKindDto.gemini => l.providerKindGemini,
};

/// Roles grouped the way people think about them: the everyday ones first.
const _roleOrder = [
  ModelRole.chat,
  ModelRole.ingest,
  ModelRole.router,
  ModelRole.stt,
  ModelRole.vision,
  ModelRole.voice,
  ModelRole.tts,
  ModelRole.reflect,
  ModelRole.lint,
  ModelRole.embedding,
];

Future<void> _changed(WidgetRef ref) async {
  ref.read(revisionProvider.notifier).bump();
  ref.read(syncControllerProvider.notifier).changed();
  await ref.read(jobRunnerProvider.notifier).kick();
}

// ─────────────────────────── providers ───────────────────────────

class _ProvidersSection extends ConsumerWidget {
  const _ProvidersSection({required this.settings});
  final AiSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    return DSection(
      title: l.providers,
      footer: l.providersFooter,
      children: [
        for (final p in settings.providers)
          DListRow(
            title: p.name,
            subtitle: [kindName(l, p.kind), ?_host(p.baseUrl)].join(' · '),
            chevron: true,
            onTap: () => editProvider(context, ref, p),
          ),
        DListRow(
          title: l.addProvider,
          leading: DIcon(DIcons.plus, size: 18, color: context.palette.accent),
          onTap: () => editProvider(context, ref, null),
        ),
      ],
    );
  }
}

/// The host of a base URL, for a compact subtitle; `null` for the default endpoint.
String? _host(String url) {
  final h = Uri.tryParse(url)?.host ?? '';
  return h.isEmpty ? null : h;
}

Future<void> editProvider(
  BuildContext context,
  WidgetRef ref,
  AiProvider? existing,
) async {
  final key = existing == null
      ? null
      : await ref.read(credentialStoreProvider).apiKey(existing.id);
  if (!context.mounted) return;
  await showDSheet<void>(
    context,
    builder: (_) => ProviderSheet(existing: existing, hasKey: key != null),
  );
}

class ProviderSheet extends ConsumerStatefulWidget {
  const ProviderSheet({super.key, this.existing, this.hasKey = false});
  final AiProvider? existing;
  final bool hasKey;

  @override
  ConsumerState<ProviderSheet> createState() => _ProviderSheetState();
}

class _ProviderSheetState extends ConsumerState<ProviderSheet> {
  late final _name = TextEditingController(text: widget.existing?.name ?? '');
  late final _url = TextEditingController(
    text: widget.existing?.baseUrl ?? '',
  );
  final _key = TextEditingController();
  late ProviderKindDto _kind =
      widget.existing?.kind ?? ProviderKindDto.openaiCompatible;
  String? _check;
  bool _checkOk = false;
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    _url.dispose();
    _key.dispose();
    super.dispose();
  }

  AiProvider _draft() => AiProvider(
    id: widget.existing?.id ?? '',
    name: _name.text.trim(),
    kind: _kind,
    baseUrl: _url.text.trim(),
    headers: widget.existing?.headers ?? const [],
    timeoutS: widget.existing?.timeoutS ?? 60,
  );

  Future<String?> _keyToUse() async {
    if (_key.text.trim().isNotEmpty) return _key.text.trim();
    final id = widget.existing?.id;
    return id == null ? null : ref.read(credentialStoreProvider).apiKey(id);
  }

  Future<void> _checkConnection() async {
    final l = L10n.of(context);
    setState(() {
      _busy = true;
      _check = null;
    });
    try {
      final models = await ref
          .read(providerApiProvider)
          .listModels(_draft(), await _keyToUse());
      _check = l.modelsFound(models.length);
      _checkOk = true;
    } catch (e) {
      _check = humanError(e);
      _checkOk = false;
    }
    if (mounted) setState(() => _busy = false);
  }

  Future<void> _save() async {
    final l = L10n.of(context);
    if (_name.text.trim().isEmpty) {
      setState(() => _error = l.nameRequired);
      return;
    }
    setState(() => _busy = true);
    try {
      final lib = await ref.read(libraryProvider.future);
      final id = await lib!.saveProvider(_draft());
      if (_key.text.trim().isNotEmpty) {
        await ref.read(credentialStoreProvider).saveApiKey(id, _key.text.trim());
      }
      await _changed(ref);
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      if (mounted) {
        setState(() {
          _busy = false;
          _error = humanError(e);
        });
      }
    }
  }

  Future<void> _remove() async {
    final id = widget.existing!.id;
    final lib = await ref.read(libraryProvider.future);
    await lib!.removeProvider(id);
    await ref.read(credentialStoreProvider).deleteApiKey(id);
    await _changed(ref);
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final api = ref.read(providerApiProvider);
    return SingleChildScrollView(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            widget.existing == null ? l.addProvider : l.editProvider,
            style: context.type.title,
          ),
          const SizedBox(height: Space.x4),
          DSegmented<ProviderKindDto>(
            value: _kind,
            onChanged: (k) => setState(() => _kind = k),
            segments: [
              for (final k in ProviderKindDto.values)
                DSegment(k, kindName(l, k)),
            ],
          ),
          const SizedBox(height: Space.x4),
          _Labeled(
            label: l.providerName,
            child: DTextField(controller: _name, hint: l.providerNameHint),
          ),
          _Labeled(
            label: l.baseUrl,
            child: DTextField(
              controller: _url,
              hint: api.defaultBaseUrl(_kind),
              forceLtr: true,
              keyboardType: TextInputType.url,
            ),
          ),
          _Labeled(
            label: l.apiKey,
            child: DTextField(
              controller: _key,
              obscure: true,
              forceLtr: true,
              hint: widget.hasKey ? l.apiKeyKept : l.apiKeyMissing,
            ),
          ),
          Row(
            children: [
              DButton(
                label: l.checkConnection,
                variant: DButtonVariant.secondary,
                onPressed: _busy ? null : _checkConnection,
              ),
              const SizedBox(width: Space.x3),
              if (_check != null)
                Expanded(
                  child: Text(
                    _check!,
                    style: context.type.small.copyWith(
                      color: _checkOk ? p.positive : p.critical,
                    ),
                  ),
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
          DButton(label: l.save, onPressed: _busy ? null : _save),
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

// ─────────────────────────── models ───────────────────────────

class _ModelsSection extends ConsumerWidget {
  const _ModelsSection({required this.settings});
  final AiSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final byRole = {for (final r in settings.roles) r.role: r};
    String providerName(String? id) =>
        settings.providers.where((x) => x.id == id).firstOrNull?.name ?? '';
    return DSection(
      title: l.models,
      footer: l.modelsFooter,
      children: [
        for (final role in _roleOrder)
          if (byRole[role] case final r?)
            DListRow(
              title: roleName(l, role),
              subtitle: switch (r) {
                RoleSetting(model: null) => l.roleNotSet,
                RoleSetting(inheritedFrom: final from?) => l.roleUses(
                  roleName(l, from),
                ),
                _ => '${providerName(r.providerId)} · ${r.model}',
              },
              trailing: r.warning == null || r.inheritedFrom != null
                  ? null
                  : Semantics(
                      label: warningText(l, r.warning!),
                      child: Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: p.pending,
                          shape: BoxShape.circle,
                        ),
                      ),
                    ),
              chevron: true,
              onTap: () => showDSheet<void>(
                context,
                builder: (_) => RoleSheet(setting: r, settings: settings),
              ),
            ),
      ],
    );
  }
}

class RoleSheet extends ConsumerStatefulWidget {
  const RoleSheet({super.key, required this.setting, required this.settings});
  final RoleSetting setting;
  final AiSettings settings;

  @override
  ConsumerState<RoleSheet> createState() => _RoleSheetState();
}

class _RoleSheetState extends ConsumerState<RoleSheet> {
  late String? _provider = widget.setting.inheritedFrom == null
      ? widget.setting.providerId
      : null;
  late final _model = TextEditingController(
    text: widget.setting.inheritedFrom == null
        ? (widget.setting.model ?? '')
        : '',
  );
  List<String> _models = const [];
  bool _busy = false;
  ProbeOutcome? _probe;
  String? _error;

  @override
  void initState() {
    super.initState();
    _provider ??= widget.settings.providers.firstOrNull?.id;
    _model.addListener(() => setState(() {}));
    _loadModels();
  }

  @override
  void dispose() {
    _model.dispose();
    super.dispose();
  }

  AiProvider? get _p =>
      widget.settings.providers.where((x) => x.id == _provider).firstOrNull;

  Future<void> _loadModels() async {
    final p = _p;
    if (p == null) return;
    try {
      final key = await ref.read(credentialStoreProvider).apiKey(p.id);
      final models = await ref.read(providerApiProvider).listModels(p, key);
      if (mounted && _p?.id == p.id) setState(() => _models = models);
    } catch (_) {
      // The list is a convenience; typing a model name always works.
      if (mounted) setState(() => _models = const []);
    }
  }

  Future<LibraryApi> _lib() async {
    final LibraryApi? lib = await ref.read(libraryProvider.future);
    return lib!;
  }

  /// Saves, then runs the real minimal call for the role.
  Future<void> _test() async {
    if (!await _save(close: false)) return;
    setState(() {
      _busy = true;
      _probe = null;
    });
    final keys = await ref
        .read(credentialStoreProvider)
        .apiKeys(widget.settings.providers.map((p) => p.id));
    try {
      final r = await (await _lib()).testRole(widget.setting.role, keys);
      if (mounted) setState(() => _probe = r);
    } catch (e) {
      if (mounted) setState(() => _error = humanError(e));
    }
    if (mounted) setState(() => _busy = false);
  }

  Future<bool> _save({bool close = true}) async {
    final l = L10n.of(context);
    if (_provider == null) return false;
    if (_model.text.trim().isEmpty) {
      setState(() => _error = l.modelRequired);
      return false;
    }
    setState(() => _error = null);
    try {
      await (await _lib()).setRole(
        widget.setting.role,
        _provider!,
        _model.text.trim(),
      );
      await _changed(ref);
      if (close && mounted) Navigator.of(context).pop();
      return true;
    } catch (e) {
      if (mounted) setState(() => _error = humanError(e));
      return false;
    }
  }

  Future<void> _clear() async {
    await (await _lib()).clearRole(widget.setting.role);
    await _changed(ref);
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final providers = widget.settings.providers;
    final lang = Localizations.localeOf(context).languageCode;
    final warning = ref
        .read(providerApiProvider)
        .capabilityWarning(widget.setting.role, _model.text);
    final query = _model.text.trim().toLowerCase();
    final suggestions = _models
        .where((m) => query.isEmpty || m.toLowerCase().contains(query))
        .where((m) => m != _model.text.trim())
        .take(8)
        .toList();
    final canClear =
        widget.setting.inheritedFrom == null &&
        widget.setting.model != null &&
        widget.setting.role != ModelRole.chat;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(roleName(l, widget.setting.role), style: context.type.title),
          const SizedBox(height: Space.x4),
          if (providers.isEmpty)
            Text(
              l.addProviderFirst,
              style: context.type.body.copyWith(color: p.inkMuted),
            )
          else ...[
            _Labeled(
              label: l.provider,
              child: Wrap(
                spacing: Space.x2,
                children: [
                  for (final x in providers)
                    DChip(
                      label: x.name,
                      selected: x.id == _provider,
                      onTap: () {
                        setState(() {
                          _provider = x.id;
                          _models = const [];
                          _probe = null;
                        });
                        _loadModels();
                      },
                    ),
                ],
              ),
            ),
            _Labeled(
              label: l.model,
              child: DTextField(
                controller: _model,
                hint: l.modelHint,
                forceLtr: true,
                mono: true,
              ),
            ),
            if (suggestions.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(bottom: Space.x3),
                child: Wrap(
                  spacing: Space.x2,
                  children: [
                    for (final m in suggestions)
                      DChip(
                        label: m,
                        selected: false,
                        onTap: () => _model.text = m,
                      ),
                  ],
                ),
              ),
            if (warning != null)
              Padding(
                padding: const EdgeInsets.only(bottom: Space.x3),
                child: Text(
                  warningText(l, warning),
                  style: context.type.small.copyWith(color: p.pending),
                ),
              ),
            Row(
              children: [
                DButton(
                  label: _busy ? l.testing : l.testRole,
                  variant: DButtonVariant.secondary,
                  onPressed: _busy ? null : _test,
                ),
                const SizedBox(width: Space.x3),
                if (_probe case final r?)
                  Expanded(
                    child: Text(
                      r.ok
                          ? l.testWorks(
                              NumberFormat.decimalPattern(lang).format(r.latencyMs),
                              r.detail,
                            )
                          : r.detail,
                      style: context.type.small.copyWith(
                        color: r.ok ? p.positive : p.critical,
                      ),
                    ),
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
            DButton(label: l.save, onPressed: _busy ? null : () => _save()),
            if (canClear) ...[
              const SizedBox(height: Space.x2),
              DButton(
                label: l.useChatModel,
                variant: DButtonVariant.quiet,
                onPressed: _busy ? null : _clear,
              ),
            ],
          ],
        ],
      ),
    );
  }
}

class _Labeled extends StatelessWidget {
  const _Labeled({required this.label, required this.child});
  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: Space.x3),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          label,
          style: context.type.caption.copyWith(color: context.palette.inkMuted),
        ),
        const SizedBox(height: Space.x1),
        child,
      ],
    ),
  );
}
