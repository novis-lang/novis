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
//! # `bytes` is a tag, not a second heap shape
//!
//! [ADR 0009](../../../docs/adr/0009-string-and-bytes.md) makes `bytes` a
//! scalar of its own, and it lands here as **one new [`Tag`] row over the
//! existing [`MwlStr`] allocation**. A `bytes` payload is a [`StrHeader`]
//! pointer, allocated, retained, released and freed by exactly the machinery
//! `string` already has; what differs is the tag byte, and nothing else.
//!
//! The two halves of that are separate choices, and each is answerable on its
//! own:
//!
//! * **One heap shape**, because the difference between the two types is the
//!   UTF-8 promise, which is a checker property. A second buffer would be the
//!   "second arena setup" ADR 0009 § 1 already rejected, and it would make
//!   § 3's `string as bytes` row — *total, free, the same buffer reinterpreted*
//!   — allocate. As it stands that conversion is a retain and a tag byte, and
//!   `bytes as string` is a UTF-8 validation over a borrow. Neither copies.
//! * **Two tags**, because a tag exists precisely to answer "which type is
//!   this?" where the static type no longer does. Sharing [`Tag::Str`] costs
//!   nothing while every `bytes` is statically typed and costs correctness the
//!   moment one is erased into a `mixed`: [`value_to_string`] would silently
//!   stringify unvalidated octets, [`value_identical`] would make a digest
//!   equal to the text that spells it although ADR 0090 § 3 makes the two
//!   types disjoint, and `Core\Json::encode` could not tell a payload it must
//!   refuse from one it may emit. Priority 2 over priority 5, per
//!   [AGENTS.md](../../../AGENTS.md)'s ordering.
//!
//! **What it spends is nothing per value** — no wider `Value`, no extra
//! allocation, no second release path ([`release`] keeps one arm for the pair,
//! reached through [`Value::buffer_ptr`]). The cost is paid once per *reader*:
//! every exhaustive match on [`Tag`] grows a row, and the compiler is what
//! collects that debt rather than a convention anyone has to remember.
//!
//! Two readers state a rule of their own rather than copying `string`'s.
//! [`value_to_string`] **refuses** a `bytes`, because ADR 0009 § 3 makes
//! `bytes as string` checked and an implicit `.` or `echo` is not that check;
//! [`bytes_to_string`] is that check, reached only from the explicit `as` and
//! retagging the same allocation once the octets validate.
//! [`value_truthy`] answers *empty is falsy, everything else truthy*, dropping
//! `string`'s `"0"` case: that case is PHP's numeric-string rule, and a `bytes`
//! never converts to a number. ADR 0035's table names no `bytes` row at all,
//! so this is the runtime's own decision and this is its home.
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
//!   `mwl_str_new`/`mwl_str_concat`/`mwl_str_concat_n`/`mwl_str_append`/
//!   `mwl_str_retain`/`mwl_str_release` primitives backing
//!   `mwl_ir::InstKind::ConstStr`/`Concat`/`StrAppend`/`Retain`/`Release`;
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
//! * [`Decimal`], ADR 0054's scalar — sign, a 96-bit mantissa and a scale of
//!   0 to 28, with the whole of § 3's arithmetic and § 4's conversions. It is
//!   **not a second heap shape or a second register shape**: a `decimal` is a
//!   [`Value`] carrying [`Tag::Decimal`], whose mantissa spends the bytes the
//!   struct otherwise calls padding. [`decimal`]'s own module docs are the one
//!   home for the layout, for why one shape was chosen over two, and for what
//!   the choice spends;
//! * [`value_identical`], the one strict-identity comparison over two
//!   [`Value`]s, and [`value_hash`], the hash that agrees with it. What
//!   identity *means* — including what it means for an object — is
//!   [`identity`]'s own docs, which is the home
//!   `docs/agent/loop-goal.md` names for that decision. It lives here rather
//!   than in `mwl-stdlib` because `Core\Arr`'s set members, `ObjectSet`,
//!   `ObjectMap` **and the `==` operator itself** all ask the same question,
//!   and a second answer would be a second set of PHP-divergence rules nothing
//!   keeps in step. Compiled code enters it by whichever door its operands'
//!   static types justify — [`mwl_array_eq`], [`mwl_str_eq`], an inline
//!   pointer comparison, or `mwl_value_identical` for a `mixed` operand — and
//!   [`identity`]'s docs own that choice too;
//! * the **allocator itself**. Every optimized binary that links this crate
//!   runs on `alloc::Pooled`, a per-thread cache of small blocks in front of
//!   [`System`](std::alloc::System) — a `#[global_allocator]` is chosen once
//!   for a whole crate graph, so this crate choosing one chooses it for
//!   `mwl-cli` and for anything embedding the runtime. **What it spends**, per
//!   [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)'s *say what
//!   you spend*: at most **~2 MB per thread** that has touched every size
//!   class — 16 classes of 16 bytes up to 256, 512 blocks each — held until
//!   the process exits and never returned to the platform. That is a
//!   priority 5 cost bought with a priority 3 gain measured at 0.31× → 0.54×
//!   of PHP on the userland suite, and it is **O(threads), never
//!   O(requests served)**: the cache is not per request, does not grow with
//!   traffic, and holds no request-owned bytes. A debug build is left on the
//!   platform heap so valgrind still sees every free, and the `sanitizer`
//!   feature extends that to this crate's own test binary — the one build
//!   where the pool would otherwise sit under a memory checker
//!   ([`counting_alloc`]). That module's own doc
//!   owns each of those decisions and is the only place they are argued.
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
//! 2. **Appending is the only string operation with an in-place fast path.**
//!    [`mwl_str_append`] writes into its target's spare capacity at a
//!    `refcount == 1`, so `$out .= $piece` is linear; every other producer —
//!    [`mwl_str_concat`], [`mwl_str_concat_n`], every `Core\Str` member —
//!    allocates its result. That is a widening of [`MwlStr`] wherever a
//!    producer can prove sole ownership, not a redesign, and [`MwlArray`]'s
//!    copy-on-write is the shape it would take.
//! 3. **`Ctx` carries no coroutine yielder and no request arena.** Both are
//!    M4/M5 (`benches/abi-probe`'s own `Ctx` shows the yielder shape ADR 0002
//!    § *Consequences* commits to). A helper cannot suspend yet.
//! 4. **No custom panic hook is installed.** ADR 0002 § *Corollary* wants the
//!    panic message routed to the request log with its request id; there is
//!    no request log until M5, and the default hook's stderr output is the
//!    right destination for a CLI script until then. [`mwl_helper!`] already
//!    captures the message into [`Ctx`], so the hook is presentation, not
//!    containment.
//! 5. **`mwl_safepoint` acts on two of its four flags.** `CPU_LIMIT` and
//!    `CANCEL` become [`FATAL`]; `COLLECT` and `DEBUG_BREAK` are cleared and
//!    ignored, since neither the cycle collector nor `mwl dap` exists.
//! 6. **An exception *this crate* builds carries a message and nothing
//!    else.** [`Thrown::new`] — reached from [`mwl_raise_new`] and from a
//!    helper's bare-message [`Fault`] — fills `message`, empties `backtrace`
//!    and `location`, and leaves `previous` null, because none of the three
//!    has a value to pass at that point. An exception MWL code constructs is
//!    unaffected: it is an ordinary [`ObjHeader`] built by an ordinary
//!    constructor, and `mwl_ir::lower` fills `location` at the `throw`. That
//!    `previous` cannot be set *at all* yet is a different gap, owned by
//!    `mwl_types::error_lib`, which explains why the synthesized constructor
//!    takes only a message.
//! 7. **There is no cycle collector, by decision rather than by omission.** A
//!    cyclic object or array graph is retained until the process exits. The wholesale
//!    request-heap drop makes cycles structurally unable to accumulate in the
//!    server (`docs/implementation-plan.md` § *Architecture*), so the optional
//!    mark-sweep collector belongs with M5/M6, where the stop-the-world path
//!    and the request arena exist. A long-running CLI script that builds
//!    cycles is the one shape that pays.

mod abi;
// Compiled where it is used: by the `#[global_allocator]` below in an
// optimized build, and in a test build by its own tests and by `counting_alloc`,
// which wraps it. A debug build takes the platform heap, for the reason that
// module's doc states.
#[cfg(any(test, not(debug_assertions)))]
mod alloc;
pub mod array;
pub mod closure;
#[cfg(test)]
mod counting_alloc;
mod ctx;
pub mod decimal;
pub mod dispatch;
mod fmt;
pub mod helpers;
pub mod identity;
pub mod object;
pub mod release;
pub mod sequence;
mod string;
pub mod throwable;
mod value;

/// The leak guard in [`object`] measures the allocator rather than trusting a
/// refcount to have reached zero, so this crate's own test binary counts live
/// bytes per thread, in front of [`alloc::Pooled`] rather than the platform
/// heap. See [`counting_alloc`] for why the counter is thread-local, why it
/// costs nothing outside `cfg(test)`, and why wrapping the shipped allocator
/// leaves both counts meaning what they meant.
#[cfg(test)]
#[global_allocator]
static COUNTING_ALLOCATOR: counting_alloc::Counting = counting_alloc::Counting;

/// MWL owns its allocator in every optimized build, and every binary that
/// links this crate gets it: a `#[global_allocator]` is chosen once for the
/// whole crate graph. See [`alloc`] for what the per-thread cache spends and
/// why a debug build is deliberately left on the platform heap.
///
/// The `sanitizer` feature takes this out too, for the reason
/// [`counting_alloc`] § *Why a sanitizer needs `Backing` to be the platform
/// heap* states: a recycled block is a block a checker never sees freed.
#[cfg(all(not(test), not(debug_assertions), not(feature = "sanitizer")))]
#[global_allocator]
static POOLED_ALLOCATOR: alloc::Pooled = alloc::Pooled;

pub use abi::{
    FATAL, Fault, HelperFn, HelperResult, MwlFn, OK, THROWN, affordable, call, run_helper,
};
pub use array::{
    ARRAY_REFCOUNT_OFFSET, ArrayHeader, MwlArray, SlotKey, mwl_array_append, mwl_array_count,
    mwl_array_get, mwl_array_get_index, mwl_array_has_key, mwl_array_key_at, mwl_array_new,
    mwl_array_next_slot, mwl_array_release, mwl_array_retain, mwl_array_set, mwl_array_set_index,
    mwl_array_unset, mwl_array_value_at,
};
pub use closure::{CLOSURE_ARITY_SLOT, CLOSURE_INVOKE, call_closure, closure_arity};
pub use ctx::{
    CARRIER_CLI_TEXT, CARRIER_HTML_MARKUP, CARRIER_TEXT_SLOT, Ctx, DEBUG_FLAGS_OFFSET, DebugFlags,
    ErrorClass, FaultSite, OutputSink, SAFEPOINT_OFFSET, STACK_CEILING, STACK_LIMIT_OFFSET,
    STACK_RESERVE, SafepointFlags, TraceEvent, is_carrier, mwl_probe_call_enter,
    mwl_probe_call_exit, mwl_probe_stmt, mwl_safepoint, mwl_stack_check,
};
pub use decimal::Decimal;
pub use dispatch::{call_method, method_address};
pub use fmt::php_float_to_string;
pub use helpers::{symbols, value_to_string, value_truthy};
pub use identity::{
    mwl_array_eq, numeric_identical, numeric_ordering, value_hash, value_identical,
};
pub use object::{
    CONSTRUCTOR, ClassDesc, ClassId, ClassTable, CodecField, CodecTy, FIELD_STRIDE, FIELDS_OFFSET,
    FieldDefault, MwlObj, OBJ_CLASS_OFFSET, OBJ_REFCOUNT_OFFSET, ObjHeader, construct,
    field_offset, mwl_abstract_method, mwl_class_method, mwl_object_class_name,
    mwl_object_field_get, mwl_object_field_set, mwl_object_instanceof, mwl_object_new,
    mwl_object_release, mwl_object_retain, mwl_object_slot_get, mwl_object_slot_set,
};
pub use string::{
    CAP_OFFSET, HEADER_ALIGN, IMMORTAL_REFCOUNT, LEN_OFFSET, MwlStr, PAYLOAD_OFFSET,
    REFCOUNT_OFFSET, StrHeader, StrWriter, immortal_header_bytes, mwl_str_append, mwl_str_concat,
    mwl_str_concat_n, mwl_str_eq, mwl_str_new, mwl_str_release, mwl_str_retain,
};
pub use throwable::{
    BACKTRACE_SLOT, ISSUES_SLOT, LOCATION_SLOT, MESSAGE_SLOT, PREVIOUS_SLOT, SLOT_COUNT, Thrown,
    ThrownClass, mwl_raise, mwl_raise_new, mwl_take_thrown, mwl_trace_push,
};
pub use value::{Tag, Value, mwl_value_release, mwl_value_retain};
