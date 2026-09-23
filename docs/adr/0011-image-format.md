# ADR-0011: Capture images are stored as JPEG q80

* Status: accepted
* Date: 2026-09-23

§3.1 allows WebP or JPEG at ~q80. The `image` crate only encodes lossless WebP; lossy WebP would need
libwebp bindings on five platforms. JPEG q80 at ≤ 1600 px is ~150–350 KB, universally viewable in
Obsidian and on Git hosts. EXIF orientation is applied, then all metadata (including GPS) is dropped
by re-encoding — a privacy win.
