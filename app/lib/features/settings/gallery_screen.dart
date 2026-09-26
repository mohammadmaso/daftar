import 'package:flutter/widgets.dart';
import 'package:go_router/go_router.dart';

import '../../app/identity.dart';
import '../../design/design.dart';
import '../../l10n/app_localizations.dart';

/// Living style guide of the design system (preview builds only). Also the subject of the
/// component golden tests that feed docs/design/.
class GalleryScreen extends StatefulWidget {
  const GalleryScreen({super.key});

  @override
  State<GalleryScreen> createState() => _GalleryScreenState();
}

class _GalleryScreenState extends State<GalleryScreen> {
  String _vault = 'life';
  bool _notify = true;
  int _segment = 0;

  @override
  Widget build(BuildContext context) {
    final l = L10n.of(context);
    final t = context.type;
    final p = context.palette;
    return DPage(
      title: l.designSystem,
      onBack: () => context.canPop() ? context.pop() : context.go('/'),
      backLabel: l.back,
      children: [
        DSection(
          title: l.galleryBrand,
          children: [
            const Padding(
              padding: EdgeInsets.all(Space.x4),
              child: Wrap(
                spacing: Space.x6,
                runSpacing: Space.x4,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  Logo(size: 56),
                  Wordmark(name: AppIdentity.nameEn, latinFont: LatinFont.sans),
                  Wordmark(
                    name: AppIdentity.nameEn,
                    latinFont: LatinFont.serif,
                  ),
                  Wordmark(name: AppIdentity.nameFa),
                ],
              ),
            ),
          ],
        ),
        DSection(
          title: l.galleryType,
          children: [
            Padding(
              padding: const EdgeInsets.all(Space.x4),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(l.sampleHeading, style: t.title),
                  const SizedBox(height: Space.x2),
                  Text(l.sampleBody, style: t.body),
                  const SizedBox(height: Space.x2),
                  Text(l.sampleMixed, style: t.body),
                  const SizedBox(height: Space.x2),
                  Text(
                    'page_edit(path, base_hash)',
                    style: TypeScale.mono.copyWith(color: p.inkMuted),
                    textDirection: TextDirection.ltr,
                  ),
                ],
              ),
            ),
          ],
        ),
        DSection(
          title: l.galleryButtons,
          children: [
            Padding(
              padding: const EdgeInsets.all(Space.x4),
              child: Wrap(
                spacing: Space.x2,
                runSpacing: Space.x2,
                children: [
                  DButton(label: l.sampleSave, onPressed: () {}),
                  DButton(
                    label: l.sampleRetry,
                    variant: DButtonVariant.secondary,
                    onPressed: () {},
                  ),
                  DButton(
                    label: l.sampleCancel,
                    variant: DButtonVariant.quiet,
                    onPressed: () {},
                  ),
                  DButton(label: l.sampleSave, onPressed: null),
                ],
              ),
            ),
          ],
        ),
        DSection(
          title: l.galleryControls,
          children: [
            Padding(
              padding: const EdgeInsets.all(Space.x4),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  DTextField(hint: l.sampleSearch, leading: DIcons.search),
                  const SizedBox(height: Space.x3),
                  Wrap(
                    spacing: Space.x2,
                    children: [
                      for (final v in const ['life', 'health', 'mind', 'work'])
                        DChip(
                          label: v,
                          selected: _vault == v,
                          onTap: () => setState(() => _vault = v),
                        ),
                    ],
                  ),
                  const SizedBox(height: Space.x3),
                  DSegmented<int>(
                    value: _segment,
                    onChanged: (v) => setState(() => _segment = v),
                    segments: [
                      DSegment(0, l.themeSystem),
                      DSegment(1, l.themeLight),
                      DSegment(2, l.themeDark),
                    ],
                  ),
                ],
              ),
            ),
            DListRow(
              title: l.sampleNotify,
              trailing: DSwitch(
                value: _notify,
                onChanged: (v) => setState(() => _notify = v),
                semanticLabel: l.sampleNotify,
              ),
            ),
          ],
        ),
        DSection(
          title: l.galleryStatus,
          children: [
            Padding(
              padding: const EdgeInsets.all(Space.x4),
              child: Wrap(
                spacing: Space.x2,
                children: [
                  DStatusPill(
                    status: ClaimStatus.confirmed,
                    label: l.statusConfirmed,
                  ),
                  DStatusPill(
                    status: ClaimStatus.proposed,
                    label: l.statusProposed,
                  ),
                  DStatusPill(
                    status: ClaimStatus.superseded,
                    label: l.statusSuperseded,
                  ),
                ],
              ),
            ),
          ],
        ),
        DSection(
          title: l.galleryIcons,
          children: [
            Padding(
              padding: const EdgeInsets.all(Space.x4),
              child: Wrap(
                spacing: Space.x4,
                runSpacing: Space.x4,
                children: [for (final i in DIcons.values) DIcon(i)],
              ),
            ),
          ],
        ),
      ],
    );
  }
}
