import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/dates.dart';
import '../../core/errors.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../core/notifications.dart';
import '../../core/reflect_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// Countries with curated helplines in the core (§4.7); "" is the international list.
const helplineCountries = ['', 'IR', 'US', 'GB', 'DE', 'CA'];

String countryName(L10n l, String code) => switch (code) {
  'IR' => l.countryIR,
  'US' => l.countryUS,
  'GB' => l.countryGB,
  'DE' => l.countryDE,
  'CA' => l.countryCA,
  _ => l.helplineInternational,
};

/// Evening times offered for the daily reflection, every half hour.
final reflectTimes = [
  for (var m = 17 * 60; m < 24 * 60; m += 30)
    '${(m ~/ 60).toString().padLeft(2, '0')}:${(m % 60).toString().padLeft(2, '0')}',
];

/// Settings › Reflect (§4.6, §4.7) and the on-demand wiki check (§6.5).
class ReflectSection extends ConsumerWidget {
  const ReflectSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final lang = Localizations.localeOf(context).languageCode;
    final prefs = ref.watch(reflectPrefsProvider).value;
    if (prefs == null) return const SizedBox.shrink();

    Future<void> save(ReflectPrefs next) async {
      final lib = await ref.read(libraryProvider.future);
      try {
        await lib?.setReflectPrefs(next);
      } catch (e) {
        if (context.mounted) showNote(context, humanError(e));
        return;
      }
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
    }

    ReflectPrefs copy({
      bool? daily,
      String? dailyTime,
      bool? weekly,
      int? weeklyDay,
      bool? notifications,
      String? helplineCountry,
    }) => ReflectPrefs(
      daily: daily ?? prefs.daily,
      dailyTime: dailyTime ?? prefs.dailyTime,
      weekly: weekly ?? prefs.weekly,
      weeklyDay: weeklyDay ?? prefs.weeklyDay,
      notifications: notifications ?? prefs.notifications,
      helplineCountry: helplineCountry ?? prefs.helplineCountry,
    );

    final time = DateTime(
      2000,
      1,
      1,
      int.tryParse(prefs.dailyTime.split(':').first) ?? 21,
      int.tryParse(prefs.dailyTime.split(':').last) ?? 30,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DSection(
          title: l.reflectSection,
          footer: l.reflectFooter,
          children: [
            DListRow(
              title: l.dailyReflection,
              trailing: DSwitch(
                value: prefs.daily,
                semanticLabel: l.dailyReflection,
                onChanged: (v) => save(copy(daily: v)),
              ),
            ),
            if (prefs.daily)
              DListRow(
                title: l.reflectTime,
                trailing: _Value(clockTime(time, lang)),
                chevron: true,
                onTap: () async {
                  final t = await _pick(
                    context,
                    title: l.reflectTime,
                    options: [
                      for (final t in reflectTimes)
                        (
                          t,
                          clockTime(
                            DateTime(
                              2000,
                              1,
                              1,
                              int.parse(t.substring(0, 2)),
                              int.parse(t.substring(3)),
                            ),
                            lang,
                          ),
                        ),
                    ],
                    selected: prefs.dailyTime,
                  );
                  if (t != null) await save(copy(dailyTime: t));
                },
              ),
            DListRow(
              title: l.weeklyReview,
              trailing: DSwitch(
                value: prefs.weekly,
                semanticLabel: l.weeklyReview,
                onChanged: (v) => save(copy(weekly: v)),
              ),
            ),
            if (prefs.weekly)
              DListRow(
                title: l.reflectDay,
                trailing: _Value(weekdayName(prefs.weeklyDay, lang)),
                chevron: true,
                onTap: () async {
                  final d = await _pick(
                    context,
                    title: l.reflectDay,
                    options: [
                      for (var d = 1; d <= 7; d++) ('$d', weekdayName(d, lang)),
                    ],
                    selected: '${prefs.weeklyDay}',
                  );
                  if (d != null) await save(copy(weeklyDay: int.parse(d)));
                },
              ),
            DListRow(
              title: l.reflectNotifications,
              trailing: DSwitch(
                value: prefs.notifications,
                semanticLabel: l.reflectNotifications,
                onChanged: (v) async {
                  await save(copy(notifications: v));
                  if (!v) return;
                  final granted = await ref
                      .read(systemNotificationsProvider)
                      .requestPermission(openLabel: l.open);
                  if (!granted && context.mounted) {
                    showNote(context, l.notificationsBlocked);
                  }
                },
              ),
            ),
            DListRow(
              title: l.helplineCountry,
              trailing: _Value(
                countryName(
                  l,
                  ref.watch(helplineCountryProvider).value ??
                      (lang == 'fa' ? 'IR' : ''),
                ),
              ),
              chevron: true,
              onTap: () async {
                final c = await _pick(
                  context,
                  title: l.helplineCountry,
                  options: [
                    for (final c in helplineCountries) (c, countryName(l, c)),
                  ],
                  selected: prefs.helplineCountry,
                );
                if (c != null) await save(copy(helplineCountry: c));
              },
            ),
          ],
        ),
        DSection(
          footer: l.checkWikiFooter,
          children: [
            DListRow(
              title: l.checkWikiNow,
              chevron: true,
              onTap: () async {
                final lib = await ref.read(libraryProvider.future);
                if (lib == null) return;
                try {
                  final r = await lib.lintNow();
                  ref.read(revisionProvider.notifier).bump();
                  ref.read(syncControllerProvider.notifier).changed();
                  if (!context.mounted) return;
                  showNote(
                    context,
                    r.findings == 0
                        ? l.lintClean
                        : [
                            l.lintFound(r.findings),
                            if (r.newCards > 0) l.lintNewCards(r.newCards),
                          ].join(' · '),
                  );
                  // Model checks of the same pass run with the job queue.
                  ref.read(jobRunnerProvider.notifier).kick();
                } catch (e) {
                  if (context.mounted) showNote(context, humanError(e));
                }
              },
            ),
          ],
        ),
      ],
    );
  }

  /// A short list in a sheet; returns the chosen value.
  Future<String?> _pick(
    BuildContext context, {
    required String title,
    required List<(String, String)> options,
    required String selected,
  }) => showDSheet<String>(
    context,
    builder: (context) {
      final p = context.palette;
      return SingleChildScrollView(
        padding: const EdgeInsetsDirectional.symmetric(vertical: Space.x4),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsetsDirectional.fromSTEB(
                Space.x4,
                0,
                Space.x4,
                Space.x2,
              ),
              child: Text(title, style: context.type.title),
            ),
            for (final (value, label) in options)
              DListRow(
                title: label,
                trailing: value == selected
                    ? DIcon(DIcons.check, size: 18, color: p.accent)
                    : null,
                onTap: () => Navigator.of(context).pop(value),
              ),
          ],
        ),
      );
    },
  );
}

class _Value extends StatelessWidget {
  const _Value(this.text);
  final String text;

  @override
  Widget build(BuildContext context) => Text(
    text,
    style: context.type.small.copyWith(color: context.palette.inkMuted),
  );
}
