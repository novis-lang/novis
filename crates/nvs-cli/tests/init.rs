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
