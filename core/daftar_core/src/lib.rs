//! Core logic for Daftar: storage, sync, indexing, the agent loop, providers and MCP.
//! The Flutter app and `daftar_cli` are thin shells over this crate.

pub mod agent;
pub mod ask;
pub mod assets;
pub mod audit;
pub mod changeset;
pub mod config;
pub mod edit;
mod error;
pub mod frontmatter;
pub mod fsutil;
pub mod ids;
pub mod index_md;
pub mod keys;
pub mod lang;
pub mod layout;
pub mod ledger;
pub mod library;
pub mod mcp;
pub mod normalize;
pub mod ops;
pub mod pages;
pub mod prompts;
pub mod providers;
pub mod queue;
pub mod raw;
pub mod review;
pub mod review_ops;
pub mod runtime;
pub mod search;
pub mod session;
pub mod sync;
pub mod testutil;
pub mod time;
pub mod tls;
pub mod tools;
pub mod validate;
pub mod voice;
pub mod wellbeing;
pub mod wiki;

pub use error::{Error, Result};

/// Product codename. Everything user-visible derives from this so the product can be renamed.
pub const APP_NAME: &str = "Daftar";

/// Lowercase identifier used for directories (`.daftar/`), URL schemes and user agents.
pub const APP_ID: &str = "daftar";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bumped whenever the on-disk repository layout changes in a way that needs migration.
pub const REPO_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CoreInfo {
    pub app_name: String,
    pub version: String,
    pub repo_schema_version: u32,
    pub target: String,
}

pub fn core_info() -> CoreInfo {
    CoreInfo {
        app_name: APP_NAME.to_owned(),
        version: VERSION.to_owned(),
        repo_schema_version: REPO_SCHEMA_VERSION,
        target: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
    }
}
