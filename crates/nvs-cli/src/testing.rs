//! The fixtures more than one module's `mod tests` needs, in the platform's own spelling.
//!
//! A helper lives here when two modules' unit tests manufacture the same state and neither owns
//! it: `crate::cache`'s tests and `crate::config`'s tests both have to produce the directory
//! `rule:config/ownership-is-the-trust-boundary` refuses, and a copy in each is two spellings of
//! one platform question that drift apart the first time a platform's answer changes. Everything
//! else stays in the `mod tests` that uses it — this is what is shared, not a drawer for every
//! fixture.

use std::path::Path;

/// Refuse `dir` the creation of a new file, without touching who owns it — the other half of what
/// a write into a working directory can fail on. Ownership is left alone deliberately: a directory
/// that also failed `rule:config/ownership-is-the-trust-boundary` would be answered by the check in
/// front of the write, and the case would never reach the filesystem it is about.
///
/// Unix drops the owner's write bit. Windows has no bit that means this — the read-only attribute
/// does not stop a file being created in a directory — so it denies `Everyone` the two rights that
/// create one, and a deny entry reaches this account along with every other.
pub(crate) fn refuse_new_files(dir: &Path) {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(dir, fs::Permissions::from_mode(0o555)).expect("a read-only mode");
    }
    #[cfg(windows)]
    {
        let denied = nvs_repo::spawn("icacls", &[])
            .arg(dir)
            .arg("/deny")
            .arg("*S-1-1-0:(OI)(CI)(WD,AD)")
            .output()
            .expect("`icacls` ships with every supported Windows");
        assert!(
            denied.status.success(),
            "`Everyone` could not be denied file creation on {}: {}",
            dir.display(),
            String::from_utf8_lossy(&denied.stderr),
        );
    }
}

/// Make `dir` writable by every local account — the state
/// `rule:config/ownership-is-the-trust-boundary` refuses — in the platform's own spelling, because
/// there is no portable one. Unix is a mode; Windows is an ACE for `Everyone` (`S-1-1-0`), added
/// through `icacls` rather than through `windows-sys` so that a test helper does not cost this
/// crate a dependency and an `unsafe` block. `icacls` is the same command `nvs_config::trust::REMEDY`
/// tells an operator to undo such a grant with.
pub(crate) fn open_to_the_world(dir: &Path) {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(dir, fs::Permissions::from_mode(0o777)).expect("a world-writable mode");
    }
    #[cfg(windows)]
    {
        let granted = nvs_repo::spawn("icacls", &[])
            .arg(dir)
            .arg("/grant")
            .arg("*S-1-1-0:(OI)(CI)(M)")
            .output()
            .expect("`icacls` ships with every supported Windows");
        assert!(
            granted.status.success(),
            "`Everyone` could not be granted write on {}: {}",
            dir.display(),
            String::from_utf8_lossy(&granted.stderr),
        );
    }
}
