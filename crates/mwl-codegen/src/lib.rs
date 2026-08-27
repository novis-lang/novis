//! MWL's baseline Cranelift backend: [`mwl_ir`]'s CFG/SSA form in, native
//! code behind [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s
//! calling convention out.
//!
//! This crate is the one place that knows *both* [`mwl_ir`] and
//! [`mwl_runtime`]. `mwl-runtime` deliberately depends on neither, so mapping
//! an [`mwl_ir::ir::Helper`] tag onto a symbol name from
//! [`mwl_runtime::symbols`] is this crate's job by design — see that crate's
//! own module docs.
//!
//! # The shape it emits
//!
//! Every compiled function has the one signature ADR 0002 makes normative:
//!
//! ```text
//! extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32
//! ```
//!
//! Nothing unwinds. Every call — a runtime helper, an MWL method, the
//! safepoint slow path — is followed by a compare-and-branch on the returned
//! status. That pair of instructions is what replaces a landing pad;
//! `benches/abi-probe`'s `compile_chain` has measured its cost since M0.
//!
//! ## Where a failing status goes
//!
//! Onward, but not blindly: [`mwl_ir::ir::Inst::on_error`] names a *landing
//! block* per call site, and the branch enters it carrying the status as a
//! block parameter. The landing block holds the frame's refcount cleanup and
//! ends in [`mwl_ir::ir::Terminator::Propagate`] (record this frame on the
//! exception's backtrace, then return the status) or
//! [`mwl_ir::ir::Terminator::Catch`] (on `THROWN`, enter the handler; anything
//! else returns onward). A `throw` reaches the same block through
//! [`mwl_ir::ir::Terminator::Throw`], which raises the exception and jumps
//! with a constant `THROWN`.
//!
//! **The backtrace is built on the error path, never the success path.** Each
//! frame's landing block passes a static label from the unit's own data
//! section to [`mwl_runtime::mwl_trace_push`]. The alternative — a push/pop
//! frame record around every call — would move that cost onto the path that
//! actually runs, which is precisely what ADR 0002 exists to avoid. See
//! `mwl_runtime::throwable`'s own docs for the one observable consequence.
//!
//! ## An object is a pointer; its fields are tagged
//!
//! An instance is a bare [`mwl_runtime::ObjHeader`] pointer in a register, and
//! `new` is one `iconst` of the class descriptor's address plus one call — see
//! [`Classes`] for why a JIT can bake that address in. A *field* is different:
//! every slot is a whole 16-byte [`mwl_runtime::Value`], so a `FieldGet` loads
//! the payload half at [`mwl_runtime::field_offset`] and a `FieldSet` stores
//! both halves. [`mwl_runtime::object`]'s own docs own that decision and state
//! its cost; this crate only queries the offset.
//!
//! The same is true of late static binding: a static method's argument slot 0
//! carries the called class rather than `null`, and a `ClassDescOf` is one
//! load at [`mwl_runtime::OBJ_CLASS_OFFSET`]. That decision — including why the
//! slot keeps a `null` *tag* — is `mwl_runtime::object`'s too; this crate emits
//! the load and the [`mwl_runtime::mwl_class_method`] lookup it feeds.
//!
//! ## Values are native, not tagged, wherever the type is known
//!
//! [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) settles every
//! operand type before lowering, so an `int` local lives in an `i64` register
//! and a `string` in a bare `StrHeader` pointer. A 16-byte
//! [`mwl_runtime::Value`] is *materialized* only where the ABI demands one —
//! at a call boundary and at the `out` slot — exactly as that type's own docs
//! say. [`ty::clif_ty`] is the whole of the mapping.
//!
//! ## The three hot-word checks
//!
//! Each is a load of one word from [`mwl_runtime::Ctx`] plus a
//! predicted-not-taken branch, and each is emitted unconditionally:
//!
//! * the **safepoint poll** at every [`mwl_ir::ir::InstKind::Safepoint`] —
//!   function entry and loop back edges, the project-start decision's two
//!   fixed sites;
//! * [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
//!   **call-stack compare**, riding the *first* of those polls so that it
//!   lands at function entry and nowhere else — one load, one compare against
//!   Cranelift's `get_stack_pointer`, branching to
//!   [`mwl_runtime::mwl_stack_check`]. It is the one of the three that is
//!   *elided*: `emit::is_leaf` answers which functions cannot grow the stack
//!   past the reserve their caller already checked with, and those carry none;
//! * [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
//!   § 1's **debug-flags check**, at every
//!   [`mwl_ir::ir::InstKind::StmtMarker`] (branching to
//!   [`mwl_runtime::mwl_probe_stmt`]) and twice at every call site, before and
//!   after (branching to [`mwl_runtime::mwl_probe_call_enter`] and
//!   [`mwl_runtime::mwl_probe_call_exit`]).
//!
//! None is behind a flag or a build configuration: ADR 0018's whole
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
//! the first backend to be exactly as wide as `mwl run examples/hello.mwl`
//! requires, and each gap below is a missing *lowering*, not a missing
//! decision:
//!
//! 0. **A `finally` does not run on every exit.** It runs on the normal exit,
//!    on a `return` out of the protected region *or out of a matched `catch`
//!    clause*, on the exception path, and at the end of a matched clause —
//!    but not when a `catch` clause's own body throws, and a
//!    `break`/`continue` out of a protected region is refused outright rather
//!    than lowered without one. See
//!    [`mwl_ir::lower::Lowering::lower_try`], which owns the whole policy.
//!    Exceptions themselves are ordinary objects: `Ty::Throwable` is gone,
//!    a user class `extends Throwable` compiles like any other, and a typed
//!    `catch` is an [`mwl_ir::ir::InstKind::InstanceOf`] chain.
//! 1. **Virtual dispatch is by name, not by slot.** An instance call whose
//!    resolved declaration some subtype overrides — and the two shapes with
//!    no static answer at all, `static::method(...)`/`new static(...)` and a
//!    call resolving to a declaration with no *body* — lower to
//!    [`mwl_ir::ir::InstKind::CallVirtual`]/`NewDynamic`, look the method up
//!    on the receiver's or the late-static-binding class through
//!    [`mwl_runtime::mwl_class_method`], and call the address it returns
//!    indirectly under the same ADR 0002 signature. Everything else binds
//!    straight to a label, because `mwl_types` answers "does anything
//!    override this" for the whole program once
//!    (`mwl_types::expr_table::ResolvedCall::overridden`). What is left is a
//!    per-class slot index instead of a string compare, which
//!    [`mwl_ir::ir::Program::classes`] already carries the table for.
//!
//!    Everything else about objects and arrays compiles: `New`, `FieldGet`,
//!    `FieldSet`, an instance `Call`, every array instruction — `ArrayNew`,
//!    `ArrayGet`, `ArraySet`, `ArrayAppend`, `ArrayUnset` and `foreach`'s
//!    `ArrayNextSlot`/`ArrayKeyAt`/`ArrayValueAt` cursor — and a
//!    `Ty::Object`/`Ty::Array` retain/release, against
//!    [`mwl_runtime::object`]'s layout, the per-class slot table
//!    [`mwl_ir::ir::Program::classes`] carries, and [`mwl_runtime::array`]'s
//!    primitives. A Tier 0 `Core` member call
//!    ([`mwl_ir::ir::InstKind::CoreCall`]) compiles too, through the helper
//!    path unchanged — [`mwl_stdlib`]'s own docs own why it needs no path of
//!    its own.
//! 2. **ADR 0018's `BRANCH` probe is not emitted.** It needs a per-edge site
//!    at [`mwl_ir::ir::Terminator::Branch`]'s lowering, which is the only one
//!    of that ADR's three sites still missing — the statement-boundary probe
//!    and the call-site `TRACE`/`PROFILE` pair are both emitted.
//! 3. **A `FATAL` still leaks the frame's locals.** A `THROWN` does not: its
//!    landing block releases them before the status travels on. The
//!    asymmetry is `mwl_ir`'s, not this crate's — see
//!    [`mwl_ir::ir::Inst::on_error`], which explains why an outcome no
//!    cleanup path and no `catch` can act on gets no landing block at all.
//! 4. **Two identical string literals are two data objects.** Each
//!    `InstKind::ConstStr` emits its own immortal header and payload under its
//!    own name, so a unit that writes `"id"` in forty places holds forty
//!    copies of it. Nothing on the request path pays — each site materializes
//!    one address either way — so what this costs is unit bytes and some
//!    instruction-cache locality, and closing it means keying a map on the
//!    literal's bytes beside `emit::Emitter::define_literal`'s counter.
//! 5. **Integer `/` compiles, and it is the one operator that picks its
//!    result representation at run time.**
//!    [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 4 types
//!    `int / int` as `int|float` — PHP-exact, so `6/3` is an integer and `7/2`
//!    is not — and that union's representation is [`mwl_ir::Ty::Tagged`], so
//!    `emit_binop` hands the row to its own `emit_int_div`: a zero-divisor
//!    guard in front, then a branch on whether the remainder is zero, joining
//!    at a phi that carries the tagged value. The signed overflow
//!    `i64::MIN / -1` takes the inexact arm rather than a second throw,
//!    because PHP answers a `float` for it. `mwl_ir::lower::Lowering::coerce`
//!    is where the union is absorbed back into a declared `float`, which is
//!    what makes `float $avg = $sum / $n;` the ADR's own worked example.
//! 6. **[`mwl_ir::ir::Terminator::Switch`] lowers to a compare chain, not a
//!    jump table.** Correct for any case set — the IR deliberately does not
//!    require a dense or sorted one — and the arms are few in the one
//!    producer there is today, ADR 0053 § 4's generator resumption (one per
//!    `yield`, plus the entry and exhausted arms). A `br_table` over a dense
//!    case set is the obvious optimisation. MWL's own `switch` statement never
//!    reaches this terminator — it lowers to a `Branch` chain, since a label
//!    is any expression of the subject's type — so closing this gap would also
//!    mean teaching `mwl_ir::lower::Lowering::lower_switch` to recognise a
//!    dense all-integer case set and reach for it.
//! 7. **Executable memory is never freed.** [`Unit`] holds its `JITModule` for
//!    the process's lifetime; `cranelift_jit::JITModule::free_memory` is
//!    `unsafe` and needs the "no compiled frame is still live" proof that
//!    [ADR 0017](../../../docs/adr/0017-hot-reload-without-restart.md)'s
//!    pointer-swap reclamation is the real home for. A one-shot `mwl run`
//!    exits before it matters.
//! 8. **Integer `+`, `-`, `*` and unary `-` throw on overflow rather than
//!    wrapping**, which
//!    [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 4 calls the
//!    divergence from PHP it is least willing to trade. `emit_binop` hands all
//!    three binary rows to `emit_checked_int_arith` and `emit_unop` takes the
//!    fourth, each reading Cranelift's `sadd_overflow`/`uadd_overflow` family
//!    — the flag the CPU already sets, so the cost is one predicted branch and
//!    no synthesized compare — and raising spec § 10's `ArithmeticError` on
//!    [`mwl_ir::ir::Inst::on_error`]'s edge through the shared
//!    `raise_arithmetic_error`, which the two zero-divisor guards now use too.
//!    The signed and unsigned rows are different instructions rather than one
//!    read two ways: a carry out of bit 63 is not a sign flip, which is what
//!    keeps `uint` exact over `0 … 2^64−1`.
//! 9. **A binary operator wants both operands in one representation, and
//!    knows only the numeric and `bool` ones.** A mixed numeric pair no longer
//!    reaches here — `mwl_ir::lower` settles `1 + 1.5` by widening the integer
//!    side and `$n < $f` by a helper, which is what keeps this crate's "a
//!    `BinOp` has one representation" invariant a genuine internal error. What
//!    is still refused is `==` over two enum values, whose `Enum(Int)`
//!    representation is not on the integral list even though comparing the two
//!    integers is exactly right — a missing arm rather than a missing
//!    mechanism, since ADR 0010 makes an enum *be* its integer.

mod emit;
mod ty;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, ModuleError};
use mwl_ir::Program;
use mwl_runtime::MwlFn;
use rustc_hash::FxHashMap;

pub use ty::clif_ty;

/// Why a program could not be compiled.
///
/// Every variant is an *engine* failure — a shape this backend does not lower
/// yet, or a host that cannot host a JIT. None of them is a user diagnostic:
/// `mwl run` has already run `mwl_types::check_program` and reported every
/// diagnostic before a single instruction is emitted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CodegenError {
    /// An IR shape this slice does not lower — see the crate docs' scope list.
    #[error("mwl-codegen does not lower {0} yet")]
    Unsupported(String),
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
        /// The MWL function being compiled.
        function: String,
        /// What Cranelift reported.
        source: Box<ModuleError>,
    },
    /// A call naming a function this compilation unit does not define.
    ///
    /// Always an engine bug rather than a user error: `mwl_types` resolved
    /// the target before lowering ever rendered its label, so a unit that
    /// contains the call and not the callee was assembled wrong.
    #[error("internal error: `{caller}` calls `{target}`, which this unit does not define")]
    UnknownTarget {
        /// The MWL function containing the call.
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
pub struct Unit {
    /// Kept alive for its pages; never read again after `compile` returns.
    _module: JITModule,
    /// Kept alive for its *descriptors*: the compiled code holds each one's
    /// address as a baked-in constant (see [`Classes`]), so the table must
    /// outlive every instance and every frame that can allocate one. Moving
    /// the table here is safe because each descriptor is individually boxed —
    /// only the `Vec`'s own three words move, never a `ClassDesc`.
    ///
    /// Shared rather than owned outright so a [`mwl_runtime::ErrorClass`]
    /// handed to a [`mwl_runtime::Ctx`] can keep it alive by itself — that is
    /// what makes installing one need no `unsafe` at the call site.
    classes: std::rc::Rc<mwl_runtime::ClassTable>,
    entries: FxHashMap<String, *const u8>,
}

impl std::fmt::Debug for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut names: Vec<&str> = self.entries.keys().map(String::as_str).collect();
        names.sort_unstable();
        f.debug_struct("Unit").field("functions", &names).finish()
    }
}

impl Unit {
    /// The compiled function `name` names, ready to call — through
    /// [`mwl_runtime::call`], which is the safe wrapper over [`MwlFn`]'s
    /// pointer contract.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "handing back a JIT-compiled code pointer as a callable is a \
                  transmute with no safe spelling; the function was declared \
                  with exactly `MwlFn`'s signature in `compile` and \
                  `finalize_definitions` has made its pages executable"
    )]
    pub fn function(&self, name: &str) -> Option<MwlFn> {
        let code = *self.entries.get(name)?;
        Some(unsafe { std::mem::transmute::<*const u8, MwlFn>(code) })
    }

    /// A handle on the class a runtime helper's bare-message failure is
    /// promoted to — spec § 10's `RuntimeError`, which is what "the world said
    /// no" means.
    ///
    /// `None` only if the unit somehow declares no such class, which the
    /// seeded exception tree (`mwl_hir::errors`) makes impossible for a
    /// program that went through the front end.
    ///
    /// The returned handle shares ownership of the descriptor table, so it may
    /// safely outlive this `Unit` — see [`mwl_runtime::ErrorClass`].
    #[must_use]
    pub fn runtime_error_class(&self) -> Option<mwl_runtime::ErrorClass> {
        let id = self.classes.id_of("RuntimeError")?;
        Some(mwl_runtime::ErrorClass::new(
            std::rc::Rc::clone(&self.classes),
            id,
        ))
    }

    /// Hands `ctx` this unit's class table. **Every embedder calls this before
    /// running any of the unit's code**, whether or not it cares about `catch`.
    ///
    /// Two obligations share the one call, and the second is the reason it is
    /// not optional:
    ///
    /// 1. *Behaviour.* A runtime helper's failure carries only a message; the
    ///    installed class is what promotes it to a catchable object with a
    ///    backtrace ([`mwl_runtime::Ctx::set_runtime_error_class`]).
    /// 2. *Safety.* Compiled code bakes each descriptor's address in as a
    ///    constant (see [`Classes`]), so an exception object still sitting on
    ///    the context points into this table and nothing else keeps it alive.
    ///    The handle installed here shares ownership of the table, which is
    ///    what makes a `Ctx` safe to outlive the `Unit` whose code it ran.
    ///    Skip the call and drop the `Unit` first, and `Ctx::pending` reads
    ///    freed memory — a use-after-free with no `unsafe` at the call site.
    pub fn install_in(&self, ctx: &mut mwl_runtime::Ctx) {
        if let Some(class) = self.runtime_error_class() {
            ctx.set_runtime_error_class(class);
        }
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
    let mut jit = Jit::new(None)?;
    jit.compile_all(program)?;
    jit.finish()
}

/// Compiles every function in `program` and returns the generated machine
/// code as text, one section per function, *instead* of a callable [`Unit`].
///
/// This is what `mwl run --dump-asm` prints. It compiles through exactly the
/// same path [`compile`] does — same ISA flags, same emitted probes — with
/// Cranelift's disassembler switched on, so what it prints is the code that
/// would have run rather than a second, differently-configured rendering.
/// Nothing is executed.
///
/// # Errors
///
/// The same three cases [`compile`] reports, for the same reasons.
pub fn disassemble(program: &Program) -> Result<String, CodegenError> {
    let mut jit = Jit::new(Some(String::new()))?;
    jit.compile_all(program)?;
    // `finish` still has to run: `finalize_definitions` is what resolves the
    // relocations, and a unit that cannot be linked is not a unit whose
    // disassembly should be reported as if it were fine.
    let disasm = jit.disasm.take().unwrap_or_default();
    jit.finish()?;
    Ok(disasm)
}

/// The JIT module under construction, plus the imported runtime symbols every
/// compiled function shares.
struct Jit {
    module: JITModule,
    ctx: codegen::Context,
    fn_ctx: FunctionBuilderContext,
    sigs: Signatures,
    /// Every function this unit defines, by its MWL name — the table
    /// `mwl_ir::ir::InstKind::Call`'s `"Class::method"` target is resolved
    /// through. Filled in a declaration pass over the whole program before
    /// any body is emitted, so a call may name a function defined later in
    /// the unit (or itself).
    functions: FxHashMap<String, cranelift_module::FuncId>,
    /// Every class the unit declares — the descriptors compiled code points
    /// at, and the field-slot index every `FieldGet`/`FieldSet` resolves
    /// through.
    classes: Classes,
    /// One entry per emitted `ConstStr`, so data-object names stay unique.
    literals: usize,
    entries: Vec<(String, cranelift_module::FuncId)>,
    /// Set only by [`disassemble`]: the accumulated text of every function's
    /// generated code. `None` is the ordinary compile, which asks Cranelift
    /// for no disassembly at all and so pays nothing for this field.
    disasm: Option<String>,
}

/// Every class the compiled unit declares, in the two forms emitted code
/// needs: the `ClassDesc` address `mwl_object_new`/`mwl_object_instanceof`
/// take, and the field-slot index a `FieldGet`/`FieldSet` turns into an
/// offset through [`mwl_runtime::field_offset`].
///
/// # Why the descriptor address is baked in as a constant
///
/// A JIT compiles at run time, so it *knows* the address of a runtime object
/// it has already built — there is nothing to relocate and no registry to
/// consult. `new Foo()` therefore emits one `iconst` and one call, which is
/// why `mwl_runtime::ClassDesc` needs no `#[repr(C)]` and no layout compiled
/// code agrees on: it is an opaque token.
///
/// The [`Unit`] that owns the table must outlive that code — see its own
/// `_classes` field.
#[derive(Debug, Default)]
struct Classes {
    table: mwl_runtime::ClassTable,
    by_label: FxHashMap<String, ClassEntry>,
    /// The same keys as `by_label`, holding the table id `ClassTable::define`
    /// needs for a parent. Separate because a descriptor address is what
    /// *compiled code* wants and an id is what the table wants.
    ids: FxHashMap<String, mwl_runtime::ClassId>,
}

/// One class's compiled-in identity and field-slot map.
#[derive(Debug)]
struct ClassEntry {
    /// Address baked into the code that allocates or tests an instance.
    desc: *const mwl_runtime::ClassDesc,
    /// Field name to slot index. Built from the *flattened* order
    /// `mwl_ir::ir::Class::fields` carries, so a slot looked up through the
    /// declaring class is valid for every subclass.
    slots: FxHashMap<String, usize>,
    /// This class's own id in `Classes::table`, and every method it answers as
    /// `(method name, declaring class label)` — kept until [`Jit::finish`],
    /// which is the first moment a compiled function has an address to put in
    /// the runtime descriptor's method table. See
    /// `mwl_runtime::ClassTable::set_methods`.
    id: mwl_runtime::ClassId,
    methods: Vec<(String, String)>,
}

impl Classes {
    /// Builds every descriptor, parents first.
    ///
    /// `mwl_ir::ir::Class::conforms` is already the *transitive* supertype
    /// set, so a class can be defined as soon as every label in it is —
    /// [`Self::define`] recurses to arrange exactly that. A `conforms` entry
    /// naming a class the unit does not declare is skipped rather than being
    /// an error: an `extends` the front end already diagnosed leaves one
    /// behind, and a second unexplained failure here would only bury the
    /// first.
    fn build(classes: &[mwl_ir::ir::Class]) -> Self {
        let mut out = Self::default();
        let by_label: FxHashMap<&str, &mwl_ir::ir::Class> = classes
            .iter()
            .map(|class| (class.label.as_str(), class))
            .collect();
        for class in classes {
            out.define(class, &by_label);
        }
        out
    }

    fn define(&mut self, class: &mwl_ir::ir::Class, source: &FxHashMap<&str, &mwl_ir::ir::Class>) {
        if self.by_label.contains_key(&class.label) {
            return;
        }
        // Reserve the label before recursing: a cyclic `extends` has already
        // been diagnosed by `mwl_hir::hierarchy`, and this must terminate
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
        if !class.codec.is_empty() {
            self.table
                .set_codec(id, class.codec.clone(), class.ctor_arity);
        }
        if !class.defaults.is_empty() {
            self.table.set_defaults(id, class.defaults.clone());
        }
        // ADR 0036 § 4's write check, at the one granularity the runtime can
        // hold: a representation with no single tag — `Ty::Tagged`, `Ty::Void`
        // — becomes `None`, which `mwl_runtime::mwl_object_slot_set` reads as
        // "unchecked". Every class with a layout carries one entry per slot
        // now, because § 4's erased receiver reaches any class at all; the
        // guard below is for the synthesized ones that carry none (a
        // closure's environment, a generator's state).
        if class.field_reprs.len() == class.fields.len() && !class.field_reprs.is_empty() {
            let tags = class
                .field_reprs
                .iter()
                .map(|ty| crate::ty::tag_of(*ty).ok())
                .collect();
            self.table.set_field_tags(id, tags);
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
            },
        );
    }

    /// The descriptor address for `label`, or `None` if the unit declares no
    /// such class.
    fn desc(&self, label: &str) -> Option<*const mwl_runtime::ClassDesc> {
        self.by_label.get(label).map(|entry| entry.desc)
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
/// `mwl-runtime`'s entry points are deliberately not all the same shape:
/// `mwl_str_concat`/`mwl_str_concat_n`/`mwl_str_retain`/`mwl_str_release`
/// operate on
/// raw `StrHeader` pointers with no context and no `Value`, because they are
/// memory primitives rather than language operations, and `mwl_probe_stmt`
/// returns nothing because a coverage probe cannot fail. Each therefore gets
/// its own signature here rather than being forced through
/// [`Signatures::helper`].
struct Signatures {
    /// ADR 0002's calling convention — `(ctx, args, out) -> status`.
    helper: Signature,
    /// `mwl_safepoint(ctx) -> status`.
    safepoint: Signature,
    /// `mwl_stack_check(ctx, sp) -> status` —
    /// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1's
    /// slow path. `sp` is `I64` for the reason every other pointer-shaped
    /// parameter here is: this JIT compiles for 64-bit targets only.
    stack_check: Signature,
    /// `mwl_probe_stmt(ctx, stmt_id)`.
    probe: Signature,
    /// `mwl_probe_call_enter(ctx, name, len)`.
    probe_call: Signature,
    /// `mwl_probe_call_exit(ctx, name, len, status)`.
    probe_call_exit: Signature,
    /// `mwl_str_concat(lhs, rhs) -> *mut StrHeader`,
    /// `mwl_str_append(target, suffix) -> *mut StrHeader` and
    /// `mwl_str_concat_n(pieces, count) -> *mut StrHeader`, which are all the
    /// same shape: two pointer-width parameters, one pointer back. The three
    /// differ in ownership and in what the second parameter *means*, not in
    /// ABI — see `mwl_ir::ir::InstKind::StrAppend` and `InstKind::Concat` — so
    /// one signature serves all of them, and a count declares itself with
    /// `AbiParam::new(ptr)` because a `usize` is pointer-width.
    str_concat: Signature,
    /// `mwl_str_eq(lhs, rhs) -> bool` and `mwl_array_eq(lhs, rhs) -> bool`,
    /// which share one shape: two raw pointers to an `I8`, like
    /// `Sigs::instanceof`.
    ptr_eq: Signature,
    /// `mwl_float_pow(base, exponent) -> f64` — ADR 0007 § 4's `**` over two
    /// `float`s, which has no machine instruction and no `LibCall` either. See
    /// [`crate::emit`]'s module doc for why it is a direct call of this shape
    /// rather than one more [`Signatures::helper`].
    float_pow: Signature,
    /// `mwl_str_retain(ptr)` / `mwl_str_release(ptr)`, and the two
    /// `mwl_throwable_*` counterparts.
    refcount: Signature,
    /// `mwl_value_retain(tag_word, bits)` / `mwl_value_release(tag_word, bits)`
    /// — the tag-dispatching pair a `mwl_ir::Ty::Tagged` operand needs, taking
    /// the register pair `crate::ty::clif_ty` describes as two words rather
    /// than one 16-byte aggregate, so no C ABI question about how such an
    /// aggregate travels ever arises.
    value_refcount: Signature,
    /// `mwl_exception_new(ptr) -> ptr`, and every other exception primitive
    /// with that one shape: `mwl_throwable_message`, `mwl_throwable_trace`,
    /// `mwl_take_thrown`.
    ptr_to_ptr: Signature,
    /// `mwl_raise(ctx, throwable)`.
    raise: Signature,
    /// `mwl_raise_new(ctx, class, message, len)` — the throw compiled code
    /// raises by itself, with no MWL `new` behind it. See
    /// `mwl_runtime::mwl_raise_new`.
    raise_new: Signature,
    /// `mwl_object_instanceof(object, desc) -> bool` — `I8`, the width a
    /// Cranelift comparison produces and the one [`ty::clif_ty`] gives
    /// [`mwl_ir::Ty::Bool`].
    instanceof: Signature,
    /// `mwl_class_method(class, name, len, fallback) -> code address` — the
    /// runtime half of `static::method(...)`'s dispatch. See
    /// `mwl_runtime::mwl_class_method`.
    class_method: Signature,
    /// `mwl_object_slot_get(ctx, object, name, len, hint, out) -> status` —
    /// ADR 0036 § 4's name-keyed shape read. The one object access that is not
    /// a fixed offset resolved here, and the one that can throw; see
    /// `mwl_ir::ir::InstKind::SlotGet`.
    slot_get: Signature,
    /// `mwl_object_slot_set(ctx, object, name, len, hint, value, out) -> status`
    /// — ADR 0036 § 4's name-keyed shape *write*. One parameter wider than
    /// [`Self::slot_get`], because the value travels through a caller-owned
    /// 16-byte slot the way [`Self::array_get`]'s result does *and* the helper
    /// ABI still writes an (ignored) result of its own; see
    /// `mwl_ir::ir::InstKind::SlotSet`.
    slot_set: Signature,
    /// `mwl_array_new() -> *mut ArrayHeader`.
    array_new: Signature,
    /// `mwl_array_get(array, key, out)` — the read primitive, whose result
    /// travels through a caller-owned 16-byte slot rather than by value; see
    /// `mwl_runtime::array`'s "the primitives compiled code calls" note for
    /// why no `Value` crosses this boundary in a register.
    array_get: Signature,
    /// `mwl_array_get_index(array, index, out)` — [`Self::array_get`] reached
    /// by the `i64` an `int` subscript already was, with no key string built
    /// at all while the array is packed. `mwl_ir::ir::InstKind::ArrayGet`'s
    /// key operand says which of the two applies, and `mwl-ir`'s module doc
    /// § *an array key is a `string`, and an `int` subscript no longer spells
    /// it* is the decision.
    array_get_index: Signature,
    /// `mwl_array_set(array, key, value) -> *mut ArrayHeader`.
    array_set: Signature,
    /// `mwl_array_set_index(array, index, value) -> *mut ArrayHeader` — the
    /// write half of [`Self::array_get_index`].
    array_set_index: Signature,
    /// `mwl_array_append(ctx, array, value, out) -> status` — the one array
    /// write that can fail, and so the one carrying ADR 0002's status shape
    /// rather than handing the array straight back. The array it yields
    /// travels through `out`, a caller-owned pointer-wide slot, the way
    /// [`Self::slot_set`]'s result travels through a 16-byte one;
    /// `mwl_runtime::mwl_array_append` owns what `out` holds on the refusal
    /// and why the refusal exists.
    array_append: Signature,
    /// `mwl_array_unset(array, key) -> *mut ArrayHeader` — two pointers in,
    /// one pointer back.
    ///
    /// It borrowed [`Self::array_append`]'s signature while the two shapes
    /// happened to agree, which is a call this crate would have miscompiled in
    /// silence the moment that one grew its fault channel. It has its own now,
    /// and no signature here is shared by two symbols whose Rust declarations
    /// are not the same shape for the same reason.
    array_unset: Signature,
    /// `mwl_array_next_slot(array, from) -> i64` — the `foreach` cursor step.
    /// `from` is a `usize` in the Rust signature, `I64` here: every target
    /// this JIT compiles for is 64-bit (see [`crate::ty::clif_ty`], which maps
    /// every pointer-shaped representation to `I64` for the same reason).
    array_next_slot: Signature,
    /// `mwl_array_key_at(array, slot) -> *mut StrHeader`.
    array_key_at: Signature,
    /// `mwl_array_value_at(array, slot, out)` — the read primitive whose
    /// result travels through a caller-owned 16-byte slot, exactly like
    /// [`Self::array_get`].
    array_value_at: Signature,
}

impl Jit {
    fn new(disasm: Option<String>) -> Result<Self, CodegenError> {
        let mut flags = settings::builder();
        for (name, value) in [
            // A JIT resolves every call through an absolute address, and the
            // pages are its own — the same configuration benches/abi-probe has
            // measured MWL's costs under since M0.
            ("use_colocated_libcalls", "false"),
            ("is_pic", "false"),
            ("opt_level", "speed"),
            // Cranelift defaults this **off**, and off means a frame larger
            // than the guard page can move the stack pointer past it in one
            // step and write into whatever lies beyond — a stack clash, which
            // is a memory-safety bug rather than the clean crash a guard page
            // exists to produce. `probestack_size_log2` defaults to 12, so a
            // probe is emitted only for a frame over 4 KiB and no MWL frame is
            // that big today: measured under callgrind on this tree, the
            // retired-instruction count is unchanged to five significant
            // figures either way (92,237,951 off vs 92,237,800 inline on a
            // call-heavy fixture; 55,399,358 vs 55,399,652 on a 200-deep
            // recursion). It is therefore insurance bought for nothing, and
            // the frame that would need it is exactly the one nobody predicts.
            //
            // **Not the same mechanism as ADR 0020 § 1's call-stack limit**,
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
            // the first time one is compiled, which measured as roughly fifty
            // consecutive `echo`s at a script's file scope. `inline` emits the
            // probe loop into the frame itself and needs no symbol, so the
            // guarantee above is the one actually in force.
            ("probestack_strategy", "inline"),
        ] {
            flags
                .set(name, value)
                .map_err(|e| CodegenError::UnsupportedHost(e.to_string()))?;
        }

        let isa = cranelift_native::builder()
            .map_err(|e| CodegenError::UnsupportedHost(e.to_owned()))?
            .finish(settings::Flags::new(flags))
            .map_err(|e| CodegenError::UnsupportedHost(e.to_string()))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        // Two symbol tables, one namespace: `mwl_runtime`'s primitives and
        // helpers, and every Tier 0 `Core` member. Both have ADR 0002's one
        // helper signature, and `mwl_stdlib`'s own docs own why a `Core` call
        // is emitted through the same path a helper call is.
        for (name, address) in mwl_runtime::symbols()
            .into_iter()
            .chain(mwl_stdlib::symbols())
        {
            builder.symbol(name, address);
        }

        let module = JITModule::new(builder);
        let sigs = Signatures::new(&module);
        Ok(Self {
            ctx: module.make_context(),
            fn_ctx: FunctionBuilderContext::new(),
            module,
            sigs,
            functions: FxHashMap::default(),
            classes: Classes::default(),
            literals: 0,
            entries: Vec::new(),
            disasm,
        })
    }

    /// Declares every function in `program`, then emits every body.
    ///
    /// The two passes are why a call can name a function declared further
    /// down the file, or itself: by the time any body is emitted, every MWL
    /// name in the unit already has a `FuncId` for `emit_call` to resolve
    /// against. Cranelift is fine with a call to a declared-but-not-yet-
    /// defined function; `finalize_definitions` is what would object if one
    /// were never defined.
    fn compile_all(&mut self, program: &Program) -> Result<(), CodegenError> {
        self.classes = Classes::build(&program.classes);
        for (index, function) in program.functions.iter().enumerate() {
            // `index` only disambiguates the Cranelift symbol name: an MWL
            // function name is not a valid symbol (`<script>` is the first
            // counter-example), and two classes may declare the same method
            // name.
            let symbol = format!("mwl{index}_{}", sanitize(&function.name));
            let id = self
                .module
                .declare_function(&symbol, Linkage::Local, &self.sigs.helper)
                .map_err(|source| CodegenError::Cranelift {
                    function: function.name.clone(),
                    source: Box::new(source),
                })?;
            self.functions.insert(function.name.clone(), id);
        }
        for function in &program.functions {
            self.compile_function(function)?;
        }
        Ok(())
    }

    /// Emits one already-declared function's body.
    fn compile_function(&mut self, function: &mwl_ir::Function) -> Result<(), CodegenError> {
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
                literals: &mut self.literals,
            },
            function,
        );
        if let Err(error) = result {
            self.module.clear_context(&mut self.ctx);
            return Err(error);
        }

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
            _module: self.module,
            classes: std::rc::Rc::new(self.classes.table),
            entries,
        })
    }

    /// Hands every runtime `ClassDesc` the compiled addresses of the methods
    /// its class answers — what `mwl_ir::ir::InstKind::CallVirtual` and
    /// `InstKind::NewDynamic` dispatch through.
    ///
    /// Runs only after `finalize_definitions`, because that is the first
    /// moment a compiled function has an address at all. A `(method,
    /// declaring class)` pair naming a function the unit does not define is
    /// skipped rather than being an error: the same treatment `Classes::define`
    /// gives a `conforms` entry it cannot resolve, and for the same reason —
    /// the front end has already reported whatever left it behind, and
    /// `mwl_class_method`'s fallback keeps the call correct regardless.
    fn bind_method_tables(&mut self) {
        for entry in self.classes.by_label.values() {
            let methods = entry
                .methods
                .iter()
                .filter_map(|(method, declaring)| {
                    let id = self.functions.get(&format!("{declaring}::{method}"))?;
                    Some((method.clone(), self.module.get_finalized_function(*id)))
                })
                .collect();
            self.classes.table.set_methods(entry.id, methods);
        }
    }
}

impl Signatures {
    fn new(module: &JITModule) -> Self {
        let ptr = types::I64;

        let mut helper = module.make_signature();
        helper.params.push(AbiParam::new(ptr)); // ctx
        helper.params.push(AbiParam::new(ptr)); // args
        helper.params.push(AbiParam::new(ptr)); // out
        helper.returns.push(AbiParam::new(types::I32)); // status

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
        raise.params.push(AbiParam::new(ptr));
        raise.params.push(AbiParam::new(ptr));

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
        slot_get.params.push(AbiParam::new(ptr)); // object
        slot_get.params.push(AbiParam::new(ptr)); // field name bytes
        slot_get.params.push(AbiParam::new(ptr)); // field name length
        slot_get.params.push(AbiParam::new(types::I64)); // slot hint
        slot_get.params.push(AbiParam::new(ptr)); // out
        slot_get.returns.push(AbiParam::new(types::I32));

        let mut slot_set = module.make_signature();
        slot_set.params.push(AbiParam::new(ptr)); // ctx
        slot_set.params.push(AbiParam::new(ptr)); // object
        slot_set.params.push(AbiParam::new(ptr)); // field name bytes
        slot_set.params.push(AbiParam::new(ptr)); // field name length
        slot_set.params.push(AbiParam::new(types::I64)); // slot hint
        slot_set.params.push(AbiParam::new(ptr)); // value
        slot_set.params.push(AbiParam::new(ptr)); // out
        slot_set.returns.push(AbiParam::new(types::I32));

        let mut array_new = module.make_signature();
        array_new.returns.push(AbiParam::new(ptr));

        let mut array_get = module.make_signature();
        array_get.params.push(AbiParam::new(ptr)); // array
        array_get.params.push(AbiParam::new(ptr)); // key
        array_get.params.push(AbiParam::new(ptr)); // out

        let mut array_set = array_get.clone();
        array_set.returns.push(AbiParam::new(ptr));

        // Spelled out rather than cloned from the key-taking pair: the middle
        // parameter is an `i64` index in the Rust signature, and only happens
        // to share `ptr`'s machine type on every target this crate builds for.
        let mut array_get_index = module.make_signature();
        array_get_index.params.push(AbiParam::new(ptr)); // array
        array_get_index.params.push(AbiParam::new(types::I64)); // index
        array_get_index.params.push(AbiParam::new(ptr)); // out

        let mut array_set_index = array_get_index.clone();
        array_set_index.returns.push(AbiParam::new(ptr));

        let mut array_append = module.make_signature();
        array_append.params.push(AbiParam::new(ptr)); // ctx
        array_append.params.push(AbiParam::new(ptr)); // array
        array_append.params.push(AbiParam::new(ptr)); // value
        array_append.params.push(AbiParam::new(ptr)); // out
        array_append.returns.push(AbiParam::new(types::I32));

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
            slot_set,
            array_new,
            array_get,
            array_get_index,
            array_set,
            array_set_index,
            array_append,
            array_unset,
            array_next_slot,
            array_key_at,
            array_value_at,
        }
    }
}

/// Reduces an MWL function name to something a linker symbol may contain.
///
/// Not a mangling scheme: [`Jit::compile_function`]'s index already supplies
/// uniqueness, so this only has to keep the name readable in a disassembly.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}
