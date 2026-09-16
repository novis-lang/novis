//! Novis's baseline Cranelift backend: [`nvs_ir`]'s CFG/SSA form in, native
//! code behind `rule:errors/propagation`'s
//! calling convention out.
//!
//! This crate is the one place that knows *both* [`nvs_ir`] and
//! [`nvs_runtime`]. `nvs-runtime` deliberately depends on neither, so mapping
//! an [`nvs_ir::ir::Helper`] tag onto a symbol name from
//! [`nvs_runtime::symbols`] is this crate's job by design — see that crate's
//! own module docs.
//!
//! # The shape it emits
//!
//! Every compiled function has the one signature `rule:errors/propagation` makes normative:
//!
//! ```text
//! extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32
//! ```
//!
//! Nothing unwinds. Every call — a runtime helper, an Novis method, the
//! safepoint slow path — is followed by a compare-and-branch on the returned
//! status. That pair of instructions is what replaces a landing pad;
//! `benches/abi-probe`'s `compile_chain` measures its cost.
//!
//! ## Where a failing status goes
//!
//! Onward, but not blindly: [`nvs_ir::ir::Inst::on_error`] names a *landing
//! block* per call site, and the branch enters it carrying the status as a
//! block parameter. The landing block holds the frame's refcount cleanup and
//! ends in [`nvs_ir::ir::Terminator::Propagate`] (record this frame on the
//! exception's backtrace, then return the status) or
//! [`nvs_ir::ir::Terminator::Catch`] (on `THROWN`, enter the handler; anything
//! else takes that terminator's `onward` block, which sweeps the frame the
//! way a `Propagate` does). A `throw` reaches the same block through
//! [`nvs_ir::ir::Terminator::Throw`], which raises the exception and jumps
//! with a constant `THROWN`.
//!
//! **The backtrace is built on the error path, never the success path.** Each
//! frame's landing block passes a static label from the unit's own data
//! section to [`nvs_runtime::nvs_trace_push`]. The alternative — a push/pop
//! frame record around every call — would move that cost onto the path that
//! actually runs, which is precisely what `rule:errors/propagation` exists to avoid. See
//! `nvs_runtime::throwable`'s own docs for the one observable consequence.
//!
//! ## An object is a pointer; its fields are tagged
//!
//! An instance is a bare [`nvs_runtime::ObjHeader`] pointer in a register, and
//! `new` is one `iconst` of the class descriptor's address plus one call — see
//! [`Classes`] for why a JIT can bake that address in. A *field* is different:
//! every slot is a whole 16-byte [`nvs_runtime::Value`], so a `FieldGet` loads
//! the payload half at [`nvs_runtime::field_offset`] and a `FieldSet` stores
//! both halves. [`nvs_runtime::object`]'s own docs own that decision and state
//! its cost; this crate only queries the offset.
//!
//! The same is true of late static binding: a static method's argument slot 0
//! carries the called class rather than `null`, and a `ClassDescOf` is one
//! load at [`nvs_runtime::OBJ_CLASS_OFFSET`]. That decision — including why the
//! slot keeps a `null` *tag* — is `nvs_runtime::object`'s too; this crate emits
//! the load and the [`nvs_runtime::nvs_class_method`] lookup it feeds.
//!
//! ## Values are native, not tagged, wherever the type is known
//!
//! `rule:types/declaration` settles every
//! operand type before lowering, so an `int` local lives in an `i64` register
//! and a `string` in a bare `StrHeader` pointer. A 16-byte
//! [`nvs_runtime::Value`] is *materialized* only where the ABI demands one —
//! at a call boundary and at the `out` slot — exactly as that type's own docs
//! say. [`ty::clif_ty`] is the whole of the mapping.
//!
//! ## The hot-word checks
//!
//! Each is a load of one word from [`nvs_runtime::Ctx`] plus a
//! predicted-not-taken branch, and each is emitted unconditionally:
//!
//! * the **safepoint poll** at every [`nvs_ir::ir::InstKind::Safepoint`] —
//!   function entry and loop back edges, the project-start decision's fixed
//!   sites;
//! * `rule:errors/on-limit`'s
//!   **call-stack compare**, riding the *first* of those polls so that it
//!   lands at function entry and nowhere else — one load, one compare against
//!   Cranelift's `get_stack_pointer`, branching to
//!   [`nvs_runtime::nvs_stack_check`]. It is the one of them that is
//!   *elided*: `emit::is_leaf` answers which functions cannot grow the stack
//!   past the reserve their caller already checked with, and those carry none;
//! * `rule:testing/debug-probes`'s **debug-flags check**, at every
//!   [`nvs_ir::ir::InstKind::StmtMarker`] (branching to
//!   [`nvs_runtime::nvs_probe_stmt`]) and twice at every call site, before and
//!   after (branching to [`nvs_runtime::nvs_probe_call_enter`] and
//!   [`nvs_runtime::nvs_probe_call_exit`]).
//!
//! None is behind a flag or a build configuration: `rule:testing/debug-probes`'s whole
//! argument is that a request already running must be able to have coverage
//! turned on, which a compiled-in-advance instrumented tier cannot do, and ADR
//! 0020's is that a stack limit no build enables is a limit that is there when
//! the runaway arrives. Their nothing-set cost is guarded in
//! `benches/abi-probe` (`tests/perf_guards.rs`), not asserted here — the
//! call-stack compare is the one that shows, as a one-time step in
//! `abi/frame_depth`, which is a benchmark that does nothing but call.
//!
//! # Scope of this slice
//!
//! Narrow by authorization, not by accident — `docs/agent/loop-goal.md` allows
//! the first backend to be exactly as wide as `nvs run examples/hello.nvs`
//! requires, and each gap below is a missing *lowering*, not a missing
//! decision. **A closed gap is deleted and its number retired**, never reused
//! and never handed to a survivor, so the list has holes on purpose — every
//! `nvs-codegen` gap N named anywhere else in the tree keeps meaning what it
//! meant when it was written:
//!
//! Exceptions are ordinary objects here: there is no `Ty::Throwable`, a user
//! class `extends Throwable` compiles like any other, and a typed `catch` is an
//! [`nvs_ir::ir::InstKind::InstanceOf`] chain. A `finally` runs on every exit
//! from its region — `nvs_ir::lower::Lowering::lower_try` owns that policy
//! whole, and this backend emits the copies it lowers.
//!
//! 1. **Virtual dispatch is by name, not by slot.** An instance call whose
//!    resolved declaration some subtype overrides — and the two shapes with
//!    no static answer at all, `static::method(...)`/`new static(...)` and a
//!    call resolving to a declaration with no *body* — lower to
//!    [`nvs_ir::ir::InstKind::CallVirtual`]/`NewDynamic`, look the method up
//!    on the receiver's or the late-static-binding class through
//!    [`nvs_runtime::nvs_class_method`], and call the address it returns
//!    indirectly under the same `rule:errors/propagation` signature. Everything else binds
//!    straight to a label, because `nvs_types` answers "does anything
//!    override this" for the whole program once
//!    (`nvs_types::expr_table::ResolvedCall::overridden`). What is left is a
//!    per-class slot index instead of a string compare, which
//!    [`nvs_ir::ir::Program::classes`] already carries the table for.
//!
//!    Everything else about objects and arrays compiles: `New`, `FieldGet`,
//!    `FieldSet`, an instance `Call`, every array instruction — `ArrayNew`,
//!    `ArrayGet`, `ArraySet`, `ArrayAppend`, `ArraySpread`, `ArrayUnset` and
//!    `foreach`'s
//!    `ArrayNextSlot`/`ArrayKeyAt`/`ArrayValueAt` cursor — and a
//!    `Ty::Object`/`Ty::Array` retain/release, against
//!    [`nvs_runtime::object`]'s layout, the per-class slot table
//!    [`nvs_ir::ir::Program::classes`] carries, and [`nvs_runtime::array`]'s
//!    primitives. A Tier 0 `Core` member call
//!    ([`nvs_ir::ir::InstKind::CoreCall`]) compiles too, through the helper
//!    path unchanged — [`nvs_stdlib`]'s own docs own why it needs no path of
//!    its own.
//! 2. **`rule:testing/debug-probes`'s `BRANCH` probe is not emitted.** It needs a per-edge site
//!    at [`nvs_ir::ir::Terminator::Branch`]'s lowering, which is the only one
//!    of that ADR's sites still missing — the statement-boundary probe and the
//!    call-site `TRACE`/`PROFILE` pair are both emitted.
//! 4. **Two identical string literals are two data objects.** Each
//!    `InstKind::ConstStr` emits its own immortal header and payload under its
//!    own name, so a unit that writes `"id"` in forty places holds forty
//!    copies of it. Nothing on the request path pays — each site materializes
//!    one address either way — so what this costs is unit bytes and some
//!    instruction-cache locality, and closing it means keying a map on the
//!    literal's bytes beside `emit::Emitter::define_literal`'s counter.
//! 5. **Integer `/` compiles, and it is the one operator that picks its
//!    result representation at run time.**
//!    `rule:types/arithmetic` types
//!    `int / int` as `int|float` — PHP-exact, so `6/3` is an integer and `7/2`
//!    is not — and that union's representation is [`nvs_ir::Ty::Tagged`], so
//!    `emit_binop` hands the row to its own `emit_int_div`: a zero-divisor
//!    guard in front, then a branch on whether the remainder is zero, joining
//!    at a phi that carries the tagged value. The signed overflow
//!    `i64::MIN / -1` takes the inexact arm rather than a second throw,
//!    because PHP answers a `float` for it. `nvs_ir::lower::Lowering::coerce`
//!    is where the union is absorbed back into a declared `float`, which is
//!    what makes `float $avg = $sum / $n;` the ADR's own worked example.
//! 6. **[`nvs_ir::ir::Terminator::Switch`] lowers to a compare chain, not a
//!    jump table.** Correct for any case set — the IR deliberately does not
//!    require a dense or sorted one — and the arms are few in its one
//!    producer, `rule:iteration/generators`'s generator resumption (one per
//!    `yield`, plus the entry and exhausted arms). A `br_table` over a dense
//!    case set is the obvious optimisation. Novis's own `switch` statement never
//!    reaches this terminator — it lowers to a `Branch` chain, since a label
//!    is any expression of the subject's type — so closing this gap would also
//!    mean teaching `nvs_ir::lower::Lowering::lower_switch` to recognise a
//!    dense all-integer case set and reach for it.
//! 7. **Executable memory is never freed.** [`Unit`] holds its `JITModule` for
//!    the process's lifetime; `cranelift_jit::JITModule::free_memory` is
//!    `unsafe` and needs the "no compiled frame is still live" proof that
//!    `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s
//!    pointer-swap reclamation is the real home for. A one-shot `nvs run`
//!    exits before it matters.
//! 8. **Integer `+`, `-`, `*` and unary `-` throw on overflow rather than
//!    wrapping**, which
//!    `rule:types/arithmetic` calls the
//!    divergence from PHP it is least willing to trade. `emit_binop` hands the
//!    binary rows to `emit_checked_int_arith` and `emit_unop` takes the unary
//!    one, each reading Cranelift's `sadd_overflow`/`uadd_overflow` family
//!    — the flag the CPU already sets, so the cost is one predicted branch and
//!    no synthesized compare — and raising spec § 10's `ArithmeticError` on
//!    [`nvs_ir::ir::Inst::on_error`]'s edge through the shared
//!    `raise_arithmetic_error`, which the zero-divisor guards use too.
//!    The signed and unsigned rows are different instructions rather than one
//!    read two ways: a carry out of bit 63 is not a sign flip, which is what
//!    keeps `uint` exact over `0 … 2^64−1`.
//! 9. **A binary operator wants both operands in one representation, and
//!    knows only the numeric and `bool` ones.** A mixed numeric pair does not
//!    reach here — `nvs_ir::lower` settles `1 + 1.5` by widening the integer
//!    side and `$n < $f` by a helper, which is what keeps this crate's "a
//!    `BinOp` has one representation" invariant a genuine internal error. What
//!    is refused is `==` over two enum values, whose `Enum(Int)`
//!    representation is not on the integral list even though comparing the two
//!    integers is exactly right — a missing arm rather than a missing
//!    mechanism, since `rule:enums/closed-integer-type` makes an enum *be* its integer.

mod emit;
mod ty;

use std::sync::{Arc, Mutex, PoisonError};

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, ModuleError};
use cranelift_object::{ObjectBuilder, ObjectModule};
use nvs_ir::Program;
use nvs_runtime::NvsFn;
use rustc_hash::FxHashMap;

pub use ty::clif_ty;

/// Why a program could not be compiled.
///
/// Every variant is an *engine* failure — a shape this backend does not lower
/// yet, or a host that cannot host a JIT. None of them is a user diagnostic:
/// `nvs run` has already run `nvs_types::check_program` and reported every
/// diagnostic before a single instruction is emitted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CodegenError {
    /// An IR shape this slice does not lower — see the crate docs' scope list.
    #[error("nvs-codegen does not lower {0} yet")]
    Unsupported(String),
    /// A question the IR's own invariants say is never asked, asked anyway.
    ///
    /// Always an engine bug rather than a language hole, exactly as
    /// [`CodegenError::UnknownTarget`] is, and separated from
    /// [`CodegenError::Unsupported`] for that reason rather than for its
    /// wording: `tools/holes.py` reads the `Unsupported` constructor as the
    /// inventory of shapes the language still refuses, and a bug sitting on
    /// that worklist is an item no session can ever close.
    /// `crates/nvs-ir/tests/refusals.rs` is the gate over that inventory.
    #[error("internal error: {0}")]
    Internal(String),
    /// Cranelift rejected the generated code, or the JIT module did.
    ///
    /// Rendered with the source's `Debug` as well as its `Display`: a verifier
    /// rejection's `Display` is the bare words "Verifier errors", and the
    /// instruction and value it actually names — the only part that says
    /// *which* lowering is wrong — lives one level down in the `Debug` form.
    #[error(
        "internal error: cranelift rejected the code generated for `{function}`: {source} \
         ({source:?})"
    )]
    Cranelift {
        /// The Novis function being compiled.
        function: String,
        /// What Cranelift reported.
        source: Box<ModuleError>,
    },
    /// A call naming a function this compilation unit does not define.
    ///
    /// Always an engine bug rather than a user error: `nvs_types` resolved
    /// the target before lowering ever rendered its label, so a unit that
    /// contains the call and not the callee was assembled wrong.
    #[error("internal error: `{caller}` calls `{target}`, which this unit does not define")]
    UnknownTarget {
        /// The Novis function containing the call.
        caller: String,
        /// The `"Class::method"` label it named.
        target: String,
    },
    /// The host machine cannot run a Cranelift JIT at all.
    #[error("this host is not a supported cranelift target: {0}")]
    UnsupportedHost(String),
}

/// One compiled compilation unit: the native code, plus the module that owns
/// the pages it lives on.
///
/// Holding the [`JITModule`] is what keeps the code mapped — see the crate
/// docs' known gap 7 on why nothing unmaps it.
///
/// # Calling into a unit from Rust
///
/// A compiled function's argument array is `[receiver, ...declared arguments]`
/// — `1 + arity` slots, slot 0 the implicit receiver and the first declared
/// parameter at slot **1**. [`nvs_runtime::NvsFn`] owns that rule. What matters
/// on this side of it is that [`nvs_runtime::call`] is handed a pointer and
/// never a length, so a hand caller that passes the declared arguments alone
/// reads one `Value` past the end of its own slice for *every* parameter and
/// gets no diagnostic from anywhere — it answers with whatever was next in
/// memory, which on a given platform can be the right answer by luck.
///
/// So the shape is not documented at the caller, it is **named**: take the
/// entry point that matches what you are calling and there is nothing left to
/// spell wrong.
///
/// | You are calling | Take |
/// |---|---|
/// | the entry script frame, which has no receiver at all | [`Self::script`], then [`ScriptFn::call`] |
/// | a `static` method — receiver is the called class | [`Self::call_static`] |
/// | an instance method, on a fresh instance | [`Self::call_on_new_instance`] |
/// | `rule:testing/fixtures`'s fixture | [`Self::build_fixture`] |
/// | nothing — you only want to know it compiled | [`Self::has_function`] |
///
/// [`Self::raw_function`] is the escape hatch underneath all of them, and the
/// only callers it should have are `benches/abi-probe`'s three, which build the
/// slot array once outside a timed loop on purpose. Reaching for it anywhere
/// else means writing the receiver slot by hand, which is the thing this list
/// exists to stop.
pub struct Unit {
    /// Kept alive for its pages; never read again after the unit is built.
    _code: Code,
    /// Kept alive for its *descriptors*: the compiled code holds each one's
    /// address as a baked-in constant (see [`Classes`]), so the table must
    /// outlive every instance and every frame that can allocate one. Moving
    /// the table here is safe because each descriptor is individually boxed —
    /// only the `Vec`'s own three words move, never a `ClassDesc`.
    ///
    /// Shared rather than owned outright so a [`nvs_runtime::ErrorClass`]
    /// handed to a [`nvs_runtime::Ctx`] can keep it alive by itself — that is
    /// what makes installing one need no `unsafe` at the call site.
    ///
    /// The share is atomic because one compiled unit is read by every core
    /// (`rule:security/isolate-shares-nothing`'s "immutable compiled code" is
    /// the whole of what crosses), so the handle has to be able to. What that
    /// costs is one atomic increment per install rather than one non-atomic
    /// one, against a table the unit owns exactly one of —
    /// `rule:programs/memory-priority`'s trade in the direction it is meant to
    /// go.
    classes: std::sync::Arc<nvs_runtime::ClassTable>,
    entries: FxHashMap<String, *const u8>,
    /// Every compiled function's declared shape, by the same name as
    /// [`Unit::entries`] — moved out of the builder rather than dropped with
    /// it, so [`Unit::call_static`] can check an argument count against the
    /// arity the callee was compiled with.
    ///
    /// It is a move and not a copy: the builder filled this map in its
    /// declaration pass and has no reader left after
    /// [`UnitBuilder::bind_method_tables`], so what this costs is the map's own
    /// bytes living as long as the `Unit` instead of being freed at
    /// `finish` — one `MethodShape` and one `String` key per compiled function,
    /// per unit. That buys a hand caller a refusal where it would otherwise
    /// read slots the callee's frame does not own, which is
    /// `rule:programs/memory-priority`'s trade in the
    /// direction it is meant to go.
    shapes: FxHashMap<String, MethodShape>,
    /// This unit's static-property initializers, in the slot order the
    /// compiled code baked in — `nvs_ir::ir::Program::statics`' own order.
    /// Handed to a context by [`Unit::install_in`].
    ///
    /// Shared rather than owned outright for the reason `classes` is: a context
    /// armed from this list keeps it, so an `rule:security/isolate-shares-nothing` method-entry isolate of
    /// that context can arm itself against the same slot numbering with no unit
    /// in hand (`nvs_runtime::Ctx::method_isolate`). Atomically shared for the
    /// reason `classes` is too: the unit behind it is one per program, not one
    /// per core.
    statics: std::sync::Arc<[Option<nvs_runtime::FieldDefault>]>,
}

/// Whatever keeps a [`Unit`]'s code mapped for as long as the unit lives.
///
/// A [`Unit`]'s addresses are raw pointers into pages somebody owns, and there
/// are two somebodies: [`compile`]'s own `JITModule`, and — since `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`
/// — a loader holding the private mapping it placed a cached payload into.
/// The unit is the same type either way, because everything above this field
/// reads an address and never asks where it came from; only the drop differs,
/// and both arms free exactly the pages they made.
#[expect(
    dead_code,
    reason = "both arms are held for their `Drop` and read by nothing: the addresses in \
              `Unit::entries` point into whichever mapping this owns, so what the field does \
              is outlive them"
)]
enum Code {
    /// A unit this process compiled: `cranelift-jit`'s own mapping.
    /// Boxed only to keep the two arms the same size: a `JITModule` is a few
    /// hundred bytes and a loader's owner is one pointer, and a `Unit` is moved
    /// more often than this is dropped.
    Jit(Box<JITModule>),
    /// A unit this process loaded, owned by whoever placed it — `nvs-cli`'s
    /// `cache::Loaded` is the only one, and it is in another crate, which is
    /// why this is a trait object rather than a named type.
    Placed(Box<dyn Placed>),
}

/// A placed payload, read by the symbol names `nvs-codegen` emitted it under.
///
/// This is the loader's counterpart to `Module::get_finalized_function`, and
/// the whole of what [`Descriptors::bind`] and [`Descriptors::into_unit`] need
/// from `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`'s mapping: the addresses this unit's own functions ended up
/// at. Implemented outside this crate, by whoever owns those pages.
///
/// **`Send + Sync` is a supertrait because a [`Unit`] crosses cores.** One
/// compiled unit is shared by the whole fleet, so whatever owns its pages is
/// shared too, and stating that here is what keeps [`Unit`]'s own `unsafe impl`
/// from having to vouch for an implementation in another crate. A loader whose
/// mapping is thread-affine cannot implement this trait, which is the refusal
/// the right way round.
pub trait Placed: Send + Sync {
    /// Where this owner placed the function `symbol` names, or [`None`] if the
    /// payload defines no such function — which § 3 makes a skipped method row
    /// rather than an error, exactly as the JIT path skips an undefined one.
    fn address_of(&self, symbol: &str) -> Option<*const u8>;
}

/// A unit is `Send` and `Sync` because one compiled unit serves every core.
///
/// That is the whole of `rule:security/isolate-shares-nothing`'s "immutable
/// compiled code": `nvs-cli`'s compiled-unit cache publishes one
/// [`std::sync::Arc<Unit>`](std::sync::Arc) and every core resolving a request
/// against that file reads it, rather than each core compiling the file for
/// itself. Two auto traits stand between that and the type, and each has one
/// reason it does not hold on its own:
///
/// - **[`Unit::entries`] holds `*const u8`**, and a raw pointer is neither.
///   Each is a function address inside pages [`Unit::_code`] owns, so it is
///   valid for exactly as long as the unit is, on whichever core reads it.
///   Moving the address is not the operation that needs a contract — calling
///   through it is, and [`Unit::call_static`] carries that `unsafe` itself.
/// - **[`Code::Jit`] holds a `JITModule`, which is `!Sync`** for the
///   `RefCell<HashMap<..>>` its symbol lookup memoizes into. Nothing here can
///   reach it: the field is private, is written by exactly the two constructors
///   below, and is read by *nothing* — its `#[expect(dead_code)]` is that fact
///   in the compiler's own words. It exists to be dropped, once, when the last
///   handle goes, and a `JITModule` is `Send`, so that drop is sound on any
///   core. [`Code::Placed`] needs no such argument: [`Placed`] states the bound.
///
/// Everything else the unit holds already crosses on its own terms —
/// [`nvs_runtime::ClassTable`] carries its own argument, `MethodShape` and
/// [`nvs_runtime::FieldDefault`] are plain data.
///
/// **What this does not say is that a unit is mutable from two cores.** It has
/// no `&mut self` method at all: everything that writes one belongs to
/// [`UnitBuilder`] or [`Descriptors`], both of which are consumed to produce it.
#[expect(
    unsafe_code,
    reason = "the two auto traits are blocked by a pointer whose pages the unit \
              owns and by a field nothing reads; the bullets above are the argument"
)]
// SAFETY: see the doc comment above — every address in `entries` points into
// pages `_code` keeps mapped for the unit's whole life, and `_code` itself is
// unreachable through `&Unit`.
unsafe impl Send for Unit {}

#[expect(
    unsafe_code,
    reason = "shared reads of a unit with no `&mut self` method, on `Send`'s \
              argument above"
)]
// SAFETY: as `Send` above. `&Unit` exposes reads only, and the one `!Sync`
// field is private and read by nothing.
unsafe impl Sync for Unit {}

impl std::fmt::Debug for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut names: Vec<&str> = self.entries.keys().map(String::as_str).collect();
        names.sort_unstable();
        f.debug_struct("Unit").field("functions", &names).finish()
    }
}

/// A [`Unit`]'s entry script frame, ready to run.
///
/// A script frame is the one compiled function with **no** implicit receiver
/// (`nvs_ir::lower`'s *No implicit receiver*), so its ABI slot array is empty.
/// This type is how that stops being something a caller has to know: there is
/// no argument to pass, so there is no argument to pass short, and [`Self::call`]
/// is the only spelling of the call.
///
/// One word and `Copy`, so a caller that has to know the frame exists before it
/// builds the machinery to run it — `nvs run` checks before it starts a
/// scheduler — carries this across the gap instead of carrying a label and
/// looking it up again later.
#[derive(Debug, Clone, Copy)]
pub struct ScriptFn(NvsFn);

impl ScriptFn {
    /// Runs the frame on `ctx`.
    ///
    /// # Errors
    ///
    /// The status the run reported, with its message left on `ctx` — this is
    /// [`nvs_runtime::call`]'s own result, unchanged.
    pub fn call(self, ctx: &mut nvs_runtime::Ctx) -> Result<nvs_runtime::Value, i32> {
        nvs_runtime::call(self.0, ctx, &[])
    }
}

impl Unit {
    /// This unit's **entry script frame**, ready to run: the top-level
    /// statements of the program's first file, which is what an embedder
    /// enters.
    ///
    /// `None` if no such frame was compiled. For a program that went through
    /// the front end that means the label drifted rather than that the program
    /// had no top-level statements — every file gets a frame, and
    /// [`nvs_ir::lower::ENTRY_SCRIPT_LABEL`] is the one home of the string the
    /// producer and this consumer both spell.
    #[must_use]
    pub fn script(&self) -> Option<ScriptFn> {
        self.raw_function(nvs_ir::lower::ENTRY_SCRIPT_LABEL)
            .map(ScriptFn)
    }

    /// Whether this unit compiled a function called `name`.
    ///
    /// What a test asks when the claim is that a lowering *happened* — a
    /// property hook's `get` body, a namespaced method's class-qualified label
    /// — and nothing beyond it. It answers without handing back a pointer, so
    /// an existence check cannot quietly grow into a call that fills no
    /// receiver slot.
    #[must_use]
    pub fn has_function(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// The runtime descriptor this unit built for `label`, or `None` for a
    /// class the program never declared.
    ///
    /// Borrowed from the table the unit owns, so it is readable without
    /// `unsafe` — see `nvs_runtime::ClassTable::desc_of`. What compiled code
    /// holds is that descriptor's address, baked in at every site that
    /// allocates or tests an instance.
    #[must_use]
    pub fn class_desc(&self, label: &str) -> Option<&nvs_runtime::ClassDesc> {
        self.classes.desc_of(label)
    }

    /// The raw code pointer compiled under `name`, with **its whole ABI
    /// contract left to the caller**.
    ///
    /// The pointer alone is not enough to call a method with. Slot 0 of the
    /// argument array is the implicit receiver and the first declared parameter
    /// is at slot 1 ([`NvsFn`] owns the rule), so a caller that hands
    /// [`nvs_runtime::call`] the declared arguments alone reads one `Value`
    /// past the end of its own slice for every parameter, on every platform,
    /// with no diagnostic anywhere.
    ///
    /// Nothing in this workspace should reach for this except a measurement
    /// that has to build its slot array once, outside the loop it is timing.
    /// [`Unit`]'s own docs name the entry point for every other shape, and one
    /// of them fills the receiver slot for you.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "handing back a JIT-compiled code pointer as a callable is a \
                  transmute with no safe spelling; the function was declared \
                  with exactly `NvsFn`'s signature in `compile` and \
                  `finalize_definitions` has made its pages executable"
    )]
    pub fn raw_function(&self, name: &str) -> Option<NvsFn> {
        let code = *self.entries.get(name)?;
        Some(unsafe { std::mem::transmute::<*const u8, NvsFn>(code) })
    }

    /// A handle on the class a runtime helper's bare-message failure is
    /// promoted to — spec § 10's `RuntimeError`, which is what "the world said
    /// no" means.
    ///
    /// `None` only if the unit somehow declares no such class, which the
    /// seeded exception tree (`nvs_hir::errors`) makes impossible for a
    /// program that went through the front end.
    ///
    /// The returned handle shares ownership of the descriptor table, so it may
    /// safely outlive this `Unit` — see [`nvs_runtime::ErrorClass`].
    #[must_use]
    pub fn runtime_error_class(&self) -> Option<nvs_runtime::ErrorClass> {
        let id = self.classes.id_of("RuntimeError")?;
        Some(nvs_runtime::ErrorClass::new(
            std::sync::Arc::clone(&self.classes),
            id,
        ))
    }

    /// Constructs the class labelled `class` and calls its `method` on the
    /// fresh instance — `rule:testing/runner-is-strict`'s one test, run from outside compiled code.
    ///
    /// `None` when this unit declares no such class, which is an internal
    /// inconsistency for a runner whose roster (`nvs_types::ExprTypeTable::tests`)
    /// came out of the same compile — the label is the same string both tables
    /// are keyed by. Everything past that is
    /// [`nvs_runtime::construct_and_call`]'s, including which statuses mean the
    /// test failed.
    ///
    /// This is where the descriptor's liveness is *provable* rather than
    /// promised, which is why the entry point is here and not in `nvs-cli`:
    /// the table is owned by `self`, so the pointer is live for the borrow.
    ///
    /// # Errors
    ///
    /// [`nvs_runtime::construct_and_call`]'s status, with its message left on
    /// `ctx`.
    pub fn call_on_new_instance(
        &self,
        ctx: &mut nvs_runtime::Ctx,
        class: &str,
        method: &str,
        args: &[nvs_runtime::Value],
    ) -> Option<Result<(), i32>> {
        let desc = self.classes.desc(self.classes.id_of(class)?);
        #[expect(
            unsafe_code,
            reason = "the descriptor came out of the table this unit owns, so \
                      it is live for the whole of this borrow — and for the \
                      call, `ctx` holding a shared handle on the same table \
                      through `install_in`"
        )]
        Some(unsafe { nvs_runtime::construct_and_call(ctx, desc, method, args) })
    }

    /// Calls the `static` method `Class::method` from outside compiled code,
    /// with `args` in written order and nothing else — this fills the receiver
    /// slot [`Self::raw_function`] leaves to the caller.
    ///
    /// A `static` method's receiver is the **called class**, so slot 0 gets
    /// this unit's descriptor for `class` rather than a null: late static
    /// binding reads that slot, and a null there would answer `static::` with
    /// a wild pointer instead of the class the caller named. The descriptor is
    /// process-wide and immortal, so unlike an instance receiver it needs no
    /// reference taken for the call.
    ///
    /// `class` names the function to call **and** the class that reaches the
    /// receiver slot, which is one string because an inherited body is
    /// compiled once, under the class that declares it: a method `Derived`
    /// inherits is reached by naming `Base`, and then `static::` inside it
    /// answers `Base` too. A hand caller that needs a derived called-class
    /// wants the descriptor's own method table, which is
    /// [`Self::call_on_new_instance`]'s path.
    ///
    /// `None` when this unit declares no such class or compiled no such
    /// method, which is an internal inconsistency for a label that came out of
    /// the same compile.
    ///
    /// # Panics
    ///
    /// If `args` is not as long as the method's declared arity. That is the
    /// second half of the same defect the receiver slot is the first half of:
    /// a short slice leaves the callee reading slots this frame does not own,
    /// and nothing downstream can notice, because `nvs_runtime::call` is handed
    /// a pointer and never a length.
    ///
    /// A panic rather than the recorded fault
    /// [`nvs_runtime::construct_and_call`] answers the same condition with,
    /// and the difference is deliberate: that one serves a `nvs test` runner
    /// where a mismatched roster is a user program's inconsistency to report,
    /// while every caller of this one is Rust code in this workspace, for which
    /// a wrong count is a bug in the test rather than an outcome. A status
    /// would also be indistinguishable from one the callee itself raised —
    /// `tests/stack_limit.rs` asserts on exactly `Err(FATAL)` from a call made
    /// through here, and would go on passing.
    ///
    /// # Errors
    ///
    /// The status the call reported, with its message left on `ctx`.
    pub fn call_static(
        &self,
        ctx: &mut nvs_runtime::Ctx,
        class: &str,
        method: &str,
        args: &[nvs_runtime::Value],
    ) -> Option<Result<nvs_runtime::Value, i32>> {
        let label = format!("{class}::{method}");
        let target = self.raw_function(&label)?;
        // Both maps are filled per compiled function and `shapes` in the
        // earlier pass, so a hit in one is a hit in the other.
        let arity = self.shapes.get(&label)?.arity;
        assert_eq!(
            usize::try_from(arity).unwrap_or(usize::MAX),
            args.len(),
            "`{label}` declares {arity} parameter(s) and this call supplies {}",
            args.len()
        );
        let receiver =
            nvs_runtime::Value::class_desc(self.classes.desc(self.classes.id_of(class)?));
        let mut slots = Vec::with_capacity(args.len() + 1);
        slots.push(receiver);
        slots.extend_from_slice(args);
        Some(nvs_runtime::call(target, ctx, &slots))
    }

    /// Builds `rule:testing/fixtures`'s fixture `method` of `class` into `fixtures`,
    /// calling it with the values `needs` names — the ones the checker
    /// resolved its own parameters to, and therefore ones an earlier call
    /// already built.
    ///
    /// A fixture is `static` (`nvs_types::code::E_FIXTURE_METHOD_SHAPE`), so
    /// it is reached as the compiled function `Class::method` rather than
    /// through a descriptor's method table: there is no receiver, and a call
    /// through that table would put one in slot 0 and shift every argument
    /// past it.
    ///
    /// `None` when this unit compiled no such function, which is an internal
    /// inconsistency for a roster that came out of the same compile.
    ///
    /// # Errors
    ///
    /// [`nvs_runtime::Fixtures::build`]'s status, with its message left on
    /// `ctx`.
    pub fn build_fixture(
        &self,
        ctx: &mut nvs_runtime::Ctx,
        fixtures: &mut nvs_runtime::Fixtures,
        class: &str,
        method: &str,
        needs: &[String],
    ) -> Option<Result<(), i32>> {
        let target = self.raw_function(&format!("{class}::{method}"))?;
        let receiver =
            nvs_runtime::Value::class_desc(self.classes.desc(self.classes.id_of(class)?));
        Some(fixtures.build(ctx, method, target, receiver, needs))
    }

    /// Hands `ctx` this unit's class table. **Every embedder calls this before
    /// running any of the unit's code**, whether or not it cares about `catch`.
    ///
    /// Several obligations share the one call, and *Safety* below is the
    /// reason it is not optional:
    ///
    /// 1. *Behaviour.* A runtime helper's failure carries a message and a
    ///    § 10 class name; the installed class is the anchor that resolves
    ///    the name to a catchable object with a backtrace
    ///    ([`nvs_runtime::Ctx::set_runtime_error_class`]).
    /// 2. *Safety.* Compiled code bakes each descriptor's address in as a
    ///    constant (see [`Classes`]), so an exception object still sitting on
    ///    the context points into this table and nothing else keeps it alive.
    ///    The handle installed here shares ownership of the table, which is
    ///    what makes a `Ctx` safe to outlive the `Unit` whose code it ran.
    ///    Skip the call and drop the `Unit` first, and `Ctx::pending` reads
    ///    freed memory — a use-after-free with no `unsafe` at the call site.
    /// 3. *State.* A `static` property's storage is the **request's**, not the
    ///    process's (`nvs_runtime::ctx`'s own docs), so arming it is part of
    ///    arming the context. Compiled code indexes that vector by a slot
    ///    number this unit fixed at compile time, which is why the unit hands
    ///    it over rather than an embedder building one. The list is **shared**
    ///    with the context, not copied into it, so a context can afterwards arm
    ///    a child against this unit's numbering with no `Unit` in hand — which
    ///    is what an `rule:security/isolate-shares-nothing` method entry is
    ///    (`nvs_runtime::Ctx::method_isolate`, and `nvs_runtime::script`'s
    ///    module doc for why that needs no resolver).
    pub fn install_in(&self, ctx: &mut nvs_runtime::Ctx) {
        if let Some(class) = self.runtime_error_class() {
            ctx.set_runtime_error_class(class);
        }
        ctx.install_statics(std::sync::Arc::clone(&self.statics));
    }
}

/// Compiles every function in `program` to native code.
///
/// # Errors
///
/// [`CodegenError::Unsupported`] for an IR shape this slice does not lower
/// (crate docs, scope list), [`CodegenError::Cranelift`] if Cranelift rejects
/// what was generated — always a bug in this crate, never in the input — and
/// [`CodegenError::UnsupportedHost`] on a machine Cranelift has no backend
/// for.
pub fn compile(program: &Program) -> Result<Unit, CodegenError> {
    let mut unit = UnitBuilder::new(None)?;
    unit.compile_all(program)?;
    unit.finish()
}

/// Compiles every function in `program` into a relocatable object file, and
/// returns its bytes — `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`
/// 's cached payload.
///
/// **The same walk [`compile`] runs**, and that is the point rather than an
/// implementation detail: `emit.rs` is handed a different [`Module`] and
/// nothing else, so the two products cannot drift into two semantics. What
/// differs is only what a `Module` decides — the JIT resolves a call, a runtime
/// helper and a class descriptor to an address in its own process, while the
/// object leaves each of them an undefined symbol for whoever loads the file to
/// resolve.
///
/// The bytes are the host's own object format (COFF on Windows, ELF elsewhere),
/// for the host's ISA: an artifact is addressed by a key that already covers
/// the target, so a file that would not load here is a cache miss rather than
/// anything this function has to refuse.
///
/// # Errors
///
/// The same cases [`compile`] reports, plus a refusal from the object writer
/// itself, which is an engine bug.
pub fn compile_object(program: &Program) -> Result<Vec<u8>, CodegenError> {
    let mut unit = UnitBuilder::for_object()?;
    unit.compile_all(program)?;
    unit.finish_object()
}

/// Compiles every function in `program` and returns the generated machine
/// code as text, one section per function, *instead* of a callable [`Unit`].
///
/// This is what `nvs run --dump-asm` prints. It compiles through exactly the
/// same path [`compile`] does — same ISA flags, same emitted probes — with
/// Cranelift's disassembler switched on, so what it prints is the code that
/// would have run rather than a second, differently-configured rendering.
/// Nothing is executed.
///
/// # Errors
///
/// The same cases [`compile`] reports, for the same reasons.
pub fn disassemble(program: &Program) -> Result<String, CodegenError> {
    let mut unit = UnitBuilder::new(Some(String::new()))?;
    unit.compile_all(program)?;
    // `finish` still has to run: `finalize_definitions` is what resolves the
    // relocations, and a unit that cannot be linked is not a unit whose
    // disassembly should be reported as if it were fine.
    let disasm = unit.disasm.take().unwrap_or_default();
    unit.finish()?;
    Ok(disasm)
}

/// Compiles every function in `program` and returns the **Cranelift IR** this
/// crate emitted for it as text, one section per function, *instead* of a
/// callable [`Unit`].
///
/// [`disassemble`]'s sibling one level up, and the one a test asserting on the
/// *shape* of the emitted code wants: where a load sits is a fact about what
/// was emitted, while which instruction it became is a fact about the machine
/// that ran the test. Taken before the backend compiles the function, so what
/// it prints is what this crate wrote rather than what Cranelift made of it.
/// Nothing is executed.
///
/// # Errors
///
/// The same cases [`compile`] reports, for the same reasons.
pub fn clif(program: &Program) -> Result<String, CodegenError> {
    let mut unit = UnitBuilder::new(None)?;
    unit.clif = Some(String::new());
    unit.compile_all(program)?;
    let clif = unit.clif.take().unwrap_or_default();
    unit.finish()?;
    Ok(clif)
}

/// Every class descriptor a unit declares, built from the lowered IR and from
/// nothing else — `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`
/// 's answer to where a warm cache hit's descriptors come from.
///
/// A cached payload leaves every `nvs_class_desc_*` undefined (§ 2) and its
/// loader resolves one against "the `ClassDesc` this process allocated" (§ 3) —
/// but a run that skipped codegen allocated none, and no payload carries one.
/// § 2 closes that the only way that needs neither a format change nor a
/// serialized runtime type: **what a warm hit skips is codegen, not the front
/// end**, so the `nvs_ir::Program` the front end has just lowered is walked for
/// its classes exactly as [`compile`] walks it, and what falls out are the same
/// descriptors, in the same order, that a cold compile of the same source would
/// have allocated. A hit already means "the same source and the same
/// toolchain" — the cache key covers both — so that is an identity rather than
/// a hope.
///
/// A method table is deliberately **not** built here. A
/// [`nvs_runtime::MethodRow`] holds a compiled function's address and there is
/// none until the payload has been placed, so § 3's order is build, place,
/// relocate, protect, then bind — this type is the first of those steps and
/// [`Self::bind`] is the last, the loader's counterpart to
/// [`UnitBuilder::bind_method_tables`].
///
/// **Costs** one `ClassDesc` per class the unit declares, one symbol name per
/// class, and one symbol name plus a shape per function it declares, for as
/// long as the caller holds this — the same allocation the JIT path already
/// makes at the same scale, and freed with the table.
#[derive(Debug)]
pub struct Descriptors {
    /// Held for the descriptors themselves, exactly as [`Unit`] holds this same
    /// table: an address handed out by [`Self::resolve`] is written into machine
    /// code that will dereference it, so the table has to outlive every frame
    /// that code can enter. [`Self::bind`] is what writes into it, and the
    /// wiring that turns placed pages into a [`Unit`] is what reads it next.
    classes: Classes,
    /// Every function the program declares, by its Novis label, to the symbol
    /// [`function_symbol`] named it and the shape a
    /// [`nvs_runtime::MethodRow`] carries. Both halves are read once, by
    /// [`Self::bind`], and both are recorded in the one walk that saw the
    /// function — so a row's address and its arity cannot come to describe two
    /// different callees, exactly as `UnitBuilder::shapes` guarantees on the
    /// JIT path.
    functions: FxHashMap<String, (String, MethodShape)>,
    /// The program's static-property initializers, in the slot order
    /// `nvs_ir::ir::Program::statics` carries and the payload's code baked in —
    /// [`Unit::statics`]'s own contents, read off the IR here for the reason
    /// every other field is: the loading process lowered the same source.
    statics: std::sync::Arc<[Option<nvs_runtime::FieldDefault>]>,
    /// [`class_desc_symbol`]'s name for each descriptor, to the address a
    /// relocation against it resolves to. Built once here rather than searched
    /// per relocation: a loader asks this for every undefined symbol in the
    /// payload, which is once per class *per referring section*.
    ///
    /// [`core_desc_symbols`]'s rows sit in here beside the unit's own, because
    /// a payload that folded a markup literal left that name undefined too and
    /// a loader resolves every undefined symbol out of one table.
    by_symbol: FxHashMap<String, *const u8>,
}

impl Descriptors {
    /// Builds every descriptor `program` declares, parents first.
    #[must_use]
    pub fn of(program: &Program) -> Self {
        let classes = Classes::build(&program.classes, &program.shape_codecs);
        let by_symbol = classes
            .descriptors()
            .map(|(label, desc)| (class_desc_symbol(label), desc.cast::<u8>()))
            // The unit's wire contracts are relocated against exactly as its
            // descriptors are, so a loader resolves both out of one table.
            .chain(
                classes
                    .shape_codecs()
                    .map(|(key, codec)| (shape_codec_symbol(key), codec.cast::<u8>())),
            )
            .chain(core_desc_symbols())
            .collect();
        let functions = program
            .functions
            .iter()
            .enumerate()
            .map(|(index, function)| {
                (
                    function.name.clone(),
                    (
                        function_symbol(index, &function.name),
                        MethodShape::of(&function.params),
                    ),
                )
            })
            .collect();
        Self {
            classes,
            by_symbol,
            functions,
            statics: program
                .statics
                .iter()
                .map(|prop| prop.default_value.clone())
                .collect(),
        }
    }

    /// The address `symbol` names, or [`None`] if it is neither a descriptor of
    /// a class this unit declares nor one of the `Core` descriptors
    /// [`core_desc_symbols`] publishes — which `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` makes a cache miss on the
    /// footing of a wrong `env_hash`, never an error.
    ///
    /// The spelling is asked of [`class_desc_symbol`] rather than matched here,
    /// so the emitting end and the resolving end of a relocation cannot drift
    /// apart.
    #[must_use]
    pub fn resolve(&self, symbol: &str) -> Option<*const u8> {
        self.by_symbol.get(symbol).copied()
    }

    /// § 3's last step: hands every descriptor built here the addresses the
    /// *loader* placed this unit's methods at, so a `CallVirtual` reaching one
    /// of these classes dispatches to the payload's own code.
    ///
    /// `code` is the placed payload, read by symbol name — the loader's
    /// counterpart to the `Module::get_finalized_function` that fills the same
    /// rows on the JIT path ([`UnitBuilder::bind_method_tables`]). The name it
    /// is asked for is derived here rather than by the loader, because
    /// [`function_symbol`]'s index is the position in the `nvs_ir::Program`
    /// this table was built from and only this end holds it.
    ///
    /// Runs **after** the pages are executable, and that is not a race: a row
    /// is written into a descriptor this process owns, never back into the
    /// mapping, which is why binding can follow `mprotect` rather than needing
    /// a writable page. A `(method, declaring class)` pair the payload does not
    /// define is skipped rather than being an error, on exactly the terms
    /// [`UnitBuilder::bind_method_tables`] skips one — the front end has already
    /// reported whatever left it behind, and `nvs_class_method`'s fallback keeps
    /// the call correct regardless.
    pub fn bind(&mut self, code: &dyn Placed) {
        let functions = &self.functions;
        for entry in self.classes.by_label.values() {
            let methods = entry
                .methods
                .iter()
                .filter_map(|(method, declaring, public)| {
                    let label = format!("{declaring}::{method}");
                    let (symbol, shape) = functions.get(&label)?;
                    Some(nvs_runtime::MethodRow {
                        name: method.clone(),
                        code: code.address_of(symbol)?,
                        arity: shape.arity,
                        param_tags: shape.param_tags,
                        public: *public,
                        // Every row here is a compiled Novis function, for the
                        // reason its JIT counterpart gives.
                        native: false,
                    })
                })
                .collect();
            self.classes.table.set_methods(entry.id, methods);
            // The accessors, joined the same way and skipped on the same
            // terms. No `{declaring}::{method}` is assembled here: a hook's
            // label *is* its symbol's name, spelled once by
            // `nvs_types::signatures::hook_label`.
            let hooks = entry
                .hooks
                .iter()
                .filter_map(|(property, label, set)| {
                    let (symbol, shape) = functions.get(label)?;
                    Some(nvs_runtime::HookRow {
                        property: property.clone(),
                        set: *set,
                        code: code.address_of(symbol)?,
                        param_tags: shape.param_tags,
                    })
                })
                .collect();
            self.classes.table.set_hooks(entry.id, hooks);
        }
    }

    /// The end of `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`: these descriptors and `code`'s placed pages,
    /// assembled into the same [`Unit`] a cold compile of the same program
    /// would have produced.
    ///
    /// Every field a [`Unit`] carries is either something this table already
    /// holds or something [`Self::of`]'s walk over the IR already read — an
    /// address per function, from `code`; a shape per function, the class table
    /// and the static-property defaults, from here. Nothing is asked of the
    /// caller twice, which is what stops a unit being assembled against a
    /// *different* program than its descriptors were built from. So both paths
    /// hand their caller one type with one set of guarantees, and nothing above
    /// this line has to know which one ran.
    ///
    /// [`Self::bind`] runs first, here rather than at the caller: a unit whose
    /// method tables were left empty answers a `CallVirtual` through its
    /// fallback, which is a wrong answer rather than a failure, and no caller
    /// should be able to reach it by forgetting a call.
    #[must_use]
    pub fn into_unit(mut self, code: Box<dyn Placed>) -> Unit {
        self.bind(code.as_ref());
        let entries = self
            .functions
            .iter()
            .filter_map(|(label, (symbol, _))| Some((label.clone(), code.address_of(symbol)?)))
            .collect();
        let shapes = self
            .functions
            .iter()
            .map(|(label, (_, shape))| (label.clone(), *shape))
            .collect();
        Unit {
            _code: Code::Placed(code),
            classes: std::sync::Arc::new(self.classes.table),
            entries,
            shapes,
            statics: self.statics,
        }
    }
}

/// The compilation unit under construction: whichever [`Module`] will finalize
/// it, plus the tables every emitted function shares.
///
/// `M` is a [`JITModule`] for [`compile`] and an [`ObjectModule`] for
/// [`compile_object`], and the whole of the difference between the two products
/// is in that one parameter — [`Self::compile_all`], [`Self::compile_function`]
/// and all of `emit.rs` are the same code either way, which is the standing
/// decision that an object backend is a second `Module` and never a second
/// lowering. What each backend *finishes* with is its own: only
/// `UnitBuilder<JITModule>` has a `finish`, because only a JIT has an address
/// to hand back.
struct UnitBuilder<M> {
    module: M,
    ctx: codegen::Context,
    fn_ctx: FunctionBuilderContext,
    sigs: Signatures,
    /// Every function this unit defines, by its Novis name — the table
    /// `nvs_ir::ir::InstKind::Call`'s `"Class::method"` target is resolved
    /// through. Filled in a declaration pass over the whole program before
    /// any body is emitted, so a call may name a function defined later in
    /// the unit (or itself).
    functions: FxHashMap<String, cranelift_module::FuncId>,
    /// Every function's *declared shape*, by the same Novis name — its arity
    /// and its parameter-tag word, read off `nvs_ir::ir::Function::params` in
    /// the same declaration pass that fills [`UnitBuilder::functions`].
    ///
    /// Recorded for every function and read for the methods alone: a
    /// `nvs_runtime::MethodRow` carries the shape a caller holding only tagged
    /// values needs, and this is the one place that fact is still in hand —
    /// [`UnitBuilder::bind_method_tables`] runs after finalization, where a function is
    /// an address and nothing else.
    shapes: FxHashMap<String, MethodShape>,
    /// Every class the unit declares — the descriptors compiled code points
    /// at, and the field-slot index every `FieldGet`/`FieldSet` resolves
    /// through.
    classes: Classes,
    /// Every `static` property the unit declares, mapped from the
    /// `(declaring class, name)` pair `nvs_ir::ir::InstKind::StaticGet` names
    /// to its slot number — the static-storage counterpart of
    /// [`ClassEntry::slots`], and resolved exactly the same way: once, before
    /// any body is emitted, so the machine sees a constant index.
    statics: FxHashMap<(String, String), u32>,
    /// The same table's initializers, in slot order — see [`Unit::statics`].
    static_defaults: Vec<Option<nvs_runtime::FieldDefault>>,
    /// Every class descriptor's address, under the symbol name compiled code
    /// relocates against — [`class_desc_symbol`]'s spelling, and the third
    /// symbol table [`UnitBuilder::new`] gives the module.
    ///
    /// Shared with the closure that reads it, because the two happen at
    /// opposite ends of a compile: `JITBuilder::symbol` takes an address
    /// *now* and the descriptors do not exist until [`UnitBuilder::compile_all`]
    /// builds them, while a `symbol_lookup_fn` is not called until
    /// `finalize_definitions` relocates. The `Mutex` is what makes that
    /// closure `Send`, which `cranelift-jit` requires; it is uncontended.
    desc_symbols: Arc<Mutex<FxHashMap<String, usize>>>,
    /// One entry per emitted `ConstStr`, so data-object names stay unique.
    literals: usize,
    entries: Vec<(String, cranelift_module::FuncId)>,
    /// Set only by [`disassemble`]: the accumulated text of every function's
    /// generated code. `None` is the ordinary compile, which asks Cranelift
    /// for no disassembly at all and so pays nothing for this field.
    disasm: Option<String>,
    /// Set only by [`clif`]: the accumulated Cranelift IR of every function, as
    /// this crate emitted it. `None` is the ordinary compile, which renders
    /// nothing and so pays nothing for this field.
    clif: Option<String>,
}

/// Every class the compiled unit declares, in the two forms emitted code
/// needs: the `ClassDesc` address `nvs_object_new`/`nvs_object_instanceof`
/// take, and the field-slot index a `FieldGet`/`FieldSet` turns into an
/// offset through [`nvs_runtime::field_offset`].
///
/// # Why the descriptor address is a relocation rather than a constant
///
/// A JIT compiles at run time, so it *knows* the address of a runtime object
/// it has already built, and could bake that address in as an `iconst`. It
/// does not. The address reaches the code as a
/// relocation against the name [`class_desc_symbol`] mints, because `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`
/// 's payload is this same lowering walk emitted into an object file, and a
/// host address written into an object file is wrong the moment another
/// process reads it — the descriptors it names were allocated by the process
/// that compiled, not by the one that will run.
///
/// **Nothing on the hot path pays for it.** `is_pic` is off (see [`UnitBuilder::new`]), so
/// a symbol value lowers to the same absolute `movabs` an `iconst` would, with
/// an `Abs8` relocation attached; under [`JITModule`] that relocation resolves
/// through the lookup closure [`UnitBuilder::new`] installs, to the very address this
/// table holds. The descriptor stays an opaque token — `nvs_runtime::ClassDesc`
/// needs no `#[repr(C)]` and no layout compiled code agrees on — and all the
/// relocation adds is a *record* of where the address came from.
///
/// The [`Unit`] that owns the table must outlive that code — see its own
/// `_classes` field.
#[derive(Debug, Default)]
struct Classes {
    table: nvs_runtime::ClassTable,
    by_label: FxHashMap<String, ClassEntry>,
    /// The same keys as `by_label`, holding the table id `ClassTable::define`
    /// needs for a parent. Separate because a descriptor address is what
    /// *compiled code* wants and an id is what the table wants.
    ids: FxHashMap<String, nvs_runtime::ClassId>,
    /// One `nvs_runtime::ShapeCodec` per `nvs_ir::ir::Program::shape_codecs`
    /// entry, under the key that entry carries — what
    /// `nvs_ir::ir::InstKind::ShapeCodecConst` names and
    /// [`shape_codec_symbol`] mints a relocation for. A shape's contract is a
    /// table of its own rather than a field of `by_label`'s entry because one
    /// shape class answers for every field *type*; `nvs-ir`'s module docs own
    /// that choice.
    shape_codecs: FxHashMap<String, *const nvs_runtime::ShapeCodec>,
}

/// One compiled function's declared shape, as
/// `nvs_runtime::MethodRow` carries it: how many parameters it takes and which
/// runtime tag each one requires.
///
/// The receiver is subtracted here and nowhere else. `nvs_ir::ir::Function`'s
/// parameter 0 is the implicit `$this` (that field's own doc comment), while a
/// row describes what a *call site* writes — so a caller compares its argument
/// count against this and fills the callee's slots from 1.
#[derive(Clone, Copy, Debug, Default)]
struct MethodShape {
    arity: u32,
    param_tags: u64,
}

impl MethodShape {
    /// `params` as a row, with parameter 0 read as the receiver.
    ///
    /// A function with no parameters at all is a script frame or a synthesized
    /// factory rather than a method — it has no receiver to subtract, and no
    /// method table names it either, so a zero arity is the answer both ways.
    fn of(params: &[nvs_ir::Ty]) -> Self {
        let declared = params.split_first().map_or(&[][..], |(_, rest)| rest);
        Self {
            arity: u32::try_from(declared.len()).unwrap_or(u32::MAX),
            param_tags: nvs_ir::lower::pack_param_tags(declared.iter().copied()),
        }
    }
}

/// One class's compiled-in identity and field-slot map.
#[derive(Debug)]
struct ClassEntry {
    /// Address baked into the code that allocates or tests an instance.
    desc: *const nvs_runtime::ClassDesc,
    /// Field name to slot index. Built from the *flattened* order
    /// `nvs_ir::ir::Class::fields` carries, so a slot looked up through the
    /// declaring class is valid for every subclass.
    slots: FxHashMap<String, usize>,
    /// This class's own id in `Classes::table`, and every method it answers as
    /// `(method name, declaring class label, is `public`)` — kept until
    /// [`UnitBuilder::finish`], which is the first moment a compiled function has an
    /// address to put in the runtime descriptor's method table. See
    /// `nvs_runtime::ClassTable::set_methods`.
    id: nvs_runtime::ClassId,
    methods: Vec<(String, String, bool)>,
    /// Every property hook it answers as `(property name, hook label, is the
    /// `set` accessor)`, kept for [`Self::methods`]' reason and spent at the
    /// same moment — see `nvs_runtime::ClassTable::set_hooks`. The label is
    /// already the one the hook's function is emitted under, so this half
    /// needs no `{declaring}::{method}` join.
    hooks: Vec<(String, String, bool)>,
    /// `nvs_ir::ir::Class::conforms` verbatim — this class's *transitive*
    /// supertype set, kept because [`Classes::conforming_to`] needs the
    /// hierarchy read the other way round and this crate may not deref a
    /// descriptor to ask (`#![deny(unsafe_code)]`).
    ///
    /// **Costs** one `String` per edge per class, for the life of the compiled
    /// unit — never per request, and freed with the unit.
    conforms: Vec<String>,
}

impl Classes {
    /// Builds every descriptor, parents first.
    ///
    /// `nvs_ir::ir::Class::conforms` is already the *transitive* supertype
    /// set, so a class can be defined as soon as every label in it is —
    /// [`Self::define`] recurses to arrange exactly that. A `conforms` entry
    /// naming a class the unit does not declare is skipped rather than being
    /// an error: an `extends` the front end already diagnosed leaves one
    /// behind, and a second unexplained failure here would only bury the
    /// first.
    fn build(classes: &[nvs_ir::ir::Class], shapes: &[nvs_ir::ir::ShapeCodec]) -> Self {
        let mut out = Self::default();
        let by_label: FxHashMap<&str, &nvs_ir::ir::Class> = classes
            .iter()
            .map(|class| (class.label.as_str(), class))
            .collect();
        for class in classes {
            out.define(class, &by_label);
        }
        // `rule:core-classes/derive-field-list`'s nested field names a class that may be defined after
        // the one holding it — or be that same class, since § 2 makes
        // recursion the data's problem rather than the table's — so the codec
        // is joined only once every descriptor above exists, and the contracts
        // go ahead of that join: a codec field naming an inline shape resolves
        // to a table that has to exist before the join reads it, where one
        // naming a class resolves to a descriptor the pass above already wrote.
        // A written shape has no declaration to define it from, which is why it
        // is a pass of its own at all.
        out.define_shape_codecs(shapes);
        out.link_codecs(classes);
        out
    }

    /// Fills in every class's `rule:core-classes/derive-attribute` codecs — the JSON one and the row one
    /// alike, each with its nested fields' descriptors resolved — the pass
    /// [`nvs_runtime::ClassTable::set_codec`]'s docs describe.
    ///
    /// A nested label this unit does not define leaves a null, which
    /// `nvs_stdlib::json` reports as the internal error it is: the checker
    /// refused an unreachable field type long before here
    /// (`nvs_types::derive`'s `resolve_field_types`), so a miss is this join
    /// disagreeing with itself rather than anything a program wrote.
    fn link_codecs(&mut self, classes: &[nvs_ir::ir::Class]) {
        for class in classes {
            let Some(id) = self.ids.get(&class.label).copied() else {
                continue;
            };
            if !class.codec.is_empty() {
                let nested = self.nested_descs(&class.codec);
                let shapes = self.nested_shapes(&class.codec);
                self.table
                    .set_codec(id, class.codec.clone(), class.ctor_arity, nested, shapes);
            }
            // The row half, on the same terms: a class carrying both
            // attributes has two field lists and fills both, and one carrying
            // only `#[Db\Derive]` reaches `set_db_codec` alone — which is why
            // that one writes the constructor arity too.
            if !class.db_codec.is_empty() {
                let nested = self.nested_descs(&class.db_codec);
                self.table
                    .set_db_codec(id, class.db_codec.clone(), class.ctor_arity, nested);
            }
        }
    }

    /// Hands the class table every inline shape's wire contract, keyed the way
    /// the call site that wrote it names one — the shape half of
    /// [`Self::link_codecs`], run in the same second pass because a shape field
    /// naming a class resolves through [`Self::nested_descs`] exactly as a
    /// class's field does.
    ///
    /// **What it spends**, per `rule:programs/memory-priority`: one table per
    /// distinct shape written in the program, owned by the unit's class table
    /// for that unit's life — O(distinct types), never O(requests served).
    fn define_shape_codecs(&mut self, shapes: &[nvs_ir::ir::ShapeCodec]) {
        let by_key: FxHashMap<&str, &nvs_ir::ir::ShapeCodec> = shapes
            .iter()
            .map(|shape| (shape.key.as_str(), shape))
            .collect();
        for shape in shapes {
            self.define_shape_codec(shape, &by_key);
        }
    }

    /// One shape's table, with every shape its own fields reach defined first
    /// so the pointers handed to
    /// [`nvs_runtime::ClassTable::define_shape_codec`] are already resolved.
    ///
    /// The recursion terminates on the type: a shape's field types are written
    /// out in full where the shape is, so no contract reaches itself. An entry
    /// already defined is handed back rather than defined twice, which is what
    /// keeps `{a: {n: int}, b: {n: int}}` one nested table and not two.
    fn define_shape_codec(
        &mut self,
        shape: &nvs_ir::ir::ShapeCodec,
        by_key: &FxHashMap<&str, &nvs_ir::ir::ShapeCodec>,
    ) -> *const nvs_runtime::ShapeCodec {
        if let Some(codec) = self.shape_codecs.get(&shape.key) {
            return *codec;
        }
        let mut nested_shapes: Vec<*const nvs_runtime::ShapeCodec> =
            Vec::with_capacity(shape.fields.len());
        for field in &shape.fields {
            let inner = field
                .shape
                .as_deref()
                .and_then(|key| by_key.get(key).copied());
            nested_shapes.push(match inner {
                Some(inner) => self.define_shape_codec(inner, by_key),
                None => std::ptr::null(),
            });
        }
        let nested = self.nested_descs(&shape.fields);
        let codec = self
            .table
            .define_shape_codec(shape.fields.clone(), nested, nested_shapes);
        self.shape_codecs.insert(shape.key.clone(), codec);
        codec
    }

    /// One contract per field of `codec`, null except where the field names an
    /// inline shape — [`Self::nested_descs`]' twin for the second pointer a
    /// `nvs_runtime::CodecTy::Shape` field decodes through, and answerable only
    /// because [`Self::define_shape_codecs`] has already run.
    fn nested_shapes(
        &self,
        codec: &[nvs_runtime::CodecField],
    ) -> Vec<*const nvs_runtime::ShapeCodec> {
        codec
            .iter()
            .map(|field| {
                field
                    .shape
                    .as_deref()
                    .and_then(|key| self.shape_codecs.get(key).copied())
                    .unwrap_or(std::ptr::null())
            })
            .collect()
    }

    /// Every wire contract this unit defines, as `(key, address)` — what
    /// [`UnitBuilder::compile_all`] publishes under [`shape_codec_symbol`]'s
    /// names so a relocation against one of them resolves, on
    /// [`Self::descriptors`]' terms exactly.
    fn shape_codecs(&self) -> impl Iterator<Item = (&str, *const nvs_runtime::ShapeCodec)> {
        self.shape_codecs
            .iter()
            .map(|(key, codec)| (key.as_str(), *codec))
    }

    /// One descriptor per field of `codec`, null except where the field names
    /// a class — the resolved half of [`nvs_runtime::CodecField::class`], and
    /// the reason [`Self::link_codecs`] is a second pass.
    fn nested_descs(
        &self,
        codec: &[nvs_runtime::CodecField],
    ) -> Vec<*const nvs_runtime::ClassDesc> {
        codec
            .iter()
            .map(|field| {
                field
                    .class
                    .as_deref()
                    .and_then(|label| self.ids.get(label).copied())
                    .map_or(std::ptr::null(), |nested| self.table.desc(nested))
            })
            .collect()
    }

    fn define(&mut self, class: &nvs_ir::ir::Class, source: &FxHashMap<&str, &nvs_ir::ir::Class>) {
        if self.by_label.contains_key(&class.label) {
            return;
        }
        // Reserve the label before recursing: a cyclic `extends` has already
        // been diagnosed by `nvs_hir::hierarchy`, and this must terminate
        // rather than re-report it.
        let mut parents = Vec::with_capacity(class.conforms.len());
        for label in &class.conforms {
            let Some(parent) = source.get(label.as_str()) else {
                continue;
            };
            if !self.by_label.contains_key(label) {
                self.define(parent, source);
            }
            if let Some(id) = self.ids.get(label) {
                parents.push(*id);
            }
        }
        let id = self.table.define(&class.label, &class.fields, &parents);
        if !class.defaults.is_empty() {
            self.table.set_defaults(id, class.defaults.clone());
        }
        // `rule:types/erased-member-access`'s write check, at the one granularity the runtime can
        // hold: a representation with no single tag — `Ty::Tagged`, `Ty::Void`
        // — becomes `None`, which `nvs_runtime::nvs_object_slot_set` reads as
        // "unchecked". Every class with a layout carries one entry per slot,
        // because § 4's erased receiver reaches any class at all; the guard
        // below is for the synthesized ones that carry none (a closure's
        // environment, a generator's state).
        if class.field_reprs.len() == class.fields.len() && !class.field_reprs.is_empty() {
            let tags = class
                .field_reprs
                .iter()
                .map(|ty| crate::ty::tag_of(*ty).ok())
                .collect();
            self.table.set_field_tags(id, tags);
        }
        // `rule:errors/record-transformations`'s redaction row, at the one granularity a dump can ask:
        // the bit rides down untouched, `nvs_types` having decided it where
        // the qualifier still exists. Guarded on the same length agreement,
        // for the same synthesized classes.
        if class.secret_fields.len() == class.fields.len() && !class.secret_fields.is_empty() {
            self.table
                .set_secret_fields(id, class.secret_fields.clone());
        }
        // `rule:security/reflection-enforces-visibility`'s visibility bit, at the one granularity a reflective
        // read can ask: the slot it is about to open. Guarded on the same length
        // agreement, for the same synthesized classes.
        if class.public_fields.len() == class.fields.len() && !class.public_fields.is_empty() {
            self.table
                .set_public_fields(id, class.public_fields.clone());
        }
        let slots = class
            .fields
            .iter()
            .enumerate()
            .map(|(index, name)| (name.clone(), index))
            .collect();
        self.ids.insert(class.label.clone(), id);
        self.by_label.insert(
            class.label.clone(),
            ClassEntry {
                desc: self.table.desc(id),
                slots,
                id,
                methods: class.methods.clone(),
                hooks: class.hooks.clone(),
                conforms: class.conforms.clone(),
            },
        );
    }

    /// Every class this unit declares that **is a** `base`, as
    /// `(label, descriptor)` — `base` itself included, since
    /// `rule:types/class-reference`'s rows admit `T` as readily as a class that is a `T`.
    ///
    /// This is the closed set `nvs_ir::ir::InstKind::ClassDescIn` is compiled
    /// against, and it is answered here rather than in the runtime because the
    /// answer is already sitting in this table: a name lookup at run time would
    /// need a per-unit registry and a relocation at every site to reach it.
    ///
    /// **Sorted by label**, because the result is baked into machine code and
    /// two builds of one unit have to emit the same instructions for
    /// `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`'s
    /// checksum to mean what it claims.
    fn conforming_to(&self, base: &str) -> Vec<(&str, *const nvs_runtime::ClassDesc)> {
        let mut out: Vec<(&str, *const nvs_runtime::ClassDesc)> = self
            .by_label
            .iter()
            .filter(|(label, entry)| {
                label.as_str() == base || entry.conforms.iter().any(|up| up == base)
            })
            .map(|(label, entry)| (label.as_str(), entry.desc))
            .collect();
        out.sort_unstable_by_key(|(label, _)| *label);
        out
    }

    /// Every class this unit declares, as `(label, descriptor address)` — what
    /// [`UnitBuilder::compile_all`] publishes under [`class_desc_symbol`]'s names so a
    /// relocation against one of them resolves.
    fn descriptors(&self) -> impl Iterator<Item = (&str, *const nvs_runtime::ClassDesc)> {
        self.by_label
            .iter()
            .map(|(label, entry)| (label.as_str(), entry.desc))
    }

    /// The descriptor address for `label`, or `None` if the unit declares no
    /// such class.
    fn desc(&self, label: &str) -> Option<*const nvs_runtime::ClassDesc> {
        self.by_label.get(label).map(|entry| entry.desc)
    }

    /// Whether this unit defines the wire contract `key` names — [`Self::desc`]
    /// for the table beside the descriptor, and asked for the same reason: a
    /// body may not relocate against a symbol nothing will publish.
    fn defines_shape_codec(&self, key: &str) -> bool {
        self.shape_codecs.contains_key(key)
    }

    /// The slot `class::field` occupies, or `None` if either is unknown.
    fn slot(&self, class: &str, field: &str) -> Option<usize> {
        self.by_label
            .get(class)
            .and_then(|entry| entry.slots.get(field))
            .copied()
    }
}

/// The signatures the runtime exports, beyond the helper ABI itself.
///
/// `nvs-runtime`'s entry points are deliberately not all the same shape:
/// `nvs_str_concat`/`nvs_str_concat_n`/`nvs_str_retain`/`nvs_str_release`
/// operate on
/// raw `StrHeader` pointers with no context and no `Value`, because they are
/// memory primitives rather than language operations, and `nvs_probe_stmt`
/// returns nothing because a coverage probe cannot fail. Each therefore gets
/// its own signature here rather than being forced through
/// [`Signatures::helper`].
struct Signatures {
    /// `rule:errors/propagation`'s calling convention — `(ctx, args, out) -> status`.
    helper: Signature,
    /// `(ctx, args, argc, out) -> status` — [`Self::helper`] with the
    /// argument **count** passed beside the slot, for the two helpers whose
    /// arity belongs to the call site rather than to their own declaration:
    /// `nvs_runtime::nvs_call_closure` and `nvs_call_closure_proven`, which are
    /// `nvs_ir::Helper::CallClosure` and `CallClosureProven` — the dynamic and
    /// the checked spellings of `rule:types/closure-literal`'s `$fn(...)`.
    /// Every other helper's arity is a literal in its `nvs_helper!` expansion,
    /// so no count crosses the boundary at all.
    helper_variadic: Signature,
    /// `nvs_safepoint(ctx) -> status`.
    safepoint: Signature,
    /// `nvs_stack_check(ctx, sp) -> status` —
    /// `rule:errors/on-limit`'s
    /// slow path. `sp` is `I64` for the reason every other pointer-shaped
    /// parameter here is: this JIT compiles for 64-bit targets only.
    stack_check: Signature,
    /// `nvs_probe_stmt(ctx, stmt_id)`.
    probe: Signature,
    /// `nvs_probe_call_enter(ctx, name, len)`.
    probe_call: Signature,
    /// `nvs_probe_call_exit(ctx, name, len, status)`.
    probe_call_exit: Signature,
    /// `nvs_str_concat(lhs, rhs) -> *mut StrHeader`,
    /// `nvs_str_append(target, suffix) -> *mut StrHeader` and
    /// `nvs_str_concat_n(pieces, count) -> *mut StrHeader`, which are all the
    /// same shape: two pointer-width parameters, one pointer back. They
    /// differ in ownership and in what the second parameter *means*, not in
    /// ABI — see `nvs_ir::ir::InstKind::StrAppend` and `InstKind::Concat` — so
    /// one signature serves all of them, and a count declares itself with
    /// `AbiParam::new(ptr)` because a `usize` is pointer-width.
    str_concat: Signature,
    /// `nvs_str_eq(lhs, rhs) -> bool` and `nvs_array_eq(lhs, rhs) -> bool`,
    /// which share one shape: two raw pointers to an `I8`, like
    /// `Sigs::instanceof`.
    ptr_eq: Signature,
    /// `nvs_float_pow(base, exponent) -> f64` — `rule:types/arithmetic`'s `**` over two
    /// `float`s, which has no machine instruction and no `LibCall` either. See
    /// [`crate::emit`]'s module doc for why it is a direct call of this shape
    /// rather than one more [`Signatures::helper`].
    float_pow: Signature,
    /// `nvs_str_retain(ptr)` / `nvs_str_release(ptr)`, and the two
    /// `nvs_throwable_*` counterparts.
    refcount: Signature,
    /// `nvs_value_retain(tag_word, bits)` / `nvs_value_release(tag_word, bits)`
    /// — the tag-dispatching pair a `nvs_ir::Ty::Tagged` operand needs, taking
    /// the register pair `crate::ty::clif_ty` describes as two words rather
    /// than one 16-byte aggregate, so no C ABI question about how such an
    /// aggregate travels ever arises.
    value_refcount: Signature,
    /// `nvs_exception_new(ptr) -> ptr`, and every other exception primitive
    /// with that one shape: `nvs_throwable_message`, `nvs_throwable_trace`,
    /// `nvs_take_thrown`.
    ptr_to_ptr: Signature,
    /// `nvs_raise(ctx, throwable, source)` — the third operand is the throw
    /// site's own carrier, or the zero word where the raise is no site of its
    /// own. See `nvs_runtime::nvs_raise`.
    raise: Signature,
    /// `nvs_raise_new(ctx, class, message, len)` — the throw compiled code
    /// raises by itself, with no Novis `new` behind it. See
    /// `nvs_runtime::nvs_raise_new`.
    raise_new: Signature,
    /// `nvs_object_instanceof(object, desc) -> bool` — `I8`, the width a
    /// Cranelift comparison produces and the one [`ty::clif_ty`] gives
    /// [`nvs_ir::Ty::Bool`].
    ///
    /// Shared with `nvs_value_instanceof(subject_ptr, desc) -> bool`, which
    /// `emit::Emitter::emit_instanceof` calls instead for a
    /// [`nvs_ir::Ty::Tagged`] subject: two pointer arguments and an `I8`
    /// result either way, only the first argument's pointee differing.
    instanceof: Signature,
    /// `nvs_class_method(class, name, len, fallback) -> code address` — the
    /// runtime half of `static::method(...)`'s dispatch. See
    /// `nvs_runtime::nvs_class_method`.
    class_method: Signature,
    /// `nvs_object_slot_get(ctx, receiver, name, len, hint, out) -> status` —
    /// `rule:types/erased-member-access`'s name-keyed shape read. The one object access that is not
    /// a fixed offset resolved here, and the one that can throw; see
    /// `nvs_ir::ir::InstKind::SlotGet`. The receiver travels by *address*,
    /// as a whole 16-byte value, because a `mixed` one arrives with a tag
    /// nothing proved and this helper is where it is checked.
    slot_get: Signature,
    /// `nvs_object_slot_probe(receiver, name, len, hint) -> bool` — the
    /// presence half of [`Self::slot_get`], and `I8` for [`Self::instanceof`]'s
    /// reason. Narrower than the read by both of the parameters
    /// `rule:errors/propagation` asks for: the question cannot fail, so there is
    /// no `ctx` to raise through and no `out` to write a value to. See
    /// `nvs_ir::ir::InstKind::SlotProbe`.
    slot_probe: Signature,
    /// `nvs_object_slot_set(ctx, receiver, name, len, hint, value, out) -> status`
    /// — `rule:types/erased-member-access`'s name-keyed shape *write*. One parameter wider than
    /// [`Self::slot_get`], because the value travels through a caller-owned
    /// 16-byte slot the way [`Self::array_value_at`]'s result does *and* the helper
    /// ABI still writes an (ignored) result of its own; see
    /// `nvs_ir::ir::InstKind::SlotSet`.
    slot_set: Signature,
    /// `nvs_object_key_get(ctx, receiver, key, out) -> status` — `rule:types/property-key-access`'s
    /// keyed read, which is [`Self::slot_get`]'s helper with the name arriving
    /// as a value. Narrower rather than wider: the `(ptr, len)` pair and the
    /// slot hint both go, because a key carries its bytes behind a header only
    /// the runtime knows and has no static name to have taken a position from.
    /// See `nvs_ir::ir::InstKind::KeyGet`.
    key_get: Signature,
    /// `nvs_object_key_set(ctx, receiver, key, value, out) -> status` —
    /// [`Self::key_get`]'s write half, one 16-byte value slot wider for the
    /// reason [`Self::slot_set`] is one wider than [`Self::slot_get`].
    key_set: Signature,
    /// `nvs_array_new() -> *mut ArrayHeader`.
    array_new: Signature,
    /// `nvs_array_set(array, key, value) -> *mut ArrayHeader`. There is no
    /// read signature beside it: an `nvs_ir::ir::InstKind::ArrayGet` throws on
    /// an absent key, so it travels the helper ABI ([`Self::helper`]) against
    /// `nvs_array_required_get`, which tells a rendered key from an `int`
    /// subscript by its own tag rather than by a second signature here.
    array_set: Signature,
    /// `nvs_array_set_index(array, index, value) -> *mut ArrayHeader` — the
    /// write reached by the `i64` an `int` subscript already was, with no key
    /// string built at all while the array is packed. `nvs-ir`'s module doc
    /// § *an array key is a `string`, and an `int` subscript no longer spells
    /// it* is the decision.
    array_set_index: Signature,
    /// `nvs_array_append(ctx, array, value, out) -> status` — the one array
    /// write that can fail, and so the one carrying `rule:errors/propagation`'s status shape
    /// rather than handing the array straight back. The array it yields
    /// travels through `out`, a caller-owned pointer-wide slot, the way
    /// [`Self::slot_set`]'s result travels through a 16-byte one;
    /// `nvs_runtime::nvs_array_append` owns what `out` holds on the refusal
    /// and why the refusal exists.
    array_append: Signature,
    /// `nvs_array_spread(ctx, array, subject, out) -> status` — the `[...$a]`
    /// element's whole-array copy. [`Self::array_append`]'s shape, and it
    /// carries the fault channel for the same reason: a renumbered key is an
    /// append. Its own signature all the same, because its third parameter is
    /// an array pointer where that one's is a 16-byte value slot, and this
    /// struct's rule is that no signature is shared by two symbols whose Rust
    /// declarations are not the same shape for the same reason.
    array_spread: Signature,
    /// `nvs_array_unset(array, key) -> *mut ArrayHeader` — two pointers in,
    /// one pointer back.
    ///
    /// Its own signature, never one borrowed from a symbol whose shape happens
    /// to agree: no signature here is shared by two symbols whose Rust
    /// declarations are not the same shape for the same reason, because a
    /// borrowed one turns the moment either symbol grows a parameter into a
    /// silent miscompile.
    array_unset: Signature,
    /// `nvs_array_next_slot(array, from) -> i64` — the `foreach` cursor step.
    /// `from` is a `usize` in the Rust signature, `I64` here: every target
    /// this JIT compiles for is 64-bit (see [`crate::ty::clif_ty`], which maps
    /// every pointer-shaped representation to `I64` for the same reason).
    array_next_slot: Signature,
    /// `nvs_array_key_at(array, slot) -> *mut StrHeader`.
    array_key_at: Signature,
    /// `nvs_array_value_at(array, slot, out)` — the read primitive whose
    /// result travels through a caller-owned 16-byte slot — see
    /// `nvs_runtime::array`'s "the primitives compiled code calls" note for
    /// why no `Value` crosses this boundary in a register.
    array_value_at: Signature,
}

/// The host ISA both backends compile for, under the flags that are a policy
/// rather than a tuning choice — `tests/backend_policy.rs` pins those of them
/// that leave no trace in a compiled unit.
///
/// **`is_pic` is the only flag the backends disagree about**, and the
/// disagreement is forced rather than a preference: a JIT owns the pages it
/// writes into and resolves every call and every descriptor to an absolute
/// address, while a relocatable object has no address to bake and every
/// reference in it must be one a loader can place. Everything else is shared on
/// purpose — an object emitted under different tuning would be evidence about
/// some other compiler, not about the one whose output runs.
fn host_isa(is_pic: bool) -> Result<codegen::isa::OwnedTargetIsa, CodegenError> {
    let mut flags = settings::builder();
    for (name, value) in [
        // A JIT resolves every call through an absolute address, and the
        // pages are its own — the configuration benches/abi-probe measures
        // Novis's costs under.
        ("use_colocated_libcalls", "false"),
        ("is_pic", if is_pic { "true" } else { "false" }),
        ("opt_level", "speed"),
        // Cranelift defaults this **off**, and off means a frame larger
        // than the guard page can move the stack pointer past it in one
        // step and write into whatever lies beyond — a stack clash, which
        // is a memory-safety bug rather than the clean crash a guard page
        // exists to produce. `probestack_size_log2` defaults to 12, so a
        // probe is emitted only for a frame over 4 KiB and no Novis frame
        // is that big: under callgrind, the retired-instruction count is
        // unchanged either way on a call-heavy fixture and on a deep
        // recursion. It is therefore insurance bought for nothing, and
        // the frame that would need it is exactly the one nobody predicts.
        //
        // **Not the same mechanism as `rule:errors/on-limit`'s call-stack limit**,
        // which counts *depth* against a `Ctx` field at the safepoint's
        // emit site. That catches a runaway recursion of ordinary frames;
        // this catches one oversized frame skipping the guard. Neither
        // covers the other, and both are wanted.
        ("enable_probestack", "true"),
        // Cranelift's default *strategy* is `outline`, which emits a call
        // to a `__cranelift_probestack` libcall — a symbol with a custom
        // register convention that no JIT gets for free, and that this
        // module's `builder.symbol` loop below does not supply. So an
        // outline probe does not protect an oversized frame; it panics
        // `cranelift-jit` with `can't resolve libcall __cranelift_probestack`
        // the first time one is compiled, which a script's file scope
        // reaches in a modest run of consecutive `echo`s. `inline` emits the
        // probe loop into the frame itself and needs no symbol, so the
        // guarantee above is the one actually in force.
        ("probestack_strategy", "inline"),
    ] {
        flags
            .set(name, value)
            .map_err(|e| CodegenError::UnsupportedHost(e.to_string()))?;
    }

    cranelift_native::builder()
        .map_err(|e| CodegenError::UnsupportedHost(e.to_owned()))?
        .finish(settings::Flags::new(flags))
        .map_err(|e| CodegenError::UnsupportedHost(e.to_string()))
}

impl UnitBuilder<JITModule> {
    /// The in-process backend: code compiled into pages this process owns, with
    /// every symbol it names resolved to an address before it is ever called.
    fn new(disasm: Option<String>) -> Result<Self, CodegenError> {
        let isa = host_isa(false)?;
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        // Two symbol tables, one namespace: `nvs_runtime`'s primitives and
        // helpers, and every Tier 0 `Core` member. Both have `rule:errors/propagation`'s one
        // helper signature, and `nvs_stdlib`'s own docs own why a `Core` call
        // is emitted through the same path a helper call is.
        for (name, address) in nvs_runtime::symbols()
            .into_iter()
            .chain(nvs_stdlib::symbols())
        {
            builder.symbol(name, address);
        }
        // The third: every `Core` class descriptor a folded constant names,
        // which is data rather than code and so carries a name of its own
        // shape. `core_desc_symbols` owns why this process already knows that
        // address while it does not yet know one of the unit's own.
        for (name, address) in core_desc_symbols() {
            builder.symbol(name, address);
        }
        // The last table, and the one that cannot be filled here: a class *this
        // unit declares* gets its descriptor from `compile_all`, long after
        // this builder is consumed, so its address is published through a
        // lookup closure the module calls at relocation time instead. See
        // `Classes`' own docs for why the address is a relocation at all.
        let desc_symbols: Arc<Mutex<FxHashMap<String, usize>>> = Arc::default();
        let published = Arc::clone(&desc_symbols);
        builder.symbol_lookup_fn(Box::new(move |name| {
            let table = published.lock().unwrap_or_else(PoisonError::into_inner);
            table
                .get(name)
                .map(|address| std::ptr::with_exposed_provenance(*address))
        }));

        Ok(Self::around(JITModule::new(builder), desc_symbols, disasm))
    }
}

impl UnitBuilder<ObjectModule> {
    /// The out-of-process backend: `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`'s relocatable object, for a
    /// cache file some later process will load.
    ///
    /// What differs from [`UnitBuilder::new`] follows from the file outliving
    /// the process that wrote it. `is_pic` is on, because
    /// there is no address here to bake. And there is no symbol table of any
    /// kind — no `builder.symbol` loop and no `symbol_lookup_fn` — because
    /// leaving every runtime helper and every `nvs_class_desc_*` undefined is
    /// precisely what makes the payload loadable somewhere else: an undefined
    /// symbol is a relocation record, and a resolved one is an address that was
    /// only ever true for the compiling process.
    fn for_object() -> Result<Self, CodegenError> {
        let isa = host_isa(true)?;
        // The object's own name is metadata: a cached artifact is addressed by
        // `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s key, which the file's contents cannot contribute to.
        let builder = ObjectBuilder::new(isa, "nvs", cranelift_module::default_libcall_names())
            .map_err(|source| CodegenError::Cranelift {
                function: "<unit>".to_owned(),
                source: Box::new(source),
            })?;
        Ok(Self::around(
            ObjectModule::new(builder),
            Arc::default(),
            None,
        ))
    }

    /// Writes the object out, which is this backend's whole answer.
    ///
    /// No counterpart to [`UnitBuilder::bind_method_tables`] runs here: a
    /// method row holds a *code address*, and there is none until whoever loads
    /// this file has placed it.
    fn finish_object(self) -> Result<Vec<u8>, CodegenError> {
        self.module.finish().emit().map_err(|source| {
            emit::internal(&format!(
                "an object file this unit could not write: {source}"
            ))
        })
    }
}

impl<M: Module> UnitBuilder<M> {
    /// The tables every backend shares, around a module only that backend's
    /// own constructor knows how to build.
    fn around(
        module: M,
        desc_symbols: Arc<Mutex<FxHashMap<String, usize>>>,
        disasm: Option<String>,
    ) -> Self {
        let sigs = Signatures::new(&module);
        Self {
            ctx: module.make_context(),
            fn_ctx: FunctionBuilderContext::new(),
            module,
            sigs,
            functions: FxHashMap::default(),
            shapes: FxHashMap::default(),
            classes: Classes::default(),
            desc_symbols,
            statics: FxHashMap::default(),
            static_defaults: Vec::new(),
            literals: 0,
            entries: Vec::new(),
            disasm,
            clif: None,
        }
    }

    /// Declares every function in `program`, then emits every body.
    ///
    /// The two passes are why a call can name a function declared further
    /// down the file, or itself: by the time any body is emitted, every Novis
    /// name in the unit already has a `FuncId` for `emit_call` to resolve
    /// against. Cranelift is fine with a call to a declared-but-not-yet-
    /// defined function; `finalize_definitions` is what would object if one
    /// were never defined.
    fn compile_all(&mut self, program: &Program) -> Result<(), CodegenError> {
        self.classes = Classes::build(&program.classes, &program.shape_codecs);
        // Publish every descriptor before any body is emitted, for the same
        // reason the function declarations below come first: a lowering may
        // name a class declared further down, and by relocation time every
        // name a body relocated against has to resolve or the JIT panics.
        // `expose_provenance` rather than `addr`, because this address is
        // about to be written into machine code and dereferenced there.
        {
            let mut symbols = self
                .desc_symbols
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for (label, desc) in self.classes.descriptors() {
                symbols.insert(class_desc_symbol(label), desc.expose_provenance());
            }
            // And every wire contract beside them, for the same reason and on
            // the same terms: a body naming one has to find it at relocation
            // time or the JIT panics.
            for (key, codec) in self.classes.shape_codecs() {
                symbols.insert(shape_codec_symbol(key), codec.expose_provenance());
            }
        }
        // The slot number *is* the position in `Program::statics`, which
        // `nvs_ir::lower` already sorted; nothing here reorders it, because
        // the vector handed to `nvs_runtime::Ctx::install_statics` has to be
        // indexed by the very numbers baked into the code below.
        for (slot, prop) in program.statics.iter().enumerate() {
            let slot = u32::try_from(slot)
                .map_err(|_| emit::internal("a unit declaring more than 2^32 statics"))?;
            self.statics
                .insert((prop.class.clone(), prop.name.clone()), slot);
            self.static_defaults.push(prop.default_value.clone());
        }
        for (index, function) in program.functions.iter().enumerate() {
            let symbol = function_symbol(index, &function.name);
            let id = self
                .module
                .declare_function(&symbol, Linkage::Local, &self.sigs.helper)
                .map_err(|source| CodegenError::Cranelift {
                    function: function.name.clone(),
                    source: Box::new(source),
                })?;
            self.functions.insert(function.name.clone(), id);
            self.shapes
                .insert(function.name.clone(), MethodShape::of(&function.params));
        }
        for function in &program.functions {
            self.compile_function(function)?;
        }
        Ok(())
    }

    /// Emits one already-declared function's body.
    fn compile_function(&mut self, function: &nvs_ir::Function) -> Result<(), CodegenError> {
        let id =
            *self
                .functions
                .get(&function.name)
                .ok_or_else(|| CodegenError::UnknownTarget {
                    caller: "<unit>".to_owned(),
                    target: function.name.clone(),
                })?;

        self.ctx.func.signature = self.sigs.helper.clone();
        // Must be set per function: `Module::clear_context` resets it along
        // with the rest of the context.
        self.ctx.set_disasm(self.disasm.is_some());
        let result = emit::emit_function(
            &mut self.module,
            &mut self.ctx,
            &mut self.fn_ctx,
            emit::UnitTables {
                sigs: &self.sigs,
                functions: &self.functions,
                classes: &self.classes,
                statics: &self.statics,
                literals: &mut self.literals,
            },
            function,
        );
        if let Err(error) = result {
            self.module.clear_context(&mut self.ctx);
            return Err(error);
        }

        self.collect_clif(&function.name);
        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|source| CodegenError::Cranelift {
                function: function.name.clone(),
                source: Box::new(source),
            })?;
        self.collect_disasm(&function.name);
        self.module.clear_context(&mut self.ctx);
        self.entries.push((function.name.clone(), id));
        Ok(())
    }

    /// Appends the just-emitted function's Cranelift IR, if one was asked for.
    ///
    /// Called before `define_function`, which is what turns that IR into
    /// machine code: what this renders is the shape the emitter wrote.
    fn collect_clif(&mut self, name: &str) {
        let Some(buffer) = self.clif.as_mut() else {
            return;
        };
        buffer.push_str("; ");
        buffer.push_str(name);
        buffer.push('\n');
        buffer.push_str(&self.ctx.func.to_string());
        buffer.push('\n');
    }

    /// Appends the just-compiled function's disassembly, if one was asked for.
    ///
    /// Called between `define_function` and `clear_context`, which is the only
    /// window the compiled code is still attached to the context.
    fn collect_disasm(&mut self, name: &str) {
        let Some(buffer) = self.disasm.as_mut() else {
            return;
        };
        let vcode = self
            .ctx
            .compiled_code()
            .and_then(|code| code.vcode.as_deref());
        buffer.push_str("; ");
        buffer.push_str(name);
        buffer.push('\n');
        match vcode {
            // Cranelift has no disassembler for every backend it can emit
            // for; saying so beats printing an empty section.
            None => buffer.push_str(";   <no disassembly available on this target>\n"),
            Some(text) => {
                buffer.push_str(text);
                if !text.ends_with('\n') {
                    buffer.push('\n');
                }
            }
        }
        buffer.push('\n');
    }
}

impl UnitBuilder<JITModule> {
    /// Finalizes the unit into callable code — the JIT's own ending, and the
    /// reason it is not on the shared impl: `finalize_definitions` resolves
    /// every relocation against an address in *this* process, which is exactly
    /// what an object file must not do.
    fn finish(mut self) -> Result<Unit, CodegenError> {
        self.module
            .finalize_definitions()
            .map_err(|source| CodegenError::Cranelift {
                function: "<unit>".to_owned(),
                source: Box::new(source),
            })?;
        self.bind_method_tables();
        let entries = self
            .entries
            .iter()
            .map(|(name, id)| (name.clone(), self.module.get_finalized_function(*id)))
            .collect();
        Ok(Unit {
            _code: Code::Jit(Box::new(self.module)),
            classes: std::sync::Arc::new(self.classes.table),
            entries,
            shapes: self.shapes,
            statics: self.static_defaults.into(),
        })
    }

    /// Hands every runtime `ClassDesc` the compiled addresses of the methods
    /// its class answers — what `nvs_ir::ir::InstKind::CallVirtual` and
    /// `InstKind::NewDynamic` dispatch through.
    ///
    /// Runs only after `finalize_definitions`, because that is the first
    /// moment a compiled function has an address at all. A `(method,
    /// declaring class)` pair naming a function the unit does not define is
    /// skipped rather than being an error: the same treatment `Classes::define`
    /// gives a `conforms` entry it cannot resolve, and for the same reason —
    /// the front end has already reported whatever left it behind, and
    /// `nvs_class_method`'s fallback keeps the call correct regardless.
    fn bind_method_tables(&mut self) {
        for entry in self.classes.by_label.values() {
            let methods = entry
                .methods
                .iter()
                .filter_map(|(method, declaring, public)| {
                    let label = format!("{declaring}::{method}");
                    let id = self.functions.get(&label)?;
                    // The shape is recorded under the very label the address
                    // is, in the one pass that saw the function, so the two
                    // halves of a row cannot describe two different callees.
                    let shape = self.shapes.get(&label).copied().unwrap_or_default();
                    Some(nvs_runtime::MethodRow {
                        name: method.clone(),
                        code: self.module.get_finalized_function(*id),
                        arity: shape.arity,
                        param_tags: shape.param_tags,
                        public: *public,
                        // Every row here is a compiled Novis function, which
                        // owns its parameters — `nvs-stdlib` is the only
                        // producer of a native one.
                        native: false,
                    })
                })
                .collect();
            self.classes.table.set_methods(entry.id, methods);
            // The accessors, on the AOT binder's terms exactly — the label a
            // hook's function was compiled under is the label the roster
            // carries, so nothing is assembled here.
            let hooks = entry
                .hooks
                .iter()
                .filter_map(|(property, label, set)| {
                    let id = self.functions.get(label)?;
                    let shape = self.shapes.get(label).copied().unwrap_or_default();
                    Some(nvs_runtime::HookRow {
                        property: property.clone(),
                        set: *set,
                        code: self.module.get_finalized_function(*id),
                        param_tags: shape.param_tags,
                    })
                })
                .collect();
            self.classes.table.set_hooks(entry.id, hooks);
        }
    }
}

impl Signatures {
    /// Every signature this unit calls through, built from whichever `Module`
    /// is finalizing it — `make_signature` only asks the target's own calling
    /// convention, so the answers are the same either way.
    fn new(module: &dyn Module) -> Self {
        let ptr = types::I64;

        let mut helper = module.make_signature();
        helper.params.push(AbiParam::new(ptr)); // ctx
        helper.params.push(AbiParam::new(ptr)); // args
        helper.params.push(AbiParam::new(ptr)); // out
        helper.returns.push(AbiParam::new(types::I32)); // status

        let mut helper_variadic = module.make_signature();
        helper_variadic.params.push(AbiParam::new(ptr)); // ctx
        helper_variadic.params.push(AbiParam::new(ptr)); // args
        helper_variadic.params.push(AbiParam::new(ptr)); // argc
        helper_variadic.params.push(AbiParam::new(ptr)); // out
        helper_variadic.returns.push(AbiParam::new(types::I32)); // status

        let mut safepoint = module.make_signature();
        safepoint.params.push(AbiParam::new(ptr));
        safepoint.returns.push(AbiParam::new(types::I32));

        let mut stack_check = module.make_signature();
        stack_check.params.push(AbiParam::new(ptr));
        stack_check.params.push(AbiParam::new(types::I64));
        stack_check.returns.push(AbiParam::new(types::I32));

        let mut probe = module.make_signature();
        probe.params.push(AbiParam::new(ptr));
        probe.params.push(AbiParam::new(types::I32));

        let mut probe_call = module.make_signature();
        probe_call.params.push(AbiParam::new(ptr));
        probe_call.params.push(AbiParam::new(ptr));
        probe_call.params.push(AbiParam::new(ptr));

        let mut probe_call_exit = probe_call.clone();
        probe_call_exit.params.push(AbiParam::new(types::I32));

        let mut str_concat = module.make_signature();
        str_concat.params.push(AbiParam::new(ptr));
        str_concat.params.push(AbiParam::new(ptr));
        str_concat.returns.push(AbiParam::new(ptr));

        let mut ptr_eq = module.make_signature();
        ptr_eq.params.push(AbiParam::new(ptr));
        ptr_eq.params.push(AbiParam::new(ptr));
        ptr_eq.returns.push(AbiParam::new(types::I8));

        let mut float_pow = module.make_signature();
        float_pow.params.push(AbiParam::new(types::F64));
        float_pow.params.push(AbiParam::new(types::F64));
        float_pow.returns.push(AbiParam::new(types::F64));

        let mut refcount = module.make_signature();
        refcount.params.push(AbiParam::new(ptr));

        let mut value_refcount = module.make_signature();
        value_refcount.params.push(AbiParam::new(types::I64));
        value_refcount.params.push(AbiParam::new(types::I64));

        let mut ptr_to_ptr = module.make_signature();
        ptr_to_ptr.params.push(AbiParam::new(ptr));
        ptr_to_ptr.returns.push(AbiParam::new(ptr));

        let mut raise = module.make_signature();
        raise.params.push(AbiParam::new(ptr)); // ctx
        raise.params.push(AbiParam::new(ptr)); // the exception object
        raise.params.push(AbiParam::new(ptr)); // the throw site's carrier, or zero

        let mut raise_new = module.make_signature();
        raise_new.params.push(AbiParam::new(ptr)); // ctx
        raise_new.params.push(AbiParam::new(ptr)); // class descriptor
        raise_new.params.push(AbiParam::new(ptr)); // message bytes
        raise_new.params.push(AbiParam::new(ptr)); // message length

        let mut instanceof = module.make_signature();
        instanceof.params.push(AbiParam::new(ptr)); // object
        instanceof.params.push(AbiParam::new(ptr)); // class descriptor
        instanceof.returns.push(AbiParam::new(types::I8));

        let mut class_method = module.make_signature();
        class_method.params.push(AbiParam::new(ptr)); // class descriptor
        class_method.params.push(AbiParam::new(ptr)); // method name bytes
        class_method.params.push(AbiParam::new(ptr)); // method name length
        class_method.params.push(AbiParam::new(ptr)); // fallback address
        class_method.returns.push(AbiParam::new(ptr));

        let mut slot_get = module.make_signature();
        slot_get.params.push(AbiParam::new(ptr)); // ctx
        slot_get.params.push(AbiParam::new(ptr)); // receiver, by address
        slot_get.params.push(AbiParam::new(ptr)); // field name bytes
        slot_get.params.push(AbiParam::new(ptr)); // field name length
        slot_get.params.push(AbiParam::new(types::I64)); // slot hint
        slot_get.params.push(AbiParam::new(ptr)); // out
        slot_get.returns.push(AbiParam::new(types::I32));

        let mut slot_probe = module.make_signature();
        slot_probe.params.push(AbiParam::new(ptr)); // receiver, by address
        slot_probe.params.push(AbiParam::new(ptr)); // field name bytes
        slot_probe.params.push(AbiParam::new(ptr)); // field name length
        slot_probe.params.push(AbiParam::new(types::I64)); // slot hint
        slot_probe.returns.push(AbiParam::new(types::I8));

        let mut slot_set = module.make_signature();
        slot_set.params.push(AbiParam::new(ptr)); // ctx
        slot_set.params.push(AbiParam::new(ptr)); // receiver, by address
        slot_set.params.push(AbiParam::new(ptr)); // field name bytes
        slot_set.params.push(AbiParam::new(ptr)); // field name length
        slot_set.params.push(AbiParam::new(types::I64)); // slot hint
        slot_set.params.push(AbiParam::new(ptr)); // value
        slot_set.params.push(AbiParam::new(ptr)); // out
        slot_set.returns.push(AbiParam::new(types::I32));

        let mut key_get = module.make_signature();
        key_get.params.push(AbiParam::new(ptr)); // ctx
        key_get.params.push(AbiParam::new(ptr)); // receiver, by address
        key_get.params.push(AbiParam::new(ptr)); // key, by address
        key_get.params.push(AbiParam::new(ptr)); // out
        key_get.returns.push(AbiParam::new(types::I32));

        let mut key_set = module.make_signature();
        key_set.params.push(AbiParam::new(ptr)); // ctx
        key_set.params.push(AbiParam::new(ptr)); // receiver, by address
        key_set.params.push(AbiParam::new(ptr)); // key, by address
        key_set.params.push(AbiParam::new(ptr)); // value, by address
        key_set.params.push(AbiParam::new(ptr)); // out
        key_set.returns.push(AbiParam::new(types::I32));

        let mut array_new = module.make_signature();
        array_new.returns.push(AbiParam::new(ptr));

        let mut array_set = module.make_signature();
        array_set.params.push(AbiParam::new(ptr)); // array
        array_set.params.push(AbiParam::new(ptr)); // key
        array_set.params.push(AbiParam::new(ptr)); // value
        array_set.returns.push(AbiParam::new(ptr));

        // Spelled out rather than cloned from the key-taking one: the middle
        // parameter is an `i64` index in the Rust signature, and only happens
        // to share `ptr`'s machine type on every target this crate builds for.
        let mut array_set_index = module.make_signature();
        array_set_index.params.push(AbiParam::new(ptr)); // array
        array_set_index.params.push(AbiParam::new(types::I64)); // index
        array_set_index.params.push(AbiParam::new(ptr)); // value
        array_set_index.returns.push(AbiParam::new(ptr));

        let mut array_append = module.make_signature();
        array_append.params.push(AbiParam::new(ptr)); // ctx
        array_append.params.push(AbiParam::new(ptr)); // array
        array_append.params.push(AbiParam::new(ptr)); // value
        array_append.params.push(AbiParam::new(ptr)); // out
        array_append.returns.push(AbiParam::new(types::I32));

        let mut array_spread = module.make_signature();
        array_spread.params.push(AbiParam::new(ptr)); // ctx
        array_spread.params.push(AbiParam::new(ptr)); // array
        array_spread.params.push(AbiParam::new(ptr)); // subject
        array_spread.params.push(AbiParam::new(ptr)); // out
        array_spread.returns.push(AbiParam::new(types::I32));

        let mut array_unset = module.make_signature();
        array_unset.params.push(AbiParam::new(ptr)); // array
        array_unset.params.push(AbiParam::new(ptr)); // key
        array_unset.returns.push(AbiParam::new(ptr));

        let mut array_next_slot = module.make_signature();
        array_next_slot.params.push(AbiParam::new(ptr)); // array
        array_next_slot.params.push(AbiParam::new(types::I64)); // from
        array_next_slot.returns.push(AbiParam::new(types::I64));

        let mut array_key_at = module.make_signature();
        array_key_at.params.push(AbiParam::new(ptr)); // array
        array_key_at.params.push(AbiParam::new(types::I64)); // slot
        array_key_at.returns.push(AbiParam::new(ptr));

        let mut array_value_at = module.make_signature();
        array_value_at.params.push(AbiParam::new(ptr)); // array
        array_value_at.params.push(AbiParam::new(types::I64)); // slot
        array_value_at.params.push(AbiParam::new(ptr)); // out

        Self {
            helper,
            helper_variadic,
            safepoint,
            stack_check,
            probe,
            probe_call,
            probe_call_exit,
            str_concat,
            ptr_eq,
            float_pow,
            refcount,
            value_refcount,
            ptr_to_ptr,
            raise,
            raise_new,
            instanceof,
            class_method,
            slot_get,
            slot_probe,
            slot_set,
            key_get,
            key_set,
            array_new,
            array_set,
            array_set_index,
            array_append,
            array_spread,
            array_unset,
            array_next_slot,
            array_key_at,
            array_value_at,
        }
    }
}

/// The name this backend gives the function at `index` in
/// `nvs_ir::Program::functions`.
///
/// The one home of that spelling, and every caller of it is deliberate:
/// [`UnitBuilder::compile_all`] declares every function under it, and
/// [`Descriptors::of`] derives it again from the same walk over the same
/// program so a loader can ask a placed payload for a method's address. Both
/// ends index the very `nvs_ir::Program` a warm hit's front end just lowered
/// (`rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`), so the index they hand in is the same one.
fn function_symbol(index: usize, label: &str) -> String {
    // `index` only disambiguates the symbol name: an Novis function name is not
    // a valid symbol (`<script>` is the first counter-example), and two classes
    // may declare the same method name.
    format!("nvs{index}_{}", sanitize(label))
}

/// Whether `symbol` is the name this backend gave the function `label`.
///
/// A function is emitted as `nvs<index>_<sanitize(label)>` — the index only
/// because [`sanitize`] can collide, and it is handed out by the walk over
/// `nvs_ir::Program::functions` rather than by anything a reader of the file
/// holds. So a loader cannot *derive* a function's symbol name the way
/// [`class_desc_symbol`] lets it derive a descriptor's; it can only recognise
/// one, which is what this answers, and it is `pub` for exactly the reason that
/// one is: the object backend's loader lives in another crate and a name two
/// crates must spell identically is a function one of them exports.
///
/// `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header` names `nvs_ir::lower::ENTRY_SCRIPT_LABEL` as the label the
/// entry frame carries, and that is the only `label` any caller outside this
/// crate has a reason to ask about.
#[must_use]
pub fn is_function_symbol(symbol: &str, label: &str) -> bool {
    let Some(rest) = symbol.strip_prefix("nvs") else {
        return false;
    };
    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return false;
    }
    rest[digits..]
        .strip_prefix('_')
        .is_some_and(|tail| tail == sanitize(label))
}

/// The symbol name a class descriptor's address is relocated against.
///
/// **This is a mangling scheme, unlike [`sanitize`]**, and it has to be: both
/// ends of the relocation derive the name from the label alone — the emitter
/// declaring the import, and whoever resolves it, which under `JITModule` is
/// [`UnitBuilder::compile_all`]'s table and under an object backend is the loader
/// reading the file in another process. There is no index to disambiguate
/// with, so two different labels must never collide. `sanitize` would collide
/// `Foo\Bar` with `Foo_Bar`; escaping every non-alphanumeric byte as `_xx`
/// cannot, because an escape's introducer is itself escaped.
///
/// It is `pub` for that second end. The loader `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` describes is in
/// another crate, and a name two crates must spell identically is a function
/// one of them exports rather than a rule both restate — a second
/// implementation of this loop is a mangling scheme that agrees with this one
/// only until someone edits one of them.
pub fn class_desc_symbol(label: &str) -> String {
    mangled("nvs_class_desc_", label)
}

/// Every `Core` class descriptor a unit may relocate against, under
/// [`class_desc_symbol`]'s name and at the address this process leaked it at.
///
/// A hole-free `` html`…` `` folds to a `Core\Html\Markup` constant in the
/// unit's own data section (`rule:core-classes/html-literal`), and the class
/// word of that constant is a relocation like any other — but against a
/// descriptor `nvs_stdlib` owns for the whole process rather than one this unit
/// built, so [`Classes`] holds no row for it and the symbol is an import the
/// unit never defines. Both ends that resolve a descriptor read this:
/// [`UnitBuilder::new`]'s symbol table, and [`Descriptors::resolve`], which is
/// what a warm cache hit relocates a stored payload against.
fn core_desc_symbols() -> impl Iterator<Item = (String, *const u8)> {
    nvs_stdlib::class_descriptors()
        .into_iter()
        .map(|(label, desc)| (class_desc_symbol(label), desc.cast::<u8>()))
}

/// The symbol name an inline shape's `nvs_runtime::ShapeCodec` address is
/// relocated against — [`class_desc_symbol`]'s twin for the contract a call
/// site carries beside the descriptor, and mangled by the same scheme for the
/// same reason: both ends of the relocation derive the name from the key alone.
///
/// The key is `nvs_ir::ir::ShapeCodec::key`, which renders the contract itself,
/// so two units that wrote the same shape mint the same symbol and one that
/// wrote `{n: string}` where another wrote `{n: int}` cannot collide with it.
#[must_use]
pub fn shape_codec_symbol(key: &str) -> String {
    mangled("nvs_shape_codec_", key)
}

/// `prefix` followed by `label` with every non-alphanumeric byte escaped as
/// `_xx` — the one mangling scheme, shared by the two symbol families that need
/// a name both ends of a relocation can derive. [`class_desc_symbol`]'s docs
/// own why an escape rather than [`sanitize`].
fn mangled(prefix: &str, label: &str) -> String {
    let mut out = String::from(prefix);
    for byte in label.bytes() {
        if byte.is_ascii_alphanumeric() {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("_{byte:02x}"));
        }
    }
    out
}

/// Reduces an Novis function name to something a linker symbol may contain.
///
/// Not a mangling scheme: [`UnitBuilder::compile_function`]'s index already supplies
/// uniqueness, so this only has to keep the name readable in a disassembly.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_module::FuncOrDataId;
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_ir::Ty;
    use nvs_ir::lower::FN_PARAM_TAG_ANY;

    /// The whole front end over `source`, lowered but not compiled.
    ///
    /// `tests/common/mod.rs` has the same helper for the end-to-end binaries.
    /// This copy exists because the tests below assert on [`UnitBuilder`]'s *private*
    /// tables — what the module declared, and what a symbol resolves to — and
    /// nothing outside this file can reach those.
    fn lower(source: &str) -> Program {
        let mut map = SourceMap::new();
        let id = map.add("test.nvs", source);
        let src = map.file(id);

        let mut diags = Diagnostics::new();
        let stmts = nvs_syntax::parse_file(src, &mut diags);
        let module = nvs_hir::resolve_file(&stmts, src, &mut diags);
        let mut interner = nvs_types::TypeInterner::new();
        let mut exprs = nvs_types::ExprTypeTable::new();
        let files = [nvs_types::ProgramFile { src, stmts: &stmts }];
        let enums =
            nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
        assert!(
            !diags.has_errors(),
            "the fixture does not type-check: {:?}",
            diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        );

        let layouts = nvs_types::build_class_layouts(&files, &module.graph);
        nvs_ir::lower::lower_file(
            nvs_ir::lower::ENTRY_SCRIPT_LABEL,
            &stmts,
            src,
            &exprs,
            &interner,
            &enums,
            &layouts,
        )
    }

    /// `rule:core-classes/html-literal`'s folded constant names a descriptor no
    /// unit builds, so the two ends that resolve one both answer for it out of
    /// [`core_desc_symbols`]: a unit declaring no class at all still resolves
    /// the carrier, and [`Classes::desc`] is the door that stays shut.
    #[test]
    fn the_markup_carrier_resolves_for_a_unit_that_declares_no_class() {
        let name = class_desc_symbol(nvs_runtime::CARRIER_HTML_MARKUP);
        let published = core_desc_symbols()
            .find(|(symbol, _)| *symbol == name)
            .expect("the carrier's descriptor is published")
            .1;
        assert!(!published.is_null());

        let program = lower("<?nvs\nint $x = 1;\n");
        assert_eq!(Descriptors::of(&program).resolve(&name), Some(published));
        assert!(
            Classes::build(&program.classes, &program.shape_codecs)
                .desc(nvs_runtime::CARRIER_HTML_MARKUP)
                .is_none(),
            "the unit's own table answers for the carrier, so the import is not one"
        );
    }

    /// Compiles `source` and hands back the JIT with its tables intact —
    /// stopping short of [`UnitBuilder::finish`], which consumes them.
    fn compiled(source: &str) -> UnitBuilder<JITModule> {
        let program = lower(source);
        let mut jit = UnitBuilder::new(None).expect("this host has a Cranelift backend");
        jit.compile_all(&program).expect("the fixture compiles");
        jit
    }

    /// Asserts that `label`'s descriptor reached the code as a relocation, and
    /// that the relocation resolves to the address the table holds.
    ///
    /// The first half is the property: an **imported** symbol is one this unit
    /// declared and did not define, so every use of it leaves a relocation
    /// record — which is the whole of what `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`'s object payload needs
    /// and the whole of what an `iconst` immediate destroys. The second is the
    /// promise that comes with it: under `JITModule` the record resolves to
    /// the very address the table holds, so nothing on the hot path pays for
    /// the indirection.
    fn assert_relocated(jit: &UnitBuilder<JITModule>, label: &str) {
        let name = class_desc_symbol(label);
        let Some(FuncOrDataId::Data(id)) = jit.module.declarations().get_name(&name) else {
            panic!("no code relocated against `{name}`, so `{label}`'s address was baked in");
        };
        assert_eq!(
            jit.module.declarations().get_data_decl(id).linkage,
            Linkage::Import,
            "`{name}` is defined by this unit, so its uses carry no relocation"
        );

        let published = jit
            .desc_symbols
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let expected = jit
            .classes
            .desc(label)
            .expect("the unit declares the class")
            .expose_provenance();
        assert_eq!(
            published.get(&name).copied(),
            Some(expected),
            "`{name}` does not resolve to `{label}`'s descriptor, so the JIT path moved"
        );
    }

    #[test]
    fn two_class_labels_never_share_a_descriptor_symbol() {
        // Both ends of the relocation derive the name from the label and
        // nothing else, so a collision is not a readability problem: it is one
        // class resolving to another's descriptor. `sanitize` collides these
        // two; the escape does not, and the escaped `_` is why.
        assert_eq!(class_desc_symbol("Foo\\Bar"), "nvs_class_desc_Foo_5cBar");
        assert_eq!(class_desc_symbol("Foo_Bar"), "nvs_class_desc_Foo_5fBar");
        assert_eq!(class_desc_symbol("Point"), "nvs_class_desc_Point");
    }

    #[test]
    fn the_entry_frame_s_symbol_is_recognised_and_no_other_is() {
        // `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`'s loader finds the frame to enter by walking the symbol
        // table, because the index in the name is the emitter's and not
        // derivable. What it must not do is accept a *helper* the payload leaves
        // undefined, or the frame of some other function whose sanitized name
        // happens to start the same way.
        let entry = nvs_ir::lower::ENTRY_SCRIPT_LABEL;
        assert!(is_function_symbol(
            &format!("nvs0_{}", sanitize(entry)),
            entry
        ));
        assert!(is_function_symbol(
            &format!("nvs17_{}", sanitize(entry)),
            entry
        ));
        assert!(!is_function_symbol("nvs_echo_str", entry));
        assert!(!is_function_symbol(&class_desc_symbol("Point"), entry));
        assert!(!is_function_symbol("nvs0_Widget__g", entry));
        assert!(is_function_symbol("nvs0_Widget__g", "Widget::g"));
    }

    #[test]
    fn a_class_descriptor_address_is_a_relocation_not_an_immediate() {
        let jit = compiled(
            "<?nvs
class Point {
    public int $x = 1;
}

var $p = new Point();
echo $p->x;
",
        );
        assert_relocated(&jit, "Point");
    }

    #[test]
    fn an_instanceof_target_is_a_relocation_not_an_immediate() {
        // `Circle` is never constructed here, so the `instanceof` is the only
        // lowering that could have asked for its descriptor: the import below
        // is that site's relocation and no other's.
        let jit = compiled(
            "<?nvs
class Shape {
    public int $sides = 0;
}

class Circle extends Shape {
}

var $s = new Shape();
if ($s instanceof Circle) { echo \"circle\"; }
",
        );
        assert_relocated(&jit, "Circle");
    }

    /// The programs both backends are asked for, one per lowering family a
    /// [`Module`] gets a say in: a descriptor address, an `instanceof`, a
    /// statically resolved call, a string literal's data object, a branch and a
    /// loop. Those are the sites where an object file needs a relocation and a
    /// JIT needs an address, so they are where two walks would first disagree.
    const BOTH_BACKENDS: &[(&str, &str)] = &[
        (
            "a class, its field and its method",
            "<?nvs
class Point {
    public int $x = 1;

    public function shifted(int $by): int {
        return $this->x + $by;
    }
}

var $p = new Point();
echo $p->shifted(2);
",
        ),
        (
            "an instanceof against a class never constructed",
            "<?nvs
class Shape {
    public int $sides = 0;
}

class Circle extends Shape {
}

var $s = new Shape();
if ($s instanceof Circle) { echo \"circle\"; }
",
        ),
        (
            "a statically resolved call and a string literal",
            "<?nvs
class Tag {
    public static function of(int $n): string {
        return \"tag\";
    }
}

echo Tag::of(3);
",
        ),
        (
            "a branch inside a loop",
            "<?nvs
var $total = 0;
var $i = 0;
while ($i < 10) {
    if ($i > 4) {
        $total = $total + $i;
    }
    $i = $i + 1;
}
echo $total;
",
        ),
    ];

    /// `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`'s keystone: the object backend is a second [`Module`] behind
    /// the one lowering walk, so every program the JIT emits, it emits too.
    ///
    /// A failure here is not "the object file came out wrong". It is that the
    /// two products no longer come from the same walk, which is the one thing
    /// the goal's standing decision forbids outright — a second lowering is a
    /// second semantics.
    #[test]
    fn the_object_module_emits_every_program_the_jit_module_does() {
        for (what, source) in BOTH_BACKENDS {
            let program = lower(source);

            let mut jit = UnitBuilder::new(None).expect("this host has a Cranelift backend");
            jit.compile_all(&program)
                .unwrap_or_else(|err| panic!("the JIT does not emit {what}: {err}"));

            let object = compile_object(&program)
                .unwrap_or_else(|err| panic!("the object module does not emit {what}: {err}"));
            assert!(
                object.len() > 64,
                "the object module answered {} bytes for {what}, which is no unit at all",
                object.len()
            );
        }
    }

    /// Every symbol a module was asked to declare, in one comparable form:
    /// what it is, the name it carries into a linker, and its linkage.
    ///
    /// This is the whole of what a `Module` is told by the walk, which is why
    /// comparing it compares the walks rather than the products — two backends
    /// that were handed the same declarations in the same order ran the same
    /// code to produce them.
    fn declared(module: &dyn Module) -> Vec<String> {
        let decls = module.declarations();
        let mut out: Vec<String> = decls
            .get_functions()
            .map(|(id, decl)| format!("fn {} {:?}", decl.linkage_name(id), decl.linkage))
            .chain(
                decls
                    .get_data_objects()
                    .map(|(id, decl)| format!("data {} {:?}", decl.linkage_name(id), decl.linkage)),
            )
            .collect();
        out.sort();
        out
    }

    /// The other half of the keystone: not just that both backends *finish*,
    /// but that they were told the same thing on the way there.
    ///
    /// Their machine code cannot be compared — `is_pic` differs, so the same
    /// call is a different encoding — and comparing it would be the wrong claim
    /// anyway. What must match is the walk's own output: every function and
    /// every literal, under the same name and the same linkage, in one shared
    /// pass over [`BOTH_BACKENDS`].
    #[test]
    fn the_two_modules_answer_the_same_for_every_lowering_fixture() {
        for (what, source) in BOTH_BACKENDS {
            let program = lower(source);

            let mut jit = UnitBuilder::new(None).expect("this host has a Cranelift backend");
            jit.compile_all(&program)
                .expect("the JIT emits the fixture");
            let mut object = UnitBuilder::for_object().expect("this host has a Cranelift backend");
            object
                .compile_all(&program)
                .expect("the object module emits the fixture");

            assert_eq!(
                declared(&jit.module),
                declared(&object.module),
                "the two modules were handed different declarations for {what}, so one walk \
                 has become two"
            );
        }
    }

    /// A call to a function this unit defines reaches the code as a relocation
    /// against the callee's symbol, not as a baked address.
    ///
    /// Under `JITModule` it always is — `func_addr` against a `FuncId` is a
    /// relocation whichever module finalizes it — so the claim is only
    /// *readable* on the object product, where the relocation table survives
    /// finalization. That is what this reads: a `nvs_class_desc_*` import proves
    /// nothing about a call, and a call site is the other half of what `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`
    /// 's loader has to place.
    #[test]
    fn a_statically_resolved_call_target_is_a_relocation_not_an_immediate() {
        use object::{Object, ObjectSection, ObjectSymbol};

        let program = lower(
            "<?nvs
class Tag {
    public static function of(int $n): string {
        return \"tag\";
    }
}

echo Tag::of(3);
",
        );
        let mut unit = UnitBuilder::for_object().expect("this host has a Cranelift backend");
        unit.compile_all(&program).expect("the fixture compiles");
        let id = *unit
            .functions
            .get("Tag::of")
            .expect("the unit defines the callee");
        let callee = unit
            .module
            .declarations()
            .get_function_decl(id)
            .linkage_name(id)
            .into_owned();
        let bytes = unit.finish_object().expect("the object is written");

        let file = object::File::parse(&*bytes).expect("cranelift wrote a readable object");
        // Mach-O prefixes every linker-visible name with `_` and ELF and COFF do not, so a
        // relocation names the callee in the object format's spelling rather than in the one
        // `linkage_name` hands back. Deriving what to look for from the object's own format is
        // what keeps this a claim about the *symbol* on every host.
        let wanted = match file.format() {
            object::BinaryFormat::MachO => format!("_{callee}"),
            _ => callee,
        };
        let mut targets = Vec::new();
        for section in file.sections() {
            for (_, reloc) in section.relocations() {
                if let object::RelocationTarget::Symbol(index) = reloc.target()
                    && let Ok(symbol) = file.symbol_by_index(index)
                    && let Ok(name) = symbol.name()
                {
                    targets.push(name.to_owned());
                }
            }
        }
        assert!(
            targets.contains(&wanted),
            "nothing in the object relocates against `{wanted}`, so the call to it was baked \
             in as an address this process alone could use. Relocations found: {targets:?}"
        );
    }

    /// The row a descriptor carries describes what a **call site** writes, and
    /// `nvs_ir::ir::Function::params` describes what the *callee* declares —
    /// the two differ by the implicit receiver in slot 0, and [`MethodShape`]
    /// is the one place that difference is taken. Getting it wrong is not a
    /// wrong count but a shifted one: every nibble would then judge the
    /// argument beside the one it describes, which is the priority-1 hole the
    /// word exists to close, arrived at from inside the compiler.
    #[test]
    fn a_method_shape_subtracts_the_receiver_and_packs_the_rest() {
        // `$this`, then `string`, then `int` — nibbles 5 and 2 (`param_tag_nibble`).
        let shape = MethodShape::of(&[Ty::Object, Ty::Str, Ty::Int]);
        assert_eq!(shape.arity, 2);
        assert_eq!(shape.param_tags, 0x25);

        // A `mixed` parameter is the one nibble that is not a tag.
        let tagged = MethodShape::of(&[Ty::Object, Ty::Tagged]);
        assert_eq!(tagged.arity, 1);
        assert_eq!(tagged.param_tags, u64::from(FN_PARAM_TAG_ANY));

        // A receiver and nothing else, and a frame with no receiver at all —
        // a script body — both answer zero rather than underflowing.
        assert_eq!(MethodShape::of(&[Ty::Object]).arity, 0);
        assert_eq!(MethodShape::of(&[]).arity, 0);
        assert_eq!(MethodShape::of(&[]).param_tags, 0);
    }

    /// What `nvs-cli`'s unit cache publishes has to be shareable, or "compile
    /// exactly once" becomes "compile once per core" — see [`Unit`]'s own
    /// `unsafe impl`s for why the two auto traits are sound here.
    ///
    /// A compile-time assertion: removing either impl stops this building
    /// rather than leaving a green test over a cache that quietly split.
    #[test]
    fn a_unit_crosses_a_core_boundary_behind_an_arc() {
        const fn crosses<T: Send + Sync>() {}
        crosses::<Unit>();
        crosses::<std::sync::Arc<Unit>>();
        // The loader's half is bounded by the trait rather than vouched for,
        // so a `Placed` implementation that is not shareable fails at *its*
        // definition and not here.
        crosses::<Box<dyn Placed>>();
        // The JIT's half is vouched for, and this is the one clause of that
        // argument a dependency can take away: `Unit`'s `Send` rests on the
        // last handle being able to drop a `JITModule` on any core. `Sync` is
        // deliberately not asserted — the module memoizes symbol lookups
        // through a `RefCell`, and the argument is that nothing can reach it.
        const fn sends<T: Send>() {}
        sends::<JITModule>();
    }
}
