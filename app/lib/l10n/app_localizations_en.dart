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
  String get voicePending => 'Voice note · waiting to be transcribed';

  @override
  String get photoPending => 'Photo · waiting to be described';

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
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString changes to sync',
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

  @override
  String get providers => 'Providers';

  @override
  String get providersFooter =>
      'Keys stay in this device\'s secure storage; each device asks for them once.';

  @override
  String get addProvider => 'Add provider';

  @override
  String get editProvider => 'Provider';

  @override
  String get providerKindOpenai => 'OpenAI-compatible';

  @override
  String get providerKindAnthropic => 'Anthropic';

  @override
  String get providerKindGemini => 'Gemini';

  @override
  String get providerName => 'Name';

  @override
  String get providerNameHint => 'My OpenRouter';

  @override
  String get baseUrl => 'Address';

  @override
  String get apiKey => 'API key';

  @override
  String get apiKeyKept => 'Saved on this device; leave empty to keep it';

  @override
  String get apiKeyMissing => 'No key on this device';

  @override
  String get checkConnection => 'Check';

  @override
  String modelsFound(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'Connected · $nString models',
      one: 'Connected · 1 model',
      zero: 'Connected; no model list',
    );
    return '$_temp0';
  }

  @override
  String get remove => 'Remove';

  @override
  String get nameRequired => 'Give the provider a name.';

  @override
  String get models => 'Models';

  @override
  String get modelsFooter =>
      'Roles without a model of their own use the Chat model.';

  @override
  String get roleRouter => 'Routing';

  @override
  String get roleIngest => 'Filing';

  @override
  String get roleChat => 'Chat';

  @override
  String get roleVoice => 'Voice conversation';

  @override
  String get roleVision => 'Photos';

  @override
  String get roleReflect => 'Reflection';

  @override
  String get roleLint => 'Upkeep';

  @override
  String get roleStt => 'Speech to text';

  @override
  String get roleTts => 'Text to speech';

  @override
  String get roleEmbedding => 'Embeddings (optional)';

  @override
  String get roleNotSet => 'Not set';

  @override
  String roleUses(String role) {
    return 'Uses $role';
  }

  @override
  String get warnNoVision => 'This model probably can\'t read images.';

  @override
  String get warnNotStt => 'This doesn\'t look like a speech-to-text model.';

  @override
  String get warnNotTts => 'This doesn\'t look like a text-to-speech model.';

  @override
  String get warnNotEmbedding => 'This doesn\'t look like an embedding model.';

  @override
  String get warnNotConversational => 'This model can\'t hold a conversation.';

  @override
  String get provider => 'Provider';

  @override
  String get model => 'Model';

  @override
  String get modelHint => 'Model name';

  @override
  String get modelRequired => 'Choose or type a model.';

  @override
  String get testRole => 'Test';

  @override
  String get testing => 'Testing…';

  @override
  String testWorks(String ms, String reply) {
    return 'Works · $ms ms · $reply';
  }

  @override
  String get useChatModel => 'Use the Chat model';

  @override
  String get addProviderFirst => 'Add a provider first.';

  @override
  String filedTo(String vaults) {
    return 'Filed to $vaults';
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
      other: '$nString pages updated',
      one: '1 page updated',
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
      other: '$nString claims to review',
      one: '1 claim to review',
    );
    return '$_temp0';
  }

  @override
  String get listSeparator => ', ';

  @override
  String filingWaits(String role) {
    return 'Filing waits for a $role model. Set it up in Settings.';
  }

  @override
  String get retry => 'Retry';

  @override
  String get wikiTitle => 'Wiki';

  @override
  String get searchWiki => 'Search the wiki';

  @override
  String get recentPages => 'Recently updated';

  @override
  String get pinnedPages => 'Pinned';

  @override
  String get allVaults => 'All';

  @override
  String noResults(String q) {
    return 'Nothing matches “$q”.';
  }

  @override
  String get emptyWiki => 'Your wiki fills up as notes are filed.';

  @override
  String get backlinks => 'Linked from';

  @override
  String get noBacklinks => 'No other page links here yet.';

  @override
  String get localGraph => 'Nearby pages';

  @override
  String get editPage => 'Edit';

  @override
  String sourcesCount(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString sources',
      one: '1 source',
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
      other: '$nString pages',
      one: '1 page',
    );
    return '$_temp0';
  }

  @override
  String updatedOn(String date) {
    return 'Updated $date';
  }

  @override
  String get pageMissing => 'This page does not exist yet.';

  @override
  String get pages => 'Pages';

  @override
  String get fieldConfidence => 'confidence';

  @override
  String get fieldSupersededBy => 'replaced by';

  @override
  String get calloutConflict => 'Edited on two devices';

  @override
  String get calloutWarning => 'Warning';

  @override
  String get calloutNote => 'Note';

  @override
  String get rebuildIndex => 'Rebuild search index';

  @override
  String indexRebuilt(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString pages indexed',
      one: '1 page indexed',
    );
    return '$_temp0';
  }

  @override
  String get unsavedChanges => 'You have unsaved changes.';

  @override
  String get keepEditing => 'Keep editing';

  @override
  String get discard => 'Discard';

  @override
  String get openPage => 'Open';

  @override
  String get activityTitle => 'Activity';

  @override
  String get reviewTitle => 'Review';

  @override
  String toReview(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString to review',
      one: '1 to review',
    );
    return '$_temp0';
  }

  @override
  String get opUndo => 'Undid a filing';

  @override
  String get opCompensate => 'Removed an undone note';

  @override
  String get opReview => 'Reviewed a card';

  @override
  String get opSaveAnswer => 'Saved to the wiki';

  @override
  String get opLint => 'Checked the wiki';

  @override
  String get opReflect => 'Reflection';

  @override
  String get opNothingFiled => 'Read a note; nothing needed filing';

  @override
  String get undoneTag => 'undone';

  @override
  String get whyHere => 'Why here';

  @override
  String confidencePct(String pct) {
    return '$pct sure';
  }

  @override
  String get changes => 'Changes';

  @override
  String get rawDiff => 'Raw diff';

  @override
  String usageTokens(String tokens) {
    return '$tokens tokens';
  }

  @override
  String costApprox(String cost) {
    return 'about $cost';
  }

  @override
  String get undoAction => 'Undo';

  @override
  String get undoUndo => 'Undo this undo';

  @override
  String get moveToVault => 'Move to vault…';

  @override
  String get rerunWithNote => 'Re-run with a note…';

  @override
  String get rerunHint => 'e.g. Sara is my cousin, not my colleague';

  @override
  String get excludeSource => 'Exclude this note';

  @override
  String get undoneNote => 'Undone.';

  @override
  String get undoQueued =>
      'Later changes overlap; it will be undone carefully when online.';

  @override
  String get refiling => 'Filing it again.';

  @override
  String get noActivity => 'Nothing has been filed yet.';

  @override
  String get reviewEmpty => 'Nothing to review.';

  @override
  String get confirm => 'Confirm';

  @override
  String get reject => 'Reject';

  @override
  String get dismiss => 'Dismiss';

  @override
  String get keepFiling => 'Right';

  @override
  String routingQuestion(String vaults) {
    return 'Filed to $vaults — right?';
  }

  @override
  String get claimQuestion => 'Is this right?';

  @override
  String replacesClaim(String text) {
    return 'Replaces: $text';
  }

  @override
  String get conflictCard => 'Edited on two devices';

  @override
  String get conflictHint =>
      'Both versions are kept in the page. Edit it, then mark it resolved.';

  @override
  String get resolved => 'Resolved';

  @override
  String get lintCard => 'Wiki check';

  @override
  String get questionCard => 'A question for you';

  @override
  String get schemaCard => 'Suggested rule change';

  @override
  String get swipeHint => 'Swipe right to confirm, left to reject.';

  @override
  String fixSuggestion(String fix) {
    return 'Suggested fix: $fix';
  }

  @override
  String get askTitle => 'Ask';

  @override
  String get askHint => 'Ask about your notes…';

  @override
  String get askScopeAll => 'Everything';

  @override
  String storyScope(String name) {
    return 'Story: $name';
  }

  @override
  String get saveToWiki => 'Save to wiki';

  @override
  String get savedToWiki => 'Saved; it will be filed.';

  @override
  String get saveDraft => 'Save as draft';

  @override
  String get draftSaved => 'Draft saved.';

  @override
  String get attachPhoto => 'Attach a photo';

  @override
  String get removePhoto => 'Remove the photo';

  @override
  String get send => 'Send';

  @override
  String get talkMode => 'Talk';

  @override
  String get thinking => 'Reading your wiki…';

  @override
  String get askEmpty => 'Ask anything about what you have captured.';

  @override
  String askReading(String page) {
    return 'Reading: $page';
  }

  @override
  String get talkToSomeone => 'Talk to someone';

  @override
  String get talkToSomeoneBody =>
      'You don\'t have to carry this alone. Someone trained can listen right now.';

  @override
  String get newConversation => 'New conversation';

  @override
  String get voiceListening => 'Listening';

  @override
  String get voiceThinking => 'Thinking';

  @override
  String get voiceSpeaking => 'Speaking';

  @override
  String get voiceMuted => 'Muted';

  @override
  String get mute => 'Mute';

  @override
  String get unmute => 'Unmute';

  @override
  String get endConversation => 'End';

  @override
  String get captions => 'Captions';

  @override
  String get voiceNoted => 'Noted.';

  @override
  String get voiceSaved => 'Conversation saved; it will be filed.';

  @override
  String get voiceNeedsMic => 'Voice mode needs the microphone.';

  @override
  String get mcpServers => 'Outside tools (MCP)';

  @override
  String get mcpFooter =>
      'Server settings sync between devices; credentials stay on each device, which signs in once.';

  @override
  String get addMcpServer => 'Add server';

  @override
  String mcpConnected(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'Connected · $nString tools',
      one: 'Connected · 1 tool',
    );
    return '$_temp0';
  }

  @override
  String get mcpNeedsAuth => 'Connect on this device';

  @override
  String get mcpDesktopOnly => 'Runs a local program; desktop only';

  @override
  String get mcpDisabled => 'Off';

  @override
  String get mcpChecking => 'Checking…';

  @override
  String get transportHttp => 'Streamable HTTP';

  @override
  String get transportSse => 'HTTP + SSE';

  @override
  String get transportStdio => 'Local program';

  @override
  String get stdioMobile => 'Local programs only run on desktop.';

  @override
  String get command => 'Command';

  @override
  String get arguments => 'Arguments, one per line';

  @override
  String get envNames => 'Environment variable names, one per line';

  @override
  String get authNone => 'None';

  @override
  String get authBearer => 'Token';

  @override
  String get authHeader => 'Key in a header';

  @override
  String get authQuery => 'Key in the address';

  @override
  String get authHeaders => 'Custom headers';

  @override
  String get authOAuth => 'Sign in (OAuth)';

  @override
  String get authClient => 'Client credentials';

  @override
  String get authLabel => 'Access';

  @override
  String get authName => 'Header or parameter name';

  @override
  String get headerNames => 'Header names, one per line';

  @override
  String get secretValue => 'Token or key (this device only)';

  @override
  String get headerValues =>
      'Header values, “name: value” per line (this device only)';

  @override
  String get envValues => 'Values, NAME=value per line (this device only)';

  @override
  String get clientId => 'Client id (empty: register automatically)';

  @override
  String get clientSecret => 'Client secret (this device only)';

  @override
  String get scopesLabel => 'Scopes, separated by spaces';

  @override
  String get policyLabel => 'When a tool may change something';

  @override
  String get policyAsk => 'Always ask';

  @override
  String get policyReadOnly => 'Ask unless read-only';

  @override
  String get policyAllow => 'Always allow';

  @override
  String get serverOn => 'On';

  @override
  String get signIn => 'Sign in';

  @override
  String get signedIn => 'Connected on this device.';

  @override
  String toolApproval(String server, String tool) {
    return '$server wants to run $tool';
  }

  @override
  String get allow => 'Allow';

  @override
  String get deny => 'Don\'t allow';

  @override
  String get readOnlyTool => 'Only reads';

  @override
  String get reflectSection => 'Reflect';

  @override
  String get reflectFooter =>
      'Reflections are written from your own captures and filed to Journal. A notification never says what a hard day was about.';

  @override
  String get dailyReflection => 'Daily reflection';

  @override
  String get reflectTime => 'Time';

  @override
  String get weeklyReview => 'Weekly review';

  @override
  String get reflectDay => 'Day';

  @override
  String get reflectNotifications => 'Notifications';

  @override
  String get notificationsBlocked =>
      'Notifications are turned off for this app in system settings.';

  @override
  String get helplineCountry => 'Helpline country';

  @override
  String get helplineInternational => 'International';

  @override
  String get countryIR => 'Iran';

  @override
  String get countryUS => 'United States';

  @override
  String get countryGB => 'United Kingdom';

  @override
  String get countryDE => 'Germany';

  @override
  String get countryCA => 'Canada';

  @override
  String get checkWikiNow => 'Check the wiki now';

  @override
  String get checkWikiFooter =>
      'Looks for broken links, orphan pages and captures that never got filed. It also runs by itself after every few filings.';

  @override
  String get lintClean => 'Nothing needs fixing.';

  @override
  String lintFound(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$nString things to look at',
      one: '1 thing to look at',
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
      other: '$nString new in Review',
      one: '1 new in Review',
    );
    return '$_temp0';
  }

  @override
  String get reflectChannel => 'Reflections';

  @override
  String get open => 'Open';

  @override
  String get close => 'Close';

  @override
  String get recordVoiceNote => 'Record a voice note';

  @override
  String get commandPalette => 'Commands';

  @override
  String get paletteHint => 'Search, open, capture or ask';

  @override
  String get paletteNewNote => 'New note';

  @override
  String get paletteTakePhoto => 'Take a photo';

  @override
  String paletteSaveNote(String text) {
    return 'Save as a note: $text';
  }

  @override
  String paletteAsk(String text) {
    return 'Ask: $text';
  }

  @override
  String paletteGoTo(String place) {
    return 'Go to $place';
  }

  @override
  String get paletteNoMatch => 'Nothing matches.';

  @override
  String get paletteShortcutHint => 'Ctrl+K opens this anywhere';

  @override
  String get paletteShortcutHintMac => '⌘K opens this anywhere';

  @override
  String sharedSaved(int n) {
    final intl.NumberFormat nNumberFormat = intl.NumberFormat.decimalPattern(
      localeName,
    );
    final String nString = nNumberFormat.format(n);

    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'Saved $nString shared items',
      one: 'Saved what you shared',
    );
    return '$_temp0';
  }

  @override
  String get voiceNoticeTitle => 'Talking';

  @override
  String get voiceNoticeBody =>
      'Listening. Open the app to end the conversation.';

  @override
  String get voiceNoticeChannel => 'Voice conversation';
}
