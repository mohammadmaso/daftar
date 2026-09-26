//! Settings › Vaults: the user's own list of vaults (§3.2).

use daftar_core::config::{Bilingual, is_builtin_vault};
use daftar_core::vaults;

use super::library::LibraryHandle;

pub struct VaultSettings {
    pub id: String,
    pub title_en: String,
    pub title_fa: String,
    /// What belongs in the vault; the assistant files by it.
    pub purpose: String,
    pub archived: bool,
    pub fiction: bool,
    /// Life and Stories: can be renamed, never archived or removed.
    pub builtin: bool,
    /// Pages in the vault; only an empty vault can be removed.
    pub pages: u32,
}

fn err(e: daftar_core::Error) -> anyhow::Error {
    match e {
        // The sentence is meant for the user; drop the "invalid input:" prefix.
        daftar_core::Error::Invalid(m) => anyhow::anyhow!(m),
        other => anyhow::anyhow!(other.to_string()),
    }
}

impl LibraryHandle {
    /// Every vault, archived ones included, in the user's order.
    pub fn vault_settings(&self) -> anyhow::Result<Vec<VaultSettings>> {
        let lib = self.session().library();
        vaults::list(lib)
            .map_err(err)?
            .into_iter()
            .map(|v| {
                Ok(VaultSettings {
                    pages: vaults::page_count(lib, &v.id).map_err(err)? as u32,
                    builtin: is_builtin_vault(&v.id),
                    id: v.id,
                    title_en: v.title.en,
                    title_fa: v.title.fa,
                    purpose: v.purpose,
                    archived: v.archived,
                    fiction: v.fiction,
                })
            })
            .collect()
    }

    /// Returns the new vault's id.
    pub fn add_vault(
        &self,
        title_en: String,
        title_fa: String,
        purpose: String,
    ) -> anyhow::Result<String> {
        vaults::add(
            self.session().library(),
            Bilingual {
                en: title_en,
                fa: title_fa,
            },
            &purpose,
        )
        .map_err(err)
    }

    pub fn edit_vault(
        &self,
        id: String,
        title_en: String,
        title_fa: String,
        purpose: String,
    ) -> anyhow::Result<()> {
        vaults::edit(
            self.session().library(),
            &id,
            Bilingual {
                en: title_en,
                fa: title_fa,
            },
            &purpose,
        )
        .map_err(err)
    }

    pub fn set_vault_archived(&self, id: String, archived: bool) -> anyhow::Result<()> {
        vaults::set_archived(self.session().library(), &id, archived).map_err(err)
    }

    /// Removes an empty vault; a vault with pages must be archived instead.
    pub fn remove_vault(&self, id: String) -> anyhow::Result<()> {
        vaults::remove(self.session().library(), &id).map_err(err)
    }
}
