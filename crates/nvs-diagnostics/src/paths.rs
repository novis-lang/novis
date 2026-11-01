//! Lexical path normalization: the one copy every crate that compares or joins
//! paths without touching the filesystem calls.
//!
//! It lives here because this is the lowest crate that `nvs-config` and
//! `nvs-types` both depend on, and the embedded file table in this crate needs it
//! too.

use std::path::{Component, Path, PathBuf};

/// `path` with every `.` dropped and every `..` resolved lexically, with no
/// filesystem access.
///
/// A `..` takes away the folder before it. A `..` with no folder before it in a
/// relative path is kept, because dropping it would turn `../etc/nvs.toml` into
/// `etc/nvs.toml` and read a different file than the one written. A `..` right
/// after a root is dropped, because a root has no parent. A path made of `.`
/// alone gives the empty path.
#[must_use]
pub fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir) => {}
                // A drive-relative `C:` is not a root, so its `..` is kept, as
                // it is for any relative path.
                None | Some(Component::ParentDir | Component::CurDir | Component::Prefix(_)) => {
                    out.push(Component::ParentDir);
                }
            },
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(path: &str) -> PathBuf {
        lexical_normalize(Path::new(path))
    }

    #[test]
    fn lexical_normalize_drops_a_current_dir_and_pops_a_parent() {
        assert_eq!(
            norm("/app/src/./lib/../lib/db.nvs"),
            PathBuf::from("/app/src/lib/db.nvs")
        );
        assert_eq!(norm("src/./a/b/../c.nvs"), PathBuf::from("src/a/c.nvs"));
        assert_eq!(norm("a/.."), PathBuf::new());
        assert_eq!(norm("."), PathBuf::new());
    }

    #[test]
    fn lexical_normalize_keeps_a_parent_it_cannot_pop_from_a_relative_path() {
        assert_eq!(norm("../etc/nvs.toml"), PathBuf::from("../etc/nvs.toml"));
        assert_eq!(norm("a/../../b"), PathBuf::from("../b"));
        // A kept `..` is never popped by the next one.
        assert_eq!(norm("../../x"), PathBuf::from("../../x"));
        assert_eq!(norm("./../x"), PathBuf::from("../x"));
    }

    #[test]
    fn lexical_normalize_drops_a_parent_at_a_root() {
        assert_eq!(norm("/../etc"), PathBuf::from("/etc"));
        assert_eq!(norm("/a/../../b"), PathBuf::from("/b"));
        #[cfg(windows)]
        assert_eq!(norm(r"C:\..\Windows"), PathBuf::from(r"C:\Windows"));
    }
}
