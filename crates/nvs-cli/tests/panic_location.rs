//! A panic in a binary this workspace links still names its file and line.
//!
//! On Windows the workspace links with rust-lld (`.cargo/config.toml`), so
//! this pins what that linker must keep: the panic location and the unwind
//! that ends the panicking thread with a failed test. A symbolized backtrace
//! is not checked here, because an integration test is linked without a PDB
//! (`tools/build/tests_without_pdb.rs`).
//!
//! `nvs` has no way to panic on purpose, and adding one to the shipped binary
//! would be a crash any user could reach. So the case starts its own test
//! binary again as a child, which this crate's tests share their linker and
//! profile with, and reads the child's stderr.

/// Set in the child's environment, so only the child panics.
const CHILD: &str = "NOVIS_PANIC_LOCATION_CHILD";

/// The line of the `panic!` below, which the parent looks for in stderr.
const PANIC_LINE: u32 = line!() + 9;

/// The child's half: panics when [`CHILD`] is set, and does nothing in a
/// normal run.
#[test]
fn child_panics_when_asked() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    panic!("the child panics on purpose");
}

/// The child fails, and its stderr names `tests/panic_location.rs`, the
/// `panic!`'s line and its message.
#[test]
fn a_panic_names_its_file_and_line() {
    let exe = std::env::current_exe().expect("the test binary knows its own path");
    // The child is this test binary, and it opens nothing in the tree.
    let out = nvs_repo::spawn(exe, &[])
        .args([
            "--exact",
            "child_panics_when_asked",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("the test binary starts again as a child");
    // Windows prints the path with `\`, and the check reads both alike.
    let stderr = String::from_utf8_lossy(&out.stderr).replace('\\', "/");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success(),
        "the child panicked: {stdout}\n{stderr}"
    );

    let location = format!("tests/panic_location.rs:{PANIC_LINE}:");
    assert!(
        stderr.contains(&location),
        "the panic names `{location}`: {stderr}"
    );
    assert!(
        stderr.contains("the child panics on purpose"),
        "the panic's message is printed: {stderr}"
    );
}
