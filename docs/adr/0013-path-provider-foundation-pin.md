# ADR-0013: Pin path_provider_foundation to 2.4.x

* Status: accepted (revisit at M9)
* Date: 2026-09-23

path_provider_foundation 2.6 moved to FFI via `objective_c`, whose native-assets build hook runs on
every host and requires `ld.lld` next to the discovered clang, even when building for Linux. On a
stock Ubuntu with clang-18 but without `lld-18`, `flutter build linux` fails. Pinning the Apple
implementation to 2.4.x (pigeon-based, functionally identical for our use) keeps every desktop and CI
image buildable without extra system packages. Remove the pin once the hook skips non-Apple targets.
