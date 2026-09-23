import 'dart:ui';

/// Product name in one place so the codename can be changed (brief: "Daftar" is a codename).
abstract final class AppIdentity {
  static const nameEn = 'Daftar';
  static const nameFa = 'دفتر';

  static String name(Locale locale) =>
      locale.languageCode == 'fa' ? nameFa : nameEn;
}
