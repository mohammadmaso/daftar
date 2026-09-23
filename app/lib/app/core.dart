import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../src/rust/api/info.dart';

export '../src/rust/api/info.dart' show CoreInfo;

/// Build info of the linked Rust core. Overridden in tests so widget tests need no native lib.
final coreInfoProvider = Provider<CoreInfo>((ref) => coreInfo());
