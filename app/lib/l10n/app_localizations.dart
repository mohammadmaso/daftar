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
  /// **'Voice note · waiting to be transcribed'**
  String get voicePending;

  /// No description provided for @photoPending.
  ///
  /// In en, this message translates to:
  /// **'Photo · waiting to be described'**
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

  /// No description provided for @providers.
  ///
  /// In en, this message translates to:
  /// **'Providers'**
  String get providers;

  /// No description provided for @providersFooter.
  ///
  /// In en, this message translates to:
  /// **'Keys stay in this device\'s secure storage; each device asks for them once.'**
  String get providersFooter;

  /// No description provided for @addProvider.
  ///
  /// In en, this message translates to:
  /// **'Add provider'**
  String get addProvider;

  /// No description provided for @editProvider.
  ///
  /// In en, this message translates to:
  /// **'Provider'**
  String get editProvider;

  /// No description provided for @providerKindOpenai.
  ///
  /// In en, this message translates to:
  /// **'OpenAI-compatible'**
  String get providerKindOpenai;

  /// No description provided for @providerKindAnthropic.
  ///
  /// In en, this message translates to:
  /// **'Anthropic'**
  String get providerKindAnthropic;

  /// No description provided for @providerKindGemini.
  ///
  /// In en, this message translates to:
  /// **'Gemini'**
  String get providerKindGemini;

  /// No description provided for @providerName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get providerName;

  /// No description provided for @providerNameHint.
  ///
  /// In en, this message translates to:
  /// **'My OpenRouter'**
  String get providerNameHint;

  /// No description provided for @baseUrl.
  ///
  /// In en, this message translates to:
  /// **'Address'**
  String get baseUrl;

  /// No description provided for @apiKey.
  ///
  /// In en, this message translates to:
  /// **'API key'**
  String get apiKey;

  /// No description provided for @apiKeyKept.
  ///
  /// In en, this message translates to:
  /// **'Saved on this device; leave empty to keep it'**
  String get apiKeyKept;

  /// No description provided for @apiKeyMissing.
  ///
  /// In en, this message translates to:
  /// **'No key on this device'**
  String get apiKeyMissing;

  /// No description provided for @checkConnection.
  ///
  /// In en, this message translates to:
  /// **'Check'**
  String get checkConnection;

  /// No description provided for @modelsFound.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =0{Connected; no model list} =1{Connected · 1 model} other{Connected · {n} models}}'**
  String modelsFound(int n);

  /// No description provided for @remove.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get remove;

  /// No description provided for @nameRequired.
  ///
  /// In en, this message translates to:
  /// **'Give the provider a name.'**
  String get nameRequired;

  /// No description provided for @models.
  ///
  /// In en, this message translates to:
  /// **'Models'**
  String get models;

  /// No description provided for @modelsFooter.
  ///
  /// In en, this message translates to:
  /// **'Roles without a model of their own use the Chat model.'**
  String get modelsFooter;

  /// No description provided for @roleRouter.
  ///
  /// In en, this message translates to:
  /// **'Routing'**
  String get roleRouter;

  /// No description provided for @roleIngest.
  ///
  /// In en, this message translates to:
  /// **'Filing'**
  String get roleIngest;

  /// No description provided for @roleChat.
  ///
  /// In en, this message translates to:
  /// **'Chat'**
  String get roleChat;

  /// No description provided for @roleVoice.
  ///
  /// In en, this message translates to:
  /// **'Voice conversation'**
  String get roleVoice;

  /// No description provided for @roleVision.
  ///
  /// In en, this message translates to:
  /// **'Photos'**
  String get roleVision;

  /// No description provided for @roleReflect.
  ///
  /// In en, this message translates to:
  /// **'Reflection'**
  String get roleReflect;

  /// No description provided for @roleLint.
  ///
  /// In en, this message translates to:
  /// **'Upkeep'**
  String get roleLint;

  /// No description provided for @roleStt.
  ///
  /// In en, this message translates to:
  /// **'Speech to text'**
  String get roleStt;

  /// No description provided for @roleTts.
  ///
  /// In en, this message translates to:
  /// **'Text to speech'**
  String get roleTts;

  /// No description provided for @roleEmbedding.
  ///
  /// In en, this message translates to:
  /// **'Embeddings (optional)'**
  String get roleEmbedding;

  /// No description provided for @roleNotSet.
  ///
  /// In en, this message translates to:
  /// **'Not set'**
  String get roleNotSet;

  /// No description provided for @roleUses.
  ///
  /// In en, this message translates to:
  /// **'Uses {role}'**
  String roleUses(String role);

  /// No description provided for @warnNoVision.
  ///
  /// In en, this message translates to:
  /// **'This model probably can\'t read images.'**
  String get warnNoVision;

  /// No description provided for @warnNotStt.
  ///
  /// In en, this message translates to:
  /// **'This doesn\'t look like a speech-to-text model.'**
  String get warnNotStt;

  /// No description provided for @warnNotTts.
  ///
  /// In en, this message translates to:
  /// **'This doesn\'t look like a text-to-speech model.'**
  String get warnNotTts;

  /// No description provided for @warnNotEmbedding.
  ///
  /// In en, this message translates to:
  /// **'This doesn\'t look like an embedding model.'**
  String get warnNotEmbedding;

  /// No description provided for @warnNotConversational.
  ///
  /// In en, this message translates to:
  /// **'This model can\'t hold a conversation.'**
  String get warnNotConversational;

  /// No description provided for @provider.
  ///
  /// In en, this message translates to:
  /// **'Provider'**
  String get provider;

  /// No description provided for @model.
  ///
  /// In en, this message translates to:
  /// **'Model'**
  String get model;

  /// No description provided for @modelHint.
  ///
  /// In en, this message translates to:
  /// **'Model name'**
  String get modelHint;

  /// No description provided for @modelRequired.
  ///
  /// In en, this message translates to:
  /// **'Choose or type a model.'**
  String get modelRequired;

  /// No description provided for @testRole.
  ///
  /// In en, this message translates to:
  /// **'Test'**
  String get testRole;

  /// No description provided for @testing.
  ///
  /// In en, this message translates to:
  /// **'Testing…'**
  String get testing;

  /// No description provided for @testWorks.
  ///
  /// In en, this message translates to:
  /// **'Works · {ms} ms · {reply}'**
  String testWorks(String ms, String reply);

  /// No description provided for @useChatModel.
  ///
  /// In en, this message translates to:
  /// **'Use the Chat model'**
  String get useChatModel;

  /// No description provided for @addProviderFirst.
  ///
  /// In en, this message translates to:
  /// **'Add a provider first.'**
  String get addProviderFirst;

  /// No description provided for @filedTo.
  ///
  /// In en, this message translates to:
  /// **'Filed to {vaults}'**
  String filedTo(String vaults);

  /// No description provided for @pagesUpdated.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 page updated} other{{n} pages updated}}'**
  String pagesUpdated(int n);

  /// No description provided for @claimsToReview.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 claim to review} other{{n} claims to review}}'**
  String claimsToReview(int n);

  /// No description provided for @listSeparator.
  ///
  /// In en, this message translates to:
  /// **', '**
  String get listSeparator;

  /// No description provided for @filingWaits.
  ///
  /// In en, this message translates to:
  /// **'Filing waits for a {role} model. Set it up in Settings.'**
  String filingWaits(String role);

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retry;

  /// No description provided for @wikiTitle.
  ///
  /// In en, this message translates to:
  /// **'Wiki'**
  String get wikiTitle;

  /// No description provided for @searchWiki.
  ///
  /// In en, this message translates to:
  /// **'Search the wiki'**
  String get searchWiki;

  /// No description provided for @recentPages.
  ///
  /// In en, this message translates to:
  /// **'Recently updated'**
  String get recentPages;

  /// No description provided for @pinnedPages.
  ///
  /// In en, this message translates to:
  /// **'Pinned'**
  String get pinnedPages;

  /// No description provided for @allVaults.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get allVaults;

  /// No description provided for @noResults.
  ///
  /// In en, this message translates to:
  /// **'Nothing matches “{q}”.'**
  String noResults(String q);

  /// No description provided for @emptyWiki.
  ///
  /// In en, this message translates to:
  /// **'Your wiki fills up as notes are filed.'**
  String get emptyWiki;

  /// No description provided for @backlinks.
  ///
  /// In en, this message translates to:
  /// **'Linked from'**
  String get backlinks;

  /// No description provided for @noBacklinks.
  ///
  /// In en, this message translates to:
  /// **'No other page links here yet.'**
  String get noBacklinks;

  /// No description provided for @localGraph.
  ///
  /// In en, this message translates to:
  /// **'Nearby pages'**
  String get localGraph;

  /// No description provided for @editPage.
  ///
  /// In en, this message translates to:
  /// **'Edit'**
  String get editPage;

  /// No description provided for @sourcesCount.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 source} other{{n} sources}}'**
  String sourcesCount(int n);

  /// No description provided for @pagesCount.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 page} other{{n} pages}}'**
  String pagesCount(int n);

  /// No description provided for @updatedOn.
  ///
  /// In en, this message translates to:
  /// **'Updated {date}'**
  String updatedOn(String date);

  /// No description provided for @pageMissing.
  ///
  /// In en, this message translates to:
  /// **'This page does not exist yet.'**
  String get pageMissing;

  /// No description provided for @pages.
  ///
  /// In en, this message translates to:
  /// **'Pages'**
  String get pages;

  /// No description provided for @fieldConfidence.
  ///
  /// In en, this message translates to:
  /// **'confidence'**
  String get fieldConfidence;

  /// No description provided for @fieldSupersededBy.
  ///
  /// In en, this message translates to:
  /// **'replaced by'**
  String get fieldSupersededBy;

  /// No description provided for @calloutConflict.
  ///
  /// In en, this message translates to:
  /// **'Edited on two devices'**
  String get calloutConflict;

  /// No description provided for @calloutWarning.
  ///
  /// In en, this message translates to:
  /// **'Warning'**
  String get calloutWarning;

  /// No description provided for @calloutNote.
  ///
  /// In en, this message translates to:
  /// **'Note'**
  String get calloutNote;

  /// No description provided for @rebuildIndex.
  ///
  /// In en, this message translates to:
  /// **'Rebuild search index'**
  String get rebuildIndex;

  /// No description provided for @indexRebuilt.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 page indexed} other{{n} pages indexed}}'**
  String indexRebuilt(int n);

  /// No description provided for @unsavedChanges.
  ///
  /// In en, this message translates to:
  /// **'You have unsaved changes.'**
  String get unsavedChanges;

  /// No description provided for @keepEditing.
  ///
  /// In en, this message translates to:
  /// **'Keep editing'**
  String get keepEditing;

  /// No description provided for @discard.
  ///
  /// In en, this message translates to:
  /// **'Discard'**
  String get discard;

  /// No description provided for @openPage.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get openPage;

  /// No description provided for @activityTitle.
  ///
  /// In en, this message translates to:
  /// **'Activity'**
  String get activityTitle;

  /// No description provided for @reviewTitle.
  ///
  /// In en, this message translates to:
  /// **'Review'**
  String get reviewTitle;

  /// No description provided for @toReview.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 to review} other{{n} to review}}'**
  String toReview(int n);

  /// No description provided for @opUndo.
  ///
  /// In en, this message translates to:
  /// **'Undid a filing'**
  String get opUndo;

  /// No description provided for @opCompensate.
  ///
  /// In en, this message translates to:
  /// **'Removed an undone note'**
  String get opCompensate;

  /// No description provided for @opReview.
  ///
  /// In en, this message translates to:
  /// **'Reviewed a card'**
  String get opReview;

  /// No description provided for @opSaveAnswer.
  ///
  /// In en, this message translates to:
  /// **'Saved to the wiki'**
  String get opSaveAnswer;

  /// No description provided for @opLint.
  ///
  /// In en, this message translates to:
  /// **'Checked the wiki'**
  String get opLint;

  /// No description provided for @opReflect.
  ///
  /// In en, this message translates to:
  /// **'Reflection'**
  String get opReflect;

  /// No description provided for @opNothingFiled.
  ///
  /// In en, this message translates to:
  /// **'Read a note; nothing needed filing'**
  String get opNothingFiled;

  /// No description provided for @undoneTag.
  ///
  /// In en, this message translates to:
  /// **'undone'**
  String get undoneTag;

  /// No description provided for @whyHere.
  ///
  /// In en, this message translates to:
  /// **'Why here'**
  String get whyHere;

  /// No description provided for @confidencePct.
  ///
  /// In en, this message translates to:
  /// **'{pct} sure'**
  String confidencePct(String pct);

  /// No description provided for @changes.
  ///
  /// In en, this message translates to:
  /// **'Changes'**
  String get changes;

  /// No description provided for @rawDiff.
  ///
  /// In en, this message translates to:
  /// **'Raw diff'**
  String get rawDiff;

  /// No description provided for @usageTokens.
  ///
  /// In en, this message translates to:
  /// **'{tokens} tokens'**
  String usageTokens(String tokens);

  /// No description provided for @costApprox.
  ///
  /// In en, this message translates to:
  /// **'about {cost}'**
  String costApprox(String cost);

  /// No description provided for @undoAction.
  ///
  /// In en, this message translates to:
  /// **'Undo'**
  String get undoAction;

  /// No description provided for @undoUndo.
  ///
  /// In en, this message translates to:
  /// **'Undo this undo'**
  String get undoUndo;

  /// No description provided for @moveToVault.
  ///
  /// In en, this message translates to:
  /// **'Move to vault…'**
  String get moveToVault;

  /// No description provided for @rerunWithNote.
  ///
  /// In en, this message translates to:
  /// **'Re-run with a note…'**
  String get rerunWithNote;

  /// No description provided for @rerunHint.
  ///
  /// In en, this message translates to:
  /// **'e.g. Sara is my cousin, not my colleague'**
  String get rerunHint;

  /// No description provided for @excludeSource.
  ///
  /// In en, this message translates to:
  /// **'Exclude this note'**
  String get excludeSource;

  /// No description provided for @undoneNote.
  ///
  /// In en, this message translates to:
  /// **'Undone.'**
  String get undoneNote;

  /// No description provided for @undoQueued.
  ///
  /// In en, this message translates to:
  /// **'Later changes overlap; it will be undone carefully when online.'**
  String get undoQueued;

  /// No description provided for @refiling.
  ///
  /// In en, this message translates to:
  /// **'Filing it again.'**
  String get refiling;

  /// No description provided for @noActivity.
  ///
  /// In en, this message translates to:
  /// **'Nothing has been filed yet.'**
  String get noActivity;

  /// No description provided for @reviewEmpty.
  ///
  /// In en, this message translates to:
  /// **'Nothing to review.'**
  String get reviewEmpty;

  /// No description provided for @confirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get confirm;

  /// No description provided for @reject.
  ///
  /// In en, this message translates to:
  /// **'Reject'**
  String get reject;

  /// No description provided for @dismiss.
  ///
  /// In en, this message translates to:
  /// **'Dismiss'**
  String get dismiss;

  /// No description provided for @keepFiling.
  ///
  /// In en, this message translates to:
  /// **'Right'**
  String get keepFiling;

  /// No description provided for @routingQuestion.
  ///
  /// In en, this message translates to:
  /// **'Filed to {vaults} — right?'**
  String routingQuestion(String vaults);

  /// No description provided for @claimQuestion.
  ///
  /// In en, this message translates to:
  /// **'Is this right?'**
  String get claimQuestion;

  /// No description provided for @replacesClaim.
  ///
  /// In en, this message translates to:
  /// **'Replaces: {text}'**
  String replacesClaim(String text);

  /// No description provided for @conflictCard.
  ///
  /// In en, this message translates to:
  /// **'Edited on two devices'**
  String get conflictCard;

  /// No description provided for @conflictHint.
  ///
  /// In en, this message translates to:
  /// **'Both versions are kept in the page. Edit it, then mark it resolved.'**
  String get conflictHint;

  /// No description provided for @resolved.
  ///
  /// In en, this message translates to:
  /// **'Resolved'**
  String get resolved;

  /// No description provided for @lintCard.
  ///
  /// In en, this message translates to:
  /// **'Wiki check'**
  String get lintCard;

  /// No description provided for @questionCard.
  ///
  /// In en, this message translates to:
  /// **'A question for you'**
  String get questionCard;

  /// No description provided for @schemaCard.
  ///
  /// In en, this message translates to:
  /// **'Suggested rule change'**
  String get schemaCard;

  /// No description provided for @swipeHint.
  ///
  /// In en, this message translates to:
  /// **'Swipe right to confirm, left to reject.'**
  String get swipeHint;

  /// No description provided for @fixSuggestion.
  ///
  /// In en, this message translates to:
  /// **'Suggested fix: {fix}'**
  String fixSuggestion(String fix);

  /// No description provided for @askTitle.
  ///
  /// In en, this message translates to:
  /// **'Ask'**
  String get askTitle;

  /// No description provided for @askHint.
  ///
  /// In en, this message translates to:
  /// **'Ask about your notes…'**
  String get askHint;

  /// No description provided for @askScopeAll.
  ///
  /// In en, this message translates to:
  /// **'Everything'**
  String get askScopeAll;

  /// No description provided for @storyScope.
  ///
  /// In en, this message translates to:
  /// **'Story: {name}'**
  String storyScope(String name);

  /// No description provided for @saveToWiki.
  ///
  /// In en, this message translates to:
  /// **'Save to wiki'**
  String get saveToWiki;

  /// No description provided for @savedToWiki.
  ///
  /// In en, this message translates to:
  /// **'Saved; it will be filed.'**
  String get savedToWiki;

  /// No description provided for @saveDraft.
  ///
  /// In en, this message translates to:
  /// **'Save as draft'**
  String get saveDraft;

  /// No description provided for @draftSaved.
  ///
  /// In en, this message translates to:
  /// **'Draft saved.'**
  String get draftSaved;

  /// No description provided for @attachPhoto.
  ///
  /// In en, this message translates to:
  /// **'Attach a photo'**
  String get attachPhoto;

  /// No description provided for @removePhoto.
  ///
  /// In en, this message translates to:
  /// **'Remove the photo'**
  String get removePhoto;

  /// No description provided for @send.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get send;

  /// No description provided for @talkMode.
  ///
  /// In en, this message translates to:
  /// **'Talk'**
  String get talkMode;

  /// No description provided for @thinking.
  ///
  /// In en, this message translates to:
  /// **'Reading your wiki…'**
  String get thinking;

  /// No description provided for @askEmpty.
  ///
  /// In en, this message translates to:
  /// **'Ask anything about what you have captured.'**
  String get askEmpty;

  /// No description provided for @askReading.
  ///
  /// In en, this message translates to:
  /// **'Reading: {page}'**
  String askReading(String page);

  /// No description provided for @talkToSomeone.
  ///
  /// In en, this message translates to:
  /// **'Talk to someone'**
  String get talkToSomeone;

  /// No description provided for @talkToSomeoneBody.
  ///
  /// In en, this message translates to:
  /// **'You don\'t have to carry this alone. Someone trained can listen right now.'**
  String get talkToSomeoneBody;

  /// No description provided for @newConversation.
  ///
  /// In en, this message translates to:
  /// **'New conversation'**
  String get newConversation;

  /// No description provided for @voiceListening.
  ///
  /// In en, this message translates to:
  /// **'Listening'**
  String get voiceListening;

  /// No description provided for @voiceThinking.
  ///
  /// In en, this message translates to:
  /// **'Thinking'**
  String get voiceThinking;

  /// No description provided for @voiceSpeaking.
  ///
  /// In en, this message translates to:
  /// **'Speaking'**
  String get voiceSpeaking;

  /// No description provided for @voiceMuted.
  ///
  /// In en, this message translates to:
  /// **'Muted'**
  String get voiceMuted;

  /// No description provided for @mute.
  ///
  /// In en, this message translates to:
  /// **'Mute'**
  String get mute;

  /// No description provided for @unmute.
  ///
  /// In en, this message translates to:
  /// **'Unmute'**
  String get unmute;

  /// No description provided for @endConversation.
  ///
  /// In en, this message translates to:
  /// **'End'**
  String get endConversation;

  /// No description provided for @captions.
  ///
  /// In en, this message translates to:
  /// **'Captions'**
  String get captions;

  /// No description provided for @voiceNoted.
  ///
  /// In en, this message translates to:
  /// **'Noted.'**
  String get voiceNoted;

  /// No description provided for @voiceSaved.
  ///
  /// In en, this message translates to:
  /// **'Conversation saved; it will be filed.'**
  String get voiceSaved;

  /// No description provided for @voiceNeedsMic.
  ///
  /// In en, this message translates to:
  /// **'Voice mode needs the microphone.'**
  String get voiceNeedsMic;

  /// No description provided for @mcpServers.
  ///
  /// In en, this message translates to:
  /// **'Outside tools (MCP)'**
  String get mcpServers;

  /// No description provided for @mcpFooter.
  ///
  /// In en, this message translates to:
  /// **'Server settings sync between devices; credentials stay on each device, which signs in once.'**
  String get mcpFooter;

  /// No description provided for @addMcpServer.
  ///
  /// In en, this message translates to:
  /// **'Add server'**
  String get addMcpServer;

  /// No description provided for @mcpConnected.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{Connected · 1 tool} other{Connected · {n} tools}}'**
  String mcpConnected(int n);

  /// No description provided for @mcpNeedsAuth.
  ///
  /// In en, this message translates to:
  /// **'Connect on this device'**
  String get mcpNeedsAuth;

  /// No description provided for @mcpDesktopOnly.
  ///
  /// In en, this message translates to:
  /// **'Runs a local program; desktop only'**
  String get mcpDesktopOnly;

  /// No description provided for @mcpDisabled.
  ///
  /// In en, this message translates to:
  /// **'Off'**
  String get mcpDisabled;

  /// No description provided for @mcpChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking…'**
  String get mcpChecking;

  /// No description provided for @transportHttp.
  ///
  /// In en, this message translates to:
  /// **'Streamable HTTP'**
  String get transportHttp;

  /// No description provided for @transportSse.
  ///
  /// In en, this message translates to:
  /// **'HTTP + SSE'**
  String get transportSse;

  /// No description provided for @transportStdio.
  ///
  /// In en, this message translates to:
  /// **'Local program'**
  String get transportStdio;

  /// No description provided for @stdioMobile.
  ///
  /// In en, this message translates to:
  /// **'Local programs only run on desktop.'**
  String get stdioMobile;

  /// No description provided for @command.
  ///
  /// In en, this message translates to:
  /// **'Command'**
  String get command;

  /// No description provided for @arguments.
  ///
  /// In en, this message translates to:
  /// **'Arguments, one per line'**
  String get arguments;

  /// No description provided for @envNames.
  ///
  /// In en, this message translates to:
  /// **'Environment variable names, one per line'**
  String get envNames;

  /// No description provided for @authNone.
  ///
  /// In en, this message translates to:
  /// **'None'**
  String get authNone;

  /// No description provided for @authBearer.
  ///
  /// In en, this message translates to:
  /// **'Token'**
  String get authBearer;

  /// No description provided for @authHeader.
  ///
  /// In en, this message translates to:
  /// **'Key in a header'**
  String get authHeader;

  /// No description provided for @authQuery.
  ///
  /// In en, this message translates to:
  /// **'Key in the address'**
  String get authQuery;

  /// No description provided for @authHeaders.
  ///
  /// In en, this message translates to:
  /// **'Custom headers'**
  String get authHeaders;

  /// No description provided for @authOAuth.
  ///
  /// In en, this message translates to:
  /// **'Sign in (OAuth)'**
  String get authOAuth;

  /// No description provided for @authClient.
  ///
  /// In en, this message translates to:
  /// **'Client credentials'**
  String get authClient;

  /// No description provided for @authLabel.
  ///
  /// In en, this message translates to:
  /// **'Access'**
  String get authLabel;

  /// No description provided for @authName.
  ///
  /// In en, this message translates to:
  /// **'Header or parameter name'**
  String get authName;

  /// No description provided for @headerNames.
  ///
  /// In en, this message translates to:
  /// **'Header names, one per line'**
  String get headerNames;

  /// No description provided for @secretValue.
  ///
  /// In en, this message translates to:
  /// **'Token or key (this device only)'**
  String get secretValue;

  /// No description provided for @headerValues.
  ///
  /// In en, this message translates to:
  /// **'Header values, “name: value” per line (this device only)'**
  String get headerValues;

  /// No description provided for @envValues.
  ///
  /// In en, this message translates to:
  /// **'Values, NAME=value per line (this device only)'**
  String get envValues;

  /// No description provided for @clientId.
  ///
  /// In en, this message translates to:
  /// **'Client id (empty: register automatically)'**
  String get clientId;

  /// No description provided for @clientSecret.
  ///
  /// In en, this message translates to:
  /// **'Client secret (this device only)'**
  String get clientSecret;

  /// No description provided for @scopesLabel.
  ///
  /// In en, this message translates to:
  /// **'Scopes, separated by spaces'**
  String get scopesLabel;

  /// No description provided for @policyLabel.
  ///
  /// In en, this message translates to:
  /// **'When a tool may change something'**
  String get policyLabel;

  /// No description provided for @policyAsk.
  ///
  /// In en, this message translates to:
  /// **'Always ask'**
  String get policyAsk;

  /// No description provided for @policyReadOnly.
  ///
  /// In en, this message translates to:
  /// **'Ask unless read-only'**
  String get policyReadOnly;

  /// No description provided for @policyAllow.
  ///
  /// In en, this message translates to:
  /// **'Always allow'**
  String get policyAllow;

  /// No description provided for @serverOn.
  ///
  /// In en, this message translates to:
  /// **'On'**
  String get serverOn;

  /// No description provided for @signIn.
  ///
  /// In en, this message translates to:
  /// **'Sign in'**
  String get signIn;

  /// No description provided for @signedIn.
  ///
  /// In en, this message translates to:
  /// **'Connected on this device.'**
  String get signedIn;

  /// No description provided for @toolApproval.
  ///
  /// In en, this message translates to:
  /// **'{server} wants to run {tool}'**
  String toolApproval(String server, String tool);

  /// No description provided for @allow.
  ///
  /// In en, this message translates to:
  /// **'Allow'**
  String get allow;

  /// No description provided for @deny.
  ///
  /// In en, this message translates to:
  /// **'Don\'t allow'**
  String get deny;

  /// No description provided for @readOnlyTool.
  ///
  /// In en, this message translates to:
  /// **'Only reads'**
  String get readOnlyTool;

  /// No description provided for @reflectSection.
  ///
  /// In en, this message translates to:
  /// **'Reflect'**
  String get reflectSection;

  /// No description provided for @reflectFooter.
  ///
  /// In en, this message translates to:
  /// **'Reflections are written from your own captures and filed to Journal. A notification never says what a hard day was about.'**
  String get reflectFooter;

  /// No description provided for @dailyReflection.
  ///
  /// In en, this message translates to:
  /// **'Daily reflection'**
  String get dailyReflection;

  /// No description provided for @reflectTime.
  ///
  /// In en, this message translates to:
  /// **'Time'**
  String get reflectTime;

  /// No description provided for @weeklyReview.
  ///
  /// In en, this message translates to:
  /// **'Weekly review'**
  String get weeklyReview;

  /// No description provided for @reflectDay.
  ///
  /// In en, this message translates to:
  /// **'Day'**
  String get reflectDay;

  /// No description provided for @reflectNotifications.
  ///
  /// In en, this message translates to:
  /// **'Notifications'**
  String get reflectNotifications;

  /// No description provided for @notificationsBlocked.
  ///
  /// In en, this message translates to:
  /// **'Notifications are turned off for this app in system settings.'**
  String get notificationsBlocked;

  /// No description provided for @helplineCountry.
  ///
  /// In en, this message translates to:
  /// **'Helpline country'**
  String get helplineCountry;

  /// No description provided for @helplineInternational.
  ///
  /// In en, this message translates to:
  /// **'International'**
  String get helplineInternational;

  /// No description provided for @countryIR.
  ///
  /// In en, this message translates to:
  /// **'Iran'**
  String get countryIR;

  /// No description provided for @countryUS.
  ///
  /// In en, this message translates to:
  /// **'United States'**
  String get countryUS;

  /// No description provided for @countryGB.
  ///
  /// In en, this message translates to:
  /// **'United Kingdom'**
  String get countryGB;

  /// No description provided for @countryDE.
  ///
  /// In en, this message translates to:
  /// **'Germany'**
  String get countryDE;

  /// No description provided for @countryCA.
  ///
  /// In en, this message translates to:
  /// **'Canada'**
  String get countryCA;

  /// No description provided for @checkWikiNow.
  ///
  /// In en, this message translates to:
  /// **'Check the wiki now'**
  String get checkWikiNow;

  /// No description provided for @checkWikiFooter.
  ///
  /// In en, this message translates to:
  /// **'Looks for broken links, orphan pages and captures that never got filed. It also runs by itself after every few filings.'**
  String get checkWikiFooter;

  /// No description provided for @lintClean.
  ///
  /// In en, this message translates to:
  /// **'Nothing needs fixing.'**
  String get lintClean;

  /// No description provided for @lintFound.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 thing to look at} other{{n} things to look at}}'**
  String lintFound(int n);

  /// No description provided for @lintNewCards.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{1 new in Review} other{{n} new in Review}}'**
  String lintNewCards(int n);

  /// No description provided for @reflectChannel.
  ///
  /// In en, this message translates to:
  /// **'Reflections'**
  String get reflectChannel;

  /// No description provided for @open.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get open;

  /// No description provided for @close.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// No description provided for @recordVoiceNote.
  ///
  /// In en, this message translates to:
  /// **'Record a voice note'**
  String get recordVoiceNote;

  /// No description provided for @commandPalette.
  ///
  /// In en, this message translates to:
  /// **'Commands'**
  String get commandPalette;

  /// No description provided for @paletteHint.
  ///
  /// In en, this message translates to:
  /// **'Search, open, capture or ask'**
  String get paletteHint;

  /// No description provided for @paletteNewNote.
  ///
  /// In en, this message translates to:
  /// **'New note'**
  String get paletteNewNote;

  /// No description provided for @paletteTakePhoto.
  ///
  /// In en, this message translates to:
  /// **'Take a photo'**
  String get paletteTakePhoto;

  /// No description provided for @paletteSaveNote.
  ///
  /// In en, this message translates to:
  /// **'Save as a note: {text}'**
  String paletteSaveNote(String text);

  /// No description provided for @paletteAsk.
  ///
  /// In en, this message translates to:
  /// **'Ask: {text}'**
  String paletteAsk(String text);

  /// No description provided for @paletteGoTo.
  ///
  /// In en, this message translates to:
  /// **'Go to {place}'**
  String paletteGoTo(String place);

  /// No description provided for @paletteNoMatch.
  ///
  /// In en, this message translates to:
  /// **'Nothing matches.'**
  String get paletteNoMatch;

  /// No description provided for @paletteShortcutHint.
  ///
  /// In en, this message translates to:
  /// **'Ctrl+K opens this anywhere'**
  String get paletteShortcutHint;

  /// No description provided for @paletteShortcutHintMac.
  ///
  /// In en, this message translates to:
  /// **'⌘K opens this anywhere'**
  String get paletteShortcutHintMac;

  /// No description provided for @sharedSaved.
  ///
  /// In en, this message translates to:
  /// **'{n, plural, =1{Saved what you shared} other{Saved {n} shared items}}'**
  String sharedSaved(int n);
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
