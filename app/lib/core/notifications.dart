import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../app/identity.dart';

/// Local notifications for finished reflections (§4.7). Nothing leaves the device; the text is
/// already neutral (the core never puts details of a heavy day into a notification).
abstract class SystemNotifications {
  /// Asks the OS for permission where it needs asking (Android 13+, iOS, macOS).
  Future<bool> requestPermission({required String openLabel});

  /// [channel] names the Android channel; [openLabel] is the Linux default action.
  Future<void> show(
    String title,
    String body, {
    required String channel,
    required String openLabel,
  });
}

class LocalSystemNotifications implements SystemNotifications {
  LocalSystemNotifications();

  final _plugin = FlutterLocalNotificationsPlugin();
  Future<bool>? _ready;
  var _next = 0;

  static const _darwin = DarwinInitializationSettings(
    requestAlertPermission: false,
    requestBadgePermission: false,
    requestSoundPermission: false,
  );

  Future<bool> _init(String openLabel) => _ready ??= () async {
    if (kIsWeb) return false;
    final ok = await _plugin.initialize(
      settings: InitializationSettings(
        android: const AndroidInitializationSettings('@mipmap/ic_launcher'),
        iOS: _darwin,
        macOS: _darwin,
        linux: LinuxInitializationSettings(defaultActionName: openLabel),
        windows: const WindowsInitializationSettings(
          appName: AppIdentity.nameEn,
          appUserModelId: 'dev.${AppIdentity.nameEn}.app',
          guid: '5f0c2d8e-3c61-4b7e-9a3f-1e6f0b7d2a41',
        ),
      ),
    );
    return ok ?? false;
  }();

  @override
  Future<bool> requestPermission({required String openLabel}) async {
    if (!await _init(openLabel)) return false;
    if (Platform.isAndroid) {
      return await _plugin
              .resolvePlatformSpecificImplementation<
                AndroidFlutterLocalNotificationsPlugin
              >()
              ?.requestNotificationsPermission() ??
          false;
    }
    if (Platform.isIOS) {
      return await _plugin
              .resolvePlatformSpecificImplementation<
                IOSFlutterLocalNotificationsPlugin
              >()
              ?.requestPermissions(alert: true) ??
          false;
    }
    if (Platform.isMacOS) {
      return await _plugin
              .resolvePlatformSpecificImplementation<
                MacOSFlutterLocalNotificationsPlugin
              >()
              ?.requestPermissions(alert: true) ??
          false;
    }
    return true;
  }

  @override
  Future<void> show(
    String title,
    String body, {
    required String channel,
    required String openLabel,
  }) async {
    if (!await _init(openLabel)) return;
    await _plugin.show(
      id: _next++,
      title: title,
      body: body,
      notificationDetails: NotificationDetails(
        android: AndroidNotificationDetails(
          'reflect',
          channel,
          importance: Importance.defaultImportance,
          priority: Priority.defaultPriority,
        ),
        iOS: const DarwinNotificationDetails(),
        macOS: const DarwinNotificationDetails(),
      ),
    );
  }
}

final systemNotificationsProvider = Provider<SystemNotifications>(
  (ref) => LocalSystemNotifications(),
);
