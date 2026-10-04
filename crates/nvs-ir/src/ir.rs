//! The CFG/SSA IR itself: a [`Program`] of [`Function`]s, each a graph of
//! [`BasicBlock`]s of SSA [`Inst`]ructions ending in exactly one
//! [`Terminator`]. See the crate's own module docs for this crate's scope and
//! known gaps.

use nvs_diagnostics::Span;
use nvs_render::Source;

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
    /// Every `static` property the unit declares, in slot order — see
    /// [`StaticProp`], and [`InstKind::StaticGet`] for what a slot number is.
    pub statics: Vec<StaticProp>,
    /// One wire contract per **distinct inline shape** written as a type
    /// argument anywhere in the unit, sorted by [`ShapeCodec::key`] — see
    /// [`ShapeCodec`], and the crate docs' *A shape's wire contract* for why a
    /// shape's contract is a table of its own rather than a field of the class
    /// its descriptor names.
    pub shape_codecs: Vec<ShapeCodec>,
    /// Every `enum` the checker resolved for this unit — the program's own
    /// declarations and the `Core` ones its table is seeded with alike, in no
    /// particular order. See [`Enum`].
    pub enums: Vec<Enum>,
}

impl Program {
    /// The source span of every statement in the program, indexed by the
    /// number a compiled coverage probe passes to `nvs_runtime::nvs_probe_stmt`.
    ///
    /// A [`StmtId`] numbers from zero within each function. Codegen adds the
    /// function's base to it, which is the count of statements in every
    /// function before it in [`Self::functions`], so a number names one
    /// statement in the whole program. This list is the same functions in the
    /// same order, so its length is the size an `nvs_runtime::StmtHits` table
    /// for this program needs. It reads only the IR, so it holds for a unit
    /// loaded from the compile cache too.
    #[must_use]
    pub fn stmt_spans(&self) -> Vec<Span> {
        self.functions
            .iter()
            .flat_map(|function| function.stmt_spans.iter().copied())
            .collect()
    }
}

/// One declared `enum`'s whole shape, on its way to
/// `nvs_runtime::ClassTable::define_enum`.
///
/// Not a [`Class`], and the difference is the reason this list exists at all:
/// `rule:enums/representation` makes a case *be* the integer behind it, so an
/// enum has no descriptor, no instance and nothing compiled code reaches. What
/// a unit still has to carry is the shape itself, because
/// `rule:enums/reflection` reports it at run time and the checker is the only
/// place it exists.
///
/// Copied off `nvs_types::enums::EnumTable` rather than collected out of the
/// functions that lowered, for [`ShapeCodec`]'s reason exactly: a declaration
/// is a fact about the program, and a unit that mentions one of its cases
/// nowhere still declares it.
#[derive(Clone, Debug)]
pub struct Enum {
    /// The enum's rendered name, spelled the way [`Class::label`] is.
    pub label: String,
    /// Whether the backing type is `uint` — `rule:enums/one-backing-type`'s
    /// other option, and the whole of what decides which integer type a case's
    /// value reads back as.
    pub unsigned: bool,
    /// Every declared case as `(name, value)`, widened to `i128` so one field
    /// carries both backings with no lossy cast. Ordering is
    /// `nvs_runtime::ClassTable::define_enum`'s, which is where it is settled.
    pub cases: Vec<(String, i128)>,
}

/// The wire contract of one inline shape a call site wrote as a type argument —
/// `Core\Json::decodeAs<{n: int}>`'s `{n: int}`, read as the per-field list
/// `nvs_runtime::ShapeCodec` holds.
///
/// One of these per distinct **contract**, not per call site: two calls writing
/// the same field names at the same types share one table, because
/// [`Self::key`] is that contract's own rendering. The class they also share is
/// a different sharing on different terms — `crate::lower::shape_class_label`
/// keys it on the field *names* alone, which is exactly why the contract cannot
/// ride on the descriptor and is carried beside it instead.
#[derive(Clone, Debug)]
pub struct ShapeCodec {
    /// What [`InstKind::ShapeCodecConst`] names this table by, and what
    /// `nvs-codegen` mints the relocation symbol from —
    /// `crate::lower::shape_codec_key`'s rendering of [`Self::fields`], which
    /// is the whole of what makes two contracts different.
    pub key: String,
    /// Every field of the shape, in the sorted field-name order the shape
    /// class lays its slots out in — so a field's index is its slot and its
    /// `nvs_types::CodecField::param` alike, and a decoder joins nothing.
    pub fields: Vec<nvs_types::CodecField>,
}

/// One `static` property's storage: the identity a
/// [`InstKind::StaticGet`]/[`InstKind::StaticSet`] names, and the constant
/// every request starts it at.
///
/// A static property has **no instance slot** ([`Class::fields`] never lists
/// one), and its storage is not the class's at all: it is one entry in the
/// request's own flat slot vector, whose index is this value's position in
/// [`Program::statics`]. `nvs_runtime::ctx`'s docs own the lifetime — the slot
/// is request-scoped, armed from [`Self::default_value`] when the request
/// starts and released when it ends — and `docs/adr/README.md`
/// § *Decisions taken at project start* owns why it is request-scoped rather
/// than process-global.
///
/// The pair `(class, name)` is the *declaring* class's label and the property's
/// own name, which is exactly what `nvs_types::expr_table::ExprInfo::StaticProperty`
/// carries. A subclass reading an inherited static therefore resolves to the
/// ancestor's entry, with no flattening step: `Sub::$count` and `Base::$count`
/// are one storage, as they are in PHP.
#[derive(Clone, Debug)]
pub struct StaticProp {
    /// The declaring class's rendered label, spelled the way
    /// [`InstKind::FieldGet::class`] is.
    pub class: String,
    /// The property's own name, `$`-sigil not included.
    pub name: String,
    /// What the slot holds, so a read can load the payload half alone — the
    /// same "the declared type settles the representation" rule a field slot
    /// has ([`Class::field_reprs`]).
    pub repr: Ty,
    /// The declared initializer, or `None` for a nullable or `lateinit`
    /// static `rule:classes/definite-property-initialization` required no default of, whose slot starts each
    /// request at `null`.
    pub default_value: Option<nvs_types::FieldDefault>,
}

/// One class's or interface's runtime shape: what an instance's field slots
/// hold, and what it is an instance *of*.
///
/// A straight copy of `nvs_types::layout::ClassLayout`, carried here so that
/// `nvs-codegen` can build the `nvs_runtime::ClassTable` a compiled unit owns
/// without depending on `nvs-types`. The resolution behind it — a subclass's
/// slots following its parent's, a transitive supertype set — happens once, in
/// that module, for the reason its own docs give: it needs `nvs_hir::ClassGraph`,
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
    /// Populated for every class with a layout, plus an `rule:types/object-top` shape
    /// literal's synthesized one: [`InstKind::SlotSet`] reaches any class at
    /// all through § 4's erased receiver, and that write is the one site with
    /// no statically known field type of its own. A slot whose declared type
    /// nothing recorded is [`Ty::Tagged`] rather than absent, so the vector
    /// stays index-aligned with [`Self::fields`] — see `nvs_ir::lower`'s
    /// `field_slots`. `nvs-codegen` maps each entry to the one
    /// `nvs_runtime::Tag` it admits — [`Ty::Tagged`] and [`Ty::Void`] admit
    /// several or none and become "unchecked" — and hands the result to
    /// `nvs_runtime::ClassTable::set_field_tags`, whose own docs state what
    /// that check buys and what it misses.
    ///
    /// **Cost:** one `Ty` per field slot per class at compile time, and one
    /// byte per slot per descriptor at run time — paid once per compiled
    /// unit, not per request (`rule:config/an-edit-reaches-the-next-request-without-a-restart`'s cache).
    pub field_reprs: Vec<Ty>,
    /// Whether each field slot's *declared* type carries `rule:security/secret-qualifier`'s
    /// `secret` qualifier, in [`Self::fields`]' own order — or **empty**,
    /// which means "nothing told this class", never "no slot is `secret`".
    ///
    /// `rule:errors/record-transformations`'s redaction row states one rule about one record, and its property
    /// half cannot be decided anywhere below the checker: `secret` is a
    /// qualifier on a *declared* type, and every representation under it — the
    /// tag, the slot, the allocation — is the same one a plain `string` has.
    /// So the answer is carried rather than recomputed. `nvs-codegen` hands it
    /// to `nvs_runtime::ClassTable::set_secret_fields`, and
    /// `nvs_stdlib::debug`'s walk reads it off the *instance*'s descriptor,
    /// which is what makes a `secret` property redact through an erased view
    /// exactly as through its own type.
    ///
    /// Filled beside [`Self::field_reprs`] by one join — see `nvs_ir::lower`'s
    /// `field_slots`. **Cost:** one `bool` per field slot per class, once per
    /// compiled unit, not per request.
    pub secret_fields: Vec<bool>,
    /// Whether each field slot is readable from outside this class, in
    /// [`Self::fields`]' own order — or **empty**, which means "nothing told
    /// this class", never "no slot is readable".
    ///
    /// A straight copy of `nvs_types::layout::ClassLayout::public_fields`,
    /// which owns why the bit is carried rather than recomputed:
    /// `rule:security/reflection-enforces-visibility`
    /// makes a reflective read face the check ordinary code faces, and the
    /// keyword that decides it exists nowhere below the front end.
    /// `nvs-codegen` hands it to
    /// `nvs_runtime::ClassTable::set_public_fields`, and `nvs_stdlib::reflect`
    /// reads it off the *instance*'s descriptor — the only thing a value whose
    /// class the checker never saw carries.
    ///
    /// **Cost:** one `bool` per field slot per class, once per compiled unit,
    /// not per request.
    pub public_fields: Vec<bool>,
    /// A straight copy of `nvs_types::layout::ClassLayout::protected_fields`,
    /// which owns why the level rather than the readable/not pair is what
    /// travels: `rule:security/reflection-enforces-visibility` gives a
    /// `protected` member to every class in the hierarchy that declares it and
    /// a `private` one to the declaring class alone, and the two are one
    /// answer to [`Self::public_fields`]. `nvs-codegen` hands it to
    /// `nvs_runtime::ClassTable::set_protected_fields`, and
    /// `nvs_runtime::visibility` reads it off the instance's descriptor.
    ///
    /// **Cost:** one `bool` per field slot per class, once per compiled unit,
    /// not per request.
    pub protected_fields: Vec<bool>,
    /// A straight copy of `nvs_types::layout::ClassLayout::constants`, which
    /// owns the roster's flattening rule and why a folded value travels rather
    /// than the declaration's source text. `nvs-codegen` turns each row into a
    /// `nvs_runtime::ConstantDesc` on the class's descriptor, and
    /// `Core\Reflect\ClassInfo::constants` is what reads it back.
    ///
    /// Empty for every class this crate synthesizes — a closure environment, a
    /// generator's state machine, a shape literal's carrier — on
    /// [`Self::public_fields`]' terms exactly: nothing declared one, rather
    /// than a class that declares none.
    ///
    /// **Cost:** one row per declared constant per class, once per compiled
    /// unit, not per request.
    pub constants: Vec<nvs_types::ClassConstant>,
    /// A straight copy of `nvs_types::layout::ClassLayout::attributes`, which
    /// owns why the roster is own-only where the constants above are flattened,
    /// and what a payload field's folded value is.
    /// `nvs-codegen` turns each row into a `nvs_runtime::AttributeDesc` on the
    /// class's descriptor, and `Core\Reflect\ClassInfo::attributes` is what
    /// reads it back.
    ///
    /// Empty for every class this crate synthesizes, on [`Self::constants`]'
    /// terms exactly: nothing wrote a `#[...]` on a closure environment, rather
    /// than a declaration that carries none.
    ///
    /// **Cost:** one row per attach site per class, plus one folded value per
    /// payload field, once per compiled unit, not per request.
    pub attributes: Vec<nvs_types::ClassAttribute>,
    /// Each field slot's declared type as its declaration spells it, in
    /// [`Self::fields`]' own order — or **empty**, on [`Self::public_fields`]'
    /// terms exactly: "nothing told this class", never "every slot is
    /// untyped".
    ///
    /// A straight copy of `nvs_types::layout::ClassLayout::field_types`, which
    /// owns why the name travels rather than the `nvs_runtime::Tag` the
    /// runtime already holds per slot. `nvs-codegen` hands it to
    /// `nvs_runtime::ClassTable::set_field_types`, and
    /// `Core\Reflect\PropertyInfo` is what reads it back.
    ///
    /// **Cost:** one `String` per field slot per class, once per compiled
    /// unit, not per request.
    pub field_types: Vec<String>,
    /// The classes an object written into each field slot must be one of, in
    /// [`Self::fields`]' own order — or **empty**, on [`Self::field_reprs`]'
    /// terms exactly: "nothing told this class".
    ///
    /// `Some` for a slot whose declared type is one or more declared classes
    /// or interfaces and, at most, `null`: `Node`, `?Node`, `A|B`. The labels
    /// are resolved by the checker, so they compare against
    /// `nvs_runtime::ClassDesc::conforms_to_name` where the declaration's
    /// spelling in [`Self::field_types`] could not. `None` for every other
    /// slot, an enum-typed one among them, whose case is an `int` that
    /// [`Self::field_reprs`]' tag already checks. `nvs-codegen` hands it to
    /// `nvs_runtime::ClassTable::set_field_classes`, and
    /// `nvs_runtime::write_erased_property` is what reads it.
    ///
    /// **Cost:** one `Option<Vec<String>>` per field slot per class, once per
    /// compiled unit, not per request.
    pub field_classes: Vec<Option<Vec<String>>>,
    /// Every *other* class and interface an instance of this one also is,
    /// transitively, as labels. Excludes the class itself.
    pub conforms: Vec<String>,
    /// Every method an instance of this class answers, as a
    /// [`crate::MethodEntry`] — a straight copy of
    /// `nvs_types::layout::ClassLayout::methods`, which owns the precedence
    /// rule, the visibility bits and what an empty parameter roster means.
    /// `nvs-codegen` turns each row into the `nvs_runtime::MethodRow` the
    /// runtime descriptor's method table holds, which is what
    /// [`InstKind::CallVirtual`] dispatches through, joining the compiled
    /// address and the callee's declared shape onto it — neither of which
    /// exists until a function has been compiled, which is why only what has
    /// no source below the front end travels here. A parameter's *name* and its
    /// *declared type* are exactly that: [`Function::params`] carries one
    /// representation per slot, which neither spells a name nor separates two
    /// declared types sharing it.
    ///
    /// **Cost:** two `String`s per declared parameter per method per class,
    /// once per compiled unit, not per request.
    pub methods: Vec<crate::MethodEntry>,
    /// Every property hook an instance of this class answers, as `(property
    /// name, hook label, is the `set` accessor)` — a straight copy of
    /// `nvs_types::layout::ClassLayout::hooks`, which owns the precedence rule
    /// and why a hook is not on [`Self::methods`].
    ///
    /// [`Self::methods`]' journey exactly: `nvs-codegen` joins each row's
    /// compiled address on — the label here is the one the hook's own
    /// [`Function`] is emitted under — and the runtime descriptor holds the
    /// result as a `nvs_runtime::HookRow`, so an access through an erased
    /// receiver can run the accessor a statically resolved one calls
    /// directly.
    pub hooks: Vec<(String, String, bool)>,
    /// `rule:core-classes/derive-attribute`'s derived JSON
    /// codec, in declaration order — empty for a class carrying no
    /// `#[Json\Derive]`, which is every class in a program that never writes
    /// the attribute.
    ///
    /// The join of two tables neither crate holds alone:
    /// `nvs_types::derive` reads the attribute, the wire keys and each field's
    /// declared type off the declaration, and `nvs_types::layout` fixes the
    /// slot order — see `crate::lower::lower_file`, which is where the two
    /// meet. Carried through to `nvs_runtime::ClassDesc` so `Core\Json`'s
    /// encoder and decoder can work an instance without asking the program
    /// anything.
    pub codec: Vec<nvs_types::CodecField>,
    /// `rule:core-classes/derive-attribute`'s derived **row**
    /// codec — [`Self::codec`]'s twin for `#[Db\Derive]`, joined to the same
    /// slot order in the same place and carried through to
    /// `nvs_runtime::ClassDesc::db_codec`, which owns why the two lists are
    /// separate rather than one read twice.
    pub db_codec: Vec<nvs_types::CodecField>,
    /// Every field slot that declares an `= expr` default, as `(slot, value)`
    /// in slot order — empty for a class declaring none, which is most of
    /// them.
    ///
    /// The join of the same two tables [`Self::codec`] joins, on the same
    /// terms and in the same place (`crate::lower::lower_program`):
    /// `nvs_types::signatures` evaluated each default against the property's
    /// declared type, and `nvs_types::layout` fixed the slot order. An
    /// ancestor's default lands in the slot that ancestor's property owns,
    /// because a slot is looked up by *name*.
    ///
    /// **This crate emits no instruction for it.** `nvs-codegen` copies it
    /// onto `nvs_runtime::ClassDesc` and `nvs_runtime::NvsObj::new` writes the
    /// slots, which is the only shape that reaches [`InstKind::New`],
    /// [`InstKind::NewDynamic`] and `rule:core-classes/derive-attribute`'s native decoder alike — see
    /// `nvs_types::defaults`, which owns why an initializer cannot be spliced
    /// between allocation and construction.
    pub defaults: Vec<(usize, nvs_types::FieldDefault)>,
    /// How many parameters this class's `constructor` declares — see
    /// `nvs_runtime::ClassDesc::ctor_arity`, which is where it ends up and
    /// which owns why it is carried beside the field list rather than derived
    /// from it. Zero for a class with no codec.
    pub ctor_arity: usize,
    /// Whether this class is a closure literal's environment class — the one
    /// `crate::lower::closure` mints for a `fn (...) { ... }` or a `(...)`
    /// first-class callable, carrying that literal's captures as its fields
    /// and its compiled body as the `invoke` method.
    ///
    /// `nvs-codegen` hands it to `nvs_runtime::ClassTable::set_closure`, and
    /// `nvs_runtime::ClassDesc::is_closure` owns why the runtime carries the
    /// answer rather than testing for the `invoke` in the method table. It is
    /// the same fact `crate::lower::CLOSURE_MARKER` puts in
    /// [`Self::conforms`], reaching the runtime by the one route a descriptor
    /// walk cannot: a class test compares descriptor *addresses*, so the
    /// marker answers `$x is callable` inside the unit that emitted it, while
    /// native code holding a closure from any unit at all asks this bit.
    ///
    /// **Cost:** one `bool` per class per compiled unit, once per unit, not
    /// per request.
    pub is_closure: bool,
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
    /// `nvs_types::check.rs`'s `check_method` seeding `$this` into its own
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
    /// `rule:errors/propagation` error edge
    /// ([`Inst::on_error`]) an instruction in the body carries. Leaving the
    /// second kind out is how a landing block ends up looking unreachable
    /// from a block that plainly branches into it.
    ///
    /// A block may appear more than once — a `Branch` whose two arms are the
    /// same block lists it twice, and so does a call whose error edge is a
    /// landing block another call already named. Callers dedupe if they care;
    /// `nvs-codegen`'s reverse-postorder walk does, by visiting marks.
    #[must_use]
    pub fn successors(&self) -> Vec<BlockId> {
        let mut out: Vec<BlockId> = self.insts.iter().filter_map(|inst| inst.on_error).collect();
        match &self.term {
            Terminator::Return(_) | Terminator::Propagate { .. } => {}
            Terminator::Jump(target) => out.push(*target),
            Terminator::Throw { landing, .. } => out.push(*landing),
            Terminator::Catch { handler, onward } => {
                out.push(*handler);
                out.push(*onward);
            }
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
    /// `rule:errors/propagation`'s error edge:
    /// the landing block a non-`OK` status returned by this instruction
    /// branches to.
    ///
    /// **`Some` for every instruction that returns a status at all**, which is
    /// the rule rather than a list of the ones a *program* can fail in:
    /// [`InstKind::Call`], [`InstKind::CallVirtual`], [`InstKind::New`],
    /// [`InstKind::NewDynamic`], [`InstKind::CoreCall`], every
    /// [`InstKind::HelperCall`], [`InstKind::SlotGet`]/[`InstKind::SlotSet`],
    /// [`InstKind::ArrayGet`] in each of [`crate::ir::AbsentKey`]'s shapes,
    /// [`InstKind::ArrayAppend`], [`InstKind::ArraySpread`], **every
    /// integer arithmetic row** — `+`, `-`, `*`, `/` and `%` over
    /// [`crate::ty::Ty::Int`]/[`crate::ty::Ty::Uint`] as an
    /// [`InstKind::BinOp`], and unary `-` over the same two as an
    /// [`InstKind::UnOp`] — and **`/` over
    /// [`crate::ty::Ty::Float`]**. All of those throw
    /// `rule:types/arithmetic`'s
    /// `ArithmeticError`: the divisions on a zero divisor, and the rest
    /// on overflow, which that section makes a throw rather than a wrap
    /// or a promotion to `float`. The float `/` is on the list because § 4
    /// refuses the zero divisor before the operand types are consulted, so it
    /// is one rule and not two; it is the *only* float row here, every other
    /// float row being total. None of these is a call at all —
    /// `nvs-codegen` tests and raises inline, so this edge is the frame's
    /// cleanup path and nothing else. `None` is for the instructions that
    /// return no status to check: a comparison, the rest of the float rows, an
    /// [`InstKind::Concat`], a constant, a phi, a refcount operation, a
    /// relabelling.
    ///
    /// **A status a `catch` cannot act on still gets a landing block**, and
    /// that is the whole reason the rule is phrased over the status rather
    /// than over what a program can recover from. A conversion helper
    /// (`Helper::IntToString`, the truthy table) fails only by the miscompile
    /// guard `nvs_runtime::helpers` describes, `Helper::Exit`'s *success* is a
    /// non-`OK` status, and a stack-limit or deadline stop can arrive out of
    /// any call at all — each of those is a `FATAL` or an `EXITED` that ends
    /// the request rather than something a `catch` selects. Giving them no
    /// landing block would be cheaper by a cold block per site and would leak
    /// the entire frame every time one fired, which is `O(requests served)`
    /// growth on a shape an attacker can drive (unbounded recursion) and on
    /// one an ordinary CLI program writes (`exit()`). The block is on the
    /// failing edge, so nothing on the request path pays for it.
    ///
    /// [`Terminator::Catch`] is the other half: a landing block *inside* a
    /// `try` releases no local on the way to its handler, so its `onward`
    /// exit — the one an uncatchable status takes — is where those releases
    /// live.
    ///
    /// The landing block it names holds exactly the
    /// [`InstKind::Release`]s this frame owes on the error path — the frame's
    /// live refcounted locals at this instruction's own program point — and
    /// ends in [`Terminator::Propagate`] or [`Terminator::Catch`]. It is one
    /// block per call site rather than one shared per region, so a `catch`
    /// handler's phis get a distinct predecessor per site; see
    /// `crate::lower::Lowering::landing_block`.
    pub on_error: Option<BlockId>,
    /// Where a raise this instruction makes **by itself** says it happened:
    /// the datum `nvs-codegen` bakes into the cold block it raises from, and
    /// the one `nvs_runtime::nvs_raise_new` decodes into both the exception's
    /// `location` and the innermost frame of its backtrace
    /// (`rule:errors/a-record-names-where-it-was-produced`).
    ///
    /// `Some` on two rows, and both take it from the same
    /// `crate::lower::Lowering::source` a `throw` in that statement would take
    /// for its own [`InstKind::SourceConst`], so every spelling of *where*
    /// comes from one datum. The checked arithmetic rows, whose failure is
    /// raised inline rather than returned by a callee, get it from
    /// `crate::lower::Lowering::emit_raising`; [`InstKind::SeedRaiseSite`] gets
    /// it from `crate::lower::Lowering::landing_block`, which is naming the
    /// statement whose call failed rather than a raise of its own.
    ///
    /// `None` everywhere else: an instruction that fails by a callee's status
    /// names its site in that callee's own arguments where it names one at
    /// all, and the loop-counter increment `crate::lower` synthesizes for a
    /// `foreach` has no statement of the program's own to name.
    ///
    /// **The datum, not an operand.** A [`InstKind::SourceConst`] beside the
    /// arithmetic would put a relocated address in the block that *does* the
    /// arithmetic — an instruction spent on the path where nothing throws, and
    /// `rule:errors/throw-is-not-slower` is what prices that path. Carried
    /// here, the blob is baked where the raise is, beside the message bytes
    /// that cold block already bakes.
    pub raise_site: Option<Source>,
}

/// What one [`Inst`] does.
#[derive(Debug)]
#[non_exhaustive]
pub enum InstKind {
    /// `rule:testing/debug-probes`'s statement-boundary probe site: marks where the
    /// previous lowered statement's instructions end and the next one's
    /// begin, in program order. Defines no value.
    StmtMarker(StmtId),
    /// A reserved safepoint poll site — function entry (recursion) or a
    /// loop's back edge, the sites the project-start "safepoints emitted
    /// from the first backend commit" decision names
    /// ([`docs/adr/README.md`](/docs/adr/README.md)'s "Decisions
    /// taken at project start" section) and that ADR 0018 § *Negative*
    /// contrasts its own, denser probe grid against. `nvs-codegen` lowers it
    /// to one load of the context's safepoint word and a predicted-not-taken
    /// branch, and the **function-entry** one — the first in the entry
    /// block — also carries `rule:errors/on-limit`'s call-stack compare, which is why
    /// that ADR calls the site "not a new pass and not a new emit site".
    /// Reserving the shape ahead of any of that is the point: inserting it
    /// after the fact means re-walking every already-lowered
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
    /// A `decimal` constant — `rule:types/numeric-literal-placement`'s untyped-until-placed literal, once a target type has placed it.
    ///
    /// Carried as its parts rather than as the sixteen-byte image
    /// [`crate::ty::Ty::Decimal`] describes, because this crate does not
    /// depend on `nvs-runtime` and that image's bit positions are
    /// `nvs_runtime::decimal`'s one home. `nvs-codegen` depends on both and is
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
    /// The never-written marker — a [`crate::ty::Ty::Tagged`] value carrying
    /// `nvs_runtime::Tag::Unset` over a zero payload, which allocates nothing
    /// and owns nothing.
    ///
    /// [`Self::ConstNull`]'s row for the one constant that is **not a value**:
    /// `rule:core-api/omission-is-not-a-written-null` needs an omitted
    /// nullable option to arrive at a `Core` helper as something a written
    /// `null` cannot also be, and `Tag::Unset` is already defined as distinct
    /// from every legal value including `null`
    /// (`rule:classes/an-unwritten-property-read-throws`).
    ///
    /// Its one producer is an omitted option or shape field whose default is
    /// `nvs_types::defaults::ConstArg::NeverWritten`
    /// ([`crate::lower::Lowering::emit_const_arg`]); no expression produces
    /// one, and nothing hands it back to a program
    /// (`rule:core-api/the-marker-never-reaches-a-program`). It is
    /// [`crate::ty::Ty::Tagged`] rather than a representation of its own
    /// because the tag *is* the whole of it, and a tagged value is the one
    /// representation that carries its own — so the ABI a bag flattens into is
    /// untouched (`rule:core-api/the-bag-abi-is-unchanged`).
    ConstUnset,
    /// A `string` literal's cooked bytes — a fresh [`Ty::Str`] value with
    /// exactly one implicit owner (itself), the same "one natural reference"
    /// starting point [`InstKind::New`] gives a freshly constructed object.
    /// See `crate::lower::cook_str_literal`'s own doc comment for exactly
    /// which escape sequences are cooked and which are a known gap.
    ConstStr(String),
    /// A `bytes` constant's octets — [`InstKind::ConstStr`]'s row under
    /// [`crate::ty::Ty::Bytes`], and the same one heap allocation with one
    /// implicit owner, since the two types share a representation and differ
    /// only in the tag a boxed value carries
    /// (`rule:types/bytes`).
    ///
    /// **No expression produces one**: there is no `bytes` literal in the
    /// language, so this exists for a `Core` signature's optional `bytes`
    /// parameter, whose default a call site materializes
    /// (`nvs_types::defaults::ConstArg::Bytes`). `Core\Bytes::join`'s
    /// `$separator = ""` is such a default.
    ConstBytes(Vec<u8>),
    /// A hole-free `` html`…` ``'s cooked bytes, as the whole
    /// `Core\Html\Markup` they denote: one immortal instance in the unit's own
    /// data section, its class word and its one text slot both relocations
    /// (`nvs_runtime::immortal_object_bytes`), so the literal costs nothing per
    /// execution where the lift it replaces costs an object per evaluation —
    /// `rule:core-classes/html-literal`'s *What it costs to run*.
    ///
    /// [`Ty::Object`], and a fresh value on the same terms
    /// [`InstKind::ConstStr`] is one: the reference it starts with is implicit,
    /// and every retain and release of it is the compare against
    /// `nvs_runtime::IMMORTAL_REFCOUNT` that steps over the word.
    ///
    /// A literal with holes never reaches here. Its bytes are not known until
    /// it runs, so it stays the join and the lift
    /// [`crate::lower::Lowering::lower_markup_literal`] emits.
    ConstMarkup(String),
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
    /// this instruction's own block — `if`/`while`'s join points construct
    /// them; see `crate::lower`'s module docs.
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
    /// fully resolved by `nvs_types::expr_table::ExprTypeTable` before
    /// lowering ever sees it — there is no virtual dispatch to model here,
    /// only "which function does this invoke." See the crate docs' "no
    /// virtual dispatch" known gap for what happens once a receiver's static
    /// and runtime types can actually differ.
    Call {
        /// The resolved target, rendered `"Class::method"` — a label for
        /// `crate::print`/a future codegen symbol table, not itself
        /// resolvable back to a `QName` (this crate never depends on
        /// `nvs-hir`; see `crate::lower`'s module docs).
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
        /// and calls `Animal::constructor`. Only `nvs_types` knows which
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
    /// The `nvs_runtime::ShapeCodec` address a call on
    /// `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`' roster carries beside the
    /// descriptor it was handed: the wire contract of the **inline shape** its
    /// type argument wrote. One `iconst` of that table's address, reached the
    /// way [`InstKind::ClassDescConst`] reaches a descriptor's, and produced at
    /// [`crate::ty::Ty::ClassDesc`] because that is the same engine-owned
    /// address in the same slot spelling — a `Tag::Null` byte over a pointer,
    /// not refcounted, zero meaning "none".
    ///
    /// `None` is a call whose type argument named a **class**, whose contract
    /// is on its own `nvs_runtime::ClassDesc` and needs no second constant, and
    /// it emits that zero. The slot is there either way, so one member reads
    /// one ABI rather than branching on what its call site happened to write.
    ShapeCodecConst {
        /// The [`Program::shape_codecs`] entry this names, by its
        /// [`ShapeCodec::key`].
        shape: Option<String>,
    },
    /// Where a producer was called — `rule:errors/a-record-names-where-it-was-produced`'s
    /// file, one-based line and enclosing `Class::member`, carried to the
    /// producer as `nvs_runtime::source::encode`'s bytes baked into the unit's
    /// own data section.
    ///
    /// One address, reached the way [`InstKind::ShapeCodecConst`] reaches a
    /// codec's table and produced at [`crate::ty::Ty::ClassDesc`] for that
    /// variant's reason exactly — an engine-owned address in the same slot
    /// spelling, a `Tag::Null` byte over a pointer, not refcounted, zero
    /// meaning "none". The blob is self-describing, so no second operand
    /// carries a length: a producer takes the fixed
    /// `rule:errors/propagation` signature, where every slot has to be spelled
    /// in an ABI rather than added at a call.
    ///
    /// `lower::Lowering::source` is the one derivation of what this holds, and
    /// the backtrace label beside it renders off that same one — two spellings
    /// of "where" that agree today is what the rule exists to rule out.
    ///
    /// `None` is a producer reached with no call site to name — the thunk a
    /// callable reference synthesizes, whose record is produced wherever that
    /// callable is later invoked and not where it was written. It emits the
    /// zero word, which `nvs_runtime::source` reads as "none" and the envelope
    /// omits rather than rendering empty, exactly as
    /// [`InstKind::ShapeCodecConst`]'s own `None` does.
    SourceConst {
        /// The datum, exactly as `lower::Lowering::source` built it.
        source: Option<Source>,
    },
    /// What the checker prepared out of the literal a call on
    /// `nvs_stdlib::registry::PREPARED_MEMBERS`' roster was written with,
    /// carried to the member as that roster's argument 0.
    ///
    /// One `iconst` of [`Prepared::code`]'s word, produced at
    /// [`crate::ty::Ty::Int`] because the word *is* the datum: unlike the two
    /// constants above, nothing is baked into the unit's data and no address is
    /// handed over. That is the channel's bound rather than an optimization —
    /// what preparation leaves behind is a fact about the text, and a compiled
    /// automaton belongs to the core that built it.
    ///
    /// `None` is a call whose argument was not a literal, and it emits
    /// `nvs_types::CORE_REGEX_PREPARED_NONE`. The slot is there either way, so
    /// one member reads one ABI rather than branching on what its call site
    /// happened to write — [`InstKind::ShapeCodecConst`]'s own `None` for the
    /// same reason.
    PreparedConst {
        /// The fact, or `None` for the zero word.
        fact: Option<Prepared>,
    },
    /// The [`Ty::ClassDesc`] of the class `object` is actually an instance of
    /// — one load at `nvs_runtime::OBJ_CLASS_OFFSET`, retaining nothing (a
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
    /// Lowers to a `nvs_runtime::nvs_class_method` lookup of `method` on
    /// `lsb`, falling back to `fallback` (the label `nvs_types` statically
    /// resolved, which a class the unit compiled no method table for still
    /// needs), then an indirect call through the ordinary
    /// `rule:errors/propagation` signature — so
    /// every probe, status check and landing block is identical to
    /// [`InstKind::Call`]'s. Ownership is identical too: `receiver` and each
    /// argument are transferred, and the callee releases them.
    ///
    /// Deliberately *not* what an ordinary `$obj->method(...)` lowers to: that
    /// stays statically resolved (`nvs-codegen`'s known gap 1), so nothing on
    /// the hot path pays for a name lookup. See `nvs_runtime::object`'s docs
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
        /// back to. Codegen substitutes `nvs_runtime::nvs_abstract_method`,
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
        /// The `Class::constructor` label `nvs_types` resolved for the
        /// statically known class, or `None` if nothing in that chain declares
        /// one. Used as the lookup's fallback; the lookup itself always asks
        /// the allocated class first, so a subclass's own constructor wins.
        ctor: Option<String>,
        /// Each constructor argument, already lowered — empty when `ctor` is
        /// `None`.
        args: Vec<ValueId>,
    },
    /// `rule:types/class-reference`'s checked way *into* a `class<T>`: the [`Ty::ClassDesc`] of the
    /// class `subject` denotes, or a null one when it denotes no class that is
    /// a `base`. Both of § 2's rows are this one instruction, told apart by
    /// `subject`'s own representation:
    ///
    /// * a [`Ty::Str`] subject is the **string door** — the name must be
    ///   `base` or a class that is a `base`;
    /// * a [`Ty::ClassDesc`] subject is the **narrowing** `class<U>` →
    ///   `class<T>` — the descriptor in hand must already be one of those.
    ///
    /// One instruction rather than two because the answer is the same set
    /// either way, and the set is what the instruction is: **`nvs-codegen`
    /// enumerates the classes this unit declares that conform to `base`** and
    /// compares the subject against each — `nvs_runtime::nvs_str_eq` per
    /// candidate for a name, one `icmp` per candidate for a descriptor. That
    /// closed set is `rule:types/class-reference`'s own argument for why a `tainted` string may
    /// pass through the conversion at all: the output range is the classes the
    /// program's own source declares to be `base`s, and nothing a caller writes
    /// can widen it.
    ///
    /// **What it costs:** O(subclasses of `base` in the unit) compares, branch
    /// free, on a path that is a dynamic factory lookup — priority 3 spent
    /// where a hash would have bought a per-unit table and a per-site
    /// relocation to reach it. A hierarchy with enough implementors for that to
    /// matter is the trigger to bake a table instead, and nothing above this
    /// instruction would change.
    ///
    /// The null is not a value the language admits: [`crate::lower`] tests for
    /// it and throws, exactly as `rule:types/conversion`'s other checked rows do, so no
    /// consumer of a [`Ty::ClassDesc`] ever sees one. `Dog::class as
    /// class<Animal>` never reaches here at all — both sides are written out,
    /// so § 2 decides it where it stands and it lowers to
    /// [`InstKind::ClassDescConst`].
    ClassDescIn {
        /// The name or the descriptor to resolve — a [`Ty::Str`] or a
        /// [`Ty::ClassDesc`], and nothing else.
        subject: ValueId,
        /// The class the answer must conform to, rendered the same way
        /// [`InstKind::New::class`] is.
        base: String,
    },
    /// Reads a compile-time-known field off an object — `$obj->prop` whose
    /// receiver's static type resolved to a known declaring class (an
    /// `nvs_types::expr_table::ExprInfo::Property` entry exists for it). No
    /// actual byte offset is computed here: `class`/`field` are labels for a
    /// future codegen layout pass, the same "resolved identity, not yet a
    /// machine offset" shape `Call`/`New`'s own `target`/`class` labels
    /// already use. A property access whose receiver erased to a shape or to
    /// plain `object` (`rule:types/erased-member-access`) never reaches here: it has no declaring
    /// class to name, so the checker records an
    /// `nvs_types::expr_table::ExprInfo::ShapeProperty` instead and
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
    /// exactly the same [`ExprInfo::Property`](nvs_types::expr_table::ExprInfo::Property)
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
    /// Reads a `static` property — `Class::$prop`, `self::$prop`, and every
    /// other spelling of the same storage.
    ///
    /// `class`/`name` are the **declaring** class's label and the property's
    /// own name, exactly the resolved identity
    /// `nvs_types::expr_table::ExprInfo::StaticProperty` carries — they stay
    /// labels here rather than becoming a slot number for
    /// [`InstKind::FieldGet`]'s reason, and `nvs-codegen` resolves the pair
    /// through [`Program::statics`] the same way it resolves a field name
    /// through [`Class::fields`]. What reaches the machine is a constant
    /// offset into the request's slot vector, which is the whole difference
    /// from [`InstKind::SlotGet`]'s name-keyed fetch: a static read costs two
    /// loads — the base out of the context, the payload out of the slot — and
    /// no call.
    ///
    /// **Borrows, exactly like [`InstKind::FieldGet`].** The slot keeps its
    /// one reference and this takes none, so a consumer that keeps the value
    /// retains it — `crate::lower::is_aliasing_read` lists
    /// `ExprKind::StaticPropertyAccess` for that reason.
    StaticGet {
        /// The declaring class's label.
        class: String,
        /// The property's own name, `$`-sigil not included.
        name: String,
    },
    /// Writes a `static` property — [`InstKind::StaticGet`]'s counterpart, and
    /// [`InstKind::FieldSet`]'s refcounting policy unchanged: the lowering
    /// retains the incoming value if it is an aliasing read, reads the slot's
    /// previous value back with a [`InstKind::StaticGet`] and releases it, and
    /// only then stores. Defines no value.
    StaticSet {
        /// The declaring class's label.
        class: String,
        /// The property's own name, `$`-sigil not included.
        name: String,
        /// The new value, already lowered.
        value: ValueId,
    },
    /// Reads a slot off an object **by index** — `$issue->path`, whose
    /// receiver is an `rule:types/erased-member-access` shape rather than a named class.
    ///
    /// The difference from [`InstKind::FieldGet`] is that the layout is not
    /// known here. A shape is anonymous and methodless, so there is no class
    /// label for codegen to resolve an offset through — and the receiver's
    /// *static* shape need not be the concrete value's own: `rule:types/shape-type`'s
    /// width subtyping lets a `{x: int, y: int}` reach a `{y: int}`
    /// parameter, where the two lay their slots out differently. So this is
    /// § 4's **name-keyed fetch**, `nvs_runtime::nvs_object_slot_get`, and not
    /// a fixed offset; § 4 says so outright, and defers the per-call-site
    /// specialization that would make it one to that ADR's *Revisiting*.
    ///
    /// [`Self::SlotGet::slot`] is carried anyway, as a *hint*: the receiver's own
    /// shape is the overwhelmingly common case, and where it holds, the
    /// runtime's lookup is one name comparison rather than a scan.
    ///
    /// **What an absent field answers is [`AbsentKey`]**, the same enum
    /// [`InstKind::ArrayGet`] carries one storage kind along. Under
    /// [`AbsentKey::Throws`] — every read written outside a guard — a name
    /// the concrete class does not carry is § 4's catchable throw, and the
    /// result is the field's own representation. Under [`AbsentKey::Null`] —
    /// a read under a `??`, an `isset` or an `empty`, which is the only way
    /// `rule:types/shape-type`'s optional field is reachable without one — it
    /// answers `null`, and the result is [`Ty::Tagged`] however narrow the
    /// field's declared type is. `nvs-codegen` picks
    /// `nvs_runtime::nvs_object_slot_get` or
    /// `nvs_runtime::nvs_object_slot_optional_get` off this field; the two
    /// take the same arguments, so one `RuntimeSig` covers both.
    ///
    /// **Fallible either way**, so both are emitted through
    /// `crate::lower::Lowering::emit_fallible` and carry `rule:errors/propagation`'s error
    /// edge like any call: the two refusals that are not about presence — a
    /// receiver that is not an object, and a slot that was never written —
    /// throw under both answers. A field the receiver's static shape lists as
    /// *required* reaches neither, being proven present; it is the erased
    /// half of § 4 and the optional field that arrive here.
    ///
    /// **The receiver may be a [`Ty::Tagged`], and the runtime checks its
    /// tag.** `rule:types/conversion`'s `mixed` is an erased receiver like a plain
    /// `object` is, with the one difference that nothing proved it holds an
    /// object at all — so `crate::lower::expr`'s `ReceiverProof::Erased` emits no
    /// [`InstKind::Untag`] for it and the whole value travels here. A
    /// non-object receiver is a catchable throw in PHP's own wording; every
    /// *statically* non-object receiver is `E0495` at the checker.
    ///
    /// Borrows its receiver exactly as [`InstKind::FieldGet`] does: the slot
    /// keeps owning what it holds, so a consumer that outlives the receiver
    /// owes the read value a retain.
    SlotGet {
        /// The receiver, already lowered — a [`Ty::Object`], or a
        /// [`Ty::Tagged`] whose tag this instruction checks.
        object: ValueId,
        /// The field's own name, `$`-sigil not included — what the fetch is
        /// actually keyed on.
        field: String,
        /// The field's position in the *receiver's static* shape, sorted by
        /// name: a hint, not the answer, and `0` where the receiver is a
        /// plain `object` with no static shape to take a position from. See
        /// this variant's own docs.
        slot: u32,
        /// What this read answers when the receiver's concrete class carries
        /// no field of that name.
        absent: AbsentKey,
    },
    /// `$x is {path: string}`'s presence question, and the one object access
    /// that **cannot fail**: does the receiver carry a readable field of this
    /// name, answered as a [`Ty::Bool`]?
    ///
    /// One call to `nvs_runtime::nvs_object_slot_probe`, keyed on the **name**
    /// and taking [`Self::SlotProbe::slot`] as the same hint, for the reasons
    /// [`InstKind::SlotGet`] gives for both. It answers `true` exactly when
    /// that instruction under [`AbsentKey::Throws`] would answer a value rather
    /// than throw, so `probe`-then-`get` is a total pair and the read's error
    /// edge on the proven side is unreachable — the IR carries it anyway,
    /// holding no proof to erase it with.
    ///
    /// **Infallible, so no error edge.** `rule:types/type-test` makes `is`
    /// total, and the three states answering `false` here are the three
    /// [`InstKind::SlotGet`] throws on: a receiver holding no object, a
    /// concrete class carrying no field of that name, and a slot that was never
    /// written. `nvs-codegen` emits no status check for it, and
    /// `crate::lower::Lowering::emit` needs no landing pad.
    ///
    /// **Why not a third [`AbsentKey`].** That enum says what an absent *read*
    /// answers, and every arm of it answers the field's own value; this asks a
    /// different question and answers a `bool`. Nor could the read hand absence
    /// back as a value instead: `nvs_runtime::Tag::Unset` is a storage state
    /// that never becomes an expression's value, and [`AbsentKey::Null`] cannot
    /// tell an absent `{a: ?int}` from an `a` holding `null` — the distinction
    /// `rule:types/type-test`'s shape row exists to make.
    ///
    /// Borrows its receiver and allocates nothing: one call, and one name
    /// comparison where the hint holds. The shape walk spends that once per
    /// field, which is `rule:types/type-test`'s stated O(n).
    SlotProbe {
        /// The subject, already lowered — a [`Ty::Object`], or a [`Ty::Tagged`]
        /// whose tag this instruction checks and answers `false` for.
        object: ValueId,
        /// The field's own name, `$`-sigil not included — what the probe is
        /// keyed on.
        field: String,
        /// The field's position in the shape being *tested against*, sorted by
        /// name: a hint, not the answer, and wrong for the reason
        /// [`InstKind::SlotGet`]'s is — the subject's own class is not the shape
        /// this asks about.
        slot: u32,
    },
    /// `$issue->path = "x";` — [`InstKind::SlotGet`]'s write half, and
    /// `rule:types/erased-member-access`'s other paragraph: one call to
    /// `nvs_runtime::nvs_object_slot_set`, keyed on the **name** and taking
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
    /// compares is the tag; `nvs_runtime::object`'s module docs
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
        /// The receiver, already lowered — a [`Ty::Object`], or a
        /// [`Ty::Tagged`] whose tag this instruction checks, exactly as
        /// [`InstKind::SlotGet`]'s does.
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
    /// `$obj->$key` —
    /// `rule:types/property-key-access`'s keyed read, and [`InstKind::SlotGet`] with the name arriving as a
    /// **value** instead of as a `String` this instruction carries.
    ///
    /// That is the whole difference, and it is § 5's recorded choice: a key is
    /// a name, so what the access needs is exactly the by-name lookup on the
    /// receiver's concrete descriptor that `rule:types/erased-member-access`'s erased read already
    /// performs — one call, no allocation, and the same catchable throw for a
    /// name the concrete class does not carry. The alternative § 5 weighed was
    /// a closed-set chain over the key's roster, one `BinOp::Eq` per name and
    /// an ordinary [`InstKind::FieldGet`] per arm joined by a
    /// [`InstKind::Phi`]; it needs no new instruction, and it spends a
    /// comparison per property and a block per property *at every access* to
    /// arrive at the lookup this one does in a call. It would also have to tag
    /// each arm's read into the union's representation before the join, so the
    /// static-read advantage it looks like it buys is not there.
    ///
    /// **Per-property hooks and a declared `PropertyObserver` therefore behave
    /// exactly as they do on `rule:types/erased-member-access`'s erased path**, which is what § 5
    /// says and not a second answer: the erased access's own known gap — it
    /// reaches storage past a per-property `get`/`set` hook — is recorded on
    /// `nvs_runtime::nvs_object_key_get` and closes for every caller at once.
    ///
    /// Borrows its receiver and its key exactly as [`InstKind::SlotGet`]
    /// borrows its receiver: the slot keeps owning what it holds, so a consumer
    /// that outlives the receiver owes the read value a retain.
    KeyGet {
        /// The receiver, already lowered — a [`Ty::Object`], or a
        /// [`Ty::Tagged`] whose tag the runtime checks, exactly as
        /// [`InstKind::SlotGet`]'s does.
        object: ValueId,
        /// The member name, as a [`Ty::Str`] value: a `property<T>` erases to
        /// one (`crate::lower::erase_checked_ty`), so the key *is* this string
        /// and no conversion stands between them.
        key: ValueId,
    },
    /// `$obj->$key = v;` — [`InstKind::KeyGet`]'s write half, which is
    /// [`InstKind::SlotSet`] with the same one substitution, and `rule:types/property-key-access`'s
    /// "a write is `rule:types/erased-member-access`'s checked erased store".
    ///
    /// Every rule [`InstKind::SlotSet`] states holds here for its reasons: the
    /// name is resolved on the receiver's own descriptor, **no field is ever
    /// created**, the incoming value is checked at run time against what the
    /// concrete class declares that property to hold, what the slot held is
    /// released by the runtime, and the stored value is **borrowed** rather
    /// than transferred because the write can throw with both operands in hand.
    /// Defines nothing: a [`Ty::Void`] result, like a statement call.
    KeySet {
        /// The receiver — see [`InstKind::KeyGet::object`].
        object: ValueId,
        /// The member name, as a [`Ty::Str`] value — see
        /// [`InstKind::KeyGet::key`].
        key: ValueId,
        /// The value to store, already coerced to the union's static
        /// representation. Borrowed; see this variant's own docs.
        value: ValueId,
    },
    /// `$obj is Class` — one linear scan of the receiver's flattened
    /// supertype set, defining a [`Ty::Bool`].
    ///
    /// `class` is usually a label into [`crate::ir::Program::classes`],
    /// exactly like [`InstKind::New::class`], and it may name an *interface*
    /// as readily as a class: `nvs_types::layout` gives an interface a
    /// descriptor with no slots for precisely this test (and for a typed
    /// `catch`, which lowers to the same instruction). `rule:types/type-test`'s
    /// value arm `$x is $cls` supplies a [`Ty::ClassDesc`] value in its place
    /// and asks the identical question — [`TestedClass`] owns why the two
    /// forms are one instruction. Reads `value` without retaining it, the way
    /// [`InstKind::FieldGet`] reads its receiver.
    ///
    /// **The subject may be a [`Ty::Tagged`], and the tag is checked at run
    /// time.** A `mixed` or an untested `?Box` is what this instruction is
    /// for, so `nvs-codegen` passes such a subject as a whole value by
    /// address — the same shape [`InstKind::SlotGet`]'s receiver takes — and a
    /// tag that is not an object answers `false`. Unlike the name-keyed fetch
    /// there is nothing to throw about: the question was "is it one", not
    /// "read a field off it". A subject whose *declared* type can hold no
    /// object is folded to a constant at the checker instead
    /// (`rule:types/type-test`), so this never sees a scalar representation.
    ClassTest {
        /// The subject, already lowered — a [`Ty::Object`], or a
        /// [`Ty::Tagged`] whose tag this instruction checks.
        value: ValueId,
        /// The class or interface tested against: a name written at the site,
        /// or the descriptor a `class<T>` operand evaluated to. See
        /// [`TestedClass`].
        class: TestedClass,
    },
    /// `.` string concatenation: builds a fresh [`Ty::Str`] value from the
    /// cooked bytes of every piece, each already [`Ty::Str`] by the time this
    /// instruction sees them — see `crate::lower::Lowering::concat_operand`,
    /// which converts a scalar operand through an [`InstKind::HelperCall`]
    /// first and an object operand through the `toString`
    /// `nvs_types::expr::operators::require_stringable` resolved for it.
    /// Modeled as a dedicated instruction rather than a runtime-helper call
    /// itself, the same "native instruction over already-typed operands"
    /// treatment [`InstKind::BinOp`] already gives scalar arithmetic — `.` only
    /// ever needs this one fixed shape, unlike the open-ended, enum-tagged set
    /// [`HelperCall`](InstKind::HelperCall) exists for.
    ///
    /// **N-ary, not binary, and that is what makes it one allocation.** `.` is
    /// left-associative and an interpolated string is a run of pieces, so both
    /// producers could fold into a chain of two-operand `Concat`s — where every
    /// link of that chain allocates a buffer holding the accumulation so far
    /// and copies it, so an n-piece concatenation allocates n-1 buffers and
    /// copies its leading pieces n-1 times. One instruction carrying every
    /// piece is one allocation, sized once, with each piece copied once:
    /// `crate::lower::Lowering::lower_concat` flattens the `.` spine and
    /// `crate::lower::Lowering::lower_interpolated_parts` hands its pieces over
    /// whole. `nvs_runtime`'s `nvs_str_concat_n` is the entry point, with the
    /// two-piece case kept on `nvs_str_concat` because it needs neither the
    /// stack array nor the count.
    ///
    /// `pieces` always holds **two or more**. A single-piece interpolation
    /// emits no `Concat` at all — `lower_interpolated_parts`' own doc comment
    /// says what it does instead — and nothing else produces one.
    ///
    /// **Consumes one reference to the leading piece and yields one to the
    /// result**, on [`InstKind::ArraySet`]'s protocol, which that variant's own
    /// doc comment is the worked statement of. Every other piece is only
    /// *read*, exactly the way [`InstKind::FieldGet`] reads its `object`
    /// receiver without retaining it, so ownership of each stays wherever it
    /// already was.
    ///
    /// That one hand-off is what makes `$s = $s . $x` linear rather than
    /// quadratic: `nvs_runtime`'s concatenation primitives write into the
    /// leading buffer whenever nothing else holds it, and sole ownership is the
    /// whole of what makes that write unobservable — so the reference has to
    /// arrive here rather than stay in the slot it came from. It is supplied
    /// three ways, and `crate::lower::Lowering::emit_concat` is the one home
    /// for the choice between the first two: a piece no durable slot owns (a
    /// literal, a nested `Concat`'s own result, a freshly converted
    /// [`HelperCall`](InstKind::HelperCall) result) is **handed over** — the
    /// release `crate::lower::Lowering::owned_temporaries` would have emitted
    /// after this instruction is what pays for it; a piece a slot does own is
    /// **retained** first, so the slot keeps its own and this consumes the
    /// extra; and `$s = $s . e`, where the assignment re-points `$s` at the
    /// result anyway, hands the binding's own reference over with neither a
    /// retain nor a release — `crate::lower::Lowering::lower_string_self_concat`,
    /// which is [`InstKind::StrAppend`]'s bookkeeping over this instruction.
    ///
    /// Every later piece that no durable slot owns is still released right
    /// after this instruction reads it, since nothing else ever will.
    ///
    /// The result owns exactly one reference and it is this instruction's
    /// caller's, whether the runtime allocated a fresh buffer or wrote into the
    /// leading piece's own — so `crate::lower::is_aliasing_read` stays `false`
    /// for `ExprKind::Binary`, same as it already is for
    /// [`InstKind::ConstStr`]/[`InstKind::New`]/[`InstKind::Call`].
    Concat {
        /// The operands in evaluation order, each already lowered and already
        /// [`Ty::Str`]. Two or more. One reference to the first is consumed.
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
    /// one are that same one slot's. `nvs_runtime`'s `nvs_str_append` owns
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
    /// only way an Novis binding ever names an exception this frame did not
    /// construct itself.
    ///
    /// The value may be **null**: a runtime helper's bare-message failure has
    /// no object behind it unless the driver installed a class to build one
    /// from (`nvs_runtime::Ctx::set_runtime_error_class`). Every operation the
    /// dispatch performs on it tolerates that — [`InstKind::ClassTest`]
    /// answers `false`, so no clause matches and the throw is re-raised.
    ///
    /// Defined as an instruction rather than a [`Helper`] call for the same
    /// reason [`InstKind::Concat`] is one: it has a single fixed shape, takes
    /// no Novis operand, and cannot fail — so it needs neither the argument
    /// list nor the status check `HelperCall` exists to carry.
    TakeThrown,
    /// Names the site a pending failure was raised at, for the one shape that
    /// records none: an exception caught in the frame it happened in.
    ///
    /// A backtrace is built from the frames an exception *leaves*
    /// ([`Terminator::Propagate`]), and a raise compiled code makes renders
    /// its own frame from [`Inst::raise_site`]. A helper raising its own fault
    /// does neither, so the one landing site where no frame is ever pushed —
    /// the catchable edge of [`Terminator::Catch`] — is where that exception
    /// would reach a `catch` naming nowhere. This instruction is emitted
    /// there, and nowhere else, carrying the enclosing statement's site in
    /// [`Inst::raise_site`] exactly as the checked arithmetic rows carry
    /// theirs.
    ///
    /// Defines no value and cannot fail. `nvs_runtime::nvs_raise_site` is the
    /// whole of what it lowers to, and it writes only where nothing else
    /// already named a frame — so a `throw` caught beside itself and an
    /// exception arriving from a callee both pass through it untouched.
    ///
    /// **It sits on the caught edge rather than in the landing block**, so
    /// neither the path where the call succeeded nor [`Terminator::Catch`]'s
    /// `onward` exit runs it: a `FATAL` leaving the frame is not a `Throwable`
    /// and has no backtrace to seed (`rule:errors/escalation-ladder`).
    SeedRaiseSite,
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
    /// `rule:classes/clone-is-shallow`'s shallow, same-heap, single-level copy. Defines a fresh
    /// [`crate::ty::Ty::Object`] with exactly one natural owner, exactly like
    /// [`InstKind::New`], whose every slot holds what the original's held with
    /// one more reference taken.
    ///
    /// No hook runs and nothing can fail: `rule:classes/two-copy-depths` has no `__clone`, so this
    /// carries no status check and no landing block, the same as
    /// [`InstKind::Concat`]. Reads `object` without retaining it, the way
    /// [`InstKind::FieldGet`] reads its receiver.
    Clone {
        /// The object being copied.
        object: ValueId,
    },
    /// Defines a value with the *same machine bits* as `operand` under a
    /// different [`crate::ty::Ty`] — the whole of a conversion that
    /// `rule:types/conversion` calls
    /// "total, free ... same representation, reinterpreted": an enum to its
    /// backing `int`/`uint`, `rule:types/conversion`'s `string as bytes`, and one shape
    /// that is not a language-level conversion at all — a
    /// [`crate::ty::Ty::ClassDesc`] read as an `int` so that
    /// [`InstKind::ClassDescIn`]'s null answer can be *compared*. A descriptor
    /// is an address and an `int` is the machine word holding one, so that
    /// relabelling is the same free one the rows above are; what it buys is a
    /// null test written with `BinOp::Eq` rather than a fourth instruction
    /// whose only content would be a comparison the IR already has.
    ///
    /// Emitted rather than simply relabelling the operand in `crate::lower`
    /// because an IR value's representation is a property of the instruction
    /// that *defined* it — two names for one definition would mean two
    /// answers to [`Inst::ty`] for the same [`crate::ids::ValueId`], and
    /// `nvs-codegen`'s value map has exactly one slot per id.
    ///
    /// The two representations must share a Cranelift type, which
    /// `nvs-codegen` asserts: this is a relabelling, never a bit cast, so it
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
    /// (`nvs_codegen::ty::tag_of`).
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
    /// only emits an `Untag` where `nvs_types` has already proved which
    /// representation the value holds — the non-`null` arm of a `??`, a
    /// `?->` or an `if ($x !== null)` narrowing. A runtime *test* is
    /// [`InstKind::IsNull`], and a conversion that can genuinely fail is an
    /// `rule:types/conversion` checked row, not this.
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
    ///
    /// A [`crate::ty::Ty::Object`] operand is admitted too, and is the same
    /// compare one representation down: an object is a bare pointer and a
    /// null one *is* `null` (`nvs_runtime::object`'s own decision). Its one
    /// producer is `nvs_ir::lower`'s `emit_never_written_guard`, where the
    /// null pointer is `rule:classes/an-unwritten-property-read-throws`'s never-written slot rather than a value
    /// any expression produced.
    IsNull {
        /// The tagged value being tested.
        operand: ValueId,
    },
    /// Whether a [`crate::ty::Ty::Tagged`] value's tag is the one `repr`
    /// carries once it is materialized (`nvs_codegen::ty::tag_of`) —
    /// [`crate::ty::Ty::Bool`], one masked compare, no call.
    ///
    /// `rule:types/type-test`'s `$x is T` for every row a tag settles by
    /// itself: a scalar, `null`, plain `object` and a bare `array`. A class, a
    /// named element type, a literal and an enum case each need more than a
    /// tag — the descriptor walk [`Self::ClassTest`] emits, the element walk
    /// `as array<T>` already emits, or a payload compare — and none of those
    /// replaces this compare, they follow it. A subject that is not
    /// [`crate::ty::Ty::Tagged`] never reaches here at all: it carries exactly
    /// one tag, so `crate::lower` answers it as a constant.
    ///
    /// It **borrows** its operand exactly as [`Self::IsNull`] does: no retain,
    /// no release, no ownership transfer, no status and no landing block.
    ///
    /// The tag is the **low byte** of the tag word rather than the whole word,
    /// which is the one way this is not `IsNull` with a different immediate: a
    /// `decimal` spells its scale and sign in the same word
    /// (`nvs_runtime::decimal`), so a compare against the word would answer
    /// `false` for every `decimal` that is not zero-scaled and positive.
    TagIs {
        /// The tagged value whose tag is read.
        operand: ValueId,
        /// The representation whose tag it must carry.
        repr: Ty,
    },
    /// Invokes one of a small, closed, engine-owned set of runtime
    /// conversions — one of the milestone's named ingredients. `helper` is a
    /// fixed [`Helper`] tag, never a resolved
    /// class/method name: unlike [`InstKind::Call`]'s `target`, nothing here
    /// comes from `nvs_types::expr_table::ExprTypeTable` or a class
    /// hierarchy, so there is no receiver, no virtual dispatch question, and
    /// no reason to share `Call`'s shape (see `crate::lower`'s module docs'
    /// design-choices section for why a dedicated instruction was chosen
    /// over reusing `Call` with a synthetic target label, and an enum tag
    /// over a string name). Like [`InstKind::Call`]/[`InstKind::New`], this
    /// does not yet model `rule:errors/propagation`'s checked-return convention — no status,
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
    /// value — `rule:types/arrays`'s insertion-ordered, string-keyed hash. A
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
        /// `(key, value)` pairs, in insertion order — `rule:types/arrays`'s
        /// "iteration order is insertion order, always." Always empty for a
        /// literal with an explicit `key =>` element; see above.
        entries: Vec<(String, ValueId)>,
    },
    /// Reads the element at `key` off `array` — `$arr[$i]`, whose base
    /// statically resolved to a known `array<T>` element type (an
    /// `nvs_types::expr_table::ExprInfo::Index` entry exists for it; see
    /// `crate::lower::Lowering::lower_expr`'s `Index` arm). `key` is either a
    /// [`crate::ty::Ty::Str`], or a
    /// [`crate::ty::Ty::Int`] for a subscript that was already an `int` and
    /// therefore never rendered — which is the crate docs' *an array key is
    /// a `string`, and an `int` subscript does not spell it*. `rule:types/arrays`'s key normalization is untouched (`$a[8]` is `$a["8"]`);
    /// the decimal is simply never produced. A `uint` subscript is
    /// rendered by `crate::lower::Lowering::lower_array_key`, with the
    /// exact [`Helper::UintToString`] conversion
    /// [`Lowering::concat_operand`](crate::lower::Lowering::concat_operand)
    /// already gives `.`'s scalar operand rather than a new policy; that
    /// function's own doc comment says why an `i64` index cannot carry it.
    ///
    /// **What an absent `key` answers is [`AbsentKey`]**, and it decides the
    /// rest of this instruction's shape. Under [`AbsentKey::Throws`] — every
    /// read written outside a guard — it is a *fallible* instruction carrying
    /// `rule:errors/propagation`'s error edge like a call: emitted through
    /// `crate::lower::Lowering::emit_fallible`, given the same status check
    /// every helper call gets by `nvs-codegen`, against the runtime entry
    /// point `nvs_array_required_get` — which is why the two representations
    /// above are told apart there, by the key's own tag, rather than by
    /// picking a symbol here. PHP warns and yields `null`; `rule:php-migration/every-divergence-is-deliberate-and-listed` row 11
    /// records the divergence, and that helper's doc comment says why yielding
    /// `null` there would be a null dereference rather than a value. A stored `null` is
    /// *not* an absent key and reads back unchanged. Under
    /// [`AbsentKey::Null`] it is infallible, its result is
    /// [`crate::ty::Ty::Tagged`] whatever the element type is, and the entry
    /// point is `nvs_array_optional_get`. The write side asks the
    /// same question and answers it differently — an absent key vivifies —
    /// which is what [`Helper::ArrayRowForWrite`] exists for. Reads `array`
    /// without
    /// retaining it, the same way `FieldGet` reads its `object` receiver — a
    /// caller copying the result into a second durable slot retains it
    /// there instead (`crate::lower::is_aliasing_read` also matches
    /// `ExprKind::Index`, so the `bind_local`/`lower_call_args`/
    /// `StmtKind::Return` insertion points already do this with no call site
    /// of their own).
    ArrayGet {
        /// The array, already lowered.
        array: ValueId,
        /// The lookup key, already lowered — `Ty::Str` or `Ty::Int`.
        key: ValueId,
        /// What this read answers when `key` names no entry.
        absent: AbsentKey,
    },
    /// Writes `value` at `key` into `array` — `$arr[$i] = expr;`, inserting a
    /// fresh entry when `key` isn't already present and overwriting (per ADR
    /// 0007 § 5's copy-on-write value semantics, releasing whatever it
    /// already held) otherwise. Unlike [`InstKind::FieldSet`], which reads
    /// the field's *previous* value back with a [`InstKind::FieldGet`] before
    /// releasing it — safe there because a declared field always exists on a
    /// definitely-initialized instance (`rule:classes/definite-property-initialization`) — an array key may or may
    /// not already be present, so this instruction bundles the entire
    /// replace-or-insert operation rather than splitting it into a get/
    /// release pair the way `FieldSet` does: no codegen exists yet to make
    /// that split observable, and modeling a conditional get here would mean
    /// guessing at PHP's own missing-key behavior at the one place — an
    /// *ordinary* new-key insert — where nothing should be missing to begin
    /// with. `key` is `Ty::Str` or `Ty::Int`, the same representations
    /// [`InstKind::ArrayGet`]'s own doc comment describes.
    /// `crate::lower::Lowering::lower_reassignment`'s `Index`-target arm
    /// retains `key`/`value` first when either is
    /// [`crate::ty::Ty::is_refcounted`] and an aliasing read (the same
    /// caller-side retain a call argument/array-literal element already
    /// gets) — the array durably owns both after this instruction runs.
    ///
    /// **Defines a fresh [`crate::ty::Ty::Array`] value: the array that now
    /// holds the entry.** `rule:types/arrays`'s copy-on-write value semantics mean a
    /// write into an array a second binding also holds must separate, which
    /// produces a *different* allocation — so this instruction consumes one
    /// reference to `array` and yields one reference to the result, which is
    /// the same pointer whenever `array` was solely owned. `nvs_runtime`'s
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
    /// — `$a[] = expr;`, `rule:types/arrays`'s append syntax
    /// (`crate::lower::Lowering::lower_reassignment`'s `Index`-target arm,
    /// reached when the target's subscript is `None`). Unlike
    /// [`InstKind::ArraySet`], no key is lowered or carried here at all: PHP's
    /// real rule tracks "the highest `int` key ever used, plus one" as part of
    /// the array's own runtime state (surviving explicit `int`-keyed inserts,
    /// removals, and earlier appends alike), which is genuinely a property of
    /// the array value itself, not something a lowering pass can compute from
    /// the source text the way a literal's positional index or an explicit
    /// key already can. This instruction leaves that counter's storage and
    /// increment entirely to whatever `nvs-codegen`'s own array representation
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
    /// append whose next integer key is already live, and `nvs_runtime::array`
    /// § *the append is the one array write with a fault channel* owns both
    /// the refusal and what it leaves each operand's reference holding.
    ArrayAppend {
        /// The array, already lowered.
        array: ValueId,
        /// The value to append, already lowered.
        value: ValueId,
    },
    /// Copies every entry of `subject` into `array` — the `...$a` element of
    /// an array literal (`rule:types/arrays`), lowered by
    /// `crate::lower::Lowering::lower_array_literal`.
    ///
    /// One instruction rather than a lowered loop over
    /// [`InstKind::ArrayNextSlot`]/[`InstKind::ArrayKeyAt`]/[`InstKind::ArrayValueAt`]:
    /// the copy is a whole-array operation with no user code inside it, so a
    /// lowered loop would spend three calls and a branch per entry to express
    /// what the runtime already walks in one pass, and it would put a
    /// control-flow join inside an *expression* that has none.
    ///
    /// **Which key survives is a language rule, not a representation
    /// detail**, and `nvs_runtime::nvs_array_spread` is its one home: a key
    /// that reads as a canonical decimal integer is renumbered under this
    /// array's own append counter, and every other key is preserved in place.
    ///
    /// **Defines a fresh [`crate::ty::Ty::Array`] value**, on exactly
    /// [`InstKind::ArraySet`]'s consume-one-reference-yield-one protocol.
    /// `subject` is **borrowed** — the only array operand of a write that is
    /// neither consumed nor stored — so the lowering stages a freshly-built
    /// one as an ordinary owned temporary rather than transferring it. Each
    /// value copied is retained by the runtime, since the entry ends up held
    /// by two arrays; nothing here needs a retain emitted beside it.
    ///
    /// Carries an [`Inst::on_error`] edge for the same reason
    /// [`InstKind::ArrayAppend`] does, because for a renumbered key it *is*
    /// that append.
    ArraySpread {
        /// The array under construction, already lowered.
        array: ValueId,
        /// The array whose entries are copied out, already lowered.
        subject: ValueId,
    },
    /// Removes `key` from `array` if it is present — `unset($a[$k]);`, ADR
    /// 0028 § 3's one surviving `unset` target (the declared-*property* form
    /// is a diagnostic `nvs_types::expr::check_unset_target` already reports,
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
    /// there is none — one step of a `foreach` cursor over `rule:types/arrays`'s
    /// insertion order, defining a [`crate::ty::Ty::Int`].
    ///
    /// A cursor rather than a borrowed iterator because compiled loop-body
    /// code runs between two steps; `nvs_runtime::array`'s `nvs_array_next_slot`
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
    /// index map's own `NvsStr`, and handing compiled code a borrowed pointer
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
    /// same `nvs_types::expr_table::ResolvedCall`, but lowered separately for
    /// one reason: there is no compiled Novis function to name. A `Core` member
    /// is native Rust behind an `rule:errors/propagation`
    /// *helper* entry point, so this carries the linker symbol
    /// `nvs_stdlib::registry` registered rather than a `Class::method` label,
    /// and `nvs-codegen` emits it through the same path
    /// [`InstKind::HelperCall`] takes. It is not a [`Helper`], though: that
    /// enum is a closed set this crate owns, and `Core`'s membership is
    /// `nvs-stdlib`'s to decide.
    ///
    /// **Arguments are borrowed, never consumed** — the helper convention,
    /// which is the opposite of `InstKind::Call`'s. So
    /// `crate::lower::Lowering::lower_call_args` inserts no retain here, and
    /// the caller keeps owning every reference it passed;
    /// `nvs_stdlib`'s own docs own that rule and why `rule:core-api/shape-rules`'s purity
    /// requirement is what makes it safe. A refcounted *result* is a fresh
    /// reference this frame owns, exactly like a `Call`'s.
    ///
    /// There is no receiver *field*, and an instance member needs none: a
    /// `Core`-owned class's member (`$match->text()`,
    /// `nvs_stdlib::registry::CoreTy::Instance`) puts its receiver in
    /// `args[0]`, which is where the ABI would have carried a separate field
    /// anyway. `crate::lower`'s `MethodCall` arm is what puts it there, and it
    /// borrows the receiver exactly as it borrows every other argument — so a
    /// *freshly built* receiver is released by that arm rather than by the
    /// callee. `rule:core-api/shape-rules` R20 keeps the two directions apart: a static member is
    /// unreachable through a value and an instance member is a compile error
    /// through the class name, so no member is ever reached both ways.
    CoreCall {
        /// The linker symbol the implementation is reachable at, from
        /// `nvs_stdlib::registry::CoreMethod::symbol`.
        symbol: &'static str,
        /// The already-lowered arguments, positional.
        args: Vec<ValueId>,
    },
}

/// Which class an [`InstKind::ClassTest`] tests against.
///
/// Two forms rather than two instructions, because the test is the same test:
/// `nvs-codegen` calls the same runtime helper with the same two arguments,
/// and all that differs is where the descriptor's address comes from. A
/// written name is one the unit already laid out, so its address is an
/// `iconst`; `rule:types/type-test`'s value arm `$x is $cls` already has the address in a
/// register, because a `class<T>` value *is* a descriptor.
///
/// The written form keeps its label rather than lowering to a
/// [`TestedClass::Descriptor`] over an [`InstKind::ClassDescConst`], for the
/// reason [`InstKind::New`] keeps its own: the class is a fact of the program
/// that every consumer of this instruction reads by name — the printer,
/// `crate::lower::tests`, and codegen's refusal for a class this unit declares
/// no descriptor for — and none of them can name a [`ValueId`].
#[derive(Debug)]
pub enum TestedClass {
    /// A class or interface written at the site. Every producer but `rule:types/class-reference-sites`'s dynamic one gives this, including the `catch` ladders and
    /// `as`'s own downcast check.
    Named(String),
    /// The [`Ty::ClassDesc`] a `class<T>` operand evaluated to, tested against
    /// by address. Never a null descriptor: `as` is a class reference's only
    /// source and throws rather than yielding one ([`InstKind::ClassDescIn`]).
    Descriptor(ValueId),
}

/// What the checker prepared out of one written literal, as
/// [`InstKind::PreparedConst`] carries it — the closed set of facts a member on
/// `nvs_stdlib::registry::PREPARED_MEMBERS`' roster can be handed.
///
/// This crate's twin of `nvs_types::expr_table::PreparedFact`, and it is a
/// second enum for [`ShapeCodec`]'s reason: what the checker recorded is its
/// own vocabulary, and what a unit emits is this one. [`Self::code`] is the
/// only place the two meet, and the words it answers with live once, beside
/// the runtime that decodes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prepared {
    /// A literal pattern the linear engine expresses —
    /// `rule:core-classes/regex-two-tiers`'s first tier.
    RegexLinear,
    /// A literal pattern only the backtracking engine expresses. The word that
    /// saves a call: the linear engine's parser has already refused this text
    /// once, while checking.
    RegexBacktracking,
    /// A literal CLDR date pattern the checker compiled — the word that saves
    /// the compile itself: a written pattern belongs to a set fixed when the
    /// program was, so `nvs_stdlib::cldr` holds it per core rather than
    /// building it per call.
    CldrPattern,
}

impl Prepared {
    /// This fact as `nvs_stdlib::registry::PREPARED_MEMBERS`' argument 0
    /// carries it.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::RegexLinear => nvs_types::CORE_REGEX_PREPARED_LINEAR,
            Self::RegexBacktracking => nvs_types::CORE_REGEX_PREPARED_BACKTRACKING,
            Self::CldrPattern => nvs_types::CORE_CLDR_PREPARED_PATTERN,
        }
    }
}

/// [`Prepared::code`] over the absence too — the whole of what
/// [`InstKind::PreparedConst`] encodes to, for the emitter that holds the
/// `Option` rather than the fact.
///
/// It lives here rather than in `nvs-codegen` because the zero word is part of
/// the same encoding the variants are, and an emitter spelling it itself would
/// be a second opinion about an ABI.
#[must_use]
pub const fn prepared_code(fact: Option<Prepared>) -> i64 {
    match fact {
        Some(fact) => fact.code(),
        None => nvs_types::CORE_REGEX_PREPARED_NONE,
    }
}

/// What an [`InstKind::ArrayGet`] or an [`InstKind::SlotGet`] answers when the
/// name it is keyed on names nothing.
///
/// Two answers rather than one because PHP has two: a bare `$a["k"]` warns
/// and yields `null` (Novis throws instead — `rule:php-migration/every-divergence-is-deliberate-and-listed` row 11), while
/// `$a["k"] ?? "d"` is defined as *"absent or `null`, without the warning"*
/// and must produce the default. `rule:types/shape-type`'s optional field asks
/// the identical question of an object: `{a?: string}` proves the type and not
/// the presence, so `$p->a` throws and `$p->a ?? "d"` must not — which is why
/// one enum serves both instructions rather than each growing its own bit.
/// The guard is recognized in `nvs_types`, which records it on the access's
/// own `nvs_types::expr_table::ExprInfo::Index` or `ExprInfo::ShapeProperty`
/// entry, because whether a read sits under a `??` is a question about the
/// expression tree that this crate would otherwise have to re-derive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbsentKey {
    /// Throw. The read carries `rule:errors/propagation`'s error edge, and its
    /// result is the element's or the field's own representation.
    Throws,
    /// Answer `null`. The result is [`crate::ty::Ty::Tagged`] — "the element,
    /// or `null`" is a nullable however narrow the declared type is — which is
    /// exactly what the `??` above it tests. An [`InstKind::ArrayGet`] under
    /// this answer is infallible; an [`InstKind::SlotGet`] still is not, its
    /// receiver's tag being a second question this one does not cover.
    Null,
}

/// One member of the closed set of engine-owned runtime conversions
/// [`InstKind::HelperCall`] can invoke — a fixed, non-exhaustive enum for
/// the same reason [`BinOp`]/[`UnOp`] already are one: the set is small,
/// closed, and known entirely to this crate and `nvs-codegen`, never
/// user-extensible, so a string name would only trade compile-time
/// exhaustiveness for nothing. The families: a scalar-to-
/// [`crate::ty::Ty::Str`] conversion — for `.` concatenation
/// (`crate::lower::Lowering::concat_operand`), with `UintToString` reused
/// verbatim by `crate::lower::Lowering::lower_array_key` to render the one
/// array subscript that cannot travel unrendered, and `IntToString` by
/// `crate::lower::Lowering::lower_rendered_array_key` for the one caller
/// that needs a `Ty::Str` key (`rule:types/arrays`) — and a
/// scalar-or-`Ty::Array`-to-[`crate::ty::Ty::Bool`] truthiness test, ADR
/// 0035's table, used by `crate::lower::Lowering::lower_truthy_cond` for an
/// `if`/`while` condition whose static type isn't already `bool` (a
/// `Ty::Object` condition needs none of these: `rule:enums/truthiness` makes it always
/// truthy with nothing to inspect at runtime, so that case lowers straight to
/// a fresh [`InstKind::ConstBool`] instead). A family of its own is
/// [`Helper::ArrayRowForWrite`], here for the reason its own doc
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
    /// A [`Ty::ClassDesc`] to the class's own fully qualified name — the
    /// run-time half of `::class`, and the only way a name leaves a
    /// descriptor.
    ///
    /// The operand needs no conversion to reach this: `nvs_codegen::ty::tag_of`
    /// already spells a [`Ty::ClassDesc`] slot as a `Tag::Null` byte with the
    /// address in the payload, which is exactly what
    /// `nvs_runtime::Value::as_class_desc` reads. So the descriptor rides into
    /// the helper in the representation it already had.
    ///
    /// **What it costs:** one `NvsStr` allocation per evaluation, charged to
    /// the isolate that asked. A descriptor is process-wide and its name never
    /// changes, so one cached string per class would remove the allocation —
    /// and is deliberately *not* done: that payload is reference counted, and
    /// sharing one across isolates is the request-isolation boundary
    /// [AGENTS.md](/AGENTS.md)'s first priority does not trade.
    ClassDescName,
    /// A class constant read off a [`Ty::ClassDesc`] by name — the run-time
    /// half of `static::NAME` (`rule:statements/static-is-a-member-modifier`),
    /// and the one constant read that is not inlined. Two operands: the
    /// descriptor, and the name as a `Ty::Str`. A descriptor's constant table
    /// is flattened own-first, so the answer is the called class's own
    /// override where it has one; the result is typed at the declaring
    /// class's declaration, which `nvs_types::conformance` holds every
    /// redeclaration assignable to.
    ///
    /// **What it costs:** one scan of that table per evaluation, and for a
    /// `string` constant one `NvsStr` allocation, for [`Self::ClassDescName`]'s
    /// reason — the table is process-wide and its strings are not shared
    /// across isolates. `self::NAME` and `Class::NAME` pay nothing: they are
    /// still inlined.
    ClassConstant,
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
    /// `bytes` truthiness: falsy iff empty, and **deliberately not**
    /// [`Self::StrTruthy`]'s row. The one place the two differ is the
    /// one-octet buffer `"0"`, which is falsy for a `string` because that is
    /// PHP's numeric-string rule; a
    /// `rule:types/bytes` `bytes` never
    /// converts to a number, so carrying the quirk over would make a buffer
    /// falsy for a reason that does not apply to it.
    /// `rule:expressions/truthy-table`'s
    /// table states the row; `nvs_runtime::value_truthy`'s `Tag::Bytes` arm is
    /// the same rule reached through a `mixed`.
    BytesTruthy,
    /// `array<T>` truthiness: falsy iff empty, for any `T`.
    ArrayTruthy,
    /// Truthiness of a [`crate::ty::Ty::Tagged`] value —
    /// `rule:expressions/truthy-table`'s last
    /// table row, where a `mixed` or a union "resolved dynamically per this
    /// table, dispatching on the value's runtime type".
    ///
    /// The row the helpers above name statically, chosen at run time
    /// instead: `nvs_runtime::value_truthy` reads the tag and applies the same
    /// rule the matching helper would have, so a `mixed` holding `"0"` and a
    /// `string $s = "0"` answer alike. It is the truthiness twin of
    /// [`Self::Identical`], which is `rule:expressions/mixed-equality`'s equality row for the same
    /// operand shape and reached by the same reasoning: a tag is what a type
    /// no longer answers.
    ///
    /// Like that helper it cannot fail — every tag has a row — so it carries
    /// no error edge.
    ValueTruthy,
    /// One level of a nested array-element write's descent
    /// (`crate::lower::Lowering::lower_reassignment`'s `Index` arm): the row
    /// `args[1]` names in the array `args[0]`, **with a reference of its
    /// own** — a retain of what was there, or a freshly allocated empty
    /// array when the key is absent, which is PHP's auto-vivification.
    ///
    /// It exists because a write asks the absent-key question and gets the
    /// opposite answer to a read's: [`InstKind::ArrayGet`] *throws* there
    /// (`rule:php-migration/every-divergence-is-deliberate-and-listed` row 11), while `$g[9][0] = 1` must build the row PHP
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
    /// `rule:types/arithmetic`, which
    /// **throws** on either overflow kind rather than wrapping or promoting,
    /// so this and its siblings below carry `rule:errors/propagation`'s error edge like any call.
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
    /// `rule:types/arithmetic` permits comparison across the pair precisely where it
    /// forbids arithmetic, because an exact comparison is always computable
    /// even where a common arithmetic type is not.
    DecimalEq,
    /// `a < b` with a `decimal` operand. **`>` is this helper with its
    /// operands swapped** — see [`Self::DecimalEq`] for why each of these
    /// helpers covers a comparison and its mirror.
    DecimalLt,
    /// `a <= b` with a `decimal` operand; `>=` is this one swapped.
    DecimalLtEq,
    /// `a <=> b` with a `decimal` operand — the ordering [`Self::DecimalLt`]
    /// asks one question of, answered whole as an `int`. `rule:types/arithmetic` grants
    /// this row for the reason [`Self::DecimalEq`] states: an exact comparison
    /// is computable across every pairing, including the `decimal`/`float` one
    /// arithmetic refuses. An unordered pair is `1`, as it is for
    /// [`Self::NumericCmp`] and [`BinOp::Cmp`].
    DecimalCmp,
    /// `$n as uint` — `rule:types/conversion`'s `int` ↔ `uint` row. Exact, or **throws**
    /// on a negative value. One of the helpers that can fail rather
    /// than convert, so each is emitted through
    /// `crate::lower::Lowering::emit_fallible` and carries `rule:errors/propagation`'s error
    /// edge like any call.
    IntToUint,
    /// `$n as int` — the same row the other way. Exact, or throws above
    /// `i64::MAX`.
    UintToInt,
    /// `$n as float` — exact, or throws above 2^53, where `f64` stops
    /// representing every integer. `rule:types/conversion` says so outright: `as` "never
    /// rounds, truncates, or substitutes a default."
    IntToFloat,
    /// `$n as float` from a `uint` — [`Self::IntToFloat`]'s row, unsigned.
    UintToFloat,
    /// `$f as int` — integral and in range, or throws. Rounding is
    /// `floor`/`ceil`/`round`, "said out loud" (`rule:types/conversion`), so this
    /// deliberately refuses `1.5` rather than picking one of the three.
    FloatToInt,
    /// `$f as uint` — [`Self::FloatToInt`]'s row, unsigned.
    FloatToUint,
    /// `$s as int` — the *whole* string must be an exact decimal integer
    /// literal, or this throws. `rule:types/conversion`: "No leading-garbage rule, no
    /// `0`" — PHP's `(int)"12abc" === 12` and `(int)"abc" === 0` are both
    /// gone.
    StrToInt,
    /// `$s as uint` — [`Self::StrToInt`]'s row, unsigned.
    StrToUint,
    /// `$s as float` — the whole string must be an exact numeric literal.
    StrToFloat,
    /// `$b as string` — `rule:types/conversion`'s checked row: the buffer is well-formed UTF-8 and becomes the
    /// `string` over the *same* allocation, or this throws. It never replaces,
    /// drops or truncates a bad sequence, so it is fallible and carries
    /// `rule:errors/propagation`'s error edge like every other checked row.
    ///
    /// **The pair's other direction is not here, and that is the point.**
    /// `string as bytes` is total and free — the same `nvs_runtime::NvsStr`
    /// under a second tag — so `crate::lower::Lowering::convert` lowers it to
    /// an [`InstKind::Reinterpret`] and emits no call at all. Only the
    /// checked half needs a helper, because only the checked half runs
    /// anything.
    BytesToString,
    /// `$x as ?int` — `rule:expressions/nullable-conversion`'s non-throwing form of every row above that lands on `int`: the
    /// value `as int` would produce, or `null` where it would throw. Cannot
    /// fail, so unlike the throwing rows it carries no error edge, and
    /// its result is a [`crate::ty::Ty::Tagged`] value rather than a bare
    /// `int`.
    ///
    /// **One tag per target, not one per (source, target) pair.** This helper
    /// dispatches on the operand's runtime tag, which is what makes `rule:expressions/nullable-conversion`'s "a `null` operand yields `null`" and § 3's "from `mixed` every
    /// target has a checked path" the same code as `"42" as ?int` rather than
    /// a lowering branch per source — the operand is already a tagged `Value` by
    /// the time any helper sees it (`nvs_codegen`'s `store_value`).
    ToIntOrNull,
    /// `$x as ?uint` — [`Self::ToIntOrNull`]'s row set, unsigned.
    ToUintOrNull,
    /// `$x as ?float` — [`Self::ToIntOrNull`]'s row set, landing on `float`.
    ToFloatOrNull,
    /// `$x as ?string` — [`Self::TaggedAsString`]'s rows in `rule:expressions/nullable-conversion`'s
    /// non-throwing form, and the one `?` twin whose result is refcounted.
    ///
    /// `rule:expressions/nullable-conversion` makes the two spellings differ only in what they do with a
    /// miss, so this shares that helper's implementation rather than carrying a
    /// second copy of `rule:types/conversion`'s table: an operand that converts
    /// converts the same, `bytes` included, and one whose *conversion* fails
    /// answers `null` instead of throwing.
    ///
    /// **A `toString()` body that throws still throws.** `rule:expressions/nullable-conversion`'s `null`
    /// stands for "this conversion had no answer", not for "swallow whatever
    /// the operand did on the way": the exception the body recorded is the
    /// program's own and reaches the request unchanged. `nvs_runtime`'s
    /// `stringify_or_null` is where that line is drawn.
    ToStringOrNull,
    /// `$x as ?bytes` — [`Self::TaggedToBytes`]'s rows in `rule:expressions/nullable-conversion`'s
    /// non-throwing form, sharing that helper's one implementation of them for
    /// the reason [`Self::ToStringOrNull`] shares its own.
    ///
    /// The one `?` twin whose *checked* spelling is the only one that ever runs
    /// anything: `string as bytes` is total and free wherever the static type
    /// says so, so this helper exists exclusively for the tagged operand —
    /// which is also why `rule:expressions/nullable-conversion-availability`'s "yields `null` exactly where `as T`
    /// would throw" leaves the statically typed `$s as ?bytes` refused
    /// (`E0709`) rather than lowered here.
    ToBytesOrNull,
    /// `$x as decimal` — `rule:types/conversion`'s `→ decimal` rows, chosen by the
    /// operand's runtime tag the way [`Self::ToIntOrNull`] chooses, so one
    /// helper covers `int`, `uint`, `float`, `string` and `mixed` alike.
    /// Throws where the row fails or does not exist, so it carries an error
    /// edge.
    ToDecimal,
    /// `$x as ?decimal` — [`Self::ToDecimal`]'s rows in `rule:expressions/nullable-conversion`'s
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
    /// `"19.90"`, which is `rule:types/conversion`'s row.
    DecimalToString,
    /// A [`crate::ty::Ty::Tagged`] operand to `string` — the scalar
    /// conversions above plus `null`, chosen by the operand's **runtime** tag
    /// rather than by a static type, since a `mixed`, a `?T` or any other
    /// union has none to choose by.
    ///
    /// Reached from the two spellings that render a value without converting
    /// it: `.` concatenation and `echo`
    /// (`crate::lower::Lowering::concat_operand`). That is the same "one tag
    /// per target" arrangement [`Self::ToIntOrNull`] describes, so `mixed` needs
    /// no lowering branch of its own there. A written `expr as string` is
    /// [`Self::TaggedAsString`] instead, because a `bytes` operand converts
    /// there and is refused here.
    ///
    /// **An object operand is a `toString` call, not a tag row.** Both
    /// spellings above also land here for an object whose *static* type named
    /// no class to resolve against — an erased `object`
    /// (`rule:types/erased-member-access`), a
    /// union, a `Core`-owned class — and `nvs_runtime::stringify` answers it
    /// by asking the receiver's runtime class for `rule:classes/stringable`'s `toString`.
    /// The static path is cheaper: where `nvs_types` resolved
    /// one, an ordinary [`InstKind::CallVirtual`] is emitted and no helper is
    /// reached at all.
    ///
    /// **The one string conversion that can fail**, so unlike the static
    /// ones it is emitted through `crate::lower::Lowering::emit_fallible` and
    /// carries `rule:errors/propagation`'s error edge: an array, a closure, a resource and an
    /// object whose class declares no `toString` have no row, and
    /// `nvs_runtime::value_to_string` owns what each throws and why.
    TaggedToString,
    /// `expr as string` over a [`crate::ty::Ty::Tagged`] operand
    /// (`crate::lower::Lowering::convert`): [`Self::TaggedToString`]'s rows,
    /// plus [`Self::BytesToString`]'s checked UTF-8 row for a `bytes` tag, so
    /// `$maybe as string` over a `?bytes` answers what `$b as string` over a
    /// `bytes` does. [`Self::ToStringOrNull`] is its non-throwing twin over the
    /// same implementation. Fallible, with `rule:errors/propagation`'s error
    /// edge.
    TaggedAsString,
    /// A [`crate::ty::Ty::Tagged`] operand to `int` — `rule:types/conversion`'s `→ int`
    /// rows chosen by the operand's **runtime** tag, which is the only thing
    /// that names a row when the static type is a `mixed`, a `?T` or any other
    /// union.
    ///
    /// The throwing twin of [`Self::ToIntOrNull`], over the same row set in
    /// `nvs_runtime` and never a second copy of it: § 2's `as T` throws where
    /// `rule:expressions/nullable-conversion`'s `as ?T` answers `null`, so the pair differs only in what it
    /// does with a miss. Fallible, so it is emitted through
    /// `crate::lower::Lowering::emit_fallible` and carries `rule:errors/propagation`'s error
    /// edge — a tag with no row at all (an array, an object, a `bool`) throws
    /// here, which is `rule:types/unions-and-mixed`'s answer for `mixed` and a compile error for
    /// anything the checker can name.
    TaggedToInt,
    /// [`Self::TaggedToInt`]'s row set, unsigned.
    TaggedToUint,
    /// [`Self::TaggedToInt`]'s row set, landing on `float`.
    TaggedToFloat,
    /// A [`crate::ty::Ty::Tagged`] value stored at a union that names `float`
    /// and not the value's integer type: an `int` or `uint` tag becomes a
    /// `float` tag, exactly as [`Self::IntToFloat`] converts it, and throws
    /// above 2^53. Every other tag comes back unchanged.
    ///
    /// The result is the operand's own reference, as with
    /// [`crate::ir::InstKind::Tag`]: nothing is retained or released around
    /// it. Emitted by `crate::lower::Lowering::lower_expr` where the checker
    /// marked the value (`nvs_types::expr_table::ExprTypeTable::widens_to_float`).
    TaggedWidenToFloat,
    /// A [`crate::ty::Ty::Tagged`] operand to `bytes` — `rule:types/conversion`'s
    /// `string as bytes` row chosen by the operand's **runtime** tag, plus the
    /// identical-type row a value that is already a `bytes` takes.
    ///
    /// **This is the whole of that row that runs a call.** The statically typed
    /// spelling is total and free — one [`InstKind::Reinterpret`] over the same
    /// allocation, see [`Self::BytesToString`] — so, unlike every other
    /// conversion helper here, this one has no static sibling: a tagged operand
    /// is the only shape whose row nothing before run time can name. Fallible
    /// for the same reason [`Self::TaggedToInt`] is: a tag with no row at all
    /// throws, which is `rule:types/unions-and-mixed`'s answer for `mixed` and a compile error
    /// (`E0708`) for anything the checker can name.
    TaggedToBytes,
    /// `rule:types/conversion`'s
    /// `array<T> as array<U>` row: the one row of that table whose check is
    /// per *element* rather than per value, so the only one that is a walk.
    ///
    /// Argument 0 is the operand — a [`crate::ty::Ty::Array`], or a
    /// [`crate::ty::Ty::Tagged`] whose tag is tested first, which is `rule:types/unions-and-mixed`'s way out of `mixed` for an array. Argument 1 is the element
    /// description, `crate::lower::array_element_tags`' word: one tag nibble
    /// per level of `U`. Both are what a helper *can* carry — arguments are
    /// stored as `nvs_runtime::Value`s — and the reason an element type naming
    /// a class is refused where it is written (`E0711`) rather than lowered
    /// to this.
    ///
    /// **Nothing is copied.** `rule:types/arrays` makes `array<T>` invariant so that
    /// the O(n) restamp is visible, and the restamp is this walk; the *buffer*
    /// then stays shared, because an Novis array is copy-on-write and whichever
    /// side writes first separates itself (`nvs_runtime::array`'s
    /// `make_unique`). So the result is the same allocation under one more
    /// reference, and the row costs one tag test per element and no bytes at
    /// all.
    ///
    /// Fallible for the reason every checked row is: an element no tag row
    /// names throws, and so does an operand that is not an array.
    ToArrayOf,
    /// `$x as ?array<U>` —
    /// `rule:expressions/nullable-conversion`'s non-throwing form of [`Self::ToArrayOf`], over that helper's one
    /// implementation of the walk rather than a second copy of it. Answers
    /// `null` exactly where the checked spelling throws, and cannot fault at
    /// all — the walk runs no user code. Like [`Self::ToBytesOrNull`] it
    /// still carries a landing block, because it still returns a status:
    /// [`Inst::on_error`] is that rule's home.
    ToArrayOfOrNull,
    /// Writes one already-[`crate::ty::Ty::Str`] operand's cooked bytes to
    /// the process's standard output, unescaped — `echo`'s one and only
    /// effect under `nvs run`. Defines no value: a [`Helper`] invoked for an effect
    /// rather than a conversion, so its [`InstKind::HelperCall`] is emitted
    /// with `result: None` and every other variant's "the result is a fresh
    /// `Ty::Str` nothing else owns" release policy does not apply to it.
    /// `rule:core-classes/html-auto-escape`'s auto-escaping sink is the *HTTP response* write, not
    /// this one — whether `echo` under `nvs serve` becomes that sink is an
    /// M7 decision this deliberately does not pre-empt.
    EchoStr,
    /// [`Self::EchoStr`]'s sink over an operand that has **not** been converted
    /// to [`crate::ty::Ty::Str`] first — `echo`'s row for a
    /// [`crate::ty::Ty::Object`] or [`crate::ty::Ty::Tagged`] operand, which are
    /// the static types `rule:security/capture-answers-the-carrier`'s sink carrier can arrive under.
    ///
    /// `rule:tooling/terminal-output-is-a-sink`
    /// puts exactly one raw path in the language and § 2 makes it a *type*,
    /// `Core\Cli\Text`. A type can only be recognised while the operand still
    /// has one, so the conversion [`crate::lower::Lowering::concat_operand`]
    /// would perform has to happen *inside* the sink rather than in front of
    /// it: `nvs_runtime::nvs_echo_value` asks the class first and then renders
    /// through the same `stringify` this crate would have called. That is one
    /// helper call rather than two for these operands, and the substitution
    /// decision is taken where the class is still known.
    ///
    /// Defines no value and is emitted with `result: None`, exactly as
    /// [`Self::EchoStr`] is; the operand stays the caller's, and a non-aliasing
    /// one is released by the same `owned_temporaries` sweep.
    EchoValue,
    /// One piece of a markup literal written at `echo` — a segment's bytes or
    /// a hole's escaped ones, already [`crate::ty::Ty::Str`].
    ///
    /// The pieces are the bytes of a `Core\Html\Markup` that is never built
    /// (`rule:core-classes/html-literal`), so the sink treats them as it
    /// treats that carrier: the HTML sink writes them unchanged, and every
    /// other sink substitutes them as [`Self::EchoValue`] substitutes a
    /// `Markup`. [`Self::EchoStr`] would escape them a second time under the
    /// HTML sink. Defines no value, exactly as [`Self::EchoStr`] does.
    EchoMarkup,
    /// `exit`/`exit(...)`: record the process status its one
    /// [`crate::ty::Ty::Int`] argument names, then end the request.
    ///
    /// **The one helper whose success is a non-`OK` status.** It returns
    /// `nvs_runtime::EXITED`, so the `rule:errors/propagation` status check after it takes this
    /// instruction's error edge, the frame's live locals are released in that
    /// landing block, and every caller's own check propagates it onward. No
    /// `catch` sees it — [`Terminator::Catch`] admits only `THROWN` — and **no
    /// `finally` runs**, because every copy of a `finally` body lives behind
    /// that comparison. Both are PHP's own behaviour for `exit`;
    /// `docs/adr/README.md` § *Decisions taken at project start* owns the
    /// decision and what it costs.
    ///
    /// Defines no value and never returns normally, so it is pushed with
    /// `result: None` the way [`Self::EchoStr`] is, and always carries an
    /// error edge.
    Exit,
    /// The throw at the end of
    /// `rule:types/literal-types`'s membership test: the operand reached none of the literals its
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
    /// `rule:errors/propagation`'s error edge. The membership test itself costs no helper call
    /// at all — see `crate::lower::Lowering::lower_literal_membership` for why
    /// a comparison chain and not one call over an encoded set.
    ///
    /// It **owns** that second argument, uniquely among helpers: a call that
    /// never returns leaves no reachable instruction to release the fresh
    /// [`InstKind::ConstStr`] at, so `nvs_runtime`'s own helper drops it.
    /// The operand keeps the ordinary convention and is the caller's.
    LiteralMismatch,
    /// One argument: the [`crate::ty::Ty::Tagged`] operand of a `clone` whose
    /// tag turned out not to be an object, so `rule:classes/clone-is-shallow`'s
    /// "a new instance of `$x`'s class" names no class. It is the miss arm of
    /// the one [`InstKind::TagIs`] `crate::lower::Lowering::guard_cloneable`
    /// asks, defines no value and **never returns normally**, so it is pushed
    /// with `result: None` the way [`Self::LiteralMismatch`] is and always
    /// carries `rule:errors/propagation`'s error edge.
    ///
    /// The message is PHP's own, and it names the type it was given — which is
    /// why it is rendered in `nvs_runtime` rather than at lowering time the way
    /// [`Self::LiteralMismatch`]'s accepted set is: the operand's tag is what
    /// answers it, and that is a run-time fact.
    ///
    /// It **owns** its argument, the second helper here to do so and for
    /// [`Self::LiteralMismatch`]'s reason: a call that never returns leaves no
    /// reachable instruction to release a reference at, so lowering retains a
    /// borrowed operand in front of the call and `nvs_runtime` releases what it
    /// was handed.
    CloneOperandNotAnObject,
    /// `a == b` where at least one operand is a [`crate::ty::Ty::Tagged`] —
    /// `rule:expressions/mixed-equality`'s `mixed`-or-union case, the one pairing whose § 3 row is a runtime
    /// tag rather than a static type. **`!=` is this helper under
    /// [`UnOp::Not`]**, the arrangement [`Self::DecimalEq`] already uses.
    ///
    /// Every other row of that table is reached without this helper, because
    /// the operands' types already named it: a scalar pair is one
    /// [`BinOp::Eq`] machine comparison, and `nvs-codegen` turns a `string`,
    /// `array` or `object` pair into a direct two-pointer call or an inline
    /// pointer compare rather than tagging both sides into stack slots. So
    /// this is the *only* row that pays `rule:errors/propagation`'s calling convention, and it
    /// is the row that has nothing cheaper to pay.
    ///
    /// It cannot fail — § 5 makes a mismatched pair `false` rather than a
    /// throw — so unlike a conversion helper it carries no error edge.
    Identical,
    /// `a == b` over two operands whose *representations* differ but whose
    /// types are one domain —
    /// `rule:expressions/disjoint-comparison-refused`'s numeric row, which makes `int`, `uint` and `float` mutually
    /// comparable, and § 3's "mathematically equal across the whole domain".
    /// **`!=` is this helper under [`UnOp::Not`]**, the arrangement
    /// [`Self::DecimalEq`] already uses.
    ///
    /// A call rather than a widening conversion because **no widening between
    /// these three is exact**, which is the whole reason this row cannot be
    /// settled the way a same-representation pair is: `int` → `float` loses
    /// every integer past 2^53, `float` → `int` has no answer for a fractional
    /// or out-of-range operand, and `int` ↔ `uint` share a bit pattern at `-1`
    /// and `u64::MAX`. `nvs_runtime::numeric_identical` compares over `i128`
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
    /// `rule:types/arithmetic` closes
    /// its own table with the rule this exists to keep: "a comparison has an
    /// exact answer in the mathematical integers and can be lowered as one".
    /// It says that about `int` against `uint`, and the `int`/`uint` against
    /// `float` pairing is the same question — so widening the integer side
    /// into a `float` first, which is what the *arithmetic* rows of that table
    /// do, is wrong here twice over: it would round away every integer past
    /// 2^53, and because § 2 makes that widening **throw** rather than round,
    /// what a program would actually observe is `9007199254740993 < 1.5`
    /// raising `ArithmeticError` where PHP answers `false`.
    /// `nvs_runtime::numeric_ordering` compares over `i128` and over the
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
    /// `a < b` where at least one operand is a [`crate::ty::Ty::Tagged`] —
    /// `rule:types/arithmetic`'s
    /// ordering row chosen from the operands' runtime **tags**, because a
    /// `mixed` or a union no longer names one.
    ///
    /// It is the ordering twin of [`Self::Identical`], reached by the same
    /// reasoning and from the same place in `crate::lower::Lowering::lower_binary`:
    /// where the static types answer the row, `nvs-codegen` emits the machine
    /// comparison; where they do not, the tag answers it and the operand pair
    /// is settled here rather than in a `BinOp` over a representation neither
    /// side has. `>` is this helper with its operands swapped, the arrangement
    /// [`Self::NumericLt`] and [`Self::DecimalLt`] already use.
    ///
    /// **Unlike every other comparison helper this one carries
    /// [`Inst::on_error`]**, and that is the whole design decision in it. § 4's
    /// ordering table is a *closed* list, so a pair it names no row for — two
    /// strings, an `array<T>`, `null`, an enum case, a `bytes`, a `callable` —
    /// has no ordering at all. Where the static types show it,
    /// `nvs_types::expr::operators::reject_unordered_operand` refuses it where
    /// it is written (`E0715`); where they do not, the refusal is exactly as
    /// real and can only be made when the tags arrive, so it becomes a
    /// catchable throw carrying that diagnostic's own wording. That is `rule:types/erased-member-access`'s deferral — the checked answer of an erased operand is a throw, not
    /// a silent value — applied to the operator table instead of to a member
    /// access.
    ///
    /// Two objects behind two `mixed`s throw here as well, and deliberately:
    /// `rule:classes/comparable` orders them through a `Comparable::compareTo` **call**, which
    /// `crate::lower::Lowering::lower_object_comparison` emits from the class
    /// the site named. A helper that has only the tag names none.
    ValueLt,
    /// `a <= b` over a tagged pair — [`Self::ValueLt`]'s row inclusive, and
    /// `>=` is this one swapped. Carries the same error edge, for the same
    /// reason.
    ValueLtEq,
    /// `a <=> b` over a tagged pair — [`Self::ValueLt`]'s row read whole
    /// rather than asked one question, answered as an `int`. A `NaN` operand
    /// becomes `1`, the convention [`Self::NumericCmp`] and [`BinOp::Cmp`]
    /// already follow; a pair with no row throws, exactly as it does for the
    /// four ordering operators.
    ValueCmp,
    /// `a + b` where at least one operand is a [`crate::ty::Ty::Tagged`] —
    /// `rule:types/arithmetic`'s
    /// **arithmetic** rows chosen from the operands' runtime tags, exactly as
    /// [`Self::ValueLt`] chooses its ordering ones.
    ///
    /// The family is one table in the runtime
    /// (`nvs_runtime::helpers::value_arith`) asked a different row each, and
    /// that function's doc comment is the table's one home. What follows
    /// belongs *here*, because these are lowering decisions:
    ///
    /// * **The result is [`crate::ty::Ty::Tagged`] whatever the operands
    ///   hold**, since which row a pair of tags takes is only known when they
    ///   arrive — `$m + 1` is an `int` or a `float` or a throw. That is the
    ///   same reason integer `/` already carries that representation, and
    ///   `crate::lower::Lowering::coerce` absorbs it into a declared type by
    ///   the same rows.
    /// * **Every one carries [`Inst::on_error`]**, and for more reasons than
    ///   the ordering family has: § 4's table is closed, so a pair it names
    ///   no row for throws; *and* the rows it does name throw on overflow,
    ///   which is the divergence from PHP the ADR is least willing to trade;
    ///   *and* `int ⊕ uint` has no representable common type,
    ///   so `E0407`'s refusal arrives here as a throw when only the tags know.
    /// * **A `decimal` operand is a row of this table, not of
    ///   [`Self::DecimalAdd`]'s.** Behind a `mixed` there is no static
    ///   `decimal` to route on, so `rule:types/arithmetic`'s arithmetic rows are
    ///   answered from the tag alongside the integer ones — over the very same
    ///   `Decimal` methods, so the two ends of the row cannot disagree.
    ValueAdd,
    /// `a - b` over a tagged pair — see [`Self::ValueAdd`].
    ValueSub,
    /// `a * b` over a tagged pair — see [`Self::ValueAdd`].
    ValueMul,
    /// `a / b` over a tagged pair — see [`Self::ValueAdd`]. `rule:types/arithmetic` types
    /// integer division `int|float`, so this is the one row whose answer's tag
    /// is still a runtime question once the operands' tags are known.
    ValueDiv,
    /// `a % b` over a tagged pair — see [`Self::ValueAdd`].
    ValueMod,
    /// `a ** b` over a tagged pair — see [`Self::ValueAdd`].
    ValuePow,
    /// `a & b` over a tagged pair — see [`Self::ValueAdd`]. `rule:types/arithmetic`'s
    /// `& | ^ << >>` row is `int` and `uint` alone, the same list
    /// `nvs_types::expr::operators::reject_bitwise_operand` refuses every
    /// other operand against (`E0706`), so the bitwise members of this
    /// family have a narrower table than the arithmetic ones.
    ValueBitAnd,
    /// `a | b` over a tagged pair — see [`Self::ValueBitAnd`].
    ValueBitOr,
    /// `a ^ b` over a tagged pair — see [`Self::ValueBitAnd`].
    ValueBitXor,
    /// `a << b` over a tagged pair — see [`Self::ValueBitAnd`].
    ValueShl,
    /// `a >> b` over a tagged pair — see [`Self::ValueBitAnd`]. The one
    /// operator that reads its left operand's *signedness* rather than only its
    /// width, which behind a `mixed` is the tag rather than the declaration.
    ValueShr,
    /// `-a` over a tagged operand — [`Self::ValueAdd`]'s table asked with one
    /// operand instead of two, and the shape of it a *unary* operator reaches.
    /// The rows are `rule:types/arithmetic`'s own: `int` and `uint` throw rather than
    /// wrap, `-i64::MIN` having no `int` and every non-zero `uint` no negation
    /// at all, and `float` and `decimal` cannot fail. Everything else is the
    /// closed table's refusal, arriving as a catchable throw carrying
    /// `nvs_types::expr::operators::reject_unary_arith_operand`'s own reading
    /// (`E0705`) because that check cannot make it from a `mixed`.
    ///
    /// The result is [`crate::ty::Ty::Tagged`] for [`Self::ValueAdd`]'s reason:
    /// `-$m` is an `int`, a `float`, a `decimal` or a throw, and which one is
    /// exactly what the tag arrives to say.
    ///
    /// Unary `+` has no member here because it has no row anywhere: it is the
    /// identity over every numeric type, so
    /// `crate::lower::Lowering::lower_unary` returns the operand itself and
    /// emits no instruction, a tagged operand included.
    ValueNeg,
    /// `~a` over a tagged operand — see [`Self::ValueNeg`], over
    /// [`Self::ValueBitAnd`]'s narrower table: § 4's `& | ^ ~ << >>` row is
    /// `int` and `uint` alone, so a `float` or a `decimal` operand is a number
    /// with no bit pattern to complement and takes the refusal rather than a
    /// row.
    ValueBitNot,
    /// `$m[$k]` where the base's static type named no element type at all —
    /// [`InstKind::ArrayGet`] asked of a [`crate::ty::Ty::Tagged`] base, whose
    /// tag is what answers *whether there is an array here* before the key is
    /// looked up.
    ///
    /// This is
    /// `rule:types/erased-member-access`'s
    /// deferral applied to a subscript rather than to a member access: a
    /// `mixed` is `rule:types/conversion`'s one unchecked position, so the read is the tag's question and not
    /// the site's, and every base whose *declared* type already answers it —
    /// a scalar, an untested `?array<T>`, a union naming no array — is
    /// refused where it is written instead (`E0482`).
    ///
    /// It is a `Helper` rather than a widening of `InstKind::ArrayGet` because
    /// the two differ in exactly one row and it is the one that must not be
    /// shared: a non-array base is an *internal inconsistency* for the
    /// statically typed read, whose base is an `array<T>` by declaration, and
    /// a **catchable throw** here, carrying `E0482`'s own wording. The key and
    /// the absent-key answer are the same in both — `nvs_runtime::helpers`'
    /// `value_index` is the one implementation, and its doc comment is those
    /// rows' home.
    ///
    /// The result is [`crate::ty::Ty::Tagged`] for [`Self::ValueAdd`]'s
    /// reason: the element's type is whatever the array turns out to hold.
    /// The base and the key are both **borrowed**, exactly as `ArrayGet`
    /// borrows them, and the error edge is [`Inst::on_error`] as for every
    /// read that can fail on an absent key.
    ValueIndexGet,
    /// `$m[$k]` under a `??` — [`Self::ValueIndexGet`] with
    /// [`AbsentKey::Null`]'s answer, and one row wider: a base whose tag is
    /// not an array answers `null` too rather than throwing, which is what
    /// PHP's `??` does for `$m["k"]` over any `$m` at all. Infallible, so it
    /// carries no error edge — the same split
    /// [`AbsentKey`] draws for the statically typed pair.
    ValueIndexOptionalGet,
    /// `$a[$k]` where the *key* carries no static type — a subscript, an
    /// array literal's explicit `key =>` and an `unset` alike, over a
    /// [`crate::ty::Ty::Tagged`] key.
    ///
    /// `rule:types/arrays` keys every array by a `string` and normalizes an
    /// `int`/`uint` to its own decimal, and `nvs_types`'
    /// `check_array_key_type` refuses a `float`, `bool`, `null` or enum key
    /// wherever the declared type shows it. A `mixed` shows nothing, which is
    /// `rule:types/unions-and-mixed`'s one unchecked position reaching the key
    /// slot: the tag answers instead, and this helper is that answer — the
    /// same normalization, plus a **catchable throw** naming the tag for
    /// every kind `E0434` would have refused outright.
    ///
    /// The result is always [`crate::ty::Ty::Str`] and always a fresh buffer
    /// with exactly one owner, as [`Self::UintToString`]'s is, so
    /// `Lowering::lower_array_key`'s callers stage and release a key the same
    /// way whichever arm produced it.
    ValueToArrayKey,
    /// `a == b` over two operands at least one of which the checker typed
    /// `secret` —
    /// `rule:security/secret-comparison-is-constant-time`
    /// . The comparison is **constant-time in the contents**: it reads
    /// every byte of two equal-length operands whatever they hold, so an
    /// attacker holding one side cannot recover the other a byte at a time by
    /// timing the answer. Lengths are not hidden — a mismatch answers `false`
    /// at once, which is what `nvs_stdlib`'s `Core\Hash::equals` does for the
    /// same reason.
    ///
    /// **`!=` is this helper under [`UnOp::Not`]**, the arrangement
    /// [`Self::NumericEq`] and [`Self::DecimalEq`] already use, and it is
    /// total, so it carries no error edge.
    ///
    /// The qualifier is invisible at this level by design: a `secret string`
    /// erases to [`crate::ty::Ty::Str`] and a `secret bytes` to
    /// [`crate::ty::Ty::Bytes`], because `rule:security/secret-qualifier` spends no representation
    /// on the bit. So the *lowering* cannot re-derive the choice of helper
    /// from its operand types, and does not try: the checker records
    /// `nvs_types::expr_table::ExprInfo::SecretEquality` at the comparison and
    /// `lower_binary` reads it back.
    ///
    /// **One operand may be a [`crate::ty::Ty::Tagged`]**, which is a
    /// credential compared against a `mixed` — a decoded request field, a
    /// header, a cache read. The helper untags it: a `string` or `bytes`
    /// payload of the other side's own tag is compared in constant time, and
    /// every other tag answers `false`, which is what
    /// `nvs_runtime::value_identical` answers for the same pair. A pair of
    /// tags never arrives, `rule:security/secret-qualifier` putting the
    /// qualifier on `string` and `bytes` alone.
    ///
    /// Costed in `rule:security/secret-comparison-is-constant-time`: a few nanoseconds more per comparison
    /// than the short-circuiting row — priority 1 bought with priority 3,
    /// which is the ordering AGENTS.md states.
    SecretEq,
    /// `$fn(...)` —
    /// `rule:types/anonymous-function`'s
    /// closure, called through the variable holding it. `args[0]` is the
    /// closure object and `args[1..]` its arguments in written order.
    ///
    /// **The one variadic [`Helper`]**, and the reason the row exists at all
    /// rather than this being an [`InstKind::Call`]: there is no resolved
    /// target to name. `callable` carries no parameter list
    /// (`rule:types/anonymous-function`), so the checker types the call `mixed` and cannot say which
    /// function a variable holds; what answers both questions is the closure
    /// object itself, whose class declares the one `invoke`
    /// `nvs_runtime::call_closure` reaches through. That helper is the same
    /// one every `Core` member taking a `callable` already calls, so a
    /// closure invoked from Novis and one invoked from a native member take the
    /// identical path.
    ///
    /// Being variadic, it is the one helper whose argument count is not baked
    /// into `nvs_runtime`'s own declaration: `nvs-codegen` passes the count
    /// beside the argument slot, which is `Signatures::helper_variadic` there
    /// and one extra parameter on `nvs_call_closure` here.
    ///
    /// Arguments are **borrowed**, the treatment every helper's are given:
    /// `call_closure` retains the receiver and each argument it actually
    /// passes, and the callee's own exit sweep releases those, so the caller
    /// keeps owning exactly what it lowered. The result is a fresh
    /// [`crate::ty::Ty::Tagged`] nothing else owns — the callee's return
    /// value, transferred — and it is typed `Tagged` because `mixed` is the
    /// only answer the checker has for a call whose target it cannot name.
    ///
    /// Fallible, so it is emitted through
    /// `crate::lower::Lowering::emit_fallible` and carries `rule:errors/propagation`'s error
    /// edge: the closure's own throw or fault travels back as
    /// `Fault::Pending`, unchanged.
    CallClosure,
    /// `$fn(...)` where `$fn`'s type carries `rule:types/callable-signature`'s
    /// written signature — [`CallClosure`](Self::CallClosure) with the
    /// per-argument tag check left out.
    ///
    /// Everything about the emitted call is that row's: the closure at
    /// `args[0]`, the arguments after it in written order, the count beside the
    /// slot, borrowed arguments, a fresh [`crate::ty::Ty::Tagged`] result and
    /// `rule:errors/propagation`'s error edge. What differs is what the runtime
    /// does with them — `nvs_runtime::closure`'s `check_param_tags` is not run,
    /// because `nvs_types` checked every argument against a declared parameter
    /// type where the call was written, and `crate::lower` coerced each one
    /// into that parameter's own representation, which is the conversion the
    /// check would otherwise have performed.
    ///
    /// The closure object still carries both metadata slots. A literal does not
    /// know which kind of site will call it, and bare `callable` — the top of
    /// the lattice, and every callback a `Core` member reaches — still needs
    /// them.
    CallClosureProven,
    /// `$fn(...$args)` — [`CallClosure`](Self::CallClosure) for a call site
    /// that wrote a `...` argument, where how many arguments there are is the
    /// spread subject's own run-time length.
    ///
    /// `args[0]` is the closure and `args[1]` one array holding every argument
    /// in call order — the array `crate::lower::Lowering::lower_args_as_array`
    /// already builds for a variadic parameter's tail, each `...` flattened
    /// into it by `nvs_runtime::nvs_array_spread`. It is a second row rather
    /// than a wider [`CallClosure`](Self::CallClosure) because that one's
    /// argument count is a literal in the emitted call — `nvs-codegen` writes
    /// it beside the argument slot — and a `...` is precisely the shape with no
    /// such count, so this one is an ordinary fixed-arity helper taking two
    /// values.
    ///
    /// Ownership, the result and the error edge are all
    /// [`CallClosure`](Self::CallClosure)'s: the array is borrowed like every
    /// other helper argument, the result is a fresh
    /// [`crate::ty::Ty::Tagged`] the callee transferred, and the callee's own
    /// throw travels back as `Fault::Pending`.
    CallClosureArray,
    /// `$m->method(...)` on a **`mixed`** receiver — `rule:types/erased-member-access`'s deferral
    /// applied to a call, dispatched on the value the way
    /// [`CallClosure`](Self::CallClosure) dispatches on a closure object.
    ///
    /// `args[0]` is the receiver, still tagged — nothing proved it holds an
    /// object at all, so a tag that is not one is a catchable throw down
    /// there rather than a refusal up here. `args[1]` is the member name as an
    /// immortal `string` constant: one address in the unit's data section, no
    /// allocation per call, exactly what [`InstKind::SlotGet`]'s own name is.
    /// `args[2]` is **one array** holding every argument in written order,
    /// built by `crate::lower::Lowering::lower_args_as_array`.
    ///
    /// It is a helper rather than an [`InstKind`] for
    /// [`CallClosureArray`](Self::CallClosureArray)'s reason and its
    /// arguments are packed for the same one: how many there are is a
    /// **run-time** fact — a `...` argument's count is its subject's own
    /// length — and the callee's arity is not known here either, that being
    /// what the receiver's descriptor answers. `docs/adr/README.md`
    /// § *Decisions taken at project start* owns the convention: the method
    /// row on the receiver's own `ClassDesc` carries the callee's arity and
    /// parameter tags, and `nvs_runtime::closure`'s `check_param_tags` is the
    /// one implementation this path and `callable`'s share.
    ///
    /// Ownership is every helper's: the receiver, the name and the array are
    /// all **borrowed**, the runtime retaining each value it actually passes
    /// on so that the compiled callee's own exit sweep has a reference to
    /// release. The result is a fresh [`crate::ty::Ty::Tagged`] the callee
    /// transferred — `mixed` being the only answer the checker has for a call
    /// whose target it cannot name.
    ///
    /// Fallible, and it is the row with the most program-reachable throws:
    /// a receiver that is no object, a class with no such member, a member
    /// that is not `public`, one the row cannot describe, an argument whose
    /// tag is not the one the parameter requires, and too few arguments — all
    /// catchable, all carrying `rule:errors/propagation`'s error edge, plus the callee's own
    /// throw travelling back as `Fault::Pending`.
    CallErasedMethod,
    /// `$fn->bindTo($obj)` and `$fn->bind($obj)` —
    /// `rule:types/callable-is-the-only-function-type`'s rebind, and the first half of
    /// `$fn->call($obj, ...)`, whose second half is
    /// [`CallClosure`](Self::CallClosure) on what this returns.
    ///
    /// `args[0]` is the closure and `args[1]` the new `$this`, both borrowed.
    /// The result is a fresh closure reference: the same object for a closure
    /// that does not use `$this`, a copy holding the new `$this` for one that
    /// does. `nvs_runtime::closure::bind_closure` owns the class test, and its
    /// `LogicError` for an object the body was not checked against is this
    /// row's error edge.
    BindClosure,
}

/// A binary arithmetic or comparison operator, already resolved to a single
/// representation (no `mixed`/union dispatch — see the crate docs' "no
/// runtime-helper calls" known gap).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum BinOp {
    /// `+`. Over an `int` or a `uint` this and the arithmetic operators below
    /// carry [`Inst::on_error`] — see that field for which of them throws on
    /// what.
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`. The one operator whose result representation is not its operands':
    /// `rule:types/arithmetic` types the integer row as `int|float`, so it produces a
    /// [`crate::ty::Ty::Tagged`] and picks between the two at run time.
    ///
    /// It is also the one arithmetic operator that carries [`Inst::on_error`]
    /// over a **float**: § 4 refuses the zero divisor before the operand types
    /// are consulted, so `1.0 / 0` throws exactly where `1 / 0` does and
    /// `Core\Math::fdiv` is the member that answers IEEE's infinity instead.
    Div,
    /// `%`
    Mod,
    /// `**`. `rule:types/arithmetic` lists this beside `+`, `-` and `*`, so the integer
    /// row **throws** rather than wrapping and it carries [`Inst::on_error`]
    /// too — including for a *negative* exponent, which that row's "no
    /// promotion to `float`" leaves with no `int` to answer except where the
    /// base is `1` or `-1`. Neither representation is one instruction:
    /// `nvs-codegen` emits a square-and-multiply loop for the integer row and
    /// calls `nvs_runtime::nvs_float_pow` for the float one, there being no
    /// `fpow` on any target and no `LibCall` for it either.
    Pow,
    /// `&` — `rule:types/arithmetic` preserves the operand type, and this and the
    /// bitwise operators below are total: no pair of `int`s or `uint`s has an
    /// unrepresentable bitwise combination, so none of them carries
    /// [`Inst::on_error`].
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
    /// `>>` — [`Self::Shl`]'s rules, and one of its own: `rule:types/arithmetic` makes
    /// this **arithmetic** on an `int` and **logical** on a `uint`, so a count
    /// past the width fills with the sign bit in the first case and with zero
    /// in the second.
    Shr,
    /// `==` — `rule:expressions/one-equality-operator` makes this the language's only equality operator, with
    /// no conversion of either operand. Every operand pair that reaches here
    /// has one statically known representation, and `nvs-codegen` picks that
    /// row's comparison from it: a machine compare for a scalar, a call to
    /// `nvs_runtime::nvs_str_eq` or `nvs_runtime::nvs_array_eq` for a `string`
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
    /// objects are `rule:classes/comparable`'s `Comparable::compareTo` call, whose `int` result
    /// already *is* this operator's answer.
    Cmp,
}

/// A unary operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum UnOp {
    /// Numeric negation, `-x`. Over an `int` or a `uint` it carries
    /// [`Inst::on_error`]: `-i64::MIN` has no `int` and `-$u` no `uint` for any
    /// non-zero `$u`, and `rule:types/arithmetic` makes both a throw rather than a wrap.
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
        /// The [`crate::ty::Ty::Object`] being raised, which is all this
        /// terminator asks of it. A `throw` statement's operand is a
        /// `Throwable` or one of its subclasses, but the `catch` machinery on
        /// the other end matches by descriptor
        /// (`nvs_runtime::object::ClassDesc::conforms_to_name`, self-or-ancestor
        /// by name), so an object of a class outside that tree —
        /// `nvs_hir::errors::FINISH_MARKER` is the one the compiler declares —
        /// travels this edge, runs every `finally` on the way and matches no
        /// arm. Ownership of one reference transfers to the context;
        /// `crate::lower` retains an aliasing operand (`throw $e;`) first.
        value: ValueId,
        /// Where this `throw` is, as [`InstKind::SourceConst`] carries it: the
        /// constant `nvs_runtime::nvs_raise` fills the raised object's
        /// `location` from, and the same one a record producer takes as its
        /// argument 0, so `rule:errors/a-record-names-where-it-was-produced`
        /// has one datum with two readers rather than two spellings of where.
        ///
        /// `None` where the raise is no site of its own — a `catch` matching
        /// no clause hands the very same reference onward — and the runtime
        /// then leaves the location standing that the first throw wrote.
        source: Option<ValueId>,
        /// The landing block this frame's cleanup lives in, exactly the block
        /// [`Inst::on_error`] would name for a call at this same point.
        landing: BlockId,
    },
    /// A landing block's exit when nothing in this frame handles the failure:
    /// record `frame` on the pending exception's backtrace, then return the
    /// status onward unchanged.
    ///
    /// `frame` is the fully rendered `Class::method() at <file>:<line>` label,
    /// built at lowering time because that is where a [`nvs_diagnostics::Span`]
    /// can still be resolved to a line — `nvs-codegen` sees only byte offsets.
    /// The line is the enclosing statement's, read from the same per-statement
    /// span table [`Function::stmt_spans`] already carries for `rule:testing/debug-probes`'s
    /// probes; there is no second position table.
    ///
    /// A `FATAL` pushes nothing: it is not a `Throwable` at all
    /// (`rule:errors/escalation-ladder`), which
    /// is why the status travels to `nvs_trace_push` rather than being decided
    /// here.
    Propagate {
        /// The backtrace label — see above.
        frame: String,
    },
    /// A landing block's exit inside a `try`: on `THROWN`, enter `handler`;
    /// on any other non-`OK` status, take `onward`.
    ///
    /// No frame is *pushed* here, and deliberately so: the backtrace holds the
    /// frames the exception actually unwound *out of*, and a caught throw
    /// never leaves this one. See `nvs_runtime::throwable`'s own docs for why
    /// that differs from PHP's construction-time stack snapshot. That makes
    /// this the one landing site where a raise carrying no site of its own
    /// would reach a `catch` naming nowhere, which is what
    /// [`InstKind::SeedRaiseSite`] sits on the `handler` edge to close.
    ///
    /// `onward` is a second landing block of this same frame, and it is what
    /// makes the two exits release the same things. Entering `handler`
    /// releases no local, because the handler and everything after it still
    /// name them; leaving the frame instead has to drop every one, exactly as
    /// [`Terminator::Propagate`] does at a site with no `catch` above it. A
    /// `FATAL` and an `EXITED` are the statuses that take this edge — no
    /// `catch` admits either (`rule:errors/escalation-ladder`)
    /// — so without it a `try` region would turn every one of them into a
    /// leak of the whole frame.
    Catch {
        /// The catchable exit: a block of this landing site's own that jumps
        /// to the region's dispatch. It exists so that edge-local code has
        /// somewhere to sit that the `onward` exit does not also run through
        /// — `nvs_ir::lower::Lowering::landing_block` owns the reasoning, and
        /// the region's phis name *this* block as their predecessor.
        handler: BlockId,
        /// The uncatchable-status exit: releases this frame's locals and ends
        /// in [`Terminator::Propagate`]. See above.
        onward: BlockId,
    },
    /// An N-way branch on an integer value: the first arm whose case equals
    /// `value` is entered, `default` when none does.
    ///
    /// Built for `rule:iteration/generators`'s generator resumption — `crate::lower`'s
    /// generator section owns what the cases mean there — and shaped as a
    /// general N-way terminator rather than a resumption-specific one, which
    /// is what keeps every consumer's `match` on [`Terminator`] honest.
    ///
    /// **Novis's own `switch` statement does not use it.** Its cases are
    /// arbitrary expressions of the subject's type rather than integers —
    /// `case "A":` is one string comparison call — so it lowers to a chain of
    /// [`Terminator::Branch`]es instead; `crate::lower::Lowering::lower_switch`
    /// owns that reasoning.
    ///
    /// Cases are matched in order and are **not** required to be dense,
    /// contiguous or sorted; a duplicate case is unreachable rather than an
    /// error, exactly as a duplicate `if` arm would be. `nvs-codegen` lowers
    /// this to a compare chain, which is why order is what it is — a jump
    /// table over a dense case set is the obvious optimisation and is
    /// deliberately not taken yet (see `nvs-codegen`'s own known gaps).
    ///
    /// Each arm carries its own [`EdgeId`] for the same reason
    /// [`Terminator::Branch`]'s two do: `rule:testing/debug-probes`'s branch probe is
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
    /// A two-way conditional branch, carrying the [`EdgeId`] `rule:testing/debug-probes`'s
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
