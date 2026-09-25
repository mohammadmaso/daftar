//! Human edits made in the app's editor (§8.3): saving a page is one `edit:` commit with no
//! `Op-Id`, so blame marks the lines as human-authored and the agent treats them with respect.

use git2::Repository;

use crate::library::{Library, LocalDevice};
use crate::{Error, Result, fsutil, pages, wiki};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditOutcome {
    /// Hash of the saved text, the base for the next save.
    pub hash: String,
    /// False when the text was unchanged and nothing was committed.
    pub committed: bool,
}

/// Saves `text` to the page at `path` if it still has `base_hash` (what the editor loaded).
/// A page that changed meanwhile (sync, filing, another app) is refused rather than overwritten.
pub fn save_page(
    lib: &Library,
    dev: &LocalDevice,
    path: &str,
    base_hash: &str,
    text: &str,
) -> Result<EditOutcome> {
    if !path.starts_with("vaults/") || !path.ends_with(".md") || path.contains("..") {
        return Err(Error::invalid("Only wiki pages can be edited here."));
    }
    if pages::is_generated_index(path) {
        return Err(Error::invalid(
            "The vault index is generated; edit the pages instead.",
        ));
    }
    let current = std::fs::read_to_string(lib.path(path)).unwrap_or_default();
    if wiki::content_hash(&current) != base_hash {
        return Err(Error::invalid(
            "This page changed while you were editing. Your text is kept; reopen the page to merge it.",
        ));
    }
    let text = if text.ends_with('\n') {
        text.to_owned()
    } else {
        format!("{text}\n")
    };
    if text == current {
        return Ok(EditOutcome {
            hash: base_hash.to_owned(),
            committed: false,
        });
    }
    if let Some(h) = crate::secrets::scan(&text).first() {
        return Err(Error::invalid(format!(
            "This page contains what looks like a {}. Keys and tokens never go into the repository; redact it first.",
            h.kind
        )));
    }
    if text.starts_with("---") && wiki::parse(path, &text).is_err() {
        return Err(Error::invalid(
            "The properties at the top of the page can't be read; check the lines between ---.",
        ));
    }
    fsutil::atomic_write(&lib.path(path), text.as_bytes())?;
    let mut paths = vec![path.to_owned()];
    if let Some(v) = pages::vault_of(path) {
        paths.extend(pages::regenerate_indexes(lib, &[v.to_owned()])?);
    }
    let repo = Repository::open(lib.root())?;
    let msg = format!("edit: {path}\n\nEdit-Source: app\nDevice: {}\n", dev.id);
    crate::sync::commit_paths(&repo, &paths, &msg, &crate::sync::signature(dev)?)?;
    Ok(EditOutcome {
        hash: wiki::content_hash(&text),
        committed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{device, lib_in};

    #[test]
    fn saves_as_a_human_commit_and_refuses_stale_bases() {
        let (_d, lib) = lib_in();
        let dev = device("laptop");
        let path = "vaults/life/people/sara.md";
        let doc = "---\ntype: person\ntitle: { en: \"Sara\", fa: \"سارا\" }\nsummary: \"Cousin\"\n---\n\nCousin.\n";
        fsutil::atomic_write(&lib.path(path), doc.as_bytes()).unwrap();
        let base = wiki::content_hash(doc);

        let edited = doc.replace("Cousin.\n", "Cousin. Lives in Shiraz.");
        let out = save_page(&lib, &dev, path, &base, &edited).unwrap();
        assert!(out.committed);
        let repo = Repository::open(lib.root()).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert!(
            head.message()
                .unwrap()
                .starts_with("edit: vaults/life/people/sara.md")
        );
        assert!(crate::ledger::trailer(head.message().unwrap(), "Op-Id").is_none());
        assert!(
            crate::tools::human_lines_at_head(&lib, path).contains(&7),
            "the edited line is human-authored"
        );
        assert!(
            std::fs::read_to_string(lib.path("vaults/life/index.md"))
                .unwrap()
                .contains("sara")
        );

        assert!(save_page(&lib, &dev, path, &base, "stale").is_err());
        assert!(save_page(&lib, &dev, path, &out.hash, "---\nbroken: [\n---\n").is_err());
        assert!(save_page(&lib, &dev, "vaults/life/index.md", &out.hash, "x").is_err());
        assert!(
            !save_page(
                &lib,
                &dev,
                path,
                &out.hash,
                &std::fs::read_to_string(lib.path(path)).unwrap()
            )
            .unwrap()
            .committed
        );
    }
}
