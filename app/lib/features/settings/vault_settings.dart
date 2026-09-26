import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/errors.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// Every vault, archived ones included (§3.2).
final vaultSettingsProvider = FutureProvider<List<VaultSettings>>((ref) async {
  ref.watch(revisionProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib?.vaultSettings() ?? const [];
});

/// Settings › Vaults: the user's own categories of the wiki. The assistant files by each vault's
/// purpose, so a change here reaches it on the next capture.
class VaultSection extends ConsumerWidget {
  const VaultSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final fa = Localizations.localeOf(context).languageCode == 'fa';
    final vaults =
        ref.watch(vaultSettingsProvider).value ?? const <VaultSettings>[];
    return DSection(
      title: l.vaultsTitle,
      footer: l.vaultsFooter,
      children: [
        for (final v in vaults)
          DListRow(
            title: fa ? v.titleFa : v.titleEn,
            subtitle: v.archived
                ? l.vaultArchived
                : '${l.vaultPages(v.pages)} · ${v.purpose}',
            chevron: true,
            onTap: () => showDSheet<void>(
              context,
              builder: (_) => VaultSheet(existing: v),
            ),
          ),
        DListRow(
          title: l.addVault,
          leading: DIcon(DIcons.plus, size: 18, color: p.accent),
          onTap: () =>
              showDSheet<void>(context, builder: (_) => const VaultSheet()),
        ),
      ],
    );
  }
}

class VaultSheet extends ConsumerStatefulWidget {
  const VaultSheet({super.key, this.existing});
  final VaultSettings? existing;

  @override
  ConsumerState<VaultSheet> createState() => _VaultSheetState();
}

class _VaultSheetState extends ConsumerState<VaultSheet> {
  late final _en = TextEditingController(text: widget.existing?.titleEn);
  late final _fa = TextEditingController(text: widget.existing?.titleFa);
  late final _purpose = TextEditingController(text: widget.existing?.purpose);
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _en.dispose();
    _fa.dispose();
    _purpose.dispose();
    super.dispose();
  }

  /// Runs one change, then refreshes every screen that lists vaults and schedules a sync.
  Future<void> _run(Future<void> Function(LibraryApi lib) change) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      await change(lib);
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      if (mounted) setState(() => _error = humanError(e));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _save() async {
    final l = L10n.of(context);
    final en = _en.text.trim(), fa = _fa.text.trim();
    final purpose = _purpose.text.trim();
    if (en.isEmpty && fa.isEmpty) {
      setState(() => _error = l.vaultNameRequired);
      return;
    }
    if (purpose.isEmpty) {
      setState(() => _error = l.vaultPurposeRequired);
      return;
    }
    final v = widget.existing;
    await _run(
      (lib) => v == null
          ? lib.addVault(titleEn: en, titleFa: fa, purpose: purpose)
          : lib.editVault(v.id, titleEn: en, titleFa: fa, purpose: purpose),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final v = widget.existing;
    Widget field(
      String label,
      TextEditingController c, {
      bool ltr = false,
      String? hint,
      int lines = 1,
    }) => Padding(
      padding: const EdgeInsetsDirectional.only(bottom: Space.x3),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(label, style: context.type.caption.copyWith(color: p.inkMuted)),
          const SizedBox(height: Space.x1),
          DTextField(
            controller: c,
            forceLtr: ltr,
            hint: hint,
            maxLines: lines,
            minLines: 1,
          ),
        ],
      ),
    );
    Widget note(String text) => Padding(
      padding: const EdgeInsetsDirectional.only(top: Space.x3),
      child: Text(text, style: context.type.small.copyWith(color: p.inkMuted)),
    );
    return SingleChildScrollView(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            v == null
                ? l.addVault
                : (Localizations.localeOf(context).languageCode == 'fa'
                      ? v.titleFa
                      : v.titleEn),
            style: context.type.title,
          ),
          const SizedBox(height: Space.x4),
          field(l.vaultNameEn, _en, ltr: true),
          field(l.vaultNameFa, _fa),
          field(l.vaultPurpose, _purpose, hint: l.vaultPurposeHint, lines: 3),
          if (_error != null)
            Text(
              _error!,
              style: context.type.small.copyWith(color: p.critical),
            ),
          const SizedBox(height: Space.x4),
          DButton(label: l.save, onPressed: _busy ? null : _save),
          if (v != null && v.builtin) note(l.vaultBuiltinNote),
          if (v != null && !v.builtin) ...[
            const SizedBox(height: Space.x2),
            DButton(
              label: v.archived ? l.vaultRestore : l.vaultArchive,
              variant: DButtonVariant.secondary,
              onPressed: _busy
                  ? null
                  : () =>
                        _run((lib) => lib.setVaultArchived(v.id, !v.archived)),
            ),
            // Only an empty vault can go; one with pages is archived, never deleted.
            if (v.pages == 0) ...[
              const SizedBox(height: Space.x2),
              DButton(
                label: l.remove,
                variant: DButtonVariant.quiet,
                onPressed: _busy
                    ? null
                    : () => _run((lib) => lib.removeVault(v.id)),
              ),
            ],
            if (!v.archived) note(l.vaultArchiveNote),
          ],
        ],
      ),
    );
  }
}
