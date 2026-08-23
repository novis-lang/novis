//! MWL's runtime: the half of the execution model that is ordinary Rust.
//!
//! `mwl-codegen` compiles an [`mwl_ir`]-shaped function to native code; this
//! crate owns everything that code *calls into* or *manipulates by pointer* —
//! the calling convention, the value representation, the heap layout of a
//! refcounted string, the per-request context, and the small closed set of
//! engine-owned helper functions.
//!
//! [`mwl_ir`]: ../mwl_ir/index.html
//!
//! It deliberately has **no dependency on any other MWL crate**, not even
//! `mwl-ir`. It sits at the bottom of the stack: `mwl-codegen` depends on both
//! and is the one place that maps an `mwl_ir::Helper` tag to a symbol name
//! from [`symbols`]. That keeps this crate testable on its own — every test
//! below runs without a backend existing at all.
//!
//! # The three normative shapes
//!
//! 1. **The calling convention** is
//!    [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s
//!    `extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32`. Nothing
//!    unwinds through a JIT frame; a failure travels in the return value as
//!    [`OK`]/[`THROWN`]/[`FATAL`]. Every helper is written through
//!    [`mwl_helper!`], which supplies the mandatory `catch_unwind` wrapper —
//!    that wrapper is what contains a runtime panic to one request, and it is
//!    why `panic = "unwind"` is set in every profile of the workspace
//!    `Cargo.toml`.
//! 2. **The value representation** is `docs/implementation-plan.md`'s
//!    § *Value representation*: a 16-byte tagged [`Value`]. Not NaN-boxed —
//!    PHP semantics need the full `i64` range.
//! 3. **Memory is refcounted**, copy-on-write. [`MwlStr`] is the first such
//!    representation to land, and the one the `Hello, World!` slice needs;
//!    [`MwlObj`] is the second, and [`object`]'s own docs are the one home for
//!    every decision behind it — the field-slot width, the subclass layout
//!    rule, the opaque [`ClassDesc`], and why a release is iterative.
//!
//! # What is here, and what is deliberately not
//!
//! This is the runtime half of milestone M3's vertical slice (see
//! `.claude/loop-goal.md`), landed before `mwl-codegen` exists because it is
//! testable without one. It covers exactly:
//!
//! * the ABI surface — [`OK`]/[`THROWN`]/[`FATAL`], [`Value`], [`Ctx`],
//!   [`MwlFn`], [`mwl_helper!`], and the safe [`call`] wrapper tests and
//!   codegen tests both go through;
//! * [`MwlStr`], the refcounted single-allocation string, with the
//!   `mwl_str_new`/`mwl_str_concat`/`mwl_str_retain`/`mwl_str_release`
//!   primitives backing `mwl_ir::InstKind::ConstStr`/`Concat`/`Retain`/
//!   `Release`;
//! * the [`SafepointFlags`] word and `mwl_safepoint` slow path backing
//!   `mwl_ir::InstKind::Safepoint`, and the [`DebugFlags`] word
//!   [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
//!   § 1's probe sites check, with [`mwl_probe_stmt`] as the
//!   statement-boundary probe's slow path;
//! * nine of the ten `mwl_ir::Helper` variants — see [`helpers`];
//! * [`ThrowableHeader`], the runtime-owned exception value MWL's `throw`
//!   raises and a `catch` binds, with the `mwl_exception_new`/
//!   `mwl_throwable_message`/`mwl_throwable_trace`/`mwl_raise`/
//!   `mwl_trace_push`/`mwl_take_thrown` primitives behind it. It **subsumes**
//!   the message [`Ctx`] used to carry on its own rather than sitting beside
//!   it — see that module's `Pending` for why one field carries both levels of
//!   detail, and [`throwable`]'s own docs for why the backtrace is built as
//!   the throw propagates rather than at construction;
//! * [`FaultSite`], the closed set of failures a run can be *asked* to
//!   produce, so a contained engine panic — which has no user-facing trigger
//!   by definition — is testable at all;
//! * [`MwlObj`]/[`ObjHeader`]/[`ClassDesc`]/[`ClassTable`], M4's class-instance
//!   representation, with the `mwl_object_new`/`_retain`/`_release`/
//!   `_instanceof`/`_field_get`/`_field_set`/`_class_name` primitives behind
//!   `mwl_ir::InstKind::New`/`FieldGet`/`FieldSet` and an instance
//!   `InstKind::Call`'s receiver. Landed before `mwl-codegen` can emit any of
//!   them, for the same reason [`MwlStr`] was: it is testable without a
//!   backend, and the layout is what codegen queries rather than restates.
//!
//! ## Known gaps
//!
//! Each is a missing *representation*, not a missing decision, and each is
//! named at the item it blocks:
//!
//! 1. **Arrays have no runtime representation yet**, so
//!    `mwl_ir::Helper::ArrayTruthy` has no entry point, and
//!    [`Value::release`] ignores the `Array`/`Closure`/`Resource` tags rather
//!    than decrementing anything. Those tags exist in [`Tag`] because the
//!    plan's § *Value representation* names them; nothing constructs one.
//!    `Object` no longer belongs on this list — see [`object`].
//! 2. **Copy-on-write is not implemented.** [`MwlStr`] is refcounted and
//!    immutable — every producer allocates. Nothing in the language mutates a
//!    string in place yet, so there is no observable difference; a
//!    `refcount == 1` fast path is a widening of [`MwlStr`], not a redesign.
//! 3. **A string literal allocates on every evaluation.** An immortal,
//!    statically-allocated header (refcount pinned, never freed) would let
//!    `ConstStr` be a constant pointer with no call at all. It needs codegen
//!    to emit the header into the unit's data section, so it lands with the
//!    backend, not here.
//! 4. **`Ctx` carries no coroutine yielder and no request arena.** Both are
//!    M4/M5 (`benches/abi-probe`'s own `Ctx` shows the yielder shape ADR 0002
//!    § *Consequences* commits to). A helper cannot suspend yet.
//! 5. **No custom panic hook is installed.** ADR 0002 § *Corollary* wants the
//!    panic message routed to the request log with its request id; there is
//!    no request log until M5, and the default hook's stderr output is the
//!    right destination for a CLI script until then. [`mwl_helper!`] already
//!    captures the message into [`Ctx`], so the hook is presentation, not
//!    containment.
//! 6. **`mwl_safepoint` acts on two of its four flags.** `CPU_LIMIT` and
//!    `CANCEL` become [`FATAL`]; `COLLECT` and `DEBUG_BREAK` are cleared and
//!    ignored, since neither the cycle collector nor `mwl dap` exists.
//! 7. **An exception carries a message and a backtrace and nothing else.** No
//!    code, no previous-exception chain, no file/line pair of its own, and no
//!    `backtrace` array — that last one is M4's explicit carry-over, since it
//!    returns `array<…>` and nothing lowers an array yet. A user class
//!    extending `Throwable` still has no runtime shape: every exception is a
//!    [`ThrowableHeader`], not an [`ObjHeader`]. Now that [`object`] exists,
//!    joining the two is a lowering decision rather than a missing
//!    representation — see `.claude/loop-goal.md`'s exception surface.
//! 8. **There is no cycle collector, by decision rather than by omission.** A
//!    cyclic object graph is retained until the process exits. The wholesale
//!    request-heap drop makes cycles structurally unable to accumulate in the
//!    server (`docs/implementation-plan.md` § *Architecture*), so the optional
//!    mark-sweep collector belongs with M5/M6, where the stop-the-world path
//!    and the request arena exist. A long-running CLI script that builds
//!    cycles is the one shape that pays.

mod abi;
#[cfg(test)]
mod counting_alloc;
mod ctx;
mod fmt;
pub mod helpers;
pub mod object;
mod string;
pub mod throwable;
mod value;

/// The leak guard in [`object`] measures the allocator rather than trusting a
/// refcount to have reached zero, so this crate's own test binary counts live
/// bytes per thread. See [`counting_alloc`] for why the counter is
/// thread-local and why it costs nothing outside `cfg(test)`.
#[cfg(test)]
#[global_allocator]
static COUNTING_ALLOCATOR: counting_alloc::Counting = counting_alloc::Counting;

pub use abi::{FATAL, Fault, HelperFn, HelperResult, MwlFn, OK, THROWN, call, run_helper};
pub use ctx::{
    Ctx, DEBUG_FLAGS_OFFSET, DebugFlags, FaultSite, OutputSink, SAFEPOINT_OFFSET, SafepointFlags,
    TraceEvent, mwl_probe_call_enter, mwl_probe_call_exit, mwl_probe_stmt, mwl_safepoint,
};
pub use fmt::php_float_to_string;
pub use helpers::symbols;
pub use object::{
    ClassDesc, ClassId, ClassTable, FIELD_STRIDE, FIELDS_OFFSET, MwlObj, OBJ_CLASS_OFFSET,
    OBJ_REFCOUNT_OFFSET, ObjHeader, field_offset, mwl_object_class_name, mwl_object_field_get,
    mwl_object_field_set, mwl_object_instanceof, mwl_object_new, mwl_object_release,
    mwl_object_retain,
};
pub use string::{
    LEN_OFFSET, MwlStr, PAYLOAD_OFFSET, REFCOUNT_OFFSET, StrHeader, mwl_str_concat, mwl_str_new,
    mwl_str_release, mwl_str_retain,
};
pub use throwable::{
    ThrowableHeader, mwl_exception_new, mwl_raise, mwl_take_thrown, mwl_throwable_message,
    mwl_throwable_release, mwl_throwable_retain, mwl_throwable_trace, mwl_trace_push,
};
pub use value::{Tag, Value};
