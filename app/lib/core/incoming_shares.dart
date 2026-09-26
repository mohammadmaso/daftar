import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// One thing shared into the app from another app: text, an image, or a file to import.
@immutable
class SharedItem {
  const SharedItem.text(String this.text)
    : image = null,
      file = null,
      record = false;
  const SharedItem.image(Uint8List this.image)
    : text = null,
      file = null,
      record = false;

  /// A document or recording, copied to a local path; it opens in the import preview.
  const SharedItem.file(String this.file)
    : text = null,
      image = null,
      record = false;

  /// Not a share: the home-screen widget asking to start recording.
  const SharedItem.record()
    : text = null,
      image = null,
      file = null,
      record = true;
  final String? text;
  final Uint8List? image;
  final String? file;
  final bool record;
}

/// The share sheet's inbox (§8.1). Android delivers through `MainActivity`; iOS through the Share
/// Extension and `DaftarInbox.swift` (an App Group folder). Desktops have no share sheet.
abstract class IncomingShares {
  /// Fires when something new was shared while the app was running.
  Stream<void> get arrived;

  /// Everything shared since the last call.
  Future<List<SharedItem>> take();
}

class PlatformIncomingShares implements IncomingShares {
  PlatformIncomingShares() {
    if (_supported) {
      _channel.setMethodCallHandler((call) async {
        if (call.method == 'arrived') _arrived.add(null);
      });
    }
  }

  static const _channel = MethodChannel('daftar/share');
  static bool get _supported =>
      !kIsWeb && (Platform.isAndroid || Platform.isIOS);
  final _arrived = StreamController<void>.broadcast();

  @override
  Stream<void> get arrived => _arrived.stream;

  @override
  Future<List<SharedItem>> take() async {
    if (!_supported) return const [];
    final raw = await _channel.invokeListMethod<Map<Object?, Object?>>('take');
    return [
      for (final m in raw ?? const <Map<Object?, Object?>>[])
        if (m['text'] case final String t)
          SharedItem.text(t)
        else if (m['image'] case final Uint8List b)
          SharedItem.image(b)
        else if (m['file'] case final String f)
          SharedItem.file(f)
        else if (m['record'] == true)
          const SharedItem.record(),
    ];
  }
}

final incomingSharesProvider = Provider<IncomingShares>(
  (ref) => PlatformIncomingShares(),
);
