import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_web_auth_2/flutter_web_auth_2.dart';
import 'package:url_launcher/url_launcher.dart';

/// Opens an OAuth page in the system browser (§10: never an embedded web view).
abstract class OAuthBrowser {
  /// Desktop uses a loopback redirect the core listens on; phones use `daftar://oauth/callback`.
  bool get usesLoopback;

  /// The redirect URI to register for phones.
  String get mobileRedirect => 'daftar://oauth/callback';

  /// Desktop: opens the page and returns null (the core receives the redirect).
  /// Phones: returns the redirect URL once the user is back (ASWebAuthenticationSession /
  /// Custom Tabs).
  Future<String?> open(String url);
}

class SystemOAuthBrowser implements OAuthBrowser {
  @override
  bool get usesLoopback =>
      !kIsWeb && (Platform.isLinux || Platform.isMacOS || Platform.isWindows);

  @override
  String get mobileRedirect => 'daftar://oauth/callback';

  @override
  Future<String?> open(String url) async {
    if (usesLoopback) {
      await launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
      return null;
    }
    return FlutterWebAuth2.authenticate(url: url, callbackUrlScheme: 'daftar');
  }
}

final oauthBrowserProvider = Provider<OAuthBrowser>(
  (ref) => SystemOAuthBrowser(),
);
