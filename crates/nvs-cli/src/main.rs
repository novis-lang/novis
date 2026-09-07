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
//! * `nvs info` — build, host and third-party licensing facts, PHP's
//!   `php -i` in shape and in purpose. Also spelled `nvs -i`, since that is
//!   the spelling anyone arriving from PHP will try first; see [`info`].
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
//! * `nvs serve <file>` — one core, one listening socket, every request
//!   running the entry file as its own isolate; see [`serve`].
//! * `nvs queue migrate` — `rule:core-classes/queue-storage-is-a-table`'s
//!   two tables, created by the operator's explicit command; see [`queue`],
//!   and [`worker`] for the in-process worker `[queue] workers` starts.
//! * `nvs schema plan|apply|dump` — `rule:core-classes/schema-converges`'s
//!   convergence as a command: the difference between a schema value and a live
//!   database, printed with every step's grade and SQL, applied, or read back
//!   out of the database; see [`schema`].
//! * `nvs service` — `rule:packaging/a-service-is-one-stored-argv`'s
//!   operator surface: the one argv a service manager stores; see [`service`].
//!
//! `run` **checks first**: on any diagnostic it reports and exits non-zero
//! exactly as `check` does, rather than running a program the front end
//! rejected. `fmt` and the rest of the architecture diagram
//! (`docs/implementation-plan.md` § Architecture) arrive with the milestones
//! that need them.
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

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser as ClapParser, Subcommand};
use nvs_diagnostics::{Diagnostics, Renderer, SourceMap};
use nvs_syntax::{check_declarations, parse_file};

mod api_diff;
mod ast;
mod bundle;
mod cache;
mod config;
mod doc;
mod info;
mod meta;
mod openapi;
mod queue;
mod runner;
mod schema;
mod script;
mod serve;
mod service;
mod tmp;
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
    /// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
    /// step 1: naming any file disables step 2 entirely, so an operator
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
}

#[derive(Subcommand)]
enum Command {
    /// Parse a `.nvs`/`.php` file and print its AST.
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
    /// Parse, resolve and type-check a `.nvs`/`.php` file, reporting every
    /// diagnostic found.
    Check {
        /// The file to check.
        file: PathBuf,
        /// Print the resolved `autoload` map instead of `no errors`,
        /// including what a `discover` glob skipped and what was shadowed.
        #[arg(long)]
        autoload_map: bool,
        /// Also report every public member with no `///` doc comment above it.
        /// Off by default, in every project.
        #[arg(long)]
        strict_docs: bool,
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
        /// The program's own arguments — `rule:tooling/commands-are-compiled`'s command line, which
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
    /// `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s development server and proxied origin. The file is compiled
    /// before the socket is bound and every request runs it as `rule:security/isolate-shares-nothing`'s
    /// isolate; § 4's mount table is the slice that replaces the argument with
    /// a set of entry points, and `serve`'s module doc owns why one path on the
    /// command line is already § 2's rule rather than an exception to it.
    Serve {
        /// The file every request runs.
        file: PathBuf,
        /// The address to listen on, as `host:port` — the last word over
        /// `[server] listen` (`rule:http-server/the-server-block-is-boot-class`).
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
    /// `rule:testing/nvst-is-separate`: one subcommand runs both, because they answer different
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
        /// How many `.nvst` cases run at once; the default is this machine's
        /// hardware threads. A program's `#[Test]` methods are one process and
        /// are not spread.
        #[arg(long, value_name = "N")]
        jobs: Option<std::num::NonZeroUsize>,
        /// How a program's `#[Test]` run is reported (`rule:testing/report-formats`).
        ///
        /// The default is the human format, and it is what a `.nvst` tree is
        /// always reported in: `nvs_test`'s own report is a conformance
        /// summary rather than a suite of test methods, and § 23 keeps the two
        /// from sharing anything — so naming a machine format beside a case
        /// tree is refused rather than silently ignored.
        #[arg(long, value_name = "FORMAT", default_value = "human")]
        format: runner::Format,
        /// Rewrite each failed `Core\Test::assertMatchesInline` snapshot in
        /// the source that wrote it (`rule:testing/inline-snapshots`).
        ///
        /// This is the only spelling under which `nvs test` writes to a file at
        /// all, and what it writes is the `$expected` literal and nothing else:
        /// a run without it never touches the tree, and a run with it never
        /// touches a passing snapshot. A `.nvst` tree has no snapshot to
        /// update, so naming it there is refused rather than ignored.
        #[arg(long)]
        update: bool,
    },
    /// Produce a build artifact from a checked program.
    ///
    /// Two artifacts, and naming one is required rather than defaulted: `nvs
    /// build` with nothing named would be a subcommand that succeeds having
    /// done nothing, and the group is how the next artifact joins without
    /// changing what this invocation means
    /// (`rule:routing/api-document-is-a-deterministic-build-artifact`
    /// spells the OpenAPI command,
    /// `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
    /// the bundle).
    #[command(group = clap::ArgGroup::new("artifact").required(true).args(["openapi", "compile"]))]
    Build {
        /// The entry point of the program to build.
        file: PathBuf,
        /// Write `rule:routing/api-document-is-generated-from-the-route-table`'s OpenAPI 3.1 document to standard output.
        #[arg(long)]
        openapi: bool,
        /// Write `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s portable single-file executable: this program's
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
    /// all (`rule:routing/api-diff-fails-a-breaking-change`
    /// ).
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
    /// Audit the configuration tree without running anything.
    ///
    /// A namespace beside `api`, and deliberately not part of `nvs check`,
    /// which checks *source*:
    /// `rule:config/check-and-dump-audit-the-tree-offline`
    /// separates the two. See [`config`].
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Build and inspect the durable job queue's own tables.
    ///
    /// A namespace of the operator's rather than the program's:
    /// `rule:core-classes/queue-storage-is-a-table` gives
    /// the runtime the queue's schema and has it created by an explicit command,
    /// never at boot and never from a request. See [`queue`].
    Queue {
        #[command(subcommand)]
        command: QueueCommand,
    },
    /// Converge a database on a schema value, or read the one it already holds.
    ///
    /// The operator's spelling of `Core\Db\Schema`:
    /// `rule:core-classes/schema-converges` computes the difference against the
    /// server every time, so there is no migration to order and no history to
    /// keep. See [`schema`].
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Clear what a hard-killed script left in the temporary root.
    ///
    /// The operator's half of
    /// `rule:core-classes/temporary-dir-orphan-sweep`: the runtime deletes a temporary directory when its script ends and
    /// reclaims the rest at `nvs serve` boot, and this is how a machine that
    /// never boots a server clears them. See [`tmp`].
    Tmp {
        #[command(subcommand)]
        command: TmpCommand,
    },
    /// Register this binary with the platform's service manager, or print what
    /// registering it would store.
    ///
    /// A namespace matching `nvs ctl`'s precedent
    /// (`rule:packaging/a-service-is-one-stored-argv`
    /// ): every other subcommand acts on files with no server involved, and
    /// these do not. See [`service`], whose module doc owns which half of § 1
    /// is on disk and why the other half is not.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    /// Speak the Language Server Protocol on standard input and output.
    ///
    /// Started by an editor, not by a person: it reads LSP 3.17 frames on
    /// stdin and writes them on stdout, so a terminal that runs it sees
    /// nothing and appears to hang. `rule:ide/one-server-two-thin-clients`
    /// makes this the one implementation of Novis's language smarts, which
    /// every editor client is a shell around.
    ///
    /// Takes no arguments. Everything configurable arrives in `initialize`
    /// and in `workspace/didChangeConfiguration`, because the editor is what
    /// owns the settings and a flag here would be a second, staler copy.
    Lsp,
    /// Run a tree of `.lspt` cases against the language server.
    ///
    /// The editor-behaviour suite: each case is a document, a cursor, a request
    /// and the answer frozen as text
    /// (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). It answers a
    /// different question from `nvs test` and shares no summary with it — two
    /// suites, two counts, because a number that meant both would mean neither.
    ///
    /// Prints `N passed, M failed` and exits non-zero if any case failed.
    LspTest {
        /// The case files and directories to run.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Print build, host and third-party licensing information.
    ///
    /// One call answers what this binary is and what is compiled into it,
    /// including the complete third-party attribution Novis's MIT license and
    /// its dependencies' licenses both require to be distributed with it.
    /// See `docs/decisions/0065.md`.
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
    /// The shape is `rule:tooling/meta-json`'s, and this command owns it; see [`meta`].
    Meta {
        /// Write the registry as JSON to standard output.
        #[arg(long, required = true)]
        json: bool,
        /// A program's entry point, whose own declarations join the registry
        /// under a `program` key (`rule:tooling/meta-json-takes-a-program`).
        /// Omitted, the document is the registry alone and is byte-identical
        /// to what it was before this argument existed.
        entry: Option<PathBuf>,
    },
    /// Write this program's declarations as one Markdown page per class.
    ///
    /// A renderer over `nvs meta --json`'s document and nothing else
    /// (`rule:tooling/nvs-doc-renders-and-decides-nothing`), for a project that
    /// does not have a documentation pipeline of its own; see [`doc`].
    Doc {
        /// The program's entry point.
        file: PathBuf,
        /// The directory the pages are written to, created if it is absent.
        #[arg(long, default_value = "doc")]
        out: PathBuf,
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
    /// The gate of
    /// `rule:routing/api-diff-fails-a-breaking-change`
    /// : run it in CI against the document from the last release and a
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
    /// CI before it is deployed. What the audit deliberately does not assert is
    /// [`config`]'s own module doc.
    Check {
        /// The root files to read, in order.
        ///
        /// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 1's list, given positionally: naming one disables
        /// step 2 exactly as `--config` does. With none, step 2's `./nvs.toml`
        /// is used, else step 3's shipped defaults.
        files: Vec<PathBuf>,
    },
    /// Print every key in force, one per line, in dotted-key order.
    ///
    /// `rule:config/later-wins-and-every-override-is-recorded` permits an include to override the file that pulled it in
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
/// `migrate` is the one `rule:core-classes/queue-storage-is-a-table` names outright. Everything
/// else an operator might want of a queue — its depth, a job retried by hand —
/// is a question `Core\Queue::stats` already answers from inside a request, and
/// a second answer here would need this binary to open a connection for it,
/// which is the same wall [`queue`]'s own module doc describes.
#[derive(Subcommand)]
enum QueueCommand {
    /// Create `rule:core-classes/queue-storage-is-a-table`'s jobs and dead-letter tables in the queue's
    /// database.
    ///
    /// The schema is the runtime's own — `nvs_stdlib::queue::schema`, beside the
    /// members that read the columns — and this command is what makes issuing
    /// DDL for it an operator's act rather than a request's. It converges: what
    /// runs is the difference between that value and the database. What it can
    /// and cannot do today is [`queue`]'s module doc.
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
    /// SQL — including the steps `apply` refuses, which is
    /// `rule:core-classes/schema-plan`'s rule and the reason a plan is useful
    /// against a database this deployment may not write to at all.
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
    /// not `Safe` and names the first one, which is
    /// `rule:core-classes/schema-apply-capability`'s `applySafe` on a command
    /// line. A report is never run either way.
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
    /// directories are untouchable however old they are. What it can and cannot
    /// do, and why there is no force flag, is [`tmp`]'s module doc.
    Clean {
        /// Print what would be removed and remove nothing.
        #[arg(long)]
        dry_run: bool,
    },
}

/// `nvs service`'s own subcommands.
///
/// `unit` is the one `rule:packaging/the-unit-is-printed-and-install-is-the-opt-in` makes the default on Linux:
/// generate the unit and **print** it, because the operator's configuration
/// management already owns the directory it belongs in and a binary that writes
/// there behind Ansible's back is a worse citizen than one that prints. On
/// Windows it emits § 5's equivalent `New-Service` invocation, carrying § 3's
/// encoded `ImagePath` for review rather than execution.
///
/// Every § 2 refusal runs in front of it, so `unit` is also how an operator
/// finds out that the argv they were about to install would have been refused —
/// without an elevated shell, and without having installed anything.
/// [`service`]'s module doc owns why `install`, `uninstall`, `start`, `stop`,
/// `status` and `run` are not here yet.
#[derive(Subcommand)]
enum ServiceCommand {
    /// Print the service definition this argv would be installed as, and
    /// install nothing.
    Unit {
        /// The service's name — the identity `sc create` and systemd use, and
        /// the one `nvs ctl --socket` addresses one of several servers by.
        name: String,
        /// Where the service writes diagnostics, if the named configuration
        /// does not say (§ 2).
        #[arg(long, value_name = "PATH")]
        log_file: Option<PathBuf>,
        /// The account the service runs as. The default is § 4's per-service
        /// virtual account, which has no password to rotate or leak.
        #[arg(long, value_name = "ACCOUNT")]
        account: Option<String>,
        /// Refused (`E0633`). It exists so the refusal can name it: a command
        /// line is readable by other users on the box, so an account password
        /// is prompted for instead.
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,
        /// The `nvs` arguments to store, verbatim.
        ///
        /// `--` is mandatory and is what makes § 1's "every parameter is
        /// passable" true: everything to its left is the installer's own,
        /// everything to its right is stored untouched and never interpreted.
        /// Without it a `--start` would be ambiguous between the installer and
        /// the hosted program — a defect `mysqld --install` has and one Novis
        /// does not inherit.
        #[arg(
            last = true,
            required = true,
            allow_hyphen_values = true,
            value_name = "ARGS"
        )]
        argv: Vec<String>,
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
            autoload_map,
            strict_docs,
        } => run_check(&cli.config, &file, autoload_map, strict_docs),
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
            jobs,
            format,
            update,
        } => run_test(&paths, filter, php, jobs, format, update, &cli.config),
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
        Command::Service {
            command:
                ServiceCommand::Unit {
                    name,
                    log_file,
                    account,
                    password,
                    argv,
                },
        } => service::print_unit(
            &cli.config,
            &name,
            &argv,
            log_file.as_deref(),
            account.as_deref(),
            password.as_deref(),
        ),
        Command::Lsp => match nvs_lsp::run() {
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
        Command::LspTest { paths } => {
            // The report is this terminal program's own output, which is where
            // a `.lspt` summary belongs: `rule:ide/stdout-belongs-to-the-protocol` gives stdout to
            // the protocol only in the process `nvs lsp` runs, and this is not
            // that process — `nvs_lsp::suite` writes to the sink it is handed
            // and names none.
            let mut out = std::io::stdout().lock();
            match nvs_lsp::suite::run(&paths, &mut out) {
                Ok(summary) if summary.is_success() => ExitCode::SUCCESS,
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
/// statements of every file it loaded (`rule:statements/require-is-the-only-inclusion-construct`, `rule:programs/no-runtime-autoload`), and each table
/// below is then built across that set — a class declared in a `require`d
/// file has to be a class the entry file's body can name.
///
/// `Err` is the exit code to return: a read failure, or at least one error
/// diagnostic. Warnings are rendered and do not stop anything.
fn front_end(path: &std::path::Path) -> Result<Checked, ExitCode> {
    front_end_granted(path, None, false)
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
/// whatever the program's own diagnostics happen to be. That answers
/// `nvs_types::intrinsics`' gap 6: checking has a configuration in front of it.
fn front_end_granted(
    path: &std::path::Path,
    config: Option<&[std::path::PathBuf]>,
    strict_docs: bool,
) -> Result<Checked, ExitCode> {
    let mut map = SourceMap::new();
    let id = match map.load(path) {
        Ok(id) => id,
        Err(err) => {
            eprintln!("error: could not read {}: {err}", path.display());
            return Err(ExitCode::FAILURE);
        }
    };

    // Read after the entry file and before anything is parsed: a missing program
    // is still "could not read", and a broken `nvs.toml` is the configuration
    // error rather than the first thing the parser noticed.
    let grants = match config {
        Some(config) => config::grants(config, path)?,
        None => None,
    };

    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    check_declarations(&stmts, map.file(id), &mut diags);
    // Every other file's parse and `check_declarations` happen inside the
    // walk, as each `require` target is discovered; only the entry point is
    // this function's to load.
    let (module, loaded, autoload) =
        nvs_hir::resolve_program_linted(id, stmts, &mut map, &mut diags, strict_docs);

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
fn run_check(
    config: &[std::path::PathBuf],
    path: &std::path::Path,
    autoload_map: bool,
    strict_docs: bool,
) -> ExitCode {
    match front_end_granted(path, Some(config), strict_docs) {
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
                            nvs_types::commands::ArgConv::Uuid => {
                                nvs_runtime::commands::ArgConv::Uuid
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
                                        .map(|(case, value)| {
                                            (
                                                case.clone(),
                                                match value {
                                                    nvs_types::enums::EnumValue::Int(number) => {
                                                        nvs_runtime::commands::CaseValue::Int(
                                                            *number,
                                                        )
                                                    }
                                                    nvs_types::enums::EnumValue::Uint(number) => {
                                                        nvs_runtime::commands::CaseValue::Uint(
                                                            *number,
                                                        )
                                                    }
                                                },
                                            )
                                        })
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
pub(crate) fn runtime_routes(table: &nvs_types::RouteTable) -> nvs_runtime::routes::Routes {
    nvs_runtime::routes::Routes::new(
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
    )
}

/// Which conversion § 5's declared type is, as the matcher spells it.
///
/// A closed set of *values* takes precedence over the type that describes it —
/// a union of string literals renders as a type nothing would convert, and its
/// admitted set is the whole of what § 5 narrows with. Everything the runtime
/// has no arm for is `Unconverted` rather than silently `Text`, so the gap is
/// one an arm closes rather than a behaviour somebody has to notice.
fn capture_conv(param: &nvs_types::RouteParam) -> nvs_runtime::routes::CaptureConv {
    use nvs_runtime::routes::CaptureConv;

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
        Some(_) => CaptureConv::Unconverted,
    }
}

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
    let snapshot = match config::boot_snapshot(config, &config_entry, &mut config_sources) {
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
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    // Shared rather than owned outright, because `rule:testing/in-process-request`'s in-process
    // request runs this same unit's script frame as a child isolate and the
    // seam holding it outlives no part of this run — `runner::UnderTest` is the
    // one holder, and an `Rc` is what lets the run and the seam both name it.
    let unit = std::rc::Rc::new(unit);
    let Some(entry) = unit.script() else {
        eprintln!("internal error: the script frame was not compiled");
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
    // The workers get their own handle on the same tree, taken before it is moved onto this
    // context: a job is an isolate resolved through `rule:security/capability-check-at-the-door`'s spawn door, that door asks the
    // *context* it is resolved from, and a worker's context is not the script's. `worker`'s module
    // doc owns why the deployment's own snapshot is the right answer there.
    let for_workers = queued.is_some().then(|| std::sync::Arc::clone(&snapshot));
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
    // `rule:routing/matched-once-before-the-handler`: the same crossing one table along. A program run off the
    // command line is matched against nothing — there is no request — but
    // `Core\Router`'s own members read the table, so it is installed wherever a
    // program runs rather than only where a server is answering.
    let routes = checked.exprs.routes();
    if !routes.rows().is_empty() {
        ctx.set_routes(std::sync::Arc::new(runtime_routes(routes)));
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
        .to_string(),
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
    let root = sched.spawn(ctx, nvs_runtime::TaskRoot::Request, {
        let status = std::rc::Rc::clone(&status);
        let workers = workers.clone();
        move |ctx| {
            // The returned value is discarded: the script frame answers with
            // null.
            let outcome = entry.call(ctx).map(|_| ());
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
    let compiler = script::Compiler::new(&for_compiler.config);
    // `rule:testing/in-process-request`'s seam nests inside the resolver's for the same length and
    // on the same terms — `runner::UnderTest` owns why the program under test
    // is this crate's to hold.
    let ran = nvs_runtime::script::scoped(&compiler, || {
        nvs_runtime::inproc::scoped(&under_test, || nvs_host::run_until_idle(&mut sched))
    });
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
fn run_test(
    paths: &[PathBuf],
    filter: Option<String>,
    php: PathBuf,
    jobs: Option<std::num::NonZeroUsize>,
    format: runner::Format,
    update: bool,
    config: &[PathBuf],
) -> ExitCode {
    // `rule:testing/nvst-is-separate`'s "`nvs test` runs both", decided by the path rather than
    // by a flag: a program is a `.nvs`/`.php` file and a conformance case is
    // not, so nothing has to be spelled out at the call site.
    if paths.iter().any(|path| is_program(path)) {
        let [path] = paths else {
            eprintln!("error: a program's `#[Test]` methods and `.nvst` cases are run separately");
            return ExitCode::FAILURE;
        };
        // `rule:config/the-config-is-an-immutable-snapshot`'s snapshot, resolved here for the reason `run_run`
        // resolves it above its own compile: `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact key is half
        // configuration — § 7's `[opcache]` says where artifacts live and
        // whether they are read at all, and § 4's environment digest covers the
        // loaded extension set — so a suite compiled above the tree would
        // address an artifact by an environment this run is not in. A tree that
        // does not resolve stops a test run exactly as it stops a `nvs run`.
        let mut config_sources = SourceMap::new();
        let snapshot = match config::boot_snapshot(config, path, &mut config_sources) {
            Ok(snapshot) => snapshot,
            Err(diagnostic) => {
                let mut diags = Diagnostics::new();
                diags.report(diagnostic);
                render_diagnostics(&mut diags, &config_sources);
                return ExitCode::FAILURE;
            }
        };
        // `--filter` reaches both suites, and means the same thing in each:
        // `runner::selected` owns the rule and why it is the `.nvst` tree's.
        return match front_end(path) {
            Ok(checked) => runner::run(checked, &snapshot, format, filter, update),
            Err(code) => code,
        };
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
