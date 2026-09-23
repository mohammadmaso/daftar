use ulid::Ulid;

pub fn new_id() -> Ulid {
    Ulid::generate()
}

/// ASCII kebab-case slug: lowercase `[a-z0-9-]`, no leading/trailing/double dashes.
/// Non-ASCII input collapses to dashes; callers must supply a fallback for empty results.
pub fn kebab_slug(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut dash = false;
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Device ids end up in file names on every platform, so they are slugs capped at 32 chars.
pub fn device_id(name: &str) -> String {
    let mut slug = kebab_slug(name);
    slug.truncate(32);
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        format!(
            "device-{}",
            &new_id().to_string()[20..].to_ascii_lowercase()
        )
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(kebab_slug("Vitamin D  deficiency!"), "vitamin-d-deficiency");
        assert_eq!(kebab_slug("  --Pixel 8 Pro-- "), "pixel-8-pro");
        assert_eq!(kebab_slug("کمبود ویتامین"), "");
    }

    #[test]
    fn device_ids_never_empty() {
        assert_eq!(device_id("Mohammad's Pixel 8"), "mohammad-s-pixel-8");
        assert!(device_id("گوشی من").starts_with("device-"));
        assert!(device_id(&"x".repeat(80)).len() <= 32);
    }
}
