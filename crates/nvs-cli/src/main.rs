//! The `nvs` binary.
//!
//! Seven subcommands so far, one per milestone that needed one:
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
//! * `nvs api diff <old.json> <new.json>` — the same ADR's § 4 gate over two
//!   finished documents: breaking, additive or cosmetic per change, non-zero
//!   exit on a breaking one. It compiles nothing, because the old side of a
//!   diff is a released artifact rather than a program; see [`api_diff`].
//! * `nvs info` — build, host and third-party licensing facts, PHP's
//!   `php -i` in shape and in purpose. Also spelled `nvs -i`, since that is
//!   the spelling anyone arriving from PHP will try first; see [`info`].
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
mod info;
mod openapi;
mod runner;

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
        /// Run only cases whose path contains this text.
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
    /// One artifact so far, and `--openapi` is required rather than defaulted:
    /// `nvs build` with nothing named would be a subcommand that succeeds
    /// having done nothing, and the flag is how the next artifact joins without
    /// changing what this invocation means
    /// ([ADR 0085](../../../docs/adr/0085-openapi-is-generated-from-the-route-table.md)
    /// § 3 spells the whole command).
    Build {
        /// The entry point of the program to build.
        file: PathBuf,
        /// Write ADR 0085's OpenAPI 3.1 document to standard output.
        #[arg(long, required = true)]
        openapi: bool,
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
        } => run_run(&file, dump_ir, dump_asm, fault_inject),
        Command::Test {
            paths,
            filter,
            php,
            format,
        } => run_test(&paths, filter, php, format),
        Command::Build { file, openapi } => run_build(&file, openapi),
        Command::Api {
            command: ApiCommand::Diff { old, new },
        } => api_diff::run(&old, &new),
        Command::Info { licenses } => info::run(licenses),
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
    debug_assert!(openapi, "`--openapi` is `required` at the flag");
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

/// The label the script frame is compiled and looked up under.
///
/// `nvs_ir::lower::lower_script` leaves the name to its caller; this is the
/// same spelling `nvs-ir`'s own snapshots use.
const SCRIPT: &str = "<script>";

/// ADR 0102 § 6's configured origin, read out of `./nvs.toml`'s `[app] origin`
/// — and that key alone.
///
/// **This is not `nvs.toml`'s reader**, and must not grow into one. M6's is,
/// with ADR 0064's syntax, ADR 0103's include tree and ownership check, and
/// ADR 0005's directive registry behind it; `nvs_syntax`'s module docs name it
/// as the caller that has not arrived. What is here reads one key, because ADR
/// 0102 § 6 makes the origin *configured* and refuses every other source
/// outright: with nowhere to configure it, `Core\Router::urlAbsolute` has no
/// answer it is allowed to give at all. A second key added here is a second
/// configuration format, so the next one goes in M6's reader instead.
///
/// The location is ADR 0103 § 1 step 2 — `./nvs.toml` in the working
/// directory, **exactly one directory and never a walk upward**, and never
/// beside the entry file. Step 1's `--config` is M6's closed flag list and
/// step 3's shipped defaults name no origin, so there is one place to look.
/// A file that is absent, unreadable or holds no `[app] origin` resolves
/// `None`, which is not an error here: ADR 0097 § 3's "a unit that resolves
/// none is an error" is reported at the link site, where the message can name
/// the route it could not build.
fn configured_origin() -> Option<String> {
    let text = std::fs::read_to_string("nvs.toml").ok()?;
    let mut in_app = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if let Some(table) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            in_app = table.trim() == "app";
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !in_app || key.trim() != "origin" {
            continue;
        }
        if let Some(quoted) = value
            .trim()
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
        {
            return Some(quoted.to_owned());
        }
    }
    None
}

fn run_run(
    path: &std::path::Path,
    dump_ir: bool,
    dump_asm: bool,
    fault_inject: Option<FaultSiteArg>,
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

    // The script's own frame is the request, for a CLI run: one `Ctx` writing
    // to the process's standard output.
    let mut ctx = nvs_runtime::Ctx::stdout();
    // ADR 0102 § 6's origin is resolved before the program runs and never
    // during it, which is the whole of what makes it un-sniffable.
    if let Some(origin) = configured_origin() {
        ctx.set_origin(&origin);
    }
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
        move |ctx| {
            // The returned value is discarded exactly as it was when this was a
            // direct call: the script frame answers with null.
            status.set(Some(nvs_runtime::call(entry, ctx, &[]).map(|_| ())));
        }
    });

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
    let ran = nvs_host::run_until_idle(&mut sched);
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
        Err(status) => {
            // ADR 0020's ladder is not built yet; until it is, the honest
            // report is the status and whatever message the runtime recorded.
            // The two spellings are the ladder's own tier names: an uncaught
            // throw is tier 2, a `FATAL` is tier 3 and above, and nothing
            // below the engine floor can catch either.
            let thrown = ctx.take_thrown();
            let message = thrown.message();
            if status == nvs_runtime::THROWN {
                eprintln!("Uncaught Exception: {message}");
                // The frames the exception unwound out of, `#0` first —
                // `nvs_runtime::throwable`'s own docs own the shape and why it
                // is built on the error path rather than at construction.
                // Empty for a `FATAL`, which has no backtrace by design
                // (ADR 0020), so nothing is printed for one.
                let trace = thrown.trace_as_string();
                if !trace.is_empty() {
                    eprintln!("{trace}");
                }
            } else {
                eprintln!("FATAL: {message}");
            }
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
        return match front_end(path) {
            Ok(checked) => runner::run(&checked, format),
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
