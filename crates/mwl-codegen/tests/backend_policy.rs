//! Backend settings that are a safety policy rather than a tuning choice.
//!
//! `Jit::new` and its `settings::builder()` are private, and a flag leaves no
//! trace in the compiled unit that a test could read back, so the policy is
//! pinned at its source the way `[profile.release]`'s `overflow-checks` is
//! pinned at the manifest. Crude, and the only thing that actually fails when
//! the line is deleted.

use std::fs;
use std::path::Path;

/// The backend source, read once.
fn backend() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[test]
fn stack_probes_are_enabled() {
    // Cranelift defaults `enable_probestack` to false. Off means a frame
    // larger than the guard page can step over it in one move and write past
    // it — a stack clash, and a memory-safety bug rather than the clean crash
    // the guard page exists to produce. `Jit::new`'s own comment carries the
    // measurement that says it costs nothing, and why it is not ADR 0020 § 1's
    // call-stack limit wearing a different name.
    assert!(
        backend().contains(r#"("enable_probestack", "true")"#),
        "mwl-codegen no longer enables Cranelift's stack probes. Cranelift's own default is \
         off, so deleting the flag silently reopens the stack-clash window on any frame over \
         4 KiB. If this is deliberate, the reasoning belongs beside the flag in `Jit::new`."
    );
}
