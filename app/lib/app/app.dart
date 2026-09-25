import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../core/library_state.dart';
import '../design/theme.dart';
import '../design/tokens.dart';
import '../features/capture/today_screen.dart';
import '../features/onboarding/onboarding_screen.dart';
import '../features/settings/gallery_screen.dart';
import '../features/settings/settings_screen.dart';
import '../features/shell/home_shell.dart';
import '../features/wiki/editor_screen.dart';
import '../features/wiki/page_screen.dart';
import '../features/wiki/wiki_screen.dart';
import '../l10n/app_localizations.dart';
import 'appearance.dart';
import 'features.dart';
import 'identity.dart';

final routerProvider = Provider<GoRouter>((ref) {
  final refresh = ValueNotifier(0);
  ref.listen(libraryProvider, (_, _) => refresh.value++);
  ref.onDispose(refresh.dispose);
  return GoRouter(
    refreshListenable: refresh,
    redirect: (context, state) {
      final lib = ref.read(libraryProvider);
      final at = state.matchedLocation;
      if (!lib.hasValue) return at == '/start' ? null : '/start';
      final ready = lib.value != null;
      if (!ready) return at == '/setup' ? null : '/setup';
      if (at == '/setup' || at == '/start') return '/';
      return null;
    },
    routes: [
      GoRoute(path: '/start', builder: (_, _) => const _Start()),
      GoRoute(path: '/setup', builder: (_, _) => const OnboardingScreen()),
      ShellRoute(
        builder: (context, state, child) =>
            HomeShell(location: state.matchedLocation, child: child),
        routes: [
          GoRoute(
            path: '/',
            builder: (context, _) => TodayScreen(
              showSettingsButton:
                  MediaQuery.sizeOf(context).width < kWideLayout,
            ),
          ),
          GoRoute(
            path: '/wiki',
            builder: (context, state) =>
                WikiScreen(selected: state.uri.queryParameters['path']),
            routes: [
              GoRoute(
                path: 'page',
                builder: (context, state) =>
                    PageScreen(path: state.uri.queryParameters['path'] ?? ''),
              ),
              GoRoute(
                path: 'edit',
                builder: (context, state) =>
                    EditorScreen(path: state.uri.queryParameters['path'] ?? ''),
              ),
            ],
          ),
          GoRoute(
            path: '/settings',
            builder: (context, _) => SettingsScreen(
              embedded: MediaQuery.sizeOf(context).width >= kWideLayout,
            ),
            routes: [
              if (Features.preview)
                GoRoute(
                  path: 'design',
                  builder: (_, _) => const GalleryScreen(),
                ),
            ],
          ),
        ],
      ),
    ],
  );
});

/// Blank paper while the library opens; shows the reason if it cannot be opened.
class _Start extends ConsumerWidget {
  const _Start();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lib = ref.watch(libraryProvider);
    final p = Theme.of(context).extension<DaftarTheme>()!.palette;
    return ColoredBox(
      color: p.paper,
      child: lib.hasError
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(Space.x8),
                child: Text(
                  '${lib.error}',
                  style: TextStyle(color: p.critical),
                ),
              ),
            )
          : const SizedBox.expand(),
    );
  }
}

class DaftarApp extends ConsumerWidget {
  const DaftarApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final appearance = ref.watch(appearanceProvider);
    return MaterialApp.router(
      debugShowCheckedModeBanner: false,
      onGenerateTitle: (context) =>
          AppIdentity.name(Localizations.localeOf(context)),
      routerConfig: ref.watch(routerProvider),
      locale: appearance.locale,
      supportedLocales: supportedLocales,
      localizationsDelegates: const [
        L10n.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      // Theme depends on the resolved locale (Persian gets its own type scale), which is only
      // known below Localizations, so it is applied here rather than via `theme:`.
      builder: (context, child) {
        final brightness = switch (appearance.theme) {
          ThemePref.system => MediaQuery.platformBrightnessOf(context),
          ThemePref.light => Brightness.light,
          ThemePref.dark => Brightness.dark,
        };
        final script = scriptFor(Localizations.localeOf(context));
        final mq = MediaQuery.of(context);
        final base = mq.textScaler.scale(100) / 100;
        return MediaQuery(
          data: mq.copyWith(
            textScaler: TextScaler.linear(
              (base * appearance.textSize.factor).clamp(0.8, 2.6),
            ),
          ),
          child: AnnotatedRegion<SystemUiOverlayStyle>(
            value: brightness == Brightness.dark
                ? SystemUiOverlayStyle.light
                : SystemUiOverlayStyle.dark,
            child: AnimatedTheme(
              data: buildTheme(brightness, script),
              duration: MediaQuery.maybeDisableAnimationsOf(context) ?? false
                  ? Duration.zero
                  : Motion.slow,
              curve: Motion.ease,
              child: child!,
            ),
          ),
        );
      },
    );
  }
}
