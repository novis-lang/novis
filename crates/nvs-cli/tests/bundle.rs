//! `nvs build --compile` and the executable it writes —
//! `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s
//! *Verification* list, for the claims that do not need three platforms to
//! ask.
//!
//! Through the built binary rather than by calling `bundle::build`, for the
//! reason [`openapi`](openapi) already writes down: `nvs-cli` is a binary crate
//! with no library target, and what `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` promises is what the *command*
//! produces. The round-trip of the manifest itself is a unit test next to the
//! writer (`src/bundle.rs`); this file asserts what only a real executable can
//! answer — that the payload is on disk in § 2's shape, and that
//! running it is running the program.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The two-file `require` graph the first two tests bundle.
const APP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/bundle/app.nvs");

/// The program whose file set is its `autoload` root: one class the prefix map
/// reaches, one only `Core\Program::implementing<T>()` does, and no `require`
/// at all.
const AUTOLOAD_APP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/bundle-autoload/app.nvs"
);

/// § 4's footer: `b"NVSB"`, a `u16` and two `u64`s.
const FOOTER_LEN: usize = 22;

/// Builds the fixture into a private directory and hands back the executable.
///
/// One directory per test rather than one shared: the tests here run
/// concurrently under `cargo test`, and a bundle half-written by one is not
/// something the other should ever be able to observe.
fn bundle(name: &str) -> PathBuf {
    bundle_of(APP, name)
}

/// [`bundle`] for any entry point, which is what the `autoload` half needs: the
/// claim there is about a *different program's* file set, not about a different
/// way of building the same one.
fn bundle_of(entry: &str, name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nvs-bundle-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a private directory under the temp dir");
    let exe = dir.join(if cfg!(windows) { "app.exe" } else { "app" });

    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["build", "--compile", entry, "-o"])
        .arg(&exe)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "the fixture compiles, so the bundle is written: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    exe
}

/// A length off the wire, as an index into bytes this host actually holds.
///
/// The footer's fields are `u64` because the format is the same on every
/// target; a value that does not fit this host's `usize` describes a file this
/// host could not have written, and saying so is better than wrapping into an
/// index that happens to be in range.
fn at_most_usize(n: u64) -> usize {
    usize::try_from(n).expect("a length this host wrote fits its own pointer width")
}

/// § 2's flat file list, read back out of a finished executable exactly as the
/// footer describes it: `(relative path, source)` per entry, in payload order.
///
/// Written here rather than shared with the writer on purpose — a reader that
/// is the writer's own code cannot fail to agree with it, and agreement is the
/// whole claim.
fn payload(exe: &Path) -> Vec<(String, String)> {
    let bytes = std::fs::read(exe).expect("the bundle was just written");
    let footer = &bytes[bytes.len() - FOOTER_LEN..];
    assert_eq!(&footer[0..4], b"NVSB", "§ 4's magic ends the file");
    assert_eq!(
        u16::from_le_bytes([footer[4], footer[5]]),
        1,
        "the format version this test knows"
    );
    let offset = at_most_usize(u64::from_le_bytes(
        footer[6..14].try_into().expect("eight bytes"),
    ));
    let len = at_most_usize(u64::from_le_bytes(
        footer[14..22].try_into().expect("eight bytes"),
    ));
    assert_eq!(
        offset + len + FOOTER_LEN,
        bytes.len(),
        "the manifest runs from its offset to the footer and no further"
    );

    let body = &bytes[offset..offset + len];
    let mut at = 4;
    let count = u32::from_le_bytes(body[0..4].try_into().expect("four bytes")) as usize;
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        let path_len = u32::from_le_bytes(body[at..at + 4].try_into().expect("four")) as usize;
        at += 4;
        let name = String::from_utf8(body[at..at + path_len].to_vec()).expect("UTF-8 path");
        at += path_len;
        let text_len = at_most_usize(u64::from_le_bytes(
            body[at..at + 8].try_into().expect("eight"),
        ));
        at += 8;
        let text = String::from_utf8(body[at..at + text_len].to_vec()).expect("UTF-8 source");
        at += text_len;
        files.push((name, text));
    }
    assert_eq!(at, body.len(), "the manifest is exactly its entries");
    files
}

/// §§ 2 and 3: the payload is the entry file **plus every file its `require`
/// graph statically resolved to**, as plain source, entry first.
///
/// Asserted by counting and by content rather than by looking for one name: a
/// bundler that embedded the entry alone still produces a file whose footer
/// parses, and the `require`d half is the half that would then fail on the
/// user's machine instead of at build time.
// covers: tools:cli/nvs-build-compile
#[test]
fn a_bundle_carries_the_statically_resolved_require_graph_as_source() {
    let files = payload(&bundle("graph"));

    assert_eq!(
        files.len(),
        2,
        "the entry and the one file it requires, and nothing else: {:?}",
        files.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    assert_eq!(files[0].0, "app.nvs", "the entry point is entry zero");
    assert_eq!(
        files[1].0, "lib/greet.nvs",
        "a required file keeps the relative layout it was written with, `/`-separated \
         whatever platform built it"
    );

    // Source, not artifacts — § 2's whole trade. Each file's own text is in
    // there byte for byte, which is also what makes a bundle inspectable.
    assert!(
        files[0].1.contains("require \"lib/greet.nvs\";"),
        "the entry's own source: {:?}",
        files[0].1
    );
    assert!(
        files[1].1.contains("class Greet"),
        "the required file's own source: {:?}",
        files[1].1
    );
    for (name, text) in &files {
        assert!(
            text.starts_with("<?nvs"),
            "{name} is source text and not a compiled artifact"
        );
    }
}

/// § 2's other half: what the `autoload` roots declare is frozen into the
/// payload at build time, a file no name reaches included
/// (`rule:programs/no-runtime-autoload`).
///
/// The claim is about the *root set* rather than about reachability, so the
/// file the assertion turns on is `modules/Spare.nvs`, which nothing in the
/// program names: a bundler collecting the files the graph walk loaded carries
/// `Widget` and drops it, and every other line of this program still passes.
#[test]
fn a_bundle_carries_every_file_the_autoload_roots_declare() {
    let files = payload(&bundle_of(AUTOLOAD_APP, "autoload"));

    assert_eq!(files[0].0, "app.nvs", "the entry point is entry zero");
    let mut names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "app.nvs",
            "modules/Part.nvs",
            "modules/Spare.nvs",
            "modules/Widget.nvs"
        ],
        "the entry and everything the one root declares, keeping the layout the roots are \
         written against"
    );
}

/// A bundled program resolves a name through an `autoload` root, and
/// enumerates that root, exactly as the source tree does — the machine running
/// it has no `modules/` directory at all.
///
/// The interpreted run is the oracle rather than a frozen string, for the
/// reason the `require` case below gives: identical means stdout, stderr and
/// status together.
#[test]
fn a_bundled_class_is_reached_and_enumerated_through_an_autoload_root() {
    let exe = bundle_of(AUTOLOAD_APP, "autoload-run");

    // The bundle carries its program inside itself, so running it opens nothing in the repository.
    let bundled = nvs_repo::spawn(&exe, &[])
        .output()
        .expect("the bundle is an executable this host can run");
    let interpreted = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["run", AUTOLOAD_APP])
        .output()
        .expect("the `nvs` binary this test was built beside runs");

    assert_eq!(
        String::from_utf8_lossy(&bundled.stdout),
        String::from_utf8_lossy(&interpreted.stdout),
        "the same program, so the same standard output"
    );
    assert_eq!(
        String::from_utf8_lossy(&bundled.stderr),
        String::from_utf8_lossy(&interpreted.stderr),
        "and nothing extra on standard error"
    );
    assert_eq!(
        bundled.status.code(),
        interpreted.status.code(),
        "and the same exit status"
    );

    let stdout = String::from_utf8_lossy(&bundled.stdout);
    assert!(
        stdout.contains("greeting from the autoload root"),
        "the prefix map reached a class inside the bundle: {stdout:?}"
    );
    assert!(
        stdout.contains("part spare"),
        "and the root's enumeration answered out of the payload rather than a directory \
         this host does not have: {stdout:?}"
    );
}

/// `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s *Verification*, first row: a bundled executable runs identically
/// to `nvs run` against the same source.
///
/// Identically means all three of stdout, stderr and exit status — a bundle
/// that printed the right answer while also complaining about a payload it
/// could not read would pass on stdout alone.
// covers: tools:cli/nvs-build-compile
#[test]
fn a_bundled_executable_runs_identically_to_nvs_run() {
    let exe = bundle("identical");

    // The bundle carries its program inside itself, so running it opens nothing in the repository.
    let bundled = nvs_repo::spawn(&exe, &[])
        .output()
        .expect("the bundle is an executable this host can run");
    let interpreted = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["run", APP])
        .output()
        .expect("the `nvs` binary this test was built beside runs");

    assert_eq!(
        String::from_utf8_lossy(&bundled.stdout),
        String::from_utf8_lossy(&interpreted.stdout),
        "the same program, so the same standard output"
    );
    assert_eq!(
        String::from_utf8_lossy(&bundled.stderr),
        String::from_utf8_lossy(&interpreted.stderr),
        "and nothing extra on standard error"
    );
    assert_eq!(
        bundled.status.code(),
        interpreted.status.code(),
        "and the same exit status"
    );
    assert!(
        String::from_utf8_lossy(&bundled.stdout).contains("greeting from the require graph"),
        "a positive control: both sides ran the program rather than both failing the same way"
    );
}

/// The attack written against the bundle: a program whose own commands are
/// `run` and `build`, bundled and started with the words `nvs` would act on.
/// Each reaches the program's command, and `-o out` writes no file, because
/// a bundle's whole command line belongs to the program.
// covers: tools:cli/nvs-build-compile
#[test]
fn a_bundle_hands_every_word_of_its_command_line_to_the_program() {
    let program = nvs_repo::path("tests")
        .join("hostile")
        .join("tools")
        .join("cli")
        .join("nvs-build-compile")
        .join("01-a-tool-whose-commands-are-named-like-nvs.nvs");
    let exe = bundle_of(
        program.to_str().expect("the repository path is UTF-8"),
        "argv",
    );
    // The directory `bundle_of` wrote the executable into.
    let dir = &std::env::temp_dir().join("nvs-bundle-argv");
    let started = |words: &[&str]| {
        // Run from the bundle's own directory, which has no `nvs.toml` to read.
        let out = nvs_repo::spawn(&exe, &[])
            .args(words)
            .current_dir(dir)
            .output()
            .expect("the bundle is an executable this host can run");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let run = started(&["run", "report.nvs"]);
    assert!(
        run.lines().any(|line| line == "tool run report.nvs"),
        "`run` reached the program's own command: {run:?}"
    );
    assert!(run.contains("status 0"), "and it succeeded: {run:?}");

    let build = started(&["build", "--compile", "x.nvs", "-o", "out"]);
    assert!(
        build
            .lines()
            .any(|line| line == "tool build x.nvs compile yes output out"),
        "`build --compile` reached the program with every option: {build:?}"
    );
    assert!(
        !dir.join("out").exists() && !dir.join("out.exe").exists(),
        "and nothing was built"
    );

    let help = started(&["--help"]);
    assert!(
        help.contains("status 2"),
        "`--help` is a word the program has no command for, so it prints its usage: {help:?}"
    );

    let _ = std::fs::remove_dir_all(dir);
}
