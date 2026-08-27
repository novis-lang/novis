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
    /// What each field slot's *declared* type lowers to, in [`Self::fields`]'
    /// own order — or **empty**, which means "not known", not "no fields".
    ///
    /// Populated for every class with a layout, plus an ADR 0036 shape
    /// literal's synthesized one: [`InstKind::SlotSet`] reaches any class at
    /// all through § 4's erased receiver, and that write is the one site with
    /// no statically known field type of its own. A slot whose declared type
    /// nothing recorded is [`Ty::Tagged`] rather than absent, so the vector
    /// stays index-aligned with [`Self::fields`] — see `mwl_ir::lower`'s
    /// `field_reprs`. `mwl-codegen` maps each entry to the one
    /// `mwl_runtime::Tag` it admits — [`Ty::Tagged`] and [`Ty::Void`] admit
    /// several or none and become "unchecked" — and hands the result to
    /// `mwl_runtime::ClassTable::set_field_tags`, whose own docs state what
    /// that check buys and what it misses.
    ///
    /// **Cost:** one `Ty` per field slot per class at compile time, and one
    /// byte per slot per descriptor at run time — paid once per compiled
    /// unit, not per request (ADR 0017's cache).
    pub field_reprs: Vec<Ty>,
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
    /// [ADR 0071](../../../docs/adr/0071-derived-codecs.md)'s derived JSON
    /// codec, in declaration order — empty for a class carrying no
    /// `#[Json\Derive]`, which is every class in a program that never writes
    /// the attribute.
    ///
    /// The join of two tables neither crate holds alone:
    /// `mwl_types::derive` reads the attribute, the wire keys and each field's
    /// declared type off the declaration, and `mwl_types::layout` fixes the
    /// slot order — see `crate::lower::lower_file`, which is where the two
    /// meet. Carried through to `mwl_runtime::ClassDesc` so `Core\Json`'s
    /// encoder and decoder can work an instance without asking the program
    /// anything.
    pub codec: Vec<mwl_types::CodecField>,
    /// Every field slot that declares an `= expr` default, as `(slot, value)`
    /// in slot order — empty for a class declaring none, which is most of
    /// them.
    ///
    /// The join of the same two tables [`Self::codec`] joins, on the same
    /// terms and in the same place (`crate::lower::lower_program`):
    /// `mwl_types::signatures` evaluated each default against the property's
    /// declared type, and `mwl_types::layout` fixed the slot order. An
    /// ancestor's default lands in the slot that ancestor's property owns,
    /// because a slot is looked up by *name*.
    ///
    /// **This crate emits no instruction for it.** `mwl-codegen` copies it
    /// onto `mwl_runtime::ClassDesc` and `mwl_runtime::MwlObj::new` writes the
    /// slots, which is the only shape that reaches [`InstKind::New`],
    /// [`InstKind::NewDynamic`] and ADR 0071's native decoder alike — see
    /// `mwl_types::defaults`, which owns why an initializer cannot be spliced
    /// between allocation and construction.
    pub defaults: Vec<(usize, mwl_types::FieldDefault)>,
    /// How many parameters this class's `constructor` declares — see
    /// `mwl_runtime::ClassDesc::ctor_arity`, which is where it ends up and
    /// which owns why it is carried beside the field list rather than derived
    /// from it. Zero for a class with no codec.
    pub ctor_arity: usize,
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

impl BasicBlock {
    /// Every block control can reach from this one, in no meaningful order.
    ///
    /// Both kinds of edge, because a consumer walking the CFG needs both:
    /// the terminator's own targets, and every
    /// [ADR 0002](../../../docs/adr/0002-error-propagation.md) error edge
    /// ([`Inst::on_error`]) an instruction in the body carries. Leaving the
    /// second kind out is how a landing block ends up looking unreachable
    /// from a block that plainly branches into it.
    ///
    /// A block may appear more than once — a `Branch` whose two arms are the
    /// same block lists it twice, and so does a call whose error edge is a
    /// landing block another call already named. Callers dedupe if they care;
    /// `mwl-codegen`'s reverse-postorder walk does, by visiting marks.
    #[must_use]
    pub fn successors(&self) -> Vec<BlockId> {
        let mut out: Vec<BlockId> = self.insts.iter().filter_map(|inst| inst.on_error).collect();
        match &self.term {
            Terminator::Return(_) | Terminator::Propagate { .. } => {}
            Terminator::Jump(target) => out.push(*target),
            Terminator::Throw { landing, .. } => out.push(*landing),
            Terminator::Catch { handler } => out.push(*handler),
            Terminator::Switch { arms, default, .. } => {
                out.extend(arms.iter().map(|(_, target, _)| *target));
                out.push(*default);
            }
            Terminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                out.push(*then_block);
                out.push(*else_block);
            }
        }
        out
    }
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
    /// [`InstKind::NewDynamic`], the one [`InstKind::HelperCall`] with a real
    /// failure mode ([`Helper::EchoStr`]'s write), and **every integer
    /// arithmetic row** — `+`, `-`, `*`, `/` and `%` over
    /// [`crate::ty::Ty::Int`]/[`crate::ty::Ty::Uint`] as an
    /// [`InstKind::BinOp`], and unary `-` over the same two as an
    /// [`InstKind::UnOp`]. All six throw
    /// [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 4's
    /// `ArithmeticError`: the two divisions on a zero divisor, and the other
    /// four on overflow, which that section makes a throw rather than a wrap
    /// or a promotion to `float`. None of the six is a call at all —
    /// `mwl-codegen` tests and raises inline, so this edge is the frame's
    /// cleanup path and nothing else. `None` everywhere else, which is not a
    /// gap in two different ways — a comparison, a float row and a
    /// [`InstKind::Concat`] return no status at all, and a *conversion*
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
    /// contrasts its own, denser probe grid against. `mwl-codegen` lowers it
    /// to one load of the context's safepoint word and a predicted-not-taken
    /// branch, and the **function-entry** one — the first in the entry
    /// block — also carries ADR 0020 § 1's call-stack compare, which is why
    /// that ADR calls the site "not a new pass and not a new emit site".
    /// Reserving the shape ahead of any of that was the point: inserting it
    /// after the fact would have meant re-walking every already-lowered
    /// function, the same "cheap now, expensive to retrofit" reason
    /// `crate::ids` already gives for `StmtId`/`EdgeId`. Defines no value.
    Safepoint,
    /// A `bool` constant.
    ConstBool(bool),
    /// An `int` constant.
    ConstInt(i64),
    /// A `uint` constant.
    ConstUint(u64),
    /// A `float` constant.
    ConstFloat(f64),
    /// A `decimal` constant — [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md)
    /// § 2's untyped-until-placed literal, once a target type has placed it.
    ///
    /// Carried as the three parts rather than as the sixteen-byte image
    /// [`crate::ty::Ty::Decimal`] describes, because this crate does not
    /// depend on `mwl-runtime` and that image's bit positions are
    /// `mwl_runtime::decimal`'s one home. `mwl-codegen` depends on both and is
    /// where the two meet.
    ConstDecimal {
        /// The sign; a zero mantissa is never negative.
        negative: bool,
        /// The unsigned mantissa, at most 96 bits.
        mantissa: u128,
        /// Digits after the point, at most 28.
        scale: u8,
    },
    /// The constant `null` — a [`crate::ty::Ty::Null`] value, which allocates
    /// nothing and owns nothing.
    ///
    /// Its one producer today is an omitted option whose default is "not
    /// given" ([`crate::lower::Lowering::emit_const_arg`]); a written `null`
    /// expression still needs the `?T` lowering `Ty::Tagged`'s own doc comment
    /// records as open.
    ConstNull,
    /// A `string` literal's cooked bytes — a fresh [`Ty::Str`] value with
    /// exactly one implicit owner (itself), the same "one natural reference"
    /// starting point [`InstKind::New`] gives a freshly constructed object.
    /// See `crate::lower::cook_str_literal`'s own doc comment for exactly
    /// which escape sequences are cooked this slice and which are a known
    /// gap.
    ConstStr(String),
    /// A `bytes` constant's octets — [`InstKind::ConstStr`]'s row under
    /// [`crate::ty::Ty::Bytes`], and the same one heap allocation with one
    /// implicit owner, since the two types share a representation and differ
    /// only in the tag a boxed value carries
    /// ([ADR 0009](../../../docs/adr/0009-string-and-bytes.md) § 1).
    ///
    /// **No expression produces one**: there is no `bytes` literal in the
    /// language, so this exists for a `Core` signature's optional `bytes`
    /// parameter, whose default a call site materializes
    /// (`mwl_types::defaults::ConstArg::Bytes`). `Core\Bytes::join`'s
    /// `$separator = ""` is the first.
    ConstBytes(Vec<u8>),
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
    /// already use. A property access whose receiver erased to a shape or to
    /// plain `object` (ADR 0036 § 4) never reaches here: it has no declaring
    /// class to name, so the checker records an
    /// `mwl_types::expr_table::ExprInfo::ShapeProperty` instead and
    /// `crate::lower` emits the name-keyed [`InstKind::SlotGet`].
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
    /// Reads a slot off an object **by index** — `$issue->path`, whose
    /// receiver is an ADR 0036 § 4 shape rather than a named class.
    ///
    /// The difference from [`InstKind::FieldGet`] is that the layout is not
    /// known here. A shape is anonymous and methodless, so there is no class
    /// label for codegen to resolve an offset through — and the receiver's
    /// *static* shape need not be the concrete value's own: ADR 0036 § 3's
    /// width subtyping lets a `{x: int, y: int}` reach a `{y: int}`
    /// parameter, where the two lay their slots out differently. So this is
    /// § 4's **name-keyed fetch**, `mwl_runtime::mwl_object_slot_get`, and not
    /// a fixed offset; § 4 says so outright, and defers the per-call-site
    /// specialization that would make it one to that ADR's *Revisiting*.
    ///
    /// [`Self::slot`] is carried anyway, as a *hint*: the receiver's own
    /// shape is the overwhelmingly common case, and where it holds, the
    /// runtime's lookup is one name comparison rather than a scan.
    ///
    /// **Fallible.** A name the concrete class does not carry is a catchable
    /// throw (§ 4), so this is emitted through
    /// `crate::lower::Lowering::emit_fallible` and carries ADR 0002's error
    /// edge like any call. Unreachable through a receiver whose static shape
    /// lists the field, which is every receiver the checker records one for —
    /// it is the erased half of § 4 that can reach it.
    ///
    /// Borrows its receiver exactly as [`InstKind::FieldGet`] does: the slot
    /// keeps owning what it holds, so a consumer that outlives the receiver
    /// owes the read value a retain.
    SlotGet {
        /// The receiver, already lowered.
        object: ValueId,
        /// The field's own name, `$`-sigil not included — what the fetch is
        /// actually keyed on.
        field: String,
        /// The field's position in the *receiver's static* shape, sorted by
        /// name: a hint, not the answer, and `0` where the receiver is a
        /// plain `object` with no static shape to take a position from. See
        /// this variant's own docs.
        slot: u32,
    },
    /// `$issue->path = "x";` — [`InstKind::SlotGet`]'s write half, and
    /// ADR 0036 § 4's other paragraph: one call to
    /// `mwl_runtime::mwl_object_slot_set`, keyed on the **name** and taking
    /// [`Self::SlotSet::slot`] as the same hint, for the same reason the read
    /// does. A write through an erased or widened view **never creates a
    /// field**; a name the concrete class does not carry is a catchable throw,
    /// exactly as on the read side.
    ///
    /// **Fallible for a second reason the read does not have.** § 4 checks the
    /// incoming value against the field's *real* declared type, because § 3
    /// compares a shape's field types by ordinary assignability and a shape
    /// value is aliased rather than copied — so `{n: int|string}` is a legal
    /// view of a `{n: int}` value, and a `string` written through it would sit
    /// in a slot the narrow view loads as an `int`. What the runtime actually
    /// compares is the tag; `mwl_runtime::object`'s module docs
    /// § *What a shape write checks* own that granularity and its gaps.
    ///
    /// **Borrows [`Self::SlotSet::value`]**, unlike [`InstKind::FieldSet`],
    /// which transfers: the runtime retains what it stores, so a value this
    /// expression built is an ordinary owned temporary that both exits sweep
    /// (`crate::lower::Lowering::owned_temporaries`). Transferring instead
    /// would strand that reference on the throw edge, which — this being the
    /// one field write that *can* throw — is not a hypothetical.
    ///
    /// What the slot held is released by the runtime, so no `SlotGet`/
    /// `Release` pair precedes this the way one precedes a `FieldSet`.
    /// Defines nothing: a [`Ty::Void`] result, like a statement call.
    SlotSet {
        /// The receiver, already lowered.
        object: ValueId,
        /// The field's own name, `$`-sigil not included — what the write is
        /// keyed on.
        field: String,
        /// The field's position in the *receiver's static* shape, sorted by
        /// name: a hint, not the answer, and `0` for a plain `object`
        /// receiver. See [`InstKind::SlotGet`].
        slot: u32,
        /// The value to store, already coerced to the field's static
        /// representation. Borrowed; see this variant's own docs.
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
    /// cooked bytes of every piece, each already [`Ty::Str`] by the time this
    /// instruction sees them — see `crate::lower::Lowering::concat_operand`,
    /// which converts a scalar operand through an [`InstKind::HelperCall`]
    /// first and an object operand through the `toString`
    /// `mwl_types::expr::operators::require_stringable` resolved for it.
    /// Modeled as a dedicated instruction rather than a runtime-helper call
    /// itself, the same "native instruction over already-typed operands"
    /// treatment [`InstKind::BinOp`] already gives scalar arithmetic — `.` only
    /// ever needs this one fixed shape, unlike the open-ended, enum-tagged set
    /// [`HelperCall`](InstKind::HelperCall) exists for.
    ///
    /// **N-ary, not binary, and that is what makes it one allocation.** `.` is
    /// left-associative and an interpolated string is a run of pieces, so both
    /// producers used to fold into a chain of two-operand `Concat`s — and every
    /// link of that chain allocated a buffer holding the accumulation so far
    /// and copied it, so an n-piece concatenation allocated n-1 buffers and
    /// copied its leading pieces n-1 times. One instruction carrying every
    /// piece is one allocation, sized once, with each piece copied once:
    /// `crate::lower::Lowering::lower_concat` flattens the `.` spine and
    /// `crate::lower::Lowering::lower_interpolated_parts` hands its pieces over
    /// whole. `mwl_runtime`'s `mwl_str_concat_n` is the entry point, with the
    /// two-piece case kept on `mwl_str_concat` because it needs neither the
    /// stack array nor the count.
    ///
    /// `pieces` always holds **two or more**. A single-piece interpolation
    /// emits no `Concat` at all — `lower_interpolated_parts`' own doc comment
    /// says what it does instead — and nothing else produces one.
    ///
    /// The result is a fresh value with exactly one natural owner —
    /// concatenation always allocates a new buffer, so
    /// `crate::lower::is_aliasing_read` stays `false` for `ExprKind::Binary`,
    /// same as it already is for [`InstKind::ConstStr`]/[`InstKind::New`]/
    /// [`InstKind::Call`]. No piece is retained by this instruction itself:
    /// each is only *read* to build the new buffer, exactly the way
    /// [`InstKind::FieldGet`] reads its `object` receiver without retaining
    /// it, so ownership of each stays wherever it already was (its own local
    /// slot, field, ...) — and `crate::lower::Lowering::concat_operand`'s
    /// caller releases a piece right after this instruction reads it when that
    /// piece was never such a slot to begin with (a literal, a nested
    /// `Concat`'s own result, or a freshly converted
    /// [`HelperCall`](InstKind::HelperCall) result), since nothing else will
    /// ever release it otherwise.
    Concat {
        /// The operands in evaluation order, each already lowered and already
        /// [`Ty::Str`]. Two or more.
        pieces: Vec<ValueId>,
    },
    /// `$s .= e` where `$s` is a plain [`Ty::Str`] local — the one compound
    /// assignment `crate::lower::Lowering::lower_compound_assignment` does not
    /// rewrite to `$x = $x op e`, because the rewrite's
    /// [`InstKind::Concat`] can only ever build a *fresh* buffer and copy the
    /// whole accumulation into it. In a loop that is quadratic, which is what
    /// this instruction exists to stop being.
    ///
    /// **Defines a fresh [`crate::ty::Ty::Str`] value on exactly
    /// [`InstKind::ArraySet`]'s consume-one-reference-yield-one protocol** —
    /// see that variant's own doc comment, which is the one home for it. The
    /// consequence for lowering is the same one: the *holder* of `target` — a
    /// local's `Env` binding — is re-pointed at the result, with no retain and
    /// no release of either, because the consumed reference and the produced
    /// one are that same one slot's. `mwl_runtime`'s `mwl_str_append` owns
    /// when the two are the same pointer (solely owned, and enough room) and
    /// when a copy-on-write separation makes them different ones.
    ///
    /// `suffix` is only *read*, exactly as [`InstKind::Concat`] reads both of
    /// its operands, so it is not retained here — and a `suffix` that no
    /// durable slot owns (a literal, a `Concat` result, a freshly converted
    /// [`HelperCall`](InstKind::HelperCall)) is released right after this
    /// instruction reads it, by the same
    /// `crate::lower::Lowering::owned_temporaries` staging `Concat`'s own
    /// operands go through.
    ///
    /// Restricted to a plain local target on purpose. A property or element
    /// target would have to write the result back through a `FieldSet` or an
    /// `ArraySet`, which is the write-back `Concat`'s rewrite already gets for
    /// free; those keep the rewrite.
    StrAppend {
        /// The string appended to, already lowered and already [`Ty::Str`].
        /// One reference to it is consumed.
        target: ValueId,
        /// The bytes to append, already lowered and already [`Ty::Str`].
        suffix: ValueId,
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
    /// Stages one by-reference argument: allocates a `Value`-sized slot in
    /// the *caller's* frame, stores `init` into it, and defines that slot's
    /// address as a [`crate::ty::Ty::Ref`].
    ///
    /// This is the whole caller-side half of a `&T` parameter — see
    /// [`crate::ty::Ty::Ref`], which owns the representation decision, the
    /// refcount policy and the known gap. **Retains nothing**: the staging
    /// retain that gives the slot its own reference is an ordinary
    /// [`InstKind::Retain`] `crate::lower::Lowering::lower_call_args` emits
    /// just before this, the same way `bind_local` emits one before an `Env`
    /// insert rather than folding it into a compound instruction.
    ///
    /// Allocation is per *call site*, not per execution: Cranelift stack slots
    /// are frame-scoped, so a by-reference call inside a loop restages into
    /// the same slot every iteration and costs nothing beyond the two copies.
    ///
    /// Cannot fail, so like [`InstKind::Concat`] it carries no status check
    /// and no landing block.
    RefSlot {
        /// The holder's current value, copied into the fresh slot.
        init: ValueId,
    },
    /// Reads the [`crate::ty::Ty::Ref`] slot `slot` points at, at this
    /// instruction's own [`Inst::ty`] — which is the *pointee's*
    /// representation, not [`crate::ty::Ty::Ref`].
    ///
    /// Emitted at both ends of a by-reference parameter: inside the callee for
    /// every read of the parameter, and in the caller immediately after the
    /// call to pick up whatever the callee left there. Retains nothing — the
    /// slot keeps owning its one reference, exactly the way
    /// [`InstKind::FieldGet`] leaves a field's slot owning its own.
    RefLoad {
        /// The [`crate::ty::Ty::Ref`] whose cell is read.
        slot: ValueId,
    },
    /// Writes `value` into the [`crate::ty::Ty::Ref`] slot `slot` points at.
    /// Defines no value.
    ///
    /// **Retains and releases nothing**, for [`InstKind::RefSlot`]'s reason:
    /// keeping [`crate::ty::Ty::Ref`]'s "the slot owns exactly one reference"
    /// invariant is `crate::lower::Lowering`'s job, and it does it the same
    /// way `bind_local` does for an `Env` entry — retain the incoming value
    /// when it is an aliasing read, then [`InstKind::RefLoad`] the slot's
    /// previous value and [`InstKind::Release`] it, in that order, so a
    /// self-assignment never observes a transient zero refcount.
    RefStore {
        /// The [`crate::ty::Ty::Ref`] whose cell is written.
        slot: ValueId,
        /// The value stored into it.
        value: ValueId,
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
    /// Widens a statically-typed value into a [`crate::ty::Ty::Tagged`] one:
    /// the operand's payload under the tag byte its own representation names
    /// (`mwl_codegen::ty::tag_of`).
    ///
    /// Free of any allocation and of any call — it builds a register pair —
    /// and **transfers ownership unchanged**: a tagged value carrying a
    /// refcounted payload owns exactly the reference the operand owned, so
    /// `crate::lower` inserts no retain around it and the eventual
    /// [`InstKind::Release`] of the tagged value discharges the operand's
    /// obligation. Cannot fail: no status, no landing block.
    Tag {
        /// The value being widened.
        operand: ValueId,
    },
    /// Narrows a [`crate::ty::Ty::Tagged`] value back to the representation
    /// [`Inst::ty`] names — the payload half, read at that representation.
    ///
    /// **Unchecked, and deliberately.** The tag is not compared: this crate
    /// only emits an `Untag` where `mwl_types` has already proved which
    /// representation the value holds — the non-`null` arm of a `??`, a
    /// `?->` or an `if ($x !== null)` narrowing. A runtime *test* is
    /// [`InstKind::IsNull`], and a conversion that can genuinely fail is an
    /// ADR 0007 § 2 checked row, not this.
    ///
    /// **Transfers ownership unchanged**, the mirror of [`InstKind::Tag`]:
    /// the narrowed value owns the reference the tagged one owned, so the
    /// tagged value must not also be released.
    Untag {
        /// The tagged value being narrowed.
        operand: ValueId,
    },
    /// Whether a [`crate::ty::Ty::Tagged`] value's tag is `Tag::Null` —
    /// [`crate::ty::Ty::Bool`], one compare, no call.
    ///
    /// The only instruction that reads a tag, and the whole of what `??`,
    /// `?->` and a nullable narrowing test. It borrows its operand: no
    /// retain, no release, no ownership transfer.
    IsNull {
        /// The tagged value being tested.
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
    /// positional alike) a real `ValueId` there, which this fixed-entries
    /// shape has no field for. See
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
    /// `crate::lower::Lowering::lower_expr`'s `Index` arm). `key` is in one
    /// of exactly two representations — [`crate::ty::Ty::Str`], or
    /// [`crate::ty::Ty::Int`] for a subscript that was already an `int` and
    /// therefore never rendered — which is the crate docs' *an array key is
    /// a `string`, and an `int` subscript no longer spells it*. ADR 0007
    /// § 5's key normalization is unchanged (`$a[8]` is still `$a["8"]`);
    /// where the decimal is produced is what moved. A `uint` subscript is
    /// still rendered by `crate::lower::Lowering::lower_array_key`, with the
    /// exact [`Helper::UintToString`] conversion
    /// [`Lowering::concat_operand`](crate::lower::Lowering::concat_operand)
    /// already gives `.`'s scalar operand rather than a new policy; that
    /// function's own doc comment says why an `i64` index cannot carry it.
    ///
    /// **An absent `key` throws**, so this is a *fallible* instruction and
    /// carries ADR 0002's error edge like a call: it is emitted through
    /// `crate::lower::Lowering::emit_fallible`, and `mwl-codegen` gives it the
    /// same status check every helper call gets, against the runtime entry
    /// point `mwl_array_required_get` — which is why the two representations
    /// above are told apart there, by the key's own tag, rather than by
    /// picking a symbol here. PHP warns and yields `null`; ADR 0007 § 7 row 11
    /// records the divergence and that helper's doc comment says why the old
    /// answer was a null dereference rather than a value. A stored `null` is
    /// *not* an absent key and reads back unchanged. The write side asks the
    /// same question and answers it differently — an absent key vivifies —
    /// which is what [`Helper::ArrayRowForWrite`] exists for. Reads `array`
    /// without
    /// retaining it, the same way `FieldGet` reads its `object` receiver — a
    /// caller copying the result into a second durable slot retains it
    /// there instead (`crate::lower::is_aliasing_read` now also matches
    /// `ExprKind::Index`, so the existing `bind_local`/`lower_call_args`/
    /// `StmtKind::Return` insertion points already do this with no new
    /// call site).
    ArrayGet {
        /// The array, already lowered.
        array: ValueId,
        /// The lookup key, already lowered — `Ty::Str` or `Ty::Int`.
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
    /// with. `key` is `Ty::Str` or `Ty::Int`, the same two representations
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
        /// The key to write, already lowered — `Ty::Str` or `Ty::Int`.
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
    ///
    /// The one array write carrying an [`Inst::on_error`] edge: PHP refuses an
    /// append whose next integer key is already live, and `mwl_runtime::array`
    /// § *the append is the one array write with a fault channel* owns both
    /// the refusal and what it leaves each operand's reference holding.
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
    /// There is no receiver *field*, and an instance member needs none: a
    /// `Core`-owned class's member (`$match->text()`,
    /// `mwl_stdlib::registry::CoreTy::Instance`) puts its receiver in
    /// `args[0]`, which is where the ABI would have carried a separate field
    /// anyway. `crate::lower`'s `MethodCall` arm is what puts it there, and it
    /// borrows the receiver exactly as it borrows every other argument — so a
    /// *freshly built* receiver is released by that arm rather than by the
    /// callee. ADR 0063 R20 keeps the two directions apart: a static member is
    /// unreachable through a value and an instance member is a compile error
    /// through the class name, so no member is ever reached both ways.
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
/// exhaustiveness for nothing. Three families exist so far: a scalar-to-
/// [`crate::ty::Ty::Str`] conversion — for `.` concatenation
/// (`crate::lower::Lowering::concat_operand`), with `UintToString` reused
/// verbatim by `crate::lower::Lowering::lower_array_key` to render the one
/// array subscript that cannot travel unrendered, and `IntToString` by
/// `crate::lower::Lowering::lower_rendered_array_key` for the one caller
/// that still needs a `Ty::Str` key (ADR 0007 § 5) — and a
/// scalar-or-`Ty::Array`-to-[`crate::ty::Ty::Bool`] truthiness test, ADR
/// 0035's table, used by `crate::lower::Lowering::lower_truthy_cond` for an
/// `if`/`while` condition whose static type isn't already `bool` (a
/// `Ty::Object` condition needs none of these: ADR 0035 § 4 makes it always
/// truthy with nothing to inspect at runtime, so that case lowers straight to
/// a fresh [`InstKind::ConstBool`] instead). The third has one member,
/// [`Helper::ArrayRowForWrite`], and is here for the reason its own doc
/// gives: an array primitive whose ownership answer is uniform across a
/// present and an absent key, which no borrowing read can be.
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
    /// One level of a nested array-element write's descent
    /// (`crate::lower::Lowering::lower_reassignment`'s `Index` arm): the row
    /// `args[1]` names in the array `args[0]`, **with a reference of its
    /// own** — a retain of what was there, or a freshly allocated empty
    /// array when the key is absent, which is PHP's auto-vivification.
    ///
    /// It exists because a write asks the absent-key question and gets the
    /// opposite answer to a read's: [`InstKind::ArrayGet`] *throws* there
    /// (ADR 0007 § 7 row 11), while `$g[9][0] = 1` must build the row PHP
    /// would have built. It also *borrows*, where a descent needs a reference
    /// of its own to hand the [`InstKind::ArraySet`] on the way back up.
    /// Folding the two into
    /// one entry point keeps the ownership uniform — the result is always
    /// exactly one owned reference, so the lowering emits no retain beside
    /// it — and it is why a nested write needs no branch in the IR at all.
    /// The vivified row is not inserted into `args[0]` here; the `ArraySet`
    /// on the way back up stores it, which is the same instruction that
    /// re-points an existing row.
    ArrayRowForWrite,
    /// `decimal` truthiness: falsy iff zero, at any scale.
    DecimalTruthy,
    /// `a + b` over [`crate::ty::Ty::Decimal`] —
    /// [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md) § 3, which
    /// **throws** on either overflow kind rather than wrapping or promoting,
    /// so this and the four below carry ADR 0002's error edge like any call.
    ///
    /// Either operand may be an `int` or a `uint` instead: § 3's
    /// `decimal ⊕ int` row promotes exactly in 96 bits, and the helper does it
    /// from the operand's own tag, so lowering emits no conversion for one.
    /// A `float` operand never reaches here — § 3 makes `decimal ⊕ float` a
    /// compile error.
    DecimalAdd,
    /// `a - b` over `decimal` — see [`Self::DecimalAdd`].
    DecimalSub,
    /// `a * b` over `decimal`. The scales add, so a product wanting more than
    /// 28 of them throws rather than reducing precision the way
    /// `System.Decimal` does.
    DecimalMul,
    /// `a / b` over `decimal` — always a `decimal`, never a union, rounding
    /// half to even at the maximum scale the result admits. A zero divisor
    /// throws.
    DecimalDiv,
    /// `a % b` over `decimal`, carrying the dividend's sign.
    DecimalMod,
    /// `-a` over `decimal`. Never fails: the mantissa is unsigned, so there is
    /// no asymmetric minimum to overflow.
    DecimalNeg,
    /// `a == b` with a `decimal` operand — exact, and independent of scale, so
    /// `1.10 == 1.1000`. **`!=` is this helper under [`UnOp::Not`]**, which is
    /// also what gives an unordered `NaN` operand PHP's answer.
    ///
    /// Unlike the arithmetic helpers this one takes a `float` operand too:
    /// ADR 0054 § 3 permits comparison across the pair precisely where it
    /// forbids arithmetic, because an exact comparison is always computable
    /// even where a common arithmetic type is not.
    DecimalEq,
    /// `a < b` with a `decimal` operand. **`>` is this helper with its
    /// operands swapped** — see [`Self::DecimalEq`] for why three helpers
    /// cover all six comparisons.
    DecimalLt,
    /// `a <= b` with a `decimal` operand; `>=` is this one swapped.
    DecimalLtEq,
    /// `a <=> b` with a `decimal` operand — the ordering [`Self::DecimalLt`]
    /// asks one question of, answered whole as an `int`. ADR 0054 § 3 grants
    /// this row for the reason [`Self::DecimalEq`] states: an exact comparison
    /// is computable across every pairing, including the `decimal`/`float` one
    /// arithmetic refuses. An unordered pair is `1`, as it is for
    /// [`Self::NumericCmp`] and [`BinOp::Cmp`].
    DecimalCmp,
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
    /// `$b as string` — [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)
    /// § 3's checked row: the buffer is well-formed UTF-8 and becomes the
    /// `string` over the *same* allocation, or this throws. It never replaces,
    /// drops or truncates a bad sequence, so it is fallible and carries
    /// ADR 0002's error edge like every other checked row.
    ///
    /// **The pair's other direction is not here, and that is the point.**
    /// `string as bytes` is total and free — the same `mwl_runtime::MwlStr`
    /// under a second tag — so `crate::lower::Lowering::convert` lowers it to
    /// an [`InstKind::Reinterpret`] and emits no call at all. Only the
    /// checked half needs a helper, because only the checked half runs
    /// anything.
    BytesToString,
    /// `$x as ?int` — [ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md)
    /// § 1's non-throwing form of every row above that lands on `int`: the
    /// value `as int` would produce, or `null` where it would throw. Cannot
    /// fail, so unlike the nine throwing rows it carries no error edge, and
    /// its result is a [`crate::ty::Ty::Tagged`] value rather than a bare
    /// `int`.
    ///
    /// **One tag per target, not one per (source, target) pair.** This helper
    /// dispatches on the operand's runtime tag, which is what makes ADR 0066
    /// § 2's "a `null` operand yields `null`" and § 3's "from `mixed` every
    /// target has a checked path" the same code as `"42" as ?int` rather than
    /// three lowering branches — the operand is already a tagged `Value` by
    /// the time any helper sees it (`mwl_codegen`'s `store_value`).
    ToIntOrNull,
    /// `$x as ?uint` — [`Self::ToIntOrNull`]'s row set, unsigned.
    ToUintOrNull,
    /// `$x as ?float` — [`Self::ToIntOrNull`]'s row set, landing on `float`.
    ToFloatOrNull,
    /// `$x as decimal` — ADR 0054 § 4's four `→ decimal` rows, chosen by the
    /// operand's runtime tag the way [`Self::ToIntOrNull`] chooses, so one
    /// helper covers `int`, `uint`, `float`, `string` and `mixed` alike.
    /// Throws where the row fails or does not exist, so it carries an error
    /// edge.
    ToDecimal,
    /// `$x as ?decimal` — [`Self::ToDecimal`]'s rows in ADR 0066 § 1's
    /// non-throwing form, sharing one implementation of each.
    ToDecimalOrNull,
    /// `$d as int` — integral and in range, or throws. Rounding is
    /// `Core\Decimal::floor`/`ceil`/`round`, said out loud, exactly as
    /// [`Self::FloatToInt`] already is.
    DecimalToInt,
    /// `$d as uint` — [`Self::DecimalToInt`]'s row, unsigned.
    DecimalToUint,
    /// `$d as float` — the nearest `f64`. Lossy, total, and explicit like
    /// every other `as`.
    DecimalToFloat,
    /// `$d as string` — total, and **scale-preserving**: `19.90` renders as
    /// `"19.90"`, which is ADR 0054 § 4's row.
    DecimalToString,
    /// A [`crate::ty::Ty::Tagged`] operand to `string` — the four scalar
    /// conversions above plus `null`, chosen by the operand's **runtime** tag
    /// rather than by a static type, since a `mixed`, a `?T` or any other
    /// union has none to choose by.
    ///
    /// One helper, reached from both spellings that render a value: `.`
    /// concatenation and `echo` (`crate::lower::Lowering::concat_operand`),
    /// and `expr as string` (`crate::lower::Lowering::convert`). That is the
    /// same "one tag per target" arrangement [`Self::ToIntOrNull`] describes,
    /// so `mixed` needs no lowering branch of its own in either place.
    ///
    /// **The one string conversion that can fail**, so unlike the four static
    /// ones it is emitted through `crate::lower::Lowering::emit_fallible` and
    /// carries ADR 0002's error edge: an array, an object, a closure and a
    /// resource have no row, and `mwl_runtime::value_to_string` owns what each
    /// throws and why.
    TaggedToString,
    /// A [`crate::ty::Ty::Tagged`] operand to `int` — ADR 0007 § 2's `→ int`
    /// rows chosen by the operand's **runtime** tag, which is the only thing
    /// that names a row when the static type is a `mixed`, a `?T` or any other
    /// union.
    ///
    /// The throwing twin of [`Self::ToIntOrNull`], over the same row set in
    /// `mwl_runtime` and never a second copy of it: § 2's `as T` throws where
    /// ADR 0066's `as ?T` answers `null`, so the pair differs only in what it
    /// does with a miss. Fallible, so it is emitted through
    /// `crate::lower::Lowering::emit_fallible` and carries ADR 0002's error
    /// edge — a tag with no row at all (an array, an object, a `bool`) throws
    /// here, which is ADR 0007 § 6's answer for `mixed` and a compile error for
    /// anything the checker can name.
    TaggedToInt,
    /// [`Self::TaggedToInt`]'s row set, unsigned.
    TaggedToUint,
    /// [`Self::TaggedToInt`]'s row set, landing on `float`.
    TaggedToFloat,
    /// Writes one already-[`crate::ty::Ty::Str`] operand's cooked bytes to
    /// the process's standard output, unescaped — `echo`'s one and only
    /// effect under `mwl run`, decided in `docs/agent/loop-goal.md`. Defines no
    /// value: the only [`Helper`] so far that is invoked for an effect
    /// rather than a conversion, so its [`InstKind::HelperCall`] is emitted
    /// with `result: None` and every other variant's "the result is a fresh
    /// `Ty::Str` nothing else owns" release policy does not apply to it.
    /// ADR 0024 § 5's auto-escaping sink is the *HTTP response* write, not
    /// this one — whether `echo` under `mwl serve` becomes that sink is an
    /// M7 decision this deliberately does not pre-empt.
    EchoStr,
    /// The throw at the end of
    /// [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md)
    /// § 5's membership test: the operand reached none of the literals its
    /// target names, so the checked `as` § 4 describes fails.
    ///
    /// Two arguments — the operand, which the message renders, and a
    /// [`InstKind::ConstStr`] holding the accepted set **already rendered at
    /// lowering time**, because that set is a compile-time-known list of
    /// types and this crate is the last place they exist. § 6 requires the
    /// message to name the set, and generating it from the type rather than
    /// per call site is what makes the run-time throw read the same as the
    /// `E0469` the checker reports when the operand settles the question by
    /// itself.
    ///
    /// Defines no value and **never returns normally**: it is the miss arm of
    /// a chain of [`BinOp::Eq`] comparisons, so it is pushed with
    /// `result: None` the way [`Self::EchoStr`] is, and always carries
    /// ADR 0002's error edge. The membership test itself costs no helper call
    /// at all — see `crate::lower::Lowering::lower_literal_membership` for why
    /// a comparison chain and not one call over an encoded set.
    ///
    /// It **owns** that second argument, uniquely among helpers: a call that
    /// never returns leaves no reachable instruction to release the fresh
    /// [`InstKind::ConstStr`] at, so `mwl_runtime`'s own helper drops it.
    /// The operand keeps the ordinary convention and is the caller's.
    LiteralMismatch,
    /// `a == b` where at least one operand is a [`crate::ty::Ty::Tagged`] —
    /// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
    /// § 5's `mixed`-or-union case, the one pairing whose § 3 row is a runtime
    /// tag rather than a static type. **`!=` is this helper under
    /// [`UnOp::Not`]**, the arrangement [`Self::DecimalEq`] already uses.
    ///
    /// Every other row of that table is reached without this helper, because
    /// the operands' types already named it: a scalar pair is one
    /// [`BinOp::Eq`] machine comparison, and `mwl-codegen` turns a `string`,
    /// `array` or `object` pair into a direct two-pointer call or an inline
    /// pointer compare rather than tagging both sides into stack slots. So
    /// this is the *only* row that pays ADR 0002's calling convention, and it
    /// is the row that has nothing cheaper to pay.
    ///
    /// It cannot fail — § 5 makes a mismatched pair `false` rather than a
    /// throw — so unlike a conversion helper it carries no error edge.
    Identical,
    /// `a == b` over two operands whose *representations* differ but whose
    /// types are one domain —
    /// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
    /// § 2's numeric row, which makes `int`, `uint` and `float` mutually
    /// comparable, and § 3's "mathematically equal across the whole domain".
    /// **`!=` is this helper under [`UnOp::Not`]**, the arrangement
    /// [`Self::DecimalEq`] already uses.
    ///
    /// A call rather than a widening conversion because **no widening between
    /// these three is exact**, which is the whole reason this row cannot be
    /// settled the way a same-representation pair is: `int` → `float` loses
    /// every integer past 2^53, `float` → `int` has no answer for a fractional
    /// or out-of-range operand, and `int` ↔ `uint` share a bit pattern at `-1`
    /// and `u64::MAX`. `mwl_runtime::numeric_identical` compares over `i128`
    /// and over the float's own binary value instead, so `1 == 1.0` is `true`
    /// and no pair is ever equated by a rounding neither operand asked for.
    ///
    /// A `decimal` operand never reaches here — [`Self::DecimalEq`] already
    /// takes every pairing one side of which is a `decimal`, and a
    /// [`crate::ty::Ty::Tagged`] one takes [`Self::Identical`]. Like both of
    /// those it is total, so it carries no error edge.
    NumericEq,
    /// `a < b` over that same pair of representations — the ordering half of
    /// [`Self::NumericEq`], and exact for the same reason.
    ///
    /// [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 4 closes
    /// its own table with the rule this exists to keep: "a comparison has an
    /// exact answer in the mathematical integers and can be lowered as one".
    /// It says that about `int` against `uint`, and the `int`/`uint` against
    /// `float` pairing is the same question — so widening the integer side
    /// into a `float` first, which is what the *arithmetic* rows of that table
    /// do, is wrong here twice over: it would round away every integer past
    /// 2^53, and because § 2 makes that widening **throw** rather than round,
    /// what a program would actually observe is `9007199254740993 < 1.5`
    /// raising `ArithmeticError` where PHP answers `false`.
    /// `mwl_runtime::numeric_ordering` compares over `i128` and over the
    /// float's own binary value instead, which has an answer for every pair.
    ///
    /// **`>` is this helper with its operands swapped**, the arrangement
    /// [`Self::DecimalLt`] already uses; a matched pair never reaches here at
    /// all, being one machine comparison. Total, so no error edge.
    NumericLt,
    /// `a <= b` over a mixed numeric pair — [`Self::NumericLt`]'s row
    /// inclusive, and `>=` is this one swapped.
    NumericLtEq,
    /// `a <=> b` over a mixed numeric pair — [`Self::NumericLt`]'s row, read
    /// as an ordering rather than as one question about it, and answered as an
    /// `int`. The `None` that row returns for a `NaN` operand becomes `1`,
    /// which is PHP's answer for an unordered pair and the same convention
    /// [`BinOp::Cmp`] follows for a matched one.
    NumericCmp,
    /// `a == b` over two operands at least one of which the checker typed
    /// `secret` —
    /// [ADR 0033](../../../docs/adr/0033-secret-qualifier-for-confidential-values.md)
    /// § 5. The comparison is **constant-time in the contents**: it reads
    /// every byte of two equal-length operands whatever they hold, so an
    /// attacker holding one side cannot recover the other a byte at a time by
    /// timing the answer. Lengths are not hidden — a mismatch answers `false`
    /// at once, which is what `mwl_stdlib`'s `Core\Hash::equals` does for the
    /// same reason.
    ///
    /// **`!=` is this helper under [`UnOp::Not`]**, the arrangement
    /// [`Self::NumericEq`] and [`Self::DecimalEq`] already use, and it is
    /// total, so it carries no error edge.
    ///
    /// The qualifier is invisible at this level by design: a `secret string`
    /// erases to [`crate::ty::Ty::Str`] and a `secret bytes` to
    /// [`crate::ty::Ty::Bytes`], because ADR 0033 § 1 spends no representation
    /// on the bit. So the *lowering* cannot re-derive the choice of helper
    /// from its operand types, and does not try: the checker records
    /// `mwl_types::expr_table::ExprInfo::SecretEquality` at the comparison and
    /// `lower_binary` reads it back.
    ///
    /// Costed in ADR 0033 § 5: ≈10 ns against ≈2 ns for the short-circuiting
    /// row, so **≈+8 ns per comparison** — priority 1 bought with priority 3,
    /// which is the ordering AGENTS.md states.
    SecretEq,
}

/// A binary arithmetic or comparison operator, already resolved to a single
/// representation (no `mixed`/union dispatch — see the crate docs' "no
/// runtime-helper calls" known gap).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum BinOp {
    /// `+`. Over an `int` or a `uint` this and the four below carry
    /// [`Inst::on_error`] — see that field for which of them throws on what.
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`. The one operator whose result representation is not its operands':
    /// ADR 0007 § 4 types the integer row as `int|float`, so it produces a
    /// [`crate::ty::Ty::Tagged`] and picks between the two at run time.
    Div,
    /// `%`
    Mod,
    /// `**`. ADR 0007 § 4 lists this beside `+`, `-` and `*`, so the integer
    /// row **throws** rather than wrapping and it carries [`Inst::on_error`]
    /// too — including for a *negative* exponent, which that row's "no
    /// promotion to `float`" leaves with no `int` to answer except where the
    /// base is `1` or `-1`. Neither representation is one instruction:
    /// `mwl-codegen` emits a square-and-multiply loop for the integer row and
    /// calls `mwl_runtime::mwl_float_pow` for the float one, there being no
    /// `fpow` on any target and no `LibCall` for it either.
    Pow,
    /// `&` — ADR 0007 § 4 preserves the operand type, and this and the two
    /// below are total: no pair of `int`s or `uint`s has an unrepresentable
    /// bitwise combination, so none of the three carries [`Inst::on_error`].
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `<<`. PHP's semantics, which are not the machine's: a **negative** count
    /// throws `ArithmeticError`, so this carries [`Inst::on_error`] over an
    /// `int` (a `uint` count cannot be negative), and a count of 64 or more
    /// answers `0` rather than the masked shift x86 would perform.
    Shl,
    /// `>>` — [`Self::Shl`]'s two rules, and one of its own: ADR 0007 § 4 makes
    /// this **arithmetic** on an `int` and **logical** on a `uint`, so a count
    /// past the width fills with the sign bit in the first case and with zero
    /// in the second.
    Shr,
    /// `==` — ADR 0090 makes this the language's only equality operator, with
    /// no conversion of either operand. Every operand pair that reaches here
    /// has one statically known representation, and `mwl-codegen` picks that
    /// row's comparison from it: a machine compare for a scalar, a call to
    /// `mwl_runtime::mwl_str_eq` or `mwl_runtime::mwl_array_eq` for a `string`
    /// or `array` pair, and a pointer compare for two objects. A `mixed` or
    /// union operand has no static row, so it does not lower to this at all —
    /// it takes [`Helper::Identical`].
    Eq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `<=>`, over a **matched** scalar representation — `-1`, `0` or `1` as
    /// an [`crate::ty::Ty::Int`], never the operands' own type.
    ///
    /// PHP's own answer for an unordered pair is `1` rather than `0`, so this
    /// is emitted as "less, else equal, else `1`" and not as the tidier
    /// `(a > b) - (a < b)`: the two differ on exactly the `NaN` rows, where
    /// every float comparison is false and the second formula would answer
    /// `0` — that is, "equal" — for a pair that is not.
    ///
    /// A pairing with two representations does not reach here, the same way it
    /// does not for [`Self::Eq`]: a mixed numeric pair takes
    /// [`Helper::NumericCmp`], a `decimal` one [`Helper::DecimalCmp`], and two
    /// objects are ADR 0013's `Comparable::compareTo` call, whose `int` result
    /// already *is* this operator's answer.
    Cmp,
}

/// A unary operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum UnOp {
    /// Numeric negation, `-x`. Over an `int` or a `uint` it carries
    /// [`Inst::on_error`]: `-i64::MIN` has no `int` and `-$u` no `uint` for any
    /// non-zero `$u`, and ADR 0007 § 4 makes both a throw rather than a wrap.
    Neg,
    /// Boolean negation, `!x`.
    Not,
    /// Bitwise complement, `~x`, over an `int` or a `uint`. Total — every
    /// pattern of 64 bits is a value of both — so unlike [`Self::Neg`] it
    /// carries no error edge.
    BitNot,
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
    /// An N-way branch on an integer value: the first arm whose case equals
    /// `value` is entered, `default` when none does.
    ///
    /// Built for ADR 0053 § 4's generator resumption — `crate::lower`'s
    /// generator section owns what the cases mean there — and shaped as a
    /// general N-way terminator rather than a resumption-specific one, which
    /// is what keeps every consumer's `match` on [`Terminator`] honest.
    ///
    /// **MWL's own `switch` statement does not use it.** Its cases are
    /// arbitrary expressions of the subject's type rather than integers —
    /// `case "A":` is one string comparison call — so it lowers to a chain of
    /// [`Terminator::Branch`]es instead; `crate::lower::Lowering::lower_switch`
    /// owns that reasoning.
    ///
    /// Cases are matched in order and are **not** required to be dense,
    /// contiguous or sorted; a duplicate case is unreachable rather than an
    /// error, exactly as a duplicate `if` arm would be. `mwl-codegen` lowers
    /// this to a compare chain, which is why order is what it is — a jump
    /// table over a dense case set is the obvious optimisation and is
    /// deliberately not taken yet (see `mwl-codegen`'s own known gaps).
    ///
    /// Each arm carries its own [`EdgeId`] for the same reason
    /// [`Terminator::Branch`]'s two do: ADR 0018 § 1's branch probe is
    /// per-edge, and retrofitting ids onto edges already lowered is the cost
    /// [`crate::ids`] exists to avoid.
    Switch {
        /// The value tested, at [`crate::ty::Ty::Int`].
        value: ValueId,
        /// One `(case value, target, edge)` per arm, in match order.
        arms: Vec<(i64, BlockId, EdgeId)>,
        /// Where control goes when no arm matched.
        default: BlockId,
        /// The stable id of the default edge.
        default_edge: EdgeId,
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
