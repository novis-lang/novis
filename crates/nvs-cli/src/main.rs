//! The `nvs` binary.
//!
//! Nine subcommands so far, one per milestone that needed one:
//!
//! * `nvs ast` (M1) — dump what the parser produced.
//! * `nvs check` (M2) — parse, resolve, type-check, report every diagnostic.
//!   `--autoload-map` prints the resolved `autoload` map in place of the
//!   success line, which is
//!   [ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
//!   § 1's last sentence; the shape is `nvs_hir::autoload`'s module doc.
//! * `nvs run` (M3) — all of the above, then compile and execute. Its two
//!   dump flags stop one stage earlier and print instead of running:
//!   `--dump-ir` after lowering, `--dump-asm` after code generation.
//! * `nvs test` (M4) — run a tree of `.nvst` conformance cases, or a program's
//!   own `#[Test]` methods. ADR 0079 § 23 keeps the two formats apart and puts
//!   them under one subcommand; which is meant is read off the path. The
//!   `.nvst` format, and every decision behind it, is [`nvs_test`]'s own
//!   module doc, and this crate contributes only the argument parsing and the
//!   exit code; the `#[Test]` half is [`runner`].
//! * `nvs build --openapi` — the OpenAPI 3.1 document
//!   [ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)
//!   generates from the compile-time route table, on standard output. A build
//!   artifact and never a runtime feature; see [`openapi`] for what the table
//!   supplies and what it does not yet.
//! * `nvs build --compile` (M6) —
//!   [ADR 0048](../../../docs/adr/0048-portable-single-file-executables.md)'s
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
//!   [ADR 0103](../../../docs/adr/0103-configuration-is-a-tree-of-files.md)
//!   § 9's offline audit, which exists because § 3's later-wins precedence is
//!   only safe while it is auditable. It compiles nothing and needs no server;
//!   see [`config`], which also holds the reader `run` resolves that tree
//!   through.
//! * `nvs info` — build, host and third-party licensing facts, PHP's
//!   `php -i` in shape and in purpose. Also spelled `nvs -i`, since that is
//!   the spelling anyone arriving from PHP will try first; see [`info`].
//! * `nvs meta --json` — the `Core` registry as JSON: every class, every
//!   member, and each documented member's reference card, which is
//!   [ADR 0117](../../../docs/adr/0117-an-implemented-core-member-documents-itself-in-the-registry.md)
//!   § 2's contract. A build-time consumer's input, never a runtime feature;
//!   see [`meta`].
//!
//! `run` **checks first**: on any diagnostic it reports and exits non-zero
//! exactly as `check` does, rather than running a program the front end
//! rejected. `serve`, `fmt` and the rest of the architecture diagram
//! (`docs/implementation-plan.md` § Architecture) arrive with the milestones
//! that need them.
//!
//! ## What `run` executes
//!
//! The whole file, through `nvs_ir::lower::lower_file`: every class method
//! with a body, plus one synthesized frame for the file's own top-level
//! statements — [ADR 0008](../../../docs/adr/0008-static-and-global.md) § 2's
//! "the script body is a function, so its variables are locals". That frame is
//! the entry point; the methods are reachable from it by name.
//!
//! It runs **inside a task**, on a [`nvs_host::Scheduler`] of its own with a
//! reactor installed over it, rather than on the main thread's stack. That is
//! not about concurrency at the top level — there is one task — but about what
//! is beneath it: [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
//! § 1's children are children *of the calling task*, and a `Core\Task::all`
//! in a CLI program has nowhere to put them if the program is not one. The
//! root is `TaskRoot::Request`, so a panic that reaches it fails this run
//! rather than retiring anything
//! ([ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 2).

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this crate's whole job is user-facing terminal output"
)]

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser as ClapParser, Subcommand};
use nvs_diagnostics::{Diagnostics, Renderer, SourceMap};
use nvs_syntax::{check_declarations, parse_file};

mod api_diff;
mod bundle;
mod cache;
mod config;
mod info;
mod meta;
mod openapi;
mod queue;
mod runner;
mod script;
mod serve;
mod worker;

#[derive(ClapParser)]
#[command(
    name = "nvs",
    version,
    about = "The Novis compiler and CLI",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Read this configuration file instead of `./nvs.toml`, and repeat it to
    /// read several in order.
    ///
    /// [ADR 0103](../../../docs/adr/0103-configuration-is-a-tree-of-files.md)
    /// § 1 step 1: naming any file disables step 2 entirely, so an operator
    /// who names a tree never gets a surprise merge with whatever `nvs.toml`
    /// happens to be in the working directory. A path is resolved against that
    /// directory (§ 5), and one that does not exist is a refusal rather than a
    /// skipped root.
    ///
    /// Global, because it selects the tree rather than the command: `run`
    /// resolves the snapshot its request reads, and `config check`/`config
    /// dump` audit the same files without running anything.
    #[arg(long, value_name = "PATH", global = true)]
    config: Vec<PathBuf>,

    /// Print build, host and third-party licensing information.
    ///
    /// The same report as `nvs info`, under the spelling PHP uses.
    #[arg(short = 'i', long)]
    info: bool,

    /// With `-i`: include every third-party license text in full.
    #[arg(long, requires = "info")]
    licenses: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a `.nvs`/`.php` file and print its AST.
    Ast {
        /// The file to parse.
        file: PathBuf,
    },
    /// Parse, resolve and type-check a `.nvs`/`.php` file, reporting every
    /// diagnostic found.
    Check {
        /// The file to check.
        file: PathBuf,
        /// Print the resolved `autoload` map instead of `no errors`,
        /// including what a `discover` glob skipped and what was shadowed.
        #[arg(long)]
        autoload_map: bool,
    },
    /// Check a `.nvs`/`.php` file, then compile and run it.
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
        /// Deliberately scoped to `nvs run` and nothing else: a contained
        /// engine panic has no user-facing trigger by definition, so it needs
        /// a hook to be testable at all — and that hook must never be
        /// reachable from a served request. `nvs serve` (M7) does not get one.
        /// `nvs_runtime::FaultSite` documents each site.
        #[arg(long, value_name = "SITE")]
        fault_inject: Option<FaultSiteArg>,
        /// The program's own arguments — ADR 0086 § 6's command line, which
        /// `Core\Command::run()` matches against the compiled table.
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
        /// two. `tests/conformance/core/cli-arguments-hands-back-a-word-that-looks-like-syntax-as-the-value-it-is.nvst`
        /// pins both halves, since `Core\Cli::arguments` is where the
        /// difference is observable.
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "ARGS"
        )]
        arguments: Vec<String>,
    },
    /// Serve a `.nvs`/`.php` file over HTTP, on one core, until stopped.
    ///
    /// ADR 0097's development server and proxied origin. The file is compiled
    /// before the socket is bound and every request runs it as ADR 0006's
    /// isolate; § 4's mount table is the slice that replaces the argument with
    /// a set of entry points, and `serve`'s module doc owns why one path on the
    /// command line is already § 2's rule rather than an exception to it.
    Serve {
        /// The file every request runs.
        file: PathBuf,
        /// The address to listen on, as `host:port` — the last word over
        /// `[server] listen` (ADR 0097 § 5).
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
    /// ADR 0079 § 23: one subcommand runs both, because they answer different
    /// questions about the same tree, and which one is meant is read off the
    /// path — a `.nvs`/`.php` file is a program whose compiled test table is
    /// run (§ 1), anything else is a `.nvst` case file or a directory walked
    /// for `*.nvst`. The two are not mixed in one invocation: they report
    /// differently and share no summary.
    ///
    /// Exits non-zero if any case or any test failed; a skipped one is not a
    /// failure.
    Test {
        /// The case files and directories to run.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Run only the tests whose name contains this text — a `.nvst` case's
        /// path, or a `#[Test]` method's `Class::method`.
        #[arg(long, value_name = "TEXT")]
        filter: Option<String>,
        /// The PHP binary a `--ORACLE--` case is compared against.
        #[arg(long, value_name = "PATH", default_value = "php")]
        php: PathBuf,
        /// How a program's `#[Test]` run is reported (ADR 0079 § 22).
        ///
        /// The default is the human format, and it is what a `.nvst` tree is
        /// always reported in: `nvs_test`'s own report is a conformance
        /// summary rather than a suite of test methods, and § 23 keeps the two
        /// from sharing anything — so naming a machine format beside a case
        /// tree is refused rather than silently ignored.
        #[arg(long, value_name = "FORMAT", default_value = "human")]
        format: runner::Format,
    },
    /// Produce a build artifact from a checked program.
    ///
    /// Two artifacts, and naming one is required rather than defaulted: `nvs
    /// build` with nothing named would be a subcommand that succeeds having
    /// done nothing, and the group is how the next artifact joins without
    /// changing what this invocation means
    /// ([ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)
    /// § 3 spells the OpenAPI command,
    /// [ADR 0048](../../../docs/adr/0048-portable-single-file-executables.md)
    /// § 5 the bundle).
    #[command(group = clap::ArgGroup::new("artifact").required(true).args(["openapi", "compile"]))]
    Build {
        /// The entry point of the program to build.
        file: PathBuf,
        /// Write ADR 0085's OpenAPI 3.1 document to standard output.
        #[arg(long)]
        openapi: bool,
        /// Write ADR 0048's portable single-file executable: this program's
        /// `require` graph as source, appended to a copy of the `nvs` host
        /// binary.
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
    /// all ([ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)
    /// § 4).
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
    /// Audit the configuration tree without running anything.
    ///
    /// A namespace beside `api`, and deliberately not part of `nvs check`,
    /// which checks *source*:
    /// [ADR 0103](../../../docs/adr/0103-configuration-is-a-tree-of-files.md)
    /// § 9 separates the two. See [`config`].
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Build and inspect the durable job queue's own tables.
    ///
    /// A namespace of the operator's rather than the program's:
    /// [ADR 0084](../../../docs/adr/0084-durable-background-jobs.md) § 2 gives
    /// the runtime the queue's schema and has it created by an explicit command,
    /// never at boot and never from a request. See [`queue`].
    Queue {
        #[command(subcommand)]
        command: QueueCommand,
    },
    /// Print build, host and third-party licensing information.
    ///
    /// One call answers what this binary is and what is compiled into it,
    /// including the complete third-party attribution Novis's MIT license and
    /// its dependencies' licenses both require to be distributed with it.
    /// See `docs/adr/0065-third-party-attribution-and-nvs-info.md`.
    Info {
        /// Also print every third-party license text in full.
        #[arg(long)]
        licenses: bool,
    },
    /// Print the `Core` registry — every class and member, with each
    /// documented member's reference card.
    ///
    /// `--json` is required for the reason `build --openapi` is: a `meta`
    /// with nothing named would succeed having printed nothing, and the flag
    /// is how a second format joins without changing what this one means.
    /// The shape is ADR 0117 § 2's, and this command owns it; see [`meta`].
    Meta {
        /// Write the registry as JSON to standard output.
        #[arg(long, required = true)]
        json: bool,
    },
}

/// `nvs api`'s own subcommands.
///
/// One so far. It is a subcommand group rather than a bare `nvs diff` because
/// what is being diffed is the *API*, and a bare verb would own a name the
/// next artifact would want.
#[derive(Subcommand)]
enum ApiCommand {
    /// Classify every change between two OpenAPI documents, exiting non-zero on
    /// a breaking one.
    ///
    /// The gate of
    /// [ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)
    /// § 4: run it in CI against the document from the last release and a
    /// breaking change stops the build. See [`api_diff`] for what each class
    /// covers.
    Diff {
        /// The document to compare against — the last release's.
        old: PathBuf,
        /// The document this build produced.
        new: PathBuf,
    },
}

/// `nvs config`'s own subcommands.
///
/// ADR 0103 § 9 names three of these — `check`, `dump` and `ctl config` — and
/// this enum holds the two that are offline. `ctl config` belongs to the
/// control socket [ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md)
/// § 3 reserves and arrives with `nvs ctl`.
#[derive(Subcommand)]
enum ConfigCommand {
    /// Resolve the configuration tree and report what it holds, exiting
    /// non-zero on any refusal.
    ///
    /// Offline: it reads the files and nothing else, so a tree is validated in
    /// CI before it is deployed. What the audit deliberately does not assert is
    /// [`config`]'s own module doc.
    Check {
        /// The root files to read, in order.
        ///
        /// ADR 0103 § 1 step 1's list, given positionally: naming one disables
        /// step 2 exactly as `--config` does. With none, step 2's `./nvs.toml`
        /// is used, else step 3's shipped defaults.
        files: Vec<PathBuf>,
    },
    /// Print every key in force, one per line, in dotted-key order.
    ///
    /// ADR 0103 § 3 permits an include to override the file that pulled it in
    /// *on condition* that every override is recoverable; this is where it is
    /// recovered in full. What is printed, and the one thing deliberately
    /// absent from it, is [`config::dump`]'s own doc comment.
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
/// One today, and `migrate` is the one ADR 0084 § 2 names outright. Everything
/// else an operator might want of a queue — its depth, a job retried by hand —
/// is a question `Core\Queue::stats` already answers from inside a request, and
/// a second answer here would need this binary to open a connection for it,
/// which is the same wall [`queue`]'s own module doc describes.
#[derive(Subcommand)]
enum QueueCommand {
    /// Create ADR 0084 § 2's jobs and dead-letter tables in the queue's
    /// database.
    ///
    /// The statements are the runtime's own — `nvs_stdlib::queue`'s own lists,
    /// beside the members that read the columns — and this command is what
    /// makes issuing them an operator's act rather than a request's. What it
    /// can and cannot do today is [`queue`]'s module doc.
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
    },
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
    // ADR 0048 § 4, and it happens before clap sees anything: a bundled
    // executable's `argv` belongs to the program it carries, so an app whose
    // first argument is `run` or `--help` must not have it read as one of
    // ours. An ordinary `nvs` finds no footer and falls straight through.
    if let Some(payload) = bundle::embedded() {
        return bundle::run(payload);
    }

    let cli = Cli::parse();

    // `-i` and a subcommand are two requests, and guessing which one was
    // meant is worse than saying so. Clap cannot express this as a conflict
    // — a subcommand is not an argument it can name — so it is checked here.
    if cli.info && cli.command.is_some() {
        eprintln!("error: `-i`/`--info` cannot be combined with a subcommand");
        return ExitCode::FAILURE;
    }
    if cli.info {
        return info::run(cli.licenses);
    }

    // Unreachable: `arg_required_else_help` makes a bare `nvs` print help.
    let Some(command) = cli.command else {
        return ExitCode::FAILURE;
    };

    match command {
        Command::Ast { file } => run_ast(&file),
        Command::Check { file, autoload_map } => run_check(&file, autoload_map),
        Command::Run {
            file,
            dump_ir,
            dump_asm,
            fault_inject,
            arguments,
        } => run_run(
            &file,
            dump_ir,
            dump_asm,
            fault_inject,
            &cli.config,
            arguments,
        ),
        Command::Serve { file, listen, port } => {
            serve::run(&file, listen.as_deref(), port, &cli.config)
        }
        Command::Test {
            paths,
            filter,
            php,
            format,
        } => run_test(&paths, filter, php, format),
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
                },
        } => queue::migrate(&cli.config, &files, connection.as_deref(), dry_run),
        Command::Info { licenses } => info::run(licenses),
        Command::Meta { json: _ } => meta::run(),
    }
}

fn run_ast(path: &std::path::Path) -> ExitCode {
    let mut map = SourceMap::new();
    let id = match map.load(path) {
        Ok(id) => id,
        Err(err) => {
            eprintln!("error: could not read {}: {err}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    check_declarations(&stmts, map.file(id), &mut diags);

    render_diagnostics(&mut diags, &map);
    println!("{stmts:#?}");

    if diags.has_errors() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// A **program** that has been through the whole front end with no error.
///
/// `run` needs everything `check` produces plus the three tables `nvs-ir`
/// lowering reads back — the resolved-target table, the type interner that
/// backs it, and the class-layout table — so the pipeline is shared rather
/// than written twice.
struct Checked {
    map: SourceMap,
    /// The entry point's own file. It names the program (`nvs run <path>`),
    /// so it stays a single id even though `files` is now a set.
    id: nvs_diagnostics::SourceId,
    /// Every file the entry point's `require`/`autoload` graph reached, the
    /// entry file first — `nvs_hir::resolve_program`'s order contract.
    files: Vec<nvs_hir::Loaded>,
    interner: nvs_types::TypeInterner,
    exprs: nvs_types::ExprTypeTable,
    /// Every declared enum's backing type and its cases' values, handed back
    /// by `check_program` rather than rebuilt — `nvs_ir::lower` needs a case's
    /// constant for ADR 0047 § 3's membership test.
    enums: nvs_types::EnumTable,
    layouts: nvs_types::ClassLayoutTable,
    /// The autoload map the graph walk consulted, kept for
    /// `check --autoload-map` and read by nothing else here.
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
/// statements of every file it loaded (ADR 0021, ADR 0061), and each table
/// below is then built across that set — a class declared in a `require`d
/// file has to be a class the entry file's body can name.
///
/// `Err` is the exit code to return: a read failure, or at least one error
/// diagnostic. Warnings are rendered and do not stop anything.
fn front_end(path: &std::path::Path) -> Result<Checked, ExitCode> {
    let mut map = SourceMap::new();
    let id = match map.load(path) {
        Ok(id) => id,
        Err(err) => {
            eprintln!("error: could not read {}: {err}", path.display());
            return Err(ExitCode::FAILURE);
        }
    };

    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    check_declarations(&stmts, map.file(id), &mut diags);
    // Every other file's parse and `check_declarations` happen inside the
    // walk, as each `require` target is discovered; only the entry point is
    // this function's to load.
    let (module, loaded, autoload) = nvs_hir::resolve_program(id, stmts, &mut map, &mut diags);

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
        let enums =
            nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);

        if diags.has_errors() {
            render_diagnostics(&mut diags, &map);
            return Err(ExitCode::FAILURE);
        }
        (enums, nvs_types::build_class_layouts(&files, &module.graph))
    };
    render_diagnostics(&mut diags, &map);

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

/// `nvs check`, and with `--autoload-map` also ADR 0061 § 1's last sentence:
/// the resolved prefix → roots map, what a `discover` glob passed over and
/// what an explicit prefix shadowed.
///
/// The map is printed only when the check succeeded, because a program that
/// does not resolve has not necessarily finished building one — the walk
/// stops probing the moment the `require` graph is in doubt, and printing a
/// partial map beside a wall of errors would be read as the whole of it.
/// Paths are shown relative to the entry point's own directory, which is what
/// `autoload`'s literals are written against (§ 1).
fn run_check(path: &std::path::Path, autoload_map: bool) -> ExitCode {
    match front_end(path) {
        Ok(checked) => {
            if autoload_map {
                let base = match path.parent() {
                    Some(dir) if !dir.as_os_str().is_empty() => dir,
                    _ => std::path::Path::new("."),
                };
                print!("{}", checked.autoload.render(base));
            } else {
                println!("no errors");
            }
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

/// `nvs build --openapi` — [ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)
/// § 3's emission, on standard output.
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

/// ADR 0086 § 6's table, as the runtime carries it.
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
                        conv: match arg.conv {
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

/// The label the script frame is compiled and looked up under.
///
/// `nvs_ir::lower::lower_script` leaves the name to its caller; this is the
/// same spelling `nvs-ir`'s own snapshots use.
const SCRIPT: &str = "<script>";

fn run_run(
    path: &std::path::Path,
    dump_ir: bool,
    dump_asm: bool,
    fault_inject: Option<FaultSiteArg>,
    config: &[PathBuf],
    arguments: Vec<String>,
) -> ExitCode {
    let checked = match front_end(path) {
        Ok(checked) => checked,
        Err(code) => return code,
    };
    let src = checked.map.file(checked.id);

    let program = nvs_ir::lower::lower_program(
        SCRIPT,
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
    let unit = match nvs_codegen::compile(&program) {
        Ok(unit) => unit,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let Some(entry) = unit.function(SCRIPT) else {
        eprintln!("internal error: the script frame was not compiled");
        return ExitCode::FAILURE;
    };

    // ADR 0078 § 1: the tree is resolved and folded into one snapshot **before**
    // the request exists, and the request then clones it once. A refusal here is
    // a refusal to start — ADR 0103 § 3's later-wins and § 6's boundary are only
    // worth anything if a tree that does not resolve stops the run.
    let mut config_sources = SourceMap::new();
    // A bundled program's entry file is a synthetic path inside the payload
    // (ADR 0048 § 4), and `trust::canonical` has no filesystem entry to
    // examine for it. The executable itself is what an `[[app]]` block could
    // legitimately key on, and it is also all § 1's single trust domain
    // leaves to key on: the only principal here is whoever ran the binary.
    let config_entry = if nvs_diagnostics::embedded::is_active() {
        std::env::current_exe().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    let snapshot = match config::boot_snapshot(config, &config_entry, &mut config_sources) {
        Ok(snapshot) => snapshot,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &config_sources);
            return ExitCode::FAILURE;
        }
    };
    // ADR 0103 § 7's advisories: a secret file another account can read is
    // reported and does not stop the run.
    if !snapshot.warnings.is_empty() {
        let mut diags = Diagnostics::new();
        for warning in &snapshot.warnings {
            diags.report(warning.clone());
        }
        render_diagnostics(&mut diags, &config_sources);
    }

    // ADR 0084 § 2's `workers` is per *instance*, and a CLI run is one — so a
    // run of this tree claims jobs beside its script, including ones another
    // instance enqueued and never finished. Read here rather than inside the
    // worker because the snapshot is moved onto the context a dozen lines
    // below, and `queue_for` is the same resolution boot already accepted
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
    // ADR 0102 § 6's origin is resolved before the program runs and never
    // during it, which is the whole of what makes it un-sniffable. It comes off
    // the snapshot, so it is the origin of the `[[app]]` blocks that actually
    // match this entry file (ADR 0104 § 2) and not of any block in the file.
    if let Some(origin) = snapshot.origin.clone() {
        ctx.set_origin(&origin);
    }
    // The workers get their own handle on the same tree, taken before it is moved onto this
    // context: a job is an isolate resolved through ADR 0118 § 2's spawn door, that door asks the
    // *context* it is resolved from, and a worker's context is not the script's. `worker`'s module
    // doc owns why the deployment's own snapshot is the right answer there.
    let for_workers = queued.is_some().then(|| std::sync::Arc::clone(&snapshot));
    ctx.set_config(snapshot);
    // ADR 0086 § 6: `Core\Command`'s members are generated from the table the
    // front end already built, so the rows cross here — once, before the program
    // starts, like everything else this context is handed.
    // `nvs_runtime::commands` owns why they cross as a runtime value rather than
    // being folded while checking.
    let commands = checked.exprs.commands();
    if !commands.rows().is_empty() {
        ctx.set_commands(std::sync::Arc::new(runtime_commands(commands)));
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
    // member: ADR 0118 § 2 keeps `argv[0]` out of `nvs-stdlib`, and a served
    // request has no name to give it anyway.
    ctx.set_program_name(
        config_entry
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    // Hands the context the unit's class table: the class a helper's
    // bare-message failure is promoted to, and the shared ownership that lets
    // the context outlive the unit. `Unit::install_in` owns both reasons.
    unit.install_in(&mut ctx);
    if let Some(site) = fault_inject {
        ctx.inject_fault(site.into());
    }
    // ADR 0072 §§ 1 and 3: the program is a *task*, because the children a
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
    // Two things have to come back out of the task, and they come back by
    // different routes. The call's status is written into a cell the body
    // captures, since a task's body returns nothing; the `Ctx` arrives in the
    // `Finished` the scheduler hands back, because it was moved into the task
    // rather than borrowed by it, and everything reported below is read off
    // that one.
    let status: std::rc::Rc<std::cell::Cell<Option<Result<(), i32>>>> =
        std::rc::Rc::new(std::cell::Cell::new(None));
    let root = sched.spawn(ctx, nvs_runtime::TaskRoot::Request, {
        let status = std::rc::Rc::clone(&status);
        let workers = workers.clone();
        move |ctx| {
            // The returned value is discarded exactly as it was when this was a
            // direct call: the script frame answers with null.
            let outcome = nvs_runtime::call(entry, ctx, &[]).map(|_| ());
            // ADR 0072 § 6: a CLI run has no response, so the script's own
            // frame returning is when "after the response" is —
            // `nvs_runtime::deferred` owns that reading and why a request that
            // did not return ordinarily runs none of its deferred work. It runs
            // *inside* the task, because a deferred closure is a task like any
            // other and a child of one is spawned off the caller the scheduler
            // is holding.
            if outcome.is_ok() {
                // ADR 0127 § 4: the last statement, then the queue, then
                // teardown — and *before* the deferred work below, because
                // ADR 0072 § 6's `afterResponse` is what runs after the
                // response and this queue is what delays the end of one.
                nvs_stdlib::script::run_exit_hooks(ctx, outcome, None);
                nvs_runtime::deferred::run_deferred(ctx);
            } else if outcome == Err(nvs_runtime::THROWN) {
                // ADR 0020 §§ 3 and 6: nothing below caught this, so the ladder
                // is climbed from tier 3 — the operator's own `.nvs`, run as an
                // isolate — and only when there is no handler, or it failed,
                // does the floor report the record itself. Zero retries, and
                // one record: what tier 3 is handed is exactly what tier 4
                // would have written.
                //
                // It happens **here, inside the task**, rather than beside the
                // exit code below, because tier 3 is an isolate and an isolate
                // wants the scheduler, the reactor and the script resolver this
                // run installed — all three of which are taken down with the
                // run itself, a dozen lines before the exit code is decided.
                let thrown = ctx.take_thrown();
                // ADR 0020 § 2's tier 2, and this is the "request root" that
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
                // ADR 0127 § 4's throw path, at its strongest reading: the
                // failure hooks run first "so a misbehaving queue cannot starve
                // the failure report", and the record above *is* that report —
                // so the queue runs after tier 3 and the floor as well as after
                // tier 2, and still before native teardown. It is handed the
                // same live `Throwable` tier 2 was.
                nvs_stdlib::script::run_exit_hooks(ctx, outcome, Some(&thrown));
            } else {
                // `exit` drains the queue and a `FATAL` runs none of it —
                // `nvs_stdlib::script::run_exit_hooks` is the one place that
                // decides which, per ADR 0127 §§ 2 and 3, so this arm asks
                // nothing about the status it is passing on.
                nvs_stdlib::script::run_exit_hooks(ctx, outcome, None);
            }
            status.set(Some(outcome));
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

    // And the workers, after the task above rather than before it. The order is
    // a cost and not a preference: a worker's first act is a database handshake
    // and the run queue is FIFO, so a program that never parks would otherwise
    // wait one out before it could exit. `worker`'s module doc has the two
    // numbers.
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
    // ADR 0006's isolate runs another file, and this is the only crate that can
    // turn a path into one — `script`'s module doc owns the two decisions in
    // it, and `nvs_runtime::script` owns why the edge runs this way round.
    // Installed for the whole run rather than per spawn: the unit cache behind
    // it is what makes a second isolate over one path share compiled code.
    // Held on this stack for the length of the run rather than leaked: `scoped`
    // owns why the seam's `&'static` does not oblige a `Box::leak`, and the unit
    // cache goes down with it here.
    let compiler = script::Compiler::default();
    let ran = nvs_runtime::script::scoped(&compiler, || nvs_host::run_until_idle(&mut sched));
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

    // Flushed before anything is reported: Rust's standard output is
    // line-buffered, and `echo "Hello, World!"` has no trailing newline.
    if let Err(error) = ctx.flush_output() {
        eprintln!("error: could not flush output: {error}");
        return ExitCode::FAILURE;
    }

    // ADR 0106 § 2's outer boundary caught a panic under the task root. For a
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
            // A `FATAL`, which never reaches tiers 1 and 2 (ADR 0020 § 5) and
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
fn run_test(
    paths: &[PathBuf],
    filter: Option<String>,
    php: PathBuf,
    format: runner::Format,
) -> ExitCode {
    // ADR 0079 § 23's "`nvs test` runs both", decided by the path rather than
    // by a flag: a program is a `.nvs`/`.php` file and a conformance case is
    // not, so nothing has to be spelled out at the call site.
    if paths.iter().any(|path| is_program(path)) {
        let [path] = paths else {
            eprintln!("error: a program's `#[Test]` methods and `.nvst` cases are run separately");
            return ExitCode::FAILURE;
        };
        // `--filter` reaches both suites, and means the same thing in each:
        // `runner::selected` owns the rule and why it is the `.nvst` tree's.
        return match front_end(path) {
            Ok(checked) => runner::run(checked, format, filter),
            Err(code) => code,
        };
    }
    if format != runner::Format::Human {
        // § 22's formats report a `#[Test]` run, and § 23 keeps the two suites
        // from sharing a summary — so a machine format over a `.nvst` tree
        // names a document this subcommand does not produce.
        eprintln!("error: `--format` reports a program's `#[Test]` methods, not a `.nvst` tree");
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

/// Whether `path` names an Novis **program** rather than a `.nvst` case tree —
/// the two spellings `nvs run` itself accepts, and no directory, since a
/// directory of programs has no entry point to check.
fn is_program(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nvs") || ext.eq_ignore_ascii_case("php"))
}

fn render_diagnostics(diags: &mut Diagnostics, map: &SourceMap) {
    if diags.is_empty() {
        return;
    }
    diags.sort_by_position();
    let renderer = Renderer::new()
        .with_color(std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal());
    let mut out = Vec::new();
    renderer
        .render_all(diags.iter(), map, &mut out)
        .expect("rendering to an in-memory buffer cannot fail");
    eprint!("{}", String::from_utf8_lossy(&out));
}
