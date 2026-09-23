// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class L10nEn extends L10n {
  L10nEn([String locale = 'en']) : super(locale);

  @override
  String get settingsTitle => 'Settings';

  @override
  String get appearance => 'Appearance';

  @override
  String get language => 'Language';

  @override
  String get languageSystem => 'System';

  @override
  String get languageEnglish => 'English';

  @override
  String get languagePersian => 'فارسی';

  @override
  String get theme => 'Theme';

  @override
  String get themeSystem => 'System';

  @override
  String get themeLight => 'Light';

  @override
  String get themeDark => 'Dark';

  @override
  String get textSize => 'Text size';

  @override
  String get textSizeFooter => 'Applies on top of your system text size.';

  @override
  String get about => 'About';

  @override
  String get version => 'Version';

  @override
  String get coreVersion => 'Core';

  @override
  String coreVersionValue(String version, String target) {
    return '$version · $target';
  }

  @override
  String get repoFormat => 'Repository format';

  @override
  String repoFormatValue(String n) {
    return 'v$n';
  }

  @override
  String privacyNote(String app) {
    return 'No telemetry. $app talks only to your Git remote, your AI providers and your MCP servers.';
  }

  @override
  String get designSystem => 'Design system';

  @override
  String get back => 'Back';

  @override
  String get galleryButtons => 'Buttons';

  @override
  String get galleryControls => 'Controls';

  @override
  String get galleryStatus => 'Claim status';

  @override
  String get galleryType => 'Type';

  @override
  String get galleryIcons => 'Icons';

  @override
  String get sampleSave => 'Save';

  @override
  String get sampleCancel => 'Cancel';

  @override
  String get sampleRetry => 'Retry';

  @override
  String get sampleSearch => 'Search your wiki';

  @override
  String get statusConfirmed => 'confirmed';

  @override
  String get statusProposed => 'proposed';

  @override
  String get statusSuperseded => 'superseded';

  @override
  String get sampleNotify => 'Notify me in the evening';

  @override
  String get sampleHeading => 'Filed to Life and Health';

  @override
  String get sampleBody =>
      'Slept badly, headache by noon. Sara called about the trip to Isfahan.';

  @override
  String get sampleMixed => 'جلسه با Sara درباره‌ی project جدید خوب بود.';

  @override
  String get welcomeLine => 'A notebook that files itself.';

  @override
  String get welcomeBody =>
      'Talk, type or snap a photo. Everything stays in a private Git repository you own.';

  @override
  String get connectRepo => 'Connect a private repository';

  @override
  String get startLocal => 'Start on this device for now';

  @override
  String get connectTitle => 'Your repository';

  @override
  String get connectHttps => 'HTTPS + token';

  @override
  String get connectSsh => 'SSH key';

  @override
  String get repoUrl => 'Repository address';

  @override
  String get repoUrlHintHttps => 'https://github.com/you/notes.git';

  @override
  String get repoUrlHintSsh => 'git@github.com:you/notes.git';

  @override
  String get accessToken => 'Access token';

  @override
  String get tokenHelp =>
      'A fine-grained token with read and write access to this one repository is enough.';

  @override
  String get branch => 'Branch';

  @override
  String get generateKey => 'Create a key for this device';

  @override
  String get publicKey => 'Public key';

  @override
  String get copy => 'Copy';

  @override
  String get copied => 'Copied';

  @override
  String get sshHelp =>
      'Add this key with write access: GitHub → repository Settings → Deploy keys. GitLab → Settings → Repository → Deploy keys. Gitea → Settings → Deploy Keys.';

  @override
  String get continueAction => 'Continue';

  @override
  String get deviceTitle => 'Name this device';

  @override
  String get deviceHelp =>
      'Shown in history, so you know where each note came from.';

  @override
  String get deviceName => 'Device name';

  @override
  String get connecting => 'Connecting…';

  @override
  String get preparing => 'Preparing your notebook…';

  @override
  String get tryAgain => 'Try again';

  @override
  String get todayTitle => 'Today';

  @override
  String get emptyToday => 'Nothing captured yet today.';

  @override
  String get emptyTodayHint => 'Hold the button and talk, or tap it to type.';

  @override
  String get stageSaved => 'Saved';

  @override
  String get stageWorking => 'Filing…';

  @override
  String get stageFailed => 'Couldn\'t file';

  @override
  String get stageFiled => 'Filed';

  @override
  String get stageExcluded => 'Excluded';

  @override
  String get voiceNote => 'Voice note';

  @override
  String get voicePending =>
      'Voice note · transcribed when a speech provider is set up';

  @override
  String get photoPending =>
      'Photo · described when a vision provider is set up';

  @override
  String get captureRecordLabel => 'Hold to record, tap to type';

  @override
  String get captureCamera => 'Take a photo';

  @override
  String get pinVault => 'Choose where the next capture goes';

  @override
  String get vaultAuto => 'Auto';

  @override
  String get slideToCancel => 'Slide to cancel';

  @override
  String get slideUpToLock => 'Slide up to lock';

  @override
  String get recordingCancelled => 'Recording discarded';

  @override
  String get micDenied =>
      'Microphone access is off. Turn it on in system settings to record.';

  @override
  String get recordingFailed => 'Couldn\'t record on this device.';

  @override
  String get stop => 'Save';

  @override
  String get cancel => 'Cancel';

  @override
  String get typeSomething => 'What\'s on your mind?';

  @override
  String get save => 'Save';

  @override
  String savedTo(String vault) {
    return 'Saved · $vault';
  }

  @override
  String get saved => 'Saved';

  @override
  String get syncSynced => 'Synced';

  @override
  String syncLocal(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n changes to sync',
      one: '1 change to sync',
    );
    return '$_temp0';
  }

  @override
  String get syncSyncing => 'Syncing…';

  @override
  String get syncOffline => 'Offline';

  @override
  String get syncAttention => 'Needs attention';

  @override
  String get syncNoRemote => 'On this device only';

  @override
  String get repository => 'Repository';

  @override
  String get remote => 'Remote';

  @override
  String get notConnected => 'Not connected';

  @override
  String get connect => 'Connect…';

  @override
  String get device => 'This device';

  @override
  String get folder => 'Folder';

  @override
  String get syncNow => 'Sync now';

  @override
  String lastSynced(String time) {
    return 'Last synced $time';
  }

  @override
  String get obsidianNote =>
      'This folder is a plain Markdown vault; open it in Obsidian any time.';
}
