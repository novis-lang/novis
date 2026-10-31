//! `nvs run`: the configuration it reads before running, and the exit status
//! it ends with, as `docs/reference/tools/10-cli.md` § *nvs run* states them.
//!
//! Through the built binary, because both are properties of the whole
//! command: which file is read is decided before any program compiles, and
//! the status is what a shell, a cron job or CI sees.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// A directory of the case's own, so two cases running in parallel never share a fixture, and
/// locked to this account: the data folder a run creates in it, and the `nvs.toml` the run writes
/// there, pass `rule:config/ownership-is-the-trust-boundary`'s check only under a parent that does.
fn scratch(case: &str) -> nvs_repo::Scratch {
    nvs_repo::scratch_private(case)
}

/// Runs `nvs run` with `args` from `dir`, which is the program's working
/// directory and so where `./nvs.toml` is looked for. The data folder is
/// `dir/.nvsdata`, so no case shares the one beside the binary.
fn run_in(dir: &Path, args: &[&str]) -> Output {
    run_with_data(dir, &dir.join(".nvsdata"), args)
}

/// [`run_in`] with the data folder `data`.
fn run_with_data(dir: &Path, data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .arg("run")
        .args(args)
        .current_dir(dir)
        .env_remove("NOVIS_NO_INIT")
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

const PROGRAM: &str = "<?nvs\necho \"ran\", \"\\n\";\n";

/// `./nvs.toml` in the working directory is read, and a `--config` flag reads
/// its own file *instead*: a broken `./nvs.toml` stops a plain run with its
/// diagnostic, and the same run with `--config` never opens it. A named file
/// that does not exist is `E0605`. With no `./nvs.toml` the run writes the
/// shipped template into the data folder, never into the working directory.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_reads_the_working_directory_s_nvs_toml_unless_a_config_is_named() {
    let dir = scratch("config-lookup");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    fs::write(dir.join("nvs.toml"), "this is [ not a configuration\n").unwrap();
    fs::write(dir.join("empty.toml"), "").unwrap();

    let looked_up = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&looked_up.stderr);
    assert_eq!(
        looked_up.status.code(),
        Some(1),
        "the broken file stops the run: {stderr}"
    );
    assert!(
        stderr.contains("E0601"),
        "the diagnostic names the file's error: {stderr}"
    );
    assert!(
        stderr.contains("nvs.toml"),
        "and the file it is in: {stderr}"
    );
    assert!(looked_up.stdout.is_empty(), "the program never ran");

    let named = run_in(&dir, &["--config", "empty.toml", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&named.stderr);
    assert_eq!(
        named.status.code(),
        Some(0),
        "`./nvs.toml` is not read at all: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&named.stdout), "ran\n");

    let missing = run_in(&dir, &["--config", "absent.toml", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert_eq!(
        missing.status.code(),
        Some(1),
        "a named file must exist: {stderr}"
    );
    assert!(stderr.contains("E0605"), "{stderr}");
    assert!(missing.stdout.is_empty(), "the program never ran");

    fs::remove_file(dir.join("nvs.toml")).unwrap();
    let none = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&none.stderr);
    assert_eq!(
        none.status.code(),
        Some(0),
        "no `./nvs.toml` is no error: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&none.stdout), "ran\n");
    assert!(
        dir.join(".nvsdata").join("nvs.toml").is_file(),
        "the run wrote the template into the data folder"
    );
    assert!(
        !dir.join("nvs.toml").exists(),
        "and nothing into the working directory"
    );
}

/// An anchor bundle that opens but holds no certificate stops the run before
/// the program starts, as `E0641` naming the file: an outbound client with no
/// anchor to trust could verify no server at all.
#[test]
fn an_anchor_bundle_with_no_certificate_stops_the_run_as_e0641() {
    let dir = scratch("tls-no-anchor");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    fs::write(dir.join("anchors.pem"), "not a certificate\n").unwrap();
    fs::write(
        dir.join("nvs.toml"),
        "[http.client.tls]\nroots = ['anchors.pem']\n",
    )
    .unwrap();

    let run = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("error[E0641]"), "{stderr}");
    assert!(
        stderr.contains("anchors.pem"),
        "the file is named: {stderr}"
    );
    assert!(run.stdout.is_empty(), "the program never ran");
}

/// The data folder's `nvs.toml` is read when the working directory has none,
/// and `./nvs.toml` wins when it is there. A broken data-folder file stops the
/// run and names that file, which is how the case tells which file was read.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_reads_the_data_folder_s_nvs_toml_when_the_working_directory_has_none() {
    let dir = scratch("data-lookup");
    let data = dir.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    fs::write(data.join("nvs.toml"), "this is [ not a configuration\n").unwrap();

    let from_data = run_with_data(&dir, &data, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&from_data.stderr);
    assert_eq!(
        from_data.status.code(),
        Some(1),
        "the data folder's broken file is read and stops the run: {stderr}"
    );
    assert!(stderr.contains("E0601"), "{stderr}");

    fs::write(dir.join("nvs.toml"), "").unwrap();
    let local = run_with_data(&dir, &data, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&local.stderr);
    assert_eq!(
        local.status.code(),
        Some(0),
        "`./nvs.toml` wins, so the data folder's file is not read: {stderr}"
    );
    assert_eq!(String::from_utf8_lossy(&local.stdout), "ran\n");
}

/// A relative `--data` resolves against the working directory, and
/// `--no-init` skips the file but still creates the folder.
// covers: tools:cli/nvs-run
#[test]
fn a_relative_data_folder_starts_at_the_working_directory_and_no_init_skips_only_the_file() {
    let dir = scratch("data-relative");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();

    let skipped = run_with_data(&dir, Path::new("relative"), &["--no-init", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&skipped.stderr);
    assert_eq!(skipped.status.code(), Some(0), "{stderr}");
    assert!(
        dir.join("relative").is_dir(),
        "the folder is created in the working directory"
    );
    assert!(
        !dir.join("relative").join("nvs.toml").exists(),
        "`--no-init` skips the file"
    );

    let written = run_with_data(&dir, Path::new("relative"), &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&written.stderr);
    assert_eq!(written.status.code(), Some(0), "{stderr}");
    assert!(dir.join("relative").join("nvs.toml").is_file());
    assert!(!dir.join("nvs.toml").exists());
}

/// A data folder that cannot be created prints one warning and the run goes
/// on, on the shipped defaults, writing nothing.
// covers: tools:cli/nvs-run
#[test]
fn an_unusable_data_folder_warns_once_and_the_run_succeeds() {
    let dir = scratch("data-unusable");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    // A file where a folder must be: nothing can be created under it.
    fs::write(dir.join("blocked"), "").unwrap();

    let out = run_with_data(&dir, &dir.join("blocked").join("data"), &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "the run goes on: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ran\n");
    assert_eq!(
        stderr
            .matches("warning: Novis cannot use its data folder")
            .count(),
        1,
        "exactly one warning: {stderr}"
    );
    assert!(stderr.contains("--data"), "it names the flag: {stderr}");
    assert!(!dir.join("nvs.toml").exists(), "nothing is written instead");
}

/// The body of a template an older version wrote: a block this version does not know.
const OLD_TEMPLATE: &str = "# an older template\n[control]\nsocket = \"nvs.sock\"\n";

/// `nvs config check` with the data folder `data`, from `dir`.
fn config_check(dir: &Path, data: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .args(["config", "check"])
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// The data folder's `nvs.toml` that an older version wrote and nobody changed is replaced with
/// this version's template before it is read, by a run and by `nvs config check` alike, so a block
/// that version removed stops neither. The same file with its body changed is the operator's: it
/// is read as it is, and its unknown block stops the run.
// covers: tools:cli/nvs-run
#[test]
fn an_unedited_older_template_in_the_data_folder_is_replaced_before_it_is_read() {
    let dir = scratch("refresh");
    let data = dir.join(".nvsdata");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    let first = run_in(&dir, &["main.nvs"]);
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    let file = data.join("nvs.toml");
    let current = nvs_config::data::config_template();
    assert_eq!(fs::read_to_string(&file).unwrap(), current);

    let old = nvs_config::data::mark("0.0.1", OLD_TEMPLATE);
    fs::write(&file, &old).unwrap();
    let out = run_in(&dir, &["main.nvs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ran\n");
    assert_eq!(fs::read_to_string(&file).unwrap(), current);

    fs::write(&file, &old).unwrap();
    let out = config_check(&dir, &data);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(fs::read_to_string(&file).unwrap(), current);

    let edited = old.replace("nvs.sock", "other.sock");
    fs::write(&file, &edited).unwrap();
    let out = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "an edited file is read: {stderr}"
    );
    assert!(stderr.contains("E0601"), "{stderr}");
    assert_eq!(fs::read_to_string(&file).unwrap(), edited);
}

/// A read-only `nvs.toml` that is an older template is read as it is: the run succeeds and says
/// nothing about the file.
// covers: tools:cli/nvs-run
#[test]
fn a_read_only_older_template_is_read_as_it_is_and_the_run_goes_on() {
    let dir = scratch("refresh-read-only");
    let data = dir.join(".nvsdata");
    fs::write(dir.join("main.nvs"), PROGRAM).unwrap();
    let first = run_in(&dir, &["main.nvs"]);
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    let file = data.join("nvs.toml");
    let old = nvs_config::data::mark("0.0.1", "# an older template that this version reads\n");
    fs::write(&file, &old).unwrap();
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&file, permissions.clone()).unwrap();

    let out = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ran\n");
    assert!(
        !stderr.contains("warning") && !stderr.contains("nvs.toml"),
        "nothing is said about the file: {stderr}"
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), old);

    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "the scratch folder's own file, made writable again so it can be deleted"
    )]
    permissions.set_readonly(false);
    fs::set_permissions(&file, permissions).unwrap();
}

/// A program that writes one log record and then one line of output.
const LOGGING: &str =
    "<?nvs\nCore\\Log::write(Core\\Log::Level::Warn, \"logged\");\necho \"ran\", \"\\n\";\n";

/// Runs [`LOGGING`] from `dir` with the data folder `data`. With `managed`,
/// `$NOTIFY_SOCKET` is set, which is how systemd starts a service; without it,
/// the variable is removed, which is how a terminal starts a command.
fn run_logging(dir: &Path, data: &Path, managed: bool) -> Output {
    fs::write(dir.join("main.nvs"), LOGGING).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(data)
        .args(["run", "main.nvs"])
        .current_dir(dir)
        .env_remove("NOVIS_NO_INIT")
        .env_remove("NOTIFY_SOCKET");
    if managed {
        command.env("NOTIFY_SOCKET", dir.join("no-manager.sock"));
    }
    command
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// A command run from a terminal keeps its log records off the data folder,
/// whatever the configuration leaves unset: no `[log] target` writes no
/// `logs/nvs.log`.
// covers: tools:cli/nvs-run, directive:log.target
#[test]
fn a_command_line_run_with_no_log_target_writes_no_log_file() {
    let dir = scratch("log-command-line");
    let data = dir.join(".nvsdata");
    let out = run_logging(&dir, &data, false);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("logged"),
        "the record is the program's output: {stdout}"
    );
    assert!(data.join("logs").is_dir(), "the folder is still created");
    assert!(!data.join("logs").join("nvs.log").exists());
}

/// A run a service manager started, with no `[log] target`, writes its records
/// to `logs/nvs.log` in the data folder. A target the configuration names wins,
/// `stderr` included, and an unusable data folder leaves the records where a
/// command line puts them.
// covers: tools:cli/nvs-run, directive:log.target
#[cfg(unix)]
#[test]
fn a_service_with_no_log_target_writes_to_the_data_folder() {
    let dir = scratch("log-service");
    let data = dir.join(".nvsdata");
    let out = run_logging(&dir, &data, true);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ran\n");
    let log = fs::read_to_string(data.join("logs").join("nvs.log")).expect("the service log");
    assert!(log.contains("logged"), "{log}");

    let named = scratch("log-service-named");
    fs::write(named.join("nvs.toml"), "[log]\ntarget = \"stderr\"\n").unwrap();
    let out = run_logging(&named, &named.join(".nvsdata"), true);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("logged"));
    assert!(!named.join(".nvsdata").join("logs").join("nvs.log").exists());

    let blocked = scratch("log-service-unusable");
    fs::write(blocked.join("blocked"), "").unwrap();
    let out = run_logging(&blocked, &blocked.join("blocked").join("data"), true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("warning: Novis cannot use its data folder"));
    assert!(String::from_utf8_lossy(&out.stdout).contains("logged"));
}

/// Every row of the *Exit status* table: `exit(n)` is `n`, `exit("message")`
/// prints the message and is `0`, an uncaught throwable is `1` with its
/// log record on standard error only, and a wrong command line is `2`.
// covers: tools:cli/nvs-run
#[test]
fn nvs_run_ends_with_the_status_its_reference_table_names() {
    let dir = scratch("exit-status");
    let cases = [
        (
            "exits.nvs",
            "<?nvs\necho \"bye\", \"\\n\";\nexit(3);\n",
            3,
            "bye\n",
        ),
        ("says.nvs", "<?nvs\nexit(\"done\");\n", 0, "done"),
        (
            "throws.nvs",
            "<?nvs\necho \"before\", \"\\n\";\nthrow new LogicError(\"boom\");\n",
            1,
            "before\n",
        ),
    ];
    for (file, source, status, stdout) in cases {
        fs::write(dir.join(file), source).unwrap();
        let out = run_in(&dir, &[file]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(status), "{file}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), stdout, "{file}");
        if file == "throws.nvs" {
            assert!(
                stderr.contains("LogicError") && stderr.contains("boom"),
                "the record names the class and the message on standard error: {stderr}"
            );
        }
    }

    let wrong = run_in(&dir, &["--no-such-flag", "exits.nvs"]);
    assert_eq!(wrong.status.code(), Some(2), "the command line was wrong");
    assert!(wrong.stdout.is_empty(), "nothing ran");
}

/// A process a `Core\Task` child task leaves running is stopped when that task
/// ends, and its `error` record on standard error says the task ended. The
/// task's end is not the program's, so the run still exits `0`, as
/// `rule:core-classes/process-spawn` says.
// covers: Core\Process::spawn
#[test]
fn a_child_a_task_left_running_is_reported_and_the_run_still_exits_0() {
    let dir = scratch("task-child-exit-status");
    let path = if cfg!(windows) {
        r"C:\\Windows\\System32\\sort.exe"
    } else {
        "/bin/cat"
    };
    fs::write(
        dir.join("main.nvs"),
        format!(
            "<?nvs\nvar $done = Core\\Task::all({{\n    started: fn (): int => {{\n        \
             Core\\Process\\Handle $child = Core\\Process::spawn(\"{path}\", []);\n        \
             return 1;\n    }},\n}});\necho \"tasks done: \", $done->started, \"\\n\";\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.process]\nexec = true\n",
    )
    .unwrap();

    let out = run_in(&dir, &["main.nvs"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "the task's report: {stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "tasks done: 1\n");
    let records: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains("Core\\\\Process::spawn"))
        .collect();
    assert_eq!(records.len(), 1, "one record for the one process: {stderr}");
    assert!(
        records[0].contains(r#""level":"error""#)
            && records[0].contains("when the task that started it ended"),
        "the record is an error that names the task: {stderr}"
    );
}

/// A program that starts PowerShell, which starts a second PowerShell on its
/// own and prints its own process id and then the second one's. Both keep
/// running for two minutes. `nvs` passes the line on and sleeps too.
///
/// `-NoNewWindow` makes `Start-Process` call `CreateProcess` with handle
/// inheritance, so the grandchild inherits every inheritable handle the child
/// has.
#[cfg(windows)]
const STARTS_A_CHILD_AND_A_GRANDCHILD: &str = r#"<?nvs
Core\Process\Handle $child = Core\Process::spawn(
    "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
    ["-NoProfile", "-Command", '$g = Start-Process -PassThru -NoNewWindow -FilePath powershell.exe -ArgumentList "-NoProfile -Command Start-Sleep 120"; "$PID $($g.Id)"; Start-Sleep 120']
);
?bytes $pids = $child->readStdout();
echo ($pids ?? "" as bytes) as string, "\n";
Core\Time::sleep(120s);
"#;

/// `--dump-asm` prints the machine code of every function the program compiles to, each under a
/// `; <name>` line, and does not run the program, as `docs/reference/tools/10-cli.md` §
/// *nvs run* says. The listing is the compile `run` performs, so each section ends in the
/// target's return instruction, and no section says the target has no disassembler.
// covers: tools:cli/nvs-run
#[test]
fn dump_asm_prints_a_listing_per_function_and_does_not_run_the_program() {
    let dir = scratch("dump-asm");
    fs::write(dir.join("main.nvs"), DUMPED).unwrap();

    let ran = run_in(&dir, &["main.nvs"]);
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "42\nhi there\n");

    let dumped = run_in(&dir, &["--dump-asm", "main.nvs"]);
    let stderr = String::from_utf8_lossy(&dumped.stderr);
    assert_eq!(dumped.status.code(), Some(0), "{stderr}");
    let listing = String::from_utf8_lossy(&dumped.stdout);
    for line in ["42", "hi there"] {
        assert!(
            !listing.lines().any(|printed| printed == line),
            "the program did not run, so `{line}` is not printed:\n{listing}"
        );
    }
    assert!(
        !listing.contains("no disassembly available"),
        "every CI target has a disassembler:\n{listing}"
    );

    let mut sections = std::collections::BTreeMap::<&str, Vec<&str>>::new();
    let mut current = None;
    for line in listing.lines() {
        if let Some(name) = line.strip_prefix("; ") {
            current = Some(name);
            sections.entry(name).or_default();
        } else if let (Some(name), false) = (current, line.trim().is_empty()) {
            sections.entry(name).or_default().push(line.trim());
        }
    }
    for function in ["Maths::double", "Maths::greet", "<script>"] {
        let body = sections
            .get(function)
            .unwrap_or_else(|| panic!("`{function}` has a section:\n{listing}"));
        assert!(
            body.iter()
                .any(|instruction| instruction.starts_with("ret")),
            "`{function}`'s section is its machine code, a return included:\n{listing}"
        );
    }
}

/// [`dump_asm_prints_a_listing_per_function_and_does_not_run_the_program`]'s program: two
/// methods and the script that calls them.
const DUMPED: &str = r#"<?nvs
class Maths {
    public static function double(int $n): int {
        return $n * 2;
    }
    public static function greet(string $who): string {
        return "hi " . $who;
    }
}
echo Maths::double(21), "\n";
echo Maths::greet("there"), "\n";
"#;

/// When `nvs` is killed, the process its program started ends with it, and what
/// that process started on its own does not. Neither of them holds `nvs`'s own
/// standard output. The kill is `TerminateProcess`, so no destructor in `nvs`
/// runs: only the operating system closing `nvs`'s handles can end the child.
///
/// Each half is its own assertion. A child that inherited `nvs`'s standard
/// output passes it on to the grandchild, which keeps running and keeps that
/// pipe open, so the read here does not end when `nvs` does. A child outside a
/// job that ends with `nvs` is still running after the kill. A grandchild
/// inside that job is not.
// covers: Core\Process::spawn
#[cfg(windows)]
#[test]
fn a_spawned_process_holds_none_of_nvs_s_handles_and_only_it_ends_when_nvs_is_killed() {
    use std::io::Read;
    use std::process::{Child, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// Ends `nvs` and every process the case saw, however the case ends. The
    /// grandchild is still running when the case passes, so this always has
    /// work to do.
    struct Leftovers {
        nvs: Child,
        pids: Vec<u32>,
    }
    impl Drop for Leftovers {
        fn drop(&mut self) {
            let _ = self.nvs.kill();
            let _ = self.nvs.wait();
            for pid in &self.pids {
                let _ = nvs_repo::spawn("taskkill", &[])
                    .args(["/F", "/PID", &pid.to_string()])
                    .output();
            }
        }
    }

    fn running(pid: u32) -> bool {
        let listed = nvs_repo::spawn("tasklist", &[])
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
            .expect("`tasklist` runs");
        String::from_utf8_lossy(&listed.stdout).contains(&format!("\"{pid}\""))
    }

    let dir = scratch("spawned-process-isolation");
    fs::write(dir.join("main.nvs"), STARTS_A_CHILD_AND_A_GRANDCHILD).unwrap();
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.process]\nexec = true\n",
    )
    .unwrap();
    let nvs = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(dir.join(".nvsdata"))
        .args(["run", "main.nvs"])
        .current_dir(&dir)
        .env_remove("NOVIS_NO_INIT")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the `nvs` binary this test was built beside runs");
    let mut left = Leftovers {
        nvs,
        pids: Vec::new(),
    };

    // Each chunk of `nvs`'s output, and `None` at its end.
    let mut stdout = left.nvs.stdout.take().expect("standard output is piped");
    let (tell, heard) = mpsc::channel::<Option<Vec<u8>>>();
    std::thread::spawn(move || {
        let mut chunk = [0_u8; 256];
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) | Err(_) => {
                    let _ = tell.send(None);
                    return;
                }
                Ok(read) => {
                    let _ = tell.send(Some(chunk[..read].to_vec()));
                }
            }
        }
    });

    let mut printed = Vec::new();
    while !printed.contains(&b'\n') {
        match heard.recv_timeout(Duration::from_secs(60)) {
            Ok(Some(chunk)) => printed.extend(chunk),
            other => panic!("`nvs` printed no line: {other:?}, {printed:?}"),
        }
    }
    let line = String::from_utf8_lossy(&printed);
    left.pids = line
        .split_whitespace()
        .map(|pid| {
            pid.parse()
                .unwrap_or_else(|_| panic!("the line is two process ids: {line:?}"))
        })
        .collect();
    let &[child, grandchild] = left.pids.as_slice() else {
        panic!("the line is two process ids: {line:?}");
    };
    assert!(running(child), "the child runs while `nvs` does");
    assert!(running(grandchild), "the grandchild runs while `nvs` does");

    left.nvs.kill().expect("`nvs` is still running");
    left.nvs.wait().expect("`nvs` is reaped");
    let killed = Instant::now();

    let ended = heard.recv_timeout(Duration::from_secs(20));
    assert!(
        matches!(ended, Ok(None)),
        "`nvs`'s standard output ends when `nvs` does, not when its child does: {ended:?}"
    );
    while running(child) {
        assert!(
            killed.elapsed() < Duration::from_secs(20),
            "the child ends with `nvs`"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    // A job ends all of its processes at once, so a grandchild in it would be
    // gone well within this.
    std::thread::sleep(Duration::from_secs(1));
    assert!(
        running(grandchild),
        "what the child started on its own keeps running after `nvs` ends"
    );
}

/// A program that starts a shell, which starts `sleep` in the background on its
/// own and prints its own process id and then the `sleep`'s. Both keep running
/// for two minutes. `nvs` passes the line on and sleeps too.
#[cfg(unix)]
const STARTS_A_SHELL_AND_A_SLEEP: &str = r#"<?nvs
Core\Process\Handle $child = Core\Process::spawn(
    "/bin/sh",
    ["-c", 'sleep 120 & echo "$$ $!"; wait']
);
?bytes $pids = $child->readStdout();
echo ($pids ?? "" as bytes) as string, "\n";
Core\Time::sleep(120s);
"#;

/// When `nvs` is killed, the process its program started ends with it, and what
/// that process started on its own does not. The kill is `SIGKILL`, so no
/// destructor in `nvs` runs: only the watcher `nvs` started beside its first
/// child can end the child. The watcher ignores the `SIGINT` a Ctrl-C sends
/// the terminal's process group, and exits once it has ended the child.
// covers: Core\Process::spawn
#[cfg(unix)]
#[test]
fn a_spawned_process_ends_when_nvs_is_killed_and_what_it_started_does_not() {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Stdio};
    use std::time::{Duration, Instant};

    /// Ends `nvs` and every process the case saw, however the case ends. The
    /// `sleep` is still running when the case passes, so this always has work
    /// to do.
    struct Leftovers {
        nvs: Child,
        pids: Vec<u32>,
    }
    impl Drop for Leftovers {
        fn drop(&mut self) {
            let _ = self.nvs.kill();
            let _ = self.nvs.wait();
            for pid in &self.pids {
                let _ = nvs_repo::spawn("kill", &[])
                    .args(["-KILL", &pid.to_string()])
                    .output();
            }
        }
    }

    /// Whether `pid` is a process that has not ended. A process that has ended
    /// and not been reaped yet is a zombie, state `Z`, and counts as ended.
    fn running(pid: u32) -> bool {
        let listed = nvs_repo::spawn("ps", &[])
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .expect("`ps` runs");
        let state = String::from_utf8_lossy(&listed.stdout);
        let state = state.trim();
        !state.is_empty() && !state.starts_with('Z')
    }

    /// The processes whose parent is `parent`.
    fn children_of(parent: u32) -> Vec<u32> {
        let listed = nvs_repo::spawn("ps", &[])
            .args(["-A", "-o", "pid=", "-o", "ppid="])
            .output()
            .expect("`ps` runs");
        String::from_utf8_lossy(&listed.stdout)
            .lines()
            .filter_map(|line| {
                let mut ids = line.split_whitespace().map(str::parse::<u32>);
                match (ids.next(), ids.next()) {
                    (Some(Ok(pid)), Some(Ok(ppid))) if ppid == parent => Some(pid),
                    _ => None,
                }
            })
            .collect()
    }

    /// Waits up to twenty seconds for `pid` to end.
    fn ends(pid: u32, what: &str) {
        let since = Instant::now();
        while running(pid) {
            assert!(since.elapsed() < Duration::from_secs(20), "{what}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    let dir = scratch("spawned-process-parent-death");
    fs::write(dir.join("main.nvs"), STARTS_A_SHELL_AND_A_SLEEP).unwrap();
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.process]\nexec = true\n",
    )
    .unwrap();
    let nvs = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(dir.join(".nvsdata"))
        .args(["run", "main.nvs"])
        .current_dir(&dir)
        .env_remove("NOVIS_NO_INIT")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the `nvs` binary this test was built beside runs");
    let mut left = Leftovers {
        nvs,
        pids: Vec::new(),
    };

    // The reader lives to the end of the case. Dropping it would close the
    // pipe, and `nvs` would end on a broken pipe at its next write, taking the
    // child and the watcher with it before they are looked at.
    let mut out = BufReader::new(left.nvs.stdout.take().expect("standard output is piped"));
    let mut line = String::new();
    out.read_line(&mut line).expect("`nvs` prints a line");
    left.pids = line
        .split_whitespace()
        .map(|pid| {
            pid.parse()
                .unwrap_or_else(|_| panic!("the line is two process ids: {line:?}"))
        })
        .collect();
    let &[child, grandchild] = left.pids.as_slice() else {
        panic!("the line is two process ids: {line:?}");
    };
    assert!(
        matches!(left.nvs.try_wait(), Ok(None)),
        "`nvs` still runs once it has printed the line"
    );
    let watchers: Vec<u32> = children_of(left.nvs.id())
        .into_iter()
        .filter(|&pid| pid != child)
        .collect();
    let &[watcher] = watchers.as_slice() else {
        panic!("`nvs` started one process beside its child: {watchers:?}");
    };
    left.pids.push(watcher);
    assert!(running(child), "the child runs while `nvs` does");
    assert!(running(grandchild), "the `sleep` runs while `nvs` does");

    let _ = nvs_repo::spawn("kill", &[])
        .args(["-INT", &watcher.to_string()])
        .output();
    std::thread::sleep(Duration::from_millis(500));
    assert!(running(watcher), "the watcher ignores `SIGINT`");

    left.nvs.kill().expect("`nvs` is still running");
    left.nvs.wait().expect("`nvs` is reaped");

    ends(child, "the child ends with `nvs`");
    ends(watcher, "the watcher exits once `nvs` is gone");
    // The watcher kills only the processes `nvs` started, so a `sleep` it
    // reached as well would be gone well within this.
    std::thread::sleep(Duration::from_secs(1));
    assert!(
        running(grandchild),
        "what the child started on its own keeps running after `nvs` ends"
    );
}
