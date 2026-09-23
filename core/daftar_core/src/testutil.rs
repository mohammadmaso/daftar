//! Helpers shared by unit tests, integration tests and the CLI's scenario runner.
#![doc(hidden)]

use crate::library::{Library, LocalDevice};

pub fn zoned(s: &str) -> jiff::Zoned {
    s.parse().expect("valid zoned datetime")
}

pub fn device(id: &str) -> LocalDevice {
    LocalDevice {
        id: id.to_owned(),
        name: id.to_owned(),
    }
}

/// A fresh initialised library in a temp dir (no remote).
pub fn lib_in() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().expect("tempdir");
    let lib = crate::sync::init_local(dir.path(), "main").expect("init");
    (dir, lib)
}
