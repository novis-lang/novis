//! `nvs init` — `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3's
//! file, written because an operator asked for it rather than because a command resolved no tree.
//!
//! Through the built binary rather than by calling `config::init`, for the reason
//! [`check_grants`](check_grants) already writes down: `nvs-cli` is a binary crate with no library
//! target, and what is under test here is a command whose entire input is the working directory of
//! the process running it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A directory of this test's own, two levels below the temp dir.
///
/// Nested rather than placed directly in it because `rule:config/ownership-is-the-trust-boundary`
/// is asked about the directory before anything is created there, and a Unix `/tmp` is mode `1777`:
/// a case placed directly in it would be answered by that refusal rather than by the write it is
/// about. The root this nests under is created by this process and carries the umask's ordinary
/// bits.
fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("nvs-init-cmd-{}", std::process::id()));
    let dir = root.join(format!("{unique}-{name}"));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("a scratch directory under the temp dir is creatable");
    dir
}

/// `nvs init`, run *in* `dir`.
fn init(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("init")
        .current_dir(dir)
        .output()
        .expect("the binary under test runs")
}

/// The explicit door, both halves: it writes the file a project command would have written for
/// itself, byte for byte, and a second run refuses rather than replacing it.
///
/// The refusal is an `error:` and a non-zero exit, where the same refusal on a run is a note and
/// nothing else — a command that exists only to produce this file has nothing to carry on with, and
/// a script calling it has to be able to tell "one is already there" from "one was written".
#[test]
fn nvs_init_writes_the_same_file_and_refuses_to_overwrite_one() {
    let dir = scratch("explicit");
    let written = dir.join("nvs.toml");

    let first = init(&dir);
    assert!(
        first.status.success(),
        "a directory this process created takes the file: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        String::from_utf8_lossy(&first.stdout).contains("nvs.toml"),
        "and says where it went: {:?}",
        String::from_utf8_lossy(&first.stdout)
    );
    assert_eq!(
        std::fs::read(&written).expect("the file the command reported is on disk"),
        nvs_config::default_file().as_bytes(),
        "what it wrote is the shipped default file and nothing else"
    );

    let second = init(&dir);
    assert!(
        !second.status.success(),
        "a second run is a refusal, not a rewrite"
    );
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.starts_with("error:"),
        "asking for a file that cannot be written is an error here: {stderr:?}"
    );
    assert!(
        stderr.contains("never overwritten"),
        "and it says why: {stderr:?}"
    );
    assert_eq!(
        std::fs::read(&written).expect("the file is still there"),
        nvs_config::default_file().as_bytes(),
        "the file the first run wrote is untouched"
    );
}

/// `--config` names where the file goes, exactly as it names where `nvs serve` reads it from: a
/// binary on the `PATH` run from anywhere writes into the directory an operator set aside for
/// configuration, and leaves the directory it was run in alone.
///
/// Two paths are a refusal that writes neither, because the flag is repeatable for a tree with
/// several roots and a template is one file.
#[test]
fn nvs_init_writes_to_the_one_path_config_names() {
    let elsewhere = scratch("named-cwd");
    let dir = scratch("named-target");
    let named = dir.join("server.toml");

    let run = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--config")
        .arg(&named)
        .arg("init")
        .current_dir(&elsewhere)
        .output()
        .expect("the binary under test runs");
    assert!(
        run.status.success(),
        "a named path in a directory this process created takes the file: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        std::fs::read(&named).expect("the named file is on disk"),
        nvs_config::default_file().as_bytes(),
        "what it wrote there is the shipped default file"
    );
    assert!(
        !elsewhere.join("nvs.toml").exists(),
        "and the working directory was left as it was found"
    );

    let second = dir.join("second.toml");
    let both = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--config")
        .arg(&second)
        .arg("--config")
        .arg(dir.join("third.toml"))
        .arg("init")
        .current_dir(&elsewhere)
        .output()
        .expect("the binary under test runs");
    assert!(
        !both.status.success() && !second.exists(),
        "two named paths are refused and neither is written: {}",
        String::from_utf8_lossy(&both.stderr)
    );
}

/// Every file under `dir`, at any depth.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("a folder of the layout is readable") {
        let path = entry.expect("a readable entry").path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// The install chapter's simplest layout, from an empty folder to a program that ran: `config`,
/// `cache` and `logs` inside one folder, `nvs` started from another directory, and `--config` the
/// only thing saying where the configuration is.
///
/// `nvs init` writes nothing where the folder does not exist, and the file it does write has every
/// key commented out, so its only lines that are neither blank nor a comment are block headers.
/// The two keys that point at the other two folders are then set the way an operator sets them —
/// the `#` removed and the path written with `/` — and one `nvs run` uses both: the compiled
/// program is stored under `cache`, and the record the program logged is in `logs` and not on
/// `stderr`.
// covers: tools:install/the-layout
#[test]
fn the_simplest_layout_is_written_by_init_and_one_run_from_elsewhere_uses_cache_and_logs() {
    let base = scratch("layout");
    let elsewhere = scratch("layout-cwd");
    for folder in ["config", "cache", "logs"] {
        std::fs::create_dir(base.join(folder)).expect("a folder of the layout is creatable");
    }
    let nvs = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        // A recorded run of this test binary sets the switch, and this case is about the cache.
        command
            .current_dir(&elsewhere)
            .env_remove("NOVIS_NO_FILE_CACHE");
        command
    };

    let missing = base.join("conf").join("nvs.toml");
    let refused = nvs()
        .arg("--config")
        .arg(&missing)
        .arg("init")
        .output()
        .expect("the binary under test runs");
    assert!(
        !refused.status.success() && !base.join("conf").exists(),
        "a folder that does not exist is not created for the file: {}",
        String::from_utf8_lossy(&refused.stderr)
    );

    let config = base.join("config").join("nvs.toml");
    let written = nvs()
        .arg("--config")
        .arg(&config)
        .arg("init")
        .output()
        .expect("the binary under test runs");
    assert!(
        written.status.success(),
        "the config folder takes the file: {}",
        String::from_utf8_lossy(&written.stderr)
    );
    let template = std::fs::read_to_string(&config).expect("the file is where `--config` named");
    let live: Vec<&str> = template
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    assert!(
        !live.is_empty()
            && live
                .iter()
                .all(|line| line.starts_with('[') && line.ends_with(']')),
        "every key is commented out, so only block headers are live: {live:?}"
    );

    let slashed = |path: PathBuf| path.display().to_string().replace('\\', "/");
    let cache_line = format!("file_cache_dir = \"{}\"", slashed(base.join("cache")));
    let log_line = format!(
        "target = \"file:{}\"",
        slashed(base.join("logs").join("nvs.log"))
    );
    let mut block = "";
    let mut set = 0;
    let edited: Vec<&str> = template
        .lines()
        .map(|line| {
            if line.starts_with('[') {
                block = line;
            }
            match (block, line.split(' ').next()) {
                ("[opcache]", Some("#file_cache_dir")) => {
                    set += 1;
                    cache_line.as_str()
                }
                ("[log]", Some("#target")) => {
                    set += 1;
                    log_line.as_str()
                }
                _ => line,
            }
        })
        .collect();
    assert_eq!(
        set, 2,
        "the file names both keys, each once under its own block"
    );
    std::fs::write(&config, edited.join("\n")).expect("the configuration is editable");

    let program = elsewhere.join("index.nvs");
    std::fs::write(
        &program,
        "<?nvs\nCore\\Log::write(Core\\Log\\Level::Error, \"started\");\necho \"served\\n\";\n",
    )
    .expect("the program is written");
    let ran = nvs()
        .arg("--config")
        .arg(&config)
        .arg("run")
        .arg(&program)
        .output()
        .expect("the binary under test runs");
    let stderr = String::from_utf8_lossy(&ran.stderr);
    assert!(
        ran.status.success(),
        "the edited file is a configuration: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "served\n");
    assert!(
        !stderr.contains("file_cache_dir") && !stderr.contains("started"),
        "the cache folder is not refused and the record is not on `stderr`: {stderr}"
    );
    assert!(
        files_under(&base.join("cache"))
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "nvsc")),
        "the compiled program is stored under the cache folder"
    );
    let log = std::fs::read_to_string(base.join("logs").join("nvs.log"))
        .expect("the log file is in the log folder");
    assert!(
        log.contains("\"msg\":\"started\""),
        "and holds the record the program wrote: {log}"
    );

    drop(std::fs::remove_dir_all(&base));
    drop(std::fs::remove_dir_all(&elsewhere));
}

/// Gives every account on the host the right to write in `dir`, or takes it back: the one state
/// `rule:config/ownership-is-the-trust-boundary` refuses, in each platform's own terms.
///
/// On Windows that is a grant to `BUILTIN\Users`, named by its SID because a group's name is
/// localized, and carrying no inheritance flags so a folder inside `dir` does not take it. On Unix
/// it is the write bit for others.
fn writable_by_others(dir: &Path, open: bool) {
    #[cfg(windows)]
    {
        const USERS: &str = "*S-1-5-32-545";
        let grant = format!("{USERS}:(WD)");
        if open {
            icacls(dir, &["/grant:r", &grant]);
        } else {
            icacls(dir, &["/remove:g", USERS]);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = if open { 0o757 } else { 0o755 };
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode))
            .expect("a scratch directory's mode is this process's to change");
    }
}

/// One `icacls` edit of `dir`, which has to succeed.
#[cfg(windows)]
fn icacls(dir: &Path, edit: &[&str]) {
    // `icacls` edits the scratch directory it is given and opens nothing in the repository.
    let ran = nvs_repo::spawn("icacls", &[])
        .arg(dir)
        .args(edit)
        .output()
        .expect("`icacls` ships with Windows");
    assert!(
        ran.status.success(),
        "icacls {edit:?} on `{}`: {}",
        dir.display(),
        String::from_utf8_lossy(&ran.stdout)
    );
}

/// The install chapter's table of what each command checks, for the two rows that look one level
/// up: `nvs init` checks the folder it writes into and the folder that contains that folder, and
/// a command that compiles checks the cache folder the same way.
///
/// A folder every account may write fails `nvs init` whichever of the two it is: exit status `1`,
/// an `error:` naming the folder that failed — the outer one when that is the one to change — and
/// no file. Taking the right back is the whole repair.
///
/// The same outer folder fails the cache folder inside it, and that is a `warning:` and a program
/// that still runs with nothing stored. `nvs run` reads its configuration out of a folder every
/// account may write without a word, because only `nvs serve` and `nvs ctl reload` check the
/// configuration files.
// covers: tools:install/what-novis-checks
#[test]
fn nvs_init_and_the_cache_are_checked_one_folder_up_and_nvs_run_does_not_check_the_config() {
    let outer = scratch("one-level-up");
    let elsewhere = scratch("one-level-up-cwd");
    let inner = outer.join("config");
    let cache = outer.join("cache");
    for folder in [&inner, &cache] {
        std::fs::create_dir(folder).expect("a folder of the layout is creatable");
    }
    let target = inner.join("nvs.toml");
    let nvs = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        // A recorded run of this test binary sets the switch, and half of this case is the cache.
        command
            .current_dir(&elsewhere)
            .env_remove("NOVIS_NO_FILE_CACHE")
            .arg("--config")
            .arg(&target);
        command
    };
    // A message names the folder it is about between backticks, which the path of the file to be
    // written — the same folder and more — never matches.
    let named = |folder: &Path| {
        let canonical = nvs_config::trust::canonical(folder).expect("the folder exists");
        format!("`{}` ", canonical.display())
    };

    for failing in [&outer, &inner] {
        writable_by_others(failing, true);
        let refused = nvs()
            .arg("init")
            .output()
            .expect("the binary under test runs");
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert_eq!(
            refused.status.code(),
            Some(1),
            "a folder every account may write is a refusal: {stderr}"
        );
        assert!(!target.exists(), "and nothing is written");
        assert!(
            stderr.starts_with("error:") && stderr.contains(&named(failing)),
            "the message names the folder to change, `{}`: {stderr}",
            failing.display()
        );
        assert!(
            stderr.contains("the directory that contains it"),
            "and says that two folders were checked: {stderr}"
        );
        writable_by_others(failing, false);
    }

    let written = nvs()
        .arg("init")
        .output()
        .expect("the binary under test runs");
    assert!(
        written.status.success() && target.exists(),
        "with the right taken back from both, the file is written: {}",
        String::from_utf8_lossy(&written.stderr)
    );

    let slashed = cache.display().to_string().replace('\\', "/");
    std::fs::write(
        &target,
        format!("[opcache]\nfile_cache_dir = \"{slashed}\"\n"),
    )
    .expect("the configuration is editable");
    let program = elsewhere.join("index.nvs");
    std::fs::write(&program, "<?nvs\necho \"served\\n\";\n").expect("the program is written");
    let stored = || {
        files_under(&cache)
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "nvsc"))
    };

    writable_by_others(&inner, true);
    writable_by_others(&outer, true);
    let warned = nvs()
        .arg("run")
        .arg(&program)
        .output()
        .expect("the binary under test runs");
    let stderr = String::from_utf8_lossy(&warned.stderr);
    assert!(
        warned.status.success() && String::from_utf8_lossy(&warned.stdout) == "served\n",
        "the program runs, its configuration read from a folder every account may write: {stderr}"
    );
    assert!(
        stderr.starts_with("warning:") && stderr.contains(&named(&outer)),
        "the cache folder is refused for the folder that contains it, `{}`: {stderr}",
        outer.display()
    );
    assert!(!stored(), "and a refused cache folder stores nothing");

    writable_by_others(&outer, false);
    let quiet = nvs()
        .arg("run")
        .arg(&program)
        .output()
        .expect("the binary under test runs");
    let stderr = String::from_utf8_lossy(&quiet.stderr);
    assert!(
        quiet.status.success() && stderr.is_empty(),
        "with the right taken back from the outer folder there is nothing to say: {stderr}"
    );
    assert!(stored(), "and the compiled program is stored");

    drop(std::fs::remove_dir_all(&outer));
    drop(std::fs::remove_dir_all(&elsewhere));
}

/// The install chapter's Windows steps, run with `icacls` on a layout that starts the way Windows
/// creates one on a drive that is not the system drive: `Authenticated Users` may change the
/// folder the layout is made in, and every folder of the layout takes that entry from it.
///
/// `nvs init` refuses that layout, and its message names the group, the SID `icacls` takes for it
/// and the commands of step 2. `/remove:g` alone is no repair, because an entry a folder takes
/// from the one above it is not that folder's to remove; after `/inheritance:d` it is, and the two
/// commands on the one outer folder are the repair for the folders inside it as well. Steps 3 and
/// 4, and the grant that gives reading back, change nothing the check reads: a group that may only
/// read and run passes, and so does a single account that may change the cache folder.
///
/// Each group in the chapter's SID table is then given the right to change the configuration
/// folder by the SID the table has for it. The three groups of ordinary accounts are refused with
/// that SID printed back, and `Administrators` passes.
// covers: tools:install/windows
#[cfg(windows)]
#[test]
fn the_icacls_steps_turn_a_folder_every_account_may_change_into_one_nvs_init_writes_into() {
    const AUTHENTICATED_USERS: &str = "*S-1-5-11";
    const USERS: &str = "*S-1-5-32-545";
    const EVERYONE: &str = "*S-1-1-0";
    const ADMINISTRATORS: &str = "*S-1-5-32-544";
    // `LOCAL SERVICE` is the chapter's `svc-novis` here: an account every Windows has, which is
    // not the one running this test and is in none of the groups the check reads.
    const SERVICE: &str = "*S-1-5-19";

    let drive = scratch("windows-steps");
    let elsewhere = scratch("windows-steps-cwd");
    icacls(
        &drive,
        &["/grant", &format!("{AUTHENTICATED_USERS}:(OI)(CI)M")],
    );
    let novis = drive.join("novis");
    let config = novis.join("config");
    let cache = novis.join("cache");
    for folder in [&config, &cache, &novis.join("logs")] {
        std::fs::create_dir_all(folder).expect("step 1 creates the folders");
    }
    let target = config.join("nvs.toml");
    let nvs = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        // A recorded run of this test binary sets the switch, and the last step is the cache.
        command
            .current_dir(&elsewhere)
            .env_remove("NOVIS_NO_FILE_CACHE")
            .arg("--config")
            .arg(&target);
        command
    };
    let refused = |why: &str| {
        let ran = nvs()
            .arg("init")
            .output()
            .expect("the binary under test runs");
        let stderr = String::from_utf8_lossy(&ran.stderr).into_owned();
        assert_eq!(ran.status.code(), Some(1), "{why}: {stderr}");
        assert!(
            stderr.starts_with("error:") && !target.exists(),
            "{why}, and nothing is written: {stderr}"
        );
        stderr
    };
    let written = |why: &str| {
        let ran = nvs()
            .arg("init")
            .output()
            .expect("the binary under test runs");
        assert!(
            ran.status.success() && target.exists(),
            "{why}: {}",
            String::from_utf8_lossy(&ran.stderr)
        );
        std::fs::remove_file(&target).expect("the file it wrote is this account's to delete");
    };

    let stderr = refused("a layout made on such a drive is one every account may change");
    assert!(
        stderr.contains("Authenticated Users")
            && stderr.contains(&format!("`{AUTHENTICATED_USERS}`")),
        "the message names the group and the SID to remove: {stderr}"
    );
    for command in [
        "icacls <path> /inheritance:d",
        "icacls <path> /remove:g <SID>",
        "icacls <path> /grant *S-1-5-32-545:(OI)(CI)RX",
    ] {
        assert!(
            stderr.contains(command),
            "and gives the chapter's `{command}`: {stderr}"
        );
    }

    icacls(&novis, &["/remove:g", AUTHENTICATED_USERS]);
    refused("`/remove:g` leaves an entry the folder takes from the one above it");

    icacls(&novis, &["/inheritance:d"]);
    icacls(&novis, &["/remove:g", AUTHENTICATED_USERS]);
    written("step 2 on the outer folder repairs `config` with it");

    icacls(&config, &["/inheritance:d"]);
    icacls(&config, &["/remove:g", USERS]);
    icacls(&config, &["/grant", &format!("{SERVICE}:(OI)(CI)RX")]);
    for folder in [&cache, &novis.join("logs")] {
        icacls(folder, &["/grant", &format!("{SERVICE}:(OI)(CI)M")]);
    }
    written("steps 3 and 4 leave a layout `nvs init` writes into");
    icacls(&config, &["/grant", &format!("{USERS}:(OI)(CI)RX")]);
    written("a group that may read and run is not one that may write");

    for (group, sid) in [
        ("Authenticated Users", AUTHENTICATED_USERS),
        ("Users", USERS),
        ("Everyone", EVERYONE),
    ] {
        icacls(&config, &["/grant", &format!("{sid}:(OI)(CI)M")]);
        let stderr = refused("a group of ordinary accounts that may change the folder");
        assert!(
            stderr.contains(group) && stderr.contains(&format!("`{sid}`")),
            "`{group}` is refused by the SID the chapter's table gives it, `{sid}`: {stderr}"
        );
        icacls(&config, &["/remove:g", sid]);
    }
    icacls(&config, &["/grant", &format!("{ADMINISTRATORS}:(OI)(CI)F")]);
    written("`Administrators` with every right is inside the boundary");

    let slashed = cache.display().to_string().replace('\\', "/");
    std::fs::write(
        &target,
        format!("[opcache]\nfile_cache_dir = \"{slashed}\"\n"),
    )
    .expect("the configuration is editable");
    let program = elsewhere.join("index.nvs");
    std::fs::write(&program, "<?nvs\necho \"served\\n\";\n").expect("the program is written");
    let ran = nvs()
        .arg("run")
        .arg(&program)
        .output()
        .expect("the binary under test runs");
    let stderr = String::from_utf8_lossy(&ran.stderr);
    assert!(
        ran.status.success() && stderr.is_empty(),
        "a cache folder one named account may change is not refused: {stderr}"
    );
    assert!(
        files_under(&cache)
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "nvsc")),
        "and the compiled program is stored in it"
    );

    drop(std::fs::remove_dir_all(&drive));
    drop(std::fs::remove_dir_all(&elsewhere));
}

/// Takes from this account the right to create a file in `dir`, or gives it back, and changes
/// nothing `rule:config/ownership-is-the-trust-boundary` reads: no account gains a right.
///
/// On Windows that is an entry denying `Everyone` the one right, because a read-only folder there
/// still takes a new file. On Unix it is the owner's write bit.
fn writable_by_this_account(dir: &Path, writable: bool) {
    #[cfg(windows)]
    {
        const EVERYONE: &str = "*S-1-1-0";
        if writable {
            icacls(dir, &["/remove:d", EVERYONE]);
        } else {
            icacls(dir, &["/deny", &format!("{EVERYONE}:(WD)")]);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = if writable { 0o755 } else { 0o555 };
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode))
            .expect("a scratch directory's mode is this process's to change");
    }
}

/// The install chapter's table of messages, row by row: every refusal names a path, and what the
/// table says to do about that path is the whole repair.
///
/// A folder a group of ordinary accounts may write is named in the message, the outer folder here
/// although the file was going one level below it, and on Windows the SID to remove is printed
/// beside it. A file already there is kept byte for byte, and deleting it is what lets the next
/// run write one. A folder that passes the check and that this account may not write is the
/// operating system's own refusal: its error number, the file named, and no remedy, because
/// nothing about the folder is too open. A cache folder that fails the check is a `warning:` and a
/// program that still runs, with nothing stored until the named folder is repaired.
///
/// The row for a path another account owns is `nvs-config`'s `tests/trust.rs`: giving a folder to
/// another account takes a right a test does not have.
// covers: tools:install/when-something-is-refused
#[test]
fn each_refusal_names_a_path_and_the_tables_repair_is_the_whole_repair() {
    let outer = scratch("refused");
    let elsewhere = scratch("refused-cwd");
    let inner = outer.join("config");
    let cache = outer.join("cache");
    for folder in [&inner, &cache] {
        std::fs::create_dir(folder).expect("a folder of the layout is creatable");
    }
    let target = inner.join("nvs.toml");
    let nvs = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        // A recorded run of this test binary sets the switch, and the last row is the cache.
        command
            .current_dir(&elsewhere)
            .env_remove("NOVIS_NO_FILE_CACHE")
            .arg("--config")
            .arg(&target);
        command
    };
    let init = || {
        let ran = nvs()
            .arg("init")
            .output()
            .expect("the binary under test runs");
        (
            ran.status.code(),
            String::from_utf8_lossy(&ran.stderr).into_owned(),
        )
    };
    let outer_named = format!(
        "`{}` ",
        nvs_config::trust::canonical(&outer)
            .expect("the folder exists")
            .display()
    );

    // Row 1: a group of ordinary accounts can write to the named path.
    #[cfg(windows)]
    let open: [(&dyn Fn(), &str); 1] = [(
        &|| writable_by_others(&outer, true),
        "grants write access to `BUILTIN\\Users` (`*S-1-5-32-545`",
    )];
    #[cfg(unix)]
    let chmod = |bits: u32| {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&outer, std::fs::Permissions::from_mode(bits))
            .expect("a scratch directory's mode is this process's to change");
    };
    #[cfg(unix)]
    let open: [(&dyn Fn(), &str); 3] = [
        (&|| chmod(0o775), "is group-writable"),
        (&|| chmod(0o757), "is world-writable"),
        (&|| chmod(0o777), "is group- and world-writable"),
    ];
    for (opened, said) in open {
        opened();
        let (status, stderr) = init();
        assert_eq!(
            status,
            Some(1),
            "a folder others may write is a refusal: {stderr}"
        );
        assert!(
            stderr.starts_with("error:") && stderr.contains(&format!("{outer_named}{said}")),
            "the message names the outer folder and says `{said}`: {stderr}"
        );
        assert!(!target.exists(), "and nothing is written");
        writable_by_others(&outer, false);
    }
    let (status, stderr) = init();
    assert!(
        status == Some(0) && target.exists(),
        "removing that right from the named folder is the whole repair: {stderr}"
    );

    // Row 4: `nvs init` found a file at that path.
    let edited = "[limits]\nmemory = \"96M\"\n";
    std::fs::write(&target, edited).expect("the configuration is editable");
    let (status, stderr) = init();
    assert_eq!(
        status,
        Some(1),
        "a file already there is a refusal: {stderr}"
    );
    assert!(
        stderr
            .contains("nvs.toml` was not written: it already exists, and it is never overwritten"),
        "the message names the file and says it is kept: {stderr}"
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("the file is still there"),
        edited,
        "and the edited file is kept byte for byte"
    );
    std::fs::remove_file(&target).expect("the file is this account's to delete");

    // Row 3: the permissions are strict enough, and this account cannot write there. `root`
    // writes whatever the mode says, so the row is not reachable as that account.
    #[cfg(windows)]
    let (denied, reachable) = ("(os error 5)", true);
    #[cfg(unix)]
    let (denied, reachable) = ("(os error 13)", {
        use std::os::unix::fs::MetadataExt as _;
        std::fs::metadata(&outer).expect("the folder exists").uid() != 0
    });
    if reachable {
        writable_by_this_account(&inner, false);
        let (status, stderr) = init();
        assert_eq!(
            status,
            Some(1),
            "a folder this account may not write is a refusal: {stderr}"
        );
        assert!(
            stderr.contains("nvs.toml` was not written: ") && stderr.contains(denied),
            "the message names the file and carries `{denied}`: {stderr}"
        );
        assert!(
            !stderr.contains("help:") && !target.exists(),
            "with no remedy, because no permission is too open, and no file: {stderr}"
        );
        writable_by_this_account(&inner, true);
    }
    let (status, stderr) = init();
    assert!(
        status == Some(0) && target.exists(),
        "with no file there and the right to write, the file is written: {stderr}"
    );

    // Row 5: the cache folder, or the folder that contains it, failed the check.
    let slashed = cache.display().to_string().replace('\\', "/");
    std::fs::write(
        &target,
        format!("[opcache]\nfile_cache_dir = \"{slashed}\"\n"),
    )
    .expect("the configuration is editable");
    let program = elsewhere.join("index.nvs");
    std::fs::write(&program, "<?nvs\necho \"served\\n\";\n").expect("the program is written");
    let run = || {
        let ran = nvs()
            .arg("run")
            .arg(&program)
            .output()
            .expect("the binary under test runs");
        assert!(
            ran.status.success() && String::from_utf8_lossy(&ran.stdout) == "served\n",
            "the program works whatever the cache folder is: {}",
            String::from_utf8_lossy(&ran.stderr)
        );
        String::from_utf8_lossy(&ran.stderr).into_owned()
    };
    let stored = || {
        files_under(&cache)
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "nvsc"))
    };

    writable_by_others(&outer, true);
    let stderr = run();
    assert!(
        stderr.starts_with(&format!(
            "warning: `[opcache] file_cache_dir = \"{slashed}\"` is not used"
        )) && stderr.contains(&outer_named),
        "the warning quotes the setting and names the folder to change: {stderr}"
    );
    assert!(!stored(), "and until then nothing is stored");

    writable_by_others(&outer, false);
    let stderr = run();
    assert!(
        stderr.is_empty() && stored(),
        "with the named folder repaired the cache is used and nothing is said: {stderr}"
    );

    drop(std::fs::remove_dir_all(&outer));
    drop(std::fs::remove_dir_all(&elsewhere));
}
