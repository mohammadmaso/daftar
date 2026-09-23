import 'package:daftar/app/appearance.dart';
import 'package:daftar/design/design.dart';
import 'package:daftar/features/settings/settings_screen.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../helpers.dart';

TextDirection _dir(WidgetTester t) =>
    Directionality.of(t.element(find.byType(SettingsScreen)));

Palette _palette(WidgetTester t) =>
    t.element(find.byType(SettingsScreen)).palette;

void main() {
  testWidgets('language switches live and mirrors layout', (tester) async {
    await pumpApp(tester, location: '/settings');
    expect(find.text('Settings'), findsOneWidget);
    expect(_dir(tester), TextDirection.ltr);

    await tester.tap(find.text('فارسی'));
    await tester.pumpAndSettle();

    expect(find.text('تنظیمات'), findsOneWidget);
    expect(_dir(tester), TextDirection.rtl);
    expect(
      tester.element(find.byType(SettingsScreen)).type.family,
      'Vazirmatn',
    );

    await tester.tap(find.text('English'));
    await tester.pumpAndSettle();
    expect(find.text('Settings'), findsOneWidget);
    expect(_dir(tester), TextDirection.ltr);
  });

  testWidgets('system language follows the platform locale', (tester) async {
    await pumpApp(tester, location: '/settings');
    tester.platformDispatcher.localesTestValue = const [Locale('fa', 'IR')];
    await tester.pumpAndSettle();
    expect(_dir(tester), TextDirection.rtl);
  });

  testWidgets('theme switches live', (tester) async {
    await pumpApp(tester, location: '/settings');
    expect(_palette(tester), same(Palette.light));

    await tester.tap(find.text('Dark'));
    await tester.pumpAndSettle();
    expect(_palette(tester), same(Palette.dark));

    // "System" appears in both the language and theme rows; the theme row comes second.
    await tester.tap(find.text('System').last);
    await tester.pumpAndSettle();
    expect(_palette(tester), same(Palette.light));
  });

  testWidgets('system theme follows platform brightness', (tester) async {
    await pumpApp(tester, platformBrightness: Brightness.dark, location: '/settings');
    expect(_palette(tester), same(Palette.dark));
  });

  testWidgets('choices persist across restarts', (tester) async {
    await pumpApp(tester, location: '/settings');
    await tester.tap(find.text('فارسی'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('تیره'));
    await tester.pumpAndSettle();

    final sp = await SharedPreferences.getInstance();
    expect(sp.getString('appearance.language'), LanguagePref.fa.name);
    expect(sp.getString('appearance.theme'), ThemePref.dark.name);

    final saved = {for (final k in sp.getKeys()) k: sp.get(k)!};
    await tester.pumpWidget(const SizedBox());
    await pumpApp(tester, prefs: saved, location: '/settings');
    expect(_dir(tester), TextDirection.rtl);
    expect(_palette(tester), same(Palette.dark));
  });

  for (final lang in ['en', 'fa']) {
    testWidgets('200% text lays out without overflow ($lang)', (tester) async {
      tester.platformDispatcher.textScaleFactorTestValue = 2.0;
      await pumpApp(
        tester,
        prefs: prefsFor(language: lang, theme: 'light'),
      );
      expect(tester.takeException(), isNull);
    });
  }
}
