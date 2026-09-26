#![allow(dead_code)]

pub mod archivist;

use std::path::{Path, PathBuf};

use daftar_core::ledger::{self, LedgerEntry, OpType};
use daftar_core::library::{Library, LocalDevice};
use daftar_core::queue::Queue;
use daftar_core::raw::{self, NewCapture, RawItem, RawKind, RawStatus};
use daftar_core::sync::{self, GitAuth, SyncOutcome};
use daftar_core::testutil::zoned;
use git2::Repository;

pub struct Remote {
    _dir: tempfile::TempDir,
    pub path: PathBuf,
    hidden: PathBuf,
}

impl Remote {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("remote.git");
        let hidden = dir.path().join("remote.git.offline");
        let mut opts = git2::RepositoryInitOptions::new();
        opts.bare(true).initial_head("main");
        Repository::init_opts(&path, &opts).unwrap();
        Self {
            _dir: dir,
            path,
            hidden,
        }
    }
    pub fn url(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
    /// Simulates losing the network.
    pub fn go_offline(&self) {
        std::fs::rename(&self.path, &self.hidden).unwrap();
    }
    pub fn go_online(&self) {
        std::fs::rename(&self.hidden, &self.path).unwrap();
    }
    pub fn commit_subjects(&self) -> Vec<String> {
        let repo = Repository::open_bare(&self.path).unwrap();
        let mut walk = repo.revwalk().unwrap();
        walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)
            .unwrap();
        walk.push_ref("refs/heads/main").unwrap();
        walk.map(|o| {
            repo.find_commit(o.unwrap())
                .unwrap()
                .summary()
                .ok()
                .flatten()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
    }
}

pub struct Device {
    _dir: tempfile::TempDir,
    pub lib: Library,
    pub queue: Queue,
    pub dev: LocalDevice,
}

impl Device {
    pub fn clone_from(remote: &Remote, name: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("lib");
        let lib = sync::clone(&remote.url(), &root, &GitAuth::None, "main").unwrap();
        let now = zoned("2026-09-23T09:00:00+03:30[Asia/Tehran]");
        let dev = lib.set_device(name, "test", &now).unwrap();
        let queue = Queue::open(&lib.db_path()).unwrap();
        Self {
            _dir: dir,
            lib,
            queue,
            dev,
        }
    }

    pub fn root(&self) -> &Path {
        self.lib.root()
    }

    pub fn capture(&self, text: &str, at: &str) -> RawItem {
        raw::create(
            &self.lib,
            &self.dev,
            &zoned(at),
            RawKind::Text,
            NewCapture {
                text: text.into(),
                ..Default::default()
            },
        )
        .unwrap()
    }

    pub fn sync(&self) -> SyncOutcome {
        sync::sync(
            &self.lib,
            &self.queue,
            &self.dev,
            &GitAuth::None,
            &zoned("2026-09-23T20:00:00+03:30[Asia/Tehran]"),
        )
        .unwrap()
    }

    pub fn write(&self, rel: &str, content: &str) {
        let p = self.lib.path(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.lib.path(rel)).unwrap_or_default()
    }

    /// A deterministic stand-in for an AI ingest op: writes `page`, marks the source ingested,
    /// writes a ledger entry and commits it as one op commit — exactly the shape M2 produces.
    pub fn fake_ingest(&self, source: &RawItem, page: &str, line: &str, op_id: &str) -> String {
        let mut text = self.read(page);
        if text.is_empty() {
            text = "# Page\n\n".into();
        }
        text.push_str(line);
        text.push('\n');
        self.write(page, &text);
        raw::set_status(&self.lib, &source.path, RawStatus::Ingested).unwrap();
        let entry = LedgerEntry {
            op_id: op_id.into(),
            op_type: OpType::Ingest,
            sources: vec![source.meta.id.clone()],
            router: None,
            models: vec!["mock".into()],
            pages_created: vec![],
            pages_updated: vec![page.into()],
            claims_added: vec![],
            review_items: vec![],
            usage: Default::default(),
            started_at: "2026-09-23T10:00:00+03:30".into(),
            finished_at: "2026-09-23T10:00:01+03:30".into(),
            device: self.dev.id.clone(),
            summary: String::new(),
            note: None,
            forced_vault: None,
            replayed_from: None,
            reverts: None,
            rejected_claims: vec![],
        };
        let ledger_rel = ledger::write(&self.lib, &entry).unwrap();
        let repo = Repository::open(self.root()).unwrap();
        let mut idx = repo.index().unwrap();
        for p in [page, source.path.as_str(), ledger_rel.as_str()] {
            idx.add_path(Path::new(p)).unwrap();
        }
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = git2::Signature::now(&self.dev.name, "t@daftar.local").unwrap();
        let msg =
            ledger::commit_message("ingest: test", &entry, Some(&source.path), &["life".into()]);
        repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&head])
            .unwrap();
        ledger_rel
    }

    pub fn raw_files(&self) -> Vec<String> {
        let mut out = vec![];
        let mut stack = vec![self.lib.path("raw")];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                if e.path().is_dir() {
                    stack.push(e.path());
                } else if e.path().extension().is_some_and(|x| x == "md") {
                    out.push(e.file_name().to_string_lossy().into_owned());
                }
            }
        }
        out.sort();
        out
    }
}

pub fn no_conflict_markers(root: &Path) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(t) = std::fs::read_to_string(&p) {
                for m in ["<<<<<<<", ">>>>>>>", "\n=======\n"] {
                    assert!(!t.contains(m), "conflict marker in {}", p.display());
                }
            }
        }
    }
}
