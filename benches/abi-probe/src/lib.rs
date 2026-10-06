//! Architecture invariants and cost baselines for Novis's execution model.
//!
//! This crate is not part of the compiler. It exists because decisions in
//! `docs/adr/` rest on how Cranelift, `corosensei` and Wasmtime actually behave
//! on the host platform, and those are properties of our dependencies rather
//! than of our code. A dependency bump can silently invalidate them.
//!
//! The invariants are checked continuously rather than once:
//!
//! | probe | guards |
//! |---|---|
//! | `tests/invariants.rs` | the checked-return ABI and coroutine behaviour of [ADR 0002](/docs/decisions/0002.md) |
//! | `tests/unwind_unavailable.rs` | the *premise* of ADR 0002 — that native unwinding is unavailable |
//! | `tests/wasm_sandbox.rs` | the sandbox guarantees of [ADR 0003](/docs/decisions/0003.md) |
//! | `tests/wasm_async.rs` | the async bridge of [ADR 0246](/docs/decisions/0246.md) § 5: a guest call parks, yields and traps inside a coroutine |
//! | `tests/perf_guards.rs` | order-of-magnitude regressions in the quoted costs |
//! | [`process`] | the cost of the child process that [ADR 0006](/docs/decisions/0006.md) replaces with an in-process isolate |
//! | `shared/isolate.rs` | the cost of the in-process isolate it is replaced *with* — the other half of that comparison, outside this library because it needs the compiler crates and those are dev-only |
//! | `benches/*` | the quoted costs themselves, tracked over time |
//! | `examples/callgrind_spike.rs` | the callgrind-instruction-count premise of [ADR 0026](/docs/decisions/0026.md)'s historical performance dashboard — Linux/WSL only, see that ADR |
//!
//! # The ABI under test
//!
//! Every compiled Novis function and every runtime helper has one shape:
//!
//! ```text
//! extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32
//! //            ^ request   ^ argument    ^ result     ^ 0 = ok, else throw pending
//! ```
//!
//! Errors travel in the return value, never by unwinding. See [`OK`], [`THROWN`]
//! and [`FATAL`].

pub mod process;

#[cfg(feature = "wasm-probe")]
pub mod wasm;

use std::panic::{self, AssertUnwindSafe};

use corosensei::Yielder;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

/// Success. The result has been written through the `out` pointer.
pub const OK: i32 = 0;

/// An Novis-level exception is pending in [`Ctx::pending`]. A `catch` may handle it.
pub const THROWN: i32 = 1;

/// Unrecoverable: a resource limit, or an internal error caught at a helper
/// boundary. Propagates to the request boundary and Novis code cannot catch it.
pub const FATAL: i32 = 2;

/// Mirrors Novis's planned 16-byte tagged value.
///
/// NaN-boxing is deliberately not used: PHP semantics require the full `i64`
/// range, which will not fit alongside a tag in 64 bits.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Value {
    /// Type tag. Only `2` (integer) is meaningful in the probe.
    pub tag: u64,
    /// Payload.
    pub bits: u64,
}

impl Value {
    /// An integer value.
    #[must_use]
    pub const fn int(v: i64) -> Self {
        Self {
            tag: 2,
            bits: v as u64,
        }
    }

    /// Reads the payload as an integer.
    #[must_use]
    pub const fn as_int(self) -> i64 {
        self.bits as i64
    }
}

/// What a suspended task is waiting for.
///
/// The real scheduler yields a registration with the reactor; the probe only
/// needs to prove that suspension works with JIT frames live on the stack.
#[derive(Debug, PartialEq, Eq)]
pub enum Waiting {
    /// Waiting on I/O, tagged so a test can confirm which call suspended.
    Io(u64),
}

/// The coroutine yielder type used throughout the probe.
pub type Yield = Yielder<(), Waiting>;

/// Per-request state, passed to every Novis function and helper.
///
/// Deliberately shaped like the real `Ctx` will be: it carries the coroutine
/// yielder (so any helper can suspend without the language needing `async`), the
/// pending exception, and counters standing in for the request's accounting.
#[repr(C)]
#[derive(Debug)]
pub struct Ctx {
    /// Set while executing inside a coroutine; null on the main stack.
    ///
    /// This is what removes async colouring: a helper reaches the yielder
    /// through the context rather than through its own signature, so no caller
    /// up the chain needs to be marked `async`.
    pub yielder: *const Yield,
    /// Message for a pending [`THROWN`] or [`FATAL`] status.
    ///
    /// `Cow` rather than `String` on purpose. Allocating a message on every
    /// throw costs several times the entire propagation path, and PHP code
    /// throws on ordinary control-flow paths. The real runtime should keep the
    /// same property: a static message must not allocate.
    pub pending: Option<std::borrow::Cow<'static, str>>,
    /// How many times a helper ran. Proves native code actually executed.
    pub helper_calls: u64,
    /// How many times a helper suspended the coroutine.
    pub suspends: u64,
    /// Stands in for the real runtime's debug-flags word
    /// ([ADR 0018](/docs/decisions/0018.md)
    /// § 1) and, by the same shape, its safepoint word: compiled code loads
    /// it and branches on non-zero at every probe site. Left zero throughout,
    /// because the number being guarded is the all-bits-*off* cost.
    pub debug_flags: u64,
    /// Written by each statement-shaped store in a probe chain — see
    /// [`Probe::compile_probe_chain`] for why the chains store anything at
    /// all.
    pub scratch: u64,
    /// How many times a probe site's slow path ran. Zero in the measurement;
    /// non-zero only when a test deliberately sets [`Self::debug_flags`].
    pub probe_hits: u64,
}

/// Byte offset of [`Ctx::debug_flags`] — the word a probe site loads.
pub const DEBUG_FLAGS_OFFSET: i32 = std::mem::offset_of!(Ctx, debug_flags) as i32;

/// Byte offset of [`Ctx::scratch`] — where a statement-shaped store lands.
pub const SCRATCH_OFFSET: i32 = std::mem::offset_of!(Ctx, scratch) as i32;

impl Ctx {
    /// A context with no coroutine attached.
    #[must_use]
    pub fn new() -> Self {
        Self {
            yielder: std::ptr::null(),
            pending: None,
            helper_calls: 0,
            suspends: 0,
            debug_flags: 0,
            scratch: 0,
            probe_hits: 0,
        }
    }

    /// Clears the pending message and counters between measurements.
    pub fn reset(&mut self) {
        self.pending = None;
        self.helper_calls = 0;
        self.suspends = 0;
        self.scratch = 0;
        self.probe_hits = 0;
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled Novis function.
///
/// `unsafe` because the three pointers carry a contract the type cannot express:
/// each must be non-null, aligned, and valid for the duration of the call, and
/// `out` must be writable. Compiled code satisfies this by construction; hand
/// callers should go through [`call`], which is a safe wrapper.
pub type NvsFn = unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32;

// ---------------------------------------------------------------------------
// Runtime helpers
// ---------------------------------------------------------------------------

/// Defines a runtime helper with Novis's helper ABI.
///
/// The details below are load-bearing and easy to get wrong by hand, which is
/// why this is a macro rather than a convention:
///
/// * `extern "C"`, **not** `extern "C-unwind"`. Since Rust 1.81 a panic escaping
///   a plain `extern "C"` function aborts, and an abort in a server that shares
///   one process across all requests kills every in-flight request.
/// * The body is wrapped in `catch_unwind`, converting a panic into [`FATAL`].
///   This is what contains a runtime bug to a single request without needing any
///   unwind tables. It costs nothing when no panic occurs.
macro_rules! probe_helper {
    (
        $(#[$meta:meta])*
        fn $name:ident($ctx:ident, $arg:ident, $out:ident) $body:block
    ) => {
        $(#[$meta])*
        ///
        /// # Safety
        ///
        /// `ctx`, `arg` and `out` must each be non-null, aligned and valid for
        /// the duration of the call, and `out` must be writable. Compiled Novis
        /// code satisfies this by construction.
        #[allow(
            unsafe_code,
            reason = "a runtime helper's signature is fixed by the ABI in \
                      ADR 0002; the pointer contract cannot be expressed in \
                      the type, so the function is honestly marked unsafe"
        )]
        pub unsafe extern "C" fn $name(
            ctx: *mut Ctx,
            arg: *const Value,
            out: *mut Value,
        ) -> i32 {
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
                #[allow(
                    unsafe_code,
                    reason = "codegen guarantees these pointers are valid, \
                              non-null and non-aliasing for the call's duration"
                )]
                // SAFETY: `ctx`, `arg` and `out` come from compiled Novis code,
                // which always passes a live request context, a readable
                // argument slot and a writable result slot.
                let ($ctx, $arg, $out): (&mut Ctx, Value, &mut Value) =
                    unsafe { (&mut *ctx, *arg, &mut *out) };
                $body
            }));

            match outcome {
                Ok(status) => status,
                Err(payload) => {
                    let msg = payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
                        .unwrap_or_else(|| "<non-string panic payload>".to_string());
                    #[allow(
                        unsafe_code,
                        reason = "same contract as above; reached only while \
                                  unwinding out of the Rust closure"
                    )]
                    // SAFETY: as above. Allocating here is fine: this path is
                    // a contained bug, not ordinary control flow.
                    unsafe {
                        (*ctx).pending = Some(format!("internal error: {msg}").into());
                    }
                    FATAL
                }
            }
        }
    };
}

probe_helper! {
    /// Doubles its argument, with deliberate escape hatches used by the
    /// invariant tests:
    ///
    /// * `42` raises an ordinary Novis exception ([`THROWN`]).
    /// * `99` panics, standing in for a bug in the runtime, which must surface
    ///   as [`FATAL`] rather than killing the process.
    fn probe_double(ctx, arg, out) {
        ctx.helper_calls += 1;
        let n = arg.as_int();
        if n == 42 {
            // Borrowed, not allocated — see `Ctx::pending`.
            ctx.pending = Some("NvsError: helper refused 42".into());
            return THROWN;
        }
        if n == 99 {
            panic!("unreachable branch reached with n=99");
        }
        *out = Value::int(n * 2);
        OK
    }
}

probe_helper! {
    /// Suspends the coroutine, then doubles its argument after being resumed.
    ///
    /// The point is that the frames above this helper are JIT-compiled native
    /// code, and they survive the stack switch intact.
    ///
    /// # Panics
    ///
    /// Panics if called outside a coroutine. That panic is caught by the helper
    /// wrapper and reported as [`FATAL`], so it is a test failure rather than a
    /// crash.
    fn probe_suspend(ctx, arg, out) {
        ctx.helper_calls += 1;
        let n = arg.as_int();
        if n == 42 {
            // Borrowed, not allocated — see `Ctx::pending`.
            ctx.pending = Some("NvsError: helper refused 42".into());
            return THROWN;
        }
        if n == 99 {
            panic!("unreachable branch reached with n=99");
        }

        assert!(!ctx.yielder.is_null(), "probe_suspend requires a coroutine");
        #[allow(
            unsafe_code,
            reason = "the yielder outlives every frame of the coroutine it belongs to"
        )]
        // SAFETY: `ctx.yielder` is set by the coroutine body before any call
        // into JIT code and cleared when the coroutine returns, so it is live
        // for the whole of this call.
        let yielder: &Yield = unsafe { &*ctx.yielder };

        // The stack switch, with JIT frames live above us.
        yielder.suspend(Waiting::Io(n as u64));
        ctx.suspends += 1;

        *out = Value::int(n * 2);
        OK
    }
}

/// The slow path behind an [ADR 0018](/docs/decisions/0018.md)
/// § 1 probe site — the shape `nvs_runtime::nvs_probe_stmt` has, reduced to
/// the one thing this probe needs to observe.
///
/// It returns nothing: a coverage probe cannot fail, so unlike a helper there
/// is no status for the site to check, and unlike a safepoint there is no stop
/// to propagate. That asymmetry is deliberate and is what makes a probe site
/// strictly cheaper than a poll — which is why the guard threshold is stated
/// against the poll's cost class rather than the poll's exact number.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the duration of the call.
/// Compiled code satisfies this by construction.
#[allow(
    unsafe_code,
    reason = "the probe signature is fixed by what codegen emits; the pointer \
              contract cannot be expressed in the type"
)]
pub unsafe extern "C" fn probe_stmt(ctx: *mut Ctx, _stmt: u32) {
    #[allow(
        unsafe_code,
        reason = "codegen guarantees the context pointer is live for the call"
    )]
    // SAFETY: reached only from compiled code, which always passes a live
    // request context. Nothing here can panic, so no `catch_unwind` is needed
    // to keep an unwind out of the JIT frame above.
    unsafe {
        (*ctx).probe_hits += 1;
    }
}

/// Which helper sits at the bottom of a compiled chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Helper {
    /// [`probe_double`] — pure computation.
    Double,
    /// [`probe_suspend`] — suspends the coroutine.
    Suspend,
}

impl Helper {
    const fn symbol(self) -> &'static str {
        match self {
            Self::Double => "probe_double",
            Self::Suspend => "probe_suspend",
        }
    }
}

// ---------------------------------------------------------------------------
// JIT harness
// ---------------------------------------------------------------------------

/// Compiles chains of native frames that use Novis's calling convention.
///
/// Each frame calls the next, checks the returned status, and either copies the
/// 16-byte result up or returns the status onward — exactly the code shape
/// `nvs-codegen` will emit.
pub struct Probe {
    module: JITModule,
    ctx: codegen::Context,
    fn_ctx: FunctionBuilderContext,
    /// Distinguishes symbol names across repeated `compile_chain` calls.
    seq: usize,
}

impl std::fmt::Debug for Probe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Probe")
            .field("seq", &self.seq)
            .finish_non_exhaustive()
    }
}

impl Probe {
    /// Builds a JIT module for the host machine with the probe helpers linked in.
    ///
    /// # Panics
    ///
    /// Panics if the host is not a Cranelift target, or if the ISA cannot be
    /// constructed — either is an environment problem, not a runtime condition.
    #[must_use]
    pub fn new() -> Self {
        let mut flags = settings::builder();
        flags.set("use_colocated_libcalls", "false").unwrap();
        flags.set("is_pic", "false").unwrap();
        // Match what Novis's baseline tier will ask for, so the measured costs
        // correspond to shipped code rather than to a debug configuration.
        flags.set("opt_level", "speed").unwrap();

        let isa = cranelift_native::builder()
            .expect("host machine is not a supported cranelift target")
            .finish(settings::Flags::new(flags))
            .expect("failed to construct the host ISA");

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        builder.symbol(Helper::Double.symbol(), probe_double as *const u8);
        builder.symbol(Helper::Suspend.symbol(), probe_suspend as *const u8);
        builder.symbol("probe_stmt", probe_stmt as *const u8);

        let module = JITModule::new(builder);
        Self {
            ctx: module.make_context(),
            fn_ctx: FunctionBuilderContext::new(),
            module,
            seq: 0,
        }
    }

    /// The Novis function signature: `(ctx, arg, out) -> status`.
    fn signature(&self) -> Signature {
        let ptr = types::I64;
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr)); // ctx
        sig.params.push(AbiParam::new(ptr)); // arg
        sig.params.push(AbiParam::new(ptr)); // out
        sig.returns.push(AbiParam::new(types::I32)); // status
        sig
    }

    /// [`probe_stmt`]'s signature: `(ctx, stmt_id)`, no return.
    fn probe_signature(&self) -> Signature {
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // ctx
        sig.params.push(AbiParam::new(types::I32)); // stmt id
        sig
    }

    /// Compiles `depth` nested frames whose innermost call target is `helper`,
    /// and returns the outermost.
    ///
    /// Every frame forwards the *same* argument pointer downward, so only the
    /// helper performs arithmetic. That is deliberate: it isolates the cost of a
    /// frame — the call, the status check and the 16-byte result copy — from the
    /// cost of whatever work a real function would do.
    ///
    /// # Panics
    ///
    /// Panics if `depth` is 0, or if Cranelift rejects the generated IR, which
    /// would be a bug in this harness.
    pub fn compile_chain(&mut self, depth: usize, helper: Helper) -> NvsFn {
        self.compile_probe_chain(depth, helper, 0, false)
    }

    /// [`Self::compile_chain`], with `stmts` statement-shaped stores in each
    /// frame ahead of its call, each optionally preceded by
    /// [ADR 0018](/docs/decisions/0018.md)
    /// § 1's debug-flags check: load one word out of the context, branch on
    /// non-zero, fall through to the next statement.
    ///
    /// # Why a store, and not just the check
    ///
    /// Measuring the check means compiling the same chain twice and
    /// subtracting, and that only works if the two chains differ by exactly
    /// the check. A chain of bare, back-to-back loads of one address with
    /// nothing in between is not what real code looks like, and Cranelift's
    /// alias analysis would be entitled to collapse them into one — which
    /// would measure a single load however many probe sites were asked for.
    /// A statement that *does something* is both more faithful and what makes
    /// the load unavoidable: a store through the context pointer is exactly
    /// the kind of write that may alias the flags word, so each site's load
    /// stands. Both chains pay for the stores; the difference is the checks.
    ///
    /// # Panics
    ///
    /// See [`Self::compile_chain`].
    pub fn compile_probe_chain(
        &mut self,
        depth: usize,
        helper: Helper,
        stmts: usize,
        probe: bool,
    ) -> NvsFn {
        assert!(depth > 0, "a chain needs at least one frame");
        let sig = self.signature();
        let probe_sig = self.probe_signature();
        let probe_id: FuncId = self
            .module
            .declare_function("probe_stmt", Linkage::Import, &probe_sig)
            .expect("failed to declare the probe import");
        let seq = self.seq;
        self.seq += 1;

        let mut callee: FuncId = self
            .module
            .declare_function(helper.symbol(), Linkage::Import, &sig)
            .expect("failed to declare the helper import");

        for level in 0..depth {
            let id = self
                .module
                .declare_function(&format!("chain{seq}_{level}"), Linkage::Local, &sig)
                .expect("failed to declare a chain function");
            self.ctx.func.signature = sig.clone();

            {
                let target_config = self.module.target_config();
                let mut b = FunctionBuilder::new(&mut self.ctx.func, &mut self.fn_ctx);
                let callee_ref = self.module.declare_func_in_func(callee, b.func);
                let probe_ref = self.module.declare_func_in_func(probe_id, b.func);

                let entry = b.create_block();
                let ok_block = b.create_block();
                let err_block = b.create_block();

                // Scratch space for the callee's 16-byte result.
                let slot = b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    16,
                    4, // align_shift: 2^4 == 16 bytes
                ));

                b.append_block_params_for_function_params(entry);
                b.switch_to_block(entry);
                b.seal_block(entry);
                let ctx_p = b.block_params(entry)[0];
                let arg_p = b.block_params(entry)[1];
                let out_p = b.block_params(entry)[2];

                // The statement-shaped run this frame performs before its
                // call — see this method's own doc comment.
                for stmt in 0..stmts {
                    if probe {
                        // ADR 0018 § 1's site, exactly as `nvs-codegen` emits
                        // it: one load of the context's flags word, one
                        // branch predicted not taken, and an out-of-line call
                        // that never runs while every bit is off. `MemFlagsData`
                        // deliberately carries only `notrap` — the word is
                        // written from outside this frame, so nothing may
                        // treat the load as redundant.
                        let flags = b.ins().load(
                            types::I64,
                            MemFlagsData::new().with_notrap(),
                            ctx_p,
                            DEBUG_FLAGS_OFFSET,
                        );
                        let slow = b.create_block();
                        let cont = b.create_block();
                        b.ins().brif(flags, slow, &[], cont, &[]);

                        b.switch_to_block(slow);
                        b.seal_block(slow);
                        let id = b.ins().iconst(types::I32, stmt as i64);
                        b.ins().call(probe_ref, &[ctx_p, id]);
                        b.ins().jump(cont, &[]);

                        b.switch_to_block(cont);
                        b.seal_block(cont);
                    }
                    let value = b.ins().iconst(types::I64, stmt as i64);
                    b.ins()
                        .store(MemFlagsData::trusted(), value, ctx_p, SCRATCH_OFFSET);
                }

                let tmp_p = b.ins().stack_addr(types::I64, slot, 0);
                let call = b.ins().call(callee_ref, &[ctx_p, arg_p, tmp_p]);
                let status = b.inst_results(call)[0];

                // This pair of instructions is what replaces a landing pad.
                let failed = b.ins().icmp_imm_s(IntCC::NotEqual, status, 0);
                b.ins().brif(failed, err_block, &[], ok_block, &[]);

                b.switch_to_block(ok_block);
                b.seal_block(ok_block);
                let lo = b.ins().load(types::I64, MemFlagsData::trusted(), tmp_p, 0);
                let hi = b.ins().load(types::I64, MemFlagsData::trusted(), tmp_p, 8);
                b.ins().store(MemFlagsData::trusted(), lo, out_p, 0);
                b.ins().store(MemFlagsData::trusted(), hi, out_p, 8);
                let zero = b.ins().iconst(types::I32, 0);
                b.ins().return_(&[zero]);

                // Where a real frame drops its locals' refcounts before
                // propagating. Explicit IR, which is half the reason ADR 0002
                // prefers this to unwinding.
                b.switch_to_block(err_block);
                b.seal_block(err_block);
                b.ins().return_(&[status]);

                b.finalize(target_config);
            }

            self.module
                .define_function(id, &mut self.ctx)
                .expect("cranelift rejected the generated chain function");
            self.module.clear_context(&mut self.ctx);
            callee = id;
        }

        self.module
            .finalize_definitions()
            .expect("failed to finalize JIT definitions");

        let code = self.module.get_finalized_function(callee);
        #[allow(
            unsafe_code,
            reason = "the compiled function was generated with exactly this \
                      signature a few lines above"
        )]
        // SAFETY: `callee` was declared with `self.signature()`, which is
        // `NvsFn`'s signature, and `finalize_definitions` has made the code
        // executable.
        unsafe {
            std::mem::transmute::<*const u8, NvsFn>(code)
        }
    }
}

impl Default for Probe {
    fn default() -> Self {
        Self::new()
    }
}

/// Calls `f` with a fresh result slot and returns `(status, result)`.
///
/// The safe wrapper over [`NvsFn`]: deriving the three pointers from live
/// references is what discharges that type's safety contract, so probes never
/// need an `unsafe` block of their own.
pub fn call(f: NvsFn, ctx: &mut Ctx, arg: Value) -> (i32, Value) {
    let mut out = Value::default();
    #[allow(
        unsafe_code,
        reason = "the three pointers are derived from live, non-aliasing \
                  references in this frame, which is exactly NvsFn's contract"
    )]
    // SAFETY: `ctx`, `arg` and `out` are borrowed from locals that outlive the
    // call, are non-null and aligned, and `out` is a unique mutable borrow.
    let status = unsafe { f(ctx, &arg, &mut out) };
    (status, out)
}

/// What came out of a coroutine run.
#[derive(Debug)]
pub struct CoroutineRun<R> {
    /// How many times the body suspended before completing.
    pub suspends: usize,
    /// The context, handed back so the caller can inspect its counters.
    pub ctx: Ctx,
    /// Whatever the body returned.
    pub value: R,
}

/// Runs `body` on its own stack, driving it to completion.
///
/// This is what Novis's scheduler will do when it enters a task: publish the
/// yielder into the request context so that *any* helper can suspend, then
/// resume until the task finishes. Because the yielder is reached through the
/// context rather than through a function's signature, nothing in the call chain
/// needs to be marked `async` — which is the whole point of choosing stackful
/// coroutines.
///
/// The context is moved in and handed back out rather than borrowed, because
/// `corosensei::Coroutine::new` requires a `'static` closure. That also keeps
/// callers free of raw pointers.
///
/// # Panics
///
/// Propagates a panic from `body` only if the helper wrapper failed to contain
/// it, which would itself be the failure the invariant tests look for.
pub fn in_coroutine<R: 'static>(
    mut ctx: Ctx,
    body: impl FnOnce(&mut Ctx) -> R + 'static,
) -> CoroutineRun<R> {
    let mut coro = corosensei::Coroutine::<(), Waiting, (Ctx, R)>::new(move |yielder, ()| {
        // Taking a raw pointer to the yielder is safe; only dereferencing it is
        // not, and that happens in the helpers, which own that `unsafe`.
        ctx.yielder = std::ptr::from_ref(yielder);
        let value = body(&mut ctx);
        // Cleared before the context leaves the coroutine, so no dangling
        // pointer can escape to the caller.
        ctx.yielder = std::ptr::null();
        (ctx, value)
    });

    let mut suspends = 0;
    loop {
        match coro.resume(()) {
            corosensei::CoroutineResult::Yield(_) => suspends += 1,
            corosensei::CoroutineResult::Return((ctx, value)) => {
                return CoroutineRun {
                    suspends,
                    ctx,
                    value,
                };
            }
        }
    }
}
