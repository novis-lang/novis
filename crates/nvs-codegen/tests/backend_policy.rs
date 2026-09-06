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

/// The lowering walk's source, read once.
fn emitter() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/emit.rs");
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[test]
fn a_call_to_the_units_own_function_is_not_colocated() {
    // `cranelift-module` sets `colocated` from `linkage.is_final()`, true for
    // a function this unit defines, and a colocated callee is a 32-bit
    // PC-relative call. `cranelift-jit` asks the OS for each code allocation
    // separately, so two can land more than 2 GiB apart, and applying that
    // relocation then panics inside cranelift with `PosOverflow` — before any
    // diagnostic of ours. The ASan leg reproduces it because the sanitizer's
    // reservations spread the allocations out; an ordinary run usually does
    // not, which is exactly why deleting the line needs to fail here too and
    // not only on the one leg that happens to notice.
    assert!(
        emitter().contains("ext_funcs[reference].colocated = false"),
        "nvs-codegen no longer clears `colocated` on a call to one of the unit's own \
         functions, so such a call is a 32-bit PC-relative one again and a code allocation \
         further than 2 GiB from its caller panics cranelift-jit rather than running. If \
         this is deliberate, the reasoning belongs beside the line in `callee_ref`."
    );
}

#[test]
fn a_reference_to_the_units_own_data_is_not_colocated() {
    // The same 2 GiB range limit as the test above, on the other kind of
    // symbol. A string literal is `Linkage::Local` data, so `is_final()` is
    // true and `symbol_value` lowers to a PC-relative address; `cranelift-jit`
    // allocates the data blob separately from the code that reads it, and the
    // ASan leg spreads them far enough apart to overflow the relocation. A
    // class descriptor is an `Import` and arrives non-colocated already, so
    // only the literal path is exposed — and both go through the helper all
    // the same.
    //
    // Every site is checked, not just one, because a grep for the helper's
    // name passes as soon as *any* caller keeps it, which would let the
    // literal path regress in silence.
    let source = emitter();
    let sites: Vec<_> = source
        .lines()
        .zip(source.lines().skip(1))
        .filter(|(line, _)| line.contains("declare_data_in_func("))
        .collect();
    assert!(
        !sites.is_empty(),
        "backend_policy can no longer find the data-import sites it guards; `emit.rs` names \
         them differently now, and this test is checking nothing."
    );
    for (site, next) in sites {
        assert!(
            next.contains("self.clear_colocated(global)"),
            "nvs-codegen no longer clears `colocated` on a reference to one of the unit's own \
             data symbols: `{}` is not followed by the call. A string literal is \
             `Linkage::Local`, so it would be addressed PC-relative again, and a data \
             allocation further than 2 GiB from the code reading it panics cranelift-jit \
             rather than running. If this is deliberate, the reasoning belongs beside \
             `clear_colocated`.",
            site.trim()
        );
    }
}

#[test]
fn stack_probes_are_enabled() {
    // Cranelift defaults `enable_probestack` to false. Off means a frame
    // larger than the guard page can step over it in one move and write past
    // it — a stack clash, and a memory-safety bug rather than the clean crash
    // the guard page exists to produce. `Jit::new`'s own comment carries the
    // measurement that says it costs nothing, and why it is not `rule:errors/on-limit`'s
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
    // the first time a frame crosses `probestack_size_log2`'s 4 KiB, which a
    // script's file scope reaches in a modest run of statements — and is why
    // this is a run rather than another source grep.
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
