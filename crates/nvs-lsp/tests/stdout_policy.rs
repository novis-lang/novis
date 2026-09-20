//! stdout belongs to the protocol, and a lint alone cannot say so.
//!
//! `rule:ide/stdout-belongs-to-the-protocol` is a property of the *graph*, not
//! of one crate: `lsp_server::Connection::stdio()` hands this process's standard
//! output to a writer thread, so a single stray byte from anywhere beneath the
//! server splits a `Content-Length` frame and the client's next read lands
//! mid-message. The server then stops answering with no error printed anywhere,
//! which is the worst diagnostic a bug can have.
//!
//! `clippy::print_stdout` is warned workspace-wide and this crate takes no
//! allowance from it, but a lint covers the crate it is written in and
//! `nvs-cli` legitimately allows it — so the moment `nvs-lsp` depends on a
//! crate that also serves a terminal program, the lint stops being the answer.
//! This walks the dependency closure instead.
//!
//! **Scope, named rather than implied.** What is checked is this repository's
//! own crates, read off their manifests. The third-party half is covered
//! differently and cannot be covered this way: `lsp-server` and `lsp-types` are
//! the only two, `lsp-server` warns on `print_stdout` and `print_stderr` in its
//! own manifest, and `Cargo.lock` is what pins which versions those are.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The directory holding every crate of this repository, which is all this test reads.
fn crates_dir() -> PathBuf {
    nvs_repo::path("crates")
}

/// `file`, which is under [`crates_dir`], as it is written from the repository root.
fn from_the_root(crates: &Path, file: &Path) -> PathBuf {
    Path::new("crates").join(file.strip_prefix(crates).unwrap_or(file))
}

/// Read a file this test knows must exist, or fail naming it.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// The `nvs-*` crates named in `crate`'s `[dependencies]`.
///
/// `[dev-dependencies]` is deliberately not read: a dev-dependency is linked
/// into a test binary and never into the server, so including it would fail
/// this test over code the server cannot reach.
fn direct_dependencies(krate: &str) -> Vec<String> {
    let manifest = read(&crates_dir().join(krate).join("Cargo.toml"));
    let mut section = String::new();
    let mut named = Vec::new();

    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line.to_owned();
            continue;
        }
        if section != "[dependencies]" || line.starts_with('#') {
            continue;
        }
        // Both spellings this repository uses: `nvs-x.workspace = true` and
        // `nvs-x = { workspace = true }`.
        let Some(name) = line.split(['.', '=', ' ']).next() else {
            continue;
        };
        if name.starts_with("nvs-") {
            named.push(name.to_owned());
        }
    }

    named
}

/// Every crate of ours the language server links, `nvs-lsp` included.
fn linked_crates() -> BTreeSet<String> {
    let mut reached = BTreeSet::new();
    let mut pending = vec!["nvs-lsp".to_owned()];

    while let Some(krate) = pending.pop() {
        if !reached.insert(krate.clone()) {
            continue;
        }
        pending.extend(direct_dependencies(&krate));
    }

    reached
}

/// Every `.rs` file under `dir`, recursively.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));

    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }

    found
}

/// The ways a Rust file reaches this process's standard output.
///
/// `stdout()` rather than `io::stdout()`, so an imported spelling counts too.
const WRITERS: &[&str] = &["println!(", "print!(", "stdout()"];

/// The one crate in the closure that names `stdout()` legitimately.
///
/// `nvs-lsp` links the type checker, which links `nvs-stdlib` for the `Core`
/// signature registry, which links the runtime — so `nvs_runtime::Ctx`'s
/// `OutputSink::Stdout` is now under the server. That sink is how a program's
/// `echo` reaches a terminal under `nvs run`, and it writes only to a `Ctx` a
/// caller has explicitly wired to it. Nothing beneath the server wires one, and
/// [`nothing_under_the_server_wires_a_program_to_stdout`] is that half of the
/// claim; what is exempted here is the sink's own implementation, never the two
/// macros, which stay an offence in every crate including this one.
const THE_OUTPUT_SINK: &str = "nvs-runtime";

/// Whether `writer` is a hit this crate is allowed.
///
/// Only the bare `stdout()` handle, and only in the crate that implements the
/// sink: a `println!` there would still be a stray byte with no `Ctx` behind
/// it.
fn exempt(krate: &str, writer: &str) -> bool {
    krate == THE_OUTPUT_SINK && writer == "stdout()"
}

#[test]
fn no_crate_the_server_links_writes_to_stdout() {
    let crates = crates_dir();
    let mut offences = Vec::new();

    for krate in linked_crates() {
        for file in sources(&crates.join(&krate).join("src")) {
            for (number, line) in read(&file).lines().enumerate() {
                let code = line.trim_start();
                // A comment naming the thing is how this rule is explained, and
                // several of them do. Only what compiles counts.
                if code.starts_with("//") {
                    continue;
                }
                // `eprintln!(` ends in `println!(` and `eprint!(` in `print!(`,
                // and stderr is where logging is supposed to go — so the
                // stderr macros come out before the line is searched at all.
                let printing = code.replace("eprintln!(", "").replace("eprint!(", "");
                for writer in WRITERS {
                    if printing.contains(writer) && !exempt(&krate, writer) {
                        let path = from_the_root(&crates, &file);
                        offences.push(format!("{}:{}: {code}", path.display(), number + 1));
                    }
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "a crate the language server links writes to stdout:\n  {}\nstdout is the LSP framing \
         (`rule:ide/stdout-belongs-to-the-protocol`); anything printed there desynchronises the \
         stream and the server stops answering with nothing to read anywhere. Logging goes to \
         stderr and to `window/logMessage`.",
        offences.join("\n  ")
    );
}

/// The ways a caller asks for the runtime's standard-output sink.
const WIRINGS: &[&str] = &["OutputSink::Stdout", "Ctx::stdout("];

/// The other half of [`THE_OUTPUT_SINK`]'s exemption, and the reason that
/// exemption is not a hole.
///
/// The runtime may write to stdout because a `Ctx` was wired to it, so what has
/// to hold under the server is that nothing wires one. `nvs-lsp` cannot: it does
/// not name `nvs-runtime` in its own manifest, so `Ctx` is not a type it can
/// spell. What is left is the crates in between, and this is them.
#[test]
fn nothing_under_the_server_wires_a_program_to_stdout() {
    let crates = crates_dir();
    let mut offences = Vec::new();

    for krate in linked_crates() {
        if krate == THE_OUTPUT_SINK {
            continue;
        }
        for file in sources(&crates.join(&krate).join("src")) {
            for (number, line) in read(&file).lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for wiring in WIRINGS {
                    if code.contains(wiring) {
                        let path = from_the_root(&crates, &file);
                        offences.push(format!("{}:{}: {code}", path.display(), number + 1));
                    }
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "a crate the language server links wires an output sink to stdout:\n  {}\nthe runtime is \
         allowed its `stdout()` only because reaching it takes a `Ctx` a caller built on purpose \
         (`rule:ide/stdout-belongs-to-the-protocol`), and the server builds none. A crate that \
         wires one underneath it puts a program's `echo` into the LSP framing.",
        offences.join("\n  ")
    );
}
