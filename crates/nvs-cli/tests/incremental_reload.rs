//! `[opcache] incremental`, the edit state a serving process keeps between two compiles of one
//! program: each source file's checked result and each function's relocatable code.
//!
//! The directive's startup default follows the mode, as `settle`'s does
//! (`rule:config/a-startup-default-is-never-flipped`), and it is `System`-class, so no request can
//! make a server keep memory or free it. The counts a reload does with it on are the
//! `program reloaded` record's, and every claim here is a count, never a clock
//! (`rule:programs/less-work-is-taken`).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use nvs_test::serve::{BOOT, POLL};

/// How long one record has to arrive before the case fails.
const BOUND: Duration = Duration::from_secs(60);

/// The `[opcache]` and `[server]` lines of every serving case: only `nvs reload` starts a
/// check, because the timed one runs once an hour.
const HOURLY: &str = "[opcache]\nvalidate = \"hash\"\nrevalidate_freq = \"1h\"\nsettle = \"1m\"\n\n\
                      [server]\ndrain_timeout = \"5s\"\n";

/// Two library files and the entry that requires both. `a.nvs` and `b.nvs` read nothing of each
/// other, and only the entry calls `B::two`.
const A: &str = "<?nvs\nclass A {\n    public static function one(): string {\n        return \"one\";\n    }\n}\n";
const B: &str = "<?nvs\nclass B {\n    public static function two(): string {\n        return \"two\";\n    }\n}\n";
const APP: &str = "<?nvs\nrequire './a.nvs';\nrequire './b.nvs';\necho A::one(), B::two();\n";

/// `a.nvs` with an edit inside `A::one`'s body that moves no line.
const A_BODY: &str = "<?nvs\nclass A {\n    public static function one(): string {\n        return \"uno\";\n    }\n}\n";

/// `b.nvs` with `B::two`'s signature moved: it takes an optional parameter.
const B_SIGNATURE: &str = "<?nvs\nclass B {\n    public static function two(int $n = 0): string {\n        return \"two\";\n    }\n}\n";

/// A running `nvs serve app.nvs` over [`A`], [`B`] and [`APP`] in a directory of its own. Dropping
/// it kills the process and removes the directory.
struct Served {
    child: Child,
    dir: PathBuf,
    data: nvs_repo::Scratch,
    stderr: Arc<Mutex<String>>,
}

impl Served {
    /// Starts the server over [`A`], [`B`] and [`APP`] in a fresh directory named for `case`,
    /// with `config` as its `nvs.toml`, and waits until it listens.
    fn start(case: &str, config: &str) -> Self {
        let files = [("a.nvs", A), ("b.nvs", B), ("app.nvs", APP)]
            .map(|(name, text)| (name.to_owned(), text.to_owned()));
        Self::start_over(case, config, &files)
    }

    /// Starts the server over `files`, each a name and its text, whose entry is `app.nvs`.
    fn start_over(case: &str, config: &str, files: &[(String, String)]) -> Self {
        let dir = scratch(case);
        for (name, text) in files {
            write_file(&dir.join(name), text);
        }
        std::fs::write(dir.join("nvs.toml"), config).expect("the case's directory takes a file");
        let data = nvs_repo::scratch_private_for_socket("i", "run/serve-99999-1700000000000.new");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(data.path())
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");
        let stderr = Arc::new(Mutex::new(String::new()));
        let collected = Arc::clone(&stderr);
        let err_pipe = child.stderr.take().expect("standard error is piped");
        thread::spawn(move || {
            for line in BufReader::new(err_pipe).lines().map_while(Result::ok) {
                let mut all = collected.lock().expect("no reader panicked");
                all.push_str(&line);
                all.push('\n');
            }
        });
        let out_pipe = child.stdout.take().expect("standard output is piped");
        let (sent, listening) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(out_pipe).lines().map_while(Result::ok) {
                if line.starts_with("listening on http://") {
                    let _ = sent.send(());
                }
            }
        });
        let served = Self {
            child,
            dir,
            data,
            stderr,
        };
        assert!(
            listening.recv_timeout(BOOT).is_ok(),
            "`nvs serve` did not start listening within {BOOT:?}: {}",
            served.said()
        );
        served
    }

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// Replaces `name` in the served directory with `text`, runs `nvs reload`, and returns the
    /// `program reloaded` record it caused.
    fn edit_and_reload(&self, name: &str, text: &str) -> String {
        let before = self.records().len();
        write_file(&self.dir.join(name), text);
        let ran = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(self.data.path())
            .arg("reload")
            .stdin(Stdio::null())
            .output()
            .expect("the `nvs` binary this test was built beside starts");
        assert_eq!(
            ran.status.code(),
            Some(0),
            "`nvs reload` printed {}{}\nthe server wrote: {}",
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr),
            self.said()
        );
        let started = Instant::now();
        while self.records().len() <= before {
            assert!(
                started.elapsed() < BOUND,
                "no `program reloaded` record within {BOUND:?}: {}",
                self.said()
            );
            thread::sleep(POLL);
        }
        self.records().remove(before)
    }

    /// Every `program reloaded` line so far.
    fn records(&self) -> Vec<String> {
        self.said()
            .lines()
            .filter(|line| line.contains("program reloaded"))
            .map(str::to_owned)
            .collect()
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Writes `text` to `path` in one rename, so the server never reads half a file.
fn write_file(path: &Path, text: &str) {
    let name = path.file_name().expect("the path names a file");
    let temp = path.with_file_name(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::write(&temp, text).expect("the case's directory takes a file");
    std::fs::rename(&temp, path).expect("the case's file is replaced");
}

/// The count `field` of a `program reloaded` record, written as `field=N` or `"field":N`.
fn count(record: &str, field: &str) -> usize {
    [format!("{field}="), format!("\"{field}\":")]
        .iter()
        .find_map(|at| {
            let rest = &record[record.find(at.as_str())? + at.len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .unwrap_or_else(|| panic!("the record has no `{field}` count: {record}"))
}

/// A development server's configuration, with its log in JSON, which carries a record's counts.
fn development() -> String {
    format!("[mode]\ndefault = \"development\"\n\n[log]\nformat = \"json\"\n\n{HOURLY}")
}

/// An empty directory of this case's own under `CARGO_TARGET_TMPDIR`.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("incremental-reload-{case}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    dir
}

/// The `opcache.incremental` line of `nvs config dump --origin` over `toml`, written as `dir`'s
/// `nvs.toml`, with `dir/.nvsdata` as the data folder.
fn incremental_line(dir: &Path, toml: &str) -> String {
    std::fs::write(dir.join("nvs.toml"), toml).expect("the case's directory takes a file");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(dir.join(".nvsdata"))
        .args(["config", "dump", "--origin", "nvs.toml"])
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stdout = String::from_utf8(out.stdout).expect("the audit is UTF-8");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
        .lines()
        .find(|line| line.starts_with("opcache.incremental "))
        .unwrap_or_else(|| panic!("`opcache.incremental` has a row: {stdout}"))
        .trim_end()
        .to_owned()
}

/// An unwritten `incremental` is `false` in `production` and `true` in `development`, and the
/// dump names the mode that chose it. A written value wins over the mode. No request may set it,
/// and a reload applies it.
#[test]
fn incremental_startup_default_follows_the_mode() {
    use nvs_config::directive::{Apply, lookup};

    let dir = scratch("default");

    let production = incremental_line(&dir, "[mode]\ndefault = \"production\"\n");
    assert!(
        production.contains("= false ") && production.ends_with("    default (production)"),
        "production frees the edit state: {production}"
    );

    let development = incremental_line(&dir, "[mode]\ndefault = \"development\"\n");
    assert!(
        development.contains("= true ") && development.ends_with("    default (development)"),
        "development keeps the edit state: {development}"
    );

    let written = incremental_line(
        &dir,
        "[mode]\ndefault = \"development\"\n\n[opcache]\nincremental = false\n",
    );
    assert!(
        written.contains("= false ") && written.ends_with("nvs.toml"),
        "a written value wins over the mode's default: {written}"
    );

    let row = lookup("opcache.incremental").expect("`opcache.incremental` has a registry row");
    assert_eq!(row.apply, Apply::Reload, "a reload applies it");
    assert_eq!(
        nvs_config::settable::how("opcache.incremental"),
        None,
        "it is `System`: no request may set it"
    );

    drop(std::fs::remove_dir_all(&dir));
}

/// An edit inside one method's body checks the one file it is in and compiles the one function
/// it moved. The entry, which calls that method, is not checked again.
// covers: tools:server/nvs-reload
#[test]
fn a_body_edit_reload_checks_one_file_and_compiles_one_function() {
    let served = Served::start("body", &development());
    let record = served.edit_and_reload("a.nvs", A_BODY);
    assert_eq!(
        count(&record, "checked"),
        1,
        "the record: {record}\nthe server wrote: {}",
        served.said()
    );
    assert_eq!(count(&record, "compiled"), 1, "the record: {record}");
}

/// A signature edit checks its own file and the file that calls the method, and not the file
/// that reads nothing of it.
// covers: tools:server/nvs-reload
#[test]
fn a_signature_edit_rechecks_the_files_that_use_it() {
    let served = Served::start("signature", &development());
    let record = served.edit_and_reload("b.nvs", B_SIGNATURE);
    assert_eq!(
        count(&record, "checked"),
        2,
        "`b.nvs` and the entry that calls `B::two`: {record}"
    );
}

/// A production server keeps no checked file and no function code between two compiles: with the
/// disk cache off, a body edit checks every file and compiles every function again.
// covers: tools:server/nvs-reload
#[test]
fn a_production_server_frees_the_edit_state_after_a_compile() {
    let config = format!("[mode]\ndefault = \"production\"\n\n{HOURLY}")
        .replace("[opcache]\n", "[opcache]\nfile_cache = false\n");
    let served = Served::start("production", &config);
    let record = served.edit_and_reload("a.nvs", A_BODY);
    assert_eq!(count(&record, "checked"), 3, "every file: {record}");
    assert_eq!(count(&record, "reused"), 0, "no kept code: {record}");
    assert!(count(&record, "compiled") > 1, "every function: {record}");
}

/// The record of a swap counts the files checked and the functions compiled and reused: after a
/// body edit, one compiled and the rest reused.
// covers: tools:server/nvs-reload
#[test]
fn the_reload_record_counts_compiled_and_reused_functions() {
    let served = Served::start("counts", &development());
    let first = served.edit_and_reload("a.nvs", A_BODY);
    assert_eq!(count(&first, "compiled"), 1, "the record: {first}");
    let reused = count(&first, "reused");
    assert!(reused >= 2, "`B::two` and the entry are reused: {first}");
    // A second body edit compiles the one function again and reuses the same others.
    let again = served.edit_and_reload("a.nvs", &A_BODY.replace("uno", "eins"));
    assert_eq!(count(&again, "compiled"), 1, "the record: {again}");
    assert_eq!(count(&again, "reused"), reused, "the record: {again}");
}

/// Library file `n` of a generated program: one class with one method that returns `n`.
fn generated_class(n: usize, value: usize) -> String {
    format!(
        "<?nvs\nclass C{n} {{\n    public static function value(): int {{\n        return {value};\n    }}\n}}\n"
    )
}

/// A program of `size` library files and the entry that requires each and calls its method.
fn generated_program(size: usize) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = (0..size)
        .map(|n| (format!("c{n}.nvs"), generated_class(n, n)))
        .collect();
    let mut app = String::from("<?nvs\n");
    for n in 0..size {
        app.push_str(&format!("require './c{n}.nvs';\n"));
    }
    app.push_str("int $sum = 0;\n");
    for n in 0..size {
        app.push_str(&format!("$sum += C{n}::value();\n"));
    }
    app.push_str("echo $sum;\n");
    files.push(("app.nvs".to_owned(), app));
    files
}

/// The work of a reload after a body edit does not grow with the program: at 110, 220 and 440
/// files, it checks one file, compiles one function and writes one source-file entry.
// covers: tools:server/nvs-reload
#[test]
fn a_body_edit_costs_the_same_work_at_every_program_size() {
    for size in [110, 220, 440] {
        let served = Served::start_over(
            &format!("size-{size}"),
            &development(),
            &generated_program(size),
        );
        // `C7::value` returns 7, and the edit makes it return 9: the same lines, a new body.
        let record = served.edit_and_reload("c7.nvs", &generated_class(7, 9));
        let work = [
            count(&record, "checked"),
            count(&record, "compiled"),
            count(&record, "written"),
        ];
        assert_eq!(
            work,
            [1, 1, 1],
            "checked, compiled and written at {size} files: {record}"
        );
        assert!(
            count(&record, "reused") > size,
            "every other function is reused at {size} files: {record}"
        );
    }
}
