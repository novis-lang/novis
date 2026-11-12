//! Every wait that outlives a request's `wall_time` says why, at a site listed
//! for that reason, only the one funnel suspends a task, and no member clamps
//! its own wait to the wall deadline.
//!
//! `rule:errors/on-limit` ends a request's wait at its wall deadline, and
//! `rule:concurrency/the-parking-contract` puts that bound in the one funnel
//! every park goes through. A wait is bounded unless it parks as
//! `Waiting::ParkedPast`, so the reason a wait outlives the deadline is the
//! one thing a reviewer has to check. These tests read the sources, so a new
//! reason, or a known reason in a new file, fails here and names its line.

use std::fs;
use std::path::{Path, PathBuf};

/// Each reason a wait may outlive `wall_time`, with the files under `crates/`
/// allowed to name it. A reason with no file yet has an empty list.
const REASONS: &[(&str, &[&str])] = &[
    (
        "ServerTask",
        &[
            "nvs-cli/src/serve.rs",
            "nvs-cli/src/serve/exporters.rs",
            "nvs-cli/src/worker.rs",
            "nvs-host/src/worker.rs",
            "nvs-server/src/schedule.rs",
            "nvs-server/src/serve.rs",
        ],
    ),
    (
        "ConnectionDrive",
        &["nvs-host/src/block_on.rs", "nvs-server/src/serve.rs"],
    ),
    (
        "Join",
        &[
            "nvs-host/src/group.rs",
            "nvs-host/src/isolate.rs",
            "nvs-host/src/worker.rs",
        ],
    ),
    ("BlockingJob", &["nvs-host/src/blocking.rs"]),
    ("DebugStop", &["nvs-host/src/debug.rs"]),
    ("RequestEnd", &[]),
];

/// The files that pass a reason through without choosing one: the enum's own
/// home, and the clock waits that take a reason as an argument.
const FORWARDERS: &[&str] = &["nvs-host/src/scheduler.rs", "nvs-host/src/timer.rs"];

/// The file that holds the funnel, and the funnel's name.
const FUNNEL_FILE: &str = "nvs-host/src/scheduler.rs";
const FUNNEL: &str = "fn yield_on(";

#[test]
fn every_wait_past_wall_time_names_a_reason_at_a_listed_site() {
    let crates = crates_dir();
    let mut offences = Vec::new();
    let mut used: Vec<(&str, String)> = Vec::new();

    for (file, lines) in code_of_every_crate(&crates) {
        for (index, (number, code)) in lines.iter().enumerate() {
            for reason in after_each(code, "PastWallTime::") {
                let name: String = reason
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                match REASONS.iter().find(|(known, _)| *known == name) {
                    Some((known, files)) if files.contains(&file.as_str()) => {
                        used.push((known, file.clone()));
                    }
                    Some(_) => offences.push(format!(
                        "crates/{file}:{number}: `PastWallTime::{name}` is not listed for this file"
                    )),
                    None => offences.push(format!(
                        "crates/{file}:{number}: `PastWallTime::{name}` is not a listed reason"
                    )),
                }
            }
            for argument in after_each(code, "ParkedPast(") {
                // The argument may start on the next line, as rustfmt breaks a
                // long call.
                let argument = if argument.trim().is_empty() {
                    lines.get(index + 1).map_or("", |(_, next)| next.as_str())
                } else {
                    argument
                };
                let named = argument.trim_start().starts_with("PastWallTime::")
                    || argument
                        .trim_start()
                        .starts_with("nvs_host::PastWallTime::");
                if !named && !FORWARDERS.contains(&file.as_str()) {
                    offences.push(format!(
                        "crates/{file}:{number}: `ParkedPast` takes a reason that is not written \
                         at the site"
                    ));
                }
            }
        }
    }

    for (reason, files) in REASONS {
        for file in *files {
            if !used.iter().any(|(r, f)| r == reason && f == file) {
                offences.push(format!(
                    "crates/{file}: listed for `PastWallTime::{reason}`, and no wait there names it"
                ));
            }
        }
    }

    assert!(
        offences.is_empty(),
        "a wait outlives `wall_time` at a site the table does not list:\n  {}\nA wait parks as \
         `Waiting::Parked` and is bounded by its tree's wall deadline unless it has a reason. Add \
         the site to `REASONS` only when its reason is the one the variant's doc states.",
        offences.join("\n  ")
    );
}

#[test]
fn only_the_funnel_suspends_a_task() {
    let crates = crates_dir();
    let mut offences = Vec::new();
    let mut in_funnel = 0;

    for (file, lines) in code_of_every_crate(&crates) {
        if file != FUNNEL_FILE {
            // The task's yielder type is private to the scheduler. A file
            // that names it could suspend a task past the funnel.
            for (number, code) in &lines {
                if code.contains("TaskYielder") {
                    offences.push(format!("crates/{file}:{number}: names the task's yielder"));
                }
            }
            continue;
        }
        // Inside the scheduler, every suspend of a yielder is in the funnel.
        let mut depth_at_funnel: Option<i64> = None;
        let mut depth = 0i64;
        for (number, code) in &lines {
            if code.contains(FUNNEL) {
                depth_at_funnel = Some(depth);
            }
            if code.contains(".suspend(") {
                if depth_at_funnel.is_some() {
                    in_funnel += 1;
                } else {
                    offences.push(format!(
                        "crates/{file}:{number}: suspends a yielder outside `yield_on`"
                    ));
                }
            }
            depth += braces(code);
            if depth_at_funnel.is_some_and(|start| depth <= start && !code.contains(FUNNEL)) {
                depth_at_funnel = None;
            }
        }
    }

    assert!(
        offences.is_empty(),
        "a task is suspended outside the funnel:\n  {}\n`yield_on` is where a park is bounded by \
         its tree's wall deadline, so a suspend anywhere else is an unbounded wait.",
        offences.join("\n  ")
    );
    assert_eq!(
        in_funnel, 1,
        "`yield_on` suspends the task's yielder exactly once"
    );
}

/// The names a member once used to clamp its own wait to the wall deadline.
/// The funnel is the one bound, so each is gone, and a second layer would hide
/// a missing first one (`rule:errors/on-limit`).
const HAND_CLAMPS: &[&str] = &[
    "within_wall_time(",
    "wall_clamp(",
    "wall_time_fault(",
    "wall_time_left(",
    "wall_bound(",
    "within_wall(",
    "wall_or(",
    "set_wall(",
    "send_by(",
];

#[test]
fn no_member_clamps_its_wait_to_the_wall_deadline_by_hand() {
    let crates = crates_dir();
    let mut offences = Vec::new();
    for (file, lines) in code_of_every_crate(&crates) {
        for (number, code) in &lines {
            for name in HAND_CLAMPS {
                let named = code.match_indices(name).any(|(at, _)| {
                    !code[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_')
                });
                if named {
                    offences.push(format!("crates/{file}:{number}: `{name}`"));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a member clamps its wait to the wall deadline by hand:\n  {}\nThe scheduler's funnel \
         ends every park at the wall deadline. Work off the core reads `off_core_deadline`.",
        offences.join("\n  ")
    );
}

/// The text after each match of `needle` in `code`.
fn after_each<'a>(code: &'a str, needle: &str) -> impl Iterator<Item = &'a str> {
    let needle_len = needle.len();
    code.match_indices(needle)
        .map(move |(at, _)| &code[at + needle_len..])
}

/// The net change in brace depth over one line of code.
fn braces(code: &str) -> i64 {
    code.chars().fold(0, |depth, c| match c {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

/// Every `.rs` file under each crate's `src`, as its path under `crates/` with
/// `/` separators, and its compiled lines numbered from 1. A line comment and
/// everything from a column-zero `#[cfg(test)]` on are left out, because a test
/// module parks however its case needs.
fn code_of_every_crate(crates: &Path) -> Vec<(String, Vec<(usize, String)>)> {
    let mut files = Vec::new();
    for krate in entries(crates) {
        let src = krate.join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let text = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("{} is read: {error}", file.display()));
            let lines = text
                .lines()
                .enumerate()
                .take_while(|(_, line)| !line.starts_with("#[cfg(test)]"))
                .filter(|(_, line)| !line.trim_start().starts_with("//"))
                .map(|(index, line)| (index + 1, line.to_owned()))
                .collect();
            let relative = file
                .strip_prefix(crates)
                .expect("a source is under crates/")
                .to_string_lossy()
                .replace('\\', "/");
            (relative, lines)
        })
        .collect()
}

fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in entries(dir) {
        if entry.is_dir() {
            sources(&entry, into);
        } else if entry.extension().is_some_and(|extension| extension == "rs") {
            into.push(entry);
        }
    }
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{} is listed: {error}", dir.display()))
        .map(|entry| entry.expect("a directory entry is read").path())
        .collect()
}

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate is under crates/")
        .to_path_buf()
}
