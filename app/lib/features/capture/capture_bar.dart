import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';

import '../../core/bidi.dart';
import '../../core/job_runner.dart';
import '../../core/library_state.dart';
import '../../core/recorder.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// Vault pinned for the next capture only (ADR-0012). `null` = automatic routing.
final pinnedVaultProvider = NotifierProvider<PinnedVault, String?>(
  PinnedVault.new,
);

class PinnedVault extends Notifier<String?> {
  @override
  String? build() => null;
  void set(String? v) => state = v;
  String? take() {
    final v = state;
    state = null;
    return v;
  }
}

enum _RecState { idle, recording, locked }

/// Bottom capture controls: vault pin · hold-to-record / tap-to-type · camera.
class CaptureBar extends ConsumerStatefulWidget {
  const CaptureBar({super.key});

  @override
  ConsumerState<CaptureBar> createState() => _CaptureBarState();
}

class _CaptureBarState extends ConsumerState<CaptureBar> {
  static const _cancelDistance = 96.0;
  static const _lockDistance = 72.0;

  _RecState _rec = _RecState.idle;
  Offset _drag = Offset.zero;
  final _levels = <double>[];
  StreamSubscription<double>? _levelSub;
  static const _tickEvery = Duration(milliseconds: 100);
  Duration _elapsed = Duration.zero;
  Timer? _tick;
  bool _starting = false;

  VoiceRecorder get _recorder => ref.read(voiceRecorderProvider);

  @override
  void dispose() {
    _levelSub?.cancel();
    _tick?.cancel();
    super.dispose();
  }

  Future<void> _afterCapture(String message) async {
    HapticFeedback.lightImpact();
    ref.read(revisionProvider.notifier).bump();
    ref.read(syncControllerProvider.notifier).changed();
    ref.read(jobRunnerProvider.notifier).kick();
    if (mounted) showNote(context, message);
  }

  String _savedMessage(String? vault) {
    final l = L10n.of(context);
    if (vault == null) return l.saved;
    final vaults = ref.read(vaultsProvider).value ?? const [];
    final lang = Localizations.localeOf(context).languageCode;
    final v = vaults.where((x) => x.id == vault).firstOrNull;
    return l.savedTo(
      v == null ? vault : (lang == 'fa' ? v.titleFa : v.titleEn),
    );
  }

  // ─────────────── voice ───────────────

  Future<void> _startRecording() async {
    if (_starting || _rec != _RecState.idle) return;
    _starting = true;
    final l = L10n.of(context);
    try {
      if (!await _recorder.hasPermission()) {
        if (mounted) showNote(context, l.micDenied);
        return;
      }
      await _recorder.start();
      HapticFeedback.mediumImpact();
      _levels.clear();
      _levelSub = _recorder.levels.listen((v) {
        setState(() {
          _levels.add(v);
          if (_levels.length > 48) _levels.removeAt(0);
        });
      });
      _elapsed = Duration.zero;
      _tick = Timer.periodic(
        _tickEvery,
        (_) => setState(() => _elapsed += _tickEvery),
      );
      setState(() {
        _rec = _RecState.recording;
        _drag = Offset.zero;
      });
    } catch (_) {
      if (mounted) showNote(context, l.recordingFailed);
    } finally {
      _starting = false;
    }
  }

  void _stopUi() {
    _levelSub?.cancel();
    _tick?.cancel();
    setState(() => _rec = _RecState.idle);
  }

  Future<void> _finishRecording() async {
    if (_rec == _RecState.idle) return;
    _stopUi();
    final path = await _recorder.stop();
    if (path == null || _elapsed < const Duration(milliseconds: 600)) {
      await _recorder.cancel();
      return;
    }
    final lib = await ref.read(libraryProvider.future);
    if (lib == null) return;
    final vault = ref.read(pinnedVaultProvider.notifier).take();
    await lib.captureVoice(path, vault: vault);
    await _afterCapture(_savedMessage(vault));
  }

  Future<void> _cancelRecording() async {
    if (_rec == _RecState.idle) return;
    _stopUi();
    await _recorder.cancel();
    HapticFeedback.selectionClick();
    if (mounted) showNote(context, L10n.of(context).recordingCancelled);
  }

  bool _isRtl() => Directionality.of(context) == TextDirection.rtl;

  void _onDrag(Offset fromOrigin) {
    if (_rec != _RecState.recording) return;
    setState(() => _drag = fromOrigin);
    final towardStart = _isRtl() ? _drag.dx : -_drag.dx;
    if (towardStart > _cancelDistance) {
      _cancelRecording();
    } else if (-_drag.dy > _lockDistance) {
      HapticFeedback.mediumImpact();
      setState(() => _rec = _RecState.locked);
    }
  }

  // ─────────────── text & photo ───────────────

  Future<void> _typeText() async {
    final text = await showDSheet<String>(
      context,
      builder: (_) => const _TextSheet(),
    );
    if (text == null || text.trim().isEmpty) return;
    final lib = await ref.read(libraryProvider.future);
    if (lib == null) return;
    final vault = ref.read(pinnedVaultProvider.notifier).take();
    await lib.captureText(text.trim(), vault: vault);
    await _afterCapture(_savedMessage(vault));
  }

  Future<void> _takePhoto() async {
    final picker = ImagePicker();
    final source = picker.supportsImageSource(ImageSource.camera)
        ? ImageSource.camera
        : ImageSource.gallery;
    final file = await picker.pickImage(
      source: source,
      requestFullMetadata: false,
    );
    if (file == null) return;
    final bytes = await file.readAsBytes();
    final lib = await ref.read(libraryProvider.future);
    if (lib == null) return;
    final vault = ref.read(pinnedVaultProvider.notifier).take();
    await lib.capturePhoto(bytes, vault: vault);
    await _afterCapture(_savedMessage(vault));
  }

  Future<void> _pickVault() async {
    final picked = await showDSheet<({String? id})>(
      context,
      builder: (_) => const _VaultSheet(),
    );
    if (picked != null) ref.read(pinnedVaultProvider.notifier).set(picked.id);
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final recording = _rec != _RecState.idle;
    final pinned = ref.watch(pinnedVaultProvider);
    final vaults = ref.watch(vaultsProvider).value ?? const [];
    final lang = Localizations.localeOf(context).languageCode;
    final pinnedLabel = pinned == null
        ? l.vaultAuto
        : (vaults
                  .where((v) => v.id == pinned)
                  .map((v) => lang == 'fa' ? v.titleFa : v.titleEn)
                  .firstOrNull ??
              pinned);

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        AnimatedSize(
          duration: motion(context, Motion.standard),
          curve: Motion.ease,
          child: recording
              ? _RecordingPanel(
                  levels: _levels,
                  elapsed: _elapsed,
                  locked: _rec == _RecState.locked,
                  drag: _drag,
                  onSave: _finishRecording,
                  onCancel: _cancelRecording,
                )
              : const SizedBox(width: double.infinity),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(
            Space.x4,
            Space.x2,
            Space.x4,
            Space.x4,
          ),
          child: Row(
            children: [
              Expanded(
                child: Align(
                  alignment: AlignmentDirectional.centerStart,
                  child: recording
                      ? const SizedBox.shrink()
                      : DChip(
                          label: pinnedLabel,
                          selected: pinned != null,
                          onTap: _pickVault,
                        ),
                ),
              ),
              Semantics(
                label: l.captureRecordLabel,
                button: true,
                child: GestureDetector(
                  onTap: recording ? null : _typeText,
                  onLongPressStart: (_) => _startRecording(),
                  onLongPressMoveUpdate: (d) => _onDrag(d.offsetFromOrigin),
                  onLongPressEnd: (_) {
                    if (_rec == _RecState.recording) _finishRecording();
                  },
                  child: _RecordButton(active: recording),
                ),
              ),
              Expanded(
                child: Align(
                  alignment: AlignmentDirectional.centerEnd,
                  child: recording
                      ? const SizedBox.shrink()
                      : DIconButton(
                          icon: DIcons.camera,
                          onPressed: _takePhoto,
                          semanticLabel: l.captureCamera,
                          color: p.inkMuted,
                        ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _RecordButton extends StatelessWidget {
  const _RecordButton({required this.active});
  final bool active;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return AnimatedContainer(
      duration: motion(context, Motion.quick),
      curve: Motion.ease,
      width: active ? 80 : 68,
      height: active ? 80 : 68,
      decoration: BoxDecoration(
        color: active ? p.critical : p.accent,
        shape: BoxShape.circle,
        border: Border.all(color: p.paper, width: 3),
      ),
      child: Center(child: DIcon(DIcons.mic, size: 28, color: p.onAccent)),
    );
  }
}

class _RecordingPanel extends StatelessWidget {
  const _RecordingPanel({
    required this.levels,
    required this.elapsed,
    required this.locked,
    required this.drag,
    required this.onSave,
    required this.onCancel,
  });

  final List<double> levels;
  final Duration elapsed;
  final bool locked;
  final Offset drag;
  final VoidCallback onSave;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final m = elapsed.inMinutes.toString().padLeft(1, '0');
    final s = (elapsed.inSeconds % 60).toString().padLeft(2, '0');
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(horizontal: Space.x4),
      padding: const EdgeInsets.all(Space.x4),
      decoration: BoxDecoration(
        color: p.raised,
        borderRadius: BorderRadius.circular(Radii.large),
        border: Border.all(color: p.hairline),
      ),
      child: Column(
        children: [
          Row(
            children: [
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: p.critical,
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: Space.x2),
              Text(
                '$m:$s',
                style: TypeScale.mono.copyWith(color: p.ink),
                textDirection: TextDirection.ltr,
              ),
              const SizedBox(width: Space.x3),
              // Waveform is media: never mirrored.
              Expanded(
                child: Directionality(
                  textDirection: TextDirection.ltr,
                  child: SizedBox(
                    height: 32,
                    child: CustomPaint(painter: _WavePainter(levels, p.accent)),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.x3),
          if (locked)
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                DButton(
                  label: l.cancel,
                  variant: DButtonVariant.quiet,
                  onPressed: onCancel,
                ),
                DButton(label: l.stop, icon: DIcons.check, onPressed: onSave),
              ],
            )
          else
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    DIcon(DIcons.back, size: 16, color: p.inkMuted),
                    const SizedBox(width: Space.x1),
                    Text(
                      l.slideToCancel,
                      style: context.type.caption.copyWith(color: p.inkMuted),
                    ),
                  ],
                ),
                Text(
                  l.slideUpToLock,
                  style: context.type.caption.copyWith(color: p.inkMuted),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

class _WavePainter extends CustomPainter {
  _WavePainter(this.levels, this.color);
  final List<double> levels;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    const bars = 48;
    final w = size.width / bars;
    final paint = Paint()
      ..color = color
      ..strokeWidth = math.max(1.5, w * 0.5)
      ..strokeCap = StrokeCap.round;
    final start = bars - levels.length;
    for (var i = 0; i < bars; i++) {
      final v = i >= start ? levels[i - start] : 0.0;
      final h = math.max(2.0, v * size.height);
      final x = i * w + w / 2;
      canvas.drawLine(
        Offset(x, (size.height - h) / 2),
        Offset(x, (size.height + h) / 2),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(_WavePainter old) => true;
}

class _TextSheet extends StatefulWidget {
  const _TextSheet();

  @override
  State<_TextSheet> createState() => _TextSheetState();
}

class _TextSheetState extends State<_TextSheet> {
  final _c = TextEditingController();
  TextDirection? _dir;

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    return Padding(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Directionality(
            textDirection: _dir ?? Directionality.of(context),
            child: DTextField(
              controller: _c,
              hint: l.typeSomething,
              autofocus: true,
              minLines: 4,
              maxLines: 12,
              keyboardType: TextInputType.multiline,
              onChanged: (t) => setState(
                () => _dir = t.trim().isEmpty ? null : directionOf(t),
              ),
            ),
          ),
          const SizedBox(height: Space.x3),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              DButton(
                label: l.cancel,
                variant: DButtonVariant.quiet,
                onPressed: () => Navigator.of(context).pop(),
              ),
              const SizedBox(width: Space.x2),
              ValueListenableBuilder(
                valueListenable: _c,
                builder: (context, v, _) => DButton(
                  label: l.save,
                  onPressed: v.text.trim().isEmpty
                      ? null
                      : () => Navigator.of(context).pop(_c.text),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _VaultSheet extends ConsumerWidget {
  const _VaultSheet();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final vaults = ref.watch(vaultsProvider).value ?? const [];
    final pinned = ref.watch(pinnedVaultProvider);
    final lang = Localizations.localeOf(context).languageCode;
    return Padding(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.pinVault, style: context.type.heading),
          const SizedBox(height: Space.x3),
          Wrap(
            spacing: Space.x2,
            runSpacing: Space.x2,
            children: [
              DChip(
                label: l.vaultAuto,
                selected: pinned == null,
                onTap: () => Navigator.of(context).pop((id: null)),
              ),
              for (final v in vaults)
                DChip(
                  label: lang == 'fa' ? v.titleFa : v.titleEn,
                  selected: pinned == v.id,
                  onTap: () => Navigator.of(context).pop((id: v.id)),
                ),
            ],
          ),
        ],
      ),
    );
  }
}
