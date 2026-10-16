//! The build script every workspace crate with integration tests runs, as its whole `build =` or
//! called from its own `build.rs`: on windows-msvc, each `tests/*.rs` program is linked without a
//! PDB.
//!
//! The linker writes the debug info of every crate a program links into that program's PDB, and with
//! one program per test file those PDBs were most of a debug build, in `target/debug` and again in
//! `target/covws`, for files nothing reads. A failing test still names its file and line, because the
//! panic location is compiled into the program, and coverage still works, because its counters and
//! function names are sections of the program itself. What goes is a symbolized Rust backtrace from
//! inside an integration test. `NVS_TEST_PDB=1` brings the PDBs back; setting or clearing it reruns
//! this script, which rebuilds the crate and everything downstream of it.
//!
//! `nvs.exe`, every other binary and each crate's unit-test program keep their PDBs: cargo has no
//! link argument for unit-test programs alone, and a binary's backtrace is read in the field.

#![allow(
    clippy::print_stdout,
    reason = "`cargo:` directives on stdout are how a build script talks to Cargo"
)]

/// Prints the link argument for this crate's integration tests, and the one variable that changes it.
pub(crate) fn main() {
    // Any `rerun-if` line replaces cargo's default of rerunning on every change in the package; an edit
    // to this file reruns it anyway, because the script is rebuilt.
    println!("cargo:rerun-if-env-changed=NVS_TEST_PDB");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if msvc && std::env::var_os("NVS_TEST_PDB").is_none() {
        println!("cargo:rustc-link-arg-tests=/DEBUG:NONE");
    }
}
