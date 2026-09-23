import 'package:daftar/core/dates.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';

void main() {
  setUpAll(() => initializeDateFormatting());

  test('gregorian → jalali known dates', () {
    final cases = {
      DateTime(2026, 3, 21): (1405, 1, 1),
      DateTime(2026, 9, 23): (1405, 7, 1),
      DateTime(2025, 3, 20): (1403, 12, 30), // 1403 is a leap year
      DateTime(1979, 2, 11): (1357, 11, 22),
      DateTime(2024, 12, 31): (1403, 10, 11),
    };
    cases.forEach((g, j) {
      final r = JalaliDate.fromDateTime(g);
      expect((r.year, r.month, r.day), j, reason: '$g');
    });
  });

  test('formats per language', () {
    final d = DateTime(2026, 9, 23, 14, 5);
    expect(longDate(d, 'en'), 'Wednesday, 23 September');
    expect(longDate(d, 'fa'), 'چهارشنبه ۱ مهر');
    expect(clockTime(d, 'fa'), '۱۴:۰۵');
  });
}
