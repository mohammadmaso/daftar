use std::io;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("git: {0}")]
    Git(#[from] git2::Error),
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("image: {0}")]
    Image(#[from] image::ImageError),
    #[error("time: {0}")]
    Time(#[from] jiff::Error),
    #[error("not a {app} library: {0}", app = crate::APP_NAME)]
    NotALibrary(String),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("network unavailable")]
    Offline,
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("sync needs attention: {0}")]
    NeedsAttention(String),
    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid(msg.into())
    }
}
