//! The `nvs` binary.
//!
//! One subcommand per milestone that needed one:
//!
//! * `nvs ast` (M1) — dump what the parser produced, as Rust's `Debug` for a
//!   person or as `rule:ide/ast-json-schema-is-frozen`'s JSON for the AST
//!   panel that shells out to it. Resilient by default, since the file a panel
//!   is opened for is the one that does not compile; see [`ast`].
//! * `nvs check` (M2) — parse, resolve, type-check, report every diagnostic.
//!   `--autoload-map` prints the resolved `autoload` map in place of the
//!   success line, which is
//!   `rule:programs/autoload`'s last sentence; the shape is `nvs_hir::autoload`'s module doc.
//!   `--json` prints those same diagnostics as
//!   `rule:ide/check-json-is-the-diagnostic-record-as-a-document`'s document
//!   instead of rendering them for a terminal, for CI and for the agents that
//!   drive this compiler; see [`check`].
//! * `nvs run` (M3) — all of the above, then compile and execute. Its dump
//!   flags stop one stage earlier and print instead of running:
//!   `--dump-ir` after lowering, `--dump-asm` after code generation.
//! * `nvs test` (M4) — run a tree of `.nvst` conformance cases, or a program's
//!   own `#[Test]` methods. `rule:testing/nvst-is-separate` keeps the two formats apart and puts
//!   them under one subcommand; which is meant is read off the path. The
//!   `.nvst` format, and every decision behind it, is [`nvs_test`]'s own
//!   module doc, and this crate contributes only the argument parsing and the
//!   exit code; the `#[Test]` half is [`runner`].
//! * `nvs build --openapi` — the OpenAPI 3.1 document
//!   `rule:routing/api-document-is-generated-from-the-route-table`
//!   generates from the compile-time route table, on standard output. A build
//!   artifact and never a runtime feature; see [`openapi`] for what the table
//!   supplies and what it does not yet.
//! * `nvs build --compile` (M6) —
//!   `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s
//!   portable single-file executable: the program's `require` graph as plain
//!   source, appended to a copy of this binary. Also the one subcommand this
//!   binary can *be*: a copy carrying that payload runs it instead of parsing
//!   arguments at all; see [`bundle`].
//! * `nvs api diff <old.json> <new.json>` — the same ADR's § 4 gate over two
//!   finished documents: breaking, additive or cosmetic per change, non-zero
//!   exit on a breaking one. It compiles nothing, because the old side of a
//!   diff is a released artifact rather than a program; see [`api_diff`].
//! * `nvs config check` (M6) — resolve the configuration tree and report what
//!   it holds, exiting non-zero on any refusal.
//!   `rule:config/check-and-dump-audit-the-tree-offline`
//!   's offline audit, which exists because § 3's later-wins precedence is
//!   only safe while it is auditable. It compiles nothing and needs no server;
//!   see [`config`], which also holds the reader `run` resolves that tree
//!   through.
//! * `nvs info` — build, host and third-party licensing facts, `php -i` in
//!   shape and in purpose but not in spelling:
//!   `rule:packaging/the-cli-surface-is-novis-own` leaves it a subcommand and
//!   no short flag; see [`info`].
//! * `nvs meta --json [entry]` — the `Core` registry as JSON: every class,
//!   every member, and each documented member's reference card, which is
//!   `rule:tooling/meta-json`
//!   's contract. Given an entry point it prints that program's own
//!   declarations beside the registry
//!   (`rule:tooling/meta-json-takes-a-program`), which is the one input every
//!   documentation renderer reads. A build-time consumer's input, never a
//!   runtime feature; see [`meta`].
//! * `nvs doc <entry>` — that same document rendered as one Markdown page per
//!   class (`rule:tooling/nvs-doc-renders-and-decides-nothing`). A renderer
//!   with no source of truth of its own, shipped for a project that has no
//!   documentation pipeline; see [`doc`].
//! * `nvs tmp clean` —
//!   `rule:core-classes/temporary-dir-orphan-sweep`'s orphan sweep, run by hand: every entry under the owned temporary
//!   root whose owning process is gone, removed. Keyed on liveness and never on
//!   age, with `--dry-run` and deliberately no force flag; see [`tmp`], and
//!   [`serve`] for the other half of § 4, which is the same walk at boot.
//! * `nvs serve [<file>]` — one worker per core, every socket `[server] listen`
//!   names, every request running the entry file as its own isolate; see
//!   [`serve`].
//! * `nvs queue migrate` — `rule:core-classes/queue-storage-is-a-table`'s
//!   two tables, created by the operator's explicit command; see [`queue`],
//!   and [`worker`] for the in-process worker `[queue] workers` starts.
//! * `nvs schema plan|apply|dump` — `rule:core-classes/schema-converges`'s
//!   convergence as a command: the difference between a schema value and a live
//!   database, printed with every step's grade and SQL, applied, or read back
//!   out of the database; see [`schema`].
//! * `nvs service` — `rule:packaging/a-service-is-one-stored-argv`'s
//!   operator surface: the one argv a service manager stores; see [`service`].
//! * `nvs fmt <path>...` — the one canonical layout, written back into each
//!   file. `--check`, `--diff` and `--stdin` are I/O modes around the same
//!   call and no flag changes a byte of the output
//!   (`rule:tooling/fmt-is-one-canonical-style`); no other subcommand runs it
//!   and an unformatted file is never a diagnostic
//!   (`rule:tooling/fmt-is-never-a-diagnostic`); see [`fmt`].
//!
//! `run` **checks first**: on any diagnostic it reports and exits non-zero
//! exactly as `check` does, rather than running a program the front end
//! rejected. The rest of the architecture diagram
//! (`docs/implementation-plan.md` § Architecture) arrives with the milestones
//! that need it.
//!
//! ## What `run` executes
//!
//! The whole file, through `nvs_ir::lower::lower_file`: every class method
//! with a body, plus one synthesized frame for the file's own top-level
//! statements — `rule:statements/storage-that-outlives-a-call`'s
//! "the script body is a function, so its variables are locals". That frame is
//! the entry point; the methods are reachable from it by name.
//!
//! It runs **inside a task**, on a [`nvs_host::Scheduler`] of its own with a
//! reactor installed over it, rather than on the main thread's stack. That is
//! not about concurrency at the top level — there is one task — but about what
//! is beneath it: `rule:concurrency/a-child-belongs-to-the-calling-task`
//! 's children are children *of the calling task*, and a `Core\Task::all`
//! in a CLI program has nowhere to put them if the program is not one. The
//! root is `TaskRoot::Request`, so a panic that reaches it fails this run
//! rather than retiring anything
//! (`rule:http-server/containment-does-not-end-at-the-helper`
//! ).

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this crate's whole job is user-facing terminal output"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::builder::styling::{Effects, Style};
use clap::{Parser as ClapParser, Subcommand};
use nvs_diagnostics::{Diagnostics, Renderer, SourceMap};
use nvs_syntax::{check_declarations, parse_file};

mod agent;
mod api_diff;
mod ast;
mod bundle;
mod cache;
mod check;
mod config;
mod control;
mod coverage;
mod ctl;
#[cfg_attr(
    all(not(test), not(windows)),
    expect(
        dead_code,
        reason = "the status machine is a value on every platform so its cases run everywhere; \
                  only Windows has a manager to drive it"
    )
)]
mod dispatch;
mod doc;
mod events;
mod fmt;
mod info;
mod meta;
mod openapi;
mod peer;
mod queue;
mod runner;
mod schema;
mod script;
mod serve;
mod service;
mod stop;
#[cfg(test)]
mod testing;
mod tmp;
mod worker;

/// The two lines every `nvs --help` opens with: what this program is, then
/// exactly which build is answering.
///
/// The date is the commit's own rather than the build's, for the reason
/// `rule:packaging/a-build-records-no-timestamp` gives — two builds of one
/// commit print the same banner and stay byte-identical. The underline is an
/// `anstyle` sequence, which clap writes through `anstream`, so a redirected
/// or piped `--help` gets plain text and `NO_COLOR` is honoured without this
/// function testing for either.
fn header() -> String {
    let name = Style::new().effects(Effects::UNDERLINE);
    format!(
        "{}Novis{} — The Web-Native Programming Language\nversion: {} · commit: {} · {}",
        name.render(),
        name.render_reset(),
        env!("CARGO_PKG_VERSION"),
        env!("NVS_COMMIT"),
        env!("NVS_COMMIT_DATE"),
    )
}

/// What `nvs --version` prints after the name: the release, then the commit and
/// its date as [`header`] prints them, because a bug report and an upgrade check
/// both need the commit and `--version` is where people look for it.
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (commit ",
    env!("NVS_COMMIT"),
    ", ",
    env!("NVS_COMMIT_DATE"),
    ")"
);

/// The environment variables `nvs` reads, which `nvs --help` lists below the options.
const ENVIRONMENT: &str = "\
Environment:
  NOVIS_NO_INIT        Do not write a `nvs.toml` when there is none to read, as `--no-init` does.
  NOVIS_NO_FILE_CACHE  Read and write no compiled artifact, so every program is compiled again.
  NVS_FOOTPRINT_LOG    Append each `Core` class looked up, file read, directory listed and path
                       tested to this file, one line each. Used to record what a test run needs.";

#[derive(ClapParser)]
#[command(
    name = "nvs",
    version = VERSION,
    about = header(),
    after_long_help = ENVIRONMENT,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Reads this configuration file instead of `./nvs.toml`. Repeat the option
    /// to read several files, in order.
    ///
    /// When you name a file, `nvs` does not look for `./nvs.toml`. A relative
    /// path starts at the current folder. A file that does not exist is an
    /// error.
    ///
    /// Every subcommand accepts this option.
    // Every `///` line above is printed under each subcommand's `--help`, so it
    // is written as `AGENTS.md` § *Text an end user reads* asks. Naming any file
    // disables the search, so a named tree is never merged with whatever is in
    // the working directory. It is global because it selects the tree rather
    // than the command: `run` resolves the snapshot its request reads, and
    // `config check`/`config dump` audit the same files without running
    // anything.
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
    // step 1, and § 5 is the resolution against the working directory.
    #[arg(long, value_name = "PATH", global = true)]
    config: Vec<PathBuf>,

    /// Does not create `nvs.toml` when there is none.
    ///
    /// Without this option, a project command that finds no configuration file
    /// creates `nvs.toml` with the default settings in the current folder. With
    /// this option, the folder does not change, and the command uses the same
    /// default settings. Setting `NOVIS_NO_INIT` in the environment does the
    /// same.
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
    // step 3; see [`config::init_gate`].
    #[arg(long, global = true)]
    no_init: bool,
}

/// The project commands, which are the ones that write the shipped default file when they resolve
/// no tree: the commands that read a configuration **in order to execute something**.
///
/// Everything else leaves the directory alone. An audit that creates the file it is auditing
/// reports on its own output, and an `nvs lsp` that writes into every folder an editor opens is a
/// defect rather than a convenience. This table is the whole of that decision, so a sixth command
/// joins it here with a sentence rather than by threading a flag through an arm.
///
/// `nvs build` resolves no tree today and so reaches nothing to write; it is named because the
/// answer for it is decided, not because it currently does anything.
fn initializes(command: &Command) -> bool {
    matches!(
        command,
        Command::Run { .. }
            | Command::Serve { .. }
            | Command::Test { .. }
            | Command::Build { .. }
            | Command::Check { .. }
    )
}

#[derive(Subcommand)]
enum Command {
    /// Parse a Novis file and print its AST.
    Ast {
        /// The file to parse.
        file: PathBuf,
        /// Print the frozen JSON schema rather than the `Debug` rendering.
        #[arg(long)]
        json: bool,
        /// Print the tree the parser recovered into. The default, and
        /// spelled out so a caller can pin it.
        #[arg(long, conflicts_with = "strict")]
        resilient: bool,
        /// Print no tree at all once an error is reported.
        #[arg(long)]
        strict: bool,
    },
    /// Parse, resolve and type-check a Novis file, reporting every diagnostic
    /// found.
    Check {
        /// The file to check.
        file: PathBuf,
        /// Print the diagnostics as the frozen JSON document rather than
        /// rendering them for a terminal.
        // Refused beside `--autoload-map` rather than one of them winning:
        // both write to standard output, and a document with a map printed
        // after it is not a document.
        #[arg(long, conflicts_with = "autoload_map")]
        json: bool,
        /// Print the resolved `autoload` map instead of `no errors`,
        /// including what a `discover` glob skipped and what was shadowed.
        #[arg(long)]
        autoload_map: bool,
        /// Also report every public member with no `///` doc comment above it.
        /// Off by default, in every project.
        #[arg(long)]
        strict_docs: bool,
    },
    /// Check a Novis file, then compile and run it.
    Run {
        /// The file to run.
        file: PathBuf,
        /// Print the lowered IR instead of compiling it.
        #[arg(long)]
        dump_ir: bool,
        /// Print the generated machine code instead of running it.
        #[arg(long, conflicts_with = "dump_ir")]
        dump_asm: bool,
        /// Provoke an engine failure at a named site, for testing containment.
        ///
        /// Scoped to `nvs run` and nothing else: a contained engine panic has
        /// no user-facing trigger by definition, so it needs a hook to be
        /// testable at all — and that hook must never be reachable from a
        /// served request, so `nvs serve` has none.
        // `nvs_runtime::FaultSite` documents each site.
        #[arg(long, value_name = "SITE")]
        fault_inject: Option<FaultSiteArg>,
        /// Count what the program did — statements executed, calls made,
        /// allocations, bytes — and print the four totals on stderr at exit.
        ///
        /// The counts are the same on every machine for the same program and
        /// binary, which is what `bun nv proofs --record-perf` keeps them
        /// for (`rule:testing/bench-counters`). Stdout is untouched, so a
        /// program's own output is what it always was.
        #[arg(long, conflicts_with_all = ["dump_ir", "dump_asm"])]
        count: bool,
        /// Answer the request this file describes, rather than run as a
        /// program that is answering none.
        ///
        /// A flag and never an environment variable: whether a program is
        /// answering a request decides what every `Core\Request` member does,
        /// and a variable would make that a semantic change nothing at the
        /// call site shows. `nvs-test` writes one of these files per `.nvst`
        /// case that describes a request and owns the format
        /// (`nvs_test::request`); written by hand, it is how a request is
        /// reproduced without standing a listener up in front of it.
        #[arg(long, value_name = "FILE")]
        request: Option<PathBuf>,
        /// Run as a WebSocket connection whose peer sends the frames this
        /// file lists, and print every frame the program sends.
        ///
        /// What makes `Core\Socket::current()` answer outside a server, so a
        /// connection script can be run, shown and attacked from one file.
        /// The format is `peer`'s module doc. Given beside `--request`, the
        /// program is the request and the peer is the client of the
        /// connection its `Core\Socket::upgrade` opens, which runs once the
        /// request has ended.
        #[arg(long, value_name = "FILE")]
        peer: Option<PathBuf>,
        /// Run as an event-stream connection, and publish the values this
        /// file lists to the topics the program subscribes to.
        ///
        /// What makes `Core\Sse::current()` and `receive()` answer outside a
        /// server: the program is the script a `Core\Sse::upgrade` opens, and
        /// the stream it writes is printed as a client reads it. Beside a
        /// `--request`, the file feeds the stream that request's
        /// `Core\Sse::upgrade` opens instead. The format is `events`'s module
        /// doc.
        #[arg(long, value_name = "FILE", conflicts_with = "peer")]
        events: Option<PathBuf>,
        /// The program's own arguments, which `Core\Command::run()` matches
        /// against the program's compiled command table.
        ///
        /// Everything past the file is the program's and nothing here reads it,
        /// which is why it is `trailing_var_arg`: a command declaring a
        /// `--verbose` of its own must not be answered by `nvs run`.
        ///
        /// **One word is the exception, and it is the conventional one:** a
        /// leading `--` is this parser's escape token and is spent here rather
        /// than passed on, exactly as `cargo run --` spends one. It is spent
        /// once and only in that position — a `--` anywhere later is an
        /// ordinary word, and a program that wants one first is started with
        /// two.
        // `rule:tooling/commands-are-compiled` is the command line this hands
        // over. Both halves of the `--` rule are pinned by
        // `tests/conformance/core/cli-arguments-hands-back-a-word-that-looks-like-syntax-as-the-value-it-is.nvst`,
        // since `Core\Cli::arguments` is where the difference is observable.
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "ARGS"
        )]
        arguments: Vec<String>,
    },
    /// Run a Novis application until stopped: HTTP requests on every core, the
    /// `[[schedule]]` entries on the core that ticks, and that core's `[queue]`
    /// workers draining jobs beside them.
    ///
    /// One process is the whole deployment — one unit to install and one thing
    /// to supervise — and stopping it drains all three: no request, no fire and
    /// no claimed job is abandoned, and nothing new is taken. A tree that writes
    /// no `[[schedule]]` spawns no ticker and one that writes no `[queue]`
    /// starts no worker, so each costs a read at boot and no task at all.
    ///
    /// The development server and the proxied origin are the same command.
    /// Every file it serves is compiled before the socket is bound, and every
    /// request runs one of them in an isolate that shares nothing with any
    /// other request.
    ///
    /// `nvs serve app.nvs` serves that one file. `nvs serve` with no file
    /// serves every file that the `[[server.mount]]` blocks in the
    /// configuration mount.
    // `rule:concurrency/one-process-serves-requests-schedules-and-jobs` is why
    // the first paragraph names three subsystems: what an operator reads off
    // `--help` is the mental model they form of the process, and one naming the
    // accept loop alone is what sends them looking for a second thing to
    // install. `rule:http-server/two-deployments-and-nothing-a-proxy-owns` and
    // `rule:security/isolate-shares-nothing`. § 4's mount table is the set of
    // entry points, and `serve`'s module doc owns why the argument is
    // optional and why one path on the command line is § 2's rule rather than
    // an exception to it.
    Serve {
        /// The file every request runs. It is optional when the configuration
        /// has `[[server.mount]]` blocks: the server then serves every file
        /// that those blocks mount.
        file: Option<PathBuf>,
        /// The address to listen on, as `host:port` — the last word over
        /// `[server] listen`.
        // `rule:http-server/the-server-block-is-boot-class`.
        #[arg(long, value_name = "ADDR")]
        listen: Option<String>,
        /// The port to listen on, keeping the host `[server] listen` chose.
        ///
        /// Conflicts with `--listen` rather than being merged into it: two
        /// spellings of one address are two requests, and guessing which was
        /// meant is worse than saying so.
        #[arg(long, conflicts_with = "listen", value_name = "PORT")]
        port: Option<u16>,
    },
    /// Run a program's `#[Test]` methods, or a tree of `.nvst` conformance
    /// cases.
    ///
    /// Which one runs is read off the path. A `.nvs` file is a program, and
    /// its `#[Test]` methods run. A directory holding `.nvs` files is one
    /// program made of every `.nvs` file under it, in name order: put one file
    /// in the directory that requires your application's bootstrap file, and
    /// every test file in it sees the classes that file loads. Anything else
    /// is a `.nvst` case file or a directory walked for `*.nvst`. A directory
    /// holding both kinds of file is refused, and the two kinds never run in
    /// one invocation: they report differently and share no summary.
    ///
    /// Exits non-zero if any case or any test failed; a skipped one is not a
    /// failure.
    // `rule:testing/nvst-is-separate` § 1, and
    // `rule:testing/a-directory-of-programs-is-one-test-program` for the directory.
    Test {
        /// The program or directory of programs to run, or the case files and
        /// directories.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Run only the tests whose name contains this text — a `.nvst` case's
        /// path, or a `#[Test]` method's `Class::method`.
        #[arg(long, value_name = "TEXT")]
        filter: Option<String>,
        /// The PHP binary a `--ORACLE--` case is compared against.
        #[arg(long, value_name = "PATH", default_value = "php")]
        php: PathBuf,
        /// How many `.nvst` cases run at once; the default is this machine's
        /// hardware threads. A program's `#[Test]` methods are one process and
        /// are not spread.
        #[arg(long, value_name = "N")]
        jobs: Option<std::num::NonZeroUsize>,
        /// How a program's `#[Test]` run is reported.
        ///
        /// The default is the human format, and it is what a `.nvst` tree is
        /// always reported in: a conformance run's report is a summary rather
        /// than a suite of test methods, and the two share nothing — so naming
        /// a machine format beside a case tree is refused rather than silently
        /// ignored.
        // `rule:testing/report-formats`, and § 23 is what keeps the two reports
        // from sharing a shape.
        #[arg(long, value_name = "FORMAT", default_value = "human")]
        format: runner::Format,
        /// Rewrite each failed `Core\Test::assertMatchesInline` snapshot in
        /// the source that wrote it.
        ///
        /// This is the only spelling under which `nvs test` writes to a source
        /// file, and what it writes is the `$expected` literal and nothing
        /// else: a run without it never touches the program, and a run with it
        /// never touches a passing snapshot. A `.nvst` tree has no snapshot to
        /// update, so naming it there is refused rather than ignored.
        // `rule:testing/inline-snapshots`.
        #[arg(long)]
        update: bool,
        /// Write the run's line coverage to this file in the lcov format.
        ///
        /// Every line a statement starts on is listed with the number of
        /// times it ran, `0` for a line no test reached. A `.nvst` tree has no
        /// coverage to report, so naming it there is refused.
        // `rule:testing/debug-probes`; `crate::coverage` owns how the counts are taken.
        #[arg(long, value_name = "FILE", conflicts_with = "list")]
        coverage_lcov: Option<PathBuf>,
        /// Write the run's line coverage to this file as Clover XML.
        ///
        /// The same lines and counts as `--coverage-lcov`, in the format
        /// PHPUnit's `--coverage-clover` writes. Both can be given in one run.
        #[arg(long, value_name = "FILE", conflicts_with = "list")]
        coverage_clover: Option<PathBuf>,
        /// Report which `#[Test]` methods the program declares, and where each
        /// one is written, without running any of them.
        ///
        /// This is what an editor's Test Explorer populates its tree from, and
        /// it is answered from the table the compile already built — so a
        /// program whose tests fail, hang or `exit` lists exactly as a passing
        /// one does. It excludes `--update`, which rewrites what a run
        /// produced, and a `.nvst` tree, which is discovered by walking a
        /// directory rather than by compiling a program.
        // `rule:testing/report-formats`, and `rule:ide/every-feature-is-staged-behind-its-dependency`
        // is why the CLI surface lands before the client that reads it.
        #[arg(long, conflicts_with = "update")]
        list: bool,
        /// Run only the `.nvst` case files this file lists, one path per
        /// line, out of the trees named.
        ///
        /// A case is judged and counted exactly as a whole-tree run judges it,
        /// and the summary counts the listed cases. A listed path that no named
        /// tree holds is refused before anything runs.
        #[arg(long, value_name = "FILE")]
        cases: Option<PathBuf>,
        /// Record each `.nvst` case into this directory, which is created
        /// when it is not there.
        ///
        /// Every `nvs` process a case starts writes its coverage counters to
        /// `<name>-<pid>.profraw` when the binary is built with coverage, and
        /// the classes and files it used to `<name>.log`, and compiles with
        /// no artifact cache. `<name>` is the case's path with `/` and `\`
        /// as `~` and every other character that is not a letter, a digit,
        /// `.`, `_` or `-` as `@` and two hex digits.
        #[arg(long, value_name = "DIR")]
        record: Option<PathBuf>,
    },
    /// Produce a build artifact from a checked program.
    ///
    /// Naming an artifact is required rather than defaulted: `nvs build` with
    /// nothing named would be a subcommand that succeeds having done nothing,
    /// and the group is how the next artifact joins without changing what this
    /// invocation means.
    // `rule:routing/api-document-is-a-deterministic-build-artifact` spells the
    // OpenAPI command, and
    // `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
    // the bundle.
    #[command(group = clap::ArgGroup::new("artifact").required(true).args(["openapi", "compile"]))]
    Build {
        /// The entry point of the program to build.
        file: PathBuf,
        /// Write the OpenAPI 3.1 document generated from this program's route
        /// table to standard output.
        // `rule:routing/api-document-is-generated-from-the-route-table`.
        #[arg(long)]
        openapi: bool,
        /// Write a portable single-file executable: this program's `require`
        /// graph as source, appended to a copy of the `nvs` host binary.
        // `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`.
        #[arg(long)]
        compile: bool,
        /// Where to write the executable (default: the entry file's stem).
        #[arg(short = 'o', long, value_name = "PATH", requires = "compile")]
        output: Option<PathBuf>,
    },
    /// Work with the API document `nvs build --openapi` produces.
    ///
    /// A group rather than a flag on `build`, because `diff` reads two finished
    /// documents and compiles nothing: the old side of a diff is the last
    /// release's artifact, and there may be no source for it on this machine at
    /// all.
    // `rule:routing/api-diff-fails-a-breaking-change`.
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
    /// Write the shipped default `nvs.toml` into the working directory, or
    /// to the one path `--config` names.
    ///
    /// The directory the file goes into must already exist. It and the
    /// directory containing it must both pass the ownership check, because
    /// this is a file `nvs serve` will read as configuration.
    ///
    /// The explicit door to the file a project command writes for itself when
    /// it finds none, and where every refusal of that implicit write points: a
    /// directory the ownership check rejects, or one an operator asked to be
    /// left alone with `--no-init`. An existing file is never overwritten, and
    /// asking for one that cannot be written is an error here rather than the
    /// note it is on a run.
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
    // step 3; see [`config::init`].
    Init,
    /// Audit the configuration tree without running anything.
    ///
    /// A namespace of its own rather than part of `nvs check`, which checks
    /// *source*: a configuration tree and a program are audited separately.
    // `rule:config/check-and-dump-audit-the-tree-offline` separates the two;
    // see [`config`].
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Build and inspect the durable job queue's own tables.
    ///
    /// A namespace of the operator's rather than the program's: the runtime
    /// holds the queue's schema, and the tables are created by an explicit
    /// command — never at boot and never from a request.
    // `rule:core-classes/queue-storage-is-a-table`; see [`queue`].
    Queue {
        #[command(subcommand)]
        command: QueueCommand,
    },
    /// Converge a database on a schema value, or read the one it already holds.
    ///
    /// The operator's spelling of `Core\Db\Schema`: the difference against the
    /// live server is computed every time, so there is no migration to order
    /// and no history to keep.
    // `rule:core-classes/schema-converges`; see [`schema`].
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Clear what a hard-killed script left in the temporary root.
    ///
    /// The runtime deletes a temporary directory when its script ends and
    /// reclaims the rest at `nvs serve` boot; this is how a machine that never
    /// boots a server clears them.
    // The operator's half of `rule:core-classes/temporary-dir-orphan-sweep`;
    // see [`tmp`].
    Tmp {
        #[command(subcommand)]
        command: TmpCommand,
    },
    /// Drive a running server over its control socket: reload it, read what it
    /// is serving, or ask what it is doing.
    ///
    /// A namespace of its own because every other subcommand acts on files with
    /// no server involved, and these three exist only where a long-running
    /// process does.
    // `rule:config/one-local-control-socket`'s client; see [`ctl`], whose module
    // doc owns the drive and why an answer from another build is refused unread.
    Ctl {
        /// The endpoint to reach, which is how one of several servers on a host
        /// is addressed.
        ///
        /// Read without resolving a tree at all, so a server whose configuration
        /// has moved is still reachable by the name it is listening on. Absent,
        /// the tree's `[control] socket` says where to look.
        #[arg(long, value_name = "PATH", global = true)]
        socket: Option<PathBuf>,
        #[command(subcommand)]
        command: CtlCommand,
    },
    /// Installs, starts, stops and removes a Novis program that runs as a
    /// system service.
    ///
    /// A system service runs in the background, with no terminal. It can start
    /// when the computer starts.
    // A namespace of its own because every other subcommand acts on files with
    // no server involved, and these do not.
    // `rule:packaging/a-service-is-one-stored-argv`, matching `nvs ctl`'s
    // precedent; see [`service`], whose module doc owns how a verb reaches this
    // machine's own service manager.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    /// Another name for `nvs service install`.
    // The syntax `mysqld --install` and `httpd -k install` made familiar.
    // Hidden: § 1 names it an accepted alias rather than a second way to write
    // the command, and a help listing both would be advertising two.
    #[command(hide = true)]
    InstallService(ServiceInstall),
    /// Rewrite Novis files into the one canonical layout.
    ///
    /// Takes no configuration and has no style flag: the output is a pure
    /// function of the input bytes, so two projects run through this tool
    /// cannot disagree about what formatted means. The flags below are I/O
    /// modes, and each one decides only where the text goes.
    ///
    /// A file whose parse reports an error is refused rather than rewritten,
    /// named on standard error, and left exactly as it was.
    // `rule:tooling/fmt-is-one-canonical-style` and
    // `rule:tooling/fmt-check-writes-nothing`; see [`fmt`].
    Fmt {
        /// The files and directories to format. A directory is walked for the
        /// `.nvs` files under it.
        #[arg(required_unless_present = "stdin")]
        paths: Vec<PathBuf>,
        /// Write nothing and name each file that would change, exiting
        /// non-zero if any would.
        #[arg(long, alias = "dry-run", conflicts_with_all = ["diff", "stdin"])]
        check: bool,
        /// Write nothing and print a unified diff per file that would change,
        /// exiting non-zero if any would.
        // Refused beside `--check` for the reason `Check`'s `--json` is
        // refused beside `--autoload-map`: both render the same list onto the
        // same standard output, and one printed after the other is neither.
        #[arg(long, conflicts_with = "stdin")]
        diff: bool,
        /// Format what standard input holds onto standard output, naming no
        /// file. A refusal writes nothing at all to standard output.
        #[arg(long, conflicts_with = "paths")]
        stdin: bool,
    },
    /// Speak the Language Server Protocol on standard input and output.
    ///
    /// Started by an editor, not by a person: it reads LSP 3.17 frames on
    /// stdin and writes them on stdout, so a terminal that runs it sees
    /// nothing and appears to hang. It is the one implementation of Novis's
    /// language smarts, and every editor client is a shell around it.
    ///
    /// Configures nothing. Everything configurable arrives in `initialize`
    /// and in `workspace/didChangeConfiguration`, because the editor is what
    /// owns the settings and a flag here would be a second, staler copy.
    ///
    /// `--stdio` is the one exception, and it configures nothing either: it is
    /// the convention a language client appends to say which transport it
    /// chose, and stdio is the only transport this server has. Clients append
    /// it without being asked — `vscode-languageclient` does so for any
    /// `Executable` whose `transport` is set — and a server that rejected it
    /// would exit before reading a frame, which reaches the user as an editor
    /// with no language features rather than as an argument error.
    // `rule:ide/one-server-two-thin-clients`.
    Lsp {
        /// Accepted and ignored, so a client that names its transport is not
        /// refused for it.
        #[arg(long)]
        stdio: bool,
    },
    /// Run a tree of `.lspt` cases against the language server.
    ///
    /// The editor-behaviour suite: each case is a document, a cursor, a request
    /// and the answer frozen as text. It answers a different question from `nvs
    /// test` and shares no summary with it — two suites, two counts, because a
    /// number that meant both would mean neither.
    ///
    /// Prints `N passed, M failed` and exits non-zero if any case failed.
    // `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
    LspTest {
        /// The case files and directories to run.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Print the request × construct matrix instead of the summary line.
        ///
        /// Coverage is inferred from the node each case's question landed on
        /// and never declared by the case, so the matrix is a reading of the
        /// corpus rather than a list anyone maintains.
        // `rule:ide/lspt-coverage-is-inferred`.
        #[arg(long)]
        coverage: bool,
    },
    /// Print build, host and third-party licensing information.
    ///
    /// One call answers what this binary is and what is compiled into it,
    /// including the complete third-party attribution Novis's MIT license and
    /// its dependencies' licenses both require to be distributed with it.
    // `docs/decisions/0065.md` is why this is one call rather than several.
    Info {
        /// Also print every third-party license text in full.
        #[arg(long)]
        licenses: bool,
    },
    /// Print the `Core` registry — every class and member, with each
    /// documented member's reference card — and, given an entry point, that
    /// program's own declarations beside it.
    ///
    /// `--json` is required for the reason `build --openapi` is: a `meta`
    /// with nothing named would succeed having printed nothing, and the flag
    /// is how a second format joins without changing what this one means.
    // The shape is `rule:tooling/meta-json`'s and this command owns it; see
    // [`meta`].
    Meta {
        /// Write the registry as JSON to standard output.
        #[arg(long, required = true)]
        json: bool,
        /// A program's entry point, whose own declarations join the registry
        /// under a `program` key. Omitted, the document is the registry alone.
        // `rule:tooling/meta-json-takes-a-program`: without an entry the
        // document is byte-identical to what it was before this argument
        // existed.
        entry: Option<PathBuf>,
    },
    /// Write this program's declarations as one Markdown page per class.
    ///
    /// A renderer over `nvs meta --json`'s document and nothing else, for a
    /// project that does not have a documentation pipeline of its own.
    // `rule:tooling/nvs-doc-renders-and-decides-nothing`; see [`doc`].
    Doc {
        /// The program's entry point.
        file: PathBuf,
        /// The directory the pages are written to, created if it is absent.
        #[arg(long, default_value = "doc")]
        out: PathBuf,
    },
    /// Answer a coding agent from the registry this binary carries.
    ///
    /// A verb of its own rather than an argument to `nvs doc`, which takes a
    /// path: a symbol is not a path, and a query form under it would be
    /// ambiguous at the argument parser before it was ambiguous to a reader.
    // `rule:tooling/an-agent-asks-the-binary`; see [`agent`].
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Write one declaration file per `Core` class, enum and attribute.
    ///
    /// An editor opens these files when you jump to a `Core` name. Each file
    /// has the real signatures and the reference cards, and empty bodies. The
    /// language server writes the same files by itself; this command writes
    /// them for any other reader.
    // `rule:ide/the-stub-tree-is-where-core-is-declared`; the generator is
    // `nvs_lsp::stubs`, and this is its second caller.
    Stubs {
        /// The directory the files are written to, created if it is absent.
        #[arg(long, default_value = "stubs")]
        out: PathBuf,
    },
}

/// `nvs agent`'s own verbs — the read half of
/// `rule:tooling/an-agent-asks-the-binary`'s surface.
///
/// Each renders `nvs meta --json`'s document and holds nothing, so the binary
/// that compiles a program is the binary that answers for it.
#[derive(Subcommand, Debug)]
enum AgentCommand {
    /// Print the short document that makes a coding agent productive: the
    /// lookup protocol, one worked program, the capability model, the refusals
    /// and the chapter map.
    ///
    /// It is generated from the reference chapters this binary embeds, so it
    /// describes the language this binary compiles and nothing else.
    // `rule:tooling/a-primer-claim-is-executed`.
    Primer,
    /// Print one line per registered member, one per enum, exception and
    /// attribute beside them, one per chapter of the reference and per heading
    /// in it, and one per configuration key, command, flag and error code.
    // `rule:tooling/the-index-is-one-line-per-member`,
    // `rule:tooling/the-index-names-every-key-command-and-code`.
    Index {
        /// Print the lines as one JSON document: an `entries` array with one
        /// record for each line, which has the line's symbol, its kind and the
        /// line itself.
        #[arg(long)]
        json: bool,
    },
    /// Print the index lines whose name matches a query: a class or member
    /// name, a heading of the reference, which is where a keyword such as
    /// `autoload` is found, or a configuration key, a command, a flag or an
    /// error code.
    ///
    /// A command rather than an instruction to grep the index, because a
    /// namespaced name loses its backslash to the shell before `grep` sees it
    /// and the empty result that follows is indistinguishable from a name the
    /// language does not have.
    Find {
        /// What to match against a name or a heading, without case. A flag
        /// is written as it is typed, as `--port`.
        #[arg(allow_hyphen_values = true)]
        query: String,
        /// Print the matching lines as the JSON document `index --json`
        /// prints. No match gives an empty `entries` array.
        #[arg(long)]
        json: bool,
    },
    /// Print one symbol's card — its signature, its prose, its parameters and
    /// what it throws — or the section of the reference a line names.
    Show {
        /// The symbol an index line opens with, as `Core\IO::read` or
        /// `programs#autoload-find-a-class-by-its-namespace`, or the name after
        /// `config: `, `command: `, `flag: ` or `code: `, as `E0621`. A
        /// chapter's own name, as `programs`, lists its sections.
        symbol: String,
        /// Print the card as one JSON object, with each part of the card as a
        /// field. An unknown symbol gives an `error` and a `nearest` list, on
        /// standard output, and the command still fails.
        #[arg(long)]
        json: bool,
    },
    /// Install this surface into the project in the working directory: an
    /// `AGENTS.md` stanza, and the files the coding agent you run needs beside
    /// it. Each file names the four commands above and the `nvs check` loop.
    ///
    /// The agent is found from the environment it runs commands in. Codex,
    /// Cursor and OpenCode read `AGENTS.md`, so they need no other file. Claude
    /// Code gets a skill and GitHub Copilot an instructions file.
    ///
    /// Claude Code and Cursor also get a hook in their settings file:
    /// after each edit of a `.nvs` file, the agent runs `nvs agent hook`,
    /// which checks the file and shows the agent its errors.
    ///
    /// None of the files states a language fact. The four commands give those
    /// answers, and a copy of one would be wrong after the language changes.
    ///
    /// Running it again updates every file it wrote that nobody has edited
    /// since, whichever agent runs it, and refuses a file somebody has edited.
    // `rule:tooling/an-adapter-carries-protocol-and-never-language`.
    Init {
        /// Install for this agent, and not the one found in the environment.
        /// Give it more than once for more than one agent.
        #[arg(
            long = "agent",
            value_name = "NAME",
            value_parser = clap::builder::PossibleValuesParser::new(agent::agent_names()),
            conflicts_with = "all"
        )]
        agents: Vec<String>,
        /// Install for every agent this command knows.
        #[arg(long)]
        all: bool,
        /// Replace a file somebody edited after `init` wrote it.
        #[arg(long)]
        force: bool,
        /// Write nothing; name each file that is missing, outdated or edited,
        /// and fail when there is one.
        #[arg(long, conflicts_with = "force")]
        check: bool,
        /// Add no hook, and remove each hook an earlier run added. With
        /// `--check`, hooks are not checked.
        #[arg(long)]
        no_hooks: bool,
    },
    /// Check the `.nvs` file a coding agent just edited, and show the agent
    /// its errors. `nvs agent init` adds this command to Claude Code's and
    /// Cursor's settings, and the agent runs it after each edit.
    ///
    /// The command reads the agent's JSON message on standard input. It
    /// checks the file the way `nvs check <file>` does, in the directory the
    /// message names. When the file has errors, it prints the first five as
    /// JSON for the agent and says how many more there are. Warnings are not
    /// shown. A file that is not a `.nvs` file, a clean file and a message it
    /// cannot read print nothing. The command always exits with status 0, so
    /// it never stops the agent.
    // `rule:tooling/an-adapter-carries-protocol-and-never-language`.
    Hook {
        /// The agent that runs the command, which decides the shape of the
        /// JSON it prints.
        #[arg(value_parser = clap::builder::PossibleValuesParser::new(agent::hooked_agent_names()))]
        agent: String,
    },
}

/// `nvs api`'s own subcommands.
///
/// A subcommand group rather than a bare `nvs diff` because what is being
/// diffed is the *API*, and a bare verb would own a name the next artifact
/// would want.
#[derive(Subcommand)]
enum ApiCommand {
    /// Classify every change between two OpenAPI documents, exiting non-zero on
    /// a breaking one.
    ///
    /// Run it in CI against the document from the last release and a breaking
    /// change stops the build.
    // The gate of `rule:routing/api-diff-fails-a-breaking-change`; see
    // [`api_diff`] for what each class covers.
    Diff {
        /// The document to compare against — the last release's.
        old: PathBuf,
        /// The document this build produced.
        new: PathBuf,
    },
}

/// `nvs config`'s own subcommands.
///
/// `rule:config/check-and-dump-audit-the-tree-offline` names `check`, `dump` and `ctl config`, and
/// this enum holds the offline ones. `ctl config` belongs to the
/// control socket `rule:config/one-local-control-socket`
/// reserves and arrives with `nvs ctl`.
#[derive(Subcommand)]
enum ConfigCommand {
    /// Resolve the configuration tree and report what it holds, exiting
    /// non-zero on any refusal.
    ///
    /// Offline: it reads the files and nothing else, so a tree is validated in
    /// CI before it is deployed.
    // What the audit deliberately does not assert is [`config`]'s own module
    // doc.
    Check {
        /// The root files to read, in order — the same list `--config` takes,
        /// given positionally. Naming one disables the search for `./nvs.toml`;
        /// with none, `./nvs.toml` is read, and failing that the shipped
        /// defaults.
        // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
        // steps 1 to 3.
        files: Vec<PathBuf>,
    },
    /// Print every key in force, one per line, in dotted-key order.
    ///
    /// An include may override the file that pulled it in *on condition* that
    /// every override is recoverable; this is where it is recovered in full.
    // `rule:config/later-wins-and-every-override-is-recorded`. What is printed,
    // and the one thing deliberately absent from it, is [`config::dump`]'s own
    // doc comment.
    Dump {
        /// The root files to read — `check`'s list, read the same way.
        files: Vec<PathBuf>,
        /// Also name the file each key was written in, and the file it
        /// overrode.
        #[arg(long)]
        origin: bool,
        /// Write the resolved configuration as one canonical TOML document
        /// instead, for diffing two environments.
        #[arg(long, conflicts_with = "origin")]
        toml: bool,
    },
}

/// `nvs queue`'s own subcommands.
///
/// `migrate` is the one `rule:core-classes/queue-storage-is-a-table` names outright. Everything
/// else an operator might want of a queue — its depth, a job retried by hand —
/// is a question `Core\Queue::stats` already answers from inside a request, and
/// a second answer here would need this binary to open a connection for it,
/// which is the same wall [`queue`]'s own module doc describes.
#[derive(Subcommand)]
enum QueueCommand {
    /// Create the queue's jobs and dead-letter tables in its database.
    ///
    /// The schema is the runtime's own, and this command is what makes issuing
    /// DDL for it an operator's act rather than a request's. It converges: what
    /// runs is the difference between that schema and the database.
    // `rule:core-classes/queue-storage-is-a-table`. The schema value is
    // `nvs_stdlib::queue::schema`, beside the members that read the columns,
    // and what this can and cannot do today is [`queue`]'s module doc.
    Migrate {
        /// The root files to read, in order — `config check`'s list, read the
        /// same way.
        files: Vec<PathBuf>,
        /// Migrate this `[db.<name>]` block instead of the one `[queue]
        /// connection` names.
        #[arg(long)]
        connection: Option<String>,
        /// Print the statements without running them, and succeed.
        #[arg(long)]
        dry_run: bool,
        /// Run a step that is not `Safe` — a queue table an older Novis built
        /// can need one, and `schema apply` takes the same flag for the same
        /// reason.
        #[arg(long)]
        including_risky: bool,
    },
}

/// `nvs schema`'s own subcommands.
///
/// Three, and they are one walk stopped at three points:
/// `rule:core-classes/schema-introspection`'s read alone is `dump`, the
/// difference that read feeds is `plan`, and running what the difference says is
/// `apply`. There is no `status` beside them because `plan` against a converged
/// database already prints nothing, and no `rollback` because
/// `rule:core-classes/schema-converges` keeps no history to roll back to.
#[derive(Subcommand)]
enum SchemaCommand {
    /// Print the difference between a schema value and the database, and run
    /// none of it.
    ///
    /// Every step carries its grade, the reason for that grade and its complete
    /// SQL — including the steps `apply` refuses, which is what makes a plan
    /// useful against a database this deployment may not write to at all.
    // `rule:core-classes/schema-plan`.
    Plan {
        /// The root files to read, in order — `config check`'s list, read the
        /// same way.
        files: Vec<PathBuf>,
        /// The `[db.<name>]` block to plan against.
        #[arg(long)]
        connection: String,
        /// The JSON file holding the schema value, as `dump` writes one.
        #[arg(long)]
        schema: PathBuf,
    },
    /// Run the steps of that plan that may run.
    ///
    /// Without `--including-risky` this refuses a plan holding a step that is
    /// not `Safe` and names the first one. A report is never run either way.
    // `rule:core-classes/schema-apply-capability`'s `applySafe`, on a command
    // line.
    Apply {
        /// The root files to read, in order.
        files: Vec<PathBuf>,
        /// The `[db.<name>]` block to converge.
        #[arg(long)]
        connection: String,
        /// The JSON file holding the schema value.
        #[arg(long)]
        schema: PathBuf,
        /// Run the steps that can hold a long lock or fail on existing rows as
        /// well, saying so here rather than discovering it in production.
        #[arg(long)]
        including_risky: bool,
    },
    /// Write out the schema value the database already holds, as JSON.
    Dump {
        /// The root files to read, in order.
        files: Vec<PathBuf>,
        /// The `[db.<name>]` block to read.
        #[arg(long)]
        connection: String,
    },
}

/// `nvs tmp`'s own subcommands.
///
/// One, and there is deliberately no second: `rule:core-classes/temporary-dir-orphan-sweep` gives the sweep one
/// predicate — the owning process is not alive — and no flag that overrides it,
/// so there is nothing for an operator to ask beyond "do it" and "tell me what
/// you would do".
#[derive(Subcommand)]
enum TmpCommand {
    /// Remove every entry in the temporary root whose owning process is gone.
    ///
    /// Keyed on liveness and never on age, so a long-running process's
    /// directories are untouchable however old they are, and there is no force
    /// flag.
    // What it can and cannot do, and why there is no force flag, is [`tmp`]'s
    // module doc.
    Clean {
        /// Print what would be removed and remove nothing.
        #[arg(long)]
        dry_run: bool,
    },
}

/// `nvs ctl`'s own subcommands, which are
/// `rule:config/one-local-control-socket`'s three operations and nothing else.
///
/// The roster is closed, and it is closed in [`nvs_server::control`] rather than
/// here: the method and target each operation is performed with are that
/// module's, and a surface that could be extended by adding a name here would be
/// `rule:security/no-eval`'s door under another one. [`ctl`]'s module doc owns
/// what this binary does with an answer.
#[derive(Subcommand)]
enum CtlCommand {
    /// Re-read the whole configuration tree and publish it, printing what the
    /// running process applied and what it could not.
    Reload,
    /// Print the configuration the running process is actually holding, each key
    /// with the file it was written in.
    ///
    /// `nvs config dump --origin` reads the same tree from disk, so a difference
    /// between the two is a reload that has not happened yet.
    Config,
    /// Report how many requests are in flight, and whether the process is
    /// draining.
    Status,
}

/// `nvs service`'s own subcommands, which are
/// `rule:packaging/a-service-is-one-stored-argv`'s list.
///
/// `unit` is the one `rule:packaging/the-unit-is-printed-and-install-is-the-opt-in` makes the default on Linux:
/// generate the unit and **print** it, because the operator's configuration
/// management already owns the directory it belongs in and a binary that writes
/// there behind Ansible's back is a worse citizen than one that prints. On
/// Windows it emits § 5's equivalent `New-Service` invocation, carrying § 3's
/// encoded `ImagePath` for review rather than execution.
///
/// Every § 2 refusal runs in front of both it and `install`, so `unit` is also
/// how an operator finds out that the argv they were about to install would
/// have been refused — without an elevated shell, and without having installed
/// anything. Which manager a verb reaches is `service::at_host`'s answer, and
/// [`service`]'s module doc owns why `run` is not here yet.
#[derive(Subcommand)]
enum ServiceCommand {
    // Every `///` line below is printed by `nvs service --help`, so it is
    // written as `AGENTS.md` § *Text an end user reads* asks.
    /// Installs the command after `--` as a system service. The account that
    /// the service runs as can then read its configuration file and write to
    /// its log folder and its cache folder.
    #[command(after_help = SERVICE_INSTALL_EXAMPLE)]
    Install(ServiceInstall),
    /// Removes an installed service. It also deletes everything that `install`
    /// created, and takes back the access that `install` gave.
    Uninstall {
        /// The name of the service, as it was installed.
        name: String,
        /// Prints every step, and changes nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Tells the service manager to start an installed service.
    Start {
        /// The name of the service.
        name: String,
    },
    /// Stops a running service. A server stops taking new requests. It
    /// finishes the requests it already has, and then it exits.
    Stop {
        /// The name of the service.
        name: String,
    },
    /// Prints the state of the service. For a server, it also prints how many
    /// requests are running, and whether the server is stopping.
    // The server's half is read over the control socket its own configuration
    // names.
    Status {
        /// The name of the service.
        name: String,
    },
    /// Runs the stored command in this terminal. The service manager starts
    /// the service with the same command.
    Run {
        /// The name of the service.
        name: String,
    },
    /// Prints the service definition that `install` would create. It installs
    /// nothing.
    Unit {
        /// The name of the service. The operating system uses this name.
        /// `nvs ctl --socket` also uses it to reach one server when several
        /// are running.
        name: String,
        /// The account that the service runs as. The default is the local
        /// system account, `LocalSystem`. `SYSTEM` is another name for the
        /// same account.
        // `docs/decisions/0093.md` § 4.
        #[arg(long, value_name = "ACCOUNT")]
        account: Option<String>,
        /// Not allowed. It gives error `E0633`. Other users on the computer
        /// can read a command line. The installer prompts you for the password
        /// instead.
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,
        /// The `nvs` command that the service runs, written after `--`.
        ///
        /// The `--` is required. Everything before it is an option of the
        /// installer. Everything after it is stored exactly as you wrote it.
        /// So the service command can use an option that the installer also
        /// has, for example `--start`.
        // `docs/decisions/0093.md` § 1.
        #[arg(
            last = true,
            required = true,
            allow_hyphen_values = true,
            value_name = "ARGS"
        )]
        argv: Vec<String>,
    },
}

/// The text under `nvs service install --help`: one whole command line for each
/// platform, and the rules a first install meets.
///
/// An end user reads this, so it is written as `AGENTS.md` § *Text an end user
/// reads* asks.
const SERVICE_INSTALL_EXAMPLE: &str = r#"Examples:
  Each example is shown on several lines. Type it as one line.

  Windows, in a terminal started as administrator:

    nvs service install shop
        --description "Shop web server"
        --start delayed
        --restart on-failure
        --depends-on postgresql
        -- serve D:\srv\shop\public\index.nvs --config D:\srv\shop\nvs.toml

  Linux, as root:

    nvs service install shop
        --description "Shop web server"
        --depends-on postgresql.service
        -- serve /srv/shop/public/index.nvs --config /srv/shop/nvs.toml

  A configuration file with `[[server.mount]]` blocks, and no entry file:

    nvs service install sites
        -- serve --config /srv/www/nvs.toml

  Everything before `--` is an option of the installer. Everything after `--` is
  the `nvs` command that the service runs. It must be `serve` or `run`.

  Write every path in full. Name the configuration file with `--config`. Paths
  inside the configuration file must be full too.

  The entry file is optional for `serve`. With no entry file, the service serves
  every file that the `[[server.mount]]` blocks in the configuration file mount.
  A configuration file with `[[server.mount]]` blocks must set `root` in its
  `[server]` block. Write the full path of the folder that the blocks search.

  A service has no terminal, so it needs a log file. Set this in the
  configuration file:

    [log]
    target = "file:D:/srv/shop/logs/novis.log"

  The installer creates that folder and lets the service write to it.

  Add `--dry-run` to print every step without changing anything. To remove the
  service, run `nvs service uninstall shop`."#;

/// What `nvs service install` takes, which is also what the hidden
/// `nvs install-service` takes: one struct, because two spellings of one
/// command that drifted apart in their options would be two commands.
#[derive(clap::Args)]
struct ServiceInstall {
    /// The name of the service. The operating system uses this name.
    /// `nvs ctl --socket` also uses it to reach one server when several are
    /// running.
    name: String,
    /// The account that the service runs as. The default is the local system
    /// account, `LocalSystem`. `SYSTEM` is another name for the same account.
    // `docs/decisions/0093.md` § 4.
    #[arg(long, value_name = "ACCOUNT")]
    account: Option<String>,
    /// Not allowed. It gives error `E0633`. Other users on the computer can
    /// read a command line. The installer prompts you for the password
    /// instead.
    #[arg(long, value_name = "PASSWORD")]
    password: Option<String>,
    /// When the service starts after the computer starts. The default is
    /// together with the other automatic services.
    // `docs/decisions/0093.md` § 4.
    #[arg(long, value_enum, value_name = "MODE")]
    start: Option<service::registration::StartMode>,
    /// What the service manager does when the service fails. The default
    /// starts it again.
    #[arg(long, value_enum, value_name = "POLICY")]
    restart: Option<service::registration::Restart>,
    /// A service that must start first, for example a database. Repeat this
    /// option for each one.
    #[arg(long, value_name = "SERVICE")]
    depends_on: Vec<String>,
    /// A description that administrators see next to the name. The default is
    /// the name of the service.
    #[arg(long, value_name = "TEXT")]
    description: Option<String>,
    /// Prints every step, and changes nothing.
    #[arg(long)]
    dry_run: bool,
    /// The `nvs` command that the service runs, written after `--`.
    ///
    /// The `--` is required. Everything before it is an option of the
    /// installer. Everything after it is stored exactly as you wrote it. So
    /// the service command can use an option that the installer also has, for
    /// example `--start`.
    // `docs/decisions/0093.md` § 1.
    #[arg(
        last = true,
        required = true,
        allow_hyphen_values = true,
        value_name = "ARGS"
    )]
    argv: Vec<String>,
}

/// `nvs serve`, under the service control manager if it was started by one.
///
/// On Windows the stored `ImagePath` is this command (`service::image_path`),
/// and the SCM ends a process that does not connect back to it within its
/// start timeout — so the connect is tried first, and only a console run
/// serves inline (`dispatch`). Everywhere else, and on a console, this is
/// [`serve::run`] and nothing more.
fn serve_command(
    file: Option<PathBuf>,
    listen: Option<String>,
    port: Option<u16>,
    config: Vec<PathBuf>,
    init: config::Init,
) -> ExitCode {
    let run = move || serve::run(file.as_deref(), listen.as_deref(), port, &config, init);
    #[cfg(windows)]
    let run = match dispatch::serving(Box::new(run)) {
        Ok(code) => return code,
        Err(run) => run,
    };
    run()
}

/// The stored argv, run in this process.
///
/// A service manager starts `nvs serve …` itself, so this is not how one is
/// hosted — it is how an operator sees the line the manager runs, on a
/// terminal. § 2's allowlist is two subcommands, which is why two arms here
/// are the whole of it, and anything else is a stored argv this binary no
/// longer accepts.
fn run_hosted(argv: &[String]) -> ExitCode {
    let cli =
        match Cli::try_parse_from(std::iter::once("nvs").chain(argv.iter().map(String::as_str))) {
            Ok(cli) => cli,
            Err(error) => {
                eprintln!("error: the stored argv is not one this binary accepts: {error}");
                return ExitCode::FAILURE;
            }
        };
    let no_init = std::env::var_os(config::NO_INIT);
    let init = config::init_gate(
        cli.command.as_ref().is_some_and(initializes),
        cli.no_init,
        no_init.as_deref(),
    );
    match cli.command {
        Some(Command::Serve { file, listen, port }) => {
            serve::run(file.as_deref(), listen.as_deref(), port, &cli.config, init)
        }
        Some(Command::Run {
            file,
            dump_ir,
            dump_asm,
            fault_inject,
            count,
            request,
            peer,
            events,
            arguments,
        }) => run_run(
            &file,
            dump_ir,
            dump_asm,
            fault_inject,
            count,
            request.as_deref(),
            peer.as_deref(),
            events.as_deref(),
            &cli.config,
            arguments,
            init,
        ),
        _ => {
            eprintln!("error: only `serve` and `run` may be stored as a service");
            ExitCode::FAILURE
        }
    }
}

/// Why this binary would refuse `argv` as its own command line, or `None`
/// where it would run it.
///
/// A service manager starts the stored argv with no console, so a usage error
/// there is an exit nobody reads, repeated at every restart. The installer asks
/// here, where the parser is, and refuses on the answer. The text is the
/// parser's first paragraph on one line: the usage block under it is about a
/// command the operator did not type.
fn unaccepted(argv: &[String]) -> Option<String> {
    let error =
        Cli::try_parse_from(std::iter::once("nvs").chain(argv.iter().map(String::as_str))).err()?;
    if matches!(
        error.kind(),
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
    ) {
        return Some("it prints help and exits".to_owned());
    }
    let text = error.to_string();
    let first = text.split("\n\n").next().unwrap_or(&text);
    Some(
        first
            .trim_start_matches("error: ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// `nvs service install` and `nvs install-service`, which are one command.
fn install_service(config: &[PathBuf], args: &ServiceInstall) -> ExitCode {
    let unaccepted = unaccepted(&args.argv);
    service::install(
        config,
        &args.name,
        &args.argv,
        &service::InstallOptions {
            account: args.account.as_deref(),
            password: args.password.as_deref(),
            unaccepted: unaccepted.as_deref(),
            start: args.start.unwrap_or_default(),
            restart: args.restart.unwrap_or_default(),
            depends_on: &args.depends_on,
            description: args.description.as_deref(),
            dry_run: args.dry_run,
        },
    )
}

/// The closed set of sites `--fault-inject` accepts, one per
/// [`nvs_runtime::FaultSite`].
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum FaultSiteArg {
    /// The request's second runtime helper call panics.
    HelperPanic,
}

impl From<FaultSiteArg> for nvs_runtime::FaultSite {
    fn from(arg: FaultSiteArg) -> Self {
        match arg {
            FaultSiteArg::HelperPanic => Self::HelperPanic,
        }
    }
}

fn main() -> ExitCode {
    // Before the footer read below, because a bundled application serves
    // requests too and the hook is what puts a panic under one of them into
    // that request's log. It claims nothing else: every command here is a CLI
    // run with no request beneath it, so its panic goes to stderr through the
    // default hook exactly as it did — `nvs_runtime::floor::install_panic_hook`
    // owns that split.
    nvs_runtime::floor::install_panic_hook();

    // Tier 3 of `rule:errors/escalation-ladder` lives in `nvs_host`, above the
    // crate that classifies a failure raised on a release path, so it arrives
    // there as a function pointer. Here rather than per command because every
    // command below can run compiled code, and a release happens wherever a
    // refcount reaches zero — `nvs_runtime::Ctx::with_pending_set_aside` is the
    // classifier that has no other way to reach it.
    nvs_runtime::floor::install_ladder(nvs_host::ladder::escalate);

    // `rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`, and it happens before clap sees anything: a bundled
    // executable's `argv` belongs to the program it carries, so an app whose
    // first argument is `run` or `--help` must not have it read as one of
    // ours. An ordinary `nvs` finds no footer and falls straight through.
    if let Some(payload) = bundle::embedded() {
        return bundle::run(payload);
    }

    let cli = Cli::parse();

    // Unreachable: `arg_required_else_help` makes a bare `nvs` print help.
    let Some(command) = cli.command else {
        return ExitCode::FAILURE;
    };

    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3's gate,
    // decided once from [`initializes`] and handed down by the arms that resolve a tree.
    let no_init = std::env::var_os(config::NO_INIT);
    let init = config::init_gate(initializes(&command), cli.no_init, no_init.as_deref());

    match command {
        Command::Ast {
            file,
            json,
            // Selects nothing, because it is the default. It exists so a
            // caller can write down which half of the pair it wants, and
            // clap refuses it beside `--strict` rather than picking one.
            resilient: _,
            strict,
        } => ast::run(&file, json, strict),
        Command::Check {
            file,
            json,
            autoload_map,
            strict_docs,
        } => run_check(&cli.config, &file, json, autoload_map, strict_docs, init),
        Command::Run {
            file,
            dump_ir,
            dump_asm,
            fault_inject,
            count,
            request,
            peer,
            events,
            arguments,
        } => run_run(
            &file,
            dump_ir,
            dump_asm,
            fault_inject,
            count,
            request.as_deref(),
            peer.as_deref(),
            events.as_deref(),
            &cli.config,
            arguments,
            init,
        ),
        Command::Serve { file, listen, port } => {
            serve_command(file, listen, port, cli.config.clone(), init)
        }
        Command::Test {
            paths,
            filter,
            php,
            jobs,
            format,
            update,
            coverage_lcov,
            coverage_clover,
            list,
            cases,
            record,
        } => run_test(
            &paths,
            filter,
            php,
            jobs,
            format,
            runner::Flags { update, list },
            &coverage::Requested {
                lcov: coverage_lcov,
                clover: coverage_clover,
            },
            cases,
            record,
            &cli.config,
            init,
        ),
        Command::Build {
            file,
            openapi,
            compile,
            output,
        } => {
            if compile {
                bundle::build(&file, output.as_deref())
            } else {
                run_build(&file, openapi)
            }
        }
        Command::Api {
            command: ApiCommand::Diff { old, new },
        } => api_diff::run(&old, &new),
        Command::Init => config::init(&cli.config),
        Command::Config {
            command: ConfigCommand::Check { files },
        } => config::check(&cli.config, &files),
        Command::Config {
            command:
                ConfigCommand::Dump {
                    files,
                    origin,
                    toml,
                },
        } => config::dump(&cli.config, &files, origin, toml),
        Command::Queue {
            command:
                QueueCommand::Migrate {
                    files,
                    connection,
                    dry_run,
                    including_risky,
                },
        } => queue::migrate(
            &cli.config,
            &files,
            connection.as_deref(),
            dry_run,
            including_risky,
        ),
        Command::Schema {
            command:
                SchemaCommand::Plan {
                    files,
                    connection,
                    schema,
                },
        } => schema::plan(&cli.config, &files, &connection, &schema),
        Command::Schema {
            command:
                SchemaCommand::Apply {
                    files,
                    connection,
                    schema,
                    including_risky,
                },
        } => schema::apply(&cli.config, &files, &connection, &schema, including_risky),
        Command::Schema {
            command: SchemaCommand::Dump { files, connection },
        } => schema::dump(&cli.config, &files, &connection),
        Command::Tmp {
            command: TmpCommand::Clean { dry_run },
        } => tmp::clean(&cli.config, dry_run),
        Command::Ctl { socket, command } => {
            let socket = socket.as_deref();
            match command {
                CtlCommand::Reload => ctl::reload(&cli.config, socket),
                CtlCommand::Config => ctl::config(&cli.config, socket),
                CtlCommand::Status => ctl::status(&cli.config, socket),
            }
        }
        Command::InstallService(args) => install_service(&cli.config, &args),
        Command::Service { command } => match command {
            ServiceCommand::Install(args) => install_service(&cli.config, &args),
            ServiceCommand::Uninstall { name, dry_run } => {
                service::uninstall(&cli.config, &name, dry_run)
            }
            ServiceCommand::Start { name } => service::start(&name),
            ServiceCommand::Stop { name } => service::stop(&name),
            ServiceCommand::Status { name } => service::status(&cli.config, &name),
            ServiceCommand::Run { name } => match service::stored_argv(&name) {
                Ok(argv) => run_hosted(&argv),
                Err(reported) => reported,
            },
            ServiceCommand::Unit {
                name,
                account,
                password,
                argv,
            } => service::print_unit(
                &cli.config,
                &name,
                &argv,
                account.as_deref(),
                password.as_deref(),
                unaccepted(&argv).as_deref(),
            ),
        },
        // `stdio` names the transport the client chose and there is no other
        // one to choose, so it selects nothing here.
        Command::Fmt {
            paths,
            check,
            diff,
            stdin,
        } => {
            if stdin {
                fmt::stdin()
            } else if check {
                fmt::run(&paths, fmt::Mode::Check)
            } else if diff {
                fmt::run(&paths, fmt::Mode::Diff)
            } else {
                fmt::run(&paths, fmt::Mode::Write)
            }
        }
        Command::Lsp { stdio: _ } => match nvs_lsp::run() {
            Ok(()) => ExitCode::SUCCESS,
            // stderr, and never stdout: the writer thread owns stdout and a
            // line printed there would be read as a protocol frame
            // (`rule:ide/stdout-belongs-to-the-protocol`). By the time this is
            // reached the client is usually gone, so this is for the editor's
            // server log rather than for anyone watching.
            Err(err) => {
                eprintln!("error: {err}");
                ExitCode::FAILURE
            }
        },
        Command::LspTest { paths, coverage } => {
            // The report is this terminal program's own output, which is where
            // a `.lspt` summary belongs: `rule:ide/stdout-belongs-to-the-protocol` gives stdout to
            // the protocol only in the process `nvs lsp` runs, and this is not
            // that process — `nvs_lsp::suite` writes to the sink it is handed
            // and names none.
            let mut out = std::io::stdout().lock();
            let report = if coverage {
                nvs_lsp::suite::Report::Coverage
            } else {
                nvs_lsp::suite::Report::Summary
            };
            match nvs_lsp::suite::run(&paths, &mut out, report) {
                Ok(outcome) if outcome.summary.is_success() => ExitCode::SUCCESS,
                Ok(_) => ExitCode::FAILURE,
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Info { licenses } => info::run(licenses),
        Command::Meta { json: _, entry } => meta::run(entry.as_deref()),
        Command::Doc { file, out } => doc::run(&file, &out),
        Command::Stubs { out } => match nvs_lsp::stubs::write_to(&out) {
            Ok(written) => {
                for path in written {
                    println!("{}", path.display());
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: could not write {}: {err}", out.display());
                ExitCode::FAILURE
            }
        },
        Command::Agent { command } => match command {
            AgentCommand::Primer => agent::primer(),
            AgentCommand::Index { json } => agent::index(json),
            AgentCommand::Find { query, json } => agent::find(&query, json),
            AgentCommand::Show { symbol, json } => agent::show(&symbol, json),
            AgentCommand::Init {
                agents,
                all,
                force,
                check,
                no_hooks,
            } => agent::init(&agent::InitOptions {
                agents,
                all,
                force,
                check,
                no_hooks,
            }),
            AgentCommand::Hook { agent } => agent::hook(&agent),
        },
    }
}

/// A **program** that has been through the whole front end with no error.
///
/// `run` needs everything `check` produces plus the tables `nvs-ir` lowering
/// reads back — the resolved-target table, the type interner that backs it,
/// and the class-layout table — so the pipeline is shared rather than written
/// twice.
struct Checked {
    map: SourceMap,
    /// The entry point's own file. It names the program (`nvs run <path>`),
    /// so it stays a single id even though `files` is a set.
    id: nvs_diagnostics::SourceId,
    /// Every file the entry point's `require`/`autoload` graph reached, the
    /// entry file first — `nvs_hir::resolve_program`'s order contract.
    files: Vec<nvs_hir::Loaded>,
    interner: nvs_types::TypeInterner,
    exprs: nvs_types::ExprTypeTable,
    /// Every declared enum's backing type and its cases' values, handed back
    /// by `check_program` rather than rebuilt — `nvs_ir::lower` needs a case's
    /// constant for `rule:types/enum-case-type`'s membership test.
    enums: nvs_types::EnumTable,
    layouts: nvs_types::ClassLayoutTable,
    /// The autoload map the graph walk consulted: `check --autoload-map`'s
    /// document, and the probe trace beside it, which is the field of this
    /// unit's cache key that nothing short of a finished resolution can name
    /// (`rule:packaging/autoload-probes-fold-into-the-cache-key`,
    /// `crate::script::Compiler`).
    autoload: nvs_hir::AutoloadMap,
}

impl Checked {
    /// The program as `nvs-types` and `nvs-ir` both consume it: one entry per
    /// loaded file, **the entry point first**.
    ///
    /// Position zero is where `nvs_ir::lower::lower_program` takes the script
    /// frame from, which is `nvs_hir::resolve_program`'s documented order
    /// contract rather than a coincidence — the assert is what keeps this
    /// file honest if that ever changes.
    fn program_files(&self) -> Vec<nvs_types::ProgramFile<'_>> {
        debug_assert_eq!(
            self.files[0].id, self.id,
            "resolve_program hands the entry file back first"
        );
        self.files
            .iter()
            .map(|file| nvs_types::ProgramFile {
                src: self.map.file(file.id),
                stmts: &file.stmts,
            })
            .collect()
    }
}

/// Parses, resolves and type-checks the program `path` is the entry point of,
/// rendering every diagnostic.
///
/// The unit of work here is the whole `require`/`autoload` graph, not one
/// file: `nvs_hir::resolve_program` walks it into one `Module` plus the
/// statements of every file it loaded (`rule:statements/require-is-the-only-inclusion-construct`, `rule:programs/no-runtime-autoload`), and each table
/// below is then built across that set — a class declared in a `require`d
/// file has to be a class the entry file's body can name.
///
/// `Err` is the exit code to return: a read failure, or at least one error
/// diagnostic. Warnings are rendered and do not stop anything.
fn front_end(path: &std::path::Path) -> Result<Checked, ExitCode> {
    front_end_granted(path, None, false, Sink::Text, config::Init::Never, None)
}

/// [`front_end`] with the deployment's `[capabilities]` block in front of it —
/// `rule:core-classes/db-literal-query-checking`'s check-time question, asked of the `nvs.toml` this machine
/// resolves.
///
/// `config` is the `--config` list when the caller wants that question asked and
/// `None` when it does not, which mirrors `nvs_types::check_program` and
/// `check_program_granted` because it is the same distinction one layer up.
/// `strict_docs` travels beside it for the same reason and is the same shape of
/// question — `nvs check --strict-docs` is the one caller that asks, and
/// `rule:tooling/strict-docs` makes every other front end silent about
/// documentation.
///
/// **`nvs check` is the caller that asks, and `nvs run` is deliberately not.**
/// § 10 is titled for `check` and means it: at run time the refusal is
/// `nvs_runtime::capability::require`'s, and `rule:security/denial-is-a-runtime-error` makes that a denial the
/// program is still running underneath and may catch. Hoisting it into `run`
/// would turn a catchable denial into a refusal to start, which is a different
/// language rather than an earlier answer — `tests/conformance/core/
/// db-open-asks-the-grant-about-the-host-and-then-the-address.nvst` is that
/// behaviour pinned. So `check` is the offline audit that says what this
/// deployment's configuration would refuse, and it is allowed to be the stricter
/// of the two.
///
/// The tree is read **before the program is parsed**, so a `nvs.toml` that does
/// not resolve fails the check as the configuration error it is rather than as
/// whatever the program's own diagnostics happen to be. That is what puts a
/// configuration in front of `nvs_types::intrinsics`' host check, which reads
/// nothing and refuses nobody when a caller hands it `None`.
/// `sink` is which rendering the diagnostics leave by, and it is a parameter
/// here rather than a choice at each [`emit_diagnostics`] call because a run
/// has one: the two calls below are the two ways out of this function, and
/// exactly one of them happens.
///
/// `survey` is the tree `nvs check` looks for a program to borrow an
/// `autoload` map from, and `None` for every other caller ([`front_end_in`]).
fn front_end_granted(
    path: &std::path::Path,
    config: Option<&[std::path::PathBuf]>,
    strict_docs: bool,
    sink: Sink,
    init: config::Init,
    survey: Option<&std::path::Path>,
) -> Result<Checked, ExitCode> {
    front_end_in(
        SourceMap::new(),
        path,
        config,
        strict_docs,
        sink,
        init,
        None,
        survey,
    )
}

/// [`front_end`], writing into `looked` every path the run looked at on disk,
/// whether the program compiled or not.
///
/// `crate::script::Compiler` is the caller. It keeps a compiled program and has
/// to notice when any file behind it changes, and after a failed compile the
/// only list of those files is the one this run leaves behind.
fn front_end_looking(path: &std::path::Path, looked: &mut Looked) -> Result<Checked, ExitCode> {
    front_end_in(
        SourceMap::new(),
        path,
        None,
        false,
        Sink::Text,
        config::Init::Never,
        Some(looked),
        None,
    )
}

/// Every path one front-end run looked at on disk, whatever it concluded.
///
/// `rule:config/an-edit-reaches-the-next-request-without-a-restart` checks all
/// of these, so an edit to any of them reaches the next compile.
#[derive(Debug, Default)]
struct Looked {
    /// Each file the run read, with the digest of the text it read. The entry
    /// file is the first.
    read: Vec<(std::path::PathBuf, nvs_config::cache::Digest)>,
    /// Each path it looked for and found nothing at: a `require` target that was
    /// not there, and the entry file itself when it could not be read.
    missed: Vec<std::path::PathBuf>,
    /// The `autoload` probe trace, in probe order, misses included
    /// (`rule:packaging/autoload-probes-fold-into-the-cache-key`). Empty when
    /// the run stopped before the graph walk.
    probed: Vec<std::path::PathBuf>,
    /// Every directory a discovery scan listed, with the names it held — a
    /// `discover` glob's base and every directory `implementing` walked.
    listed: Vec<nvs_hir::autoload::Listing>,
}

impl Looked {
    /// Copies out of `map` and `autoload` what the run read, missed, probed and
    /// listed.
    fn take(&mut self, map: &SourceMap, autoload: Option<&nvs_hir::AutoloadMap>) {
        self.read = map
            .files()
            .filter_map(|file| {
                let digest = nvs_config::cache::content_hash(file.text().as_bytes());
                Some((file.path()?.to_path_buf(), digest))
            })
            .collect();
        self.missed = map.missed().to_vec();
        self.probed = autoload.map_or_else(Vec::new, |map| map.probe_trace().probed().to_vec());
        self.listed = autoload.map_or_else(Vec::new, |map| map.probe_trace().listed().to_vec());
    }
}

/// [`front_end`] for an entry that exists only as `text`: the program a
/// directory of test files makes, which `nvs test` writes as one `require` per
/// file and never puts on disk
/// (`rule:testing/a-directory-of-programs-is-one-test-program`).
///
/// `path` is where the entry would sit, so every `require` in `text` resolves
/// against that directory exactly as a written entry's would.
fn front_end_synthesized(path: &std::path::Path, text: &str) -> Result<Checked, ExitCode> {
    let mut map = SourceMap::new();
    map.overlay(path, text);
    front_end_in(
        map,
        path,
        None,
        false,
        Sink::Text,
        config::Init::Never,
        None,
        None,
    )
}

/// [`front_end_granted`] over a map the caller prepared — empty for a file on
/// disk, or holding the one overlay [`front_end_synthesized`] registers.
///
/// `looked` is [`front_end_looking`]'s, filled at every way out after the entry
/// file was asked for.
///
/// **`survey` is `nvs check`'s, and lends a class file its program's
/// `autoload` map** (`rule:ide/an-autoloaded-file-borrows-its-programs-map`).
/// A file a program autoloads may not declare `autoload`, so checked as its
/// own entry point every autoloaded name in it is undeclared. When the walk
/// below found no declaration of the entry's own and reported an undeclared
/// name, the `.nvs` files under `survey` are searched in path order for the
/// first program that lends to this file, and the walk runs again with that
/// program's declarations behind the file's own. A file that declares its own
/// map, or names nothing undeclared, is checked exactly once and nothing is
/// searched. A file no program lends to keeps the first walk's diagnostics.
#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per front-end question its four callers answer differently; a struct would be a second spelling of those callers"
)]
fn front_end_in(
    mut map: SourceMap,
    path: &std::path::Path,
    config: Option<&[std::path::PathBuf]>,
    strict_docs: bool,
    sink: Sink,
    init: config::Init,
    looked: Option<&mut Looked>,
    survey: Option<&std::path::Path>,
) -> Result<Checked, ExitCode> {
    let mut id = match map.load(path) {
        Ok(id) => id,
        Err(err) => {
            report_unreadable(path, &err, sink);
            if let Some(looked) = looked {
                looked.take(&map, None);
            }
            return Err(ExitCode::FAILURE);
        }
    };

    // Read after the entry file and before anything is parsed: a missing program
    // is still "could not read", and a broken `nvs.toml` is the configuration
    // error rather than the first thing the parser noticed.
    let grants = match config {
        Some(config) => config::grants(config, path, init)?,
        None => None,
    };

    let core = nvs_stdlib::registry::link_targets();
    let roster = nvs_hir::CoreRoster::Names(&core);
    let plain = nvs_hir::lenders::is_plain(map.file(id).text());
    let (mut diags, mut module, mut loaded, mut autoload) =
        resolve_entry(&mut map, id, roster, strict_docs, &[]);
    if let Some(root) = survey
        && autoload.sites().is_empty()
        && diags.iter().any(names_an_undeclared_type)
        && let Some(lender) = nvs_hir::lenders::first_lender(root, path, plain)
    {
        map = SourceMap::new();
        id = match map.load(path) {
            Ok(id) => id,
            Err(err) => {
                report_unreadable(path, &err, sink);
                return Err(ExitCode::FAILURE);
            }
        };
        (diags, module, loaded, autoload) =
            resolve_entry(&mut map, id, roster, strict_docs, lender.sites());
    }

    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    // The one edge only the walk above knows — which file each written
    // `require` resolved to — carried across to `nvs-ir`, which calls that
    // file's script frame at the site. This function is where both phases
    // are in hand; see `ExprTypeTable::record_require_target` for why it
    // rides in that table rather than in a second argument to
    // `lower_program`.
    for file in &loaded {
        for &(span, target) in &file.requires {
            exprs.record_require_target(span, target);
        }
    }
    let (enums, layouts) = {
        let files: Vec<nvs_types::ProgramFile<'_>> = loaded
            .iter()
            .map(|file| nvs_types::ProgramFile {
                src: map.file(file.id),
                stmts: &file.stmts,
            })
            .collect();
        let enums = nvs_types::check_program_granted(
            &files,
            &module,
            grants.as_ref(),
            &mut interner,
            &mut exprs,
            &mut diags,
        );

        if let Some(looked) = looked {
            looked.take(&map, Some(&autoload));
        }
        if diags.has_errors() {
            emit_diagnostics(&mut diags, &map, sink);
            return Err(ExitCode::FAILURE);
        }
        (enums, nvs_types::build_class_layouts(&files, &module.graph))
    };
    emit_diagnostics(&mut diags, &map, sink);

    Ok(Checked {
        map,
        id,
        files: loaded,
        interner,
        exprs,
        enums,
        layouts,
        autoload,
    })
}

/// Parses the entry `id` and walks its `require`/`autoload` graph, with
/// `borrowed` behind the entry's own `autoload` declarations.
///
/// Every other file's parse and `check_declarations` happen inside the walk,
/// as each `require` target is discovered; only the entry point is this
/// function's to parse.
fn resolve_entry(
    map: &mut SourceMap,
    id: nvs_diagnostics::SourceId,
    core: nvs_hir::CoreRoster<'_>,
    strict_docs: bool,
    borrowed: &[nvs_hir::autoload::Site],
) -> (
    Diagnostics,
    nvs_hir::Module,
    Vec<nvs_hir::Loaded>,
    nvs_hir::AutoloadMap,
) {
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    check_declarations(&stmts, map.file(id), &mut diags);
    let (module, loaded, autoload) = nvs_hir::resolve_program_linted(
        id,
        stmts,
        map,
        core,
        Some(nvs_stdlib::php_names::became),
        &mut diags,
        strict_docs,
        borrowed,
    );
    (diags, module, loaded, autoload)
}

/// Whether `diagnostic` says a class, interface, enum or `type` name, or a
/// `use` of one, is declared nowhere. Those are the two a borrowed `autoload`
/// map can change.
fn names_an_undeclared_type(diagnostic: &nvs_diagnostics::Diagnostic) -> bool {
    diagnostic.code.is_some_and(|code| {
        code == nvs_diagnostics::code::E_UNDEFINED_CLASS
            || code == nvs_diagnostics::code::E_UNRESOLVED_IMPORT
    })
}

/// `nvs check`, and with `--autoload-map` also `rule:programs/autoload`'s last sentence:
/// the resolved prefix → roots map, what a `discover` glob passed over and
/// what an explicit prefix shadowed.
///
/// The map is printed only when the check succeeded, because a program that
/// does not resolve has not necessarily finished building one — the walk
/// stops probing the moment the `require` graph is in doubt, and printing a
/// partial map beside a wall of errors would be read as the whole of it.
/// Paths are shown relative to the entry point's own directory, which is what
/// `autoload`'s literals are written against (§ 1).
///
/// This is the one front end that reads the configuration
/// ([`front_end_granted`]), so `nvs check` answers `rule:core-classes/db-literal-query-checking`'s question
/// about a literal `Core\Db::open` host and reports a `nvs.toml` that does not
/// resolve as the configuration error it is.
///
/// `--strict-docs` is `rule:tooling/strict-docs`'s opt-in lint, and it is
/// `check`'s alone for the same reason the grant question is: it is an audit of
/// what a package publishes, not a condition on running one. It rides in the
/// walk that already resolves a doc comment's tags
/// (`nvs_hir::members::check_documented`), so it reaches every file the program
/// loaded rather than the entry point alone.
///
/// `--json` swaps the rendering and nothing else — the same records, in the
/// same order, on standard output as [`check`]'s document, and the success
/// line dropped because an empty `diagnostics` array already says what "no
/// errors" says. What does *not* produce a document is a failure to read the
/// entry file or to resolve the configuration: neither is a diagnostic, both
/// say so on standard error, and `nvs ast --json` answers them the same way.
fn run_check(
    config: &[std::path::PathBuf],
    path: &std::path::Path,
    json: bool,
    autoload_map: bool,
    strict_docs: bool,
    init: config::Init,
) -> ExitCode {
    let sink = if json { Sink::Json } else { Sink::Text };
    let survey = survey_root(config);
    match front_end_granted(path, Some(config), strict_docs, sink, init, Some(&survey)) {
        Ok(checked) => {
            if autoload_map {
                let base = match path.parent() {
                    Some(dir) if !dir.as_os_str().is_empty() => dir,
                    _ => std::path::Path::new("."),
                };
                print!("{}", checked.autoload.render(base));
            } else if !json {
                println!("no errors");
            }
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

/// The tree `nvs check` searches for a program that lends the checked file its
/// `autoload` map: the directory of the first `--config` file, else the
/// working directory (`.`), where the `nvs.toml` that applies is found.
fn survey_root(config: &[std::path::PathBuf]) -> std::path::PathBuf {
    config
        .iter()
        .find(|path| path.is_file())
        .and_then(|file| file.parent())
        .filter(|dir| !dir.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf)
}

/// `nvs build --openapi` — `rule:routing/api-document-is-a-deterministic-build-artifact`
/// 's emission, on standard output.
///
/// The program goes through the same front end `check` does, and the document
/// is rendered from the route table that front end produced — so a program with
/// a diagnostic emits nothing at all rather than a partial document. That is
/// the ADR's whole promise in one line: the document cannot say something the
/// compiler did not agree to.
///
/// **A program declaring no route emits nothing**, which is § 1's "generates
/// nothing and runs no pass", and it says so on standard error rather than
/// writing a document with an empty `paths`: fed to § 4's diff, an empty
/// document is the claim that every operation was removed.
fn run_build(path: &std::path::Path, openapi: bool) -> ExitCode {
    debug_assert!(
        openapi,
        "the `artifact` group is `required`, so the other artifact's absence means this one"
    );
    let checked = match front_end(path) {
        Ok(checked) => checked,
        Err(code) => return code,
    };
    let routes = checked.exprs.routes();
    if routes.rows().is_empty() {
        eprintln!("no `#[Route]` in this program: nothing to emit");
        return ExitCode::SUCCESS;
    }
    // The file stem is the only name the compiler has for a program: nothing in
    // the language declares one, and `info.title` is required by 3.1.
    let title = path.file_stem().map_or_else(
        || path.display().to_string(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let document = openapi::document(routes, &title);
    // Pretty rather than compact, because the artifact is read by people and
    // diffed by `git` as often as it is by § 4's own gate. It cannot fail: the
    // document is strings, bools and integers, and `serde_json` only errors on
    // a map key that is not a string or a float that is not finite.
    println!(
        "{}",
        serde_json::to_string_pretty(&document)
            .expect("the document holds no unserializable value")
    );
    ExitCode::SUCCESS
}

/// `rule:tooling/commands-are-compiled`'s table, as the runtime carries it.
///
/// A copy rather than a borrow, because the context outlives the front end's
/// own tables in every caller and a task owns what it was handed. It is a
/// handful of `String`s per declared command, taken once per run.
fn runtime_commands(
    table: &nvs_types::commands::CommandTable,
) -> nvs_runtime::commands::CommandTable {
    nvs_runtime::commands::CommandTable::new(
        table
            .rows()
            .iter()
            .map(|row| nvs_runtime::commands::Command {
                name: row.name.clone(),
                about: row.about.clone(),
                handler: row.handler.clone(),
                args: row
                    .args
                    .iter()
                    .map(|arg| nvs_runtime::commands::CommandArg {
                        param: arg.param.clone(),
                        spellings: arg.spellings.clone(),
                        about: arg.about.clone(),
                        default: arg.default.clone(),
                        conv: match &arg.conv {
                            nvs_types::commands::ArgConv::Text => {
                                nvs_runtime::commands::ArgConv::Text
                            }
                            nvs_types::commands::ArgConv::Flag => {
                                nvs_runtime::commands::ArgConv::Flag
                            }
                            nvs_types::commands::ArgConv::Int => {
                                nvs_runtime::commands::ArgConv::Int
                            }
                            nvs_types::commands::ArgConv::Uint => {
                                nvs_runtime::commands::ArgConv::Uint
                            }
                            nvs_types::commands::ArgConv::Decimal => {
                                nvs_runtime::commands::ArgConv::Decimal
                            }
                            // A `Parses` class crosses as the name the checker
                            // resolved, which is the label the matcher calls
                            // that class's own `parse` through. `Core\Uuid` is
                            // the exception and takes the runtime's reader
                            // instead: its parse is a `Core` member with no row
                            // in a program's class table, so there is no label
                            // to call — and one grammar either way, which is
                            // `nvs_runtime::commands::ArgConv::Uuid`'s point.
                            nvs_types::commands::ArgConv::Parses(class) => {
                                if class.as_str() == r"Core\Uuid" {
                                    nvs_runtime::commands::ArgConv::Uuid
                                } else {
                                    nvs_runtime::commands::ArgConv::Parses(class.clone())
                                }
                            }
                            // The set itself crosses, because it is the answer
                            // and not a key: the compiler resolved the union
                            // once and a matcher has nothing left to look up.
                            nvs_types::commands::ArgConv::OneOf(admitted) => {
                                nvs_runtime::commands::ArgConv::OneOf(admitted.clone())
                            }
                            // The cases cross for the set's own reason, one
                            // arm up, and the class with them: the row is what
                            // both the usage line and the refusal name the
                            // enum from, and nothing downstream has a type
                            // table to ask instead. The value narrows to ADR
                            // 0010 § 2's two integer types here rather than
                            // crossing widened — `nvs_runtime::commands::
                            // CaseValue` owns why.
                            nvs_types::commands::ArgConv::Enum { class, cases } => {
                                nvs_runtime::commands::ArgConv::Enum {
                                    class: class.clone(),
                                    cases: cases
                                        .iter()
                                        .map(|(case, value)| (case.clone(), case_value(*value)))
                                        .collect(),
                                }
                            }
                            nvs_types::commands::ArgConv::Unconverted => {
                                nvs_runtime::commands::ArgConv::Unconverted
                            }
                        },
                    })
                    .collect(),
            })
            .collect(),
    )
}

/// `rule:routing/matched-once-before-the-handler`'s table, as the runtime carries it.
///
/// A copy rather than a borrow, for [`runtime_commands`]' reason exactly: the
/// context outlives the front end's own tables in every caller, and a request
/// owns the table it was matched against. It is a handful of `String`s per
/// declared route, taken once per compiled unit and shared by every request
/// that unit answers.
///
/// **The OpenAPI half of a row does not cross** — summary, tags, security,
/// errors and example are `rule:routing/api-document-is-generated-from-the-route-table`'s document, generated from the compiler's
/// own table, and nothing a request asks reads them.
///
/// The one thing here that is a reading rather than a copy is the conversion:
/// `nvs_types::routes::RouteParam` carries the declared type as
/// `TypeInterner::describe` rendered it, and `nvs_runtime::routes::CaptureConv`
/// is the closed set a matcher needs instead. That enum's `Unconverted` doc
/// owns what is left under it.
pub(crate) fn runtime_routes(exprs: &nvs_types::ExprTypeTable) -> nvs_runtime::routes::Routes {
    let table = exprs.routes();
    let crossed = nvs_runtime::routes::Routes::new(
        table
            .rows()
            .iter()
            .map(|row| {
                let route = nvs_runtime::routes::Route::new(
                    row.verb.clone(),
                    row.path.clone(),
                    row.name.as_ref().map(|(name, _)| name.clone()),
                    row.handler.clone(),
                    row.access.clone(),
                    row.params
                        .iter()
                        // `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]` parameters are declared on
                        // the same signature and are not part of the path, so
                        // they are not what a segment fills.
                        .filter(|param| param.source == nvs_types::ParamIn::Path)
                        .map(|param| nvs_runtime::routes::Capture {
                            name: param.name.clone(),
                            conv: capture_conv(param),
                        })
                        .collect(),
                );
                // `rule:attributes/access-payload`'s opt-out, carried across rather than derived a
                // second time: the verb's half of § 4 is `Route::new`'s and the
                // declaration's half is the compiler's, so this is the one
                // place the two meet.
                if row.csrf {
                    route
                } else {
                    route.without_csrf()
                }
            })
            .collect(),
    );
    // The link half of the same crossing, off the call sites the checker
    // resolved rather than off the rows above: a mount's boot check asks
    // whether the unit *builds* an absolute link, which no declaration says.
    // Taken here so that every program this binary runs carries the same
    // answer, since a table marked in one builder and not in another is a
    // check that passes on which path compiled it.
    if exprs.links_absolutely() {
        crossed.linking_absolutely()
    } else {
        crossed
    }
}

/// Which conversion § 5's declared type is, as the matcher spells it.
///
/// A closed set of *values* takes precedence over the type that describes it —
/// a union of string literals renders as a type nothing would convert, and its
/// admitted set is the whole of what § 5 narrows with. Everything the runtime
/// has no arm for is `Unconverted` rather than silently `Text`, so the gap is
/// one an arm closes rather than a behaviour somebody has to notice. A class
/// implementing `Parses` is its own arm and carries its own name, so the
/// crossing can reach the `parse` this walk may not —
/// `nvs_runtime::routes::CaptureConv::Parses` is the home of that split.
fn capture_conv(param: &nvs_types::RouteParam) -> nvs_runtime::routes::CaptureConv {
    use nvs_runtime::routes::CaptureConv;

    // Above the closed set because an enum subset is both: its spellings are a
    // set a link is checked against, and only this arm carries what each one
    // converts to, which is the difference between handing a program its case
    // and handing it the segment.
    if let Some(narrowed) = &param.cases {
        return CaptureConv::Enum {
            class: narrowed.class.clone(),
            cases: narrowed
                .cases
                .iter()
                .map(|(spelling, value)| (spelling.clone(), case_value(*value)))
                .collect(),
        };
    }
    if let Some(allowed) = &param.allowed {
        return CaptureConv::OneOf(allowed.clone());
    }
    match param.ty.as_deref() {
        Some("int") => CaptureConv::Int,
        Some("uint") => CaptureConv::Uint,
        Some("decimal") => CaptureConv::Decimal,
        // The one class § 5 admits, spelled as `describe` renders
        // `nvs_stdlib::uuid::NAME` — matched as text like every arm above, and
        // the same spelling `openapi`'s schema arm matches.
        Some(r"Core\Uuid") => CaptureConv::Uuid,
        // A capture is always `tainted`, and both spellings render for one
        // declared `string` depending on where the qualifier was written.
        Some("string" | "tainted string") | None => CaptureConv::Text,
        // Below the `Core\Uuid` arm on purpose: that class implements `Parses`
        // too, so a rule reading the interface alone would take the one type
        // the engine ships off the match and defer it to the crossing.
        Some(class) if param.parses => CaptureConv::Parses(class.to_owned()),
        Some(_) => CaptureConv::Unconverted,
    }
}

/// One enum case's constant, narrowed to the two integer types a running
/// program holds it in.
///
/// Both tables that carry an enum's cases cross here — a command's argument and
/// a route's capture — because the narrowing is one decision:
/// `nvs_runtime::commands::CaseValue` owns why the value does not travel widened
/// to the `i128` a membership test wants.
fn case_value(value: nvs_types::enums::EnumValue) -> nvs_runtime::commands::CaseValue {
    match value {
        nvs_types::enums::EnumValue::Int(number) => nvs_runtime::commands::CaseValue::Int(number),
        nvs_types::enums::EnumValue::Uint(number) => nvs_runtime::commands::CaseValue::Uint(number),
    }
}

/// Writes the file `Core\Response::sendFile` named into the run's output, a
/// chunk at a time — what `nvs run --request` does where a server would stream
/// the file at the connection.
///
/// Nothing about the program's authority is asked again: the member resolved
/// the name under `fs.read` and refused everything it could not send, which is
/// the same reading `nvs_server::serve`'s `sent` makes. The file is read in
/// pieces for the reason the server reads it at the connection: a response of
/// any size costs this run one buffer and never a copy of the file.
fn send_file_body(ctx: &mut nvs_runtime::Ctx, path: &std::path::Path) -> std::io::Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut chunk = vec![0_u8; 64 * 1024];
    loop {
        let read = std::io::Read::read(&mut file, &mut chunk)?;
        if read == 0 {
            return Ok(());
        }
        ctx.write_output(&chunk[..read])?;
    }
}

/// Builds the carrier `nvs run --request <file>` describes.
///
/// The format is `nvs-test`'s, because that crate writes these files for a
/// `.nvst` case and a format has one home; this is its only reader. Field
/// names arrive lower-cased from there, which is the shape a served request
/// carries them in — a case that pinned `Accept` here would meet `accept` in
/// production.
///
/// **It goes through [`nvs_runtime::InboundSpec`] rather than filling a carrier
/// in itself**, which is that type's whole reason: a request nobody sent
/// becomes an `Inbound` in one place, so what a `.nvst` case's request carries
/// and what a `Core\Test::request` bag builds cannot answer differently. The
/// body is the raw spelling because the file holds the octets as they would go
/// on the wire, already typed by whatever `content-type` line it carries — and
/// a field the file wrote is a field the spec leaves alone, so the derivation
/// the two share fires only for a file written by hand that omitted one.
fn inbound_from(path: &std::path::Path) -> Result<nvs_runtime::Inbound, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    inbound_of(&text)
}

/// The crossing itself, over the file's text rather than its path.
///
/// Split from [`inbound_from`], which opens a file and decides nothing, because
/// this is the whole of what this binary decides here — and because the crate
/// has no library target, so the seam is reachable from a test only from inside
/// the binary.
fn inbound_of(text: &str) -> Result<nvs_runtime::Inbound, String> {
    let wire = nvs_test::request::read(text)?;
    let mut spec = nvs_runtime::InboundSpec::new(&wire.method, &wire.path);
    spec.set_query(&wire.query);
    for (name, value) in &wire.headers {
        spec.push_header(name, value.as_bytes());
    }
    if let Some(body) = wire.body {
        spec.set_body(nvs_runtime::SpecBody::Raw(body.into_bytes()))?;
    }
    // The peer is stated, never derived: `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s
    // walk is what decides a served request's address and scheme, and a file
    // describes the answer it reached rather than the headers it read. The two
    // `Scheme` enumerations are the same two cases in a crate that may not name
    // this one, so this is where they meet.
    spec.set_peer(
        wire.client_ip,
        match wire.scheme {
            nvs_test::case::Scheme::Http => nvs_runtime::Scheme::Http,
            nvs_test::case::Scheme::Https => nvs_runtime::Scheme::Https,
        },
    );
    // Already in the shape `nvs_server::mount::carry` leaves a served carrier
    // in, so the program reads the pair a door would have handed it.
    spec.set_mount(&wire.mount_prefix, &wire.mount_captures);
    Ok(spec.build())
}

#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per `Command::Run` flag, plus the tree the run resolves; a struct here would be a second spelling of that variant"
)]
fn run_run(
    path: &std::path::Path,
    dump_ir: bool,
    dump_asm: bool,
    fault_inject: Option<FaultSiteArg>,
    count: bool,
    request: Option<&std::path::Path>,
    peer: Option<&std::path::Path>,
    events: Option<&std::path::Path>,
    config: &[PathBuf],
    arguments: Vec<String>,
    init: config::Init,
) -> ExitCode {
    let checked = match front_end(path) {
        Ok(checked) => checked,
        Err(code) => return code,
    };
    let src = checked.map.file(checked.id);

    let program = nvs_ir::lower::lower_program(
        nvs_ir::lower::ENTRY_SCRIPT_LABEL,
        &checked.program_files(),
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    if dump_ir {
        for function in &program.functions {
            print!("{}", nvs_ir::print::print_function(function, src));
        }
        return ExitCode::SUCCESS;
    }

    if dump_asm {
        // Deliberately the same compile `run` performs, disassembled rather
        // than a second differently-configured one — see
        // `nvs_codegen::disassemble`. Like `--dump-ir`, it prints instead of
        // running.
        return match nvs_codegen::disassemble(&program) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        };
    }
    // `rule:config/the-config-is-an-immutable-snapshot`: the tree is resolved and folded into one snapshot **before**
    // the request exists, and the request then clones it once. A refusal here is
    // a refusal to start — `rule:config/later-wins-and-every-override-is-recorded`'s later-wins and § 6's boundary are only
    // worth anything if a tree that does not resolve stops the run.
    let mut config_sources = SourceMap::new();
    // A bundled program's entry file is a synthetic path inside the payload
    // (`rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`), and `trust::canonical` has no filesystem entry to
    // examine for it. The executable itself is what an `[[app]]` block could
    // legitimately key on, and it is also all § 1's single trust domain
    // leaves to key on: the only principal here is whoever ran the binary.
    let config_entry = if nvs_diagnostics::embedded::is_active() {
        std::env::current_exe().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    let snapshot = match config::boot_snapshot(config, &config_entry, &mut config_sources, init) {
        Ok(snapshot) => snapshot,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &config_sources);
            return ExitCode::FAILURE;
        }
    };
    // `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s advisories: a secret file another account can read is
    // reported and does not stop the run.
    if !snapshot.warnings.is_empty() {
        let mut diags = Diagnostics::new();
        for warning in &snapshot.warnings {
            diags.report(warning.clone());
        }
        render_diagnostics(&mut diags, &config_sources);
    }
    // The outbound TLS client is the process's, so it is built here — once, off
    // the snapshot, before anything this run compiles or executes can reach the
    // network. `config::install_tls_client` owns why the call sits at the run
    // sites rather than inside the boot every subcommand shares.
    if let Err(diagnostic) = config::install_tls_client(&snapshot) {
        let mut diags = Diagnostics::new();
        diags.report(diagnostic);
        render_diagnostics(&mut diags, &config_sources);
        return ExitCode::FAILURE;
    }

    // `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` and `rule:config/opcache-file-cache-directives-are-system`: the unit comes off disk when this environment has an
    // artifact for this program and out of Cranelift when it does not, and
    // `cache::unit_for` is the whole of that decision — every way the cache can
    // fail to answer is a compile, so nothing here reports one.
    //
    // **This is why the compile sits below the snapshot rather than above it.**
    // Both halves of the key are configuration: § 7's `[opcache]` block says
    // where artifacts live and whether they are used at all, and `rule:config/the-extension-set-is-in-every-unit-key`'s
    // environment digest — which covers the loaded extension set — is the other
    // half of every key. A pre-boot default for either would address an
    // artifact by an environment this run is not in, which is the one thing the
    // key exists to prevent.
    let (unit, _) = match cache::unit_for(
        &program,
        cache::program_digest(&checked.program_files()),
        cache::from_config(&snapshot.config).as_ref(),
    ) {
        Ok(unit) => unit,
        Err(error) => {
            report_internal(error.to_string());
            return ExitCode::FAILURE;
        }
    };
    // Shared rather than owned outright, because `rule:testing/in-process-request`'s in-process
    // request runs this same unit's script frame as a child isolate and the
    // seam holding it outlives no part of this run — `runner::UnderTest` is the
    // one holder, and an `Rc` is what lets the run and the seam both name it.
    let unit = std::rc::Rc::new(unit);
    let Some(entry) = unit.script() else {
        report_internal("the script frame was not compiled");
        return ExitCode::FAILURE;
    };

    // `rule:core-classes/queue-storage-is-a-table`'s `workers` is per *instance*, and a CLI run is one — so a
    // run of this tree claims jobs beside its script, including ones another
    // instance enqueued and never finished. Read here rather than inside the
    // worker because the snapshot is moved onto the context below, and
    // `queue_for` is the same resolution boot already accepted
    // (`nvs_config::queue`), so a refusal is impossible by the time this runs
    // and `.ok()` is not swallowing one. `workers = 0` is § 2's enqueue-only
    // deployment and starts nothing.
    let queued = nvs_config::queue::queue_for(&snapshot.config, &std::collections::BTreeMap::new())
        .ok()
        .flatten()
        .filter(|bounds| bounds.workers > 0)
        .and_then(|bounds| {
            let block = snapshot.config.db.get(&bounds.connection)?.clone();
            Some((bounds, block))
        });

    // The script's own frame is the request, for a CLI run: one `Ctx` writing
    // to the process's standard output.
    let mut ctx = nvs_runtime::Ctx::stdout();
    // `rule:routing/an-absolute-link-takes-a-configured-origin`'s origin is resolved before the program runs and never
    // during it, which is the whole of what makes it un-sniffable. It comes off
    // the snapshot, so it is the origin of the `[[app]]` blocks that actually
    // match this entry file (`rule:config/every-matching-app-block-applies-least-specific-first`) and not of any block in the file.
    if let Some(origin) = snapshot.origin.clone() {
        ctx.set_origin(&origin);
    }
    // `rule:routing/matched-once-before-the-handler`'s table, crossed into the
    // runtime's shape once: a `--request` below is matched against it, and the
    // context is handed it further down.
    let routes = std::sync::Arc::new(runtime_routes(&checked.exprs));
    // `--request`: the request is read off a file rather than off a socket,
    // and that is the whole of the difference. From here down a program
    // answering one is in the state `nvs serve` puts it in, which is what lets
    // a `.nvst` case pin what `Core\Request` answers at all. That state
    // includes the door's one match, taken here the way `runner.rs`'s
    // `UnderTest::answer` takes it, so `Core\Request::route()` answers what a
    // served request would. A program with no `#[Route]` claims nothing.
    //
    // A request whose `Upgrade` header names `websocket` is offered
    // `rule:concurrency/an-upgrade-is-spawn-shaped`'s slot, the one
    // `nvs_server::serve_connection` offers a request `hyper` framed an upgrade
    // for, so `Core\Socket::upgrade` prepares a connection here as it does
    // there. `peer::upgradable` owns why the header is the whole question.
    //
    // Every request is also offered `rule:concurrency/two-doors-one-isolate`'s
    // SSE cell, as `nvs_host::Isolate::offering_sse` offers it to every request
    // a server answers, so `Core\Sse::upgrade` prepares a stream here too.
    let mut offered = None;
    let mut offered_sse = None;
    if let Some(file) = request {
        match inbound_from(file) {
            Ok(mut inbound) => {
                if let Some(matched) = routes.match_request(inbound.method(), inbound.path()) {
                    inbound.set_route(matched);
                }
                if peer::upgradable(&inbound) {
                    let slot = nvs_runtime::UpgradeSlot::new();
                    inbound.offer_upgrade(slot.clone());
                    offered = Some(slot);
                }
                let sse = nvs_runtime::SseSlot::new();
                inbound.offer_sse(sse.clone());
                offered_sse = Some(sse);
                ctx.set_inbound(inbound);
            }
            Err(error) => {
                eprintln!("error: --request {}: {error}", file.display());
                return ExitCode::FAILURE;
            }
        }
    }
    // `--peer`: the socket a served connection's isolate is handed, read off a
    // file instead. With no `--request` the program *is* the connection, and
    // the socket is moved onto its context at the point
    // `nvs_host::Isolate::over_socket` moves a real one. Beside a `--request`
    // it is kept for the connection that request's upgrade opens, and a request
    // that upgrades with no `--peer` gets a peer that closes at once.
    let mut connection_peer = None;
    if let Some(file) = peer {
        match peer::from_file(file) {
            Ok(scripted) if request.is_some() => {
                connection_peer = Some(scripted);
            }
            Ok(scripted) => ctx.set_peer(Box::new(scripted)),
            Err(error) => {
                eprintln!("error: --peer {}: {error}", file.display());
                return ExitCode::FAILURE;
            }
        }
    }
    // `--events`: the other door. The program is handed the writing half of a
    // stream and marked as the connection `Core\Sse::upgrade` opens, which is
    // what `nvs_host::Isolate::over_event_stream` does to a served one. The
    // other half, the file's values and the program's queue go to the
    // publisher spawned beside the program below. Beside a `--request` the
    // file is kept for the stream that request's `Core\Sse::upgrade` opens,
    // and `events::connect` publishes it there.
    let mut publisher = None;
    let mut stream_feed = None;
    if let Some(file) = events {
        match events::from_file(file) {
            Ok(feed) if request.is_some() => stream_feed = Some(feed),
            Ok(feed) => {
                let (emit, drain) = events::open();
                ctx.set_body_stream(emit);
                ctx.mark_event_stream(nvs_runtime::EventStreamDoor::Connection);
                publisher = Some((feed, drain, ctx.inbox()));
            }
            Err(error) => {
                eprintln!("error: --events {}: {error}", file.display());
                return ExitCode::FAILURE;
            }
        }
    }
    // The workers get their own handle on the same tree, taken before it is moved onto this
    // context: a job is an isolate resolved through `rule:security/capability-check-at-the-door`'s spawn door, that door asks the
    // *context* it is resolved from, and a worker's context is not the script's. `worker`'s module
    // doc owns why the deployment's own snapshot is the right answer there. A holder because a
    // served worker reads a holder a reload publishes into; this run never publishes into it.
    let for_workers = queued
        .is_some()
        .then(|| std::sync::Arc::new(nvs_config::Current::new(std::sync::Arc::clone(&snapshot))));
    // And the unit cache gets one for the same reason, taken at the same
    // moment: `[opcache]` decides when a `spawn script` path is re-checked, and
    // `rule:config/the-extension-set-is-in-every-unit-key`'s environment digest is half of every key it holds
    // (`script`'s module doc). The compiler itself is built where it is
    // installed, below.
    let for_compiler = std::sync::Arc::clone(&snapshot);
    ctx.set_config(snapshot);
    // `rule:tooling/commands-are-compiled`: `Core\Command`'s members are generated from the table the
    // front end already built, so the rows cross here — once, before the program
    // starts, like everything else this context is handed.
    // `nvs_runtime::commands` owns why they cross as a runtime value rather than
    // being folded while checking.
    let commands = checked.exprs.commands();
    if !commands.rows().is_empty() {
        ctx.set_commands(std::sync::Arc::new(runtime_commands(commands)));
    }
    // `rule:routing/matched-once-before-the-handler`: the same crossing one table along.
    // `Core\Router`'s own members read the table, so it is installed wherever a
    // program runs, with or without a request, rather than only where a server
    // is answering.
    if !routes.rows().is_empty() {
        ctx.set_routes(routes);
    }
    // The words past the file are the program's own, and `Core\Command::run`
    // matches them against the table above. Written here rather than read from
    // `std::env` inside the member, because a served request has a command line
    // only in the sense that the *server* was started with one.
    ctx.set_command_line(arguments);
    // And the name the shell knows the program by, which `Core\Command`'s
    // completion scripts register against. It is the stem of the same path the
    // configuration tree keyed on above — the script's for `nvs run`, the
    // executable's for a bundle — and `Ctx::program_name` owns why each is the
    // right answer. Written here rather than read from `std::env` inside the
    // member: `rule:security/capability-check-at-the-door` keeps `argv[0]` out of `nvs-stdlib`, and a served
    // request has no name to give it anyway.
    ctx.set_program_name(
        config_entry
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    // `rule:programs/no-runtime-autoload`'s program identity, which is the one value written here that is
    // a fact about the whole graph rather than about the entry file: every
    // unit's content hash in `resolve_program`'s order, folded with `rule:config/the-extension-set-is-in-every-unit-key`
    // 's environment digest. Computed at this point because it is the last
    // one where the walked graph and the resolved snapshot are both in hand,
    // and computed eagerly because the alternative — hashing on the first call
    // — bills one unlucky request for every unit digest in the program.
    // `nvs_config::cache::program_id` owns the formula and `Ctx::program_id`
    // owns what the member is allowed to do with the answer, which is nothing
    // but hand it over.
    ctx.set_program_id(
        nvs_config::cache::program_id(
            &cache::unit_digests(&checked.program_files()),
            nvs_config::cache::env_hash(&for_compiler.config),
        )
        .to_string()
        .into(),
    );
    // Hands the context the unit's class table: the class a helper's
    // bare-message failure is promoted to, and the shared ownership that lets
    // the context outlive the unit. `Unit::install_in` owns both reasons.
    unit.install_in(&mut ctx);
    // `rule:testing/in-process-request`'s unit under test, built here because this is the last
    // point at which the unit and the checked program are both in hand. A CLI
    // run has one, and it is this program: `Core\Test::request` inside a `nvs
    // run` asks the same entry a served request would, which is what lets a
    // conformance case reach the member at all.
    let under_test = runner::UnderTest::new(&unit, &checked);
    if let Some(site) = fault_inject {
        ctx.inject_fault(site.into());
    }
    // `--count`: the probe sites every compiled unit already carries count
    // instead of recording, for the length of the run. Set before the spawn
    // because the context moves into the task below, and read back off the
    // `Finished` the scheduler hands back.
    if count {
        ctx.set_debug_flags(nvs_runtime::DebugFlags::COUNT);
    }
    // `rule:concurrency/a-child-belongs-to-the-calling-task` and `rule:concurrency/limit-and-deadline-are-the-only-bounds`: the program is a *task*, because the children a
    // `Core\Task::all` inside it asks for are children of the calling task and
    // `nvs_host::spawn_child` reads that caller off the scheduler rather than
    // being told it. One task, one core, and no thread is pinned — a CLI run
    // wants the tree, not the fan-out.
    let mut sched = nvs_host::Scheduler::new();
    // The switch the script's task throws on its way out, built before either
    // task because both ends hold it — a worker polls forever, so without it
    // `run_until_idle` would never return. The workers themselves are spawned
    // *after* the script, below; `worker`'s module doc owns what that order is
    // worth to a run that never gives one a turn.
    let workers = queued.as_ref().map(|_| worker::Workers::new());
    // The call's status and the `Ctx` come back out of the task by different
    // routes. The status is written into a cell the body captures, since a
    // task's body returns nothing; the `Ctx` arrives in the `Finished` the
    // scheduler hands back, because it was moved into the task rather than
    // borrowed by it, and everything reported below is read off that one.
    let status: std::rc::Rc<std::cell::Cell<Option<Result<(), i32>>>> =
        std::rc::Rc::new(std::cell::Cell::new(None));
    // Read off the context while it is still here, because the spawn below
    // moves it into the task: the handle is what a sampler stops this run
    // through, and it carries the ceiling the sampler charges against. It is the
    // tree root's — this is the root — so one store through it reaches every
    // isolate and task the program goes on to build.
    let tree = ctx.safepoint_view();
    let root = sched.spawn(ctx, nvs_runtime::TaskRoot::Request, {
        let status = std::rc::Rc::clone(&status);
        let workers = workers.clone();
        move |ctx| {
            // The returned value is discarded: the script frame answers with
            // null.
            let outcome = entry.call(ctx).map(|_| ());
            // `Core\Script::finish()` reaches this root as a `THROWN`, because
            // the throw path is the one every `finally` lives on, and it is an
            // ordinary end all the same. Asked before anything is taken, so the
            // arm below still has the real exception to climb the ladder with;
            // `nvs_stdlib::script::is_finish` is the one home of the question.
            let finished = outcome == Err(nvs_runtime::THROWN)
                && ctx
                    .pending_class()
                    .as_deref()
                    .is_some_and(nvs_stdlib::script::is_finish);
            // `rule:concurrency/after-response-outlives-the-connection`: a CLI run has no response, so the script's own
            // frame returning is when "after the response" is —
            // `nvs_runtime::deferred` owns that reading and why a request that
            // did not return ordinarily runs none of its deferred work. It runs
            // *inside* the task, because a deferred closure is a task like any
            // other and a child of one is spawned off the caller the scheduler
            // is holding.
            if outcome.is_ok() {
                // `rule:observability/exit-hooks-run-after-the-ladder-before-teardown`: the last statement, then the queue, then
                // teardown — and *before* the deferred work below, because
                // `rule:concurrency/after-response-outlives-the-connection`'s `afterResponse` is what runs after the
                // response and this queue is what delays the end of one.
                nvs_stdlib::script::run_exit_hooks(ctx, outcome, None);
                nvs_runtime::deferred::run_deferred(ctx);
            } else if finished {
                // The fourth ending, and every tier the arm below climbs is
                // skipped by it: a script that finished has not failed, so
                // there is no uncaught handler to run, no ladder and no record.
                // What is left is exactly the arm above — the queue, then the
                // deferred work — with the marker handed on so the report names
                // `Finish` rather than `Normal`.
                let thrown = ctx.take_thrown();
                nvs_stdlib::script::run_exit_hooks(ctx, outcome, Some(&thrown));
                nvs_runtime::deferred::run_deferred(ctx);
            } else if outcome == Err(nvs_runtime::THROWN) {
                // `rule:errors/handler-script` and `rule:errors/log-write`: nothing below caught this, so the ladder
                // is climbed from tier 3 — the operator's own `.nvs`, run as an
                // isolate — and only when there is no handler, or it failed,
                // does the floor report the record itself. Zero retries, and
                // one record: what tier 3 is handed is exactly what tier 4
                // would have written.
                //
                // It happens **here, inside the task**, rather than beside the
                // exit code below, because tier 3 is an isolate and an isolate
                // wants the scheduler, the reactor and the script resolver this
                // run installed — each of which is taken down with the run
                // itself, before the exit code is decided.
                let thrown = ctx.take_thrown();
                // `rule:errors/on-uncaught-throw`'s tier 2, and this is the "request root" that
                // section names for a CLI run: every frame below returned
                // without catching, so the program's own last word comes before
                // the operator's. It is handed the real object rather than the
                // record built from it, and running it suppresses neither of
                // the tiers below — `Ctx::run_uncaught_handler` owns both
                // rules, and the record is built afterwards so that a handler
                // that faulted has already been abandoned by the time tier 3
                // is offered one.
                ctx.run_uncaught_handler(&thrown);
                let record = nvs_runtime::floor::uncaught(&thrown);
                if !nvs_host::ladder::escalate(ctx, &record) {
                    nvs_runtime::floor::report(ctx, &record);
                }
                // `rule:observability/exit-hooks-run-after-the-ladder-before-teardown`'s throw path, at its strongest reading: the
                // failure hooks run first "so a misbehaving queue cannot starve
                // the failure report", and the record above *is* that report —
                // so the queue runs after tier 3 and the floor as well as after
                // tier 2, and still before native teardown. It is handed the
                // same live `Throwable` tier 2 was.
                nvs_stdlib::script::run_exit_hooks(ctx, outcome, Some(&thrown));
            } else {
                // `exit` drains the queue and a `FATAL` runs none of it —
                // `nvs_stdlib::script::run_exit_hooks` is the one place that
                // decides which, per `rule:observability/three-endings-fire-the-exit-queue` and `rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`, so this arm asks
                // nothing about the status it is passing on.
                nvs_stdlib::script::run_exit_hooks(ctx, outcome, None);
            }
            // `rule:http-server/a-session-is-loaded-once-and-written-whole`'s write-back, and the second of its two ends — the
            // other is `nvs-host`'s isolate teardown, which is where an HTTP
            // request ends. **After the hooks**, because an exit hook is user
            // code that may still write to the session, and here inside the
            // task for the reason tier 3 above is: the send talks to the store
            // over the reactor this run installed, and the code below the spawn
            // does not run until that has been taken down.
            ctx.end_session();
            // `rule:concurrency/a-connection-is-a-root-isolate`: the connection a
            // request's `Core\Socket::upgrade` prepared starts once the request
            // has ended, over the `--peer` socket, as `nvs_server::serve_connection`
            // starts it over the real one. Only a request that ended normally
            // opens one. `peer::connect` owns what the run prints of it.
            if let Some(upgrade) = offered.as_ref().and_then(nvs_runtime::UpgradeSlot::take) {
                if outcome.is_ok() || finished {
                    peer::connect(ctx, upgrade, connection_peer.take().unwrap_or_default());
                } else {
                    upgrade.discard();
                }
            }
            // The other door, under the same rule: the event stream a request's
            // `Core\Sse::upgrade` prepared starts once the request has ended
            // normally, fed from `--events`. `events::connect` owns what the run
            // prints of it.
            if let Some(upgrade) = offered_sse.as_ref().and_then(nvs_runtime::SseSlot::take) {
                if outcome.is_ok() || finished {
                    events::connect(ctx, upgrade, stream_feed.take().unwrap_or_default());
                } else {
                    upgrade.discard();
                }
            }
            // A finish is what the run reports, not what the frame answered:
            // the marker travelled the failure path and the ending is a success.
            status.set(Some(if finished { Ok(()) } else { outcome }));
            // The script is the run, so its end is the workers' end too — and
            // it is said from inside the task because that is where the end
            // actually is: the code below this spawn does not run until the
            // scheduler is idle, which is a state a polling worker never
            // reaches.
            if let Some(workers) = &workers {
                workers.stop();
            }
        }
    });

    if let Some((feed, drain, inbox)) = publisher {
        events::start(&mut sched, feed, drain, inbox, std::rc::Rc::clone(&status));
    }

    // And the workers, after the task above rather than before it. The order is
    // a cost and not a preference: a worker's first act is a database handshake
    // and the run queue is FIFO, so a program that never parks would otherwise
    // wait one out before it could exit. `worker`'s module doc has the numbers.
    if let (Some((bounds, block)), Some(workers), Some(snapshot)) =
        (&queued, &workers, &for_workers)
    {
        worker::start(&mut sched, workers, bounds, block, snapshot);
    }

    // The reactor is what a parked task is woken by, so it is installed even
    // for a program that never parks — `run_until_idle` refuses a scheduler
    // with no reactor on its thread, and which of the two a program is is not
    // knowable from here.
    let reactor = match nvs_host::Reactor::new() {
        Ok(reactor) => reactor,
        Err(error) => {
            eprintln!("error: could not start the reactor: {error}");
            return ExitCode::FAILURE;
        }
    };
    let installed = nvs_host::reactor::install(reactor);
    // `rule:security/isolate-shares-nothing`'s isolate runs another file, and this is the only crate that can
    // turn a path into one — `script`'s module doc owns the two decisions in
    // it, and `nvs_runtime::script` owns why the edge runs this way round.
    // Installed for the whole run rather than per spawn: the unit cache behind
    // it is what makes a second isolate over one path share compiled code.
    // Held on this stack for the length of the run rather than leaked: `scoped`
    // owns why the seam's `&'static` does not oblige a `Box::leak`, and the unit
    // cache goes down with it here.
    //
    // Behind an `Arc` so the same cache also reaches a core `nvs_host` starts
    // for a `spawn script` placed `on: "worker"`, which has no stack of this
    // one's to borrow from — `nvs_runtime::script`'s *Reaching a core that
    // starts later*. One handle for the process, withdrawn below when the run
    // ends, so a source still compiles once however many cores read it.
    let compiler = std::sync::Arc::new(script::Compiler::new(&for_compiler.config));
    let shared = nvs_runtime::script::publish(nvs_runtime::script::SharedResolver::new(
        std::sync::Arc::clone(&compiler),
    ));
    // A `spawn script` resolves a path it has spawned before without looking at
    // the file, so an edit reaches a long run through this background check.
    let _watching = script::watch(&compiler, None);
    // `rule:testing/in-process-request`'s seam nests inside the resolver's for the same length and
    // on the same terms — `runner::UnderTest` owns why the program under test
    // is this crate's to hold.
    // `rule:errors/on-limit`'s CPU ceiling reaches a program that allocates
    // nothing, writes nothing and calls nothing only if a thread that is not
    // this one is charging it, and `nvs_host::watchdog` is the one thread that
    // does. A run registers for that half alone —
    // `nvs_host::Watchdog::register_requests` owns why a `nvs run` is watched
    // for its ceiling and never reported as a wedged core — and publishes the
    // one request it is: this whole run, from here until the scheduler is idle.
    //
    // Built for a run under no cap as well, because the program may narrow
    // `limits.cpu_time` with `Core\Config::set`, and the charge that ceiling is
    // compared with has to start here — `RunningRequest::new` owns that. It
    // answers `None` only for a platform with no per-thread clock, where no
    // thread is started, and the clock it is handed is this thread's because
    // this is the thread the program runs on.
    //
    // **What it spends:** one thread for the length of the run, which wakes once
    // per sweep interval and reads one word while the run is under no cap.
    // The allocator's counters are this thread's and never reset, so the
    // run's share is the difference across it. Taken here, after the reactor
    // and the compiler are up, so what is counted is the program and the
    // scheduler under it rather than the process's own start-up; the same
    // subtraction `bun nv proofs` then makes against the empty program.
    let allocated_before = count.then(|| {
        (
            nvs_runtime::budget::allocations(),
            nvs_runtime::budget::allocated_bytes(),
        )
    });
    let charged = nvs_host::RunningRequest::new(tree, nvs_host::ThreadClock::current());
    let watchdog = charged.is_some().then(nvs_host::Watchdog::new);
    let watched = watchdog.as_ref().map(|watchdog| {
        let watched = watchdog.register_requests();
        watched.publish_safepoint(charged);
        watched
    });
    let ran = nvs_runtime::script::scoped(compiler.as_ref(), || {
        nvs_runtime::inproc::scoped(&under_test, || nvs_host::run_until_idle(&mut sched))
    });
    drop(shared);
    // The run is over at this line, so the request stops being charged at it:
    // the registration goes first, and the watchdog after it joins its thread.
    // Everything below reads a context nothing is sampling any more.
    drop(watched);
    drop(watchdog);
    drop(installed);
    if let Err(error) = ran {
        eprintln!("error: the scheduler stopped: {error}");
        return ExitCode::FAILURE;
    }

    let Some(finished) = sched
        .take_finished()
        .into_iter()
        .find(|finished| finished.id == root)
    else {
        // The script's task neither finished nor was cancelled — nothing can
        // cancel it, since it is the root and the run is over.
        eprintln!("internal error: the script's task did not finish");
        return ExitCode::FAILURE;
    };
    let mut ctx = finished.ctx;

    // `--request`'s half of the finish path a server runs.
    // `Core\Response::sendFile` leaves a name on a context answering a request,
    // and a served request is where the connection opens that file and streams
    // it. This run has no connection, so the file's bytes go to standard output
    // here, on `nvs_server::serve`'s `finished` terms: only for a program that
    // ended ordinarily, since a request that threw after naming a file is sent
    // no file. A run with no request has already had the bytes written by the
    // member itself, and the name left on its context is only a record, which
    // is `Ctx::declare_file_body`'s own answer.
    if ctx.inbound().is_some()
        && let Some(path) = ctx.take_file_body()
        && status.get() == Some(Ok(()))
        && let Err(error) = send_file_body(&mut ctx, &path)
    {
        eprintln!("error: could not send {}: {error}", path.display());
        return ExitCode::FAILURE;
    }

    // Flushed before anything is reported: Rust's standard output is
    // line-buffered, and `echo "Hello, World!"` has no trailing newline.
    if let Err(error) = ctx.flush_output() {
        eprintln!("error: could not flush output: {error}");
        return ExitCode::FAILURE;
    }

    // `--count`'s one line, on stderr so stdout stays the program's. Printed
    // whatever the outcome below: a program that threw still did what it did.
    if let Some((allocations, bytes)) = allocated_before {
        let counted = ctx.counted();
        eprintln!(
            "count: statements={} calls={} allocations={} bytes={}",
            counted.statements,
            counted.calls,
            nvs_runtime::budget::allocations().wrapping_sub(allocations),
            nvs_runtime::budget::allocated_bytes().wrapping_sub(bytes),
        );
    }

    // `rule:http-server/containment-does-not-end-at-the-helper`'s outer boundary caught a panic under the task root. For a
    // request that is a failed request; for a CLI run it is this process's
    // failure, reported as the fatal it is rather than unwinding out of `main`.
    if let Err(panic) = finished.outcome {
        eprintln!("FATAL: {}", panic.message());
        return ExitCode::FAILURE;
    }

    let Some(outcome) = status.get() else {
        eprintln!("internal error: the script's task ran nothing");
        return ExitCode::FAILURE;
    };

    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        // `exit`/`exit(n)` ended the program the way it meant to, so the
        // status it named becomes this process's and nothing is reported —
        // see `nvs_runtime::EXITED`. The low byte is what a shell can carry,
        // and truncating to it is what PHP does.
        Err(status) if status == nvs_runtime::EXITED => {
            ExitCode::from(u8::try_from(ctx.exit_code() & 0xFF).unwrap_or(0))
        }
        // A throw was already reported inside the task, where the ladder could
        // reach an isolate — see the arm above `status.set`. All that is left
        // here is the status a shell reads.
        Err(status) if status == nvs_runtime::THROWN => ExitCode::FAILURE,
        Err(_) => {
            // A `FATAL`, which never reaches tiers 1 and 2 (`rule:errors/panics-bypass-user-code`) and
            // does not yet reach tier 3 either. The honest report is still the
            // message the runtime recorded; a `FATAL` has no backtrace by
            // design, so there is nothing else to say about one.
            eprintln!("FATAL: {}", ctx.take_thrown().message());
            ExitCode::FAILURE
        }
    }
}

/// Runs a tree of `.nvst` cases and turns the summary into an exit code.
///
/// Each case is run by spawning **this** binary — `nvs_test::run` documents
/// why a subprocess rather than an in-process compile — so a debug build
/// tests itself and a release build tests itself, with nothing to configure.
#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per `Command::Test` flag; a struct here would be a second spelling of that variant"
)]
fn run_test(
    paths: &[PathBuf],
    filter: Option<String>,
    php: PathBuf,
    jobs: Option<std::num::NonZeroUsize>,
    format: runner::Format,
    flags: runner::Flags,
    coverage: &coverage::Requested,
    cases: Option<PathBuf>,
    record: Option<PathBuf>,
    config: &[PathBuf],
    init: config::Init,
) -> ExitCode {
    let runner::Flags { update, list } = flags;
    // `rule:testing/nvst-is-separate`'s "`nvs test` runs both", decided by the path rather than
    // by a flag: a program is a `.nvs` file or a directory holding `.nvs` files
    // (`rule:testing/a-directory-of-programs-is-one-test-program`), a
    // conformance case is anything else, and nothing has to be spelled out at
    // the call site.
    let programs: Vec<Option<Program>> =
        match paths.iter().map(|path| Program::named_by(path)).collect() {
            Ok(programs) => programs,
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        };
    if programs.iter().any(Option::is_some) {
        let [Some(program)] = programs.as_slice() else {
            eprintln!("error: a program's `#[Test]` methods and `.nvst` cases are run separately");
            return ExitCode::FAILURE;
        };
        if cases.is_some() || record.is_some() {
            // Both name `.nvst` case files and the processes each one starts,
            // and a program's `#[Test]` methods run in this one process.
            eprintln!(
                "error: `--cases` and `--record` run `.nvst` cases, not a program's `#[Test]` methods"
            );
            return ExitCode::FAILURE;
        }
        let path = program.configured();
        // `rule:config/the-config-is-an-immutable-snapshot`'s snapshot, resolved here for the reason `run_run`
        // resolves it above its own compile: `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact key is half
        // configuration — § 7's `[opcache]` says where artifacts live and
        // whether they are read at all, and § 4's environment digest covers the
        // loaded extension set — so a suite compiled above the tree would
        // address an artifact by an environment this run is not in. A tree that
        // does not resolve stops a test run exactly as it stops a `nvs run`.
        let mut config_sources = SourceMap::new();
        let snapshot = match config::boot_snapshot(config, path, &mut config_sources, init) {
            Ok(snapshot) => snapshot,
            Err(diagnostic) => {
                let mut diags = Diagnostics::new();
                diags.report(diagnostic);
                render_diagnostics(&mut diags, &config_sources);
                return ExitCode::FAILURE;
            }
        };
        // The same install `run_run` does above, for the same reason: a suite
        // runs the program, and a `#[Test]` method reaching an origin asks the
        // anchors the tree named rather than whichever set a first call happened
        // to settle.
        if let Err(diagnostic) = config::install_tls_client(&snapshot) {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &config_sources);
            return ExitCode::FAILURE;
        }
        // `--filter` reaches both suites, and means the same thing in each:
        // `runner::selected` owns the rule and why it is the `.nvst` tree's.
        return match program.front_end() {
            Ok(checked) => runner::run(checked, &snapshot, format, filter, flags, coverage),
            Err(code) => code,
        };
    }
    if list {
        // A `.nvst` tree is discovered by walking directories, which is what
        // the caller already did to name it, and a case file has no `#[Test]`
        // table to locate. Refused rather than ignored, for `--update`'s reason
        // just below.
        eprintln!("error: `--list` locates a program's `#[Test]` methods, not a `.nvst` tree");
        return ExitCode::FAILURE;
    }
    if update {
        // § 14's updater rewrites a `#[Test]` method's own snapshot literal,
        // and a `.nvst` case has none: its expectation is the `--EXPECT--`
        // section, which `nvs_test` compares whole and which nothing here
        // writes. Refused rather than ignored, for `--format`'s reason below.
        eprintln!("error: `--update` rewrites a program's inline snapshots, not a `.nvst` tree");
        return ExitCode::FAILURE;
    }
    if format != runner::Format::Human {
        // § 22's formats report a `#[Test]` run, and § 23 keeps the two suites
        // from sharing a summary — so a machine format over a `.nvst` tree
        // names a document this subcommand does not produce.
        eprintln!("error: `--format` reports a program's `#[Test]` methods, not a `.nvst` tree");
        return ExitCode::FAILURE;
    }
    if coverage.any() {
        // The counts come from the probes of the one program this process
        // compiles, and a `.nvst` case runs as a process of its own.
        eprintln!(
            "error: `--coverage-lcov` and `--coverage-clover` measure a program's `#[Test]` methods, not a `.nvst` tree"
        );
        return ExitCode::FAILURE;
    }

    let mut options = match nvs_test::Options::from_current_exe() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: could not locate this binary to run cases with: {error}");
            return ExitCode::FAILURE;
        }
    };
    options.filter = filter;
    options.php = php;
    if let Some(jobs) = jobs {
        options.jobs = jobs.get();
    }
    if let Some(list) = &cases {
        match std::fs::read_to_string(list) {
            Ok(text) => {
                options.only = Some(
                    text.lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(PathBuf::from)
                        .collect(),
                );
            }
            Err(error) => {
                eprintln!(
                    "error: could not read the case list {}: {error}",
                    list.display()
                );
                return ExitCode::FAILURE;
            }
        }
    }
    if let Some(dir) = &record {
        // Absolute, because each case's processes run in a working directory of their own.
        let made = std::fs::create_dir_all(dir).and_then(|()| std::path::absolute(dir));
        match made {
            Ok(dir) => options.record = Some(dir),
            Err(error) => {
                eprintln!(
                    "error: could not use {} to record into: {error}",
                    dir.display()
                );
                return ExitCode::FAILURE;
            }
        }
    }

    let mut out = std::io::stdout().lock();
    match nvs_test::run(paths, &options, &mut out) {
        Ok(summary) if summary.is_success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// What `nvs test` compiles when a path names a **program** rather than a
/// `.nvst` case tree.
///
/// The extension is the whole test for a file, and `.nvs` is the only one that
/// passes it. A path Novis will happily compile — the extension is a
/// convention rather than a rule everywhere else — is read as a case tree
/// here, because `nvs test` has to choose one of two suites from the path
/// alone and a guess that looked inside the file would make the choice
/// unpredictable. A directory is read the same way, off the extensions of the
/// files under it.
enum Program {
    /// A `.nvs` file: the program it starts.
    File(PathBuf),
    /// A directory holding `.nvs` files: one program whose entry requires
    /// every one of them, in name order
    /// (`rule:testing/a-directory-of-programs-is-one-test-program`).
    Directory { dir: PathBuf, files: Vec<PathBuf> },
}

impl Program {
    /// The program `path` names, or `None` for a `.nvst` case file or a
    /// directory holding no `.nvs` file.
    ///
    /// # Errors
    ///
    /// A directory that cannot be read, and one holding both kinds of file:
    /// the two suites report differently and share no summary, so a tree that
    /// mixes them is run in two invocations rather than guessed at.
    fn named_by(path: &std::path::Path) -> Result<Option<Self>, String> {
        if is_program(path) {
            return Ok(Some(Self::File(path.to_path_buf())));
        }
        if !path.is_dir() {
            return Ok(None);
        }
        let mut files = Vec::new();
        let mut cases = false;
        programs_under(path, &mut files, &mut cases)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if files.is_empty() {
            return Ok(None);
        }
        if cases {
            return Err(format!(
                "{}: a directory holding both `.nvs` programs and `.nvst` cases is run separately, one kind per invocation",
                path.display()
            ));
        }
        Ok(Some(Self::Directory {
            dir: path.to_path_buf(),
            files,
        }))
    }

    /// The path the configuration is folded for: the file, or the directory,
    /// which is what an `[[app]]` block's `root` covers.
    fn configured(&self) -> &std::path::Path {
        match self {
            Self::File(path) | Self::Directory { dir: path, .. } => path,
        }
    }

    /// The checked program.
    fn front_end(&self) -> Result<Checked, ExitCode> {
        match self {
            Self::File(path) => front_end(path),
            Self::Directory { dir, files } => {
                // A name no file under the directory carries, so the overlay
                // never stands in front of a real file's bytes.
                let mut name = String::from("#tests.nvs");
                while files
                    .iter()
                    .any(|file| file.file_name().is_some_and(|f| f == name.as_str()))
                {
                    name.insert(0, '#');
                }
                front_end_synthesized(&dir.join(name), &directory_entry(dir, files))
            }
        }
    }
}

/// Whether `path` carries the `.nvs` extension.
fn is_program(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nvs"))
}

/// Collects every `.nvs` file under `dir`, directories walked in name order,
/// and notes in `cases` whether a `.nvst` case sits among them.
fn programs_under(
    dir: &std::path::Path,
    into: &mut Vec<PathBuf>,
    cases: &mut bool,
) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()?;
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            programs_under(&entry, into, cases)?;
        } else if is_program(&entry) {
            into.push(entry);
        } else if entry.extension().is_some_and(|ext| ext == "nvst") {
            *cases = true;
        }
    }
    Ok(())
}

/// The entry a directory of programs is run through: one `require` per file,
/// in the order [`programs_under`] found them, each path relative to `dir` and
/// spelled with `/`, which a literal path accepts on every platform.
///
/// Under `nvs test` the entry's statements never run, so the requires only
/// bring each file into the compile; a file in the directory that requires
/// the application's bootstrap is what gives the whole directory its
/// `autoload` map.
fn directory_entry(dir: &std::path::Path, files: &[PathBuf]) -> String {
    let mut text = String::from("<?nvs\n");
    for file in files {
        let relative = file.strip_prefix(dir).unwrap_or(file);
        let spelled: Vec<String> = relative
            .components()
            .map(|part| {
                part.as_os_str()
                    .to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
            })
            .collect();
        text.push_str("require '");
        text.push_str(&spelled.join("/"));
        text.push_str("';\n");
    }
    text
}

/// Where a front end's diagnostics go, and in which rendering.
///
/// The command chooses one for the whole run, which is why no call site names
/// a rendering: two of them in one run would put a document and a rendered
/// snippet on the same standard output.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sink {
    /// The terminal rendering, on standard error. Every command's default.
    Text,
    /// `nvs check --json`'s document, on standard output.
    Json,
    /// `nvs agent hook`'s report: the errors alone, kept for the hook to give
    /// back to the agent in its tool's JSON ([`agent::keep_errors`]).
    Hook,
}

/// Renders `diags` into the sink in force.
///
/// The text half prints nothing when there is nothing to say; the JSON half
/// always prints a document, because a consumer parsing standard output would
/// read an empty one as a crash.
fn emit_diagnostics(diags: &mut Diagnostics, map: &SourceMap, sink: Sink) {
    match sink {
        Sink::Text => render_diagnostics(diags, map),
        Sink::Json => {
            diags.sort_by_position();
            println!("{}", check::document(diags.iter(), map));
        }
        Sink::Hook => agent::keep_errors(diags, map),
    }
}

fn render_diagnostics(diags: &mut Diagnostics, map: &SourceMap) {
    if diags.is_empty() {
        return;
    }
    diags.sort_by_position();
    // A Windows console shows escape sequences as text until it is told to
    // interpret them, so the test is whether standard error will, not only
    // whether it is a terminal.
    let renderer = Renderer::new().with_color(
        std::env::var_os("NO_COLOR").is_none()
            && nvs_runtime::terminal::interprets_escapes(nvs_runtime::terminal::Stream::Err),
    );
    let mut out = Vec::new();
    renderer
        .render_all(diags.iter(), map, &mut out)
        .expect("rendering to an in-memory buffer cannot fail");
    eprint!("{}", String::from_utf8_lossy(&out));
}

/// Reports a source file [`SourceMap::load`] could not read, into `sink`.
///
/// A file that is not UTF-8 is `E0006`, pointed at its first byte that is not,
/// in a map of its own that holds the file's text with each such byte replaced
/// by U+FFFD — the caller's map never sees that text. Every other read error,
/// and a file too large to load, is the uncoded `could not read` line.
fn report_unreadable(path: &std::path::Path, err: &std::io::Error, sink: Sink) {
    let fits = |len: usize| len <= nvs_diagnostics::MAX_SOURCE_LEN;
    if err.kind() == std::io::ErrorKind::InvalidData
        && std::fs::metadata(path).is_ok_and(|meta| usize::try_from(meta.len()).is_ok_and(fits))
        && let Ok(bytes) = std::fs::read(path)
        && let Err(bad) = std::str::from_utf8(&bytes)
        && let text = String::from_utf8_lossy(&bytes).into_owned()
        && fits(text.len())
        && let Ok(at) = u32::try_from(bad.valid_up_to())
    {
        let mut map = SourceMap::new();
        let id = map.add(path.display().to_string(), text);
        // U+FFFD, the character the bad byte was replaced by, is three bytes.
        let replacement = 3;
        let mut diags = Diagnostics::new();
        diags.report(
            nvs_diagnostics::Diagnostic::error(
                nvs_diagnostics::code::E_INVALID_UTF8,
                format!("`{}` is not valid UTF-8", path.display()),
            )
            .with_primary(
                nvs_diagnostics::Span::new(id, at, at + replacement),
                "this byte is not UTF-8",
            ),
        );
        emit_diagnostics(&mut diags, &map, sink);
        return;
    }
    eprintln!("error: could not read {}: {err}", path.display());
}

/// Renders an internal compiler error on standard error as `E0901`, with
/// `message` as its text.
///
/// Every [`nvs_codegen::CodegenError`] comes here, `Unsupported` among them: a
/// construct the checker accepted and the compiler cannot lower is a bug in
/// Novis, never in the program, and `E0901` is the one code for that.
fn report_internal(message: impl Into<String>) {
    render_diagnostics(&mut internal_error(message), &SourceMap::new());
}

/// [`report_internal`]'s diagnostic, which has no span because the program's
/// source is not where the fault is.
fn internal_error(message: impl Into<String>) -> Diagnostics {
    let mut diags = Diagnostics::new();
    diags.report(nvs_diagnostics::Diagnostic::error(
        nvs_diagnostics::code::E_INTERNAL,
        message,
    ));
    diags
}

#[cfg(test)]
mod tests {
    use super::{
        Cli, Command, SERVICE_INSTALL_EXAMPLE, ServiceCommand, inbound_of, initializes, unaccepted,
    };
    use clap::{CommandFactory as _, Parser as _};

    /// A command line as the words a shell would hand over: split on spaces,
    /// with a double-quoted run kept as one word.
    fn words(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut word = String::new();
        let mut quoted = false;
        for character in line.chars() {
            match character {
                '"' => quoted = !quoted,
                ' ' if !quoted => {
                    if !word.is_empty() {
                        out.push(std::mem::take(&mut word));
                    }
                }
                other => word.push(other),
            }
        }
        if !word.is_empty() {
            out.push(word);
        }
        out
    }

    /// A stored argv this binary would answer with a usage error is refused at
    /// install, because a service manager starts it with no console to write
    /// that error to. `serve` with no entry file is a command line this binary
    /// takes, and whether it has anything to serve is the installer's question
    /// of the configuration.
    #[test]
    fn an_argv_this_binary_would_not_run_is_named_as_unaccepted() {
        let argv = |line: &str| words(line);
        let why = unaccepted(&argv("run --config /srv/shop/nvs.toml")).expect("no entry file");
        assert!(why.contains("<FILE>"), "{why}");
        assert!(!why.contains("Usage"), "{why}");
        assert_eq!(unaccepted(&argv("serve --config /srv/shop/nvs.toml")), None);
        assert_eq!(
            unaccepted(&argv("serve --help")).as_deref(),
            Some("it prints help and exits")
        );
        assert_eq!(
            unaccepted(&argv(
                "serve /srv/shop/index.nvs --config /srv/shop/nvs.toml"
            )),
            None
        );
    }

    /// Every command line the help shows is one the parser takes, and the argv
    /// it stores is one this binary runs — so the example cannot drift from the
    /// options it is an example of.
    #[test]
    fn every_command_line_in_the_service_install_help_parses_and_stores_an_accepted_argv() {
        // An example runs from its `nvs service install` line to the next blank
        // one, and is one command line once those are joined.
        let lines: Vec<String> = SERVICE_INSTALL_EXAMPLE
            .split("\n\n")
            .filter(|block| block.trim_start().starts_with("nvs service install "))
            .map(|block| block.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        assert_eq!(
            lines.len(),
            3,
            "one for each platform, and one that names no entry file"
        );
        for line in &lines {
            let cli =
                Cli::try_parse_from(words(line)).unwrap_or_else(|error| panic!("{line}: {error}"));
            let Some(Command::Service {
                command: ServiceCommand::Install(args),
            }) = cli.command
            else {
                panic!("{line} is not `nvs service install`")
            };
            assert!(["shop", "sites"].contains(&args.name.as_str()), "{line}");
            assert_eq!(args.argv[0], "serve");
            assert_eq!(unaccepted(&args.argv), None, "{line}");
        }
    }

    /// The half of [`initializes`]'s table that writes nothing, read through the parser so the
    /// case names the command line rather than a variant: `nvs config check` and `nvs config dump`
    /// audit a tree, and an audit that creates the file it is auditing reports on its own output;
    /// `nvs lsp` is started by an editor in every folder it opens, and one that wrote into each of
    /// them is a defect rather than a convenience.
    ///
    /// The project commands are asserted beside them, because the decision this pins is the
    /// **split** — a table that silently lost a row would pass a case that only checked one side.
    #[test]
    fn config_check_and_dump_and_lsp_never_write() {
        let writes = |argv: &[&str]| {
            let cli = Cli::try_parse_from(argv).expect("the fixture is a command line `nvs` takes");
            initializes(&cli.command.expect("a subcommand was named"))
        };

        assert!(!writes(&["nvs", "config", "check"]));
        assert!(!writes(&["nvs", "config", "dump"]));
        assert!(!writes(&["nvs", "lsp"]));
        // The explicit door writes the file itself and resolves no tree, so it reaches no step 3
        // to be a project command at.
        assert!(!writes(&["nvs", "init"]));

        assert!(writes(&["nvs", "run", "app.nvs"]));
        assert!(writes(&["nvs", "serve", "app.nvs"]));
        assert!(writes(&["nvs", "test", "app.nvs"]));
        assert!(writes(&["nvs", "build", "--openapi", "app.nvs"]));
        assert!(writes(&["nvs", "check", "app.nvs"]));
    }

    /// One field line off the carrier, as text, so the assertions below read as
    /// the case's own lines.
    fn field(inbound: &nvs_runtime::Inbound, name: &str) -> Option<String> {
        inbound
            .headers()
            .find(|(field, _)| *field == name)
            .map(|(_, value)| String::from_utf8_lossy(value).into_owned())
    }

    /// A case's request sections become a carrier through
    /// [`nvs_runtime::InboundSpec`], and every section it wrote is answerable
    /// off the result.
    ///
    /// The whole crossing in one call, because this is the only place it is
    /// visible: `nvs_test::case::parse` reads the sections, `request::render`
    /// writes the file a runner would leave beside the case, and
    /// [`inbound_of`] builds it. It is asked here rather than in `nvs-test`,
    /// which has no dependencies on purpose and so can never name the builder,
    /// and rather than through the built binary, which would report a program's
    /// output instead of the carrier's fields.
    ///
    /// The two the file *states* — the peer's address and the scheme — are
    /// asserted beside the two the body *derives*, since a builder that filled
    /// a carrier in itself would have had to derive the second pair a second
    /// time and could disagree with the renderer about it.
    #[test]
    fn the_request_sections_build_an_inbound_spec() {
        let case = nvs_test::case::parse(
            std::path::Path::new("sections.nvst"),
            "--TEST--\nevery request section at once\n--FILE--\n<?nvs\necho 1;\n\
             --GET--\nq=novis\n--POST--\nname=ada\n--COOKIE--\nsid=abc123\n\
             --HEADERS--\nAccept: application/json\n--CLIENT_IP--\n203.0.113.9\n\
             --SCHEME--\nhttps\n--EXPECT--\n1\n",
        )
        .expect("the case parses");
        let request = case.request.as_ref().expect("the case describes a request");
        let inbound = inbound_of(&nvs_test::request::render(request)).expect("the file reads back");

        assert_eq!(inbound.method(), "POST", "a case with a body sends one");
        assert_eq!(inbound.path(), "/");
        assert_eq!(inbound.query(), "q=novis", "`--GET--` is the query string");
        assert_eq!(
            field(&inbound, "accept").as_deref(),
            Some("application/json"),
            "`--HEADERS--` arrives lower-cased, which is how a served request carries a field"
        );
        assert_eq!(
            field(&inbound, "cookie").as_deref(),
            Some("sid=abc123"),
            "`--COOKIE--` is what the case describes and one field line is how it travels"
        );
        assert_eq!(
            field(&inbound, "content-type").as_deref(),
            Some("application/x-www-form-urlencoded"),
            "`--POST--` says what it encodes to by being pairs"
        );
        assert_eq!(
            field(&inbound, "content-length").as_deref(),
            Some("8"),
            "and the octets say how many of them there are"
        );
        assert!(
            inbound.has_body(),
            "the body reaches the carrier as a body, not as a field line"
        );
        assert_eq!(
            inbound.client(),
            Some("203.0.113.9".parse().expect("a literal address")),
            "`--CLIENT_IP--` is the address the walk settled on, stated rather than derived"
        );
        assert_eq!(
            inbound.scheme(),
            nvs_runtime::Scheme::Https,
            "`--SCHEME--` is a claim only a section can make"
        );
        assert_eq!(inbound.mount_prefix(), "", "a case names no mount");
        assert!(inbound.mount_captures().is_empty());
    }

    /// A file naming a mount reaches the carrier with the prefix and the
    /// captures a served request's door would have written, which is what
    /// `Core\Request::mount()` reads.
    #[test]
    fn a_request_file_naming_a_mount_carries_it_to_the_inbound() {
        let inbound = inbound_of("--METHOD--\nGET\n--PATH--\n/orders\n--MOUNT--\n/shop\nacme\n")
            .expect("the file reads");
        assert_eq!(inbound.mount_prefix(), "/shop");
        assert_eq!(
            inbound
                .mount_captures()
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>(),
            ["acme"]
        );
        assert_eq!(inbound.path(), "/orders", "the path stays the stripped one");
    }

    /// Help text with every run of whitespace collapsed to one space.
    ///
    /// `clap`'s `wrap_help` breaks a line wherever the terminal width falls, so
    /// a token asserted against the rendering as written would fail for the
    /// width of the box it was rendered in rather than for the sentence.
    fn one_line(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// `serve`'s help names all three subsystems the one process runs, in the
    /// listing line and in the command's own `--help` alike.
    ///
    /// `rule:concurrency/one-process-serves-requests-schedules-and-jobs` is what
    /// has to be readable here: an operator who reads the accept loop alone goes
    /// looking for a second thing to install for their schedules and their jobs,
    /// and there is nothing to find. Both renderings are asserted because they
    /// are read at different moments — the listing line is what `nvs --help`
    /// prints before anyone has decided this is the command, and the long help
    /// only afterwards, so a paragraph naming the three under a first line that
    /// does not is still the wrong sentence in the place it is read.
    #[test]
    fn the_serve_help_names_requests_schedules_and_queue_workers() {
        /// One subsystem, and the word the help has to carry for it.
        const NAMED: [(&str, &str); 4] = [
            ("the accept loop", "requests"),
            ("the ticker", "[[schedule]]"),
            ("the queue the workers drain", "[queue]"),
            ("the tasks that drain it", "workers"),
        ];

        let mut cli = Cli::command();
        let serve = cli
            .find_subcommand_mut("serve")
            .expect("`nvs serve` is a subcommand");
        let listing = one_line(
            &serve
                .get_about()
                .expect("the subcommand is listed with a description")
                .to_string(),
        );
        let long = one_line(&serve.render_long_help().to_string());

        for (rendering, text) in [("the listing line", &listing), ("`serve --help`", &long)] {
            for (subsystem, word) in NAMED {
                assert!(
                    text.contains(word),
                    "{rendering} does not name {subsystem}: no `{word}` in {text:?}"
                );
            }
        }
    }

    /// The command is spelled `serve`, and `service` is the platform's
    /// service-manager namespace beside it rather than a second name for it.
    ///
    /// `docs/decisions/0154.md` § 6 settles the question the sentence above
    /// raises: naming three subsystems is what a rename would have been asked to
    /// buy, and it buys it for free. `service` is already taken by
    /// `rule:packaging/a-service-is-one-stored-argv`, whose argv is written into
    /// units on disk, so a rename into it would invalidate deployments as well
    /// as collide. The two are told apart by shape rather than by name here: one
    /// takes the file every request runs, the other takes a verb.
    #[test]
    fn the_command_is_still_spelled_serve_and_service_is_still_the_other_namespace() {
        let cli = Cli::command();
        let names: Vec<&str> = cli.get_subcommands().map(clap::Command::get_name).collect();

        assert!(
            names.contains(&"serve"),
            "the command is `nvs serve` and nothing else: {names:?}"
        );
        assert!(
            !names.contains(&"daemon"),
            "`daemon` is a noun in a list of verbs: {names:?}"
        );

        let serve = cli.find_subcommand("serve").expect("`nvs serve` is listed");
        assert!(
            serve.get_positionals().any(|arg| arg.get_id() == "file"),
            "`serve` takes the file every request runs"
        );
        assert!(
            serve.get_subcommands().next().is_none(),
            "`serve` is the whole command, not a namespace of verbs"
        );

        let service = cli
            .find_subcommand("service")
            .expect("`nvs service` is the platform service manager's namespace");
        assert!(
            service.get_subcommands().next().is_some(),
            "`service` is a namespace, so the work is in its verbs"
        );
        assert!(
            service.get_positionals().all(|arg| arg.get_id() != "file"),
            "`service` acts on a registered server, never on a file to serve"
        );
    }

    /// An internal compiler error renders as `E0901` with the error's text, and
    /// ends with the `nvs agent show` line every coded diagnostic ends with.
    #[test]
    fn an_internal_compiler_error_is_e0901() {
        let diags = super::internal_error("the script frame was not compiled");
        let mut out = Vec::new();
        nvs_diagnostics::Renderer::new()
            .with_color(false)
            .render_all(diags.iter(), &nvs_diagnostics::SourceMap::new(), &mut out)
            .expect("rendering to an in-memory buffer cannot fail");
        let text = String::from_utf8(out).expect("the rendering is UTF-8");
        assert!(
            text.starts_with("error[E0901]: the script frame was not compiled\n"),
            "{text}"
        );
        assert!(text.contains(nvs_diagnostics::AGENT_SHOW_LINE), "{text}");
    }
}
