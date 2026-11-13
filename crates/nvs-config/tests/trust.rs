//! `rule:config/ownership-is-the-trust-boundary` against a real filesystem, which is the only place it can be asked at all.
//!
//! `tests/resolve.rs` runs against an in-memory reader on purpose and so cannot see this half: that
//! the check reads the platform's own idea of who owns a path and who may write it, and that what
//! it hands back is canonical. Each case works under a directory of its own from
//! `nvs_repo::scratch_private`, which only the invoking account can reach, and that is what makes a
//! *passing* case meaningful rather than accidental.
//!
//! Making a path group-writable is one `chmod` on Unix and an ACL edit on Windows, so each
//! platform's refusals are written against its own tool: the `cfg(unix)` cases below use
//! `set_permissions`, and the `cfg(windows)` ones drive `icacls` by SID, never by account name,
//! because every one of those names is localized. What the boundary accepts is `rule:config/ownership-is-the-trust-boundary`'s own
//! text, and how the Windows half computes an effective right is
//! `crates/nvs-config/src/trust.rs`'s module doc.

use std::fs;

use nvs_config::trust::{Untrusted, check};

/// A directory of this case's own, empty, that only this account can reach. Dropping the guard
/// deletes it.
fn scratch(name: &str) -> nvs_repo::Scratch {
    nvs_repo::scratch_private(&format!("trust-{name}"))
}

/// `rule:config/ownership-is-the-trust-boundary` accepts what an ordinary installation looks like — a file this account owns in a directory
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
}

/// `rule:config/ownership-is-the-trust-boundary`'s mode half, on the file itself.
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
        matches!(why, Untrusted::Breach { .. }),
        "the file was read and refused, not merely unreadable: {why:?}",
    );
    assert!(
        why.message().contains("group-writable"),
        "the refusal says what is wrong with it: {}",
        why.message(),
    );
}

/// `rule:config/ownership-is-the-trust-boundary`'s mode half, on the directory — the case a check that stopped at the file would pass. A file
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
}

/// A link to the folder `target` at `at`. On Windows a junction, which needs no right, is made
/// where a symbolic link cannot be.
fn link_dir(target: &std::path::Path, at: &std::path::Path) {
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(target, at).is_ok();
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(target, at).is_ok()
        || nvs_repo::spawn("cmd", &[])
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(at)
            .arg(target)
            .output()
            .is_ok_and(|out| out.status.success());
    assert!(linked, "`{}` could not be linked", at.display());
}

/// A link deploy, `current` pointing at a release folder: the directory that holds the link is
/// checked as well as the folder the file is in, because whoever can write it picks the file the
/// next reload reads. A private directory holding the link passes.
#[test]
fn a_link_in_a_private_directory_passes() {
    let dir = scratch("link-private");
    let release = dir.join("releases").join("1");
    fs::create_dir_all(&release).expect("a release folder");
    fs::write(release.join("nvs.toml"), "").expect("a configuration file");
    link_dir(&release, &dir.join("current"));

    let trusted = check(&dir.join("current").join("nvs.toml")).expect("every directory is private");
    assert!(
        trusted.ends_with(std::path::Path::new("releases").join("1").join("nvs.toml")),
        "the canonical path is the release's file: {trusted:?}",
    );
}

/// The same deploy, with the directory that holds `current` group-writable. The release folder
/// and the file are private, so only the link's directory can be the breach, and the refusal names
/// it and the link.
#[cfg(unix)]
#[test]
fn a_link_in_a_group_writable_directory_is_a_breach() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("link-group-writable");
    let release = dir.join("releases").join("1");
    let shared = dir.join("shared");
    fs::create_dir_all(&release).expect("a release folder");
    fs::create_dir(&shared).expect("the directory the link sits in");
    fs::write(release.join("nvs.toml"), "").expect("a configuration file");
    link_dir(&release, &shared.join("current"));
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o775)).expect("a group-writable mode");

    let why = check(&shared.join("current").join("nvs.toml"))
        .expect_err("anyone in the group could point `current` at another file");
    assert!(
        matches!(&why, Untrusted::Breach { .. })
            && why.message().contains("group-writable")
            && why.message().contains("holds the link")
            && why.message().contains(&shared.display().to_string()),
        "the refusal names the link's directory: {why:?}",
    );
    check(&release.join("nvs.toml")).expect("the file reached with no link is trusted");
}

/// The Windows half of the case above: `BUILTIN\Users` may write the directory that holds the
/// junction, so the junction is a breach.
#[cfg(windows)]
#[test]
fn a_link_in_a_directory_a_well_known_group_may_write_is_a_breach() {
    let dir = scratch("link-dacl");
    let release = dir.join("releases").join("1");
    let shared = dir.join("shared");
    fs::create_dir_all(&release).expect("a release folder");
    fs::create_dir(&shared).expect("the directory the link sits in");
    fs::write(release.join("nvs.toml"), "").expect("a configuration file");
    link_dir(&release, &shared.join("current"));
    icacls(&shared, &["/inheritance:d"]);
    icacls(&shared, &["/grant:r", &format!("{USERS}:(WD)")]);

    let why = check(&shared.join("current").join("nvs.toml"))
        .expect_err("anyone in BUILTIN\\Users could replace the junction");
    assert!(
        matches!(&why, Untrusted::Breach { .. })
            && why.message().contains(&format!("`{USERS}`"))
            && why.message().contains("holds the link"),
        "the refusal names the link's directory: {why:?}",
    );
    check(&release.join("nvs.toml")).expect("the file reached with no link is trusted");
    icacls(&shared, &["/remove:g", USERS]);
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
            matches!(why, Untrusted::Breach { .. })
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
            matches!(why, Untrusted::Breach { .. })
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
}

/// One `icacls` edit, by SID because an account name is localized and a rename does not move it.
///
/// `/inheritance:d` first, on every edit, because a scratch directory inherits the entries of the
/// directory around it: without that, a `/remove` names an entry that is not there to remove and
/// the grant survives.
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

/// A file `nvs.toml` in a scratch folder of its own. On Windows the folder stops inheriting
/// first, so a grant on it is the only entry for that trustee.
fn lone_file(name: &str) -> (nvs_repo::Scratch, std::path::PathBuf) {
    let dir = scratch(name);
    let file = dir.join("nvs.toml");
    fs::write(&file, "").expect("a configuration file");
    #[cfg(windows)]
    {
        icacls(&dir, &["/inheritance:d"]);
        icacls(&file, &["/inheritance:d"]);
    }
    (dir, file)
}

/// `rule:config/ownership-is-the-trust-boundary`: no other account may write the file or its
/// folder. On Windows that is any trustee outside the trusted list, so the four logon groups
/// no fixed list named are each refused by SID, on the file and on its folder. On Unix it is
/// the group bit and the world bit.
// covers: tools:config/trust
#[test]
fn a_write_entry_for_any_other_account_is_a_breach() {
    let (dir, file) = lone_file("any-other-account");
    check(&file).expect("the file is inside the boundary before any grant");
    #[cfg(windows)]
    for sid in ["*S-1-5-4", "*S-1-5-2", "*S-1-5-3", "*S-1-5-6"] {
        let folder = nvs_config::trust::canonical(dir.path()).expect("the folder exists");
        for (path, named) in [
            (file.as_path(), "nvs.toml`".to_string()),
            (dir.path(), format!("`{}` ", folder.display())),
        ] {
            icacls(path, &["/grant:r", &format!("{sid}:M")]);
            let why = check(&file).expect_err("another account may write");
            assert!(
                matches!(why, Untrusted::Breach { .. })
                    && why.message().contains(&format!("`{sid}`"))
                    && why.message().contains(&named),
                "the refusal names {named} and {sid}: {why:?}",
            );
            icacls(path, &["/remove:g", sid]);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (path, normal) in [(file.as_path(), 0o600), (dir.path(), 0o700)] {
            for bit in [0o020, 0o002] {
                fs::set_permissions(path, fs::Permissions::from_mode(normal | bit))
                    .expect("this account owns the path");
                let why = check(&file).expect_err("another account may write");
                assert!(
                    matches!(why, Untrusted::Breach { .. }),
                    "a {:04o} path is refused: {why:?}",
                    normal | bit,
                );
                fs::set_permissions(path, fs::Permissions::from_mode(normal))
                    .expect("this account owns the path");
            }
        }
    }
    check(&file).expect("every grant is gone and the file is inside the boundary again");
}

/// This account's own SID as `icacls` takes it, read from `whoami`.
#[cfg(windows)]
fn my_sid() -> String {
    // `whoami` prints this account and opens nothing in the repository.
    let done = nvs_repo::spawn("whoami", &[])
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .expect("`whoami` ships with Windows");
    let line = String::from_utf8_lossy(&done.stdout);
    let sid = line
        .trim()
        .rsplit(',')
        .next()
        .expect("a CSV row")
        .trim_matches('"');
    assert!(sid.starts_with("S-"), "`whoami /user` printed {line}");
    format!("*{sid}")
}

/// The trusted accounts may hold full control: on Windows the owner, `SYSTEM`,
/// `Administrators` and `TrustedInstaller`, on the file and on its folder. On Unix an
/// owner-writable file in an owner-writable folder.
// covers: tools:config/trust
#[test]
fn a_write_entry_for_a_trusted_account_passes() {
    let (dir, file) = lone_file("trusted-account");
    #[cfg(windows)]
    {
        let me = my_sid();
        let installer = "*S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464";
        for path in [file.as_path(), dir.path()] {
            for sid in [me.as_str(), "*S-1-5-18", "*S-1-5-32-544", installer] {
                icacls(path, &["/grant:r", &format!("{sid}:F")]);
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("an owner mode");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("an owner mode");
    }
    check(&file).unwrap_or_else(|why| panic!("{}", why.message()));
}

/// `rule:config/ownership-is-the-trust-boundary`'s DACL half: an entry granting an untrusted principal a write right is a breach, and the
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
        matches!(why, Untrusted::Breach { .. }) && why.message().contains(&format!("`{USERS}`")),
        "the refusal was read off the DACL and names the principal: {why:?}",
    );

    icacls(&dir, &["/remove:g", USERS]);
    check(&dir).expect("the grant is gone and the directory is inside the boundary again");
}

/// The right asked about is the *effective* one, which is the whole reason the walk in
/// `trust.rs` accumulates deny entries ahead of grants rather than stopping at the first grant it
/// matches. A grant a deny cancels leaves the principal unable to write, and `rule:config/ownership-is-the-trust-boundary` has nothing to
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
}

/// `rule:config/ownership-is-the-trust-boundary`'s owner half, and the install chapter's row for it: a path another account owns is a breach
/// whatever its DACL grants, the refusal names the path and the owner's SID, and its help is the
/// `icacls /setowner` that gives the path to `Administrators`, with the path filled in.
///
/// The system directory is the one path every Windows has whose owner is none of the three the
/// boundary accepts, because it belongs to `NT SERVICE\TrustedInstaller`, and the check only reads
/// it.
// covers: tools:install/when-something-is-refused
#[cfg(windows)]
#[test]
fn a_directory_another_account_owns_is_a_breach_that_names_the_owner() {
    let root = std::env::var_os("SystemRoot").expect("Windows sets `SystemRoot`");
    let system = nvs_config::trust::canonical(&std::path::PathBuf::from(root).join("System32"))
        .expect("the system directory exists");

    let why = check(&system).expect_err("neither this account nor an administrator owns it");
    assert!(
        matches!(why, Untrusted::Breach { .. })
            && why
                .message()
                .starts_with(&format!("`{}` is owned by `S-1-5-80-", system.display()))
            && why
                .message()
                .ends_with("`, which is neither this account nor an administrator"),
        "the refusal names the path and the owner that is outside the boundary: {why:?}",
    );
    let help = why.help().expect("a breach carries its repair");
    assert!(
        help.contains(&format!(
            "`icacls \"{}\" /setowner *S-1-5-32-544`",
            system.display()
        )) && help.contains("Run as administrator"),
        "the repair is the command for this path, in an elevated prompt: {help}",
    );
}

/// The service account the two owner cases below judge for. It is never an administrator, so
/// a folder it trusts is trusted for its owner alone.
#[cfg(windows)]
fn local_service() -> nvs_config::trust::Account {
    nvs_config::trust::Account::named("LocalService").expect("a well-known account")
}

/// An owner that is a member of `Administrators` passes the owner test for every account. An
/// administrator can take ownership of any file, so trusting one as owner gives nobody a new
/// right. The scratch folder is owned by the test's own account, so the case needs that account
/// to be an administrator, which a CI runner's is.
// covers: tools:config/trust
#[cfg(windows)]
#[test]
#[expect(
    clippy::print_stdout,
    reason = "an account that is not an administrator says why the case did not run"
)]
fn an_owner_in_administrators_passes_for_any_account() {
    let me = nvs_config::trust::Account::current().expect("this process's token");
    if !me.administrator() {
        println!("skipped: this account is not a member of `Administrators`");
        return;
    }
    let dir = scratch("administrator-owner");
    let file = dir.join("nvs.toml");
    fs::write(&file, "").expect("a configuration file");
    for path in [dir.path(), file.as_path()] {
        nvs_config::trust::check_alone_as(path, &local_service())
            .unwrap_or_else(|why| panic!("{}: {}", path.display(), why.message()));
    }
}

/// An owner outside `Administrators` passes only for its own account. Another account, here
/// `LocalService`, gets a breach that names the owner, because that owner could change the file
/// without being an administrator.
// covers: tools:config/trust
#[cfg(windows)]
#[test]
#[expect(
    clippy::print_stdout,
    reason = "an account that is an administrator says why the case did not run"
)]
fn an_owner_outside_administrators_is_refused_for_another_account() {
    let me = nvs_config::trust::Account::current().expect("this process's token");
    if me.administrator() {
        println!("skipped: this account is a member of `Administrators`, so it owns as one");
        return;
    }
    let dir = scratch("ordinary-owner");
    nvs_config::trust::check_alone_as(dir.path(), &me).expect("the owner's own folder");
    let why = nvs_config::trust::check_alone_as(dir.path(), &local_service())
        .expect_err("`LocalService` does not trust an ordinary account's folder");
    assert!(
        matches!(why, Untrusted::Breach { .. })
            && why
                .message()
                .ends_with("which is neither the account `LocalService` nor an administrator"),
        "the refusal names the owner and the account it judged for: {why:?}",
    );
}

/// A folder `outer/inner` under a scratch folder, with `outer` the folder whose rights the two
/// parent cases below change. On Windows `outer` stops inheriting first, so a grant on it is the
/// only entry for the group, and a grant without `(OI)(CI)` stays on `outer` alone.
fn nested(name: &str) -> (nvs_repo::Scratch, std::path::PathBuf, std::path::PathBuf) {
    let dir = scratch(name);
    let outer = dir.join("outer");
    let inner = outer.join("inner");
    fs::create_dir_all(&inner).expect("two nested folders");
    #[cfg(windows)]
    icacls(&outer, &["/inheritance:d"]);
    (dir, outer, inner)
}

/// The folder above a checked folder fails only when another account can replace that folder.
/// On Windows every account may modify `outer` here, which is the right a non-system drive's root
/// gives `Authenticated Users`. On Unix `outer` is world-writable with the sticky bit, as `/tmp`
/// is. Either way no other account can delete `inner`, so `inner` passes. A file in `outer` still
/// fails, because its own folder keeps the full write test.
// covers: tools:config/trust
#[test]
fn a_parent_with_modify_for_every_account_does_not_fail_the_folder_inside_it() {
    let (_dir, outer, inner) = nested("parent-modify");
    #[cfg(windows)]
    icacls(&outer, &["/grant:r", &format!("{USERS}:M")]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&outer, fs::Permissions::from_mode(0o1777))
            .expect("a sticky world-writable mode");
    }

    check(&inner).unwrap_or_else(|why| panic!("{}", why.message()));
    let file = outer.join("nvs.toml");
    fs::write(&file, "").expect("a configuration file");
    let why = check(&file).expect_err("every account may write the file's own folder");
    assert!(
        matches!(why, Untrusted::Breach { .. }),
        "the file's folder keeps the full write test: {why:?}",
    );
    #[cfg(windows)]
    icacls(&outer, &["/remove:g", USERS]);
}

/// A folder above a checked folder that lets every account delete its entries fails the folder
/// inside it, and the refusal names the outer folder. On Windows that is the `(DC)` right. On
/// Unix it is a write bit for others without the sticky bit.
// covers: tools:config/trust
#[test]
fn a_parent_that_lets_every_account_delete_entries_fails_the_folder_inside_it() {
    let (_dir, outer, inner) = nested("parent-delete-child");
    #[cfg(windows)]
    icacls(&outer, &["/grant:r", &format!("{USERS}:(DC)")]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&outer, fs::Permissions::from_mode(0o777))
            .expect("a world-writable mode");
    }

    let named = nvs_config::trust::canonical(&outer).expect("the outer folder exists");
    let why = check(&inner).expect_err("every account could delete or replace `inner`");
    assert!(
        matches!(why, Untrusted::Breach { .. })
            && why
                .message()
                .starts_with(&format!("`{}` ", named.display())),
        "the refusal names the outer folder: {why:?}",
    );
    #[cfg(windows)]
    icacls(&outer, &["/remove:g", USERS]);
}
