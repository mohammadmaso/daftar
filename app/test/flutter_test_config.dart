import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

/// Loads the bundled fonts so goldens render real glyphs instead of Ahem boxes.
Future<void> testExecutable(FutureOr<void> Function() testMain) async {
  TestWidgetsFlutterBinding.ensureInitialized();
  const families = {
    'Inter': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'Vazirmatn': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'SourceSerif4': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'Literata': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'AtkinsonNext': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'IBMPlexSansArabic': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'NotoNaskhArabic': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'MarkaziText': ['Regular', 'Medium', 'SemiBold', 'Bold'],
    'JetBrainsMono': ['Regular', 'Medium'],
  };
  for (final MapEntry(key: family, value: weights) in families.entries) {
    final loader = FontLoader(family);
    for (final w in weights) {
      final bytes = File('assets/fonts/$family-$w.ttf').readAsBytesSync();
      loader.addFont(Future.value(ByteData.sublistView(bytes)));
    }
    await loader.load();
  }
  // Goldens are rendered on Linux only (CI); other hosts skip comparisons.
  if (!Platform.isLinux) {
    goldenFileComparator = _SkipComparator();
  }
  await testMain();
}

class _SkipComparator extends GoldenFileComparator {
  @override
  Future<bool> compare(Uint8List imageBytes, Uri golden) async => true;
  @override
  Future<void> update(Uri golden, Uint8List imageBytes) async {}
}
