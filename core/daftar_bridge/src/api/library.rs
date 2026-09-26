//! Library lifecycle, capture and sync for the Flutter app. Thin wrappers over `daftar_core`;
//! all types here are plain DTOs so the core stays free of FFI concerns.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, Weak};

use daftar_core::session::{CaptureStage, Session};
use daftar_core::sync::{self, GitAuth};
use flutter_rust_bridge::frb;

pub use daftar_core::raw::RawKind;

#[frb(mirror(RawKind))]
pub enum _RawKind {
    Voice,
    Text,
    Photo,
    VoiceConversation,
    ChatAnswer,
    Import,
}

pub enum AuthKind {
    None,
    Token,
    SshKey,
}

/// Credentials for one call. Loaded from secure storage by the app; never persisted by the core.
pub struct Auth {
    pub kind: AuthKind,
    pub username: Option<String>,
    /// Token for `Token`, OpenSSH private key text for `SshKey`.
    pub secret: String,
    pub passphrase: Option<String>,
}

impl From<Auth> for GitAuth {
    fn from(a: Auth) -> Self {
        match a.kind {
            AuthKind::None => GitAuth::None,
            AuthKind::Token => GitAuth::Token {
                username: a.username,
                token: a.secret,
            },
            AuthKind::SshKey => GitAuth::SshKey {
                private_key: a.secret,
                passphrase: a.passphrase,
            },
        }
    }
}

pub enum Stage {
    Saved,
    Working,
    Failed,
    Filed,
    Excluded,
}

/// What filing did for a capture: "Filed to Life · Health — 4 pages updated, 1 claim to review".
pub struct Filing {
    pub op_id: String,
    pub vaults: Vec<String>,
    pub pages_created: u32,
    pub pages_updated: u32,
    pub claims_to_review: u32,
    pub to_review: u32,
}

pub struct Capture {
    pub id: String,
    pub kind: RawKind,
    /// RFC 3339 with offset.
    pub captured_at: String,
    pub device: String,
    pub text: String,
    pub vault_hint: Option<String>,
    /// The imported file's name, for imports and picked recordings.
    pub file_name: Option<String>,
    pub images: Vec<String>,
    pub stage: Stage,
    pub problem: Option<String>,
    pub filing: Option<Filing>,
}

pub enum SyncState {
    Synced,
    LocalChanges,
    Offline,
    NeedsAttention,
}

pub struct SyncResult {
    pub state: SyncState,
    pub message: Option<String>,
    pub pulled: u32,
    pub pushed: u32,
    pub conflicts: Vec<String>,
    pub replays: u32,
    pub changed_paths: Vec<String>,
}

pub struct RepoStatus {
    pub uncommitted: u32,
    pub unpushed: u32,
    pub has_remote: bool,
    pub branch: String,
    pub remote_url: Option<String>,
    pub device_id: String,
    pub device_name: String,
    pub root: String,
}

pub struct Vault {
    pub id: String,
    pub title_en: String,
    pub title_fa: String,
    pub fiction: bool,
}

pub struct SshKeyPair {
    pub private_openssh: String,
    pub public_openssh: String,
}

fn err(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

/// Input problems as the plain sentence the core wrote (so the app can translate it).
fn plain(e: daftar_core::Error) -> anyhow::Error {
    match e {
        daftar_core::Error::Invalid(m) => anyhow::anyhow!(m),
        other => err(other),
    }
}

fn now() -> jiff::Zoned {
    jiff::Zoned::now()
}

/// Whether `root` already holds a library with a named device.
#[frb(sync)]
pub fn library_ready(root: String) -> bool {
    daftar_core::library::Library::open(&root).is_ok_and(|l| l.device().is_ok())
}

/// New library on this device only; a remote can be added later.
pub fn init_library(root: String, device_name: String, platform: String) -> anyhow::Result<()> {
    let lib = sync::init_local(&PathBuf::from(&root), "main").map_err(err)?;
    lib.set_device(&device_name, &platform, &now())
        .map_err(err)?;
    Ok(())
}

/// Clones (or initialises an empty) remote into `root`.
pub fn clone_library(
    url: String,
    root: String,
    auth: Auth,
    branch: String,
    device_name: String,
    platform: String,
) -> anyhow::Result<()> {
    let root = PathBuf::from(root);
    if root.exists()
        && std::fs::read_dir(&root)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return Err(anyhow::anyhow!("The target folder is not empty."));
    }
    let res = sync::clone(&url, &root, &auth.into(), &branch);
    let lib = match res {
        Ok(lib) => lib,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&root);
            return Err(err(human_error(&e)));
        }
    };
    lib.set_device(&device_name, &platform, &now())
        .map_err(err)?;
    Ok(())
}

fn human_error(e: &daftar_core::Error) -> String {
    match e {
        daftar_core::Error::Offline => {
            "Couldn't reach the repository. Check the address and your connection.".into()
        }
        daftar_core::Error::Auth(_) => {
            "The repository refused the credentials. Check the token or that the SSH key is added."
                .into()
        }
        other => other.to_string(),
    }
}

pub fn generate_ssh_key(comment: String) -> anyhow::Result<SshKeyPair> {
    let k = daftar_core::keys::generate_ed25519(&comment).map_err(err)?;
    Ok(SshKeyPair {
        private_openssh: k.private_openssh,
        public_openssh: k.public_openssh,
    })
}

/// Sessions open in this process, by canonical library root.
static OPEN: LazyLock<Mutex<HashMap<PathBuf, Weak<Session>>>> = LazyLock::new(Default::default);

#[frb(opaque)]
pub struct LibraryHandle {
    session: Arc<Session>,
}

impl LibraryHandle {
    #[frb(ignore)]
    pub(crate) fn session(&self) -> &Session {
        &self.session
    }

    #[frb(ignore)]
    pub(crate) fn session_arc(&self) -> Arc<Session> {
        self.session.clone()
    }

    /// Opens the library, or joins the session already open on it in this process: the app and a
    /// background task (Android runs both in one process) must share one queue and commit lock.
    pub fn open(root: String) -> anyhow::Result<LibraryHandle> {
        let key = std::fs::canonicalize(&root).unwrap_or_else(|_| PathBuf::from(&root));
        let mut open = OPEN.lock().unwrap_or_else(|p| p.into_inner());
        open.retain(|_, s| s.strong_count() > 0);
        if let Some(session) = open.get(&key).and_then(Weak::upgrade) {
            return Ok(LibraryHandle { session });
        }
        let session = Arc::new(Session::open(root).map_err(err)?);
        open.insert(key, Arc::downgrade(&session));
        Ok(LibraryHandle { session })
    }

    pub fn capture_text(&self, text: String, vault_hint: Option<String>) -> anyhow::Result<String> {
        Ok(self
            .session
            .capture_text(&text, vault_hint, &now())
            .map_err(err)?
            .meta
            .id)
    }

    pub fn capture_photo(
        &self,
        bytes: Vec<u8>,
        note: Option<String>,
        vault_hint: Option<String>,
    ) -> anyhow::Result<String> {
        Ok(self
            .session
            .capture_photo(&bytes, note, vault_hint, &now())
            .map_err(err)?
            .meta
            .id)
    }

    pub fn capture_voice(
        &self,
        audio_path: String,
        vault_hint: Option<String>,
    ) -> anyhow::Result<String> {
        Ok(self
            .session
            .capture_voice(&PathBuf::from(audio_path), vault_hint, &now())
            .map_err(err)?
            .meta
            .id)
    }

    /// Files the (previewed, possibly edited) text of an imported document.
    pub fn capture_import(
        &self,
        text: String,
        file_name: String,
        vault_hint: Option<String>,
    ) -> anyhow::Result<String> {
        Ok(self
            .session
            .capture_import(&text, &file_name, vault_hint, &now())
            .map_err(plain)?
            .meta
            .id)
    }

    /// Files a picked audio file: transcribed on this device, then filed like a voice note.
    pub fn capture_audio_file(
        &self,
        path: String,
        vault_hint: Option<String>,
    ) -> anyhow::Result<String> {
        Ok(self
            .session
            .capture_audio_file(&PathBuf::from(path), vault_hint, &now())
            .map_err(plain)?
            .meta
            .id)
    }

    pub fn discard(&self, id: String) -> anyhow::Result<bool> {
        self.session.discard_unsynced(&id).map_err(err)
    }

    /// Captures of the given local date, oldest first.
    pub fn day(&self, year: i32, month: u8, day: u8) -> anyhow::Result<Vec<Capture>> {
        let items = self
            .session
            .day(daftar_core::layout::Date { year, month, day })
            .map_err(err)?;
        Ok(items
            .into_iter()
            .map(|c| Capture {
                id: c.id,
                kind: c.kind,
                captured_at: c.captured_at,
                device: c.device,
                text: c.text,
                vault_hint: c.vault_hint,
                file_name: c.file_name,
                images: c.images,
                stage: match c.stage {
                    CaptureStage::Saved => Stage::Saved,
                    CaptureStage::Working => Stage::Working,
                    CaptureStage::Failed => Stage::Failed,
                    CaptureStage::Filed => Stage::Filed,
                    CaptureStage::Excluded => Stage::Excluded,
                },
                problem: c.problem,
                filing: c.filing.map(|f| Filing {
                    op_id: f.op_id,
                    vaults: f.vaults,
                    pages_created: f.pages_created as u32,
                    pages_updated: f.pages_updated as u32,
                    claims_to_review: f.claims_to_review as u32,
                    to_review: f.to_review as u32,
                }),
            })
            .collect())
    }

    pub fn status(&self) -> anyhow::Result<RepoStatus> {
        let s = self.session.status().map_err(err)?;
        let lib = self.session.library();
        Ok(RepoStatus {
            uncommitted: s.uncommitted as u32,
            unpushed: s.unpushed as u32,
            has_remote: s.has_remote,
            branch: s.branch,
            remote_url: sync::remote_url(lib).map_err(err)?,
            device_id: self.session.device().id.clone(),
            device_name: self.session.device().name.clone(),
            root: lib.root().to_string_lossy().into_owned(),
        })
    }

    pub fn set_remote(&self, url: String) -> anyhow::Result<()> {
        sync::set_remote(self.session.library(), &url).map_err(err)
    }

    pub fn vaults(&self) -> anyhow::Result<Vec<Vault>> {
        let c = self.session.library().config().map_err(err)?;
        Ok(c.active_vaults()
            .map(|v| Vault {
                id: v.id.clone(),
                title_en: v.title.en.clone(),
                title_fa: v.title.fa.clone(),
                fiction: v.fiction,
            })
            .collect())
    }

    pub fn sync(&self, auth: Auth) -> anyhow::Result<SyncResult> {
        let o = self
            .session
            .sync(&auth.into(), &now())
            .map_err(|e| err(human_error(&e)))?;
        Ok(SyncResult {
            state: match o.state {
                daftar_core::sync::SyncState::Synced => SyncState::Synced,
                daftar_core::sync::SyncState::LocalChanges => SyncState::LocalChanges,
                daftar_core::sync::SyncState::Offline => SyncState::Offline,
                daftar_core::sync::SyncState::NeedsAttention => SyncState::NeedsAttention,
            },
            message: o.message,
            pulled: o.pulled as u32,
            pushed: o.pushed as u32,
            conflicts: o.conflicts,
            replays: o.replays.len() as u32,
            changed_paths: o.changed_paths,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_session_per_library_in_a_process() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("lib").to_string_lossy().into_owned();
        init_library(root.clone(), "test".into(), "linux".into()).unwrap();
        let a = LibraryHandle::open(root.clone()).unwrap();
        let b = LibraryHandle::open(root.clone()).unwrap();
        assert!(Arc::ptr_eq(&a.session, &b.session), "shared while open");
        drop((a, b));
        let c = LibraryHandle::open(root).unwrap();
        assert_eq!(
            Arc::strong_count(&c.session),
            1,
            "reopened fresh once closed"
        );
    }
}
