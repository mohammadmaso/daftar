//! A library is one knowledge repository (§3) plus this device's local, never-synced state.
//! Local state lives in `.git/daftar/` so it travels with the clone, is never committed, and is
//! removed together with the checkout.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::fsutil::atomic_write;
use crate::layout;
use crate::{Error, Result};

pub const SCHEMA_TEMPLATE: &str = include_str!("../templates/SCHEMA.md");
pub const GITATTRIBUTES_TEMPLATE: &str = include_str!("../templates/gitattributes");
pub const GITIGNORE_TEMPLATE: &str = include_str!("../templates/gitignore");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub last_seen: String,
}

/// Device-local identity, stored in `.git/daftar/device.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalDevice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
}

impl Library {
    /// Opens an existing checkout. Fails if the directory is not a Daftar library.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.join(layout::CONFIG_FILE).is_file() || !root.join(".git").is_dir() {
            return Err(Error::NotALibrary(root.display().to_string()));
        }
        let lib = Self { root };
        fs::create_dir_all(lib.local_dir())?;
        Ok(lib)
    }

    /// Wraps a directory that is a git checkout but may not contain the structure yet.
    pub(crate) fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        let mut p = self.root.clone();
        p.extend(rel.split('/'));
        p
    }

    pub fn local_dir(&self) -> PathBuf {
        self.root.join(".git").join(crate::APP_ID)
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.local_dir().join("audio")
    }

    pub fn db_path(&self) -> PathBuf {
        self.local_dir().join("state.sqlite")
    }

    pub fn config(&self) -> Result<Config> {
        let bytes = fs::read(self.path(layout::CONFIG_FILE))?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        atomic_write(
            &self.path(layout::CONFIG_FILE),
            config.to_pretty_json().as_bytes(),
        )?;
        Ok(())
    }

    /// Read-modify-write of the shared config. The change is committed on the next sync.
    pub fn update_config<T>(&self, f: impl FnOnce(&mut Config) -> T) -> Result<T> {
        let mut c = self.config()?;
        let out = f(&mut c);
        self.save_config(&c)?;
        Ok(out)
    }

    pub fn device(&self) -> Result<LocalDevice> {
        let p = self.local_dir().join("device.json");
        let bytes = fs::read(&p).map_err(|_| {
            Error::invalid("this device has not been named yet; call set_device first")
        })?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Registers this device locally and in `.daftar/devices/<id>.json` (committed on next sync).
    pub fn set_device(&self, name: &str, platform: &str, now: &jiff::Zoned) -> Result<LocalDevice> {
        let dev = LocalDevice {
            id: crate::ids::device_id(name),
            name: name.trim().to_owned(),
        };
        fs::create_dir_all(self.local_dir())?;
        atomic_write(
            &self.local_dir().join("device.json"),
            serde_json::to_vec_pretty(&dev)?.as_slice(),
        )?;
        self.touch_device(&dev, platform, now)?;
        Ok(dev)
    }

    pub fn touch_device(&self, dev: &LocalDevice, platform: &str, now: &jiff::Zoned) -> Result<()> {
        let info = DeviceInfo {
            id: dev.id.clone(),
            name: dev.name.clone(),
            platform: platform.to_owned(),
            last_seen: crate::time::rfc3339(now),
        };
        let mut json = serde_json::to_string_pretty(&info)?;
        json.push('\n');
        atomic_write(&self.path(&layout::device_file(&dev.id)), json.as_bytes())?;
        Ok(())
    }

    /// Writes the initial structure into an empty checkout. Existing files are left untouched, so
    /// running it on a partially initialised repository is safe.
    pub fn write_structure(&self) -> Result<()> {
        let config = Config::default();
        self.write_if_missing(layout::SCHEMA_FILE, SCHEMA_TEMPLATE)?;
        self.write_if_missing(".gitattributes", GITATTRIBUTES_TEMPLATE)?;
        self.write_if_missing(".gitignore", GITIGNORE_TEMPLATE)?;
        self.write_if_missing(layout::CONFIG_FILE, &config.to_pretty_json())?;
        for v in config.active_vaults() {
            self.write_if_missing(
                &layout::vault_index(&v.id),
                &crate::index_md::empty_index(&v.title.en, &v.title.fa),
            )?;
        }
        for dir in [
            layout::RAW_DIR,
            layout::LOG_DIR,
            ".daftar/ledger",
            ".daftar/review",
        ] {
            self.write_if_missing(&format!("{dir}/.gitkeep"), "")?;
        }
        fs::create_dir_all(self.local_dir())?;
        Ok(())
    }

    fn write_if_missing(&self, rel: &str, content: &str) -> Result<()> {
        let p = self.path(rel);
        if !p.exists() {
            atomic_write(&p, content.as_bytes())?;
        }
        Ok(())
    }
}
