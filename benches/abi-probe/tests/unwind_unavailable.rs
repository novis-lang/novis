//! Guards the *premise* of `docs/decisions/0002.md`.
//!
//! Novis propagates errors by checked return rather than by unwinding, because
//! `cranelift-jit` does not register JIT frames with the platform unwinder — so
//! a Rust panic raised beneath JIT code cannot be caught above it and instead
//! terminates the process.
//!
//! That is a property of a dependency, not of our code, and it could change. If
//! it does, ADR 0002 says to reconsider, and this test is how we find out. It
//! fails loudly with an explanation rather than silently continuing to pay for a
//! workaround that is no longer needed.
//!
//! # How it works
//!
//! The dangerous case cannot be exercised in-process: when unwinding fails the
//! process dies, taking the test runner with it. So the test re-executes its own
//! binary as a child, and inspects how the child died.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "stdout is this test's channel, not its logging: the child reports through the \
              `STARTED` and `CAUGHT` markers below and the parent reads them back off its \
              `Output`, so a print here is the mechanism being tested rather than a stray \
              debug line the lint exists to catch"
)]

use std::panic::{self, AssertUnwindSafe};

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

/// Set by the parent to tell the child to run the dangerous probe.
const CHILD_ENV: &str = "NVS_PROBE_UNWIND_CHILD";
/// Printed by the child before anything risky, so the parent can tell a real
/// result from a harness misfire.
const STARTED: &str = "PROBE_CHILD_RUNNING";
/// Printed by the child only if a panic successfully unwound through JIT frames.
const CAUGHT: &str = "PROBE_UNWIND_WAS_CAUGHT";

const TEST_NAME: &str = "native_unwinding_through_jit_frames_is_still_unavailable";

/// A helper that is *allowed* to unwind, which is exactly what Novis's real
/// helpers must never be. Declared `extern "C-unwind"` so the panic is permitted
/// to escape rather than aborting at the boundary.
extern "C-unwind" fn helper_that_panics(x: i64) -> i64 {
    if x == 42 {
        panic!("probe: deliberate panic beneath a JIT frame");
    }
    x * 2
}

#[test]
fn native_unwinding_through_jit_frames_is_still_unavailable() {
    if std::env::var_os(CHILD_ENV).is_some() {
        run_child();
        return;
    }

    // The child is this same binary, and its half of the test opens no file in the tree.
    let exe = std::env::current_exe().expect("cannot locate the test binary");
    let output = nvs_repo::spawn(&exe, &[])
        .args([TEST_NAME, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("failed to spawn the probe child");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains(STARTED),
        "harness problem, not a result: the child never reached the probe.\n\
         Did the test name change? Expected to run `{TEST_NAME}`.\n\
         --- child stdout ---\n{stdout}\n--- child stderr ---\n{stderr}"
    );

    if output.status.success() {
        assert!(
            !stdout.contains(CAUGHT),
            "\n\
             ============================================================\n\
             A panic unwound through a Cranelift JIT frame and was caught.\n\
             \n\
             This did not work when ADR 0002 was written, and the fact that\n\
             it does now means cranelift-jit has started registering JIT\n\
             frames with the platform unwinder.\n\
             \n\
             ADR 0002 names this as the condition for reconsidering the\n\
             checked-return calling convention. Read docs/adr/0002-error-\n\
             propagation.md before acting: the other two reasons given there\n\
             (composing with coroutine stack switches, and keeping error-path\n\
             refcount drops visible to the optimiser) still favour checked\n\
             returns, so this is an invitation to re-evaluate rather than a\n\
             verdict.\n\
             \n\
             Update the ADR either way, so the next reader knows this was\n\
             considered and not merely missed.\n\
             ============================================================"
        );
        panic!(
            "the child exited cleanly without reporting either outcome.\n\
             --- child stdout ---\n{stdout}\n--- child stderr ---\n{stderr}"
        );
    }

    // The expected path: the child died because the unwinder could not walk
    // through the JIT frame. On Windows this surfaces as the MSVC C++ exception
    // code 0xE06D7363; on Unix as a signal.
    let code = output.status.code();
    println!(
        "premise holds: the child was terminated (exit {code:?}) rather than \
         catching the unwind, so ADR 0002's calling convention remains necessary"
    );
}

/// The child half: build a JIT frame, panic beneath it, and report whether the
/// panic could be caught above it.
fn run_child() {
    // Keep the log readable: a passing run intentionally provokes a panic, and
    // the default hook's output looks like a failure to anyone reading CI.
    panic::set_hook(Box::new(|info| {
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .unwrap_or("panic");
        // Deliberately stderr, so it cannot be confused with the markers.
        eprintln!("probe child: expected panic occurred ({msg})");
    }));

    println!("{STARTED}");

    let mut flags = settings::builder();
    flags.set("use_colocated_libcalls", "false").unwrap();
    flags.set("is_pic", "false").unwrap();
    // Ask for unwind tables explicitly, so this is a fair test of the premise
    // rather than a self-fulfilling one.
    flags.set("unwind_info", "true").unwrap();

    let isa = cranelift_native::builder()
        .expect("host is not a supported cranelift target")
        .finish(settings::Flags::new(flags))
        .expect("failed to construct the host ISA");

    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    builder.symbol("helper_that_panics", helper_that_panics as *const u8);
    let mut module = JITModule::new(builder);

    let mut sig = module.make_signature();
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));

    let helper = module
        .declare_function("helper_that_panics", Linkage::Import, &sig)
        .unwrap();
    let wrapper = module
        .declare_function("jit_wrapper", Linkage::Export, &sig)
        .unwrap();

    let mut ctx = module.make_context();
    let mut fn_ctx = FunctionBuilderContext::new();
    ctx.func.signature = sig;
    {
        let target_config = module.target_config();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);
        let helper_ref = module.declare_func_in_func(helper, b.func);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        b.seal_block(entry);
        let x = b.block_params(entry)[0];
        let call = b.ins().call(helper_ref, &[x]);
        let ret = b.inst_results(call)[0];
        b.ins().return_(&[ret]);
        b.finalize(target_config);
    }
    module.define_function(wrapper, &mut ctx).unwrap();
    module.clear_context(&mut ctx);
    module.finalize_definitions().unwrap();

    #[allow(
        unsafe_code,
        reason = "the function was just compiled with exactly this signature"
    )]
    // SAFETY: `jit_wrapper` was declared as `(i64) -> i64` above and the module
    // has been finalized, so the code pointer is valid and executable.
    let jit: extern "C-unwind" fn(i64) -> i64 =
        unsafe { std::mem::transmute(module.get_finalized_function(wrapper)) };

    // Confirm the compiled frame works at all before provoking it.
    assert_eq!(jit(5), 10, "the JIT frame itself must be sound");

    // If this returns Err, the unwinder walked through the JIT frame.
    // If unwind info is unregistered, this call terminates the process instead.
    if panic::catch_unwind(AssertUnwindSafe(|| jit(42))).is_err() {
        println!("{CAUGHT}");
    }
}
