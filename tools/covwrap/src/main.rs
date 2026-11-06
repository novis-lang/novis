//! The `RUSTC_WORKSPACE_WRAPPER` of the `covws` build, which `tools/nv/lib/covws.ts` drives.
//!
//! Cargo runs a workspace wrapper as `<wrapper> <rustc> <args...>` for the workspace's own crates
//! and never for a dependency, so dependencies build exactly as they do without coverage and write
//! no counters. This adds `-C instrument-coverage` and `--cfg nvs_coverage` only when the invocation
//! compiles for an explicit target (`--target` is on the command line). `covws.ts` always passes `--target <host triple>`,
//! and cargo then leaves `--target` off the build scripts and proc-macros it compiles for the host,
//! so those stay uninstrumented too. Everything else is passed to `rustc` unchanged, and `rustc`'s
//! exit status is this program's.

use std::ffi::OsString;
use std::process::{Command, exit};

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(rustc) = args.next() else {
        eprintln!("covwrap: cargo runs this as `covwrap <rustc> <args...>`");
        exit(2);
    };
    let rest: Vec<OsString> = args.collect();
    let status = Command::new(&rustc).args(with_coverage(rest)).status();
    match status {
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!(
                "covwrap: could not run {}: {error}",
                rustc.to_string_lossy()
            );
            exit(1);
        }
    }
}

/// `rustc`'s arguments with `-C instrument-coverage` and `--cfg nvs_coverage` added when they
/// compile for an explicit target. `nvs_runtime::capability::keep_coverage_counters` reads the cfg,
/// so a child process of this build keeps `LLVM_PROFILE_FILE` and writes no `default_*.profraw`
/// into the folder it runs in.
fn with_coverage(mut args: Vec<OsString>) -> Vec<OsString> {
    if args
        .iter()
        .any(|a| a == "--target" || a.to_string_lossy().starts_with("--target="))
    {
        args.push("-C".into());
        args.push("instrument-coverage".into());
        args.push("--cfg".into());
        args.push("nvs_coverage".into());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_crate_compiled_for_the_target_is_instrumented() {
        let got = with_coverage(args(&[
            "--crate-name",
            "nvs_cli",
            "--target",
            "x86_64-pc-windows-msvc",
        ]));
        let added = args(&["-C", "instrument-coverage", "--cfg", "nvs_coverage"]);
        assert_eq!(got[got.len() - 4..], added);
        let joined = with_coverage(args(&["--target=x86_64-unknown-linux-gnu", "src/lib.rs"]));
        assert_eq!(joined[joined.len() - 4..], added);
    }

    #[test]
    fn a_build_script_or_proc_macro_compiled_for_the_host_is_left_alone() {
        let host = args(&["--crate-name", "build_script_build", "build.rs"]);
        assert_eq!(with_coverage(host.clone()), host);
    }
}
