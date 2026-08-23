//! The `mwl` binary.
//!
//! Three subcommands so far, one per milestone that needed one:
//!
//! * `mwl ast` (M1) — dump what the parser produced.
//! * `mwl check` (M2) — parse, resolve, type-check, report every diagnostic.
//! * `mwl run` (M3) — all of the above, then compile and execute.
//!
//! `run` **checks first**: on any diagnostic it reports and exits non-zero
//! exactly as `check` does, rather than running a program the front end
//! rejected. `test`, `serve`, `fmt` and the rest of the architecture diagram
//! (`docs/implementation-plan.md` § Architecture) arrive with the milestones
//! that need them.
//!
//! ## What `run` executes
//!
//! One synthesized frame: the file's own top-level statements, lowered by
//! `mwl_ir::lower::lower_script` — [ADR 0008](../../../docs/adr/0008-static-and-global.md)
//! § 2's "the script body is a function, so its variables are locals". A
//! class's methods are *not* compiled alongside it yet, and nothing is lost by
//! that today: `mwl-codegen` does not lower a call, so no method is reachable.
//! Both halves land together.

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
use mwl_syntax::parse_file;

#[derive(ClapParser)]
#[command(name = "mwl", version, about = "The MWL compiler and CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
    },
    /// Check a `.mwl`/`.php` file, then compile and run it.
    Run {
        /// The file to run.
        file: PathBuf,
        /// Print the lowered IR instead of compiling it.
        #[arg(long)]
        dump_ir: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Ast { file } => run_ast(&file),
        Command::Check { file } => run_check(&file),
        Command::Run { file, dump_ir } => run_run(&file, dump_ir),
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

    render_diagnostics(&mut diags, &map);
    println!("{stmts:#?}");

    if diags.has_errors() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// A file that has been through the whole front end with no error.
///
/// `run` needs everything `check` produces plus the two tables `mwl-ir`
/// lowering reads back — the resolved-target table and the type interner that
/// backs it — so the pipeline is shared rather than written twice.
struct Checked {
    map: SourceMap,
    id: mwl_diagnostics::SourceId,
    stmts: Vec<mwl_syntax::ast::Stmt>,
    interner: mwl_types::TypeInterner,
    exprs: mwl_types::ExprTypeTable,
}

/// Parses, resolves and type-checks `path`, rendering every diagnostic.
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
    let module = mwl_hir::resolve_file(&stmts, map.file(id), &mut diags);
    let mut interner = mwl_types::TypeInterner::new();
    let mut exprs = mwl_types::ExprTypeTable::new();
    mwl_types::check_program(
        &stmts,
        map.file(id),
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );

    render_diagnostics(&mut diags, &map);
    if diags.has_errors() {
        return Err(ExitCode::FAILURE);
    }
    Ok(Checked {
        map,
        id,
        stmts,
        interner,
        exprs,
    })
}

fn run_check(path: &std::path::Path) -> ExitCode {
    match front_end(path) {
        Ok(_) => {
            println!("no errors");
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

fn run_run(path: &std::path::Path, dump_ir: bool) -> ExitCode {
    let checked = match front_end(path) {
        Ok(checked) => checked,
        Err(code) => return code,
    };
    let src = checked.map.file(checked.id);

    let script = mwl_ir::lower::lower_script(
        SCRIPT,
        &checked.stmts,
        src,
        &checked.exprs,
        &checked.interner,
    );
    if dump_ir {
        print!("{}", mwl_ir::print::print_function(&script, src));
        return ExitCode::SUCCESS;
    }

    let program = mwl_ir::Program {
        functions: vec![script],
    };
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
            let kind = if status == mwl_runtime::THROWN {
                "uncaught exception"
            } else {
                "fatal error"
            };
            let message = ctx
                .take_pending()
                .unwrap_or(std::borrow::Cow::Borrowed("no message was recorded"));
            eprintln!("{kind}: {message}");
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
