import 'package:daftar/core/core_text.dart';
import 'package:daftar/l10n/app_localizations.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  final en = lookupL10n(const Locale('en'));
  final fa = lookupL10n(const Locale('fa'));

  test('known core sentences are translated', () {
    expect(
      coreText('OpenAI rejected the API key.', fa),
      'OpenAI کلید API را نپذیرفت.',
    );
    expect(
      coreText(
        'No model is set up for ingest. Choose one in Settings › Models.',
        fa,
      ),
      'برای بایگانی مدلی تنظیم نشده است. در تنظیمات › مدل‌ها یکی را انتخاب کنید.',
    );
    expect(coreText('network unavailable', en), 'Offline');
    expect(
      coreText('The remote rejected the credentials: 401', fa),
      fa.coreAuthFailed,
    );
  });

  test('unknown sentences pass through unchanged', () {
    expect(coreText('Something new.', fa), 'Something new.');
  });
}
