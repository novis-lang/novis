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
//!    rule and the opaque [`ClassDesc`]; [`MwlArray`] is the third, and
//!    [`mod@array`]'s own docs are the one home for its ordered hash, its
//!    consume-one-reference-return-one mutation protocol, and the only place
//!    "copy-on-write" is literally true today. [`release`] owns the single
//!    worklist all three are freed through.
//!
//! # A tagged value's heap half: none
//!
//! `mixed`, `?T` and every other union share one representation in compiled
//! code — `mwl_ir::Ty::Tagged`, whose own doc comment owns the decision. What
//! belongs *here* is the half this crate provides, and it is deliberately
//! small: **a tagged value allocates nothing.** It is a [`Value`] carried in a
//! register pair rather than a pointer to a box, so its two halves are the two
//! halves of the struct above and materializing one into an argument slot is
//! two stores. The only new primitive it needed is the pair
//! [`mwl_value_retain`]/[`mwl_value_release`], which take those two halves and
//! branch on the tag — the whole of what "its payload may or may not be
//! refcounted" costs, and an out-of-line call where a statically-typed value
//! calls [`mwl_str_retain`] or [`mwl_object_retain`] directly. No new tag, no
//! new heap shape, no second release path: [`release`]'s one worklist already
//! frees whatever the payload turns out to be.
//!
//! Everything else a tagged value needs is a *reader*, not a representation:
//! [`value_truthy`] answers ADR 0035's table for one and [`value_to_string`]
//! answers ADR 0007 § 2's string rows, each branching on the tag out of line so
//! that compiled code keeps knowing exactly one tag layout.
//!
//! # What is here, and what is deliberately not
//!
//! This is the runtime half of milestone M3's vertical slice (see
//! `docs/agent/loop-goal.md`), landed before `mwl-codegen` exists because it is
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
//! * every `mwl_ir::Helper` variant — see [`helpers`];
//! * the pending exception, with the [`mwl_raise`]/[`mwl_raise_new`]/
//!   [`mwl_trace_push`]/[`mwl_take_thrown`] primitives behind it. The value
//!   itself is an ordinary [`ObjHeader`] — see [`throwable`]'s own docs for
//!   why there is no second representation, which slots the runtime reaches by
//!   index, and why the backtrace is built as the throw propagates rather than
//!   at construction. It **subsumes** the message [`Ctx`] used to carry on its
//!   own rather than sitting beside it — see that module's `Pending` for why
//!   one field carries both levels of detail;
//! * [`FaultSite`], the closed set of failures a run can be *asked* to
//!   produce, so a contained engine panic — which has no user-facing trigger
//!   by definition — is testable at all;
//! * [`MwlObj`]/[`ObjHeader`]/[`ClassDesc`]/[`ClassTable`], M4's class-instance
//!   representation, with the `mwl_object_new`/`_retain`/`_release`/
//!   `_instanceof`/`_field_get`/`_field_set`/`_class_name` primitives behind
//!   `mwl_ir::InstKind::New`/`FieldGet`/`FieldSet` and an instance
//!   `InstKind::Call`'s receiver. Landed before `mwl-codegen` can emit any of
//!   them, for the same reason [`MwlStr`] was: it is testable without a
//!   backend, and the layout is what codegen queries rather than restates;
//! * [`MwlArray`]/[`ArrayHeader`], M4's array representation, with the
//!   `mwl_array_new`/`_retain`/`_release`/`_get`/`_set`/`_append`/`_unset`/
//!   `_has_key`/`_count`/`_next_slot`/`_key_at`/`_value_at` primitives behind
//!   `mwl_ir::InstKind::ArrayNew`/`ArrayGet`/`ArraySet`/`ArrayAppend` and the
//!   `foreach` cursor. Landed ahead of the codegen that emits them, same as
//!   the two above;
//! * [`value_identical`], the one strict-identity comparison over two
//!   [`Value`]s, and [`value_hash`], the hash that agrees with it. What
//!   identity *means* — including what it means for an object — is
//!   [`identity`]'s own docs, which is the home
//!   `docs/agent/loop-goal.md` names for that decision. It lives here rather
//!   than in `mwl-stdlib` because `Core\Arr`'s set members, `ObjectSet` and
//!   `ObjectMap` all ask the same question, and a second answer would be a
//!   second set of PHP-divergence rules nothing keeps in step.
//!
//! ## Known gaps
//!
//! Each is a missing *representation*, not a missing decision, and each is
//! named at the item it blocks:
//!
//! 1. **`Closure` and `Resource` have no runtime representation yet**, so
//!    [`Value::release`] ignores those two tags rather than decrementing
//!    anything. They exist in [`Tag`] because the plan's § *Value
//!    representation* names them; nothing constructs one. `Object` and `Array`
//!    no longer belong on this list — see [`object`] and [`mod@array`].
//! 2. **A string is refcounted but never mutated in place.** [`MwlArray`]
//!    implements copy-on-write in full, including the `refcount == 1` in-place
//!    fast path; [`MwlStr`] is immutable instead, so every producer allocates.
//!    Nothing in the language mutates a string in place yet, so there is no
//!    observable difference; the same fast path is a widening of [`MwlStr`],
//!    not a redesign.
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
//! 7. **An exception *this crate* builds carries a message and nothing
//!    else.** [`Thrown::new`] — reached from [`mwl_raise_new`] and from a
//!    helper's bare-message [`Fault`] — fills `message`, empties `backtrace`
//!    and `location`, and leaves `previous` null, because none of the three
//!    has a value to pass at that point. An exception MWL code constructs is
//!    unaffected: it is an ordinary [`ObjHeader`] built by an ordinary
//!    constructor, and `mwl_ir::lower` fills `location` at the `throw`. That
//!    `previous` cannot be set *at all* yet is a different gap, owned by
//!    `mwl_types::error_lib`, which explains why the synthesized constructor
//!    takes only a message.
//! 8. **There is no cycle collector, by decision rather than by omission.** A
//!    cyclic object or array graph is retained until the process exits. The wholesale
//!    request-heap drop makes cycles structurally unable to accumulate in the
//!    server (`docs/implementation-plan.md` § *Architecture*), so the optional
//!    mark-sweep collector belongs with M5/M6, where the stop-the-world path
//!    and the request arena exist. A long-running CLI script that builds
//!    cycles is the one shape that pays.

mod abi;
pub mod array;
pub mod closure;
#[cfg(test)]
mod counting_alloc;
mod ctx;
mod fmt;
pub mod helpers;
pub mod identity;
pub mod object;
pub mod release;
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
pub use array::{
    ARRAY_REFCOUNT_OFFSET, ArrayHeader, MwlArray, mwl_array_append, mwl_array_count, mwl_array_get,
    mwl_array_has_key, mwl_array_key_at, mwl_array_new, mwl_array_next_slot, mwl_array_release,
    mwl_array_retain, mwl_array_set, mwl_array_unset, mwl_array_value_at,
};
pub use closure::{CLOSURE_ARITY_SLOT, CLOSURE_INVOKE, call_closure};
pub use ctx::{
    Ctx, DEBUG_FLAGS_OFFSET, DebugFlags, ErrorClass, FaultSite, OutputSink, SAFEPOINT_OFFSET,
    SafepointFlags, TraceEvent, mwl_probe_call_enter, mwl_probe_call_exit, mwl_probe_stmt,
    mwl_safepoint,
};
pub use fmt::php_float_to_string;
pub use helpers::{symbols, value_to_string, value_truthy};
pub use identity::{value_hash, value_identical};
pub use object::{
    ClassDesc, ClassId, ClassTable, FIELD_STRIDE, FIELDS_OFFSET, MwlObj, OBJ_CLASS_OFFSET,
    OBJ_REFCOUNT_OFFSET, ObjHeader, field_offset, mwl_abstract_method, mwl_class_method,
    mwl_object_class_name, mwl_object_field_get, mwl_object_field_set, mwl_object_instanceof,
    mwl_object_new, mwl_object_release, mwl_object_retain,
};
pub use string::{
    LEN_OFFSET, MwlStr, PAYLOAD_OFFSET, REFCOUNT_OFFSET, StrHeader, mwl_str_concat, mwl_str_eq,
    mwl_str_new, mwl_str_release, mwl_str_retain,
};
pub use throwable::{
    BACKTRACE_SLOT, LOCATION_SLOT, MESSAGE_SLOT, PREVIOUS_SLOT, SLOT_COUNT, Thrown, mwl_raise,
    mwl_raise_new, mwl_take_thrown, mwl_trace_push,
};
pub use value::{Tag, Value, mwl_value_release, mwl_value_retain};
