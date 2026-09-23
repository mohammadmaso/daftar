import 'package:daftar/app/appearance.dart';
import 'package:daftar/design/theme.dart';
import 'package:daftar/design/tokens.dart';
import 'package:daftar/features/settings/gallery_screen.dart';
import 'package:daftar/l10n/app_localizations.dart';
import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';

/// Component gallery goldens; these images also illustrate docs/design/style-guide.md.
void main() {
  for (final brightness in Brightness.values) {
    for (final locale in supportedLocales) {
      final name = '${brightness.name}_${locale.languageCode}';
      testWidgets('gallery · $name', (tester) async {
        tester.view.physicalSize = const Size(390, 1500) * 2;
        tester.view.devicePixelRatio = 2;
        addTearDown(tester.view.reset);
        await tester.pumpWidget(
          MaterialApp(
            debugShowCheckedModeBanner: false,
            locale: locale,
            supportedLocales: supportedLocales,
            localizationsDelegates: const [
              L10n.delegate,
              GlobalMaterialLocalizations.delegate,
              GlobalWidgetsLocalizations.delegate,
              GlobalCupertinoLocalizations.delegate,
            ],
            theme: buildTheme(
              brightness,
              locale.languageCode == 'fa' ? Script.persian : Script.latin,
            ),
            home: const GalleryScreen(),
          ),
        );
        await tester.pumpAndSettle();
        await expectLater(
          find.byType(GalleryScreen),
          matchesGoldenFile('goldens/gallery_$name.png'),
        );
      });
    }
  }
}
