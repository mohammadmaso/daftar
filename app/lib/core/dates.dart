import 'package:intl/intl.dart';

/// Solar Hijri (Jalali) calendar conversion — the calendar Persian readers expect.
/// Algorithm: Borkowski / jalaali-js (valid for years 1-3177 AH).
class JalaliDate {
  const JalaliDate(this.year, this.month, this.day);
  final int year;
  final int month;
  final int day;

  static const monthNames = [
    'فروردین', 'اردیبهشت', 'خرداد', 'تیر', 'مرداد', 'شهریور', //
    'مهر', 'آبان', 'آذر', 'دی', 'بهمن', 'اسفند',
  ];

  static JalaliDate fromDateTime(DateTime d) {
    final jdn = _g2d(d.year, d.month, d.day);
    return _d2j(jdn);
  }

  static int _div(int a, int b) => a ~/ b;
  static int _mod(int a, int b) => a - (a ~/ b) * b;

  static int _g2d(int gy, int gm, int gd) {
    var d =
        _div((gy + _div(gm - 8, 6) + 100100) * 1461, 4) +
        _div(153 * _mod(gm + 9, 12) + 2, 5) +
        gd -
        34840408;
    d = d - _div(_div(gy + 100100 + _div(gm - 8, 6), 100) * 3, 4) + 752;
    return d;
  }

  static ({int leap, int gy, int march}) _jalCal(int jy) {
    const breaks = [
      -61, 9, 38, 199, 426, 686, 756, 818, 1111, 1181, 1210, //
      1635, 2060, 2097, 2192, 2262, 2324, 2394, 2456, 3178,
    ];
    final gy = jy + 621;
    var leapJ = -14;
    var jp = breaks[0];
    var jump = 0;
    for (var i = 1; i < breaks.length; i++) {
      final jm = breaks[i];
      jump = jm - jp;
      if (jy < jm) break;
      leapJ = leapJ + _div(jump, 33) * 8 + _div(_mod(jump, 33), 4);
      jp = jm;
    }
    var n = jy - jp;
    leapJ = leapJ + _div(n, 33) * 8 + _div(_mod(n, 33) + 3, 4);
    if (_mod(jump, 33) == 4 && jump - n == 4) leapJ += 1;
    final leapG = _div(gy, 4) - _div((_div(gy, 100) + 1) * 3, 4) - 150;
    final march = 20 + leapJ - leapG;
    if (jump - n < 6) n = n - jump + _div(jump + 4, 33) * 33;
    var leap = _mod(_mod(n + 1, 33) - 1, 4);
    if (leap == -1) leap = 4;
    return (leap: leap, gy: gy, march: march);
  }

  static JalaliDate _d2j(int jdn) {
    final gy = _d2gYear(jdn);
    var jy = gy - 621;
    final r = _jalCal(jy);
    final jdn1f = _g2d(gy, 3, r.march);
    var k = jdn - jdn1f;
    if (k >= 0) {
      if (k <= 185) {
        return JalaliDate(jy, 1 + _div(k, 31), _mod(k, 31) + 1);
      }
      k -= 186;
    } else {
      jy -= 1;
      k += 179;
      if (r.leap == 1) k += 1;
    }
    return JalaliDate(jy, 7 + _div(k, 30), _mod(k, 30) + 1);
  }

  static int _d2gYear(int jdn) {
    var j = 4 * jdn + 139361631;
    j = j + _div(_div(4 * jdn + 183187720, 146097) * 3, 4) * 4 - 3908;
    final i = _div(_mod(j, 1461), 4) * 5 + 308;
    final gm = _mod(_div(i, 153), 12) + 1;
    return _div(j, 1461) - 100100 + _div(8 - gm, 6);
  }
}

String persianDigits(String s) {
  const fa = '۰۱۲۳۴۵۶۷۸۹';
  return s.replaceAllMapped(RegExp('[0-9]'), (m) => fa[int.parse(m[0]!)]);
}

/// "Wednesday, 23 September" / «چهارشنبه ۱ مهر».
String longDate(DateTime d, String languageCode) {
  if (languageCode == 'fa') {
    final j = JalaliDate.fromDateTime(d);
    final weekday = DateFormat.EEEE('fa').format(d);
    return persianDigits(
      '$weekday ${j.day} ${JalaliDate.monthNames[j.month - 1]}',
    );
  }
  return DateFormat('EEEE, d MMMM', languageCode).format(d);
}

/// "14:05" in the UI language's digits.
String clockTime(DateTime d, String languageCode) {
  final s = DateFormat.Hm('en').format(d);
  return languageCode == 'fa' ? persianDigits(s) : s;
}
