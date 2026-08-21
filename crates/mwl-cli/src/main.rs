//! The `mwl` binary.
//!
//! M1 only needs one subcommand: `mwl ast`, the plan's verification tool for
//! dumping what the parser produced. `run`/`test` and everything else in the
//! architecture diagram (`docs/implementation-plan.md` § Architecture) arrive
//! with the milestones that need them.

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Ast { file } => run_ast(&file),
        Command::Check { file } => run_check(&file),
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

fn run_check(path: &std::path::Path) -> ExitCode {
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
    let module = mwl_hir::resolve_file(&stmts, map.file(id), &mut diags);
    let mut interner = mwl_types::TypeInterner::new();
    mwl_types::check_program(&stmts, map.file(id), &module, &mut interner, &mut diags);

    render_diagnostics(&mut diags, &map);

    if diags.has_errors() {
        ExitCode::FAILURE
    } else {
        println!("no errors");
        ExitCode::SUCCESS
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
