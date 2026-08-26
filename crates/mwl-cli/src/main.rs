//! The `mwl` binary.
//!
//! Five subcommands so far, one per milestone that needed one:
//!
//! * `mwl ast` (M1) — dump what the parser produced.
//! * `mwl check` (M2) — parse, resolve, type-check, report every diagnostic.
//!   `--autoload-map` prints the resolved `autoload` map in place of the
//!   success line, which is
//!   [ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
//!   § 1's last sentence; the shape is `mwl_hir::autoload`'s module doc.
//! * `mwl run` (M3) — all of the above, then compile and execute. Its two
//!   dump flags stop one stage earlier and print instead of running:
//!   `--dump-ir` after lowering, `--dump-asm` after code generation.
//! * `mwl test` (M4) — run a tree of `.mwlt` conformance cases. The format,
//!   and every decision behind it, is [`mwl_test`]'s own module doc; this
//!   crate contributes only the argument parsing and the exit code.
//! * `mwl info` — build, host and third-party licensing facts, PHP's
//!   `php -i` in shape and in purpose. Also spelled `mwl -i`, since that is
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
//! The whole file, through `mwl_ir::lower::lower_file`: every class method
//! with a body, plus one synthesized frame for the file's own top-level
//! statements — [ADR 0008](../../../docs/adr/0008-static-and-global.md) § 2's
//! "the script body is a function, so its variables are locals". That frame is
//! the entry point; the methods are reachable from it by name.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this crate's whole job is user-facing terminal output"
)]

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser as ClapParser, Subcommand};
use mwl_diagnostics::{Diagnostics, Renderer, SourceMap};
use mwl_syntax::{check_declarations, parse_file};

mod info;

#[derive(ClapParser)]
#[command(
    name = "mwl",
    version,
    about = "The MWL compiler and CLI",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Print build, host and third-party licensing information.
    ///
    /// The same report as `mwl info`, under the spelling PHP uses.
    #[arg(short = 'i', long)]
    info: bool,

    /// With `-i`: include every third-party license text in full.
    #[arg(long, requires = "info")]
    licenses: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a `.mwl`/`.php` file and print its AST.
    Ast {
        /// The file to parse.
        file: PathBuf,
    },
    /// Parse, resolve and type-check a `.mwl`/`.php` file, reporting every
    /// diagnostic found.
    Check {
        /// The file to check.
        file: PathBuf,
        /// Print the resolved `autoload` map instead of `no errors`,
        /// including what a `discover` glob skipped and what was shadowed.
        #[arg(long)]
        autoload_map: bool,
    },
    /// Check a `.mwl`/`.php` file, then compile and run it.
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
        /// Deliberately scoped to `mwl run` and nothing else: a contained
        /// engine panic has no user-facing trigger by definition, so it needs
        /// a hook to be testable at all — and that hook must never be
        /// reachable from a served request. `mwl serve` (M7) does not get one.
        /// `mwl_runtime::FaultSite` documents each site.
        #[arg(long, value_name = "SITE")]
        fault_inject: Option<FaultSiteArg>,
    },
    /// Run `.mwlt` conformance cases.
    ///
    /// Each path is either one case file or a directory walked for `*.mwlt`.
    /// Exits non-zero if any case failed; a skipped case is not a failure.
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
    },
    /// Print build, host and third-party licensing information.
    ///
    /// One call answers what this binary is and what is compiled into it,
    /// including the complete third-party attribution MWL's MIT license and
    /// its dependencies' licenses both require to be distributed with it.
    /// See `docs/adr/0065-third-party-attribution-and-mwl-info.md`.
    Info {
        /// Also print every third-party license text in full.
        #[arg(long)]
        licenses: bool,
    },
}

/// The closed set of sites `--fault-inject` accepts, one per
/// [`mwl_runtime::FaultSite`].
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum FaultSiteArg {
    /// The request's second runtime helper call panics.
    HelperPanic,
}

impl From<FaultSiteArg> for mwl_runtime::FaultSite {
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

    // Unreachable: `arg_required_else_help` makes a bare `mwl` print help.
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
        Command::Test { paths, filter, php } => run_test(&paths, filter, php),
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
/// `run` needs everything `check` produces plus the three tables `mwl-ir`
/// lowering reads back — the resolved-target table, the type interner that
/// backs it, and the class-layout table — so the pipeline is shared rather
/// than written twice.
struct Checked {
    map: SourceMap,
    /// The entry point's own file. It names the program (`mwl run <path>`),
    /// so it stays a single id even though `files` is now a set.
    id: mwl_diagnostics::SourceId,
    /// Every file the entry point's `require`/`autoload` graph reached, the
    /// entry file first — `mwl_hir::resolve_program`'s order contract.
    files: Vec<mwl_hir::Loaded>,
    interner: mwl_types::TypeInterner,
    exprs: mwl_types::ExprTypeTable,
    /// Every declared enum's backing type and its cases' values, handed back
    /// by `check_program` rather than rebuilt — `mwl_ir::lower` needs a case's
    /// constant for ADR 0047 § 3's membership test.
    enums: mwl_types::EnumTable,
    layouts: mwl_types::ClassLayoutTable,
    /// The autoload map the graph walk consulted, kept for
    /// `check --autoload-map` and read by nothing else here.
    autoload: mwl_hir::AutoloadMap,
}

impl Checked {
    /// The program as `mwl-types` and `mwl-ir` both consume it: one entry per
    /// loaded file, **the entry point first**.
    ///
    /// Position zero is where `mwl_ir::lower::lower_program` takes the script
    /// frame from, which is `mwl_hir::resolve_program`'s documented order
    /// contract rather than a coincidence — the assert is what keeps this
    /// file honest if that ever changes.
    fn program_files(&self) -> Vec<mwl_types::ProgramFile<'_>> {
        debug_assert_eq!(
            self.files[0].id, self.id,
            "resolve_program hands the entry file back first"
        );
        self.files
            .iter()
            .map(|file| mwl_types::ProgramFile {
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
/// file: `mwl_hir::resolve_program` walks it into one `Module` plus the
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
    let (module, loaded, autoload) = mwl_hir::resolve_program(id, stmts, &mut map, &mut diags);

    let mut interner = mwl_types::TypeInterner::new();
    let mut exprs = mwl_types::ExprTypeTable::new();
    let (enums, layouts) = {
        let files: Vec<mwl_types::ProgramFile<'_>> = loaded
            .iter()
            .map(|file| mwl_types::ProgramFile {
                src: map.file(file.id),
                stmts: &file.stmts,
            })
            .collect();
        let enums =
            mwl_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);

        if diags.has_errors() {
            render_diagnostics(&mut diags, &map);
            return Err(ExitCode::FAILURE);
        }
        (enums, mwl_types::build_class_layouts(&files, &module.graph))
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

/// `mwl check`, and with `--autoload-map` also ADR 0061 § 1's last sentence:
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

/// The label the script frame is compiled and looked up under.
///
/// `mwl_ir::lower::lower_script` leaves the name to its caller; this is the
/// same spelling `mwl-ir`'s own snapshots use.
const SCRIPT: &str = "<script>";

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

    let program = mwl_ir::lower::lower_program(
        SCRIPT,
        &checked.program_files(),
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    if dump_ir {
        for function in &program.functions {
            print!("{}", mwl_ir::print::print_function(function, src));
        }
        return ExitCode::SUCCESS;
    }

    if dump_asm {
        // Deliberately the same compile `run` performs, disassembled rather
        // than a second differently-configured one — see
        // `mwl_codegen::disassemble`. Like `--dump-ir`, it prints instead of
        // running.
        return match mwl_codegen::disassemble(&program) {
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
    let unit = match mwl_codegen::compile(&program) {
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
    let mut ctx = mwl_runtime::Ctx::stdout();
    // Hands the context the unit's class table: the class a helper's
    // bare-message failure is promoted to, and the shared ownership that lets
    // the context outlive the unit. `Unit::install_in` owns both reasons.
    unit.install_in(&mut ctx);
    if let Some(site) = fault_inject {
        ctx.inject_fault(site.into());
    }
    let outcome = mwl_runtime::call(entry, &mut ctx, &[]);
    // Flushed before anything is reported: Rust's standard output is
    // line-buffered, and `echo "Hello, World!"` has no trailing newline.
    if let Err(error) = ctx.flush_output() {
        eprintln!("error: could not flush output: {error}");
        return ExitCode::FAILURE;
    }

    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(status) => {
            // ADR 0020's ladder is not built yet; until it is, the honest
            // report is the status and whatever message the runtime recorded.
            // The two spellings are the ladder's own tier names: an uncaught
            // throw is tier 2, a `FATAL` is tier 3 and above, and nothing
            // below the engine floor can catch either.
            let thrown = ctx.take_thrown();
            let message = thrown.message();
            if status == mwl_runtime::THROWN {
                eprintln!("Uncaught Exception: {message}");
                // The frames the exception unwound out of, `#0` first —
                // `mwl_runtime::throwable`'s own docs own the shape and why it
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

/// Runs a tree of `.mwlt` cases and turns the summary into an exit code.
///
/// Each case is run by spawning **this** binary — `mwl_test::run` documents
/// why a subprocess rather than an in-process compile — so a debug build
/// tests itself and a release build tests itself, with nothing to configure.
fn run_test(paths: &[PathBuf], filter: Option<String>, php: PathBuf) -> ExitCode {
    let mut options = match mwl_test::Options::from_current_exe() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: could not locate this binary to run cases with: {error}");
            return ExitCode::FAILURE;
        }
    };
    options.filter = filter;
    options.php = php;

    let mut out = std::io::stdout().lock();
    match mwl_test::run(paths, &options, &mut out) {
        Ok(summary) if summary.is_success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
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
