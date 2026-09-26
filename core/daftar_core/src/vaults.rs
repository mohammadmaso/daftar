//! The user's vaults: add, rename, archive, restore and remove (§3.2). The list lives in
//! `.daftar/config.json`; these calls also keep `vaults/<id>/index.md` in step, so both are
//! committed together as a settings change on the next sync. Every prompt reads the list from the
//! config, so the assistant sees a change on its next op.

use std::fs;

use crate::config::{Bilingual, VaultConfig};
use crate::library::Library;
use crate::{Error, Result, layout, pages};

/// Every vault, archived ones included, in the user's order.
pub fn list(lib: &Library) -> Result<Vec<VaultConfig>> {
    Ok(lib.config()?.vaults)
}

/// Adds a vault and its empty index; returns the new vault's id.
pub fn add(lib: &Library, title: Bilingual, purpose: &str) -> Result<String> {
    let id = lib.update_config(|c| c.add_vault(title, purpose))??;
    refresh_index(lib, &id)?;
    Ok(id)
}

pub fn edit(lib: &Library, id: &str, title: Bilingual, purpose: &str) -> Result<()> {
    lib.update_config(|c| c.edit_vault(id, title, purpose))??;
    // The index heading carries the vault's title.
    refresh_index(lib, id)
}

pub fn set_archived(lib: &Library, id: &str, archived: bool) -> Result<()> {
    lib.update_config(|c| c.set_vault_archived(id, archived))??;
    if !archived {
        refresh_index(lib, id)?;
    }
    Ok(())
}

/// Removes an empty vault. A vault that holds pages is archived instead, never deleted: the repo
/// is the user's knowledge.
pub fn remove(lib: &Library, id: &str) -> Result<()> {
    let mut c = lib.config()?;
    c.remove_vault(id)?;
    let n = page_count(lib, id)?;
    if n > 0 {
        return Err(Error::invalid(format!(
            "This vault still has {n} {}; archive it instead.",
            if n == 1 { "page" } else { "pages" }
        )));
    }
    lib.save_config(&c)?;
    let dir = lib.path(&format!("{}/{id}", layout::VAULTS_DIR));
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

/// Files in a vault's folder other than its generated index.
pub fn page_count(lib: &Library, id: &str) -> Result<usize> {
    fn walk(dir: &std::path::Path, top: bool) -> Result<usize> {
        let mut n = 0;
        let Ok(entries) = fs::read_dir(dir) else {
            return Ok(0);
        };
        for e in entries {
            let e = e?;
            let name = e.file_name();
            if e.file_type()?.is_dir() {
                n += walk(&e.path(), false)?;
            } else if !(top && name == "index.md") && name != ".gitkeep" {
                n += 1;
            }
        }
        Ok(n)
    }
    walk(&lib.path(&format!("{}/{id}", layout::VAULTS_DIR)), true)
}

fn refresh_index(lib: &Library, id: &str) -> Result<()> {
    pages::regenerate_indexes(lib, &[id.to_owned()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    fn t(en: &str, fa: &str) -> Bilingual {
        Bilingual {
            en: en.into(),
            fa: fa.into(),
        }
    }

    #[test]
    fn add_edit_archive_remove() {
        let (_tmp, lib) = testutil::lib_in();
        let id = add(&lib, t("Travel", "سفر"), "Trips, visas, packing lists.").unwrap();
        assert_eq!(id, "travel");
        let idx = fs::read_to_string(lib.path("vaults/travel/index.md")).unwrap();
        assert!(idx.contains("Travel"));
        let prompt = lib.config().unwrap().vaults_for_prompt();
        assert!(prompt.contains("- `travel` (Travel · سفر): Trips, visas, packing lists."));

        // Same name again gets its own folder.
        assert_eq!(
            add(&lib, t("Travel", ""), "More trips.").unwrap(),
            "travel-2"
        );
        // A Persian-only name still gets an ASCII id and both titles.
        let fa_only = add(&lib, t("", "کتاب‌ها"), "Books I read.").unwrap();
        assert_eq!(fa_only, "vault");
        assert_eq!(
            lib.config().unwrap().vault("vault").unwrap().title.en,
            "کتاب‌ها"
        );

        edit(&lib, "travel", t("Journeys", "سفرها"), "Trips.").unwrap();
        let idx = fs::read_to_string(lib.path("vaults/travel/index.md")).unwrap();
        assert!(idx.contains("Journeys"));

        set_archived(&lib, "travel", true).unwrap();
        let c = lib.config().unwrap();
        assert!(c.active_vault("travel").is_none());
        assert!(!c.vaults_for_prompt().contains("`travel`"));
        set_archived(&lib, "travel", false).unwrap();

        // A vault with pages is not removed.
        fs::create_dir_all(lib.path("vaults/travel/trips")).unwrap();
        fs::write(lib.path("vaults/travel/trips/rome.md"), "x").unwrap();
        let err = remove(&lib, "travel").unwrap_err().to_string();
        assert!(err.contains("1 page; archive it instead"), "{err}");
        assert!(lib.config().unwrap().vault("travel").is_some());

        remove(&lib, "travel-2").unwrap();
        assert!(lib.config().unwrap().vault("travel-2").is_none());
        assert!(!lib.path("vaults/travel-2").exists());
    }

    #[test]
    fn builtin_vaults_stay() {
        let (_tmp, lib) = testutil::lib_in();
        assert!(remove(&lib, "life").is_err());
        assert!(set_archived(&lib, "stories", true).is_err());
        edit(
            &lib,
            "life",
            t("Daily", "روزانه"),
            "Journal and everything else.",
        )
        .unwrap();
        // Default vaults other than Life and Stories are the user's to remove.
        remove(&lib, "work").unwrap();
        assert!(lib.config().unwrap().vault("work").is_none());
        assert!(add(&lib, t(" ", ""), "x").is_err());
        assert!(add(&lib, t("X", ""), "  ").is_err());
    }
}
