//! The `nvs` binary.
//!
//! The subcommands:
//!
//! * `nvs ast` — dump what the parser produced, as Rust's `Debug` for a
//!   person or as `rule:ide/ast-json-schema-is-frozen`'s JSON for the AST
//!   panel that shells out to it. Resilient by default, since the file a panel
//!   is opened for is the one that does not compile; see [`ast`].
//! * `nvs check` — parse, resolve, type-check, report every diagnostic.
//!   `--autoload-map` prints the resolved `autoload` map in place of the
//!   success line, which is
//!   `rule:programs/autoload`'s last sentence; the shape is `nvs_hir::autoload`'s module doc.
//!   `--json` prints those same diagnostics as
//!   `rule:ide/check-json-is-the-diagnostic-record-as-a-document`'s document
//!   instead of rendering them for a terminal, for CI and for the agents that
//!   drive this compiler; see [`check`].
//! * `nvs run` — all of the above, then compile and execute. Its dump
//!   flags stop one stage earlier and print instead of running:
//!   `--dump-ir` after lowering, `--dump-asm` after code generation.
//! * `nvs test` — run a tree of `.nvst` conformance cases, or a program's
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
//! * `nvs build --compile` —
//!   `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s
//!   portable single-file executable: the program's `require` graph as plain
//!   source, appended to a copy of this binary. Also the one subcommand this
//!   binary can *be*: a copy carrying that payload runs it instead of parsing
//!   arguments at all; see [`bundle`].
//! * `nvs api diff <old.json> <new.json>` — the same rule's § 4 gate over two
//!   finished documents: breaking, additive or cosmetic per change, non-zero
//!   exit on a breaking one. It compiles nothing, because the old side of a
//!   diff is a released artifact rather than a program; see [`api_diff`].
//! * `nvs pkg api <dir>` — a package's public API as JSON, read from the
//!   checked program and never by running it
//!   (`rule:packaging/only-a-major-may-break-the-api`); see [`package`].
//! * `nvs config check` — resolve the configuration tree and report what
//!   it holds, exiting non-zero on any refusal.
//!   `rule:config/check-and-dump-audit-the-tree-offline`
//!   's offline audit, which exists because `rule:config/later-wins-and-every-override-is-recorded`'s later-wins precedence is
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
//! * `nvs cache gc|clear` —
//!   `rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape`'s two commands over the
//!   artifact cache a run reads: § 6's eviction walk without its roll, and every entry removed;
//!   see [`cache_cmd`].
//! * `nvs update-check` — `rule:tooling/update-check-states-the-whole-picture`: the telemetry
//!   switch, this version and the latest release GitHub lists, one line per step or `--json`,
//!   exit `0` always; see [`update_check`], and [`endpoints`] for the URLs it reads.
//! * `nvs telemetry on|off|status|reset|show` — `rule:tooling/telemetry-is-two-opt-ins`'s
//!   consent and the closed counter set every invocation adds to; see [`telemetry`], and
//!   [`COMMAND_NAMES`] for the table a counter is named from.
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
//! * `nvs dap` — the debug adapter on stdio, which launches a program in its own process and
//!   never attaches (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`); see [`dap`].
//! * `nvs debug` — the debug service for one data folder, which ingests the spool a run with
//!   `[debug] store` on wrote into `debug/index.sqlite`
//!   (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`); see [`debug`].
//!
//! `run` **checks first**: on any diagnostic it reports and exits non-zero
//! exactly as `check` does, rather than running a program the front end
//! rejected.
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
use clap::{CommandFactory, FromArgMatches, Parser as ClapParser, Subcommand};
use nvs_diagnostics::{Diagnostics, Renderer, SourceMap};
use nvs_syntax::{check_declarations, parse_file};

mod agent;
mod api_diff;
mod ast;
mod build;
mod bundle;
mod cache;
mod cache_cmd;
mod check;
mod code;
mod config;
mod coverage;
mod dap;
mod debug;
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
mod endpoints;
mod events;
mod ext;
mod extensions;
mod fmt;
mod info;
mod init;
mod lsp;
mod lsp_test;
mod meta;
mod openapi;
mod package;
mod peer;
mod profile;
mod queue;
mod reload;
mod run;
mod runner;
mod schema;
mod script;
mod serve;
mod service;
mod shared;
mod stop;
mod stubs;
mod telemetry;
#[cfg(test)]
mod testing;
mod tmp;
mod update_check;
mod viewer;
mod wall;
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
        "{}Novis-Lang{}\nversion: {} · commit: {} · {}",
        name.render(),
        name.render_reset(),
        env!("NVS_VERSION"),
        env!("NVS_COMMIT"),
        env!("NVS_COMMIT_DATE"),
    )
}

/// What `nvs --version` prints after the name: the version, `+dev` after it on a
/// build that is not a release, then the commit and its date as [`header`] prints
/// them, because a bug report and an upgrade check both need the commit and
/// `--version` is where people look for it.
const VERSION: &str = concat!(
    env!("NVS_VERSION"),
    " (commit ",
    env!("NVS_COMMIT"),
    ", ",
    env!("NVS_COMMIT_DATE"),
    ")"
);

/// The variable that sets Cranelift's optimisation level for every unit of this process, `none` or
/// `speed`. `nvs test --opt-level` sets it for each process a `.nvst` case starts
/// (`rule:testing/probes-on-is-a-tested-configuration`).
pub(crate) const OPT_LEVEL: &str = "NOVIS_OPT_LEVEL";

/// The variable that compiles every unit of this process with a type test after each typed read
/// (`nvs_codegen::compile_checked_types`). It works on presence, as `NOVIS_NO_FILE_CACHE` does.
const CHECK_TYPES: &str = "NOVIS_CHECK_TYPES";

/// The variable that turns every probe site of this process on and records nothing
/// (`nvs_runtime::probe_every_site`). It works on presence, and `nvs test --probes` sets it for
/// each process a `.nvst` case starts (`rule:testing/probes-on-is-a-tested-configuration`).
const PROBES: &str = "NOVIS_PROBES";

/// The environment variables `nvs` reads, which `nvs --help` lists below the options.
const ENVIRONMENT: &str = "\
Environment:
  NOVIS_NO_INIT        Do not create `nvs.toml` in the data folder, as `--no-init` does.
  NOVIS_NO_FILE_CACHE  Read and write no compiled artifact, so every program is compiled again.
  NOVIS_OPT_LEVEL      The optimisation level: `none` or `speed`. The default is `speed`.
                       `nvs test --opt-level` sets it for the programs a `.nvst` case runs.
  NOVIS_CHECK_TYPES    Test each value a program reads against its declared type, and stop with
                       an error at the first value that does not match. A value like that is a
                       bug in Novis. Reads and writes no compiled artifact.
  NOVIS_PROBES         Run every debug probe of a program, and record nothing.
                       `nvs test --probes` sets it for the programs a `.nvst` case runs.
  NVS_UPDATE_URL       The address `nvs update-check` reads the latest release from. The default
                       is the GitHub Releases API of the Novis repository.
  NVS_TELEMETRY_URL    The address telemetry is uploaded to. If it is not set, nothing is uploaded.
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

    /// Reads this configuration file. Repeat the option to read several files,
    /// in order.
    ///
    /// Without this option, `nvs` reads `./nvs.toml`. If there is no
    /// `./nvs.toml`, it reads `nvs.toml` in the data folder (see `--data`). If
    /// there is neither, it uses the default settings.
    ///
    /// When you name a file, `nvs` does not read either `nvs.toml`. A relative
    /// path starts at the current folder. A file that does not exist is an
    /// error.
    ///
    /// Every subcommand accepts this option.
    // Every `///` line above is printed under each subcommand's `--help`, so it
    // is written as `AGENTS.md` § *Text an end user reads* asks. Naming any file
    // disables the search, so a named tree is never merged with whatever is in
    // the working directory or the data folder. It is global because it selects
    // the tree rather than the command: `run` resolves the snapshot its request
    // reads, and `config check`/`config dump` audit the same files without
    // running anything.
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
    // step 1, and `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` is the resolution against the working directory.
    #[arg(long, value_name = "PATH", global = true)]
    config: Vec<PathBuf>,

    /// Does not create `nvs.toml` in the data folder.
    ///
    /// Without this option, `run`, `serve`, `test`, `build`, `check` and `dap`
    /// create `nvs.toml` in the data folder when they find no configuration
    /// file. The
    /// file contains the default settings. With this option, no file is
    /// created, and the command uses the same default settings. Setting
    /// `NOVIS_NO_INIT` in the environment does the same.
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
    // step 4's write; see [`config::init_gate`]. It skips the file only: the
    // data folder itself is still created.
    #[arg(long, global = true)]
    no_init: bool,

    /// The folder where Novis keeps its own files: the default `nvs.toml`, the
    /// compile cache, temporary files and editor files. The default is
    /// `.nvsdata` beside the `nvs` program.
    ///
    /// A relative path starts at the current folder. If `nvs` cannot use the
    /// folder, it prints a warning and runs without it.
    ///
    /// Every subcommand accepts this option.
    // Read once, by [`set_data_folder`], before any command runs: the data
    // folder is one value per process (`nvs_config::data`).
    #[arg(long, value_name = "PATH", global = true)]
    data: Option<PathBuf>,
}

/// Names this process's data folder from `--data`, before anything reads it.
///
/// A relative path resolves against the working directory, like every other
/// path on the command line. With no flag the folder is `.nvsdata` beside the
/// binary, and a binary whose own path cannot be read sets nothing, which
/// leaves `nvs_config::data::current` to report that it has none.
fn set_data_folder(flag: Option<&std::path::Path>) {
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Ok(folder) = nvs_config::data::Folder::locate(flag, &cwd) {
        // `false` only when the folder was already set or read, and this runs
        // before either can happen.
        let _ = nvs_config::data::set(folder);
    }
}

/// The commands that use the data folder, and so create it on startup: the
/// project commands in [`initializes`], `lsp`, which keeps its editor
/// files there, and `debug`, which keeps its index there. Every other command
/// only reads what is already there, so an audit or a status query never
/// creates a folder.
fn prepares(command: &Command) -> bool {
    initializes(command) || matches!(command, Command::Lsp(_) | Command::Debug(_))
}

/// Creates the data folder for a command in [`prepares`], or prints the one
/// warning that says why it cannot be used. Never stops the command: without
/// the folder it runs with no compile cache and on the shipped defaults.
fn prepare_data_folder() {
    if let Err(unusable) = nvs_config::data::prepare() {
        eprintln!("{}", unusable.warning());
    }
}

/// Makes the data folder's `logs/nvs.log` the `[log] target` of every tree this
/// process resolves that sets none — the default for a service.
///
/// With no usable data folder the target stays `stderr`, and
/// [`prepare_data_folder`] has already printed the one warning that says why.
/// A target written in the configuration always wins: `nvs_config::log`'s
/// module doc owns that check.
fn log_like_a_service() {
    if let Some(folder) = nvs_config::data::current() {
        // `false` only when a default was already set, and this runs once.
        let _ = nvs_config::log::serve_into(&folder.service_log());
    }
}

/// Whether a service manager started this process: systemd's `$NOTIFY_SOCKET`
/// on Unix, which is also where `service::Notify::from_env` reports, and the
/// SCM on Windows, known only once `dispatch::serving` runs the server. That
/// function is compiled on Windows alone, so the name is not a doc link: on
/// every other target it would have nothing to resolve to.
fn started_by_a_manager() -> bool {
    #[cfg(unix)]
    {
        std::env::var_os("NOTIFY_SOCKET").is_some_and(|socket| !socket.is_empty())
    }
    #[cfg(windows)]
    {
        dispatch::hosted()
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// The project commands, which are the ones that write the shipped default file into the data
/// folder when they resolve no tree: the commands that read a configuration **in order to execute
/// something**.
///
/// Everything else writes no configuration file. An audit that creates the file it is auditing
/// reports on its own output, and `nvs lsp` resolves a tree per document only to type extension
/// calls. This table is the whole of that decision, so a sixth command joins it here with a
/// sentence rather than by threading a flag through an arm.
///
/// `nvs build` resolves no tree today and so reaches nothing to write; it is named because the
/// answer for it is decided, not because it currently does anything. `nvs dap` is here because
/// the program it launches resolves its tree as `nvs run` would.
fn initializes(command: &Command) -> bool {
    matches!(
        command,
        Command::Run(_)
            | Command::Serve(_)
            | Command::Test(_)
            | Command::Build(_)
            | Command::Check(_)
            | Command::Dap(_)
    )
}

/// Whether `command` may carry a telemetry upload (`rule:tooling/a-serving-process-never-uploads`):
/// only `nvs update-check` and `nvs telemetry upload`. Every variant is named, so a new command
/// does not compile until it answers, and a command that serves or stays running answers `false`.
fn carries_upload(command: &Command) -> bool {
    match command {
        Command::UpdateCheck(_) => true,
        Command::Telemetry(args) => args.uploads(),
        Command::Ast(_)
        | Command::Check(_)
        | Command::Run(_)
        | Command::Serve(_)
        | Command::Test(_)
        | Command::Build(_)
        | Command::Api(_)
        | Command::Pkg(_)
        | Command::Ext(_)
        | Command::Init(_)
        | Command::Config(_)
        | Command::Queue(_)
        | Command::Schema(_)
        | Command::Tmp(_)
        | Command::Cache(_)
        | Command::Service(_)
        | Command::InstallService(_)
        | Command::Fmt(_)
        | Command::Lsp(_)
        | Command::LspTest(_)
        | Command::Dap(_)
        | Command::Debug(_)
        | Command::Info(_)
        | Command::Meta(_)
        | Command::Doc(_)
        | Command::Agent(_)
        | Command::Code(_)
        | Command::Stubs(_) => false,
    }
}

#[derive(Subcommand)]
enum Command {
    Ast(ast::Args),
    Check(check::Args),
    Run(run::Args),
    Serve(serve::Args),
    Test(runner::Args),
    Build(build::Args),
    Api(api_diff::Args),
    Pkg(package::Args),
    Ext(ext::Args),
    Init(init::Args),
    Config(config::Args),
    Queue(queue::Args),
    Schema(schema::Args),
    Tmp(tmp::Args),
    Cache(cache_cmd::Args),
    UpdateCheck(update_check::Args),
    Telemetry(telemetry::Args),
    Service(service::Args),
    /// Another name for `nvs service install`.
    // The syntax `mysqld --install` and `httpd -k install` made familiar.
    // Hidden: § 1 names it an accepted alias rather than a second way to write
    // the command, and a help listing both would be advertising two.
    #[command(hide = true)]
    InstallService(service::Install),
    Fmt(fmt::Args),
    Lsp(lsp::Args),
    LspTest(lsp_test::Args),
    Dap(dap::Args),
    Debug(debug::Args),
    Info(info::Args),
    Meta(meta::Args),
    Doc(doc::Args),
    Agent(agent::Args),
    Code(code::Args),
    Stubs(stubs::Args),
}

/// Every subcommand's name as clap writes it: the closed set a telemetry counter names
/// (`rule:tooling/telemetry-counters-are-a-closed-set`). A test holds it equal to [`Command`]'s
/// table, so a new command that is not listed here fails it, and until then counts as `unknown`.
const COMMAND_NAMES: [&str; 30] = [
    "ast",
    "check",
    "run",
    "serve",
    "test",
    "build",
    "api",
    "pkg",
    "ext",
    "init",
    "config",
    "queue",
    "schema",
    "tmp",
    "cache",
    "update-check",
    "telemetry",
    "service",
    "install-service",
    "fmt",
    "lsp",
    "lsp-test",
    "dap",
    "debug",
    "info",
    "meta",
    "doc",
    "agent",
    "code",
    "stubs",
];

/// The subcommand `matches` ran, as its [`COMMAND_NAMES`] entry, or [`telemetry::UNKNOWN`]. The
/// name is the table's own string, never a byte of the command line.
fn invoked(matches: &clap::ArgMatches) -> &'static str {
    let name = matches.subcommand_name();
    COMMAND_NAMES
        .into_iter()
        .find(|known| Some(*known) == name)
        .unwrap_or(telemetry::UNKNOWN)
}

/// The value of `--data` on this process's command line, read without the parser, for a command
/// line the parser did not accept.
fn data_flag() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg == "--data" {
            return args.next().map(PathBuf::from);
        }
        if let Some(value) = arg.to_str().and_then(|arg| arg.strip_prefix("--data=")) {
            return Some(PathBuf::from(value));
        }
    }
    None
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
    let run = move || {
        if started_by_a_manager() {
            log_like_a_service();
        }
        serve::run(
            file.as_deref(),
            listen.as_deref(),
            port,
            &config,
            init,
            None,
        )
    };
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
    // The stored argv's own `--data`, and not the one `nvs service run` was
    // given: the service's command line is the whole of what it runs with.
    // `main` leaves the folder unset for this command so this is the one call.
    set_data_folder(cli.data.as_deref());
    if cli.command.as_ref().is_some_and(prepares) {
        prepare_data_folder();
    }
    // The line runs as the service would, so it logs where the service does.
    log_like_a_service();
    // Counted here and not in `main`, because only now is the stored line's data folder named.
    // A stored line serves or runs a program, so it never carries an upload.
    telemetry::start(
        match &cli.command {
            Some(Command::Serve(_)) => "serve",
            Some(Command::Run(_)) => "run",
            _ => telemetry::UNKNOWN,
        },
        false,
    );
    let no_init = std::env::var_os(config::NO_INIT);
    let init = config::init_gate(
        cli.command.as_ref().is_some_and(initializes),
        cli.no_init,
        no_init.as_deref(),
    );
    match cli.command {
        Some(Command::Serve(args)) => args.hosted(&cli.config, init),
        Some(Command::Run(args)) => args.hosted(&cli.config, init),
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

/// Seals this thread's open debug spool file, once the run that wrote it has ended, and returns
/// `code`.
fn sealed(code: ExitCode) -> ExitCode {
    nvs_runtime::spool::seal_now();
    code
}

fn main() -> ExitCode {
    // First of all: this binary is also the process that ends this one's
    // children when this one is killed, and that copy runs nothing else.
    if let Some(status) = nvs_runtime::child_watch::main_if_asked() {
        return status;
    }

    // Before the footer read below, because a bundled application serves
    // requests too and the hook is what puts a panic under one of them into
    // that request's log. It claims nothing else: every command here is a CLI
    // run with no request beneath it, so its panic goes to stderr through the
    // default hook exactly as it did — `nvs_runtime::floor::install_panic_hook`
    // owns that split. The telemetry hook goes first, so the floor's hook calls it only for a
    // panic no served request claimed (`telemetry::install_panic_hook`).
    telemetry::install_panic_hook();
    nvs_runtime::floor::install_panic_hook();

    // Tier 3 of `rule:errors/escalation-ladder` lives in `nvs_host`, above the
    // crate that classifies a failure raised on a release path, so it arrives
    // there as a function pointer. Here rather than per command because every
    // command below can run compiled code, and a release happens wherever a
    // refcount reaches zero — `nvs_runtime::Ctx::with_pending_set_aside` is the
    // classifier that has no other way to reach it.
    nvs_runtime::floor::install_ladder(nvs_host::ladder::escalate);

    // Before the first `env_hash`, which a bundle computes too: the built-in
    // components are part of the environment every unit is compiled for
    // (`rule:packaging/the-first-party-components-are-built-in`).
    nvs_ext::builtin::fold_into_env_hash();

    // `rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`, and it happens before clap sees anything: a bundled
    // executable's `argv` belongs to the program it carries, so an app whose
    // first argument is `run` or `--help` must not have it read as one of
    // ours. An ordinary `nvs` finds no footer and falls straight through.
    if let Some(payload) = bundle::embedded() {
        return bundle::run(payload);
    }

    // After the bundle check, so a bundled application always runs optimised code. Before any
    // command compiles, so every unit of this process is compiled at the same level.
    if let Some(name) = std::env::var_os(OPT_LEVEL) {
        let Some(level) = name.to_str().and_then(nvs_codegen::OptLevel::named) else {
            eprintln!(
                "error: `{OPT_LEVEL}` is `{}`. It must be `none` or `speed`.",
                name.display()
            );
            return ExitCode::FAILURE;
        };
        nvs_codegen::set_opt_level(level);
    }
    if std::env::var_os(CHECK_TYPES).is_some() {
        nvs_codegen::compile_checked_types();
    }
    if std::env::var_os(PROBES).is_some() {
        nvs_runtime::probe_every_site();
    }

    // A command line that names no command, `--help` and `--version` included, counts
    // `invocation.unknown` before clap prints and exits.
    let matches = Cli::command().try_get_matches().unwrap_or_else(|error| {
        telemetry::unparsed(data_flag().as_deref());
        error.exit()
    });
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| {
        telemetry::unparsed(data_flag().as_deref());
        error.exit()
    });

    // Unreachable: `arg_required_else_help` makes a bare `nvs` print help.
    let Some(command) = cli.command else {
        return ExitCode::FAILURE;
    };

    // The data folder, named once before any command reads it. `nvs service run` is the one
    // command that runs another command line, and [`run_hosted`] names it from that one.
    if !matches!(&command, Command::Service(args) if args.runs_a_stored_line()) {
        set_data_folder(cli.data.as_deref());
    }
    if prepares(&command) {
        prepare_data_folder();
    }
    // [`run_hosted`] counts a stored line itself, once it has named that line's data folder.
    if !matches!(&command, Command::Service(args) if args.runs_a_stored_line()) {
        telemetry::start(invoked(&matches), carries_upload(&command));
    }

    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 4's gate,
    // decided once from [`initializes`] and handed down by the arms that resolve a tree.
    let no_init = std::env::var_os(config::NO_INIT);
    let init = config::init_gate(initializes(&command), cli.no_init, no_init.as_deref());

    match command {
        Command::Ast(args) => args.run(),
        Command::Check(args) => args.run(&cli.config, init),
        // The main thread's local values are not dropped at exit, so its debug spool file is
        // sealed here.
        Command::Run(args) => sealed(args.run(&cli.config, init)),
        Command::Serve(args) => args.run(&cli.config, init),
        Command::Test(args) => sealed(args.run(&cli.config, init)),
        Command::Build(args) => args.run(&cli.config),
        Command::Api(args) => args.run(),
        Command::Pkg(args) => args.run(),
        Command::Ext(args) => args.run(&cli.config),
        Command::Init(args) => args.run(&cli.config),
        Command::Config(args) => args.run(&cli.config),
        Command::Queue(args) => args.run(&cli.config),
        Command::Schema(args) => args.run(&cli.config),
        Command::Tmp(args) => args.run(&cli.config),
        Command::Cache(args) => args.run(&cli.config),
        Command::UpdateCheck(args) => args.run(),
        Command::Telemetry(args) => args.run(),
        Command::InstallService(args) => args.run(&cli.config),
        Command::Service(args) => args.run(&cli.config),
        Command::Fmt(args) => args.run(),
        Command::Lsp(args) => args.run(),
        Command::LspTest(args) => args.run(&cli.config),
        Command::Dap(args) => args.run(&cli.config, init),
        Command::Debug(args) => args.run(&cli.config),
        Command::Info(args) => args.run(),
        Command::Meta(args) => args.run(),
        Command::Doc(args) => args.run(),
        Command::Stubs(args) => args.run(),
        Command::Code(args) => args.run(),
        Command::Agent(args) => args.run(),
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
    /// How many names resolution settled: every declared class, interface,
    /// enum and `type` alias, and every `use` import. `--count`'s `names`,
    /// read here because the [`nvs_hir::Module`] it is read from is not kept.
    names: usize,
    /// The `[grants]` table the program was checked against, which every
    /// unit's key folds in (`nvs_config::cache::env_hash`).
    grants: nvs_config::grants::Grants,
}

impl Checked {
    /// `--count`'s compile line: what each phase produced, each read as the
    /// length of that phase's finished output, so nothing is counted inside a
    /// phase and nothing at all without the flag (`rule:testing/bench-counters`).
    ///
    /// `tokens` is the one figure no phase keeps, so each loaded file is
    /// lexed a second time for it. `nodes` is [`nvs_syntax::walk`]'s tree,
    /// the one `Core\Ast` reads. `ir` is the instructions and terminators of
    /// every function, and is present only for a command that lowers.
    fn count_line(&self, program: Option<&nvs_ir::Program>) -> String {
        fn nodes(node: &nvs_syntax::walk::Node) -> usize {
            1 + node.children.iter().map(nodes).sum::<usize>()
        }
        let mut tokens = 0;
        let mut syntax = 0;
        for file in &self.files {
            tokens += nvs_syntax::tokenize(self.map.file(file.id), &mut Diagnostics::new()).len();
            syntax += nvs_syntax::walk::of_stmts(&file.stmts)
                .iter()
                .map(nodes)
                .sum::<usize>();
        }
        let mut line = format!(
            "compile: tokens={tokens} nodes={syntax} names={} exprs={}",
            self.names,
            self.exprs.len()
        );
        if let Some(program) = program {
            let ir: usize = program
                .functions
                .iter()
                .flat_map(|function| &function.blocks)
                .map(|block| block.insts.len() + 1)
                .sum();
            line.push_str(&format!(" ir={ir}"));
        }
        line
    }

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

/// [`front_end`] with the extension set `extensions` loaded, which `nvs run`
/// and `nvs build --compile` read from their configuration before the program
/// compiles.
fn front_end_extended(
    path: &std::path::Path,
    extensions: Vec<nvs_ext::manifest::Manifest>,
) -> Result<Checked, ExitCode> {
    front_end_in(
        SourceMap::new(),
        path,
        None,
        extensions,
        false,
        Sink::Text,
        config::Init::Never,
        None,
        None,
    )
}

/// [`front_end`] with the deployment's `[capabilities]` block in front of it —
/// `rule:core-classes/db-compile-time-query-checking`'s check-time question, asked of the `nvs.toml` this machine
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
        Vec::new(),
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
/// only list of those files is the one this run leaves behind. `extensions` is
/// the live set's manifests, which the program is typed against.
fn front_end_looking(
    path: &std::path::Path,
    extensions: Vec<nvs_ext::manifest::Manifest>,
    looked: &mut Looked,
) -> Result<Checked, ExitCode> {
    front_end_in(
        SourceMap::new(),
        path,
        None,
        extensions,
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
pub(crate) struct Looked {
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

/// [`front_end_granted`] over a map the caller prepared — empty for a file on
/// disk, or holding the one overlay [`Program::front_end`] registers for a
/// directory of programs.
///
/// `looked` is [`front_end_looking`]'s, filled at every way out after the entry
/// file was asked for.
///
/// `extensions` is the extension set a caller passing no `config` read for
/// itself ([`front_end_extended`]). A caller passing `config` gets the set that
/// configuration loads instead, read beside its grants ([`config::grants`]).
/// Either way the set's classes are declared before the walk and typed from
/// their manifests (`rule:packaging/extension-calls-are-statically-typed`).
///
/// **`survey` is `nvs check`'s, and lends a class file its program's
/// `autoload` map** (`rule:ide/an-autoloaded-file-borrows-its-programs-map`).
/// A file a program autoloads may not declare `autoload`, so checked as its
/// own entry point every autoloaded name in it is undeclared. When the walk
/// below found no declaration of the entry's own, and the walk or the type
/// phase after it reported an undeclared name, the `.nvs` files under `survey`
/// are searched in path order for the program that lends to this file
/// ([`nvs_hir::lenders::choose`]), and the walk and the type phase run again
/// with that program's declarations behind the file's own. A file that
/// declares its own map, or names nothing undeclared, is checked exactly once
/// and nothing is searched. A file no program lends to keeps the first
/// check's diagnostics.
#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per front-end question its four callers answer differently; a struct would be a second spelling of those callers"
)]
fn front_end_in(
    mut map: SourceMap,
    path: &std::path::Path,
    config: Option<&[std::path::PathBuf]>,
    extensions: Vec<nvs_ext::manifest::Manifest>,
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
    // An entry that loaded but is not on disk is a directory's program
    // ([`Program::entry`]), and its tree folds for that directory, as
    // [`Program::configured`] folds it for `nvs test`.
    let configured = match path.parent() {
        Some(dir) if !path.exists() => dir,
        _ => path,
    };
    let (grants, extensions) = match config {
        Some(config) => config::grants(config, configured, init)?,
        None => (None, extensions),
    };
    let extensions = extensions::with_built_in(extensions);
    let classes = nvs_types::ext_lib::hir_classes(&extensions);
    // `rule:security/grants-are-keyed-on-a-namespace`'s table, from the nearest
    // `package.toml` at or above the entry. The walk starts at the entry and not
    // the working directory: a run started from another directory would
    // otherwise read no table, and every package would hold the application's
    // authority. Absolute, so a relative entry still walks above the working
    // directory.
    let start = std::path::absolute(configured).unwrap_or_else(|_| configured.to_path_buf());
    let mut namespace_grants = match nvs_config::grants::Grants::read(&start, &mut map) {
        Ok(read) => read,
        Err(refused) => {
            let mut diags = Diagnostics::new();
            diags.report(refused);
            emit_diagnostics(&mut diags, &map, sink);
            if let Some(looked) = looked {
                looked.take(&map, None);
            }
            return Err(ExitCode::FAILURE);
        }
    };

    let core = nvs_stdlib::registry::link_targets();
    let closed = nvs_stdlib::registry::closed_interfaces();
    let sealed = nvs_stdlib::registry::sealed_classes();
    let imports = nvs_types::core_lib::importable();
    let roster = nvs_hir::CoreRoster::Names {
        links: &core,
        closed: &closed,
        sealed: &sealed,
        imports: &imports,
    };
    let plain = nvs_hir::lenders::is_plain(map.file(id).text());
    let mut lent: Option<nvs_hir::lenders::Lender> = None;
    let (mut diags, module, loaded, autoload, interner, exprs, enums, layouts) = loop {
        let borrowed = lent
            .as_ref()
            .map_or(&[][..], nvs_hir::lenders::Lender::sites);
        let (mut diags, module, loaded, autoload) =
            resolve_entry(&mut map, id, roster, strict_docs, borrowed, &classes);
        // `rule:packaging/a-package-file-stays-under-its-declared-prefix` and
        // `rule:packaging/a-package-cannot-reach-its-host`, over every file the walk loaded.
        // Each package's `optional` list goes into the grant table's digest, because an
        // ungranted optional capability compiles to a guard.
        let packages = nvs_types::package::check(id, &loaded, &autoload, &mut map, &mut diags);
        namespace_grants.set_optional(
            packages
                .all()
                .iter()
                .map(|package| (package.name().to_owned(), package.optional().to_vec())),
        );

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
        let files: Vec<nvs_types::ProgramFile<'_>> = loaded
            .iter()
            .map(|file| nvs_types::ProgramFile {
                src: map.file(file.id),
                stmts: &file.stmts,
            })
            .collect();
        let enums = nvs_types::check::check_program_loaded(
            &files,
            &module,
            grants.as_ref(),
            &namespace_grants,
            Some(&packages),
            &extensions,
            &mut interner,
            &mut exprs,
            &mut diags,
        );

        // Both phases report an undeclared name: name resolution for a `use`,
        // an `extends` or the left of `::`, the type phase for a written type,
        // `new` and `::class`. So the search is decided once both have run.
        if lent.is_none()
            && let Some(root) = survey
            && autoload.sites().is_empty()
            && diags.iter().any(names_an_undeclared_type)
            && let Some(lender) = nvs_hir::lenders::first_lender(root, path, plain)
        {
            drop(files);
            lent = Some(lender);
            map = SourceMap::new();
            id = match map.load(path) {
                Ok(id) => id,
                Err(err) => {
                    report_unreadable(path, &err, sink);
                    return Err(ExitCode::FAILURE);
                }
            };
            continue;
        }

        let layouts =
            (!diags.has_errors()).then(|| nvs_types::build_class_layouts(&files, &module.graph));
        drop(files);
        break (
            diags, module, loaded, autoload, interner, exprs, enums, layouts,
        );
    };

    if let Some(looked) = looked {
        looked.take(&map, Some(&autoload));
    }
    // `nvs check`'s alone (`rule:statements/an-unused-use-is-a-warning`): a
    // program that runs prints nothing about a line it never needed. A file
    // under `vendor` is not the project's to change, so it is not reported.
    if matches!(sink, Sink::Check { .. } | Sink::Fix) {
        let owned = loaded
            .iter()
            .filter(|file| !map.file(file.id).path().is_some_and(check::vendored));
        let unused = nvs_lsp::diagnostics::unused_uses(&map, owned, &module.imports);
        diags.extend(unused);
    }
    conceal_secret_literals(&mut map, &diags, &loaded, &exprs, &interner);
    let Some(layouts) = layouts else {
        autoload.note_searched_roots(
            &mut diags,
            path.parent()
                .filter(|dir| !dir.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new(".")),
        );
        emit_diagnostics(&mut diags, &map, sink);
        return Err(ExitCode::FAILURE);
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
        names: module.symbols.len() + module.imports.len(),
        grants: namespace_grants,
    })
}

/// Parses the entry `id` and walks its `require`/`autoload` graph, with
/// `borrowed` behind the entry's own `autoload` declarations and the classes of
/// the loaded extension set, `extensions`, declared ahead of it.
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
    extensions: &[nvs_hir::ExtensionClass],
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
        &mut diags,
        strict_docs,
        borrowed,
        extensions,
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
/// ([`front_end_granted`]), so `nvs check` answers `rule:core-classes/db-compile-time-query-checking`'s question
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
///
/// `--todos`, `--deny` and `--fix` are `rule:tooling/a-todo-is-a-comment-the-tools-list`'s.
/// The todos are listed after the diagnostics, on standard output, in path and
/// line order ([`check::todos`]). `--deny` changes the exit status and nothing
/// that is printed. `--fix` runs the front end silently ([`Sink::Fix`]) and
/// writes each pass's safe edits ([`check::safe_edits`]) until a pass has none
/// or [`FIX_PASSES`] have run, says how many it made, and then checks the
/// program once more the ordinary way, so what it prints is what is left.
///
/// A directory is checked as the one program `nvs test` runs for it
/// ([`Program::Directory`], `rule:testing/a-directory-of-programs-is-one-test-program`):
/// an entry that exists only in memory requires every `.nvs` file under it
/// that no other one requires, so the two commands agree on what the
/// directory is. `--autoload-map` renders its paths from the directory.
fn run_check(
    config: &[std::path::PathBuf],
    path: &std::path::Path,
    flags: &CheckFlags,
    init: config::Init,
) -> ExitCode {
    let survey = survey_root(config);
    let directory = if path.is_dir() {
        match Program::named_by(path) {
            Ok(Some(Program::Directory { dir, files })) => Some(Program::entry(&dir, &files)),
            Ok(_) => {
                eprintln!(
                    "error: {}: this directory has no `.nvs` file",
                    path.display()
                );
                return ExitCode::FAILURE;
            }
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };
    let front = |sink: Sink| match &directory {
        Some((entry, text)) => {
            let mut map = SourceMap::new();
            map.overlay(entry, text);
            front_end_in(
                map,
                entry,
                Some(config),
                Vec::new(),
                flags.strict_docs,
                sink,
                init,
                None,
                Some(&survey),
            )
        }
        None => front_end_granted(
            path,
            Some(config),
            flags.strict_docs,
            sink,
            init,
            Some(&survey),
        ),
    };
    if flags.fix {
        let (mut made, mut files) = (0, std::collections::BTreeSet::new());
        for _ in 0..FIX_PASSES {
            let _ = front(Sink::Fix);
            let mut wrote = 0;
            for file in check::take().edits {
                match check::apply(&file) {
                    Ok(0) => {}
                    Ok(n) => {
                        wrote += n;
                        files.insert(file.path);
                    }
                    Err(err) => {
                        eprintln!("error: could not write {}: {err}", file.path.display());
                        return ExitCode::FAILURE;
                    }
                }
            }
            if wrote == 0 {
                break;
            }
            made += wrote;
        }
        println!(
            "made {made} {} in {} {}",
            if made == 1 { "edit" } else { "edits" },
            files.len(),
            if files.len() == 1 { "file" } else { "files" },
        );
    }
    let sink = Sink::Check {
        json: flags.json,
        list_todos: flags.todos,
        read_todos: flags.deny.contains(&check::Deny::Todo),
    };
    match front(sink) {
        Ok(checked) => {
            if flags.count {
                eprintln!("{}", checked.count_line(None));
            }
            if flags.autoload_map {
                let base = match path.parent() {
                    _ if directory.is_some() => path,
                    Some(dir) if !dir.as_os_str().is_empty() => dir,
                    _ => std::path::Path::new("."),
                };
                print!("{}", checked.autoload.render(base));
            } else if !flags.json {
                println!("no errors");
            }
            let found = check::take();
            let denied = flags.deny.iter().any(|kind| match kind {
                check::Deny::Deprecated => found.deprecated,
                check::Deny::Todo => found.todo,
            });
            if denied {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(code) => code,
    }
}

/// How many times `nvs check --fix` checks and writes before it stops. A fix
/// whose result carries the same fix again would otherwise never stop.
const FIX_PASSES: usize = 16;

/// What `nvs check` was asked for besides the file.
struct CheckFlags {
    json: bool,
    autoload_map: bool,
    strict_docs: bool,
    count: bool,
    todos: bool,
    deny: Vec<check::Deny>,
    fix: bool,
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
/// that rule's whole promise in one line: the document cannot say something the
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
    let document = openapi::document(&checked.exprs, &title);
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
                            // table to ask instead. The value narrows to the
                            // enum's backing `int` or `uint` here
                            // (`rule:enums/one-backing-type`) rather than
                            // crossing widened — `nvs_runtime::commands::
                            // CaseValue` owns why.
                            nvs_types::commands::ArgConv::Enum { class, cases } => {
                                nvs_runtime::commands::ArgConv::Enum {
                                    class: class.clone(),
                                    cases: cases
                                        .iter()
                                        .map(|(case, value)| {
                                            (case.clone(), nvs_types::crossing::case_value(*value))
                                        })
                                        .collect(),
                                }
                            }
                            nvs_types::commands::ArgConv::Words => {
                                nvs_runtime::commands::ArgConv::Words
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
    let mut inbound = spec.build();
    // The trace a served door would take off the same lines, at a rate of zero
    // for `crate::runner`'s reason: nothing here exports a trace, and a
    // `traceparent` the file wrote is still continued, so `Core\Server::traceId`
    // reads the id the file named.
    nvs_server::trace::take(&mut inbound, None);
    Ok(inbound)
}

#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per `nvs run` flag, plus the tree the run resolves and the debug session; a struct here would be a second spelling of `run::Args`"
)]
/// `debug` is the session of `nvs dap`'s launch: the program is compiled under the debug switch,
/// its unit is loaded into the session, and its run is attached to it
/// (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`).
fn run_run(
    path: &std::path::Path,
    dump_ir: bool,
    dump_asm: bool,
    fault_inject: Option<run::FaultSiteArg>,
    count: bool,
    profile: Option<(profile::Format, Option<PathBuf>)>,
    request: Option<&std::path::Path>,
    peer: Option<&std::path::Path>,
    events: Option<&std::path::Path>,
    config: &[PathBuf],
    arguments: Vec<String>,
    init: config::Init,
    debug: Option<&nvs_host::debug::Debugger>,
) -> ExitCode {
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
    // The configuration tree is read once for the whole run, here, and every later step reads
    // this one result: the compile types extension calls against its manifests, the run takes
    // its snapshot, and the extension host loads its set. A tree that does not resolve compiles
    // the program against no extension, and the refusal is reported where the snapshot is taken
    // below, so a program's own diagnostics still come first.
    let mut config_sources = SourceMap::new();
    let booted = config::boot_origins(config, Some(&config_entry), &mut config_sources, init);
    let extensions = match &booted {
        Ok((snapshot, origins)) => {
            match extensions::manifests(&snapshot.config, origins, &config_sources) {
                Ok(manifests) => manifests,
                Err(diagnostic) => {
                    let mut diags = Diagnostics::new();
                    diags.report(diagnostic);
                    render_diagnostics(&mut diags, &config_sources);
                    return ExitCode::FAILURE;
                }
            }
        }
        Err(_) => Vec::new(),
    };
    let checked = match front_end_extended(path, extensions) {
        Ok(checked) => checked,
        Err(code) => return code,
    };
    let src = checked.map.file(checked.id);

    let lower = || {
        nvs_ir::lower::lower_program(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &checked.program_files(),
            &checked.exprs,
            &checked.interner,
            &checked.enums,
            &checked.layouts,
        )
    };
    let program = if debug.is_some() {
        nvs_ir::lower::with_debug_slots(lower)
    } else {
        lower()
    };
    // The compile's own line, before the program runs, so it is printed
    // whatever the program then does.
    if count {
        eprintln!("{}", checked.count_line(Some(&program)));
    }
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
    // a refusal to start — `rule:config/later-wins-and-every-override-is-recorded`'s later-wins and `rule:config/ownership-is-the-trust-boundary`'s boundary are only
    // worth anything if a tree that does not resolve stops the run.
    let (snapshot, origins) = match booted {
        Ok(booted) => booted,
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
    // one holder, and an `Arc` is what lets the run and the seam both name it.
    let unit = std::sync::Arc::new(unit);
    // Held for the whole run, because the session keeps the unit only while something holds it.
    let debug = debug.and_then(|debugger| {
        let sites = script::debug_unit(&unit, &program, &checked.map, dap::file_key)?;
        debugger.load(&sites);
        Some((debugger, sites))
    });
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
    // deployment and starts nothing, and so is a `--count` run: a worker runs
    // on this thread whenever the script yields, so its database handshake and
    // its polls would land in the per-thread allocation counters the count
    // line reads, at a size that depends on scheduling rather than on the
    // program (`rule:testing/bench-counters`).
    let queued = nvs_config::queue::queue_for(&snapshot.config, &std::collections::BTreeMap::new())
        .ok()
        .flatten()
        .filter(|bounds| bounds.workers > 0 && !count)
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
    let routes = std::sync::Arc::new(nvs_types::crossing::runtime_routes(&checked.exprs));
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
    //
    // The CSRF door is taken too, by `runner::refuse_a_forged_write`, once the
    // tree is on the context below, so a forged write is refused here as
    // `Core\Test::request` refuses it.
    let mut offered = None;
    let mut offered_sse = None;
    let mut arrived = None;
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
                arrived = Some(inbound);
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
    // A CLI run's app folder is its entry file's, or the `root` of an
    // `[[app]]` block over it.
    let folder = snapshot.folder();
    ctx.set_config(snapshot);
    if let Some(folder) = folder {
        ctx.set_app_folder(folder.into());
    }
    // `rule:testing/debug-surface`: a program `[debug] mode` lets start coverage counts into a
    // table located by file and line, so `Core\Debug::coverage` names what ran. Built only under
    // that ceiling: one counter per statement and edge, and one line per statement, for the run.
    if ctx
        .debug_ceiling()
        .contains(nvs_runtime::DebugFlags::COVERAGE)
    {
        let sites = coverage::Sites::of(coverage::Probes::of(&program), &checked.map);
        ctx.set_stmt_hits(Some(std::sync::Arc::new(sites.table())));
    }
    if let Some(mut inbound) = arrived {
        runner::refuse_a_forged_write(&ctx, &routes, &mut inbound);
        ctx.set_inbound(inbound);
    }
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
            nvs_config::cache::env_hash(&for_compiler.config, &checked.grants),
        )
        .to_string()
        .into(),
    );
    // `rule:observability/a-query-span-names-its-call-site`: the table a traced query reads the
    // file and line of its call from.
    ctx.set_query_sites(script::query_sites(&program, &checked.map));
    // Hands the context the unit's class table: the class a helper's
    // bare-message failure is promoted to, and the shared ownership that lets
    // the context outlive the unit. `Unit::install_in` owns both reasons.
    unit.install_in(&mut ctx);
    if let Some((debugger, sites)) = &debug {
        debugger.attach(&mut ctx, sites);
    }
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
    // `--profile`: the call probes time every call into the file, for the length of the run.
    let profiling = match profile {
        Some((format, out)) => {
            match profile::install(
                &mut ctx,
                format,
                out.as_deref(),
                &path.display().to_string(),
            ) {
                Ok(running) => Some(running),
                Err(line) => {
                    eprintln!("{line}");
                    return ExitCode::FAILURE;
                }
            }
        }
        None => None,
    };
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
    // The setup above is the host's, and its size follows the paths this run
    // was started with. The program is measured from here, so its ceiling is
    // crossed at the same allocation whatever folder it runs in
    // (`Ctx::rebase_memory`).
    ctx.rebase_memory();
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
            // *inside* the task, because a deferred callable is a task like any
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
    // one request it is, from here until the scheduler is idle. The scheduler
    // charges it for the time its own tasks run on this thread, which is the
    // thread that registers, so the clock is the program's
    // (`nvs_host::watchdog`'s *Charging a request for the time it runs*).
    //
    // Published for a run under no cap as well, because the program may narrow
    // `limits.cpu_time` with `Core\Config::set`, and the charge that ceiling is
    // compared with has to cover the whole run — `Registration::publish` owns
    // that. A platform with no per-thread clock starts no thread, because
    // nothing could be charged there.
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
    // The loaded extensions, which a compiled call into one reaches through
    // `nvs_runtime::extension` while the run's tasks run. The set is the one the
    // tree read at the top of the run names, so `E0652` points at that tree's line.
    let calls = match extensions::loaded(&for_compiler.config, &origins, &config_sources) {
        Ok(set) => std::rc::Rc::new(extensions::Calls::new(set)),
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &config_sources);
            return ExitCode::FAILURE;
        }
    };
    let calling = nvs_runtime::extension::install(
        std::rc::Rc::clone(&calls) as std::rc::Rc<dyn nvs_runtime::extension::Extensions>
    );
    // The last step of the start that reads the configuration, and the tree it read is still
    // the one read at the top. Every `.nvst` case runs through here, so a debug build that
    // reads a second tree fails the suite.
    #[cfg(debug_assertions)]
    assert_eq!(
        config::trees_read(),
        1,
        "`nvs run` reads its configuration tree once"
    );
    let watchdog = nvs_host::ThreadClock::current()
        .is_some()
        .then(nvs_host::Watchdog::new);
    // The slice clock `nvs serve` starts too, so a run's tasks share this thread
    // the way a core's requests share their core (`nvs_host::slice`). A
    // `--count` run starts none, for the queue worker's reason above: the clock
    // raises a yield by wall time, and each yield's switch and reactor poll run
    // on this thread, so where they fall would move the allocation counters the
    // count line reads (`rule:testing/bench-counters`). Its tasks then switch
    // only where they wait, which is the same place on every run.
    let _slices = watchdog.as_ref().filter(|_| !count).map(|watchdog| {
        let slices = nvs_host::SliceClock::start();
        watchdog.slice_with(&slices);
        slices
    });
    let watched = watchdog.as_ref().map(|watchdog| {
        let watched = watchdog.register_requests();
        watched.publish(tree.clone());
        watched
    });
    // `[limits] wall_time`'s half of the same stop, which the watchdog above
    // cannot give because it samples CPU time; `wall`'s module doc owns it.
    let wall_clock = wall::start(tree);
    let ran = nvs_runtime::script::scoped(compiler.as_ref(), || {
        nvs_runtime::inproc::scoped(&under_test, || nvs_host::run_until_idle(&mut sched))
    });
    drop(shared);
    // The run is over at this line, so the request stops being charged at it:
    // the registration goes first, and the watchdog after it joins its thread.
    // Everything below reads a context nothing is sampling any more.
    drop(watched);
    drop(watchdog);
    drop(wall_clock);
    drop(installed);
    drop(calling);
    // The last reference, so this ends the request and drops its resources.
    drop(calls);
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
    // `gc` is last, so a reader that matches the first four from the start of
    // the line reads them unchanged.
    if let Some((allocations, bytes)) = allocated_before {
        let counted = ctx.counted();
        eprintln!(
            "count: statements={} calls={} allocations={} bytes={} gc={}",
            counted.statements,
            counted.calls,
            nvs_runtime::budget::allocations().wrapping_sub(allocations),
            nvs_runtime::budget::allocated_bytes().wrapping_sub(bytes),
            counted.gc_cycles,
        );
    }
    // The profile is closed whatever the outcome below, like the count above.
    if let Some(running) = profiling
        && !running.finish(&mut ctx)
    {
        return ExitCode::FAILURE;
    }

    // `rule:http-server/containment-does-not-end-at-the-helper`'s outer boundary caught a panic under the task root. For a
    // request that is a failed request; for a CLI run it is this process's
    // failure, reported as the fatal it is rather than unwinding out of `main`.
    if let Err(panic) = finished.outcome {
        eprintln!("FATAL: {}", panic.message());
        ctx.end_abandoned_children();
        return ExitCode::FAILURE;
    }

    let Some(outcome) = status.get() else {
        eprintln!("internal error: the script's task ran nothing");
        return ExitCode::FAILURE;
    };

    let code = match outcome {
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
    };
    // `rule:core-classes/process-spawn`: a process the script started and left
    // running without a `wait()` or a `kill()` is stopped here and reported as
    // an error. After the report of how the script ended, so an uncaught throw
    // or a `FATAL` comes first, and here rather than in the context's `Drop` so
    // the run exits with `1`, as it does for an uncaught throw. A stream the
    // script said it would end with `end()` and did not is reported the same
    // way (`rule:core-classes/spreadsheet-writes-as-a-stream`).
    let abandoned = ctx.end_abandoned_children();
    if ctx.end_unfinished_stream() || abandoned {
        return ExitCode::FAILURE;
    }
    code
}

/// Runs a tree of `.nvst` cases and turns the summary into an exit code.
///
/// Each case is run by spawning **this** binary — `nvs_test::run` documents
/// why a subprocess rather than an in-process compile — so a debug build
/// tests itself and a release build tests itself, with nothing to configure.
#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per `nvs test` flag; a struct here would be a second spelling of `runner::Args`"
)]
fn run_test(
    paths: &[PathBuf],
    select: runner::Selection,
    jobs: Option<std::num::NonZeroUsize>,
    format: runner::Format,
    flags: runner::Flags,
    coverage: &coverage::Requested,
    cases: Option<PathBuf>,
    record: Option<PathBuf>,
    watch: bool,
    config: &[PathBuf],
    init: config::Init,
) -> ExitCode {
    if watch && format != runner::Format::Human {
        // `rule:testing/watch-reruns-on-an-edit`: a consumer reads a machine
        // format's one document once, and a watched run writes one per pass.
        eprintln!("error: `--watch` reports in the human format only");
        return ExitCode::FAILURE;
    }
    let runner::Flags {
        update,
        list,
        show_output,
        shared_worker,
        fail_fast,
        rerun_failed,
        shuffle,
        mutate,
        ref profile,
        ..
    } = flags;
    // `rule:testing/nvst-is-separate`'s "`nvs test` runs both", decided by the path rather than
    // by a flag: a program is a `.nvs` file or a directory holding `.nvs` files
    // (`rule:testing/a-directory-of-programs-is-one-test-program`), a
    // conformance case is anything else, and nothing has to be spelled out at
    // the call site. Under `--list`, a directory with any `.nvst` file in it is
    // a case tree, so an editor can list a workspace's cases by naming the
    // workspace folder. A run of such a directory stays refused, because it
    // would have to report two suites in one call.
    let named = |path: &PathBuf| {
        if list && holds_cases(path) {
            Ok(None)
        } else {
            Program::named_by(path)
        }
    };
    let programs: Vec<Option<Program>> = match paths.iter().map(named).collect() {
        Ok(programs) => programs,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if programs.iter().any(Option::is_some) {
        let [Some(program)] = programs.as_slice() else {
            eprintln!(
                "error: `nvs test` runs one program at a time. Pass one `.nvs` file or one folder. To run every test in a project, run `nvs test .` in the project folder. Run `.nvst` cases in a separate call."
            );
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
        if shared_worker {
            // The worker runs a `.nvst` case's whole program, and a program's
            // `#[Test]` methods are already each an isolate of their own.
            eprintln!(
                "error: `--shared-worker` runs `.nvst` cases, not a program's `#[Test]` methods"
            );
            return ExitCode::FAILURE;
        }
        // `rule:config/the-config-is-an-immutable-snapshot`'s snapshot, resolved here for the reason `run_run`
        // resolves it above its own compile: `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact key is half
        // configuration — § 7's `[opcache]` says where artifacts live and
        // whether they are read at all, and § 4's environment digest covers the
        // loaded extension set — so a suite compiled above the tree would
        // address an artifact by an environment this run is not in. A tree that
        // does not resolve stops a test run exactly as it stops a `nvs run`.
        let tree = match config::boot_tree(config, program.configured(), init) {
            Ok(tree) => tree,
            Err(code) => return code,
        };
        if watch {
            return watch_program_tests(program.configured(), &tree, &select, &flags, coverage);
        }
        return run_program_tests(program, &tree, format, &select, flags, coverage);
    }
    if watch {
        // A case tree has no `require` graph for the background check to read.
        eprintln!("error: `--watch` runs a program's `#[Test]` methods, not a `.nvst` tree");
        return ExitCode::FAILURE;
    }
    if !select.tags.is_empty() || !select.skip_tags.is_empty() {
        // A `.nvst` case has no `#[Test]` attribute to carry a tag, so every
        // `--tag` would select nothing.
        eprintln!(
            "error: `--tag` and `--skip-tag` select a program's `#[Test]` methods, not `.nvst` cases"
        );
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
    if fail_fast || rerun_failed || shuffle.is_some() {
        // `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle` is about a program's
        // tests, which share one process and one report. Refused rather than ignored.
        eprintln!(
            "error: `--fail-fast`, `--rerun-failed` and `--shuffle` run a program's `#[Test]` methods, not a `.nvst` tree"
        );
        return ExitCode::FAILURE;
    }
    if show_output {
        // A `.nvst` case compares its whole output with its expectation, so
        // there is no captured output for the flag to show.
        eprintln!(
            "error: `--show-output` prints the output of a program's `#[Test]` methods, not of a `.nvst` tree"
        );
        return ExitCode::FAILURE;
    }
    if format != runner::Format::Human && !(list && format == runner::Format::Json) {
        // § 22's formats report a `#[Test]` run, and § 23 keeps the two suites
        // from sharing a summary — so a machine format over a `.nvst` tree
        // names a document this subcommand does not produce. A listing has no
        // summary, which is why its JSON document is the one exception.
        eprintln!("error: `--format` reports a program's `#[Test]` methods, not a `.nvst` tree");
        return ExitCode::FAILURE;
    }
    if coverage.any() {
        // The counts come from the probes of the one program this process
        // compiles, and a `.nvst` case runs as a process of its own.
        eprintln!(
            "error: `--coverage-lcov`, `--coverage-clover` and `--coverage-cobertura` measure a program's `#[Test]` methods, not a `.nvst` tree"
        );
        return ExitCode::FAILURE;
    }
    if mutate {
        // `rule:testing/mutation-testing`: a mutant runs the tests that reached
        // its line, and only a program's tests are measured per test.
        eprintln!(
            "error: `--mutate` changes a program and runs its `#[Test]` methods, not a `.nvst` tree"
        );
        return ExitCode::FAILURE;
    }
    if profile.is_some() {
        // The calls are timed in the one program this process compiles, as coverage is.
        eprintln!("error: `--profile` times a program's `#[Test]` methods, not a `.nvst` tree");
        return ExitCode::FAILURE;
    }
    if select.bench {
        // `rule:testing/bench-counters`: a bench is a method of a program, and a case has none.
        eprintln!("error: `--bench` runs a program's `#[Bench]` methods, not a `.nvst` tree");
        return ExitCode::FAILURE;
    }

    let only = match &cases {
        Some(named) => match std::fs::read_to_string(named) {
            Ok(text) => Some(
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from)
                    .collect::<Vec<_>>(),
            ),
            Err(error) => {
                eprintln!(
                    "error: could not read the case list {}: {error}",
                    named.display()
                );
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };
    if list {
        // Answered from the directory walk alone: no case is read, and no
        // working directory is made.
        return runner::list_cases(paths, only.as_deref(), &select, format);
    }

    let root = match tmp::runner_dir(config) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let mut options = match nvs_test::Options::from_current_exe(root) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: could not locate this binary to run cases with: {error}");
            return ExitCode::FAILURE;
        }
    };
    options.filter = select.filter;
    options.shard = select.shard;
    options.only = only;
    // Every process a case starts compiles at this run's level, which `--opt-level` set.
    options.nvs_env.push((
        OPT_LEVEL.to_owned(),
        nvs_codegen::opt_level().name().to_owned(),
    ));
    // And has its probes on when `--probes` turned them on here.
    if nvs_runtime::every_site_probed() {
        options.nvs_env.push((PROBES.to_owned(), "1".to_owned()));
    }
    if let Some(jobs) = jobs {
        options.jobs = jobs.get();
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

    if shared_worker {
        match shared::Worker::start(init) {
            Ok(worker) => options.shared = Some(std::sync::Arc::new(worker)),
            Err(error) => {
                eprintln!("error: {error}");
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

/// `rule:testing/watch-reruns-on-an-edit`: runs the tests of the program at `path`, then runs
/// them again each time a file the last pass's compile looked at changes, until the process is
/// interrupted.
///
/// Each pass names the program again, so a file added to a folder of programs is in the next
/// one. The change is [`script::Rerun`]'s, which is the background check `nvs run` has.
///
/// **What it spends:** that check's one thread, and between passes the trace of the last
/// compile. Each pass's program and tests are freed when the pass is reported.
fn watch_program_tests(
    path: &std::path::Path,
    tree: &config::Tree,
    select: &runner::Selection,
    flags: &runner::Flags,
    coverage: &coverage::Requested,
) -> ExitCode {
    let setup = match TestSetup::of(tree) {
        Ok(setup) => setup,
        Err(code) => return code,
    };
    let rerun = script::Rerun::start(&tree.snapshot.config);
    loop {
        let program = match Program::named_by(path) {
            Ok(Some(program)) => program,
            Ok(None) => {
                eprintln!("error: {} has no `.nvs` program to test", path.display());
                return ExitCode::FAILURE;
            }
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        };
        let started = std::time::SystemTime::now();
        let mut looked = Looked::default();
        let code = run_program_pass(
            &program,
            tree,
            &setup,
            runner::Format::Human,
            select,
            flags.clone(),
            coverage,
            Some(&mut looked),
        );
        // A pass whose compile read nothing has nothing to watch.
        if looked.read.is_empty() && looked.missed.is_empty() {
            return code;
        }
        rerun.arm(program.watched(looked, started));
        println!("\nWaiting for a file to change. Press Ctrl+C to stop.");
        if !rerun.wait() {
            return code;
        }
        println!("\nA file changed, so the tests run again.\n");
    }
}

/// Runs `program`'s `#[Test]` methods under `tree`, which `nvs test` resolves from the run's
/// configuration and `nvs ext test` from the extension under test.
fn run_program_tests(
    program: &Program,
    tree: &config::Tree,
    format: runner::Format,
    select: &runner::Selection,
    flags: runner::Flags,
    coverage: &coverage::Requested,
) -> ExitCode {
    match TestSetup::of(tree) {
        Ok(setup) => run_program_pass(program, tree, &setup, format, select, flags, coverage, None),
        Err(code) => code,
    }
}

/// What every pass of a program's tests runs under, made once per process: the tree's outbound
/// TLS client, installed, and its extension set, loaded.
struct TestSetup {
    manifests: Vec<nvs_ext::manifest::Manifest>,
    set: std::sync::Arc<nvs_ext::load::Set>,
}

impl TestSetup {
    /// The setup `tree` names. A diagnostic is rendered here, and the `Err` is the exit code.
    fn of(tree: &config::Tree) -> Result<Self, ExitCode> {
        let refuse = |diagnostic| {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &tree.sources);
            ExitCode::FAILURE
        };
        // The same install `run_run` does, for the same reason: a suite runs the program, and a
        // `#[Test]` method reaching an origin asks the anchors the tree named rather than
        // whichever set a first call happened to settle. A process installs it once.
        config::install_tls_client(&tree.snapshot).map_err(refuse)?;
        // The suite is typed against the tree's extension set, as `nvs run` types a program, and
        // a call into one reaches the loaded set through `nvs_runtime::extension` while the suite
        // runs. The whole suite is one request to the set: its tests share the instances.
        let config = &tree.snapshot.config;
        let manifests =
            extensions::manifests(config, &tree.origins, &tree.sources).map_err(refuse)?;
        let set = extensions::loaded(config, &tree.origins, &tree.sources).map_err(refuse)?;
        Ok(Self {
            manifests,
            set: std::sync::Arc::new(set),
        })
    }
}

/// One run of `program`'s `#[Test]` methods under `setup`: the compile, the tests and the report.
///
/// `looked`, where there is one, is filled with every path the compile looked at
/// ([`front_end_looking`]), which is what `--watch` checks.
#[expect(
    clippy::too_many_arguments,
    reason = "run_program_tests' six, with the setup its caller made once and the paths --watch reads"
)]
fn run_program_pass(
    program: &Program,
    tree: &config::Tree,
    setup: &TestSetup,
    format: runner::Format,
    select: &runner::Selection,
    flags: runner::Flags,
    coverage: &coverage::Requested,
    looked: Option<&mut Looked>,
) -> ExitCode {
    let checked = match program.front_end(setup.manifests.clone(), looked) {
        Ok(checked) => checked,
        Err(code) => return code,
    };
    let set = std::sync::Arc::clone(&setup.set);
    // Each test's calls run in its own request tree, so its budget and its
    // instances are its own and end with it. The host is this thread's, so
    // each core the run starts installs one of its own over the same set.
    let calling = nvs_runtime::extension::install(std::rc::Rc::new(extensions::Tested(
        std::sync::Arc::clone(&set),
    )));
    // `rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`: every run records its
    // failed tests, and `--rerun-failed` narrows the selection to the ones the last run recorded.
    let mut flags = flags;
    let mut select = select.clone();
    flags.record_failed = runner::failed_tests_file(program.configured());
    if flags.rerun_failed {
        select.only = flags
            .record_failed
            .as_deref()
            .and_then(runner::read_failed_tests);
        if select.only.is_none() {
            let line =
                "No failed tests are recorded for this program, so every selected test runs.";
            match format {
                runner::Format::Human => println!("{line}"),
                runner::Format::Json | runner::Format::Junit => eprintln!("{line}"),
            }
        }
    }
    // `--filter` reaches both suites, and means the same thing in each:
    // `runner::selected` owns the rule and why it is the `.nvst` tree's.
    let code = runner::run(
        checked,
        &tree.snapshot,
        Some(set),
        format,
        &select,
        flags,
        coverage,
    );
    drop(calling);
    code
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
    /// every one of them that no other one requires, in name order
    /// ([`unrequired`], `rule:testing/a-directory-of-programs-is-one-test-program`).
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

    /// The checked program, typed against the extension set `extensions`.
    /// `looked`, where there is one, is filled with every path the compile
    /// looked at.
    ///
    /// A directory's entry exists only in memory, and sits where
    /// [`Self::entry_path`] puts it, so every `require` in it resolves against
    /// that directory exactly as a written entry's would
    /// (`rule:testing/a-directory-of-programs-is-one-test-program`).
    fn front_end(
        &self,
        extensions: Vec<nvs_ext::manifest::Manifest>,
        looked: Option<&mut Looked>,
    ) -> Result<Checked, ExitCode> {
        let (map, entry) = match self {
            Self::File(path) => (SourceMap::new(), path.clone()),
            Self::Directory { dir, files } => {
                let (entry, text) = Self::entry(dir, files);
                let mut map = SourceMap::new();
                map.overlay(&entry, &text);
                (map, entry)
            }
        };
        front_end_in(
            map,
            &entry,
            None,
            extensions,
            false,
            Sink::Text,
            config::Init::Never,
            looked,
            None,
        )
    }

    /// What `--watch` checks after a pass whose compile began at `started`
    /// and looked at `looked`. A folder of programs also has the folders its
    /// files are in checked, so a new file starts a pass too.
    fn watched(&self, looked: Looked, started: std::time::SystemTime) -> script::Watched {
        match self {
            Self::File(path) => script::Watched::new(path, looked, started, false, &[]),
            Self::Directory { dir, files } => {
                let dirs: std::collections::BTreeSet<PathBuf> = std::iter::once(dir.clone())
                    .chain(
                        files
                            .iter()
                            .filter_map(|file| file.parent().map(PathBuf::from)),
                    )
                    .collect();
                let dirs: Vec<PathBuf> = dirs.into_iter().collect();
                script::Watched::new(&Self::entry_path(dir, files), looked, started, true, &dirs)
            }
        }
    }

    /// Where the entry of a directory of programs would sit: under a name no
    /// file under the directory carries, so the overlay never replaces a real
    /// file's bytes.
    fn entry_path(dir: &std::path::Path, files: &[PathBuf]) -> PathBuf {
        let mut name = String::from("#tests.nvs");
        while files
            .iter()
            .any(|file| file.file_name().is_some_and(|f| f == name.as_str()))
        {
            name.insert(0, '#');
        }
        dir.join(name)
    }

    /// The entry a directory of programs runs from: where it would sit, and
    /// its text, one `require` per file of `files` that no other one requires.
    /// It never exists on disk, and `nvs test` and `nvs check` both read it.
    fn entry(dir: &std::path::Path, files: &[PathBuf]) -> (PathBuf, String) {
        let entry = Self::entry_path(dir, files);
        let roots = unrequired(&entry, dir, files);
        let text = directory_entry(dir, &roots);
        (entry, text)
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
    let sorted = |dir: &std::path::Path| -> std::io::Result<std::vec::IntoIter<PathBuf>> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()?;
        entries.sort();
        Ok(entries.into_iter())
    };
    // One sorted listing per open directory, rather than recursion, so a
    // tree of any depth walks in constant stack and in the same order.
    let mut open = vec![sorted(dir)?];
    while let Some(entries) = open.last_mut() {
        let Some(entry) = entries.next() else {
            open.pop();
            continue;
        };
        // `nvs_test::discover`'s rules: a linked directory and a cache
        // directory are not walked.
        if std::fs::symlink_metadata(&entry).is_ok_and(|meta| meta.is_dir()) {
            if !nvs_test::is_cache_dir(&entry) {
                open.push(sorted(&entry)?);
            }
        } else if is_program(&entry) {
            into.push(entry);
        } else if entry.extension().is_some_and(|ext| ext == "nvst") {
            *cases = true;
        }
    }
    Ok(())
}

/// Whether `path` is a directory with a `.nvst` file anywhere under it. A
/// directory that cannot be read holds none, and [`Program::named_by`] then
/// names the error.
fn holds_cases(path: &std::path::Path) -> bool {
    let mut files = Vec::new();
    let mut cases = false;
    path.is_dir() && programs_under(path, &mut files, &mut cases).is_ok() && cases
}

/// The files of a directory of programs that no other file in it requires, in
/// the order [`programs_under`] found them.
///
/// `require` runs every time it is reached, and `Core\Test::request` runs the
/// program's statements, so an entry that required a file another file in the
/// directory requires too would run that file twice per request: a project's
/// entry file, required by the project and again by a `tests/bootstrap.nvs`.
/// The edges are the ones the name-resolution walk resolves
/// ([`nvs_hir::Loaded::requires`]), so this agrees with the build on what a
/// path names. A directory where no file's text holds `require` has no edge
/// to find and walks nothing; any other pays one walk without the type phase,
/// and its diagnostics are dropped because the real compile reports them.
fn unrequired(entry: &std::path::Path, dir: &std::path::Path, files: &[PathBuf]) -> Vec<PathBuf> {
    let requires =
        |file: &PathBuf| std::fs::read_to_string(file).is_ok_and(|text| text.contains("require"));
    if !files.iter().any(requires) {
        return files.to_vec();
    }
    let mut map = SourceMap::new();
    map.overlay(entry, directory_entry(dir, files));
    let Ok(id) = map.load(entry) else {
        return files.to_vec();
    };
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    let (_, loaded, _) = nvs_hir::resolve_program(
        id,
        stmts,
        &mut map,
        nvs_hir::CoreRoster::Trusted,
        &mut diags,
    );
    let required: std::collections::HashSet<PathBuf> = loaded
        .iter()
        .filter(|file| file.id != id)
        .flat_map(|file| file.requires.iter().map(|&(_, target)| target))
        .filter_map(|target| map.file(target).path().map(nvs_diagnostics::canonical_key))
        .collect();
    files
        .iter()
        .filter(|file| !required.contains(&nvs_diagnostics::canonical_key(file)))
        .cloned()
        .collect()
}

/// The entry a directory of programs is run through: one `require` per file,
/// in the order [`programs_under`] found them, each path relative to `dir` and
/// spelled with `/`, which a literal path accepts on every platform.
///
/// Under `nvs test` the entry's statements never run, so the requires only
/// bring each file into the compile. A file in the directory that declares
/// the `autoload` map, or requires the file that does, gives the whole
/// directory that map: run over a project's root, that is the project's own
/// entry file.
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
    /// `nvs check`'s: the terminal rendering, or with `json` the document on
    /// standard output, and what `--deny` reads kept in [`check::keep`].
    /// `list_todos` prints every todo; `read_todos` reads them for `--deny
    /// todo` without printing them.
    Check {
        json: bool,
        list_todos: bool,
        read_todos: bool,
    },
    /// One pass of `nvs check --fix`: nothing printed, and the safe edits kept
    /// in [`check::keep`].
    Fix,
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
        Sink::Check {
            json,
            list_todos,
            read_todos,
        } => {
            let todos = if list_todos || read_todos {
                check::todos(map)
            } else {
                Vec::new()
            };
            let listed = if list_todos { &todos[..] } else { &[] };
            if json {
                diags.sort_by_position();
                println!("{}", check::document(diags.iter(), map, listed));
            } else {
                render_diagnostics(diags, map);
                for todo in listed {
                    println!("{}", todo.line());
                }
            }
            check::keep(check::Found {
                deprecated: check::uses_deprecated(diags.iter(), map),
                todo: todos.iter().any(|todo| !todo.vendored),
                edits: Vec::new(),
            });
        }
        Sink::Fix => {
            diags.sort_by_position();
            check::keep(check::Found {
                edits: check::safe_edits(diags.iter(), map),
                ..check::Found::default()
            });
        }
        Sink::Hook => agent::keep_errors(diags, map),
    }
}

/// Hands `map` every `secret` literal of the program, so no diagnostic's
/// snippet prints one (`rule:security/redaction-reaches-the-tools-own-renderings`).
///
/// Only when there is a diagnostic to render: the walk is one tree per loaded
/// file, and a program that compiles clean prints no snippet to protect.
fn conceal_secret_literals(
    map: &mut SourceMap,
    diags: &Diagnostics,
    loaded: &[nvs_hir::requires::Loaded],
    exprs: &nvs_types::ExprTypeTable,
    interner: &nvs_types::TypeInterner,
) {
    if diags.is_empty() {
        return;
    }
    let spans: Vec<_> = loaded
        .iter()
        .flat_map(|file| {
            nvs_types::redaction::secret_literals(map.file(file.id), &file.stmts, exprs, interner)
        })
        .collect();
    map.conceal(spans);
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
    use super::{Cli, inbound_of, initializes, prepares, unaccepted};
    use clap::{CommandFactory as _, Parser as _};

    /// A command line as the words a shell would hand over: split on spaces,
    /// with a double-quoted run kept as one word.
    pub(crate) fn words(line: &str) -> Vec<String> {
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

    /// The banner's first line is `README.md`'s H1, word for word, as
    /// `rule:packaging/the-banner-states-the-build` says. The underline is the
    /// only thing the two may differ by.
    #[test]
    fn the_banner_opens_with_the_readme_title() {
        let readme = include_str!("../../../README.md");
        let title = readme
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .expect("README.md has an H1");
        let header = super::header();
        let first = header.lines().next().expect("a banner line");
        let name = super::Style::new().effects(super::Effects::UNDERLINE);
        let plain = first
            .replace(&name.render().to_string(), "")
            .replace(&name.render_reset().to_string(), "");
        assert_eq!(plain, title);
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

    /// The half of [`initializes`]'s table that writes no configuration file, read through the
    /// parser so the case names the command line rather than a variant: `nvs config check` and
    /// `nvs config dump` audit a tree, and an audit that creates the file it is auditing reports
    /// on its own output; `nvs lsp` resolves a tree per document and never writes one.
    ///
    /// [`prepares`] is asserted beside it: the project commands and `nvs lsp` create the data
    /// folder, and the read-only commands never do. The project commands are asserted too, because
    /// the decision this pins is the **split** — a table that silently lost a row would pass a case
    /// that only checked one side.
    #[test]
    fn config_check_and_dump_and_lsp_never_write() {
        let command = |argv: &[&str]| {
            Cli::try_parse_from(argv)
                .expect("the fixture is a command line `nvs` takes")
                .command
                .expect("a subcommand was named")
        };
        let writes = |argv: &[&str]| initializes(&command(argv));
        let creates = |argv: &[&str]| prepares(&command(argv));

        assert!(!writes(&["nvs", "config", "check"]));
        assert!(!writes(&["nvs", "config", "dump"]));
        assert!(!writes(&["nvs", "lsp"]));
        // The explicit door writes the file itself and resolves no tree, so it reaches no step 4
        // to be a project command at.
        assert!(!writes(&["nvs", "init"]));

        for project in [
            &["nvs", "run", "app.nvs"][..],
            &["nvs", "serve", "app.nvs"],
            &["nvs", "test", "app.nvs"],
            &["nvs", "build", "--openapi", "app.nvs"],
            &["nvs", "check", "app.nvs"],
            &["nvs", "dap"],
        ] {
            assert!(writes(project), "{project:?}");
            assert!(creates(project), "{project:?}");
        }

        assert!(creates(&["nvs", "lsp"]));
        assert!(creates(&["nvs", "debug"]));
        assert!(!writes(&["nvs", "debug"]));
        for read_only in [
            &["nvs", "config", "check"][..],
            &["nvs", "config", "dump"],
            &["nvs", "tmp", "clean"],
            &["nvs", "init"],
            &["nvs", "service", "status", "shop"],
        ] {
            assert!(!creates(read_only), "{read_only:?}");
        }
    }

    /// `--data` is global, so it parses before or after the subcommand, and a relative path
    /// resolves against the working directory while an absolute one is kept as it is.
    #[test]
    fn the_data_flag_resolves_against_the_working_directory() {
        let data = |argv: &[&str]| {
            Cli::try_parse_from(argv)
                .expect("the fixture is a command line `nvs` takes")
                .data
        };
        let cwd = std::path::PathBuf::from(if cfg!(windows) { r"C:\work" } else { "/work" });
        let absolute = cwd.join("elsewhere").join("data");

        let before = data(&["nvs", "--data", "local", "run", "app.nvs"]);
        assert_eq!(before.as_deref(), Some(std::path::Path::new("local")));
        let after = data(&["nvs", "run", "--data", "local", "app.nvs"]);
        assert_eq!(after, before, "the flag is global");
        let located = nvs_config::data::Folder::locate(before.as_deref(), &cwd)
            .expect("a named folder is always located");
        assert_eq!(located.root(), cwd.join("local"));
        assert_eq!(located.config_file(), cwd.join("local").join("nvs.toml"));

        let named = data(&[
            "nvs",
            "--data",
            absolute.to_str().expect("a UTF-8 fixture"),
            "check",
            "app.nvs",
        ]);
        let located = nvs_config::data::Folder::locate(named.as_deref(), &cwd)
            .expect("a named folder is always located");
        assert_eq!(located.root(), absolute, "an absolute path is kept");

        assert_eq!(data(&["nvs", "run", "app.nvs"]), None, "no flag, no path");
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
    /// `rule:concurrency/one-process-serves-requests-schedules-and-jobs`
    /// settles the question the sentence above raises: naming three subsystems is what a rename would have been asked to
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
