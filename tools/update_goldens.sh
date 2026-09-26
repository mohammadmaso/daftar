#!/usr/bin/env bash
# Regenerates the golden images on Linux x86_64 (as CI renders them), from any host with Docker.
# The image needs Flutter 3.41.1 at /opt/flutter (see docs/ENVIRONMENT.md); the repository is
# copied into the container so host-side .dart_tool paths are never touched.
set -euo pipefail
IMAGE=${DAFTAR_FLUTTER_IMAGE:-daftar-flutter:3.41.1}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
docker run --rm \
  -v "$ROOT/app:/src:ro" -v "$ROOT/app/test/golden/goldens:/out" \
  "$IMAGE" bash -c '
    set -e
    mkdir -p /work && rsync -a --exclude .dart_tool --exclude build /src/ /work/
    cd /work && flutter pub get >/dev/null
    flutter test --update-goldens test/golden "$@"
    rsync -a --delete test/golden/goldens/ /out/
  ' _ "$@"
