//! Capture assets (§3.1): images are normalised (EXIF orientation applied, metadata stripped,
//! ≤ 1600 px long edge, JPEG q80) and committed under `raw/assets/`; audio stays device-local.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};
use jiff::Zoned;
use ulid::Ulid;

use crate::fsutil::atomic_write;
use crate::library::Library;
use crate::{Result, layout, time};

pub const MAX_EDGE: u32 = 1600;
pub const JPEG_QUALITY: u8 = 80;
pub const AUDIO_RETENTION: Duration = Duration::from_secs(14 * 24 * 3600);

/// Normalises `bytes` (any common image format) and stores it. Returns the repo-relative path.
pub fn import_image(lib: &Library, bytes: &[u8], now: &Zoned) -> Result<String> {
    let jpeg = normalise_image(bytes)?;
    let rel = layout::raw_asset(time::date_of(now), Ulid::generate(), "jpg");
    atomic_write(&lib.path(&rel), &jpeg)?;
    Ok(rel)
}

pub fn normalise_image(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    if img.width().max(img.height()) > MAX_EDGE {
        img = img.resize(MAX_EDGE, MAX_EDGE, FilterType::Lanczos3);
    }
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY).encode_image(&rgb)?;
    Ok(out)
}

/// Keeps a voice recording on this device only (not committed) so it can be re-transcribed.
pub fn store_audio(lib: &Library, raw_id: Ulid, src: &Path) -> Result<PathBuf> {
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("m4a");
    store_audio_as(lib, raw_id, src, ext)
}

/// Like [`store_audio`], under an explicit extension (the transcriber reads the format from it).
pub fn store_audio_as(lib: &Library, raw_id: Ulid, src: &Path, ext: &str) -> Result<PathBuf> {
    let dest = lib.audio_dir().join(format!("{raw_id}.{ext}"));
    fs::create_dir_all(lib.audio_dir())?;
    if src != dest {
        fs::copy(src, &dest)?;
    }
    Ok(dest)
}

pub fn audio_for(lib: &Library, raw_id: Ulid) -> Option<PathBuf> {
    let prefix = format!("{raw_id}.");
    fs::read_dir(lib.audio_dir())
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .map(|e| e.path())
}

/// Deletes local recordings older than the retention window. Returns how many were removed.
pub fn prune_audio(lib: &Library, now: SystemTime) -> Result<usize> {
    let mut removed = 0;
    let Ok(entries) = fs::read_dir(lib.audio_dir()) else {
        return Ok(0);
    };
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .map(|m| now.duration_since(m).unwrap_or_default() > AUDIO_RETENTION)
            .unwrap_or(false);
        if old && fs::remove_file(e.path()).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbImage};

    #[test]
    fn large_images_are_downscaled_to_jpeg() {
        let img =
            DynamicImage::ImageRgb8(RgbImage::from_pixel(3200, 1000, image::Rgb([200, 10, 10])));
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        let out = normalise_image(&png).unwrap();
        let back = image::load_from_memory_with_format(&out, ImageFormat::Jpeg).unwrap();
        assert_eq!((back.width(), back.height()), (1600, 500));
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(normalise_image(b"not an image").is_err());
    }
}
