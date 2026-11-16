//! A test's directory of fixture files, deleted when the test that made it ends.
//!
//! [`TempDir`] owns the guard of a directory a caller already made, such as an
//! `nvs_repo::scratch` directory, so this crate depends on nothing: where the
//! directory is and how it is deleted are the guard's, and this type adds the
//! writes and joins every caller would otherwise repeat.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// A directory of fixture files, deleted when its guard drops.
pub struct TempDir {
    guard: Box<dyn AsRef<Path> + Send + Sync>,
}

impl TempDir {
    /// A directory over `guard`, which is the directory's path and deletes it
    /// when it drops.
    #[must_use]
    pub fn new(guard: impl AsRef<Path> + Send + Sync + 'static) -> Self {
        Self {
            guard: Box::new(guard),
        }
    }

    /// The directory itself.
    #[must_use]
    pub fn root(&self) -> &Path {
        (*self.guard).as_ref()
    }

    /// The path of `name` inside the directory.
    #[must_use]
    pub fn at(&self, name: &str) -> PathBuf {
        self.root().join(name)
    }

    /// Writes `text` to `name`, creating the directories above it first.
    ///
    /// # Panics
    ///
    /// If a directory or the file cannot be written.
    pub fn write(&self, name: &str, text: &str) {
        let path = self.at(name);
        fs::create_dir_all(path.parent().expect("a fixture sits in a directory"))
            .expect("a scratch directory");
        fs::write(path, text).expect("a writable scratch file");
    }
}

impl fmt::Debug for TempDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TempDir")
            .field("root", &self.root())
            .finish()
    }
}
