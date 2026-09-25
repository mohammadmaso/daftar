import 'package:flutter_riverpod/flutter_riverpod.dart';

/// A capture started from outside the capture bar: the command palette, a keyboard shortcut, an
/// app-icon quick action or a home-screen widget (§8.1, §8.2).
enum CaptureRequest { note, record, photo }

final captureRequestProvider =
    NotifierProvider<CaptureRequests, CaptureRequest?>(CaptureRequests.new);

/// Holds one pending request until the capture bar on Today takes it, so a request made while
/// Today is still being opened is not lost.
class CaptureRequests extends Notifier<CaptureRequest?> {
  @override
  CaptureRequest? build() => null;

  void request(CaptureRequest r) => state = r;

  CaptureRequest? take() {
    final r = state;
    state = null;
    return r;
  }
}
