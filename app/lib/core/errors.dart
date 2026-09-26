import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show AnyhowException;

import 'core_text.dart';

/// One human sentence for an error from the core, in the app's language where known. Stack traces
/// never reach the UI (§9).
String humanError(Object error) {
  final raw = error is AnyhowException ? error.message : error.toString();
  final firstBlock = raw
      .split('\n\nStack backtrace')
      .first
      .split('\nStack backtrace')
      .first;
  return coreText(firstBlock.trim());
}
