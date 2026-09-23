import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_fa.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of L10n
/// returned by `L10n.of(context)`.
///
/// Applications need to include `L10n.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: L10n.localizationsDelegates,
///   supportedLocales: L10n.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the L10n.supportedLocales
/// property.
abstract class L10n {
  L10n(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static L10n of(BuildContext context) {
    return Localizations.of<L10n>(context, L10n)!;
  }

  static const LocalizationsDelegate<L10n> delegate = _L10nDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('fa'),
  ];

  /// No description provided for @settingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settingsTitle;

  /// No description provided for @appearance.
  ///
  /// In en, this message translates to:
  /// **'Appearance'**
  String get appearance;

  /// No description provided for @language.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get language;

  /// No description provided for @languageSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get languageSystem;

  /// No description provided for @languageEnglish.
  ///
  /// In en, this message translates to:
  /// **'English'**
  String get languageEnglish;

  /// No description provided for @languagePersian.
  ///
  /// In en, this message translates to:
  /// **'فارسی'**
  String get languagePersian;

  /// No description provided for @theme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get theme;

  /// No description provided for @themeSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get themeSystem;

  /// No description provided for @themeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get themeLight;

  /// No description provided for @themeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get themeDark;

  /// No description provided for @textSize.
  ///
  /// In en, this message translates to:
  /// **'Text size'**
  String get textSize;

  /// No description provided for @textSizeFooter.
  ///
  /// In en, this message translates to:
  /// **'Applies on top of your system text size.'**
  String get textSizeFooter;

  /// No description provided for @about.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get about;

  /// No description provided for @version.
  ///
  /// In en, this message translates to:
  /// **'Version'**
  String get version;

  /// No description provided for @coreVersion.
  ///
  /// In en, this message translates to:
  /// **'Core'**
  String get coreVersion;

  /// No description provided for @coreVersionValue.
  ///
  /// In en, this message translates to:
  /// **'{version} · {target}'**
  String coreVersionValue(String version, String target);

  /// No description provided for @repoFormat.
  ///
  /// In en, this message translates to:
  /// **'Repository format'**
  String get repoFormat;

  /// No description provided for @repoFormatValue.
  ///
  /// In en, this message translates to:
  /// **'v{n}'**
  String repoFormatValue(String n);

  /// No description provided for @privacyNote.
  ///
  /// In en, this message translates to:
  /// **'No telemetry. {app} talks only to your Git remote, your AI providers and your MCP servers.'**
  String privacyNote(String app);

  /// No description provided for @designSystem.
  ///
  /// In en, this message translates to:
  /// **'Design system'**
  String get designSystem;

  /// No description provided for @back.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get back;

  /// No description provided for @galleryButtons.
  ///
  /// In en, this message translates to:
  /// **'Buttons'**
  String get galleryButtons;

  /// No description provided for @galleryControls.
  ///
  /// In en, this message translates to:
  /// **'Controls'**
  String get galleryControls;

  /// No description provided for @galleryStatus.
  ///
  /// In en, this message translates to:
  /// **'Claim status'**
  String get galleryStatus;

  /// No description provided for @galleryType.
  ///
  /// In en, this message translates to:
  /// **'Type'**
  String get galleryType;

  /// No description provided for @galleryIcons.
  ///
  /// In en, this message translates to:
  /// **'Icons'**
  String get galleryIcons;

  /// No description provided for @sampleSave.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get sampleSave;

  /// No description provided for @sampleCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get sampleCancel;

  /// No description provided for @sampleRetry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get sampleRetry;

  /// No description provided for @sampleSearch.
  ///
  /// In en, this message translates to:
  /// **'Search your wiki'**
  String get sampleSearch;

  /// No description provided for @statusConfirmed.
  ///
  /// In en, this message translates to:
  /// **'confirmed'**
  String get statusConfirmed;

  /// No description provided for @statusProposed.
  ///
  /// In en, this message translates to:
  /// **'proposed'**
  String get statusProposed;

  /// No description provided for @statusSuperseded.
  ///
  /// In en, this message translates to:
  /// **'superseded'**
  String get statusSuperseded;

  /// No description provided for @sampleNotify.
  ///
  /// In en, this message translates to:
  /// **'Notify me in the evening'**
  String get sampleNotify;

  /// No description provided for @sampleHeading.
  ///
  /// In en, this message translates to:
  /// **'Filed to Life and Health'**
  String get sampleHeading;

  /// No description provided for @sampleBody.
  ///
  /// In en, this message translates to:
  /// **'Slept badly, headache by noon. Sara called about the trip to Isfahan.'**
  String get sampleBody;

  /// No description provided for @sampleMixed.
  ///
  /// In en, this message translates to:
  /// **'جلسه با Sara درباره‌ی project جدید خوب بود.'**
  String get sampleMixed;

  /// No description provided for @welcomeLine.
  ///
  /// In en, this message translates to:
  /// **'A notebook that files itself.'**
  String get welcomeLine;

  /// No description provided for @welcomeBody.
  ///
  /// In en, this message translates to:
  /// **'Talk, type or snap a photo. Everything stays in a private Git repository you own.'**
  String get welcomeBody;

  /// No description provided for @connectRepo.
  ///
  /// In en, this message translates to:
  /// **'Connect a private repository'**
  String get connectRepo;

  /// No description provided for @startLocal.
  ///
  /// In en, this message translates to:
  /// **'Start on this device for now'**
  String get startLocal;

  /// No description provided for @connectTitle.
  ///
  /// In en, this message translates to:
  /// **'Your repository'**
  String get connectTitle;

  /// No description provided for @connectHttps.
  ///
  /// In en, this message translates to:
  /// **'HTTPS + token'**
  String get connectHttps;

  /// No description provided for @connectSsh.
  ///
  /// In en, this message translates to:
  /// **'SSH key'**
  String get connectSsh;

  /// No description provided for @repoUrl.
  ///
  /// In en, this message translates to:
  /// **'Repository address'**
  String get repoUrl;

  /// No description provided for @repoUrlHintHttps.
  ///
  /// In en, this message translates to:
  /// **'https://github.com/you/notes.git'**
  String get repoUrlHintHttps;

  /// No description provided for @repoUrlHintSsh.
  ///
  /// In en, this message translates to:
  /// **'git@github.com:you/notes.git'**
  String get repoUrlHintSsh;

  /// No description provided for @accessToken.
  ///
  /// In en, this message translates to:
  /// **'Access token'**
  String get accessToken;

  /// No description provided for @tokenHelp.
  ///
  /// In en, this message translates to:
  /// **'A fine-grained token with read and write access to this one repository is enough.'**
  String get tokenHelp;

  /// No description provided for @branch.
  ///
  /// In en, this message translates to:
  /// **'Branch'**
  String get branch;

  /// No description provided for @generateKey.
  ///
  /// In en, this message translates to:
  /// **'Create a key for this device'**
  String get generateKey;

  /// No description provided for @publicKey.
  ///
  /// In en, this message translates to:
  /// **'Public key'**
  String get publicKey;

  /// No description provided for @copy.
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get copy;

  /// No description provided for @copied.
  ///
  /// In en, this message translates to:
  /// **'Copied'**
  String get copied;

  /// No description provided for @sshHelp.
  ///
  /// In en, this message translates to:
  /// **'Add this key with write access: GitHub → repository Settings → Deploy keys. GitLab → Settings → Repository → Deploy keys. Gitea → Settings → Deploy Keys.'**
  String get sshHelp;

  /// No description provided for @continueAction.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get continueAction;

  /// No description provided for @deviceTitle.
  ///
  /// In en, this message translates to:
  /// **'Name this device'**
  String get deviceTitle;

  /// No description provided for @deviceHelp.
  ///
  /// In en, this message translates to:
  /// **'Shown in history, so you know where each note came from.'**
  String get deviceHelp;

  /// No description provided for @deviceName.
  ///
  /// In en, this message translates to:
  /// **'Device name'**
  String get deviceName;

  /// No description provided for @connecting.
  ///
  /// In en, this message translates to:
  /// **'Connecting…'**
  String get connecting;

  /// No description provided for @preparing.
  ///
  /// In en, this message translates to:
  /// **'Preparing your notebook…'**
  String get preparing;

  /// No description provided for @tryAgain.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get tryAgain;

  /// No description provided for @todayTitle.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get todayTitle;

  /// No description provided for @emptyToday.
  ///
  /// In en, this message translates to:
  /// **'Nothing captured yet today.'**
  String get emptyToday;

  /// No description provided for @emptyTodayHint.
  ///
  /// In en, this message translates to:
  /// **'Hold the button and talk, or tap it to type.'**
  String get emptyTodayHint;

  /// No description provided for @stageSaved.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get stageSaved;

  /// No description provided for @stageWorking.
  ///
  /// In en, this message translates to:
  /// **'Filing…'**
  String get stageWorking;

  /// No description provided for @stageFailed.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t file'**
  String get stageFailed;

  /// No description provided for @stageFiled.
  ///
  /// In en, this message translates to:
  /// **'Filed'**
  String get stageFiled;

  /// No description provided for @stageExcluded.
  ///
  /// In en, this message translates to:
  /// **'Excluded'**
  String get stageExcluded;

  /// No description provided for @voiceNote.
  ///
  /// In en, this message translates to:
  /// **'Voice note'**
  String get voiceNote;

  /// No description provided for @voicePending.
  ///
  /// In en, this message translates to:
  /// **'Voice note · transcribed when a speech provider is set up'**
  String get voicePending;

  /// No description provided for @photoPending.
  ///
  /// In en, this message translates to:
  /// **'Photo · described when a vision provider is set up'**
  String get photoPending;

  /// No description provided for @captureRecordLabel.
  ///
  /// In en, this message translates to:
  /// **'Hold to record, tap to type'**
  String get captureRecordLabel;

  /// No description provided for @captureCamera.
  ///
  /// In en, this message translates to:
  /// **'Take a photo'**
  String get captureCamera;

  /// No description provided for @pinVault.
  ///
  /// In en, this message translates to:
  /// **'Choose where the next capture goes'**
  String get pinVault;

  /// No description provided for @vaultAuto.
  ///
  /// In en, this message translates to:
  /// **'Auto'**
  String get vaultAuto;

  /// No description provided for @slideToCancel.
  ///
  /// In en, this message translates to:
  /// **'Slide to cancel'**
  String get slideToCancel;

  /// No description provided for @slideUpToLock.
  ///
  /// In en, this message translates to:
  /// **'Slide up to lock'**
  String get slideUpToLock;

  /// No description provided for @recordingCancelled.
  ///
  /// In en, this message translates to:
  /// **'Recording discarded'**
  String get recordingCancelled;

  /// No description provided for @micDenied.
  ///
  /// In en, this message translates to:
  /// **'Microphone access is off. Turn it on in system settings to record.'**
  String get micDenied;

  /// No description provided for @recordingFailed.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t record on this device.'**
  String get recordingFailed;

  /// No description provided for @stop.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get stop;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @typeSomething.
  ///
  /// In en, this message translates to:
  /// **'What\'s on your mind?'**
  String get typeSomething;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @savedTo.
  ///
  /// In en, this message translates to:
  /// **'Saved · {vault}'**
  String savedTo(String vault);

  /// No description provided for @saved.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get saved;

  /// No description provided for @syncSynced.
  ///
  /// In en, this message translates to:
  /// **'Synced'**
  String get syncSynced;

  /// No description provided for @syncLocal.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 change to sync} other{{n} changes to sync}}'**
  String syncLocal(int n);

  /// No description provided for @syncSyncing.
  ///
  /// In en, this message translates to:
  /// **'Syncing…'**
  String get syncSyncing;

  /// No description provided for @syncOffline.
  ///
  /// In en, this message translates to:
  /// **'Offline'**
  String get syncOffline;

  /// No description provided for @syncAttention.
  ///
  /// In en, this message translates to:
  /// **'Needs attention'**
  String get syncAttention;

  /// No description provided for @syncNoRemote.
  ///
  /// In en, this message translates to:
  /// **'On this device only'**
  String get syncNoRemote;

  /// No description provided for @repository.
  ///
  /// In en, this message translates to:
  /// **'Repository'**
  String get repository;

  /// No description provided for @remote.
  ///
  /// In en, this message translates to:
  /// **'Remote'**
  String get remote;

  /// No description provided for @notConnected.
  ///
  /// In en, this message translates to:
  /// **'Not connected'**
  String get notConnected;

  /// No description provided for @connect.
  ///
  /// In en, this message translates to:
  /// **'Connect…'**
  String get connect;

  /// No description provided for @device.
  ///
  /// In en, this message translates to:
  /// **'This device'**
  String get device;

  /// No description provided for @folder.
  ///
  /// In en, this message translates to:
  /// **'Folder'**
  String get folder;

  /// No description provided for @syncNow.
  ///
  /// In en, this message translates to:
  /// **'Sync now'**
  String get syncNow;

  /// No description provided for @lastSynced.
  ///
  /// In en, this message translates to:
  /// **'Last synced {time}'**
  String lastSynced(String time);

  /// No description provided for @obsidianNote.
  ///
  /// In en, this message translates to:
  /// **'This folder is a plain Markdown vault; open it in Obsidian any time.'**
  String get obsidianNote;
}

class _L10nDelegate extends LocalizationsDelegate<L10n> {
  const _L10nDelegate();

  @override
  Future<L10n> load(Locale locale) {
    return SynchronousFuture<L10n>(lookupL10n(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'fa'].contains(locale.languageCode);

  @override
  bool shouldReload(_L10nDelegate old) => false;
}

L10n lookupL10n(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return L10nEn();
    case 'fa':
      return L10nFa();
  }

  throw FlutterError(
    'L10n.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
