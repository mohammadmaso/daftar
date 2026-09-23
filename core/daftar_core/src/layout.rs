//! Path conventions of the knowledge repository (brief §3). Pure functions; all paths are
//! repo-relative with `/` separators so they are identical on every platform and in Git.

use ulid::Ulid;

pub const SCHEMA_FILE: &str = "SCHEMA.md";
pub const APP_DIR: &str = ".daftar";
pub const CONFIG_FILE: &str = ".daftar/config.json";
pub const RAW_DIR: &str = "raw";
pub const VAULTS_DIR: &str = "vaults";
pub const LOG_DIR: &str = "log";

pub const DEFAULT_VAULTS: [&str; 5] = ["life", "health", "mind", "work", "stories"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

pub fn raw_capture(date: Date, compact_ts: &str, device: &str, id: Ulid) -> String {
    format!(
        "{RAW_DIR}/{:04}/{:02}/{:02}/{compact_ts}-{device}-{id}.md",
        date.year, date.month, date.day
    )
}

pub fn raw_asset(date: Date, id: Ulid, ext: &str) -> String {
    format!(
        "{RAW_DIR}/assets/{:04}/{:02}/{id}.{ext}",
        date.year, date.month
    )
}

pub fn ledger_entry(date: Date, op_id: Ulid) -> String {
    format!(
        "{APP_DIR}/ledger/{:04}/{:02}/{op_id}.json",
        date.year, date.month
    )
}

pub fn device_file(device: &str) -> String {
    format!("{APP_DIR}/devices/{device}.json")
}

pub fn monthly_log(date: Date) -> String {
    format!("{LOG_DIR}/{:04}-{:02}.md", date.year, date.month)
}

pub fn vault_index(vault: &str) -> String {
    format!("{VAULTS_DIR}/{vault}/index.md")
}

/// Paths the AI changeset pipeline may never write (brief §6.4).
pub fn is_protected_from_ai(path: &str) -> bool {
    path == SCHEMA_FILE
        || path.starts_with("raw/")
        || path.starts_with(".daftar/")
        || path == ".gitattributes"
        || (path.starts_with("vaults/")
            && path.ends_with("/index.md")
            && path.matches('/').count() == 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: Date = Date {
        year: 2026,
        month: 9,
        day: 3,
    };

    #[test]
    fn paths() {
        let id: Ulid = "01JABCDEFGHJKMNPQRSTVWXYZ0".parse().unwrap();
        assert_eq!(
            raw_capture(D, "20260903T141502", "pixel-8", id),
            "raw/2026/09/03/20260903T141502-pixel-8-01JABCDEFGHJKMNPQRSTVWXYZ0.md"
        );
        assert_eq!(
            raw_asset(D, id, "webp"),
            "raw/assets/2026/09/01JABCDEFGHJKMNPQRSTVWXYZ0.webp"
        );
        assert_eq!(
            ledger_entry(D, id),
            ".daftar/ledger/2026/09/01JABCDEFGHJKMNPQRSTVWXYZ0.json"
        );
        assert_eq!(monthly_log(D), "log/2026-09.md");
        assert_eq!(vault_index("health"), "vaults/health/index.md");
    }

    #[test]
    fn protection() {
        assert!(is_protected_from_ai("SCHEMA.md"));
        assert!(is_protected_from_ai("raw/2026/09/03/x.md"));
        assert!(is_protected_from_ai(".daftar/config.json"));
        assert!(is_protected_from_ai("vaults/life/index.md"));
        assert!(!is_protected_from_ai("vaults/life/topics/index.md"));
        assert!(!is_protected_from_ai("vaults/life/people/sara.md"));
    }
}
