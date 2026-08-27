//! Backend settings that are a safety policy rather than a tuning choice.
//!
//! `Jit::new` and its `settings::builder()` are private, and a flag leaves no
//! trace in the compiled unit that a test could read back, so the policy is
//! pinned at its source the way `[profile.release]`'s `overflow-checks` is
//! pinned at the manifest. Crude, and the only thing that actually fails when
//! the line is deleted.
//!
//! One of them is an exception and gets a real test below: a probe emitted the
//! *outline* way is a call to a symbol this JIT does not have, so the strategy
//! is observable — as a panic, on the first frame large enough to need one.

mod common;

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
        "nvs-codegen no longer enables Cranelift's stack probes. Cranelift's own default is \
         off, so deleting the flag silently reopens the stack-clash window on any frame over \
         4 KiB. If this is deliberate, the reasoning belongs beside the flag in `Jit::new`."
    );
}

#[test]
fn a_frame_over_the_probe_threshold_compiles_and_runs() {
    // The probe above only protects a frame if it can be emitted. Cranelift's
    // default strategy is `outline`, which is a call to `__cranelift_probestack`
    // — a symbol with its own register convention that a JIT has to supply, and
    // `Jit::new`'s `builder.symbol` loop supplies the runtime's helpers and
    // nothing else. So the default is not "protection with a call": it is
    // `cranelift-jit` panicking `can't resolve libcall __cranelift_probestack`
    // the first time a frame crosses `probestack_size_log2`'s 4 KiB, which is
    // roughly fifty statements at a script's file scope and is why this is a
    // run rather than another source grep.
    let mut source = String::from(
        "<?nvs
class Pad {
    public static function tag(int $n): string {
        return \"x\";
    }
}
",
    );
    for n in 0..200 {
        source.push_str(&format!("echo Pad::tag({n});\n"));
    }

    assert_eq!(common::output_of(&source), "x".repeat(200));
}
