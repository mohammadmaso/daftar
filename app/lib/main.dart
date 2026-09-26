import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'app/app.dart';
import 'app/appearance.dart';
import 'core/background.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final (prefs, _) = await (
    SharedPreferences.getInstance(),
    RustLib.init(),
  ).wait;
  // Best effort: a failure here only means no background sync on this device.
  unawaited(Background.register().catchError((Object _) {}));
  runApp(
    ProviderScope(
      overrides: [sharedPreferencesProvider.overrideWithValue(prefs)],
      child: const DaftarApp(),
    ),
  );
}
