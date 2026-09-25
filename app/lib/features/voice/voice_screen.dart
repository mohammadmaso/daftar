import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart' show Material;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/bidi.dart';
import '../../core/credentials.dart';
import '../../core/errors.dart';
import '../../core/job_runner.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../core/voice_io.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import '../ask/ask_screen.dart' show TalkToSomeoneCard;

/// Hands-free voice mode (§8.4): one softly moving shape, the state, optional captions, and two
/// controls. The microphone streams to the core, which decides when the user has finished, answers
/// with the wiki as memory, and speaks sentence by sentence; talking over it stops it at once.
class VoiceScreen extends ConsumerStatefulWidget {
  const VoiceScreen({super.key});

  @override
  ConsumerState<VoiceScreen> createState() => _VoiceScreenState();
}

class _VoiceScreenState extends ConsumerState<VoiceScreen>
    with SingleTickerProviderStateMixin {
  VoiceConversation? _voice;
  StreamSubscription<VoiceEventDto>? _events;
  StreamSubscription<Uint8List>? _mic;
  StreamSubscription<void>? _idle;
  VoiceStateDto _state = VoiceStateDto.listening;
  final _captions = <(bool, String)>[];
  bool _showCaptions = true;
  bool _muted = false;
  bool _help = false;
  String? _error;
  double _inLevel = 0;
  double _outLevel = 0;
  late final AnimationController _breath = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 4),
  );

  // Read once: `ref` must not be used while the widget is being disposed.
  late final VoiceMic _micDev = ref.read(voiceMicProvider);
  late final VoicePlayer _player = ref.read(voicePlayerProvider);
  late final ScreenAwake _awake = ref.read(screenAwakeProvider);

  @override
  void initState() {
    super.initState();
    _micDev;
    _player;
    _awake;
    WidgetsBinding.instance.addPostFrameCallback((_) => _start());
  }

  Future<void> _start() async {
    final l = L10n.of(context);
    final mic = _micDev;
    if (!await mic.hasPermission()) {
      setState(() => _error = l.voiceNeedsMic);
      return;
    }
    try {
      final lib = (await ref.read(libraryProvider.future))!;
      final settings = await lib.aiSettings();
      final keys = await ref
          .read(credentialStoreProvider)
          .apiKeys(settings.providers.map((p) => p.id));
      final voice = await lib.startVoice(
        const VoiceOptions(
          sampleRate: 16000,
          silenceMs: 700,
          sensitivity: 0.5,
          saveTranscript: true,
        ),
        keys,
      );
      if (!mounted) {
        await voice.end();
        return;
      }
      _voice = voice;
      _events = voice.events.listen(_onEvent);
      final player = _player;
      _idle = player.idle.listen((_) {
        _outLevel = 0;
        voice.playbackFinished();
      });
      await _awake.set(true);
      final stream = await mic.start(sampleRate: 16000);
      _mic = stream.listen((bytes) {
        final samples = pcmSamples(bytes);
        voice.feed(samples);
        final lvl = pcmLevel(samples);
        if ((lvl - _inLevel).abs() > 0.02 && mounted) {
          setState(() => _inLevel = lvl);
        }
      });
    } catch (e) {
      if (mounted) setState(() => _error = humanError(e));
    }
  }

  void _onEvent(VoiceEventDto e) {
    final player = _player;
    switch (e.kind) {
      case VoiceEventKind.state:
        setState(() => _state = e.state ?? _state);
      case VoiceEventKind.userCaption:
        setState(() => _captions.add((true, e.text ?? '')));
      case VoiceEventKind.assistantCaption:
        setState(() => _captions.add((false, e.text ?? '')));
      case VoiceEventKind.audio:
        if (e.bytes != null) {
          player.enqueue(e.bytes!);
          setState(() => _outLevel = 0.6);
        }
      case VoiceEventKind.stopPlayback:
        player.stop();
        HapticFeedback.lightImpact();
        setState(() => _outLevel = 0);
      case VoiceEventKind.needsHelp:
        setState(() => _help = true);
      case VoiceEventKind.captured:
        ref.read(revisionProvider.notifier).bump();
        ref.read(syncControllerProvider.notifier).changed();
        showNote(context, L10n.of(context).voiceNoted);
      case VoiceEventKind.error:
        setState(() => _error = e.text);
    }
  }

  Future<void> _toggleMute() async {
    HapticFeedback.selectionClick();
    setState(() => _muted = !_muted);
    await _voice?.setMuted(_muted);
  }

  Future<void> _end() async {
    final l = L10n.of(context);
    HapticFeedback.mediumImpact();
    await _stopAll();
    final saved = await _voice?.end();
    _voice = null;
    if (saved != null) {
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      unawaited(ref.read(jobRunnerProvider.notifier).kick());
      if (mounted) showNote(context, l.voiceSaved);
    }
    if (mounted && context.canPop()) context.pop();
  }

  Future<void> _stopAll() async {
    // Subscriptions are dropped without waiting; the devices below are what must stop.
    unawaited(_mic?.cancel());
    _mic = null;
    await _micDev.stop();
    await _player.stop();
    unawaited(_idle?.cancel());
    unawaited(_events?.cancel());
    await _awake.set(false);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // Respect reduced motion (§11): the shape still reacts to the voice, but does not breathe.
    if (MediaQuery.maybeDisableAnimationsOf(context) ?? false) {
      _breath.stop();
    } else if (!_breath.isAnimating) {
      _breath.repeat();
    }
  }

  @override
  void dispose() {
    _breath.dispose();
    if (_voice != null) {
      _stopAll();
      _voice!.end();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final label = switch (_state) {
      VoiceStateDto.listening || VoiceStateDto.hearing => l.voiceListening,
      VoiceStateDto.thinking => l.voiceThinking,
      VoiceStateDto.speaking => l.voiceSpeaking,
      VoiceStateDto.muted => l.voiceMuted,
    };
    final level = _state == VoiceStateDto.speaking ? _outLevel : _inLevel;
    final recent = _captions.length > 4
        ? _captions.sublist(_captions.length - 4)
        : _captions;
    return Material(
      color: p.paper,
      child: SafeArea(
        child: Column(
          children: [
            Align(
              alignment: AlignmentDirectional.centerEnd,
              child: Padding(
                padding: const EdgeInsets.all(Space.x2),
                child: DChip(
                  label: l.captions,
                  selected: _showCaptions,
                  onTap: () => setState(() => _showCaptions = !_showCaptions),
                ),
              ),
            ),
            Expanded(
              flex: 3,
              child: Center(
                child: Semantics(
                  label: label,
                  liveRegion: true,
                  child: AnimatedBuilder(
                    animation: _breath,
                    builder: (context, _) => CustomPaint(
                      size: const Size.square(220),
                      painter: _Shape(
                        phase:
                            MediaQuery.maybeDisableAnimationsOf(context) ??
                                false
                            ? 0
                            : _breath.value,
                        level: level,
                        state: _state,
                        color: _muted ? p.inkFaint : p.accent,
                        soft: p.accentSoft,
                      ),
                    ),
                  ),
                ),
              ),
            ),
            Text(
              label,
              style: context.type.heading.copyWith(color: p.inkMuted),
            ),
            const SizedBox(height: Space.x4),
            Expanded(
              flex: 2,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: Space.x6),
                child: ListView(
                  reverse: true,
                  children: [
                    if (_error != null)
                      Text(
                        _error!,
                        textAlign: TextAlign.center,
                        style: context.type.small.copyWith(color: p.critical),
                      ),
                    if (_help)
                      const Padding(
                        padding: EdgeInsets.only(bottom: Space.x3),
                        child: TalkToSomeoneCard(),
                      ),
                    if (_showCaptions)
                      for (final (user, text) in recent.reversed)
                        Padding(
                          padding: const EdgeInsets.only(bottom: Space.x2),
                          child: Text(
                            text,
                            textAlign: TextAlign.center,
                            textDirection: directionOf(
                              text,
                              fallback: Directionality.of(context),
                            ),
                            style:
                                (user ? context.type.small : context.type.body)
                                    .copyWith(color: user ? p.inkMuted : p.ink),
                          ),
                        ),
                  ],
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(
                Space.x8,
                Space.x4,
                Space.x8,
                Space.x8,
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  _RoundButton(
                    icon: DIcons.mic,
                    label: _muted ? l.unmute : l.mute,
                    filled: _muted,
                    onTap: _toggleMute,
                  ),
                  _RoundButton(
                    icon: DIcons.close,
                    label: l.endConversation,
                    filled: true,
                    critical: true,
                    onTap: _end,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _RoundButton extends StatelessWidget {
  const _RoundButton({
    required this.icon,
    required this.label,
    required this.onTap,
    this.filled = false,
    this.critical = false,
  });
  final DIcons icon;
  final String label;
  final VoidCallback onTap;
  final bool filled;
  final bool critical;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final bg = critical ? p.critical : (filled ? p.ink : p.raised);
    final fg = critical || filled ? p.paper : p.ink;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Pressable(
          onPressed: onTap,
          semanticLabel: label,
          radius: Radii.pill,
          child: Container(
            width: 64,
            height: 64,
            decoration: BoxDecoration(
              color: bg,
              shape: BoxShape.circle,
              border: Border.all(color: p.hairline),
            ),
            child: Center(child: DIcon(icon, size: 26, color: fg)),
          ),
        ),
        const SizedBox(height: Space.x2),
        Text(label, style: context.type.caption.copyWith(color: p.inkMuted)),
      ],
    );
  }
}

/// A single ink blot that breathes while listening, swells with the voice, and ripples while
/// thinking. No sparkle, no gradients: one colour and its soft tint.
class _Shape extends CustomPainter {
  _Shape({
    required this.phase,
    required this.level,
    required this.state,
    required this.color,
    required this.soft,
  });
  final double phase;
  final double level;
  final VoiceStateDto state;
  final Color color;
  final Color soft;

  @override
  void paint(Canvas canvas, Size size) {
    final c = size.center(Offset.zero);
    final base = size.width * 0.28;
    final breathe = math.sin(phase * 2 * math.pi) * 0.04;
    final r = base * (1 + breathe + level * 0.35);
    canvas.drawCircle(c, r * 1.35, Paint()..color = soft);
    final path = Path();
    const n = 72;
    final wobble = switch (state) {
      VoiceStateDto.thinking => 0.06,
      VoiceStateDto.speaking || VoiceStateDto.hearing => 0.03 + level * 0.08,
      _ => 0.02,
    };
    for (var i = 0; i <= n; i++) {
      final a = i / n * 2 * math.pi;
      final rr =
          r *
          (1 +
              wobble * math.sin(3 * a + phase * 2 * math.pi) +
              wobble * 0.5 * math.cos(5 * a - phase * 4 * math.pi));
      final pt = c + Offset(math.cos(a) * rr, math.sin(a) * rr);
      i == 0 ? path.moveTo(pt.dx, pt.dy) : path.lineTo(pt.dx, pt.dy);
    }
    canvas.drawPath(path..close(), Paint()..color = color);
  }

  @override
  bool shouldRepaint(_Shape old) =>
      old.phase != phase ||
      old.level != level ||
      old.state != state ||
      old.color != color;
}
