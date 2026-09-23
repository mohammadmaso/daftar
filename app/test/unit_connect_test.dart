import 'package:daftar/features/onboarding/onboarding_screen.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('plain http only for loopback', () {
    expect(isLoopbackHttp('http://127.0.0.1:8765/notes.git'), isTrue);
    expect(isLoopbackHttp('http://localhost/notes.git'), isTrue);
    expect(isLoopbackHttp('http://[::1]:80/n.git'), isTrue);
    expect(isLoopbackHttp('http://github.com/me/notes.git'), isFalse);
    expect(isLoopbackHttp('http://127.0.0.1.evil.com/x.git'), isFalse);
    expect(isLoopbackHttp('https://127.0.0.1/x.git'), isFalse);
  });

  test('form validity per mode', () {
    final f = ConnectForm();
    f.url.text = 'http://github.com/me/notes.git';
    f.token.text = 't';
    expect(f.valid, isFalse);
    f.url.text = 'https://github.com/me/notes.git';
    expect(f.valid, isTrue);
    f.setMode(ConnectMode.ssh);
    f.url.text = 'git@github.com:me/notes.git';
    expect(f.valid, isFalse, reason: 'needs a key first');
  });
}
