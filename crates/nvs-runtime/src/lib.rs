//! Novis's runtime: the half of the execution model that is ordinary Rust.
//!
//! `nvs-codegen` compiles an [`nvs_ir`]-shaped function to native code; this
//! crate owns everything that code *calls into* or *manipulates by pointer* —
//! the calling convention, the value representation, the heap layout of a
//! refcounted string, the per-request context, and the small closed set of
//! engine-owned helper functions.
//!
//! [`nvs_ir`]: ../nvs_ir/index.html
//!
//! It deliberately has **no dependency on any other Novis crate**, not even
//! `nvs-ir`. It sits at the bottom of the stack: `nvs-codegen` depends on both
//! and is the one place that maps an `nvs_ir::Helper` tag to a symbol name
//! from [`symbols`]. That keeps this crate testable on its own — every test
//! below runs without a backend existing at all.
//!
//! # The normative shapes
//!
//! 1. **The calling convention** is
//!    `rule:errors/propagation`'s
//!    `extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32`. Nothing
//!    unwinds through a JIT frame; a failure travels in the return value as
//!    [`OK`]/[`THROWN`]/[`FATAL`]. Every helper is written through
//!    [`nvs_helper!`], which supplies the mandatory `catch_unwind` wrapper —
//!    that wrapper is what contains a runtime panic to one request, and it is
//!    why `panic = "unwind"` is set in every profile of the workspace
//!    `Cargo.toml`. It is the *inner* of two boundaries: [`run_task`] is the
//!    outer one, at the root of a worker's task, where a fault with no helper
//!    frame beneath it is caught and [`TaskRoot`] decides whether the worker
//!    survives it (`abi`'s own module docs own the pair).
//! 2. **The value representation** is `docs/implementation-plan.md`'s
//!    § *Value representation*: a 16-byte tagged [`Value`]. Not NaN-boxed —
//!    PHP semantics need the full `i64` range.
//! 3. **Memory is refcounted**, copy-on-write. [`NvsStr`] is the string
//!    representation, and the one the `Hello, World!` slice needs; [`NvsObj`]
//!    is the instance one, and [`object`]'s own docs are the one home for
//!    every decision behind it — the field-slot width, the subclass layout
//!    rule and the opaque [`ClassDesc`]; [`NvsArray`] is the array one, and
//!    [`mod@array`]'s own docs are the one home for its ordered hash, its
//!    consume-one-reference-return-one mutation protocol, and the only place
//!    "copy-on-write" is literally true. [`release`] owns the single worklist
//!    every one of them is freed through.
//!
//! # A tagged value's heap half: none
//!
//! `mixed`, `?T` and every other union share one representation in compiled
//! code — `nvs_ir::Ty::Tagged`, whose own doc comment owns the decision. What
//! belongs *here* is the half this crate provides, and it is deliberately
//! small: **a tagged value allocates nothing.** It is a [`Value`] carried in a
//! register pair rather than a pointer to a box, so its two halves are the two
//! halves of the struct above and materializing one into an argument slot is
//! two stores. The only new primitive it needed is the pair
//! [`nvs_value_retain`]/[`nvs_value_release`], which take those two halves and
//! branch on the tag — the whole of what "its payload may or may not be
//! refcounted" costs, and an out-of-line call where a statically-typed value
//! calls [`nvs_str_retain`] or [`nvs_object_retain`] directly. No new tag, no
//! new heap shape, no second release path: [`release`]'s one worklist already
//! frees whatever the payload turns out to be.
//!
//! Everything else a tagged value needs is a *reader*, not a representation:
//! [`value_truthy`] answers `rule:expressions/truthy-positions`'s table for one and [`value_to_string`]
//! answers `rule:types/conversion`'s string rows, each branching on the tag out of line so
//! that compiled code keeps knowing exactly one tag layout.
//!
//! # `bytes` is a tag, not a second heap shape
//!
//! `rule:types/bytes` makes `bytes` a
//! scalar of its own, and it lands here as **one new [`Tag`] row over the
//! existing [`NvsStr`] allocation**. A `bytes` payload is a [`StrHeader`]
//! pointer, allocated, retained, released and freed by exactly the machinery
//! `string` already has; what differs is the tag byte, and nothing else.
//!
//! The two halves of that are separate choices, and each is answerable on its
//! own:
//!
//! * **One heap shape**, because the difference between the two types is the
//!   UTF-8 promise, which is a checker property. A second buffer would be the
//!   "second arena setup" `rule:types/bytes` already rejected, and it would make
//!   § 3's `string as bytes` row — *total, free, the same buffer reinterpreted*
//!   — allocate. As it stands that conversion is a retain and a tag byte, and
//!   `bytes as string` is a UTF-8 validation over a borrow. Neither copies.
//! * **Two tags**, because a tag exists precisely to answer "which type is
//!   this?" where the static type no longer does. Sharing [`Tag::Str`] costs
//!   nothing while every `bytes` is statically typed and costs correctness the
//!   moment one is erased into a `mixed`: [`value_to_string`] would silently
//!   stringify unvalidated octets, [`value_identical`] would make a digest
//!   equal to the text that spells it although `rule:expressions/equality-semantics` makes the two
//!   types disjoint, and `Core\Json::encode` could not tell a payload it must
//!   refuse from one it may emit. Priority 2 over priority 5, per
//!   [AGENTS.md](/AGENTS.md)'s ordering.
//!
//! **What it spends is nothing per value** — no wider `Value`, no extra
//! allocation, no second release path ([`release`] keeps one arm for the pair,
//! reached through [`Value::buffer_ptr`]). The cost is paid once per *reader*:
//! every exhaustive match on [`Tag`] grows a row, and the compiler is what
//! collects that debt rather than a convention anyone has to remember.
//!
//! A reader may state a rule of its own rather than copying `string`'s.
//! [`value_to_string`] **refuses** a `bytes`, because `rule:types/conversion` makes
//! `bytes as string` checked and an implicit `.` or `echo` is not that check;
//! [`helpers::bytes_to_string`] is that check, reached only from the explicit `as` and
//! retagging the same allocation once the octets validate.
//! [`value_truthy`] answers *empty is falsy, everything else truthy*, dropping
//! `string`'s `"0"` case: that case is PHP's numeric-string rule, and a `bytes`
//! never converts to a number. `rule:expressions/truthy-positions`'s table names no `bytes` row at all,
//! so this is the runtime's own decision and this is its home.
//!
//! # What is here, and what is deliberately not
//!
//! This is the runtime half of milestone M3's vertical slice (see
//! `docs/agent/loop-goal.md`), and every part of it is testable without a
//! backend. It covers exactly:
//!
//! * the ABI surface — [`OK`]/[`THROWN`]/[`FATAL`], [`Value`], [`Ctx`],
//!   [`NvsFn`], [`nvs_helper!`], and the safe [`call`] wrapper tests and
//!   codegen tests both go through;
//! * [`NvsStr`], the refcounted single-allocation string, with the
//!   `nvs_str_new`/`nvs_str_concat`/`nvs_str_concat_n`/`nvs_str_append`/
//!   `nvs_str_retain`/`nvs_str_release` primitives backing
//!   `nvs_ir::InstKind::ConstStr`/`Concat`/`StrAppend`/`Retain`/`Release`;
//! * the [`SafepointFlags`] word and `nvs_safepoint` slow path backing
//!   `nvs_ir::InstKind::Safepoint`, and the [`DebugFlags`] word
//!   `rule:testing/debug-probes`'s probe sites check, with [`nvs_probe_stmt`] as the
//!   statement-boundary probe's slow path;
//! * every `nvs_ir::Helper` variant — see [`helpers`];
//! * the pending exception, with the [`nvs_raise`]/[`nvs_raise_new`]/
//!   [`nvs_trace_push`]/[`nvs_take_thrown`] primitives behind it. The value
//!   itself is an ordinary [`ObjHeader`] — see [`throwable`]'s own docs for
//!   why there is no second representation, which slots the runtime reaches by
//!   index, and why the backtrace is built as the throw propagates rather than
//!   at construction. It **subsumes** the bare message rather than sitting
//!   beside one in [`Ctx`] — see that module's `Pending` for why one field
//!   carries both levels of detail;
//! * [`FaultSite`], the closed set of failures a run can be *asked* to
//!   produce, so a contained engine panic — which has no user-facing trigger
//!   by definition — is testable at all;
//! * [`NvsObj`]/[`ObjHeader`]/[`ClassDesc`]/[`ClassTable`], M4's class-instance
//!   representation, with the `nvs_object_new`/`_retain`/`_release`/
//!   `_instanceof`/`_field_get`/`_field_set`/`_class_name` primitives behind
//!   `nvs_ir::InstKind::New`/`FieldGet`/`FieldSet` and an instance
//!   `InstKind::Call`'s receiver. It does not wait on the codegen that emits
//!   any of them, for [`NvsStr`]'s reason: it is testable without a backend,
//!   and the layout is what codegen queries rather than restates;
//! * [`NvsArray`]/[`ArrayHeader`], M4's array representation, with the
//!   `nvs_array_new`/`_retain`/`_release`/`_get`/`_set`/`_append`/`_unset`/
//!   `_has_key`/`_count`/`_next_slot`/`_key_at`/`_value_at` primitives behind
//!   `nvs_ir::InstKind::ArrayNew`/`ArrayGet`/`ArraySet`/`ArrayAppend` and the
//!   `foreach` cursor. Independent of the codegen that emits them, for the
//!   same reason;
//! * [`Decimal`], `rule:types/decimal`'s scalar — sign, a 96-bit mantissa and a scale of
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
//!   than in `nvs-stdlib` because `Core\Arr`'s set members, `ObjectSet`,
//!   `ObjectMap` **and the `==` operator itself** all ask the same question,
//!   and a second answer would be a second set of PHP-divergence rules nothing
//!   keeps in step. Compiled code enters it by whichever door its operands'
//!   static types justify — [`nvs_array_eq`], [`nvs_str_eq`], an inline
//!   pointer comparison, or `nvs_value_identical` for a `mixed` operand — and
//!   [`identity`]'s docs own that choice too;
//! * the **allocator itself**. Every optimized binary that links this crate
//!   runs on `alloc::Pooled`, a per-thread cache of small blocks in front of
//!   [`System`](std::alloc::System) — a `#[global_allocator]` is chosen once
//!   for a whole crate graph, so this crate choosing one chooses it for
//!   `nvs-cli` and for anything embedding the runtime. **What it spends**, per
//!   `rule:programs/memory-priority`'s *say what
//!   you spend*: at most **~2 MB per thread** that has touched every size
//!   class — 16 classes of 16 bytes up to 256, 512 blocks each — held until
//!   the process exits and never returned to the platform. That is a
//!   priority 5 cost bought with a priority 3 gain the userland suite
//!   measures, and it is **O(threads), never O(requests served)**: the cache
//!   is not per request, does not grow with traffic, and holds no
//!   request-owned bytes. A debug build is left on the platform heap so
//!   valgrind still sees every free, and the `sanitizer` feature extends that
//!   to this crate's own test binary — the one build where the pool would
//!   otherwise sit under a memory checker (`counting_alloc`). That module's
//!   own doc owns each of those decisions and is the only place they are
//!   argued.
//!
//! **The request arena is refused rather than absent, and a helper suspends
//! through `Ctx::yielder`/`Ctx::set_yielder`** — the shape ADR 0002
//! § *Consequences* commits to. `rule:security/arena-is-an-ownership-root`
//! rejects a region per isolate, so [`object`]'s per-context live list plus its
//! walk are the ownership root instead — run at teardown by [`object::sweep`],
//! and while the request is still going by the collection a crossing of
//! `rule:errors/on-limit`'s memory ceiling asks for. That module's docs are the
//! home of the walk, of the two moments it is worth running, and of what each
//! spends.
//!
//! ## Known gaps
//!
//! Each is a missing *representation*, not a missing decision, and each is
//! named at the item it blocks:
//!
//! 8. **`nvs_safepoint` clears `DEBUG_BREAK` and acts on nothing.** The flag is
//!    dropped where `CPU_LIMIT` and `CANCEL` become [`FATAL`], since `nvs dap` —
//!    the adapter a stopped frame would be handed to — does not exist.
//!    — owner: M10

mod abi;
// Compiled where it is used: by the `#[global_allocator]` below in an
// optimized build, and in a test build by its own tests and by `counting_alloc`,
// which wraps it. A debug build takes the platform heap, for the reason that
// module's doc states.
#[cfg(any(test, not(debug_assertions)))]
mod alloc;
pub mod arith;
pub mod array;
pub mod budget;
pub mod capability;
pub mod closure;
pub mod commands;
#[cfg(test)]
pub(crate) mod counting_alloc;
pub mod csrf;
mod ctx;
pub mod decimal;
pub mod deferred;
pub mod dispatch;
pub mod drain;
pub mod environment;
pub mod floor;
mod fmt;
pub mod graph;
pub mod graphemes;
pub mod helpers;
pub mod host;
pub mod identity;
pub mod inproc;
pub mod logfile;
pub mod metrics;
pub mod object;
pub mod os;
pub mod peer;
pub mod pool;
pub mod record;
pub mod release;
pub mod routes;
pub mod script;
pub mod sequence;
pub mod source;
pub mod sse;
pub mod stream;
mod string;
pub mod sweep;
pub mod terminal;
pub mod throwable;
pub mod trace_context;
pub mod uuid;
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

/// Novis owns its allocator in **every** build that is not a test build, and
/// every binary that links this crate gets it: a `#[global_allocator]` is
/// chosen once for the whole crate graph.
///
/// [`budget`] is what makes this unconditional. The pool underneath it is still
/// optimized-builds-only — a debug build takes the platform heap, and so does a
/// `sanitizer` one, because a recycled block is a block a checker never sees
/// freed ([`alloc`] § *Why it is registered only in optimized builds*). What is
/// registered in every profile is the **counting** in front of that choice,
/// since a memory cap nothing counts against is not a cap and a debug build is
/// exactly where the acceptance fixtures run.
#[cfg(not(test))]
#[global_allocator]
static ACCOUNTING_ALLOCATOR: budget::Accounting = budget::Accounting;

pub use abi::{
    DEADLINE_POLL_BATCH, EXITED, FATAL, Fault, HelperFn, HelperFrame, HelperResult, NvsFn, OK,
    THROWN, TaskPanic, TaskRoot, Teardown, affordable, bounded_loop, call, run_helper, run_task,
};
pub use arith::nvs_float_pow;
pub use array::{
    ARRAY_REFCOUNT_OFFSET, ArrayHeader, NvsArray, SlotKey, nvs_array_append, nvs_array_count,
    nvs_array_get, nvs_array_get_index, nvs_array_has_key, nvs_array_key_at, nvs_array_new,
    nvs_array_next_slot, nvs_array_release, nvs_array_retain, nvs_array_set, nvs_array_set_index,
    nvs_array_unset, nvs_array_value_at, prime_empty_array,
};
pub use closure::{
    CLOSURE_ARITY_SLOT, CLOSURE_INVOKE, CLOSURE_PARAM_NAMES, CLOSURE_PARAM_NAMES_SLOT,
    CLOSURE_PARAM_TAG_ANY, CLOSURE_PARAM_TAGS_SLOT, call_closure, closure_arity,
    closure_param_names,
};
pub use ctx::{
    AnswerTable, AssertionOutcome, BodyNeed, BodySource, CARRIER_CLI_TEXT, CARRIER_HTML_MARKUP,
    CARRIER_TEXT_SLOT, CoreClasses, Counted, Ctx, CurrentStack, DEADLINE_OFFSET,
    DEBUG_FLAGS_OFFSET, DebugFlags, DeclaredHeader, ErrorClass, EventStreamDoor, FaultSite,
    HOLD_PIECE, HOT_LINE_BYTES, HeldChild, HeldConnection, HeldReader, HeldSocket, HeldValue,
    HttpAnswer, HttpSent, Inbound, InboundSpec, Limit, LogChannel, LogWriter, OpenSpawn,
    OutputSink, PlacedIsolate, RequestBody, SAFEPOINT_OFFSET, SPAN_EVENT_CEILING, STACK_CEILING,
    STACK_LIMIT_OFFSET, STACK_RESERVE, STATICS_OFFSET, SafepointFlags, SafepointView, Scheme,
    Session, SnapshotMismatch, SocketAnswer, SocketFrame, SpawnForm, SpecBody, SpecPart, SseSlot,
    TraceEvent, TraceKind, TreeState, Upgrade, UpgradeSlot, is_carrier, nvs_probe_call_enter,
    nvs_probe_call_exit, nvs_probe_stmt, nvs_safepoint, nvs_stack_check,
};
pub use decimal::{Decimal, NotDecimal};
pub use dispatch::{
    CrossedFixtures, Fixtures, RowValues, call_compare_to, call_erased_method,
    call_erased_method_from, call_method, call_render, call_static, call_static_bound,
    call_static_on, construct_and_call, construct_erased_from, method_address,
};
pub use drain::{Drain, DrainWake};
pub use fmt::php_float_to_string;
pub use graph::{GraphError, copy_graph, copy_graph_into, decode, encode};
pub use helpers::{stringify, symbols, to_float, to_int, to_uint, value_to_string, value_truthy};
pub use identity::{
    numeric_identical, numeric_ordering, nvs_array_eq, value_hash, value_identical,
};
pub use object::{
    COMPARE_TO, CONSTRUCTOR, ClassDesc, ClassId, ClassTable, CodecElement, CodecField, CodecTy,
    EnumCases, EnumDesc, FIELD_STRIDE, FIELDS_OFFSET, FieldDefault, HookRow, MethodRow, NvsObj,
    OBJ_ALIGN, OBJ_CLASS_OFFSET, OBJ_REFCOUNT_OFFSET, ObjHeader, ShapeCodec, construct,
    field_offset, immortal_object_bytes, nvs_abstract_method, nvs_class_method,
    nvs_object_class_name, nvs_object_field_get, nvs_object_field_set, nvs_object_instanceof,
    nvs_object_key_get, nvs_object_key_set, nvs_object_new, nvs_object_release, nvs_object_retain,
    nvs_object_slot_get, nvs_object_slot_optional_get, nvs_object_slot_probe, nvs_object_slot_set,
    nvs_value_instanceof, write_erased_property,
};
pub use peer::{
    Closing, Delivery, INBOX_CAP, Inbox, PeerError, PeerFrame, PeerSocket, slow_subscribers_closed,
};
pub use string::{
    CAP_OFFSET, COUNT_UNKNOWN, GRAPHEMES_OFFSET, HEADER_ALIGN, IMMORTAL_REFCOUNT, LEN_OFFSET,
    NvsStr, PAYLOAD_OFFSET, REFCOUNT_OFFSET, StrHeader, StrWriter, immortal_header_bytes,
    nvs_str_append, nvs_str_concat, nvs_str_concat_n, nvs_str_eq, nvs_str_new, nvs_str_release,
    nvs_str_retain,
};
pub use throwable::{
    BACKTRACE_SLOT, CONSTRAINT_SLOT, DRIVER_CODE_SLOT, ENTRY_SCRIPT_FRAME, FINISH_MARKER_NAME,
    ISSUES_SLOT, KIND_SLOT, LOCATION_SLOT, MESSAGE_SLOT, PREVIOUS_SLOT, REASON_SLOT, SLOT_COUNT,
    SQL_SLOT, SQL_STATE_SLOT, Thrown, ThrownClass, is_finish, nvs_raise, nvs_raise_new,
    nvs_raise_site, nvs_take_thrown, nvs_trace_push,
};
pub use trace_context::TraceContext;
pub use value::{Tag, Value, nvs_value_release, nvs_value_retain};
