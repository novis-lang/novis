//! `rule:config/ownership-is-the-trust-boundary` against a real filesystem, which is the only place it can be asked at all.
//!
//! `tests/resolve.rs` runs against an in-memory reader on purpose and so cannot see this half: that
//! the check reads the platform's own idea of who owns a path and who may write it, and that what
//! it hands back is canonical. Each case works under a directory of its own beneath the system
//! temporary directory — the one place every target platform gives the invoking account and
//! nobody else, which is what makes a *passing* case meaningful rather than accidental.
//!
//! Making a path group-writable is one `chmod` on Unix and an ACL edit on Windows, so each
//! platform's refusals are written against its own tool: the `cfg(unix)` cases below use
//! `set_permissions`, and the `cfg(windows)` ones drive `icacls` by SID, never by account name,
//! because every one of those names is localized. What the boundary accepts is `rule:config/ownership-is-the-trust-boundary`'s own
//! text, and how the Windows half computes an effective right is
//! `crates/nvs-config/src/trust.rs`'s module doc.

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
/// test and `rule:config/every-matching-app-block-applies-least-specific-first`'s `[[app]]` matching are both built on.
// covers: tools:config/trust
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

/// `nvs_repo::scratch_private` is what a test that runs this check writes into, so it must pass the
/// check on every machine: a file in it, the directory itself and a subdirectory are all inside
/// the boundary, whatever `target/` inherits.
#[test]
fn a_private_scratch_dir_is_inside_the_boundary() {
    let dir = nvs_repo::scratch_private("trust-private");
    let file = dir.join("nvs.toml");
    fs::write(&file, "").expect("a configuration file");
    fs::create_dir(dir.join("conf.d")).expect("a subdirectory");

    check(&file).expect("a file in a private scratch directory");
    check(&dir).expect("the private scratch directory and the one around it");
    check(&dir.join("conf.d")).expect("a subdirectory, which inherits the lock");
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

/// The installation chapter's Linux layout, asked of the check mode by mode. Every mode its
/// `install` commands set passes, on the directory and on `nvs.toml` alike; each mode it names as
/// failing is a breach that quotes the mode back; and `chmod go-w`, the repair it prescribes, is
/// the whole repair.
#[cfg(unix)]
// covers: tools:install/linux-and-macos
#[test]
fn the_installed_modes_pass_and_a_write_bit_for_the_group_or_for_others_does_not() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("installed-layout");
    let file = dir.join("nvs.toml");
    fs::write(&file, "").expect("a configuration file");
    let chmod = |path: &std::path::Path, mode: u32| {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .expect("this account owns the path");
    };
    let named = fs::canonicalize(&dir).expect("the directory, as the check will name it");

    for (folder, config) in [(0o755, 0o640), (0o750, 0o640), (0o755, 0o755)] {
        chmod(&dir, folder);
        chmod(&file, config);
        let passed = check(&file);
        assert!(
            passed.is_ok(),
            "a {folder:04o} directory holding a {config:04o} file is what the chapter installs: \
             {passed:?}",
        );
    }

    for mode in [0o775, 0o777, 0o664] {
        chmod(&dir, mode | 0o700);
        chmod(&file, 0o640);
        let why = check(&file).expect_err("the directory can be written by more than its owner");
        assert!(
            matches!(why, Untrusted::Breach(_))
                && why
                    .message()
                    .contains(&format!("mode {:04o}", mode | 0o700))
                && why.message().contains(&named.display().to_string()),
            "a directory with {mode:04o}'s write bits is refused by name and by mode: {}",
            why.message(),
        );

        chmod(&dir, 0o755);
        chmod(&file, mode);
        let why = check(&file).expect_err("the file can be written by more than its owner");
        assert!(
            matches!(why, Untrusted::Breach(_))
                && why.message().contains(&format!("mode {mode:04o}"))
                && why.message().contains("nvs.toml"),
            "a {mode:04o} file is refused by name and by mode: {}",
            why.message(),
        );

        chmod(&file, mode & !0o022);
        let repaired = check(&file);
        assert!(
            repaired.is_ok(),
            "`chmod go-w` on a {mode:04o} file is the whole repair: {repaired:?}",
        );
    }

    drop(fs::remove_dir_all(&dir));
}

/// One `icacls` edit, by SID because an account name is localized and a rename does not move it.
///
/// `/inheritance:d` first, on every edit, because a scratch directory under the temporary
/// directory inherits its entries: without that, a `/remove` names an entry that is not there to
/// remove and the grant survives.
#[cfg(windows)]
fn icacls(dir: &std::path::Path, args: &[&str]) {
    // `icacls` edits the scratch directory it is given and opens nothing in the repository.
    let done = nvs_repo::spawn("icacls", &[])
        .arg(dir)
        .args(args)
        .output()
        .expect("`icacls` ships with Windows");
    assert!(
        done.status.success(),
        "icacls {args:?} on {dir:?}: {}",
        String::from_utf8_lossy(&done.stderr),
    );
}

/// `BUILTIN\Users`, which is the group every interactive account on the box is in.
#[cfg(windows)]
const USERS: &str = "*S-1-5-32-545";

/// § 6's DACL half: an entry granting an untrusted principal a write right is a breach, and the
/// refusal names the principal rather than a mode.
#[cfg(windows)]
#[test]
fn a_directory_a_well_known_group_may_write_is_a_breach() {
    let dir = scratch("group-writable-dacl");
    icacls(&dir, &["/inheritance:d"]);
    check(&dir).expect("the scratch directory is inside the boundary before the grant");

    icacls(&dir, &["/grant:r", &format!("{USERS}:(WD)")]);
    let why = check(&dir).expect_err("anyone in BUILTIN\\Users could create a file here");
    assert!(
        matches!(why, Untrusted::Breach(_)) && why.message().contains("BUILTIN\\Users"),
        "the refusal was read off the DACL and names the principal: {why:?}",
    );

    icacls(&dir, &["/remove:g", USERS]);
    check(&dir).expect("the grant is gone and the directory is inside the boundary again");

    drop(fs::remove_dir_all(&dir));
}

/// The right asked about is the *effective* one, which is the whole reason the walk in
/// `trust.rs` accumulates deny entries ahead of grants rather than stopping at the first grant it
/// matches. A grant a deny cancels leaves the principal unable to write, and § 6 has nothing to
/// refuse.
#[cfg(windows)]
#[test]
fn a_grant_a_deny_entry_cancels_is_not_a_breach() {
    let dir = scratch("denied-grant-dacl");
    icacls(&dir, &["/inheritance:d"]);
    icacls(&dir, &["/grant:r", &format!("{USERS}:(WD)")]);
    check(&dir).expect_err("the grant alone is a breach — which is what the deny below undoes");

    icacls(&dir, &["/deny", &format!("{USERS}:(WD)")]);
    check(&dir).expect("a right denied is a right this principal does not effectively have");

    icacls(&dir, &["/remove", USERS]);
    drop(fs::remove_dir_all(&dir));
}

/// § 6's owner half, and the install chapter's row for it: a path another account owns is a breach
/// whatever its DACL grants, and the refusal names the path and the owner's SID.
///
/// The system directory is the one path every Windows has whose owner is none of the three the
/// boundary accepts, because it belongs to `NT SERVICE\TrustedInstaller`, and the check only reads
/// it.
// covers: tools:install/when-something-is-refused
#[cfg(windows)]
#[test]
fn a_directory_another_account_owns_is_a_breach_that_names_the_owner() {
    let root = std::env::var_os("SystemRoot").expect("Windows sets `SystemRoot`");
    let system = nvs_config::trust::canonical(&PathBuf::from(root).join("System32"))
        .expect("the system directory exists");

    let why = check(&system).expect_err("neither this account nor an administrator owns it");
    assert!(
        matches!(why, Untrusted::Breach(_))
            && why
                .message()
                .starts_with(&format!("`{}` is owned by `S-1-5-80-", system.display()))
            && why
                .message()
                .ends_with("`, which is neither this account nor an administrative one"),
        "the refusal names the path and the owner that is outside the boundary: {why:?}",
    );
}
