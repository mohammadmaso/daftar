import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/credentials.dart';
import '../../core/errors.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';

@immutable
class AskTurnView {
  const AskTurnView({
    required this.question,
    this.answer = '',
    this.done = false,
    this.error,
    this.needsHelp = false,
    this.hadImage = false,
  });

  final String question;

  /// Streamed so far, then the final text with checked citations.
  final String answer;
  final bool done;
  final String? error;
  final bool needsHelp;
  final bool hadImage;

  AskTurnView copyWith({
    String? answer,
    bool? done,
    String? error,
    bool? needsHelp,
  }) => AskTurnView(
    question: question,
    answer: answer ?? this.answer,
    done: done ?? this.done,
    error: error ?? this.error,
    needsHelp: needsHelp ?? this.needsHelp,
    hadImage: hadImage,
  );
}

@immutable
class AskView {
  const AskView({
    this.turns = const [],
    this.scope = const AskScopeDto(kind: AskScopeKind.all),
  });
  final List<AskTurnView> turns;
  final AskScopeDto scope;
  bool get busy => turns.isNotEmpty && !turns.last.done;
}

final askProvider = NotifierProvider<AskThread, AskView>(AskThread.new);

/// One conversation (§4.3), kept on this device while the app runs. Answers become part of the
/// wiki only through "Save to wiki".
class AskThread extends Notifier<AskView> {
  StreamSubscription<AskEvent>? _sub;

  @override
  AskView build() {
    ref.onDispose(() => _sub?.cancel());
    return const AskView();
  }

  void setScope(AskScopeDto scope) {
    if (scope.kind == state.scope.kind && scope.id == state.scope.id) return;
    _sub?.cancel();
    // A new scope is a new conversation: a story answer must not leak into a personal one.
    state = AskView(scope: scope);
  }

  void clear() {
    _sub?.cancel();
    state = AskView(scope: state.scope);
  }

  Future<void> send(
    String question, {
    Uint8List? image,
    String imageType = 'image/jpeg',
    String? reading,
  }) async {
    final q = question.trim();
    if (q.isEmpty || state.busy) return;
    final lib = await ref.read(libraryProvider.future);
    if (lib == null) return;
    final history = [
      for (final t in state.turns)
        if (t.done && t.error == null)
          AskTurn(question: t.question, answer: t.answer),
    ];
    state = AskView(
      scope: state.scope,
      turns: [
        ...state.turns,
        AskTurnView(question: q, hadImage: image != null),
      ],
    );
    final settings = await lib.aiSettings();
    final keys = await ref
        .read(credentialStoreProvider)
        .apiKeys(settings.providers.map((p) => p.id));
    // On desktop the Ask panel knows which page is open (§8.2).
    final asked = reading == null
        ? q
        : 'I am reading [[${reading.replaceAll('.md', '')}]].\n\n$q';
    final buffer = StringBuffer();
    void update(AskTurnView Function(AskTurnView) f) {
      final turns = [...state.turns];
      turns[turns.length - 1] = f(turns.last);
      state = AskView(scope: state.scope, turns: turns);
    }

    // A finished stream's cancel may never complete; don't wait for it.
    unawaited(_sub?.cancel());
    final done = Completer<void>();
    _sub = lib
        .ask(
          history,
          asked,
          state.scope,
          keys,
          image: image == null
              ? null
              : AskImage(mediaType: imageType, bytes: image),
        )
        .listen(
          (e) {
            switch (e.kind) {
              case AskEventKind.delta:
                buffer.write(e.text ?? '');
                update((t) => t.copyWith(answer: buffer.toString()));
              case AskEventKind.done:
                final a = e.answer!;
                update(
                  (t) => t.copyWith(
                    answer: a.text,
                    done: true,
                    needsHelp: a.needsHelp,
                  ),
                );
              case AskEventKind.failed:
                update((t) => t.copyWith(done: true, error: e.text ?? ''));
            }
          },
          onError: (Object err) =>
              update((t) => t.copyWith(done: true, error: humanError(err))),
          onDone: () {
            if (!state.turns.last.done) update((t) => t.copyWith(done: true));
            done.complete();
          },
        );
    return done.future;
  }
}
