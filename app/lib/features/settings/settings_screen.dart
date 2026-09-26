import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart' show NumberFormat;

import '../../app/appearance.dart';
import '../../app/core.dart';
import '../../app/features.dart';
import '../../app/identity.dart';
import '../../core/dates.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../capture/today_screen.dart' show SyncBadge;
import '../onboarding/onboarding_screen.dart';
import 'ai_settings.dart';
import 'mcp_settings.dart';
import 'reflect_settings.dart';

final repoStatusProvider = FutureProvider<RepoStatus?>((ref) async {
  ref.watch(revisionProvider);
  ref.watch(syncControllerProvider);
  final lib = await ref.watch(libraryProvider.future);
  return lib?.status();
});

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key, this.embedded = false});

  /// In the desktop layout Settings is a pane, not a pushed page.
  final bool embedded;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final locale = Localizations.localeOf(context);
    final appearance = ref.watch(appearanceProvider);
    final notifier = ref.read(appearanceProvider.notifier);
    final info = ref.watch(coreInfoProvider);
    final pct = NumberFormat.percentPattern(locale.toLanguageTag());
    final n = NumberFormat.decimalPattern(locale.toLanguageTag());

    return DPage(
      title: l.settingsTitle,
      onBack: embedded || !context.canPop() ? null : () => context.pop(),
      backLabel: l.back,
      trailing: Monogram(name: AppIdentity.name(locale)),
      children: [
        // §8.6 order: providers, models, repository, appearance, about.
        const AiSettingsSections(),
        const _RepositorySection(),
        const McpSection(),
        const ReflectSection(),
        DSection(
          title: l.appearance,
          footer: l.textSizeFooter,
          children: [
            _Stacked(
              label: l.language,
              child: DSegmented<LanguagePref>(
                value: appearance.language,
                onChanged: notifier.setLanguage,
                segments: [
                  DSegment(LanguagePref.system, l.languageSystem),
                  DSegment(LanguagePref.en, l.languageEnglish),
                  DSegment(LanguagePref.fa, l.languagePersian),
                ],
              ),
            ),
            _Stacked(
              label: l.theme,
              child: DSegmented<ThemePref>(
                value: appearance.theme,
                onChanged: notifier.setTheme,
                segments: [
                  DSegment(
                    ThemePref.system,
                    l.themeSystem,
                    icon: DIcons.device,
                  ),
                  DSegment(ThemePref.light, l.themeLight, icon: DIcons.sun),
                  DSegment(ThemePref.dark, l.themeDark, icon: DIcons.moon),
                ],
              ),
            ),
            _Stacked(
              label: l.textSize,
              child: DSegmented<TextSizePref>(
                value: appearance.textSize,
                onChanged: notifier.setTextSize,
                segments: [
                  for (final s in TextSizePref.values)
                    DSegment(s, pct.format(s.factor)),
                ],
              ),
            ),
          ],
        ),
        DSection(
          title: l.about,
          footer: l.privacyNote(AppIdentity.name(locale)),
          children: [
            DListRow(
              title: l.coreVersion,
              trailing: _Value(l.coreVersionValue(info.version, info.target)),
            ),
            DListRow(
              title: l.repoFormat,
              trailing: _Value(
                l.repoFormatValue(n.format(info.repoSchemaVersion)),
                ltr: false,
              ),
            ),
            if (Features.preview)
              DListRow(
                title: l.designSystem,
                chevron: true,
                onTap: () => context.push('/settings/design'),
              ),
          ],
        ),
      ],
    );
  }
}

class _RepositorySection extends ConsumerWidget {
  const _RepositorySection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    final status = ref.watch(repoStatusProvider).value;
    final sync = ref.watch(syncControllerProvider);
    final lang = Localizations.localeOf(context).languageCode;
    if (status == null) return const SizedBox.shrink();
    return DSection(
      title: l.repository,
      footer: l.obsidianNote,
      children: [
        DListRow(
          title: l.remote,
          subtitle: status.remoteUrl ?? l.notConnected,
          trailing: status.hasRemote
              ? null
              : DButton(
                  label: l.connect,
                  variant: DButtonVariant.secondary,
                  onPressed: () => _connect(context, ref),
                ),
        ),
        DListRow(
          title: l.device,
          trailing: _Value(status.deviceName, ltr: false),
        ),
        DListRow(title: l.branch, trailing: _Value(status.branch)),
        DListRow(
          title: l.rebuildIndex,
          chevron: true,
          onTap: () async {
            final lib = await ref.read(libraryProvider.future);
            final n = await lib?.rebuildIndex() ?? 0;
            ref.read(revisionProvider.notifier).bump();
            if (context.mounted) showNote(context, l.indexRebuilt(n));
          },
        ),
        DListRow(
          title: l.folder,
          subtitle: status.root,
          trailing: DIconButton(
            icon: DIcons.copy,
            semanticLabel: l.copy,
            color: p.inkMuted,
            onPressed: () {
              Clipboard.setData(ClipboardData(text: status.root));
              showNote(context, l.copied);
            },
          ),
        ),
        Padding(
          padding: const EdgeInsets.all(Space.x4),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const SyncBadge(),
                    if (sync.message != null)
                      Padding(
                        padding: const EdgeInsets.only(top: Space.x1),
                        child: Text(
                          sync.message!,
                          style: context.type.small.copyWith(color: p.critical),
                        ),
                      ),
                    if (sync.lastSynced != null)
                      Text(
                        l.lastSynced(clockTime(sync.lastSynced!, lang)),
                        style: context.type.caption.copyWith(color: p.inkMuted),
                      ),
                  ],
                ),
              ),
              if (status.hasRemote)
                DButton(
                  label: l.syncNow,
                  variant: DButtonVariant.secondary,
                  onPressed: sync.indicator == SyncIndicator.syncing
                      ? null
                      : () =>
                            ref.read(syncControllerProvider.notifier).syncNow(),
                ),
            ],
          ),
        ),
      ],
    );
  }

  Future<void> _connect(BuildContext context, WidgetRef ref) async {
    final form = ConnectForm();
    final ok = await showDSheet<bool>(
      context,
      builder: (context) => SingleChildScrollView(
        padding: const EdgeInsets.all(Space.x4),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(L10n.of(context).connectTitle, style: context.type.title),
            const SizedBox(height: Space.x4),
            ConnectFields(form: form),
            const SizedBox(height: Space.x4),
            ListenableBuilder(
              listenable: form,
              builder: (context, _) => DButton(
                label: L10n.of(context).continueAction,
                onPressed: form.valid
                    ? () => Navigator.of(context).pop(true)
                    : null,
              ),
            ),
          ],
        ),
      ),
    );
    if (ok == true) {
      final lib = await ref.read(libraryProvider.future);
      await lib?.setRemote(form.url.text.trim());
      await ref.read(credentialStoreProvider).saveGitAuth(form.auth());
      ref.read(revisionProvider.notifier).bump();
      await ref.read(syncControllerProvider.notifier).syncNow();
    }
    form.dispose();
  }
}

class _Stacked extends StatelessWidget {
  const _Stacked({required this.label, required this.child});
  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(Space.x4, Space.x3, Space.x4, Space.x4),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(label, style: context.type.body),
        const SizedBox(height: Space.x2),
        child,
      ],
    ),
  );
}

class _Value extends StatelessWidget {
  const _Value(this.text, {this.ltr = true});
  final String text;

  /// Technical strings (versions, branches, paths) stay LTR inside Persian UI.
  final bool ltr;

  @override
  Widget build(BuildContext context) => Text(
    text,
    textDirection: ltr ? TextDirection.ltr : null,
    style: context.type.small.copyWith(color: context.palette.inkMuted),
  );
}
