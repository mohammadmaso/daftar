import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/global_hotkey.dart';
import '../../core/job_runner.dart';
import '../../core/library_state.dart';
import '../../core/recorder.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';
import 'capture_bar.dart';

/// The compact recorder (ADR-0028): what the window shows while it is a small floating panel
/// after the system-wide shortcut. Recording starts at once; Save (Enter, or the shortcut again)
/// files the voice note and opens the app on Today; Cancel (Esc) discards it and puts the window
/// back as it was.
class MiniRecorderScreen extends ConsumerStatefulWidget {
  const MiniRecorderScreen({super.key});

  @override
  ConsumerState<MiniRecorderScreen> createState() => _MiniRecorderScreenState();
}

enum _Mini { starting, recording, denied, failed, done }

class _MiniRecorderScreenState extends ConsumerState<MiniRecorderScreen> {
  static const _tickEvery = Duration(milliseconds: 100);

  _Mini _state = _Mini.starting;
  final _levels = <double>[];
  Duration _elapsed = Duration.zero;
  Timer? _tick;
  StreamSubscription<double>? _levelSub;
  StreamSubscription<void>? _again;
  final _focus = FocusNode();

  VoiceRecorder get _recorder => ref.read(voiceRecorderProvider);
  GlobalHotkey get _desktop => ref.read(globalHotkeyProvider);

  @override
  void initState() {
    super.initState();
    // The shortcut pressed again while recording saves.
    _again = _desktop.record.listen((_) => _save());
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _focus.requestFocus();
      _start();
    });
  }

  @override
  void dispose() {
    _tick?.cancel();
    _levelSub?.cancel();
    _again?.cancel();
    _focus.dispose();
    super.dispose();
  }

  Future<void> _start() async {
    try {
      if (!await _recorder.hasPermission()) {
        if (mounted) setState(() => _state = _Mini.denied);
        return;
      }
      await _recorder.start();
    } catch (_) {
      if (mounted) setState(() => _state = _Mini.failed);
      return;
    }
    if (!mounted) return;
    unawaited(_desktop.setRecording(true));
    _levelSub = _recorder.levels.listen((v) {
      setState(() {
        _levels.add(v);
        if (_levels.length > 48) _levels.removeAt(0);
      });
    });
    _tick = Timer.periodic(
      _tickEvery,
      (_) => setState(() => _elapsed += _tickEvery),
    );
    setState(() => _state = _Mini.recording);
  }

  void _stopUi() {
    _tick?.cancel();
    _levelSub?.cancel();
    unawaited(_desktop.setRecording(false));
  }

  Future<void> _save() async {
    if (_state != _Mini.recording) return;
    setState(() => _state = _Mini.done);
    _stopUi();
    final l = L10n.of(context);
    final path = await _recorder.stop();
    if (path == null || _elapsed < const Duration(milliseconds: 600)) {
      await _recorder.cancel();
      await _leave(open: false);
      return;
    }
    final lib = await ref.read(libraryProvider.future);
    if (lib != null) {
      final vault = ref.read(pinnedVaultProvider.notifier).take();
      await lib.captureVoice(path, vault: vault);
      HapticFeedback.lightImpact();
      ref.read(revisionProvider.notifier).bump();
      ref.read(syncControllerProvider.notifier).changed();
      ref.read(jobRunnerProvider.notifier).kick();
    }
    await _leave(open: true, note: l.saved);
  }

  Future<void> _cancel() async {
    final wasRecording = _state == _Mini.recording;
    setState(() => _state = _Mini.done);
    if (wasRecording) {
      _stopUi();
      await _recorder.cancel();
    }
    await _leave(open: false);
  }

  Future<void> _leave({required bool open, String? note}) async {
    await _desktop.leaveMiniRecorder(open: open);
    if (!mounted) return;
    context.go('/');
    if (note != null) showNote(context, note);
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final m = _elapsed.inMinutes.toString();
    final s = (_elapsed.inSeconds % 60).toString().padLeft(2, '0');
    final message = switch (_state) {
      _Mini.denied => l.micDenied,
      _Mini.failed => l.recordingFailed,
      _ => null,
    };
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.enter): _save,
        const SingleActivator(LogicalKeyboardKey.numpadEnter): _save,
        const SingleActivator(LogicalKeyboardKey.space): _save,
        const SingleActivator(LogicalKeyboardKey.escape): _cancel,
      },
      child: Focus(
        focusNode: _focus,
        child: ColoredBox(
          color: p.raised,
          child: SafeArea(
            child: Padding(
              padding: const EdgeInsets.all(Space.x4),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  if (message != null)
                    Text(
                      message,
                      style: context.type.small.copyWith(color: p.ink),
                      maxLines: 3,
                      overflow: TextOverflow.ellipsis,
                    )
                  else
                    Row(
                      children: [
                        Container(
                          width: 8,
                          height: 8,
                          decoration: BoxDecoration(
                            color: _state == _Mini.recording
                                ? p.critical
                                : p.inkMuted,
                            shape: BoxShape.circle,
                          ),
                        ),
                        const SizedBox(width: Space.x2),
                        Semantics(
                          label: l.miniRecording,
                          child: Text(
                            '$m:$s',
                            style: TypeScale.mono.copyWith(color: p.ink),
                            textDirection: TextDirection.ltr,
                          ),
                        ),
                        const SizedBox(width: Space.x3),
                        Expanded(
                          child: Directionality(
                            textDirection: TextDirection.ltr,
                            child: SizedBox(
                              height: 32,
                              child: CustomPaint(
                                painter: WavePainter(_levels, p.accent),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  const SizedBox(height: Space.x3),
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          message == null ? l.miniHint : '',
                          style: context.type.caption.copyWith(
                            color: p.inkMuted,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      DButton(
                        label: l.cancel,
                        variant: DButtonVariant.quiet,
                        onPressed: _cancel,
                      ),
                      if (message == null) ...[
                        const SizedBox(width: Space.x2),
                        DButton(
                          label: l.stop,
                          icon: DIcons.check,
                          onPressed: _state == _Mini.recording ? _save : null,
                        ),
                      ],
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
