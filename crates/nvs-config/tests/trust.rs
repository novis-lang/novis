//! ADR 0103 § 6 against a real filesystem, which is the only place it can be asked at all.
//!
//! `tests/resolve.rs` runs against an in-memory reader on purpose and so cannot see this half: that
//! the check reads the platform's own idea of who owns a path and who may write it, and that what
//! it hands back is canonical. Each case works under a directory of its own beneath the system
//! temporary directory — the one place all three target platforms give the invoking account and
//! nobody else, which is what makes a *passing* case meaningful rather than accidental.
//!
//! Only the refusals a case can construct portably are asserted here. Making a path group-writable
//! is one `chmod` on Unix and an ACL edit on Windows, so the two negative cases below are
//! `cfg(unix)`; the Windows half of the boundary — the owner SID and the effective-rights sweep —
//! is `crates/nvs-config/src/trust.rs`'s module doc, and what it accepts is ADR 0103 § 6's own text.

use std::fs;
use std::path::PathBuf;

use nvs_config::trust::{Untrusted, check};

/// A directory of this case's own, empty, under the system temporary directory.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nvs-trust-{}-{name}", std::process::id()));
    drop(fs::remove_dir_all(&dir));
    fs::create_dir_all(&dir).expect("a scratch directory of this case's own");
    dir
}

/// § 6 accepts what an ordinary installation looks like — a file this account owns in a directory
/// this account owns — and hands back the canonical path, which is the thing the resolver's cycle
/// test and ADR 0104 § 2's `[[app]]` matching are both built on.
#[test]
fn a_file_this_account_owns_is_trusted_and_comes_back_canonical() {
    let dir = scratch("owned");
    let file = dir.join("nvs.toml");
    fs::write(&file, "[limits]\nmemory = \"128M\"\n").expect("a configuration file");
    fs::create_dir_all(dir.join("conf.d")).expect("a subdirectory to reach it through");

    let trusted = check(&file).expect("a file this account owns is inside the boundary");
    assert!(
        trusted.is_absolute(),
        "the path comes back absolute: {trusted:?}"
    );
    assert_eq!(
        check(&dir.join("conf.d").join("..").join("nvs.toml"))
            .expect("the same file, spelled the long way round"),
        trusted,
        "two spellings of one file resolve to one path — which is what closes a cycle a symlink \
         would otherwise hide",
    );

    drop(fs::remove_dir_all(&dir));
}

/// A path that is not there is `Unreadable` and not a `Breach`. The resolver reports the two
/// differently on purpose: absence is what `optional` is allowed to cover, and a breach never is.
#[test]
fn a_path_that_is_not_there_is_unreadable_rather_than_a_breach() {
    let dir = scratch("absent");

    let why = check(&dir.join("nvs.toml")).expect_err("there is no such file");
    assert!(
        matches!(why, Untrusted::Unreadable(_)),
        "an absent file is not a breach of the boundary: {why:?}",
    );

    drop(fs::remove_dir_all(&dir));
}

/// § 6's mode half, on the file itself.
#[cfg(unix)]
#[test]
fn a_group_writable_file_is_a_breach() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("group-writable-file");
    let file = dir.join("nvs.toml");
    fs::write(&file, "").expect("a configuration file");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o664)).expect("a group-writable mode");

    let why = check(&file).expect_err("anyone in the group could rewrite this file");
    assert!(
        matches!(why, Untrusted::Breach(_)),
        "the file was read and refused, not merely unreadable: {why:?}",
    );
    assert!(
        why.message().contains("group-writable"),
        "the refusal says what is wrong with it: {}",
        why.message(),
    );

    drop(fs::remove_dir_all(&dir));
}

/// § 6's mode half, on the directory — the case a check that stopped at the file would pass. A file
/// nobody else can write is still replaceable by anyone who can write the directory it sits in.
#[cfg(unix)]
#[test]
fn a_file_in_a_world_writable_directory_is_a_breach_too() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("world-writable-directory");
    let file = dir.join("nvs.toml");
    fs::write(&file, "").expect("a configuration file");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("an ordinary mode");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o777)).expect("a world-writable mode");

    let named = fs::canonicalize(&dir).expect("the directory, as the check will name it");
    let why = check(&file).expect_err("anyone at all could replace this file");
    assert!(
        why.message().contains("world-writable")
            && why.message().contains(&named.display().to_string()),
        "the refusal names the directory rather than the file it holds: {}",
        why.message(),
    );

    drop(fs::remove_dir_all(&dir));
}
