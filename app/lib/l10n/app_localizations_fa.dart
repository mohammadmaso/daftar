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
  String get voicePending => 'یادداشت صوتی · در انتظار تبدیل به متن';

  @override
  String get photoPending => 'عکس · در انتظار توصیف';

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
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString تغییر برای همگام‌سازی',
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

  @override
  String get providers => 'سرویس‌دهنده‌ها';

  @override
  String get providersFooter =>
      'کلیدها فقط در حافظهٔ امن همین دستگاه می‌مانند؛ هر دستگاه یک بار آن‌ها را می‌خواهد.';

  @override
  String get addProvider => 'افزودن سرویس‌دهنده';

  @override
  String get editProvider => 'سرویس‌دهنده';

  @override
  String get providerKindOpenai => 'سازگار با OpenAI';

  @override
  String get providerKindAnthropic => 'Anthropic';

  @override
  String get providerKindGemini => 'Gemini';

  @override
  String get providerName => 'نام';

  @override
  String get providerNameHint => 'OpenRouter من';

  @override
  String get baseUrl => 'نشانی';

  @override
  String get apiKey => 'کلید API';

  @override
  String get apiKeyKept =>
      'روی همین دستگاه ذخیره است؛ برای نگه‌داشتن خالی بگذارید';

  @override
  String get apiKeyMissing => 'کلیدی روی این دستگاه نیست';

  @override
  String get checkConnection => 'بررسی';

  @override
  String modelsFound(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'وصل شد · $nString مدل',
      zero: 'وصل شد؛ فهرست مدل ندارد',
    );
    return '$_temp0';
  }

  @override
  String get remove => 'حذف';

  @override
  String get nameRequired => 'برای سرویس‌دهنده نامی بگذارید.';

  @override
  String get models => 'مدل‌ها';

  @override
  String get modelsFooter =>
      'نقش‌هایی که مدل خودشان را ندارند از مدل گفت‌وگو استفاده می‌کنند.';

  @override
  String get roleRouter => 'مسیریابی';

  @override
  String get roleIngest => 'بایگانی';

  @override
  String get roleChat => 'گفت‌وگو';

  @override
  String get roleVoice => 'گفت‌وگوی صوتی';

  @override
  String get roleVision => 'عکس‌ها';

  @override
  String get roleReflect => 'بازتاب';

  @override
  String get roleLint => 'نگهداری';

  @override
  String get roleStt => 'گفتار به متن';

  @override
  String get roleTts => 'متن به گفتار';

  @override
  String get roleEmbedding => 'بردارها (اختیاری)';

  @override
  String get roleNotSet => 'تنظیم نشده';

  @override
  String roleUses(String role) {
    return 'از $role استفاده می‌کند';
  }

  @override
  String get warnNoVision => 'این مدل احتمالاً نمی‌تواند تصویر بخواند.';

  @override
  String get warnNotStt => 'این به مدل گفتار به متن شبیه نیست.';

  @override
  String get warnNotTts => 'این به مدل متن به گفتار شبیه نیست.';

  @override
  String get warnNotEmbedding => 'این به مدل بردارسازی شبیه نیست.';

  @override
  String get warnNotConversational => 'این مدل نمی‌تواند گفت‌وگو کند.';

  @override
  String get provider => 'سرویس‌دهنده';

  @override
  String get model => 'مدل';

  @override
  String get modelHint => 'نام مدل';

  @override
  String get modelRequired => 'مدلی انتخاب یا تایپ کنید.';

  @override
  String get testRole => 'آزمایش';

  @override
  String get testing => 'در حال آزمایش…';

  @override
  String testWorks(String ms, String reply) {
    return 'کار می‌کند · $ms میلی‌ثانیه · $reply';
  }

  @override
  String get useChatModel => 'استفاده از مدل گفت‌وگو';

  @override
  String get addProviderFirst => 'اول یک سرویس‌دهنده اضافه کنید.';

  @override
  String filedTo(String vaults) {
    return 'بایگانی شد در $vaults';
  }

  @override
  String pagesUpdated(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString صفحه به‌روز شد',
    );
    return '$_temp0';
  }

  @override
  String claimsToReview(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString ادعا برای بازبینی',
    );
    return '$_temp0';
  }

  @override
  String get listSeparator => '، ';

  @override
  String filingWaits(String role) {
    return 'بایگانی منتظر مدل $role است. در تنظیمات تنظیمش کنید.';
  }

  @override
  String get retry => 'تلاش دوباره';

  @override
  String get wikiTitle => 'ویکی';

  @override
  String get searchWiki => 'جستجو در ویکی';

  @override
  String get recentPages => 'تازه‌ها';

  @override
  String get pinnedPages => 'سنجاق‌شده';

  @override
  String get allVaults => 'همه';

  @override
  String noResults(String q) {
    return 'چیزی با «$q» پیدا نشد.';
  }

  @override
  String get emptyWiki => 'ویکی با بایگانی یادداشت‌ها پر می‌شود.';

  @override
  String get backlinks => 'پیوند از';

  @override
  String get noBacklinks => 'هنوز صفحه‌ی دیگری به اینجا پیوند نداده.';

  @override
  String get localGraph => 'صفحه‌های نزدیک';

  @override
  String get editPage => 'ویرایش';

  @override
  String sourcesCount(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString منبع',
    );
    return '$_temp0';
  }

  @override
  String pagesCount(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString صفحه',
    );
    return '$_temp0';
  }

  @override
  String updatedOn(String date) {
    return 'به‌روزرسانی $date';
  }

  @override
  String get pageMissing => 'این صفحه هنوز وجود ندارد.';

  @override
  String get pages => 'صفحه‌ها';

  @override
  String get fieldConfidence => 'اطمینان';

  @override
  String get fieldSupersededBy => 'جایگزین با';

  @override
  String get calloutConflict => 'ویرایش در دو دستگاه';

  @override
  String get calloutWarning => 'هشدار';

  @override
  String get calloutNote => 'یادداشت';

  @override
  String get rebuildIndex => 'بازسازی فهرست جستجو';

  @override
  String indexRebuilt(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString صفحه فهرست شد',
    );
    return '$_temp0';
  }

  @override
  String get unsavedChanges => 'تغییرهای ذخیره‌نشده دارید.';

  @override
  String get keepEditing => 'ادامه‌ی ویرایش';

  @override
  String get discard => 'دور بریز';

  @override
  String get openPage => 'باز کن';

  @override
  String get activityTitle => 'فعالیت';

  @override
  String get reviewTitle => 'بازبینی';

  @override
  String toReview(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString مورد برای بازبینی',
    );
    return '$_temp0';
  }

  @override
  String get opUndo => 'یک بایگانی برگردانده شد';

  @override
  String get opCompensate => 'محتوای یک یادداشت برگردانده شد';

  @override
  String get opReview => 'یک مورد بازبینی شد';

  @override
  String get opSaveAnswer => 'در ویکی ذخیره شد';

  @override
  String get opLint => 'ویکی بررسی شد';

  @override
  String get opReflect => 'بازتاب';

  @override
  String get opNothingFiled => 'یادداشت خوانده شد؛ چیزی برای بایگانی نبود';

  @override
  String get undoneTag => 'برگردانده‌شده';

  @override
  String get whyHere => 'چرا اینجا';

  @override
  String confidencePct(String pct) {
    return '$pct اطمینان';
  }

  @override
  String get changes => 'تغییرها';

  @override
  String get rawDiff => 'تفاوت خام';

  @override
  String usageTokens(String tokens) {
    return '$tokens توکن';
  }

  @override
  String costApprox(String cost) {
    return 'حدود $cost';
  }

  @override
  String get undoAction => 'برگرداندن';

  @override
  String get undoUndo => 'لغو برگرداندن';

  @override
  String get moveToVault => 'انتقال به دفتر…';

  @override
  String get rerunWithNote => 'اجرای دوباره با یادداشت…';

  @override
  String get rerunHint => 'مثلاً سارا دخترخاله‌ام است، نه همکارم';

  @override
  String get excludeSource => 'کنار گذاشتن این یادداشت';

  @override
  String get undoneNote => 'برگردانده شد.';

  @override
  String get undoQueued =>
      'تغییرهای بعدی هم‌پوشانی دارند؛ با اتصال به اینترنت با دقت برگردانده می‌شود.';

  @override
  String get refiling => 'دوباره بایگانی می‌شود.';

  @override
  String get noActivity => 'هنوز چیزی بایگانی نشده.';

  @override
  String get reviewEmpty => 'چیزی برای بازبینی نیست.';

  @override
  String get confirm => 'تأیید';

  @override
  String get reject => 'رد';

  @override
  String get dismiss => 'بستن';

  @override
  String get keepFiling => 'درست است';

  @override
  String routingQuestion(String vaults) {
    return 'در $vaults بایگانی شد — درست است؟';
  }

  @override
  String get claimQuestion => 'درست است؟';

  @override
  String replacesClaim(String text) {
    return 'به‌جای: $text';
  }

  @override
  String get conflictCard => 'ویرایش در دو دستگاه';

  @override
  String get conflictHint =>
      'هر دو نسخه در صفحه نگه داشته شده‌اند. ویرایشش کنید و بعد حل‌شده علامت بزنید.';

  @override
  String get resolved => 'حل شد';

  @override
  String get lintCard => 'بررسی ویکی';

  @override
  String get questionCard => 'یک پرسش';

  @override
  String get schemaCard => 'پیشنهاد تغییر قاعده';

  @override
  String get swipeHint => 'برای تأیید به راست و برای رد به چپ بکشید.';

  @override
  String fixSuggestion(String fix) {
    return 'پیشنهاد: $fix';
  }

  @override
  String get askTitle => 'بپرس';

  @override
  String get askHint => 'از یادداشت‌هایت بپرس…';

  @override
  String get askScopeAll => 'همه‌چیز';

  @override
  String storyScope(String name) {
    return 'داستان: $name';
  }

  @override
  String get saveToWiki => 'ذخیره در ویکی';

  @override
  String get savedToWiki => 'ذخیره شد؛ بایگانی می‌شود.';

  @override
  String get saveDraft => 'ذخیره به‌عنوان پیش‌نویس';

  @override
  String get draftSaved => 'پیش‌نویس ذخیره شد.';

  @override
  String get attachPhoto => 'پیوست عکس';

  @override
  String get removePhoto => 'حذف عکس';

  @override
  String get send => 'بفرست';

  @override
  String get talkMode => 'گفت‌وگو';

  @override
  String get thinking => 'در حال خواندن ویکی…';

  @override
  String get askEmpty => 'هر چیزی درباره‌ی آنچه ثبت کرده‌ای بپرس.';

  @override
  String askReading(String page) {
    return 'در حال خواندن: $page';
  }

  @override
  String get talkToSomeone => 'با کسی حرف بزن';

  @override
  String get talkToSomeoneBody =>
      'لازم نیست این را تنها به دوش بکشی. کسی که آموزش دیده همین حالا می‌تواند گوش کند.';

  @override
  String get newConversation => 'گفت‌وگوی تازه';

  @override
  String get voiceListening => 'گوش می‌دهم';

  @override
  String get voiceThinking => 'فکر می‌کنم';

  @override
  String get voiceSpeaking => 'صحبت می‌کنم';

  @override
  String get voiceMuted => 'بی‌صدا';

  @override
  String get mute => 'بی‌صدا کن';

  @override
  String get unmute => 'صدا را باز کن';

  @override
  String get endConversation => 'پایان';

  @override
  String get captions => 'زیرنویس';

  @override
  String get voiceNoted => 'یادداشت شد.';

  @override
  String get voiceSaved => 'گفت‌وگو ذخیره شد؛ بایگانی می‌شود.';

  @override
  String get voiceNeedsMic => 'حالت صوتی به میکروفون نیاز دارد.';

  @override
  String get mcpServers => 'ابزارهای بیرونی (MCP)';

  @override
  String get mcpFooter =>
      'تنظیمات سرورها بین دستگاه‌ها همگام می‌شوند؛ اعتبارنامه‌ها روی هر دستگاه می‌مانند و هر دستگاه یک بار وارد می‌شود.';

  @override
  String get addMcpServer => 'افزودن سرور';

  @override
  String mcpConnected(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'وصل · $nString ابزار',
    );
    return '$_temp0';
  }

  @override
  String get mcpNeedsAuth => 'روی این دستگاه وصل شوید';

  @override
  String get mcpDesktopOnly => 'برنامه‌ی محلی؛ فقط در رایانه';

  @override
  String get mcpDisabled => 'خاموش';

  @override
  String get mcpChecking => 'در حال بررسی…';

  @override
  String get transportHttp => 'Streamable HTTP';

  @override
  String get transportSse => 'HTTP + SSE';

  @override
  String get transportStdio => 'برنامه‌ی محلی';

  @override
  String get stdioMobile => 'برنامه‌های محلی فقط روی رایانه اجرا می‌شوند.';

  @override
  String get command => 'فرمان';

  @override
  String get arguments => 'آرگومان‌ها، هر خط یکی';

  @override
  String get envNames => 'نام متغیرهای محیطی، هر خط یکی';

  @override
  String get authNone => 'هیچ';

  @override
  String get authBearer => 'توکن';

  @override
  String get authHeader => 'کلید در سرآیند';

  @override
  String get authQuery => 'کلید در نشانی';

  @override
  String get authHeaders => 'سرآیندهای دلخواه';

  @override
  String get authOAuth => 'ورود (OAuth)';

  @override
  String get authClient => 'اعتبار کلاینت';

  @override
  String get authLabel => 'دسترسی';

  @override
  String get authName => 'نام سرآیند یا پارامتر';

  @override
  String get headerNames => 'نام سرآیندها، هر خط یکی';

  @override
  String get secretValue => 'توکن یا کلید (فقط این دستگاه)';

  @override
  String get headerValues =>
      'مقدار سرآیندها، «نام: مقدار» در هر خط (فقط این دستگاه)';

  @override
  String get envValues => 'مقدارها، NAME=value در هر خط (فقط این دستگاه)';

  @override
  String get clientId => 'شناسه‌ی کلاینت (خالی: ثبت خودکار)';

  @override
  String get clientSecret => 'رمز کلاینت (فقط این دستگاه)';

  @override
  String get scopesLabel => 'دامنه‌ها، با فاصله';

  @override
  String get policyLabel => 'وقتی ابزاری ممکن است چیزی را تغییر دهد';

  @override
  String get policyAsk => 'همیشه بپرس';

  @override
  String get policyReadOnly => 'بپرس مگر فقط‌خواندنی';

  @override
  String get policyAllow => 'همیشه اجازه بده';

  @override
  String get serverOn => 'روشن';

  @override
  String get signIn => 'ورود';

  @override
  String get signedIn => 'روی این دستگاه وصل شد.';

  @override
  String toolApproval(String server, String tool) {
    return '$server می‌خواهد $tool را اجرا کند';
  }

  @override
  String get allow => 'اجازه بده';

  @override
  String get deny => 'اجازه نده';

  @override
  String get readOnlyTool => 'فقط می‌خواند';

  @override
  String get reflectSection => 'بازتاب';

  @override
  String get reflectFooter =>
      'بازتاب‌ها از یادداشت‌های خودتان نوشته می‌شوند و در دفتر روزانه ثبت می‌شوند. اعلان هیچ‌وقت نمی‌گوید یک روز سخت درباره‌ی چه بود.';

  @override
  String get dailyReflection => 'بازتاب روزانه';

  @override
  String get reflectTime => 'ساعت';

  @override
  String get weeklyReview => 'مرور هفتگی';

  @override
  String get reflectDay => 'روز';

  @override
  String get reflectNotifications => 'اعلان‌ها';

  @override
  String get notificationsBlocked =>
      'اعلان‌های این برنامه در تنظیمات دستگاه خاموش است.';

  @override
  String get helplineCountry => 'کشور خط کمک';

  @override
  String get helplineInternational => 'بین‌المللی';

  @override
  String get countryIR => 'ایران';

  @override
  String get countryUS => 'آمریکا';

  @override
  String get countryGB => 'بریتانیا';

  @override
  String get countryDE => 'آلمان';

  @override
  String get countryCA => 'کانادا';

  @override
  String get checkWikiNow => 'همین حالا ویکی را بررسی کن';

  @override
  String get checkWikiFooter =>
      'پیوندهای شکسته، صفحه‌های بی‌پیوند و یادداشت‌هایی را که ثبت نشده‌اند پیدا می‌کند. بعد از چند بار ثبت، خودش هم اجرا می‌شود.';

  @override
  String get lintClean => 'چیزی برای درست کردن نیست.';

  @override
  String lintFound(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString مورد برای نگاه کردن',
    );
    return '$_temp0';
  }

  @override
  String lintNewCards(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString مورد تازه در مرور',
    );
    return '$_temp0';
  }

  @override
  String get reflectChannel => 'بازتاب‌ها';

  @override
  String get open => 'باز کن';

  @override
  String get close => 'بستن';

  @override
  String get recordVoiceNote => 'ضبط یادداشت صوتی';

  @override
  String get commandPalette => 'فرمان‌ها';

  @override
  String get paletteHint => 'جست‌وجو، باز کردن، ثبت یا پرسش';

  @override
  String get paletteNewNote => 'یادداشت تازه';

  @override
  String get paletteTakePhoto => 'گرفتن عکس';

  @override
  String paletteSaveNote(String text) {
    return 'ذخیره به‌عنوان یادداشت: $text';
  }

  @override
  String paletteAsk(String text) {
    return 'بپرس: $text';
  }

  @override
  String paletteGoTo(String place) {
    return 'برو به $place';
  }

  @override
  String get paletteNoMatch => 'چیزی پیدا نشد.';

  @override
  String get paletteShortcutHint => 'Ctrl+K این را از هر جا باز می‌کند';

  @override
  String get paletteShortcutHintMac => '⌘K این را از هر جا باز می‌کند';

  @override
  String sharedSaved(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString مورد هم‌رسانی‌شده ذخیره شد',
    );
    return '$_temp0';
  }
}
