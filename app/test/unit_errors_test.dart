import 'package:daftar/core/errors.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('backtraces are stripped', () {
    final e = AnyhowException("Couldn't reach the repository.\n\nStack backtrace:\n   0: <unknown>\n");
    expect(humanError(e), "Couldn't reach the repository.");
  });
}
