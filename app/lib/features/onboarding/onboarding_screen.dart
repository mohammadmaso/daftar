import 'dart:io';

import 'package:flutter/material.dart' show Material, SelectableText;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/identity.dart';
import '../../core/errors.dart';
import '../../core/library_api.dart';
import '../../core/library_state.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

enum _Step { welcome, connect, device, working }

enum ConnectMode { https, ssh }

/// First run: connect a private repository (or start local-only), name the device, clone.
class OnboardingScreen extends ConsumerStatefulWidget {
  const OnboardingScreen({super.key});

  @override
  ConsumerState<OnboardingScreen> createState() => _OnboardingScreenState();
}

class _OnboardingScreenState extends ConsumerState<OnboardingScreen> {
  _Step _step = _Step.welcome;
  bool _local = false;
  final _form = ConnectForm();
  final _device = TextEditingController(text: _defaultDeviceName());
  String? _error;

  static String _defaultDeviceName() {
    if (Platform.isAndroid) return 'Android phone';
    if (Platform.isIOS) return 'iPhone';
    if (Platform.isMacOS) return 'Mac';
    if (Platform.isWindows) return 'Windows PC';
    return 'Linux laptop';
  }

  @override
  void dispose() {
    _form.dispose();
    _device.dispose();
    super.dispose();
  }

  Future<void> _finish() async {
    setState(() {
      _step = _Step.working;
      _error = null;
    });
    final setup = ref.read(setupApiProvider);
    final root = await ref.read(libraryRootProvider.future);
    final name = _device.text.trim().isEmpty
        ? _defaultDeviceName()
        : _device.text.trim();
    try {
      if (_local) {
        await setup.initLocal(
          root,
          deviceName: name,
          platform: currentPlatform(),
        );
      } else {
        final auth = _form.auth();
        await setup.clone(
          url: _form.url.text.trim(),
          root: root,
          auth: auth,
          branch: _form.branch.text.trim().isEmpty
              ? 'main'
              : _form.branch.text.trim(),
          deviceName: name,
          platform: currentPlatform(),
        );
        await ref.read(credentialStoreProvider).saveGitAuth(auth);
      }
      await ref.read(libraryProvider.notifier).reload();
    } catch (e) {
      setState(() => _error = humanError(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    final locale = Localizations.localeOf(context);
    final body = switch (_step) {
      _Step.welcome => _Welcome(
        name: AppIdentity.name(locale),
        onConnect: () => setState(() {
          _local = false;
          _step = _Step.connect;
        }),
        onLocal: () => setState(() {
          _local = true;
          _step = _Step.device;
        }),
      ),
      _Step.connect => _Panel(
        title: l.connectTitle,
        onBack: () => setState(() => _step = _Step.welcome),
        children: [
          ConnectFields(form: _form),
          const SizedBox(height: Space.x6),
          ListenableBuilder(
            listenable: _form,
            builder: (context, _) => DButton(
              label: l.continueAction,
              onPressed: _form.valid
                  ? () => setState(() => _step = _Step.device)
                  : null,
            ),
          ),
        ],
      ),
      _Step.device => _Panel(
        title: l.deviceTitle,
        onBack: () =>
            setState(() => _step = _local ? _Step.welcome : _Step.connect),
        children: [
          Text(
            l.deviceHelp,
            style: context.type.small.copyWith(color: p.inkMuted),
          ),
          const SizedBox(height: Space.x3),
          DTextField(
            controller: _device,
            hint: l.deviceName,
            autofocus: true,
            onSubmitted: (_) => _finish(),
          ),
          const SizedBox(height: Space.x6),
          DButton(label: l.continueAction, onPressed: _finish),
        ],
      ),
      _Step.working => _Panel(
        title: _local ? l.preparing : l.connecting,
        onBack: _error == null
            ? null
            : () =>
                  setState(() => _step = _local ? _Step.device : _Step.connect),
        children: [
          if (_error == null)
            const _Progress()
          else ...[
            Text(_error!, style: context.type.body.copyWith(color: p.critical)),
            const SizedBox(height: Space.x4),
            DButton(label: l.tryAgain, onPressed: _finish),
          ],
        ],
      ),
    };
    return Material(
      color: p.paper,
      child: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: AnimatedSwitcher(
              duration: motion(context, Motion.standard),
              child: KeyedSubtree(key: ValueKey(_step), child: body),
            ),
          ),
        ),
      ),
    );
  }
}

class _Welcome extends StatelessWidget {
  const _Welcome({
    required this.name,
    required this.onConnect,
    required this.onLocal,
  });
  final String name;
  final VoidCallback onConnect;
  final VoidCallback onLocal;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final p = context.palette;
    return Padding(
      padding: const EdgeInsets.all(Space.x6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Spacer(flex: 3),
          Text(
            name,
            style: context.type.display.copyWith(
              fontSize: context.type.display.fontSize! * 1.4,
              color: p.accent,
            ),
          ),
          const SizedBox(height: Space.x3),
          Text(l.welcomeLine, style: context.type.title),
          const SizedBox(height: Space.x2),
          Text(
            l.welcomeBody,
            style: context.type.body.copyWith(color: p.inkMuted),
          ),
          const Spacer(flex: 4),
          DButton(label: l.connectRepo, onPressed: onConnect),
          const SizedBox(height: Space.x2),
          DButton(
            label: l.startLocal,
            variant: DButtonVariant.quiet,
            onPressed: onLocal,
          ),
        ],
      ),
    );
  }
}

class _Panel extends StatelessWidget {
  const _Panel({required this.title, required this.children, this.onBack});
  final String title;
  final List<Widget> children;
  final VoidCallback? onBack;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    return ListView(
      padding: const EdgeInsets.all(Space.x6),
      children: [
        Row(
          children: [
            if (onBack != null) ...[
              DIconButton(
                icon: DIcons.back,
                onPressed: onBack,
                semanticLabel: l.back,
              ),
              const SizedBox(width: Space.x1),
            ],
            Expanded(
              child: Semantics(
                header: true,
                child: Text(title, style: context.type.title),
              ),
            ),
          ],
        ),
        const SizedBox(height: Space.x6),
        ...children,
      ],
    );
  }
}

class _Progress extends StatefulWidget {
  const _Progress();

  @override
  State<_Progress> createState() => _ProgressState();
}

class _ProgressState extends State<_Progress>
    with SingleTickerProviderStateMixin {
  late final _c = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1400),
  )..repeat();

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return ClipRRect(
      borderRadius: BorderRadius.circular(Radii.pill),
      child: SizedBox(
        height: 3,
        child: AnimatedBuilder(
          animation: _c,
          builder: (context, _) => LayoutBuilder(
            builder: (context, c) => Stack(
              children: [
                Positioned.fill(child: ColoredBox(color: p.sunken)),
                PositionedDirectional(
                  start: (c.maxWidth + 80) * _c.value - 80,
                  width: 80,
                  top: 0,
                  bottom: 0,
                  child: ColoredBox(color: p.accent),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ─────────────────────────── connect form (shared with Settings) ───────────────────────────

/// Plain HTTP is accepted only for loopback remotes (local test servers, `adb reverse`).
bool isLoopbackHttp(String url) {
  final u = Uri.tryParse(url);
  return u != null &&
      u.scheme == 'http' &&
      (u.host == '127.0.0.1' || u.host == 'localhost' || u.host == '::1');
}

class ConnectForm extends ChangeNotifier {
  ConnectForm() {
    for (final c in [url, token, branch]) {
      c.addListener(notifyListeners);
    }
  }

  ConnectMode mode = ConnectMode.https;
  final url = TextEditingController();
  final token = TextEditingController();
  final branch = TextEditingController(text: 'main');
  SshKeyPair? key;

  void setMode(ConnectMode m) {
    mode = m;
    notifyListeners();
  }

  void setKey(SshKeyPair k) {
    key = k;
    notifyListeners();
  }

  bool get valid {
    final u = url.text.trim();
    return switch (mode) {
      ConnectMode.https =>
        (u.startsWith('https://') || isLoopbackHttp(u)) &&
            token.text.trim().isNotEmpty,
      ConnectMode.ssh =>
        (u.startsWith('git@') || u.startsWith('ssh://')) && key != null,
    };
  }

  Auth auth() => switch (mode) {
    ConnectMode.https => Auth(kind: AuthKind.token, secret: token.text.trim()),
    ConnectMode.ssh => Auth(kind: AuthKind.sshKey, secret: key!.privateOpenssh),
  };

  @override
  void dispose() {
    url.dispose();
    token.dispose();
    branch.dispose();
    super.dispose();
  }
}

class ConnectFields extends ConsumerWidget {
  const ConnectFields({super.key, required this.form});
  final ConnectForm form;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = L10n.of(context);
    final p = context.palette;
    return ListenableBuilder(
      listenable: form,
      builder: (context, _) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          DSegmented<ConnectMode>(
            value: form.mode,
            onChanged: form.setMode,
            segments: [
              DSegment(ConnectMode.https, l.connectHttps),
              DSegment(ConnectMode.ssh, l.connectSsh),
            ],
          ),
          const SizedBox(height: Space.x5),
          _Label(l.repoUrl),
          DTextField(
            controller: form.url,
            hint: form.mode == ConnectMode.https
                ? l.repoUrlHintHttps
                : l.repoUrlHintSsh,
            forceLtr: true,
            keyboardType: TextInputType.url,
          ),
          const SizedBox(height: Space.x4),
          if (form.mode == ConnectMode.https) ...[
            _Label(l.accessToken),
            DTextField(controller: form.token, obscure: true, forceLtr: true),
            const SizedBox(height: Space.x2),
            Text(
              l.tokenHelp,
              style: context.type.small.copyWith(color: p.inkMuted),
            ),
          ] else if (form.key == null)
            DButton(
              label: l.generateKey,
              variant: DButtonVariant.secondary,
              onPressed: () async {
                final k = await ref
                    .read(setupApiProvider)
                    .generateSshKey(
                      '${AppIdentity.nameEn.toLowerCase()}@${currentPlatform()}',
                    );
                form.setKey(k);
              },
            )
          else ...[
            _Label(l.publicKey),
            DSurface(
              padding: const EdgeInsets.all(Space.x3),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SelectableText(
                    form.key!.publicOpenssh,
                    style: TypeScale.mono.copyWith(fontSize: 12, color: p.ink),
                    textDirection: TextDirection.ltr,
                  ),
                  const SizedBox(height: Space.x2),
                  Align(
                    alignment: AlignmentDirectional.centerEnd,
                    child: DButton(
                      label: l.copy,
                      variant: DButtonVariant.secondary,
                      onPressed: () {
                        Clipboard.setData(
                          ClipboardData(text: form.key!.publicOpenssh),
                        );
                        showNote(context, l.copied);
                      },
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: Space.x2),
            Text(
              l.sshHelp,
              style: context.type.small.copyWith(color: p.inkMuted),
            ),
          ],
          const SizedBox(height: Space.x4),
          _Label(l.branch),
          DTextField(controller: form.branch, forceLtr: true),
        ],
      ),
    );
  }
}

class _Label extends StatelessWidget {
  const _Label(this.text);
  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: Space.x2),
    child: Text(
      text,
      style: context.type.label.copyWith(color: context.palette.inkMuted),
    ),
  );
}
