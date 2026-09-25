import 'dart:async';
import 'dart:collection';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:record/record.dart';
import 'package:wakelock_plus/wakelock_plus.dart';

/// Streaming microphone for voice mode: 16 kHz mono PCM with the platform's voice processing
/// (echo cancellation, noise suppression) so the assistant's own voice does not trigger barge-in.
abstract class VoiceMic {
  Future<bool> hasPermission();
  Future<Stream<Uint8List>> start({int sampleRate = 16000});
  Future<void> stop();
}

class RecordVoiceMic implements VoiceMic {
  final _rec = AudioRecorder();

  @override
  Future<bool> hasPermission() => _rec.hasPermission();

  @override
  Future<Stream<Uint8List>> start({int sampleRate = 16000}) => _rec.startStream(
    RecordConfig(
      encoder: AudioEncoder.pcm16bits,
      sampleRate: sampleRate,
      numChannels: 1,
      echoCancel: true,
      noiseSuppress: true,
      autoGain: true,
      androidConfig: const AndroidRecordConfig(
        audioSource: AndroidAudioSource.voiceCommunication,
      ),
      iosConfig: const IosRecordConfig(
        categoryOptions: [
          IosAudioCategoryOption.defaultToSpeaker,
          IosAudioCategoryOption.allowBluetooth,
          IosAudioCategoryOption.allowBluetoothA2DP,
        ],
      ),
    ),
  );

  @override
  Future<void> stop() async {
    await _rec.stop();
  }
}

/// Plays sentence audio in order; can be cut off at once for barge-in.
abstract class VoicePlayer {
  void enqueue(Uint8List bytes);

  /// Stops now and drops everything queued.
  Future<void> stop();

  /// Fires when the queue has been played to the end.
  Stream<void> get idle;

  Future<void> dispose();
}

class QueuedVoicePlayer implements VoicePlayer {
  QueuedVoicePlayer() {
    _sub = _player.onPlayerComplete.listen((_) => _next());
  }

  final _player = AudioPlayer();
  final _queue = Queue<Uint8List>();
  final _idle = StreamController<void>.broadcast();
  late final StreamSubscription<void> _sub;
  bool _playing = false;

  @override
  void enqueue(Uint8List bytes) {
    _queue.add(bytes);
    if (!_playing) _next();
  }

  Future<void> _next() async {
    if (_queue.isEmpty) {
      _playing = false;
      _idle.add(null);
      return;
    }
    _playing = true;
    await _player.play(BytesSource(_queue.removeFirst()));
  }

  @override
  Future<void> stop() async {
    _queue.clear();
    _playing = false;
    await _player.stop();
  }

  @override
  Stream<void> get idle => _idle.stream;

  @override
  Future<void> dispose() async {
    await _sub.cancel();
    await _idle.close();
    await _player.dispose();
  }
}

/// Keeps the screen on during a voice conversation (§8.4).
abstract class ScreenAwake {
  Future<void> set(bool on);
}

class WakelockScreenAwake implements ScreenAwake {
  @override
  Future<void> set(bool on) async {
    try {
      await WakelockPlus.toggle(enable: on);
    } catch (_) {
      // Not available on this platform; voice mode still works.
    }
  }
}

final voiceMicProvider = Provider<VoiceMic>((ref) => RecordVoiceMic());
final voicePlayerProvider = Provider<VoicePlayer>((ref) {
  final p = QueuedVoicePlayer();
  ref.onDispose(p.dispose);
  return p;
});
final screenAwakeProvider = Provider<ScreenAwake>(
  (ref) => WakelockScreenAwake(),
);

/// PCM16 little-endian bytes → samples.
Int16List pcmSamples(Uint8List bytes) {
  final n = bytes.lengthInBytes ~/ 2;
  final out = Int16List(n);
  final data = ByteData.sublistView(bytes);
  for (var i = 0; i < n; i++) {
    out[i] = data.getInt16(i * 2, Endian.little);
  }
  return out;
}

/// Normalised level 0..1 of a block of samples (for the voice shape): −50..0 dBFS.
double pcmLevel(Int16List s) {
  if (s.isEmpty) return 0;
  var sum = 0.0;
  for (final v in s) {
    sum += v * v;
  }
  final rms = math.sqrt(sum / s.length);
  if (rms < 1) return 0;
  final db = 20 * math.log(rms / 32768) / math.ln10;
  return ((db + 50) / 50).clamp(0.0, 1.0);
}
