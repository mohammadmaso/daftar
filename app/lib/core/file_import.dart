import 'dart:io';

import 'package:audioplayers/audioplayers.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:path_provider/path_provider.dart';

import '../src/rust/api/import.dart' as im;

export '../src/rust/api/import.dart' show ImportKind, ImportPreview;

/// Picking a file and reading it for the import preview (§4.1). Faked in widget tests.
abstract class FileImports {
  /// Lets the user pick one file. Returns a local path, or null when cancelled.
  Future<String?> pick();

  /// Reads the file for preview: its text, or just its size for a recording.
  Future<im.ImportPreview> read(String path);

  /// Characters of a document that are filed at most.
  int get charLimit;
}

class PlatformFileImports implements FileImports {
  @override
  Future<String?> pick() async {
    final file = await FilePicker.pickFile();
    if (file == null) return null;
    if (file.path case final p?) return p;
    // A content URI or a blob (some Android providers): copy it somewhere the core can read.
    final dir = await getTemporaryDirectory();
    final out = File(
      '${dir.path}${Platform.pathSeparator}import-'
      '${DateTime.now().microsecondsSinceEpoch}-${file.name}',
    );
    await out.writeAsBytes(await file.readAsBytes(), flush: true);
    return out.path;
  }

  @override
  Future<im.ImportPreview> read(String path) => im.readImport(path: path);

  @override
  int get charLimit => im.importCharLimit();
}

final fileImportsProvider = Provider<FileImports>(
  (ref) => PlatformFileImports(),
);

/// Plays a picked recording before it is filed. Faked in widget tests.
abstract class FilePlayer {
  Future<void> play(String path);
  Future<void> stop();

  /// True while playing; false once stopped or finished.
  Stream<bool> get playing;
  Future<void> dispose();
}

class AudioFilePlayer implements FilePlayer {
  final _player = AudioPlayer();

  @override
  Future<void> play(String path) => _player.play(DeviceFileSource(path));

  @override
  Future<void> stop() => _player.stop();

  @override
  Stream<bool> get playing =>
      _player.onPlayerStateChanged.map((s) => s == PlayerState.playing);

  @override
  Future<void> dispose() => _player.dispose();
}

final filePlayerProvider = Provider.autoDispose<FilePlayer>((ref) {
  final p = AudioFilePlayer();
  ref.onDispose(p.dispose);
  return p;
});

/// Picks a file and opens its preview. Used by the capture bar, the command palette and Ctrl/Cmd+O.
Future<void> pickAndImport(BuildContext context, ProviderContainer app) async {
  final path = await app.read(fileImportsProvider).pick();
  if (path == null || !context.mounted) return;
  openImport(context, path);
}

/// Opens the preview for a file that is already on disk (picked, shared or dropped).
void openImport(BuildContext context, String path) {
  context.push(
    Uri(path: '/import', queryParameters: {'path': path}).toString(),
  );
}
