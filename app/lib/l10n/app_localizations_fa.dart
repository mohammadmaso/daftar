// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Persian (`fa`).
class L10nFa extends L10n {
  L10nFa([String locale = 'fa']) : super(locale);

  @override
  String get settingsTitle => 'تنظیمات';

  @override
  String get appearance => 'ظاهر';

  @override
  String get language => 'زبان';

  @override
  String get languageSystem => 'سیستم';

  @override
  String get languageEnglish => 'English';

  @override
  String get languagePersian => 'فارسی';

  @override
  String get theme => 'پوسته';

  @override
  String get themeSystem => 'سیستم';

  @override
  String get themeLight => 'روشن';

  @override
  String get themeDark => 'تیره';

  @override
  String get textSize => 'اندازه‌ی متن';

  @override
  String get textSizeFooter => 'روی اندازه‌ی متن سیستم اعمال می‌شود.';

  @override
  String get about => 'درباره';

  @override
  String get version => 'نسخه';

  @override
  String get coreVersion => 'هسته';

  @override
  String coreVersionValue(String version, String target) {
    return '$version · $target';
  }

  @override
  String get repoFormat => 'قالب مخزن';

  @override
  String repoFormatValue(String n) {
    return 'نسخه‌ی $n';
  }

  @override
  String privacyNote(String app) {
    return 'بدون ردیابی. $app فقط با مخزن گیت، ارائه‌دهنده‌های هوش مصنوعی و سرورهای MCP شما ارتباط دارد.';
  }

  @override
  String get designSystem => 'سیستم طراحی';

  @override
  String get back => 'بازگشت';

  @override
  String get galleryButtons => 'دکمه‌ها';

  @override
  String get galleryControls => 'کنترل‌ها';

  @override
  String get galleryStatus => 'وضعیت ادعا';

  @override
  String get galleryType => 'حروف';

  @override
  String get galleryIcons => 'نمادها';

  @override
  String get sampleSave => 'ذخیره';

  @override
  String get sampleCancel => 'انصراف';

  @override
  String get sampleRetry => 'دوباره';

  @override
  String get sampleSearch => 'جست‌وجو در ویکی';

  @override
  String get statusConfirmed => 'تأییدشده';

  @override
  String get statusProposed => 'پیشنهادی';

  @override
  String get statusSuperseded => 'جایگزین‌شده';

  @override
  String get sampleNotify => 'عصرها به من یادآوری کن';

  @override
  String get sampleHeading => 'در «زندگی» و «سلامت» ثبت شد';

  @override
  String get sampleBody =>
      'بد خوابیدم، تا ظهر سردرد داشتم. سارا درباره‌ی سفر اصفهان زنگ زد.';

  @override
  String get sampleMixed => 'جلسه با Sara درباره‌ی project جدید خوب بود.';

  @override
  String get welcomeLine => 'دفتری که خودش را مرتب می‌کند.';

  @override
  String get welcomeBody =>
      'حرف بزنید، بنویسید یا عکس بگیرید. همه‌چیز در یک مخزن گیت خصوصی می‌ماند که مال خودتان است.';

  @override
  String get connectRepo => 'اتصال به مخزن خصوصی';

  @override
  String get startLocal => 'فعلاً فقط روی همین دستگاه';

  @override
  String get connectTitle => 'مخزن شما';

  @override
  String get connectHttps => 'HTTPS و توکن';

  @override
  String get connectSsh => 'کلید SSH';

  @override
  String get repoUrl => 'نشانی مخزن';

  @override
  String get repoUrlHintHttps => 'https://github.com/you/notes.git';

  @override
  String get repoUrlHintSsh => 'git@github.com:you/notes.git';

  @override
  String get accessToken => 'توکن دسترسی';

  @override
  String get tokenHelp =>
      'یک توکن با دسترسی خواندن و نوشتن فقط به همین مخزن کافی است.';

  @override
  String get branch => 'شاخه';

  @override
  String get generateKey => 'ساخت کلید برای این دستگاه';

  @override
  String get publicKey => 'کلید عمومی';

  @override
  String get copy => 'رونوشت';

  @override
  String get copied => 'رونوشت شد';

  @override
  String get sshHelp =>
      'این کلید را با دسترسی نوشتن اضافه کنید: گیت‌هاب ← تنظیمات مخزن ← Deploy keys. گیت‌لب ← Settings ← Repository ← Deploy keys. گیتی ← Settings ← Deploy Keys.';

  @override
  String get continueAction => 'ادامه';

  @override
  String get deviceTitle => 'نام این دستگاه';

  @override
  String get deviceHelp =>
      'در تاریخچه نشان داده می‌شود تا بدانید هر یادداشت از کجا آمده.';

  @override
  String get deviceName => 'نام دستگاه';

  @override
  String get connecting => 'در حال اتصال…';

  @override
  String get preparing => 'در حال آماده‌سازی دفتر…';

  @override
  String get tryAgain => 'دوباره امتحان کنید';

  @override
  String get todayTitle => 'امروز';

  @override
  String get emptyToday => 'امروز هنوز چیزی ثبت نشده.';

  @override
  String get emptyTodayHint =>
      'دکمه را نگه دارید و حرف بزنید، یا برای نوشتن بزنید.';

  @override
  String get stageSaved => 'ذخیره شد';

  @override
  String get stageWorking => 'در حال بایگانی…';

  @override
  String get stageFailed => 'بایگانی نشد';

  @override
  String get stageFiled => 'بایگانی شد';

  @override
  String get stageExcluded => 'کنار گذاشته شد';

  @override
  String get voiceNote => 'یادداشت صوتی';

  @override
  String get voicePending =>
      'یادداشت صوتی · پس از تنظیم سرویس گفتار، متن می‌شود';

  @override
  String get photoPending => 'عکس · پس از تنظیم سرویس تصویر، توصیف می‌شود';

  @override
  String get captureRecordLabel => 'برای ضبط نگه دارید، برای نوشتن بزنید';

  @override
  String get captureCamera => 'گرفتن عکس';

  @override
  String get pinVault => 'انتخاب مقصد ثبت بعدی';

  @override
  String get vaultAuto => 'خودکار';

  @override
  String get slideToCancel => 'برای لغو بکشید';

  @override
  String get slideUpToLock => 'برای قفل بالا بکشید';

  @override
  String get recordingCancelled => 'ضبط کنار گذاشته شد';

  @override
  String get micDenied =>
      'دسترسی به میکروفون خاموش است. برای ضبط، آن را در تنظیمات سیستم روشن کنید.';

  @override
  String get recordingFailed => 'ضبط روی این دستگاه ممکن نشد.';

  @override
  String get stop => 'ذخیره';

  @override
  String get cancel => 'انصراف';

  @override
  String get typeSomething => 'چه در ذهن دارید؟';

  @override
  String get save => 'ذخیره';

  @override
  String savedTo(String vault) {
    return 'ذخیره شد · $vault';
  }

  @override
  String get saved => 'ذخیره شد';

  @override
  String get syncSynced => 'همگام';

  @override
  String syncLocal(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n تغییر برای همگام‌سازی',
      one: '۱ تغییر برای همگام‌سازی',
    );
    return '$_temp0';
  }

  @override
  String get syncSyncing => 'در حال همگام‌سازی…';

  @override
  String get syncOffline => 'آفلاین';

  @override
  String get syncAttention => 'نیاز به بررسی';

  @override
  String get syncNoRemote => 'فقط روی این دستگاه';

  @override
  String get repository => 'مخزن';

  @override
  String get remote => 'مخزن دوردست';

  @override
  String get notConnected => 'متصل نیست';

  @override
  String get connect => 'اتصال…';

  @override
  String get device => 'این دستگاه';

  @override
  String get folder => 'پوشه';

  @override
  String get syncNow => 'همگام‌سازی';

  @override
  String lastSynced(String time) {
    return 'آخرین همگام‌سازی $time';
  }

  @override
  String get obsidianNote =>
      'این پوشه یک مجموعه‌ی ساده‌ی مارک‌داون است؛ هر زمان خواستید در Obsidian بازش کنید.';
}
