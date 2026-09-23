import 'package:flutter/widgets.dart';

import '../theme.dart';

/// Identity mark: first letter of the product name in an ink ring.
class Monogram extends StatelessWidget {
  const Monogram({super.key, required this.name, this.size = 36});
  final String name;
  final double size;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return ExcludeSemantics(
      child: Container(
        width: size,
        height: size,
        alignment: Alignment.center,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          border: Border.all(color: p.accent, width: 1.5),
        ),
        child: Text(
          name.characters.first,
          style: context.type.heading.copyWith(color: p.accent, height: 1.1),
        ),
      ),
    );
  }
}
