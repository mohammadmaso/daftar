import 'dart:async';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:record/record.dart';

/// Microphone recording for voice captures. Faked in widget tests.
abstract class VoiceRecorder {
  Future<bool> hasPermission();

  /// Starts recording to a new temp file.
  Future<void> start();

  /// Stops and returns the file path, or null if nothing was recorded.
  Future<String?> stop();

  /// Stops and deletes the recording.
  Future<void> cancel();

  /// Normalised input level 0..1, ~20 Hz while recording.
  Stream<double> get levels;

  Future<void> dispose();
}

class RecordVoiceRecorder implements VoiceRecorder {
  final _rec = AudioRecorder();
  String? _path;

  @override
  Future<bool> hasPermission() => _rec.hasPermission();

  @override
  Future<void> start() async {
    final dir = await getTemporaryDirectory();
    _path =
        '${dir.path}${Platform.pathSeparator}capture-${DateTime.now().microsecondsSinceEpoch}.m4a';
    await _rec.start(
      const RecordConfig(
        encoder: AudioEncoder.aacLc,
        bitRate: 48000,
        sampleRate: 16000,
        numChannels: 1,
        echoCancel: true,
        noiseSuppress: true,
      ),
      path: _path!,
    );
  }

  @override
  Future<String?> stop() async => await _rec.stop() ?? _path;

  @override
  Future<void> cancel() async {
    await _rec.cancel();
    final p = _path;
    if (p != null && File(p).existsSync()) File(p).deleteSync();
  }

  // dBFS −50..0 mapped to 0..1; below −50 is silence.
  @override
  Stream<double> get levels => _rec
      .onAmplitudeChanged(const Duration(milliseconds: 50))
      .map((a) => ((a.current + 50) / 50).clamp(0.0, 1.0));

  @override
  Future<void> dispose() => _rec.dispose();
}

final voiceRecorderProvider = Provider<VoiceRecorder>((ref) {
  final r = RecordVoiceRecorder();
  ref.onDispose(r.dispose);
  return r;
});
