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
//!    representation to land, and the one the `Hello, World!` slice needs.
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
//!   `mwl_str_new`/`mwl_str_retain`/`mwl_str_release` primitives backing
//!   `mwl_ir::InstKind::ConstStr`/`Retain`/`Release`;
//! * the [`SafepointFlags`] word and `mwl_safepoint` slow path backing
//!   `mwl_ir::InstKind::Safepoint`, and the [`DebugFlags`] word
//!   [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
//!   § 1's probe sites check;
//! * nine of the ten `mwl_ir::Helper` variants — see [`helpers`].
//!
//! ## Known gaps
//!
//! Each is a missing *representation*, not a missing decision, and each is
//! named at the item it blocks:
//!
//! 1. **Arrays and objects have no runtime representation yet**, so
//!    `mwl_ir::Helper::ArrayTruthy` has no entry point, and
//!    [`Value::release`] ignores the `Array`/`Object`/`Closure`/`Resource`
//!    tags rather than decrementing anything. Those tags exist in [`Tag`]
//!    because the plan's § *Value representation* names them; nothing
//!    constructs one.
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

mod abi;
mod ctx;
mod fmt;
pub mod helpers;
mod string;
mod value;

pub use abi::{FATAL, Fault, HelperFn, HelperResult, MwlFn, OK, THROWN, call, run_helper};
pub use ctx::{
    Ctx, DEBUG_FLAGS_OFFSET, DebugFlags, OutputSink, SAFEPOINT_OFFSET, SafepointFlags,
    mwl_safepoint,
};
pub use fmt::php_float_to_string;
pub use helpers::symbols;
pub use string::{
    LEN_OFFSET, MwlStr, PAYLOAD_OFFSET, REFCOUNT_OFFSET, StrHeader, mwl_str_new, mwl_str_release,
    mwl_str_retain,
};
pub use value::{Tag, Value};
