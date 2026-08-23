//! The CFG/SSA IR itself: a [`Program`] of [`Function`]s, each a graph of
//! [`BasicBlock`]s of SSA [`Inst`]ructions ending in exactly one
//! [`Terminator`]. See the crate's own module docs for this first slice's
//! scope and known gaps.

use mwl_diagnostics::Span;

use crate::ids::{BlockId, EdgeId, StmtId, ValueId};
use crate::ty::Ty;

/// Every function this compilation unit lowered, and every class it can
/// instantiate.
#[derive(Debug, Default)]
pub struct Program {
    /// The lowered functions, in no particular order.
    pub functions: Vec<Function>,
    /// Every class and interface declared in the unit, in no particular
    /// order — see [`Class`].
    pub classes: Vec<Class>,
}

/// One class's or interface's runtime shape: what an instance's field slots
/// hold, and what it is an instance *of*.
///
/// A straight copy of `mwl_types::layout::ClassLayout`, carried here so that
/// `mwl-codegen` can build the `mwl_runtime::ClassTable` a compiled unit owns
/// without depending on `mwl-types`. The resolution behind it — a subclass's
/// slots following its parent's, a transitive supertype set — happens once, in
/// that module, for the reason its own docs give: it needs `mwl_hir::ClassGraph`,
/// and this crate deliberately depends on neither.
#[derive(Clone, Debug)]
pub struct Class {
    /// The class's rendered `Class`/`Ns\Class` label — the same spelling
    /// [`InstKind::New::class`] and [`InstKind::FieldGet::class`] carry, and
    /// the same one a method label's class half uses.
    pub label: String,
    /// Every field slot in index order: every ancestor's first, then this
    /// class's own in declaration order. `$`-sigil not included.
    pub fields: Vec<String>,
    /// Every *other* class and interface an instance of this one also is,
    /// transitively, as labels. Excludes the class itself.
    pub conforms: Vec<String>,
    /// Every method an instance of this class answers, as `(method name,
    /// declaring class label)` — a straight copy of
    /// `mwl_types::layout::ClassLayout::methods`, which owns the precedence
    /// rule. `mwl-codegen` turns each pair into the compiled address the
    /// runtime descriptor's method table holds, which is what
    /// [`InstKind::CallVirtual`] dispatches through.
    pub methods: Vec<(String, String)>,
}

/// One lowered method or function.
#[derive(Debug)]
pub struct Function {
    /// The function's name, for diagnostics and the text listing —
    /// `crate::lower::lower_method`'s caller decides how it is qualified
    /// (bare method name, `Class::method`, ...).
    pub name: String,
    /// Each parameter's representation, positional. For a *method*, index 0
    /// is always the implicit receiver (`$this`) — every lowered method
    /// carries it, whether or not its body ever reads `$this`, mirroring
    /// `mwl_types::check.rs`'s `check_method` seeding `$this` into its own
    /// `LocalScope` unconditionally (not gated on a `static` modifier — see
    /// that function's own comment for why). Every explicit
    /// `MethodMember` parameter follows, starting at index 1. See
    /// `crate::lower::lower_method`'s own doc comment for where the
    /// receiver's value comes from.
    ///
    /// A *script body* — `crate::lower::lower_script`'s synthesized frame for
    /// a file's own top-level statements — has no receiver and no
    /// parameters at all, so this is empty for one.
    pub params: Vec<Ty>,
    /// The return representation, [`Ty::Void`] for a `void`-returning
    /// method.
    pub ret: Ty,
    /// The function's basic blocks. A straight-line body still produces
    /// exactly one; `if`/`while` each add the blocks their join point needs
    /// (see `crate::lower`'s module docs).
    pub blocks: Vec<BasicBlock>,
    /// Which block execution starts in.
    pub entry: BlockId,
    /// The source span [`crate::ids::StmtId::index`] names — see
    /// [`crate::ids`]'s own module docs on why this table, not a hash map,
    /// backs the lookup.
    pub stmt_spans: Vec<Span>,
    /// The source span [`crate::ids::EdgeId::index`] names. Non-empty for
    /// any function containing an `if`/`while`.
    pub edge_spans: Vec<Span>,
}

/// One basic block: a straight-line run of [`Inst`]ructions ending in one
/// [`Terminator`].
#[derive(Debug)]
pub struct BasicBlock {
    /// This block's own id.
    pub id: BlockId,
    /// The block's instructions, in execution order.
    pub insts: Vec<Inst>,
    /// How the block ends.
    pub term: Terminator,
}

/// One SSA instruction. Not every instruction defines a value — a
/// [`InstKind::StmtMarker`] never does, the same way a future `void`-returning
/// call would not.
#[derive(Debug)]
pub struct Inst {
    /// The SSA value this instruction defines, if any.
    pub result: Option<ValueId>,
    /// `result`'s representation, present exactly when `result` is.
    pub ty: Option<Ty>,
    /// What the instruction does.
    pub kind: InstKind,
    /// [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s error edge:
    /// the landing block a non-`OK` status returned by this instruction
    /// branches to.
    ///
    /// `Some` for the instructions that can actually fail: [`InstKind::Call`],
    /// [`InstKind::CallVirtual`], [`InstKind::New`],
    /// [`InstKind::NewDynamic`], and the one [`InstKind::HelperCall`] with a real
    /// failure mode, [`Helper::EchoStr`]'s write. `None` everywhere else,
    /// which is not a gap in two different ways — a [`InstKind::BinOp`] or a
    /// [`InstKind::Concat`] returns no status at all, and a *conversion*
    /// helper (`Helper::IntToString`, the truthy table) returns one whose only
    /// non-`OK` value is the miscompile guard `mwl_runtime::helpers` describes:
    /// a `FATAL`, which no cleanup path and no `catch` can act on, so giving
    /// it a landing block would emit code for an outcome that ends the request
    /// regardless.
    ///
    /// The consequence is stated rather than hidden: a `FATAL` raised inside a
    /// frame does not release that frame's locals. A `THROWN` — the one a
    /// program can produce and recover from — always does.
    ///
    /// The landing block it names holds exactly the
    /// [`InstKind::Release`]s this frame owes on the error path — the frame's
    /// live refcounted locals at this instruction's own program point — and
    /// ends in [`Terminator::Propagate`] or [`Terminator::Catch`]. It is one
    /// block per call site rather than one shared per region, so a `catch`
    /// handler's phis get a distinct predecessor per site; see
    /// `crate::lower::Lowering::landing_block`.
    pub on_error: Option<BlockId>,
}

/// What one [`Inst`] does.
#[derive(Debug)]
#[non_exhaustive]
pub enum InstKind {
    /// ADR 0018 § 1's statement-boundary probe site: marks where the
    /// previous lowered statement's instructions end and the next one's
    /// begin, in program order. Defines no value.
    StmtMarker(StmtId),
    /// A reserved safepoint poll site — function entry (recursion) or a
    /// loop's back edge, the two sites the project-start "safepoints emitted
    /// from the first backend commit" decision names
    /// ([`docs/adr/README.md`](../../../docs/adr/README.md)'s "Decisions
    /// taken at project start" section) and that ADR 0018 § *Negative*
    /// contrasts its own, denser probe grid against. This slice reserves the
    /// shape only — no codegen exists yet to lower it to an actual CPU-limit/
    /// cancellation/cycle-collector check, and no guard test needs it
    /// functional before M3 builds the backend on top of this IR. Reserved
    /// now rather than later for the same "cheap now, expensive to
    /// retrofit" reason `crate::ids` already gives for `StmtId`/`EdgeId`:
    /// inserting it after the fact would mean re-walking every already-
    /// lowered function. Defines no value.
    Safepoint,
    /// A `bool` constant.
    ConstBool(bool),
    /// An `int` constant.
    ConstInt(i64),
    /// A `uint` constant.
    ConstUint(u64),
    /// A `float` constant.
    ConstFloat(f64),
    /// A `string` literal's cooked bytes — a fresh [`Ty::Str`] value with
    /// exactly one implicit owner (itself), the same "one natural reference"
    /// starting point [`InstKind::New`] gives a freshly constructed object.
    /// See `crate::lower::cook_str_literal`'s own doc comment for exactly
    /// which escape sequences are cooked this slice and which are a known
    /// gap.
    ConstStr(String),
    /// Reads the function's own parameter at this positional index.
    Param(u32),
    /// A binary arithmetic or comparison operator over two already-lowered
    /// operands.
    BinOp {
        /// Which operator.
        op: BinOp,
        /// The left operand.
        lhs: ValueId,
        /// The right operand.
        rhs: ValueId,
    },
    /// A unary operator over one already-lowered operand.
    UnOp {
        /// Which operator.
        op: UnOp,
        /// The operand.
        operand: ValueId,
    },
    /// A phi node: selects the incoming value based on which predecessor
    /// block control arrived from. One entry per predecessor that can reach
    /// this instruction's own block — `if`/`while`'s join points are the
    /// first thing to construct one; see `crate::lower`'s module docs.
    /// A single-entry phi is a legal (if degenerate) case: a `while` body
    /// that never reaches its own back edge (e.g. it always returns) leaves
    /// the loop header's phi with only the pre-loop incoming edge.
    Phi {
        /// `(predecessor block, incoming value)` pairs.
        incoming: Vec<(BlockId, ValueId)>,
    },
    /// A statically resolved call — a static method call, `new`'s
    /// constructor invocation, or an instance method call
    /// (`$obj->method(...)`, including `$this->…`). The target is already
    /// fully resolved by `mwl_types::expr_table::ExprTypeTable` before
    /// lowering ever sees it — there is no virtual dispatch to model here,
    /// only "which function does this invoke." See the crate docs' "no
    /// virtual dispatch" known gap for what happens once a receiver's static
    /// and runtime types can actually differ.
    Call {
        /// The resolved target, rendered `"Class::method"` — a label for
        /// `crate::print`/a future codegen symbol table, not itself
        /// resolvable back to a `QName` (this crate never depends on
        /// `mwl-hir`; see `crate::lower`'s module docs).
        target: String,
        /// The receiver value, for an instance method call (`Some`) — `None`
        /// for a static call or `new`'s constructor invocation, neither of
        /// which has a receiver at all.
        receiver: Option<ValueId>,
        /// Each positional argument, already lowered.
        args: Vec<ValueId>,
    },
    /// `new Target(...)`: allocates an instance of `class` and, if it
    /// declares one, invokes its resolved `constructor` with `args` —
    /// bundled into one instruction rather than a separate allocation plus
    /// `InstKind::Call`, since no codegen exists yet to make that split
    /// observable (M3's backend work, once real object layout exists) and
    /// splitting it now would be a distinction with nothing to attach it to.
    New {
        /// The constructed class, rendered the same way `Call::target` is —
        /// the key into [`crate::ir::Program::classes`].
        class: String,
        /// The resolved `Class::constructor` label to invoke on the fresh
        /// instance, or `None` for a class that declares none anywhere in its
        /// chain.
        ///
        /// Carried separately from `class` because the two differ whenever a
        /// subclass inherits its constructor: `new Dog(...)` allocates a `Dog`
        /// and calls `Animal::constructor`. Only `mwl_types` knows which
        /// class actually declares it, so recovering it downstream would mean
        /// re-walking a hierarchy this crate cannot see.
        ctor: Option<String>,
        /// Each constructor argument, already lowered — empty when `ctor` is
        /// `None`.
        args: Vec<ValueId>,
    },
    /// The [`Ty::ClassDesc`] of a class named in source — `Foo::bar()`'s
    /// `Foo`, or the enclosing class for `self::`/`parent::`. One `iconst` of
    /// the descriptor address, exactly what [`InstKind::New`] already bakes in
    /// for its own class.
    ///
    /// This is late static binding's *non*-forwarding source: `Foo::bar()`
    /// sets the called class to `Foo` regardless of where the call is written,
    /// which is PHP's own rule and the reason `self::`/`static::`/`parent::`
    /// forward the caller's instead of producing one of these.
    ClassDescConst {
        /// The class, rendered the same way [`InstKind::New::class`] is.
        class: String,
    },
    /// The [`Ty::ClassDesc`] of the class `object` is actually an instance of
    /// — one load at `mwl_runtime::OBJ_CLASS_OFFSET`, retaining nothing (a
    /// descriptor is owned by the unit, not reference counted).
    ///
    /// This is where an *instance* method gets its late-static-binding class
    /// from: `$leaf->label()` enters `Registry::label` with `$this` pointing
    /// at a `LeafRegistry`, so `static::` inside it means `LeafRegistry`
    /// without anything being passed at the call site.
    ClassDescOf {
        /// The receiver, already lowered — a [`Ty::Object`].
        object: ValueId,
    },
    /// `static::method(...)` — a call whose *target* is decided at run time by
    /// the late-static-binding class, not by the checker.
    ///
    /// Lowers to a `mwl_runtime::mwl_class_method` lookup of `method` on
    /// `lsb`, falling back to `fallback` (the label `mwl_types` statically
    /// resolved, which a class the unit compiled no method table for still
    /// needs), then an indirect call through the ordinary
    /// [ADR 0002](../../../docs/adr/0002-error-propagation.md) signature — so
    /// every probe, status check and landing block is identical to
    /// [`InstKind::Call`]'s. Ownership is identical too: `receiver` and each
    /// argument are transferred, and the callee releases them.
    ///
    /// Deliberately *not* what an ordinary `$obj->method(...)` lowers to: that
    /// stays statically resolved (`mwl-codegen`'s known gap 1), so nothing on
    /// the hot path pays for a name lookup. See `mwl_runtime::object`'s docs
    /// for why the table is keyed by name and what real virtual dispatch would
    /// want instead.
    CallVirtual {
        /// The late-static-binding class to dispatch on — a
        /// [`Ty::ClassDesc`].
        lsb: ValueId,
        /// The method's own name, the key into the runtime method table.
        method: String,
        /// The statically resolved `"Class::method"` label, used when `lsb`
        /// answers nothing for `method` — `None` when the resolved
        /// declaration has no body at all (an `abstract` method, or a
        /// bodiless interface one), which names no compiled function to fall
        /// back to. Codegen substitutes `mwl_runtime::mwl_abstract_method`,
        /// so a miss is a reported `FATAL` rather than a jump through null.
        fallback: Option<String>,
        /// The receiver value for an instance target (`Some`) — `None` for a
        /// `static` one, whose slot 0 carries `lsb` instead, exactly the way
        /// [`InstKind::Call`]'s does.
        receiver: Option<ValueId>,
        /// Each positional argument, already lowered.
        args: Vec<ValueId>,
    },
    /// `new static(...)` — [`InstKind::New`] with the class taken from a
    /// [`Ty::ClassDesc`] value rather than a label, and the constructor
    /// dispatched through the same runtime method table
    /// [`InstKind::CallVirtual`] uses.
    ///
    /// This is the instruction M4's acceptance names: `new static()` reached
    /// through two levels of inheritance allocates the *called* class.
    NewDynamic {
        /// The class to allocate — a [`Ty::ClassDesc`].
        desc: ValueId,
        /// The `Class::constructor` label `mwl_types` resolved for the
        /// statically known class, or `None` if nothing in that chain declares
        /// one. Used as the lookup's fallback; the lookup itself always asks
        /// the allocated class first, so a subclass's own constructor wins.
        ctor: Option<String>,
        /// Each constructor argument, already lowered — empty when `ctor` is
        /// `None`.
        args: Vec<ValueId>,
    },
    /// Reads a compile-time-known field off an object — `$obj->prop` whose
    /// receiver's static type resolved to a known declaring class (an
    /// `mwl_types::expr_table::ExprInfo::Property` entry exists for it). No
    /// actual byte offset is computed here: `class`/`field` are labels for a
    /// future codegen layout pass, the same "resolved identity, not yet a
    /// machine offset" shape `Call`/`New`'s own `target`/`class` labels
    /// already use. A property access whose receiver erased to a shape or
    /// plain `object` (ADR 0036 § 4) has no such entry to read at all —
    /// `crate::lower` panics naming that case rather than lowering it; see
    /// the crate docs' known gaps for why (the checker itself defers the
    /// runtime-checked fallback to M4, with no IR/codegen yet to throw from).
    FieldGet {
        /// The receiver, already lowered.
        object: ValueId,
        /// The class that actually declares the field, rendered the same way
        /// `Call::target`'s class half is.
        class: String,
        /// The field's own name, `$`-sigil not included.
        field: String,
    },
    /// Writes a compile-time-known field on an object — `$obj->prop = expr;`
    /// whose receiver's static type resolved to a known declaring class,
    /// exactly the same [`ExprInfo::Property`](mwl_types::expr_table::ExprInfo::Property)
    /// resolution [`InstKind::FieldGet`] already relies on for a read (see
    /// that variant's own doc comment for why `class`/`field` stay labels
    /// rather than a machine offset, and for the erased-receiver panic case
    /// this shares). `crate::lower::Lowering::lower_reassignment` emits this
    /// alongside a retain of `value` (if it's an aliasing read) and a release
    /// of whatever the field previously held, the same refcounting policy
    /// [`Retain`](InstKind::Retain)/[`Release`](InstKind::Release)'s own doc
    /// comments describe for a local. Defines no value; operates on `object`
    /// in place.
    FieldSet {
        /// The receiver, already lowered.
        object: ValueId,
        /// The class that actually declares the field, rendered the same way
        /// `FieldGet::class` is.
        class: String,
        /// The field's own name, `$`-sigil not included.
        field: String,
        /// The new value, already lowered.
        value: ValueId,
    },
    /// `$obj instanceof Class` — one linear scan of the receiver's flattened
    /// supertype set, defining a [`Ty::Bool`].
    ///
    /// `class` is a label into [`crate::ir::Program::classes`], exactly like
    /// [`InstKind::New::class`], and it may name an *interface* as readily as
    /// a class: `mwl_types::layout` gives an interface a descriptor with no
    /// slots for precisely this test (and for a typed `catch`, which lowers
    /// to the same instruction). Reads `value` without retaining it, the way
    /// [`InstKind::FieldGet`] reads its receiver.
    InstanceOf {
        /// The receiver, already lowered — a [`Ty::Object`].
        value: ValueId,
        /// The class or interface tested against, rendered the same way
        /// `New::class` is.
        class: String,
    },
    /// `.` string concatenation: builds a fresh [`Ty::Str`] value from the
    /// cooked bytes of `lhs` and `rhs`, both already [`Ty::Str`] by the time
    /// this instruction sees them — see
    /// `crate::lower::Lowering::concat_operand`, which converts a scalar
    /// operand through an [`InstKind::HelperCall`] first, and still panics
    /// naming a `Stringable`-object operand (needs a resolved `toString`
    /// call this crate can't synthesize yet). Modeled as a dedicated
    /// instruction rather than a runtime-helper call itself, the same
    /// "native instruction over already-typed operands" treatment
    /// [`InstKind::BinOp`] already gives scalar arithmetic — `.` only ever
    /// needs this one fixed two-operand shape, unlike the open-ended,
    /// enum-tagged set [`HelperCall`](InstKind::HelperCall) exists for. The
    /// result is a fresh value with exactly one natural owner —
    /// concatenation always allocates a new buffer, so
    /// `crate::lower::is_aliasing_read` stays `false` for `ExprKind::Binary`,
    /// same as it already is for [`InstKind::ConstStr`]/[`InstKind::New`]/
    /// [`InstKind::Call`]. Neither operand is retained by this instruction
    /// itself: each is only *read* to build the new buffer, exactly the way
    /// [`InstKind::FieldGet`] reads its `object` receiver without retaining
    /// it, so ownership of `lhs`/`rhs` stays wherever it already was (their
    /// own local slot, field, ...) — and `crate::lower::Lowering::concat_operand`'s
    /// caller releases either operand right after this instruction reads it
    /// when that operand was never such a slot to begin with (a literal, a
    /// nested `Concat`'s own result, or a freshly converted
    /// [`HelperCall`](InstKind::HelperCall) result), since nothing else will
    /// ever release it otherwise.
    Concat {
        /// The left operand, already lowered and already [`Ty::Str`].
        lhs: ValueId,
        /// The right operand, already lowered and already [`Ty::Str`].
        rhs: ValueId,
    },
    /// Takes the pending exception out of the request context, transferring
    /// ownership of one reference to the [`crate::ty::Ty::Object`] this
    /// defines — the first instruction of a `catch`'s dispatch block, and the
    /// only way an MWL binding ever names an exception this frame did not
    /// construct itself.
    ///
    /// The value may be **null**: a runtime helper's bare-message failure has
    /// no object behind it unless the driver installed a class to build one
    /// from (`mwl_runtime::Ctx::set_runtime_error_class`). Every operation the
    /// dispatch performs on it tolerates that — [`InstKind::InstanceOf`]
    /// answers `false`, so no clause matches and the throw is re-raised.
    ///
    /// Defined as an instruction rather than a [`Helper`] call for the same
    /// reason [`InstKind::Concat`] is one: it has a single fixed shape, takes
    /// no MWL operand, and cannot fail — so it needs neither the argument
    /// list nor the status check `HelperCall` exists to carry.
    TakeThrown,
    /// Increments a [`Ty::is_refcounted`] value's reference count — emitted
    /// exactly where `crate::lower`'s "copy" case needs a second durable
    /// owner to see it stay alive (see that module's docs for the precise
    /// insertion policy and the syntactic "is this an aliasing read"
    /// judgment it's keyed on). Defines no value; operates on `operand`
    /// in place.
    Retain {
        /// The value being retained.
        operand: ValueId,
    },
    /// Decrements a [`Ty::is_refcounted`] value's reference count, freeing
    /// its heap allocation if it reaches zero — emitted where a durable
    /// slot's previous value is overwritten, and for every such slot still
    /// live when its owning function returns (except the one slot whose
    /// value is the return value itself, which transfers out instead — see
    /// `crate::lower::Lowering::release_all_locals`'s own doc comment).
    /// Defines no value; operates on `operand` in place. No codegen exists
    /// yet to lower this to an actual decrement-and-maybe-free sequence —
    /// reserved the same "shape now, functional once a backend exists" way
    /// [`InstKind::Safepoint`] already is.
    Release {
        /// The value being released.
        operand: ValueId,
    },
    /// `clone $obj` —
    /// [ADR 0023](../../../docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)
    /// § 1's shallow, same-heap, single-level copy. Defines a fresh
    /// [`crate::ty::Ty::Object`] with exactly one natural owner, exactly like
    /// [`InstKind::New`], whose every slot holds what the original's held with
    /// one more reference taken.
    ///
    /// No hook runs and nothing can fail: ADR 0023 has no `__clone`, so this
    /// carries no status check and no landing block, the same as
    /// [`InstKind::Concat`]. Reads `object` without retaining it, the way
    /// [`InstKind::FieldGet`] reads its receiver.
    Clone {
        /// The object being copied.
        object: ValueId,
    },
    /// Defines a value with the *same machine bits* as `operand` under a
    /// different [`crate::ty::Ty`] — the whole of a conversion that
    /// [ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md) § 5 calls
    /// "total, free ... same representation, reinterpreted": an enum to its
    /// backing `int`/`uint`, and nothing else so far.
    ///
    /// Emitted rather than simply relabelling the operand in `crate::lower`
    /// because an IR value's representation is a property of the instruction
    /// that *defined* it — two names for one definition would mean two
    /// answers to [`Inst::ty`] for the same [`crate::ids::ValueId`], and
    /// `mwl-codegen`'s value map has exactly one slot per id.
    ///
    /// The two representations must share a Cranelift type, which
    /// `mwl-codegen` asserts: this is a relabelling, never a bit cast, so it
    /// emits no machine instruction at all — the operand's own Cranelift
    /// value is recorded under the new id. It transfers no ownership and
    /// cannot fail, so like [`InstKind::Concat`] it carries no status check
    /// and no landing block.
    Reinterpret {
        /// The value being relabelled.
        operand: ValueId,
    },
    /// Invokes one of a small, closed, engine-owned set of runtime
    /// conversions — the milestone's third named ingredient, and this
    /// crate's first. `helper` is a fixed [`Helper`] tag, never a resolved
    /// class/method name: unlike [`InstKind::Call`]'s `target`, nothing here
    /// comes from `mwl_types::expr_table::ExprTypeTable` or a class
    /// hierarchy, so there is no receiver, no virtual dispatch question, and
    /// no reason to share `Call`'s shape (see `crate::lower`'s module docs'
    /// design-choices section for why a dedicated instruction was chosen
    /// over reusing `Call` with a synthetic target label, and an enum tag
    /// over a string name). Like [`InstKind::Call`]/[`InstKind::New`], this
    /// does not yet model ADR 0002's checked-return convention — no status,
    /// no error edge — since nothing in this crate models a call that can
    /// fail at all yet (no `try`/`throw` lowered); that is expected to land
    /// once such calls do, for every call-shaped instruction at once rather
    /// than only for this one.
    HelperCall {
        /// Which conversion.
        helper: Helper,
        /// Each argument, already lowered.
        args: Vec<ValueId>,
    },
    /// `[...]`/legacy `array(...)`: builds a fresh [`crate::ty::Ty::Array`]
    /// value — ADR 0007 § 5's insertion-ordered, string-keyed hash. A
    /// *purely positional* literal (no element has an explicit `key =>`)
    /// bundles the whole thing into one instruction from a fixed list of
    /// already-lowered `(key, value)` pairs, rather than an allocation plus a
    /// sequence of inserts — the same "no codegen exists yet to make the
    /// split observable" reasoning [`InstKind::New`]'s own doc comment
    /// already gives for a constructor call. Every `key` here is a decimal
    /// string computed at lowering time, not a lowered expression: each
    /// positional element's key is simply its index in the list, auto-
    /// numbered from `0` exactly like PHP's own `[$a, $b]` shorthand — known
    /// statically without a runtime index-tracking instruction. A literal
    /// with at least one explicit `key =>` element instead lowers to an
    /// *empty* `ArrayNew` (`entries` is `Vec::new()`) followed by a chain of
    /// [`InstKind::ArraySet`]s, one per element in source order, each writing
    /// into the value the last one defined —
    /// `crate::lower::Lowering::lower_array_key` gives every key (explicit or
    /// positional alike) a real `Ty::Str` `ValueId` there, which this fixed-
    /// entries shape has no field for. See
    /// `crate::lower::Lowering::lower_expr`'s `ArrayLiteral` arm for exactly
    /// which of the two shapes a given literal takes, and its own doc comment
    /// for the one PHP behavior that split deliberately does not reproduce: a
    /// positional element mixed after an explicit `int`/`uint` key still
    /// numbers from "how many positional elements came before it," not from
    /// PHP's real "highest integer key used so far" rule, since that needs
    /// the same "next available integer key" runtime counter `$a[]` append
    /// syntax is still waiting on (see the crate docs' known gaps). A
    /// `...spread` element and a `&value` element are both still unsupported
    /// either way — lowering panics naming whichever is used.
    ///
    /// The array itself is a fresh value with exactly one natural owner, the
    /// same starting point [`InstKind::New`]/[`InstKind::ConstStr`] already
    /// give — see `crate::lower::is_aliasing_read`, which stays `false` for
    /// `ExprKind::ArrayLiteral`. Each element that's itself
    /// [`crate::ty::Ty::is_refcounted`] and
    /// [`is_aliasing_read`](crate::lower::is_aliasing_read) (a bare local or a
    /// compile-time-known property read) is retained by
    /// `crate::lower::Lowering::lower_expr`'s `ArrayLiteral` arm *before* it
    /// is stored — the same caller-side retain
    /// `crate::lower::Lowering::lower_call_args` already inserts for a
    /// refcounted, aliasing call argument, since the array durably owns
    /// whatever it stores exactly the way a callee's parameter slot does. A
    /// fresh element (a literal, `new`, a nested array literal, or a call's
    /// own result) needs no retain: it already has exactly one natural owner,
    /// which simply transfers into the array's storage. An explicit key that
    /// [`is_aliasing_read`](crate::lower::is_aliasing_read) (a bare variable
    /// holding the key) gets the identical retain, mirroring
    /// `crate::lower::Lowering::lower_reassignment`'s `Index`-target arm.
    ArrayNew {
        /// `(key, value)` pairs, in insertion order — ADR 0007 § 5's
        /// "iteration order is insertion order, always." Always empty for a
        /// literal with an explicit `key =>` element; see above.
        entries: Vec<(String, ValueId)>,
    },
    /// Reads the element at `key` off `array` — `$arr[$i]`, whose base
    /// statically resolved to a known `array<T>` element type (an
    /// `mwl_types::expr_table::ExprInfo::Index` entry exists for it; see
    /// `crate::lower::Lowering::lower_expr`'s `Index` arm). `key` is already
    /// [`crate::ty::Ty::Str`] by the time this instruction sees it —
    /// `crate::lower::Lowering::lower_array_key` normalizes an `int`/`uint`
    /// subscript to its decimal-string form first (ADR 0007 § 5's key
    /// normalization, `$a[8]` is `$a["8"]`), reusing the exact
    /// [`Helper::IntToString`]/[`Helper::UintToString`] conversion
    /// [`Lowering::concat_operand`](crate::lower::Lowering::concat_operand)
    /// already gives `.`'s scalar operand rather than a new policy. Like
    /// [`InstKind::FieldGet`], this does not model what happens when `key`
    /// isn't actually present at runtime — PHP's own warning-and-`null`
    /// read — since no `try`/`throw` lowering exists yet to express a checked
    /// outcome (see the crate docs' known gaps); this instruction only
    /// models the happy path where the key is present. Reads `array` without
    /// retaining it, the same way `FieldGet` reads its `object` receiver — a
    /// caller copying the result into a second durable slot retains it
    /// there instead (`crate::lower::is_aliasing_read` now also matches
    /// `ExprKind::Index`, so the existing `bind_local`/`lower_call_args`/
    /// `StmtKind::Return` insertion points already do this with no new
    /// call site).
    ArrayGet {
        /// The array, already lowered.
        array: ValueId,
        /// The lookup key, already lowered and already `Ty::Str`.
        key: ValueId,
    },
    /// Writes `value` at `key` into `array` — `$arr[$i] = expr;`, inserting a
    /// fresh entry when `key` isn't already present and overwriting (per ADR
    /// 0007 § 5's copy-on-write value semantics, releasing whatever it
    /// already held) otherwise. Unlike [`InstKind::FieldSet`], which reads
    /// the field's *previous* value back with a [`InstKind::FieldGet`] before
    /// releasing it — safe there because a declared field always exists on a
    /// definitely-initialized instance (ADR 0022) — an array key may or may
    /// not already be present, so this instruction bundles the entire
    /// replace-or-insert operation rather than splitting it into a get/
    /// release pair the way `FieldSet` does: no codegen exists yet to make
    /// that split observable, and modeling a conditional get here would mean
    /// guessing at PHP's own missing-key behavior at the one place — an
    /// *ordinary* new-key insert — where nothing should be missing to begin
    /// with. `key` is already `Ty::Str`, normalized the same way
    /// [`InstKind::ArrayGet`]'s own doc comment describes.
    /// `crate::lower::Lowering::lower_reassignment`'s `Index`-target arm
    /// retains `key`/`value` first when either is
    /// [`crate::ty::Ty::is_refcounted`] and an aliasing read (the same
    /// caller-side retain a call argument/array-literal element already
    /// gets) — the array durably owns both after this instruction runs.
    ///
    /// **Defines a fresh [`crate::ty::Ty::Array`] value: the array that now
    /// holds the entry.** ADR 0007 § 5's copy-on-write value semantics mean a
    /// write into an array a second binding also holds must separate, which
    /// produces a *different* allocation — so this instruction consumes one
    /// reference to `array` and yields one reference to the result, which is
    /// the same pointer whenever `array` was solely owned. `mwl_runtime`'s
    /// `array` module owns that protocol and why it is the only shape open to
    /// a backend that keeps a local in an SSA register rather than a memory
    /// slot a callee could write back through. The consequence for lowering is
    /// that the *holder* of `array` — a local's `Env` binding, or the property
    /// slot an [`InstKind::FieldSet`] writes back into — is re-pointed at the
    /// result, with no retain or release of either: the consumed reference and
    /// the produced one are the holder's same one slot.
    ArraySet {
        /// The array, already lowered.
        array: ValueId,
        /// The key to write, already lowered and already `Ty::Str`.
        key: ValueId,
        /// The new value, already lowered.
        value: ValueId,
    },
    /// Appends `value` to `array` at PHP's own "next available integer key"
    /// — `$a[] = expr;`, ADR 0007 § 5's append syntax
    /// (`crate::lower::Lowering::lower_reassignment`'s `Index`-target arm,
    /// reached when the target's subscript is `None`). Unlike
    /// [`InstKind::ArraySet`], no key is lowered or carried here at all: PHP's
    /// real rule tracks "the highest `int` key ever used, plus one" as part of
    /// the array's own runtime state (surviving explicit `int`-keyed inserts,
    /// removals, and earlier appends alike), which is genuinely a property of
    /// the array value itself, not something a lowering pass can compute from
    /// the source text the way a literal's positional index or an explicit
    /// key already can. This instruction leaves that counter's storage and
    /// increment entirely to whatever `mwl-codegen`'s own array representation
    /// does with it — the same "shape now, functional once a backend exists"
    /// deferral [`InstKind::Safepoint`] already gets, not a design this crate
    /// itself had to make. `crate::lower::Lowering::lower_reassignment`
    /// retains `value` first when it [`crate::ty::Ty::is_refcounted`] and
    /// [`crate::lower::is_aliasing_read`]s existing storage — `array` durably
    /// owns it once this instruction runs, the same policy `ArraySet` already
    /// gives an explicit key's value. `$a[]` as a *read* (no subscript, no
    /// assignment) has no PHP meaning at all and stays a permanent panic in
    /// [`crate::lower::Lowering::lower_expr`]'s `Index` arm — unrelated to
    /// this instruction, which only ever appears on the write side.
    ///
    /// **Defines a fresh [`crate::ty::Ty::Array`] value**, on exactly
    /// [`InstKind::ArraySet`]'s consume-one-reference-yield-one protocol —
    /// see that variant's own doc comment, which is the one home for it.
    ArrayAppend {
        /// The array, already lowered.
        array: ValueId,
        /// The value to append, already lowered.
        value: ValueId,
    },
    /// Removes `key` from `array` if it is present — `unset($a[$k]);`, ADR
    /// 0028 § 3's one surviving `unset` target (the declared-*property* form
    /// is a diagnostic `mwl_types::expr::check_unset_target` already reports,
    /// so it never reaches lowering).
    ///
    /// **Defines a fresh [`crate::ty::Ty::Array`] value**, on exactly
    /// [`InstKind::ArraySet`]'s consume-one-reference-yield-one protocol —
    /// removing an entry separates a shared array the same way writing one
    /// does, so the holder is re-pointed through the same
    /// `crate::lower::Lowering::write_back_array`. `key` is *borrowed*, not
    /// stored: it is the only array primitive that takes a key without
    /// durably owning it, so `crate::lower::Lowering::lower_stmt`'s `Unset`
    /// arm releases a freshly converted key afterwards instead of retaining
    /// an aliasing one beforehand — the mirror image of what
    /// [`InstKind::ArraySet`] needs.
    ArrayUnset {
        /// The array, already lowered.
        array: ValueId,
        /// The key to remove, already lowered and already `Ty::Str`.
        key: ValueId,
    },
    /// The position of the first live entry at or after `from`, or `-1` when
    /// there is none — one step of a `foreach` cursor over ADR 0007 § 5's
    /// insertion order, defining a [`crate::ty::Ty::Int`].
    ///
    /// A cursor rather than a borrowed iterator because compiled loop-body
    /// code runs between two steps; `mwl_runtime::array`'s `mwl_array_next_slot`
    /// is the one home for why that makes PHP's by-value `foreach` fall out.
    /// Consumes and produces no reference at all: the loop holds its own
    /// reference to `array` for its whole duration, which
    /// `crate::lower::Lowering::lower_foreach` establishes.
    ArrayNextSlot {
        /// The array being walked, already lowered.
        array: ValueId,
        /// The position to resume from, already lowered and already `Ty::Int`.
        from: ValueId,
    },
    /// The key at `slot`, as a fresh [`crate::ty::Ty::Str`] reference the
    /// frame owns — `foreach`'s `$k` binding.
    ///
    /// Unlike [`InstKind::ArrayGet`], this *retains*: a key is stored as the
    /// index map's own `MwlStr`, and handing compiled code a borrowed pointer
    /// into it would outlive the one thing keeping it alive as soon as the
    /// body rebound `$k`. `crate::lower::Lowering::lower_foreach` releases it
    /// at every point one iteration ends.
    ArrayKeyAt {
        /// The array being walked, already lowered.
        array: ValueId,
        /// A position [`InstKind::ArrayNextSlot`] returned, already lowered.
        slot: ValueId,
    },
    /// The value at `slot`, borrowed — `foreach`'s `$v` binding, defining
    /// whatever representation the binding's declared element type lowered to.
    ///
    /// Borrowed exactly like [`InstKind::ArrayGet`], and for the same reason:
    /// `crate::lower::is_aliasing_read` makes whoever copies it into a durable
    /// slot insert the retain. `crate::lower::Lowering::lower_foreach` inserts
    /// one itself when the element type is [`crate::ty::Ty::is_refcounted`],
    /// because the binding *is* such a slot.
    ArrayValueAt {
        /// The array being walked, already lowered.
        array: ValueId,
        /// A position [`InstKind::ArrayNextSlot`] returned, already lowered.
        slot: ValueId,
    },
    /// A call to a Tier 0 `Core` member — `Core\Arr::count($a)`.
    ///
    /// Written like [`InstKind::Call`] in the source and resolved through the
    /// same `mwl_types::expr_table::ResolvedCall`, but lowered separately for
    /// one reason: there is no compiled MWL function to name. A `Core` member
    /// is native Rust behind an [ADR 0002](../../../docs/adr/0002-error-propagation.md)
    /// *helper* entry point, so this carries the linker symbol
    /// `mwl_stdlib::registry` registered rather than a `Class::method` label,
    /// and `mwl-codegen` emits it through the same path
    /// [`InstKind::HelperCall`] takes. It is not a [`Helper`], though: that
    /// enum is a closed set this crate owns, and `Core`'s membership is
    /// `mwl-stdlib`'s to decide.
    ///
    /// **Arguments are borrowed, never consumed** — the helper convention,
    /// which is the opposite of `InstKind::Call`'s. So
    /// `crate::lower::Lowering::lower_call_args` inserts no retain here, and
    /// the caller keeps owning every reference it passed;
    /// `mwl_stdlib`'s own docs own that rule and why ADR 0063's purity
    /// requirement is what makes it safe. A refcounted *result* is a fresh
    /// reference this frame owns, exactly like a `Call`'s.
    ///
    /// There is no receiver field at all: ADR 0063 R20 makes every `Core`
    /// member static, so `$a->count()` resolves to nothing.
    CoreCall {
        /// The linker symbol the implementation is reachable at, from
        /// `mwl_stdlib::registry::CoreMethod::symbol`.
        symbol: &'static str,
        /// The already-lowered arguments, positional.
        args: Vec<ValueId>,
    },
}

/// One member of the closed set of engine-owned runtime conversions
/// [`InstKind::HelperCall`] can invoke — a fixed, non-exhaustive enum for
/// the same reason [`BinOp`]/[`UnOp`] already are one: the set is small,
/// closed, and known entirely to this crate and `mwl-codegen`, never
/// user-extensible, so a string name would only trade compile-time
/// exhaustiveness for nothing. Two families exist so far: a scalar-to-
/// [`crate::ty::Ty::Str`] conversion — for `.` concatenation
/// (`crate::lower::Lowering::concat_operand`), with `IntToString`/
/// `UintToString` reused verbatim by
/// `crate::lower::Lowering::lower_array_key` to normalize an `int`/`uint`
/// array subscript to its decimal-string key form (ADR 0007 § 5) — and a
/// scalar-or-`Ty::Array`-to-[`crate::ty::Ty::Bool`] truthiness test, ADR
/// 0035's table, used by `crate::lower::Lowering::lower_truthy_cond` for an
/// `if`/`while` condition whose static type isn't already `bool` (a
/// `Ty::Object` condition needs none of these: ADR 0035 § 4 makes it always
/// truthy with nothing to inspect at runtime, so that case lowers straight to
/// a fresh [`InstKind::ConstBool`] instead).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Helper {
    /// `int` to its decimal `string` representation.
    IntToString,
    /// `uint` to its decimal `string` representation.
    UintToString,
    /// `float` to its `string` representation.
    FloatToString,
    /// `bool` to `"1"`/`""`, PHP's own bool-to-string rule.
    BoolToString,
    /// `int` truthiness: falsy iff `0`.
    IntTruthy,
    /// `uint` truthiness: falsy iff `0`.
    UintTruthy,
    /// `float` truthiness: falsy iff `0.0` (including `-0.0`; `NAN` is
    /// truthy).
    FloatTruthy,
    /// `string` truthiness: falsy iff `""` or exactly the one-character
    /// string `"0"` — PHP's own rule, so `"0.0"`/`"false"` are truthy.
    StrTruthy,
    /// `array<T>` truthiness: falsy iff empty, for any `T`.
    ArrayTruthy,
    /// `$n as uint` — ADR 0007 § 2's `int` ↔ `uint` row. Exact, or **throws**
    /// on a negative value. The first of nine helpers that can fail rather
    /// than convert, so each is emitted through
    /// `crate::lower::Lowering::emit_fallible` and carries ADR 0002's error
    /// edge like any call.
    IntToUint,
    /// `$n as int` — the same row the other way. Exact, or throws above
    /// `i64::MAX`.
    UintToInt,
    /// `$n as float` — exact, or throws above 2^53, where `f64` stops
    /// representing every integer. ADR 0007 § 2 says so outright: `as` "never
    /// rounds, truncates, or substitutes a default."
    IntToFloat,
    /// `$n as float` from a `uint` — [`Self::IntToFloat`]'s row, unsigned.
    UintToFloat,
    /// `$f as int` — integral and in range, or throws. Rounding is
    /// `floor`/`ceil`/`round`, "said out loud" (ADR 0007 § 2), so this
    /// deliberately refuses `1.5` rather than picking one of the three.
    FloatToInt,
    /// `$f as uint` — [`Self::FloatToInt`]'s row, unsigned.
    FloatToUint,
    /// `$s as int` — the *whole* string must be an exact decimal integer
    /// literal, or this throws. ADR 0007 § 2: "No leading-garbage rule, no
    /// `0`" — PHP's `(int)"12abc" === 12` and `(int)"abc" === 0` are both
    /// gone.
    StrToInt,
    /// `$s as uint` — [`Self::StrToInt`]'s row, unsigned.
    StrToUint,
    /// `$s as float` — the whole string must be an exact numeric literal.
    StrToFloat,
    /// Writes one already-[`crate::ty::Ty::Str`] operand's cooked bytes to
    /// the process's standard output, unescaped — `echo`'s one and only
    /// effect under `mwl run`, decided in `.claude/loop-goal.md`. Defines no
    /// value: the only [`Helper`] so far that is invoked for an effect
    /// rather than a conversion, so its [`InstKind::HelperCall`] is emitted
    /// with `result: None` and every other variant's "the result is a fresh
    /// `Ty::Str` nothing else owns" release policy does not apply to it.
    /// ADR 0024 § 5's auto-escaping sink is the *HTTP response* write, not
    /// this one — whether `echo` under `mwl serve` becomes that sink is an
    /// M7 decision this deliberately does not pre-empt.
    EchoStr,
}

/// A binary arithmetic or comparison operator, already resolved to a single
/// representation (no `mixed`/union dispatch — see the crate docs' "no
/// runtime-helper calls" known gap).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `==` / `===` — this slice does not yet distinguish loose from strict
    /// equality (no non-scalar operand exists yet for the two to differ on).
    Eq,
    /// `!=` / `!==`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
}

/// A unary operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum UnOp {
    /// Numeric negation, `-x`.
    Neg,
    /// Boolean negation, `!x`.
    Not,
}

/// How one [`BasicBlock`] ends.
#[derive(Debug)]
#[non_exhaustive]
pub enum Terminator {
    /// `return;` (`None`) or `return expr;` (`Some`).
    Return(Option<ValueId>),
    /// An unconditional jump — a branch's arm rejoining its merge point, or a
    /// loop's back edge to its header.
    Jump(BlockId),
    /// `throw expr;`: hands `value`'s one reference to the request context as
    /// the pending exception, then enters `landing` exactly the way a failed
    /// call's status check does.
    ///
    /// A terminator rather than an instruction because a `throw` never falls
    /// through, and because the status entering `landing` is the constant
    /// `THROWN` here rather than a value read back from a call.
    Throw {
        /// The [`crate::ty::Ty::Object`] being raised — an instance of
        /// `Throwable` or one of its subclasses. Ownership of one reference
        /// transfers to the context; `crate::lower` retains an aliasing
        /// operand (`throw $e;`) first.
        value: ValueId,
        /// The landing block this frame's cleanup lives in, exactly the block
        /// [`Inst::on_error`] would name for a call at this same point.
        landing: BlockId,
    },
    /// A landing block's exit when nothing in this frame handles the failure:
    /// record `frame` on the pending exception's backtrace, then return the
    /// status onward unchanged.
    ///
    /// `frame` is the fully rendered `Class::method() at <file>:<line>` label,
    /// built at lowering time because that is where a [`mwl_diagnostics::Span`]
    /// can still be resolved to a line — `mwl-codegen` sees only byte offsets.
    /// The line is the enclosing statement's, read from the same per-statement
    /// span table [`Function::stmt_spans`] already carries for ADR 0018's
    /// probes; there is no second position table.
    ///
    /// A `FATAL` pushes nothing: it is not a `Throwable` at all
    /// ([ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)), which
    /// is why the status travels to `mwl_trace_push` rather than being decided
    /// here.
    Propagate {
        /// The backtrace label — see above.
        frame: String,
    },
    /// A landing block's exit inside a `try`: on `THROWN`, enter `handler`;
    /// on any other non-`OK` status, return it onward the way
    /// [`Terminator::Propagate`] does.
    ///
    /// No frame is recorded here, and deliberately so: the backtrace holds the
    /// frames the exception actually unwound *out of*, and a caught throw
    /// never leaves this one. See `mwl_runtime::throwable`'s own docs for why
    /// that differs from PHP's construction-time stack snapshot.
    Catch {
        /// The `catch` clause's handler block.
        handler: BlockId,
    },
    /// A two-way conditional branch, carrying the [`EdgeId`] ADR 0018's
    /// branch probe needs on *each* outgoing edge.
    Branch {
        /// The condition value.
        cond: ValueId,
        /// The block entered when `cond` is true.
        then_block: BlockId,
        /// The stable id of the true edge.
        then_edge: EdgeId,
        /// The block entered when `cond` is false.
        else_block: BlockId,
        /// The stable id of the false edge.
        else_edge: EdgeId,
    },
}
