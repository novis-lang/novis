//! Lowers one already-checked method body to a [`crate::ir::Function`] — see
//! the crate's own module docs for which statement and expression shapes are
//! covered, and why this trusts its input rather than re-checking it.
//!
//! # Layout
//!
//! One `impl Lowering` split across this directory, which Rust allows for an
//! inherent impl inside a single crate. Each module's methods are
//! `pub(crate)`, reaching exactly as far as `lower/` and no further. The split
//! is for collision surface: `for`, `switch`, `match`, `$fn(...)` and `rule:classes/no-traits`'s
//! `by`-delegation all land here, and two sessions adding two of them should
//! not conflict.
//!
//! | module | holds |
//! |---|---|
//! | this one | the [`Lowering`] struct, the `lower_*` entry points, the `emit_*` primitives, the refcount helpers, and the declared-type lowering |
//! | [`stmt`] | statement dispatch, assignment, `unset` |
//! | [`expr`] | expression dispatch, conversions, truthiness, short-circuiting operators |
//! | [`control`] | `if`, `while`, both `foreach` shapes, `break`/`continue`, the env merge |
//! | [`exception`] | `throw`, `try`/`catch`, the landing blocks, the synthesized `Throwable` constructor |
//! | [`generator`] | `rule:iteration/generators`'s state machine — the frame, the spills, the synthesized methods |
//! | [`call`] | argument ownership, options-bag flattening, an `inout $x` argument staged and written back |
//! | [`closure`] | `rule:types/closure-literal` closure literals and their captured-environment class |
//!
//! # Control flow (`if`/`while`)
//!
//! [`Lowering`] builds a function's basic blocks incrementally: at any point
//! there is exactly one "current" block still open (not yet given a
//! [`Terminator`]), threaded through the recursive lowering calls as
//! `cur: &mut BlockId`. Lowering a plain statement appends to that block;
//! lowering `if`/`while` seals it with a [`Terminator::Branch`] and hands
//! back a new current block — the merge point (`if`) or the loop's exit
//! (`while`) — for whatever comes next. Every block created along the way is
//! guaranteed to eventually get sealed: either by a `return` inside it, by an
//! explicit `Jump` back to a merge/loop point once its own statements are
//! lowered, or, for the one block still open when the whole method body is
//! done, by [`lower_method`]'s own fallback `return;`.
//!
//! `if`'s join and `while`'s loop-header join are each a single, hand-rolled
//! two-predecessor (or, for a loop header, pre-loop/back-edge) SSA merge —
//! not a general dominance-based phi-placement algorithm, since a structured
//! `if`/`while` only ever has that one shape of join point. A local that
//! keeps the same [`ValueId`] on every incoming edge needs no phi at all
//! (SSA value numbering falls out for free, same as the straight-line
//! slice); one that differs gets a fresh [`crate::ir::InstKind::Phi`] in the
//! join block. Only a local already bound *before* the `if`/`while` can ever
//! need a phi — a name that is missing from some incoming environment (e.g.
//! declared only inside one branch) never reaches the merge, because
//! `nvs_types::check_program` already required it to be definitely assigned
//! on every path before this slice's input is trusted; using it after would
//! already have been rejected there. Such a name still owes a *release* on
//! the edges that do bind it, which is why the merge walks the union of every
//! incoming environment's names rather than the first one's — a local
//! declared inside a `try` body is bound on the landing edges taken after its
//! declaration and not on the ones taken before it, in either order.
//!
//! A `while` loop's header phis are seeded before the body is lowered (their
//! back-edge incoming value isn't known yet) and patched afterwards once the
//! body's exit environment is known — see [`Lowering::lower_while`]. Which
//! locals need a header phi at all is decided by a syntactic pre-scan
//! ([`Lowering::collect_reassigned_locals`]), not a second type-check: it
//! only has to be a safe *over-approximation* of "might be reassigned",
//! since a spurious phi is merely redundant, never wrong.
//!
//! [`Lowering::emit_safepoint`] reserves an inert [`crate::ir::InstKind::Safepoint`]
//! at function entry ([`lower_method`]) and on a `while`'s actual back edge
//! (the last thing appended to the body block before it jumps to the
//! header) — see that variant's own doc comment for why only the shape is
//! reserved.

use nvs_diagnostics::{SourceFile, SourceId, Span, code};
use nvs_render::Source;
use nvs_syntax::ast::{
    ArrayItem, AssignOp, BinaryOp, Block, CallArgs, CatchArm, CatchClause, ClassMemberKind,
    DestructureElement, DestructureTarget, Expr, ExprKind, FnBody, FnExpr, ForInit, ForeachBinding,
    IncDecOp, MatchArm, MemberName, MethodMember, Modifier, NamespaceDecl, NewTarget,
    ObjectLiteralField, SpawnOption, SpawnOptionKey, Stmt, StmtKind, StringPart, SwitchCase,
    TestOperand, Type, TypeAtom, TypeKind, UnaryOp as AstUnaryOp,
};
use nvs_types::EnumTable;
use nvs_types::expr_table::{ArgSlot, ExprInfo, ExprTypeTable, ForeachDrive, ResolvedCall};
use nvs_types::layout::ClassLayoutTable;
use nvs_types::ty::{Ty as CheckedTy, TypeId, TypeInterner};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{BlockId, EdgeId, IdGen, ValueId};
use crate::ir::{
    AbsentKey, BasicBlock, BinOp, Function, Helper, Inst, InstKind, Prepared, Terminator,
    TestedClass, UnOp,
};
use crate::ty::{EnumRepr, Ty};
use crate::{span_text, strip_sigil};

/// Panics because a shape the front end already refuses reached lowering
/// anyway, naming the diagnostic that refuses it.
///
/// `guarded_by!(code::E_ELEMENT_WRITE_ROOT_NOT_A_PLACE, "…")` panics as
/// `E0700: …`. The code comes first so that a backtrace names the guarantee
/// before the sentence describing it, and the message takes `format!`
/// arguments like any other panic.
///
/// **The spelling is the claim.** A plain `panic!` naming a shape says the gap
/// is open: `tools/holes.py` counts it and `crates/nvs-ir/tests/refusals.rs`
/// ratchets the total down, so it is owed a lowering or a diagnostic. This
/// macro says the opposite — the shape never arrives, because the code it
/// names refuses it where it is written. That test holds the claim to a
/// conformance case expecting the code, so the guarantee is checked rather
/// than asserted. The crate's own § *Known gaps* preamble contrasts the two.
macro_rules! guarded_by {
    ($code:expr, $($message:tt)+) => {
        panic!("{}: {}", $code.as_str(), format_args!($($message)+))
    };
}

// One `impl Lowering` split across this directory — see each module's own
// header. Rust allows that for an inherent impl inside one crate; the methods
// there are `pub(crate)`, which reaches exactly as far as `lower/` and no
// further.
pub(crate) mod call;
pub(crate) mod closure;
pub(crate) mod control;
pub(crate) mod convert;
pub(crate) mod exception;
pub(crate) mod expr;
pub(crate) mod generator;
pub(crate) mod operator;
pub(crate) mod stmt;

// `call`, `control`, `expr` and `stmt` only add methods to the one
// `impl Lowering` below, so they export nothing to import. The ones named
// here also carry free items this module and its siblings call.
use self::{call::delegation_forward, closure::*, exception::*, generator::*};

/// A local's current SSA binding: which value it holds, and at what
/// representation type.
type Env = FxHashMap<String, (ValueId, Ty)>;

/// One loop's exit points, gathered while its body is lowered.
/// [`Lowering::lower_while`]/[`Lowering::lower_foreach`] push one of these
/// onto [`Lowering::loop_stack`] before lowering the body and pop it back off
/// once lowering returns; [`Lowering::lower_break`]/[`Lowering::lower_continue`]
/// read the top frame's `after_block`/`continue_target` and record their own
/// `(block, env)` pair into it. The loop then folds `continue_edges` in
/// alongside the body's own fall-through exit when patching the header's
/// phis, and `break_edges` in alongside the condition's false edge when
/// building the loop's own after-block environment — see
/// [`Lowering::lower_while`]'s own doc comment for exactly how both are
/// combined.
///
/// A `switch` pushes one too ([`Lowering::lower_switch`]) — it is a `break`
/// target without being a loop, which is exactly what `continue_target: None`
/// says.
struct LoopFrame {
    /// Where a `continue` jumps: the loop header, re-running the condition —
    /// except in a `for`, where it is the step block that runs the header's
    /// third clause and *then* reaches the header ([`Lowering::lower_for`]).
    ///
    /// `None` on a `switch`'s frame, which owns a `break` but not a
    /// `continue`: [`Lowering::lower_continue`] walks past it to the innermost
    /// frame that has one, so `continue` inside a `switch` continues the
    /// enclosing **loop**. PHP instead treats a `switch` as a looping
    /// structure there, making a bare `continue` mean `break` and warning that
    /// you probably meant `continue 2` — [`Lowering::lower_switch`]'s own doc
    /// comment records why Novis takes the meaning PHP's own warning points at.
    continue_target: Option<BlockId>,
    /// Where a `break` jumps — the block right after the loop.
    after_block: BlockId,
    /// One `(block, env)` pair per `continue` lowered inside this loop's
    /// body, in source order.
    continue_edges: Vec<(BlockId, Env)>,
    /// One `(block, env)` pair per `break` lowered inside this loop's body,
    /// in source order.
    break_edges: Vec<(BlockId, Env)>,
    /// The locals a `foreach` header rebinds at the top of every iteration —
    /// its key and value bindings, each holding one owned reference for the
    /// length of *that* iteration and no longer. Empty for a `while` loop,
    /// which has no such binding.
    ///
    /// Every point one iteration ends releases them: the body's own
    /// fall-through back edge and a `continue`'s back edge alike (the next
    /// iteration rebinds both from scratch), and a `break`, which ends the
    /// last one. A `return` or a throw needs nothing here — those go through
    /// [`Lowering::release_all_locals`], which sweeps the whole `Env` these
    /// names are ordinary members of.
    iteration_owned: Vec<String>,
    /// Every name the loop *header* binds — everything that existed before the
    /// loop, plus the header phis seeded over it. On a `switch`'s frame it is
    /// simply the environment the `switch` was entered with, so the set a
    /// [`Lowering::end_iteration`] releases is "whatever a case body declared".
    ///
    /// A name in the body's environment that is **not** here was declared
    /// inside the body, so it lives for exactly one iteration and this frame
    /// owns its reference: the next iteration starts from the header
    /// environment again and would never see it, and the loop's exit
    /// environment is built from the header's too. Releasing it is therefore
    /// the same obligation [`Self::iteration_owned`] states for a `foreach`
    /// header's key and value, arrived at from the other direction — see
    /// [`Lowering::end_iteration`], which discharges both.
    carried: FxHashSet<String>,
    /// The reserved `Env` names a `foreach` keeps its own bookkeeping under —
    /// its retained reference to the array being walked, and its cursor. They
    /// are dropped from a `break` edge's recorded environment so nothing after
    /// the loop can see them; see [`Lowering::lower_foreach`] for why they
    /// live in the `Env` at all. A `switch` uses the same slot for the one
    /// reference it holds to its subject ([`Lowering::lower_switch`]).
    loop_private: Vec<String>,
    /// How deep [`Lowering::try_stack`] was when this loop's body started
    /// lowering — the boundary a `break`/`continue` unwinds down to.
    ///
    /// Every frame *above* it is a protected region the jump leaves, so each
    /// one's `finally` runs first, innermost out
    /// ([`Lowering::run_finallys_above`]). Frames at or below it enclose the
    /// whole loop and are not left at all, which is what separates this from
    /// [`Lowering::run_pending_finallys`]'s "pop everything" — a `return`
    /// leaves the function, a `break` leaves one loop.
    try_depth: usize,
}

/// Who owns a call argument's reference once the call runs.
///
/// The two conventions Novis has, and the one thing that differs between
/// lowering an [`InstKind::Call`] and an [`InstKind::CoreCall`] beyond which
/// instruction is emitted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ArgOwnership {
    /// The callee owns it — an Novis method or constructor, whose parameter is
    /// bound into its own `Env` like a local and released at its exit sweep.
    /// The caller therefore retains an aliasing refcounted argument first, so
    /// the pair balances.
    Transferred,
    /// The callee borrows it — every `rule:errors/propagation` helper, including a `Core`
    /// member, which receives a `&[Value]` and releases nothing. No retain,
    /// and the caller keeps owning what it passed; `nvs_stdlib`'s own docs own
    /// why `rule:core-api/shape-rules`'s purity rule is what makes that safe.
    Borrowed,
}

/// One `try` statement's protected region, gathered while its body is lowered.
///
/// [`Lowering::lower_try`] pushes one before lowering the body and pops it
/// back off after, exactly the way [`LoopFrame`] brackets a loop's body.
/// [`Lowering::landing_block`] reads the top frame to decide whether a failing
/// call in this region propagates onward or branches to a `catch`, and records
/// its own `(landing block, env)` pair here when it is the latter — which
/// [`Lowering::lower_try`] then folds into the handler's phis through the same
/// [`Lowering::merge_envs`] machinery a `break` edge already uses.
/// Where a `catch` clause's body goes when it finishes, and what it takes
/// with it — [`Lowering::lower_try`]'s half of the dispatch it hands
/// [`Lowering::lower_catch_clauses`].
pub(crate) struct CatchJoin<'e> {
    /// The block after the whole `try`, which every completed clause jumps to.
    after_block: BlockId,
    /// The environment at the dispatch block, after its phis — every clause
    /// body starts from a clone of this.
    dispatch_env: Env,
    /// The `(block, env)` pairs `lower_try` will merge into `after_block`'s
    /// own phis; each completed clause appends one.
    after_incoming: &'e mut Vec<(BlockId, Env)>,
}

struct TryFrame<'a> {
    /// Where a failure inside this region goes: the `catch` dispatch block, or
    /// — for a `try`/`finally` with no clauses — the block that runs the
    /// `finally` body and re-raises.
    ///
    /// For the frame pushed around a `catch` clause's *own body* it is that
    /// region's finally-and-re-raise block
    /// ([`Lowering::lower_finally_and_reraise`]) rather than the dispatch the
    /// clause was selected by: the body owes this region's `finally` on the
    /// way out however it leaves, but the clause list it is already running
    /// does not catch what it raises. `None` where that region has no
    /// `finally` at all — nothing is owed, so [`Lowering::landing_block`]
    /// looks straight past the frame and the throw reaches the enclosing
    /// region directly.
    handler: Option<BlockId>,
    /// One `(landing block, env)` pair per protected call site lowered inside
    /// this region's body, in source order.
    edges: Vec<(BlockId, Env)>,
    /// This region's `finally` body, if it has one.
    ///
    /// Held as the *AST* block rather than a lowered one because every exit
    /// out of the protected region lowers its own copy — see
    /// [`Lowering::run_pending_finallys`] for why duplication is the shape
    /// chosen over a subroutine.
    finally: Option<&'a Block>,
}

/// The class labelled `label`'s armed field slots, resolved against its own
/// flattened slot order — [`crate::ir::Class::defaults`].
///
/// Two kinds of entry, which [`nvs_types::FieldDefault::Unset`] owns the
/// sharing of: a declared `= expr` default, and `rule:classes/an-unwritten-property-read-throws`'s never-written
/// marker on a `lateinit` slot (`rule:classes/lateinit`). The two cannot collide — `rule:classes/lateinit-restrictions`'s `lateinit` is what a property with no initializer declares — but the
/// join below is written so that a default wins anyway, since a slot may only
/// be armed once and a written initializer is the one the author can see.
///
/// Its own class first and then every ancestor, so a subclass redeclaring a
/// property wins the slot the two share; a name the layout has no slot for is
/// skipped rather than mis-indexed, exactly as the codec join above skips one.
/// The constant is translated here rather than in `nvs-codegen` because
/// `nvs_types::ConstArg` is everything a *signature* can carry and
/// `nvs_types::FieldDefault` is only what a written property declaration can
/// reach: a variant outside that set is unreachable rather than lossy, since
/// `nvs_types::defaults::eval_property_default` cannot produce one.
fn property_defaults(
    label: &str,
    layout: &nvs_types::ClassLayout,
    exprs: &ExprTypeTable,
) -> Vec<(usize, nvs_types::FieldDefault)> {
    use nvs_types::FieldDefault;

    let mut image: Vec<Option<FieldDefault>> = vec![None; layout.fields.len()];
    let chain = std::iter::once(label).chain(layout.conforms.iter().map(String::as_str));
    for owner in chain {
        for (property, value) in exprs.property_defaults(owner) {
            let Some(slot) = layout.slot_of(property) else {
                continue;
            };
            if image[slot].is_some() {
                continue;
            }
            image[slot] = field_default(value);
        }
        for property in exprs.lateinit_properties(owner) {
            let Some(slot) = layout.slot_of(property) else {
                continue;
            };
            if image[slot].is_some() {
                continue;
            }
            image[slot] = Some(FieldDefault::Unset);
        }
    }
    image
        .into_iter()
        .enumerate()
        .filter_map(|(slot, value)| Some((slot, value?)))
        .collect()
}

/// One evaluated constant, as the runtime's own smaller vocabulary spells it —
/// `None` for a variant neither a written property declaration nor a written
/// parameter default can reach, which is unreachable rather than lossy for
/// [`property_defaults`]' reason and for [`codec_fields`]'. They are one
/// reason twice: `nvs_types::defaults` accepts a literal of the declared type
/// and nothing else, and the two literal sets differ only by the `= []` and the
/// folded named constant a property may additionally write.
fn field_default(value: &nvs_types::ConstArg) -> Option<nvs_types::FieldDefault> {
    use nvs_types::{ConstArg, FieldDefault};

    match value {
        ConstArg::Bool(v) => Some(FieldDefault::Bool(*v)),
        ConstArg::Int(v) => Some(FieldDefault::Int(*v)),
        ConstArg::Uint(v) => Some(FieldDefault::Uint(*v)),
        ConstArg::Float(v) => Some(FieldDefault::Float(*v)),
        ConstArg::Str(s) => Some(FieldDefault::Str(s.clone())),
        ConstArg::EmptyArray => Some(FieldDefault::EmptyArray),
        ConstArg::Null
        | ConstArg::Bytes(_)
        // Not a value at all, and a `Core` bag's omission fill besides — a
        // property slot already starts in this state without a default.
        | ConstArg::NeverWritten
        // Neither is one value: a bag and `rule:core-api/shape-flattens-at-the-abi`'s shape are a fill list
        // apiece, and no property declaration has a `Core` parameter's type.
        | ConstArg::Options(_)
        | ConstArg::RequiredShape(_)
        | ConstArg::Built { .. }
        // `rule:attributes/retrieval-folds-while-checking`'s folded retrieval, which is an expression's value and
        // never a written property default.
        | ConstArg::Shape(_)
        | ConstArg::Array(_) => None,
    }
}

/// Every `static` property the unit declares, in the order that fixes each
/// one's slot number — [`crate::ir::Program::statics`].
///
/// Sorted by `(class, name)` for the reason `lower_program` sorts its classes:
/// the layout table behind `layouts` is a hash map, and a slot number that
/// moved between runs would make every snapshot of a lowered program — and
/// every compiled unit's agreement with the context it is installed in —
/// depend on iteration order.
///
/// Never flattened along the class graph, unlike [`property_defaults`]: a
/// static's storage belongs to the class that *declares* it, and that is the
/// label the checker already resolved every access to.
fn static_props(
    layouts: &ClassLayoutTable,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> Vec<crate::ir::StaticProp> {
    let mut statics: Vec<crate::ir::StaticProp> = layouts
        .iter()
        .flat_map(|(label, _)| {
            exprs
                .static_properties(label)
                .iter()
                .map(move |(name, default)| crate::ir::StaticProp {
                    class: label.to_owned(),
                    name: name.clone(),
                    // The same fallback `field_slots` takes, for the same
                    // reason: a property the checker recorded no type for is
                    // `Ty::Tagged`, which reads and writes the whole 16 bytes
                    // rather than a payload half it cannot name.
                    repr: exprs
                        .property_types(label)
                        .iter()
                        .find(|(property, _)| property == name)
                        .map_or(Ty::Tagged, |(_, ty)| erase_checked_ty(*ty, checked_types)),
                    default_value: default.as_ref().and_then(field_default),
                })
        })
        .collect();
    statics.sort_by(|a, b| (&a.class, &a.name).cmp(&(&b.class, &b.name)));
    statics
}

/// The two things each of `label`'s field slots' *declared* type is asked, in
/// slot order — [`crate::ir::Class::field_reprs`] and
/// [`crate::ir::Class::secret_fields`], answered in one join because they are
/// one walk over one table and two walks could disagree about which
/// declaration won a slot.
///
/// The representation is the whole of what `rule:types/erased-member-access`'s erased **write**
/// check has to go on; the `secret` bit is the whole of what `rule:errors/record-transformations`'s
/// redaction row has, `nvs_types::expr::type_is_secret` deciding it at the one
/// end where the qualifier still exists.
///
/// The join is [`property_defaults`]'s exactly: own class first, then every
/// ancestor, so a subclass redeclaring a property wins the slot the two
/// share. What differs is the fallback. A default a class does not write is
/// simply absent, but every slot needs a *representation* for the vector to
/// stay aligned with [`nvs_types::ClassLayout::fields`], so a slot nothing
/// claims — a class whose signature never reached
/// [`ExprTypeTable::property_types`], or a type this crate does not represent
/// — is [`Ty::Tagged`], which `nvs-codegen` maps to "unchecked" rather than
/// to a tag that would refuse a legal write. A slot nothing claims is not
/// `secret` either, which is the safe direction only because the fallback is
/// reached by a class the checker never typed at all: a declared `secret`
/// property always reaches [`ExprTypeTable::property_types`].
/// `rule:core-classes/derive-attribute`'s field list joined
/// to `layout`'s slot order — the one place both tables are in hand, and the
/// same join for either format ([`crate::ir::Class::codec`] and
/// [`crate::ir::Class::db_codec`] differ in which table they come out of and in
/// nothing else).
///
/// A field the layout has no slot for is dropped rather than mis-indexed, and
/// so is one whose declaration named no constructor parameter:
/// `nvs_types::derive` has already reported the declaration that caused either,
/// and guessing a position here would write this field's value under another
/// property's key or pass it as another parameter.
fn codec_fields(
    codec: Option<&nvs_types::derive::DerivedCodec>,
    layout: &nvs_types::ClassLayout,
) -> Vec<nvs_types::CodecField> {
    codec.map_or_else(Vec::new, |codec| {
        codec
            .fields
            .iter()
            .filter_map(|field| {
                Some(nvs_types::CodecField {
                    key: field.key.clone(),
                    slot: layout.slot_of(&field.property)?,
                    param: field.param?,
                    ty: field.ty,
                    element: field.element.clone(),
                    // The label rides down untouched; `nvs-codegen` is the
                    // first place every descriptor exists, so it is the one
                    // that can resolve it.
                    class: field.class.clone(),
                    // The enum roster rides down untouched too, and needs no
                    // resolution at all: it is already the values themselves.
                    cases: field.cases.clone(),
                    // The second label a shape field needs, rendered here
                    // rather than carried down: a contract's key is this
                    // crate's own spelling, and `nvs_types` holds the contract
                    // it renders.
                    shape: nested_shape_key(field),
                    nullable: field.nullable,
                    // `rule:core-api/required-optional-and-nullable`'s first
                    // column, already read off the constructor parameter's
                    // default by `nvs_types::derive`. The slot order this join
                    // computes says nothing about it, so it rides down
                    // untouched like the two above.
                    required: field.required,
                    // And the constant that default evaluates to, in the
                    // runtime's own vocabulary: the door
                    // `nvs_runtime::CodecField::default` opens is a decoder
                    // with no call site to emit it from.
                    default: field.default.as_ref().and_then(field_default),
                })
            })
            .collect()
    })
}

fn field_slots(
    label: &str,
    layout: &nvs_types::ClassLayout,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> (Vec<Ty>, Vec<bool>) {
    let mut image: Vec<Option<(Ty, bool)>> = vec![None; layout.fields.len()];
    let chain = std::iter::once(label).chain(layout.conforms.iter().map(String::as_str));
    for owner in chain {
        for (property, ty) in exprs.property_types(owner) {
            let Some(slot) = layout.slot_of(property) else {
                continue;
            };
            if image[slot].is_some() {
                continue;
            }
            image[slot] = Some((
                erase_checked_ty(*ty, checked_types),
                nvs_types::expr::type_is_secret(*ty, checked_types),
            ));
        }
    }
    image
        .into_iter()
        .map(|slot| slot.unwrap_or((Ty::Tagged, false)))
        .unzip()
}

/// The name of the **entry** script frame: the top-level statements the
/// runtime enters, which are `files[0]`'s ([`lower_program`]).
///
/// [`lower_program`] and [`lower_file`] take that name as a parameter rather
/// than reading it here, so a caller lowering one file on its own can pick its
/// own — but this is the spelling every caller in this workspace passes, and
/// this constant is its **one home**. It has to have one, because the name is a
/// contract between two crates: this one compiles the frame under it and an
/// embedder looks the compiled function back up under the same string
/// (`nvs_codegen::Unit::script`), with nothing in between to notice a typo but
/// a `None` at run time.
///
/// `<` and `>` are what keep it out of a program's reach, the same job `#` and
/// `$` do in [`file_script_label`]: neither is an identifier character, so no
/// `Class::method` label a source declaration can produce collides with it.
pub const ENTRY_SCRIPT_LABEL: &str = "<script>";

/// The name of the script frame holding the file `id`'s own top-level
/// statements — every file's but the entry's, whose frame keeps the name
/// [`lower_program`]'s caller handed it.
///
/// Computed from the [`SourceId`] at both ends rather than recorded
/// anywhere: [`lower_program`] names the function, and
/// `Lowering::lower_expr_stmt`'s `Require` arm names the call, each holding
/// the id the other does — the producer through `ProgramFile::src`, the
/// consumer through `nvs_types::ExprTypeTable::require_target`.
///
/// `#` and `$` are what make it unspellable, the same thing that keeps a
/// generator's `gen#unwind` out of a program's reach: no `Class::method`
/// label a source declaration can produce collides with one of these.
#[must_use]
pub fn file_script_label(id: SourceId) -> String {
    format!("file#{}$script", id.raw())
}

/// Lowers a whole checked **program**: every class method that has a body in
/// any of `files`, plus each file's own top-level statements as its own
/// script frame — the entry's named `script`, the rest [`file_script_label`].
///
/// This is what a caller with a resolved `require`/`autoload` graph wants —
/// [`lower_file`] is the one-file spelling, and [`lower_method`] and
/// [`lower_script`] stay public for the narrower "lower exactly this one
/// thing" cases the tests use.
///
/// **Every file gets a script frame, and `files[0]`'s is the one the runtime
/// enters.** The entry's is named `script`; every other file's is
/// [`file_script_label`], which the `require` site that named the file calls
/// (`lower_expr_stmt`'s `Require` arm), so a top-level statement written in a
/// required file runs where the `require` is written rather than not at all.
/// N files cannot share one name, and they must not share one *frame*
/// either: `nvs_types::locals` checks each file's top-level body on its own,
/// so a required file's `$x` is not the caller's — declarations cross a
/// `require` and variables do not (`rule:statements/a-required-file-shares-declarations-not-locals`).
/// `nvs_hir::resolve_program`'s entry-first order is what makes indexing
/// position zero right.
///
/// Each method is named with the `Class::method` label
/// `nvs_types::expr_table::ExprTypeTable::method_label` recorded for its
/// declaration, which is the *same* label a call's
/// [`InstKind::Call::target`](crate::ir::InstKind::Call) is rendered from —
/// see that accessor's own doc comment for why the label is spelled in
/// `nvs-types` rather than here. A method whose declaration has no recorded
/// label is skipped: nothing can call it by a name that was never resolved,
/// so lowering it would only produce an unreachable function.
///
/// Enums are not walked; interfaces are, because an `interface` method may
/// carry a body
/// (`rule:classes/interface-default-methods`) and a default is an ordinary compiled method under the interface's own
/// label. § 4's `implements I by $field;` forwards have no declaration to walk
/// at all — one is synthesized per `nvs_types::Delegation` at the end of this
/// function, beside the method-table row that makes it reachable.
///
/// # Panics
///
/// If `files` is empty — a program is its entry file plus whatever that
/// reached, so there is always at least one. Otherwise the same way
/// [`lower_method`]/[`lower_script`] do, naming the shape this slice does not
/// lower. See [`lower_method`]'s own note.
#[must_use]
pub fn lower_program(
    script: &str,
    files: &[nvs_types::ProgramFile<'_>],
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    layouts: &ClassLayoutTable,
) -> crate::ir::Program {
    fn walk(
        stmts: &[Stmt],
        src: &SourceFile,
        exprs: &ExprTypeTable,
        checked_types: &TypeInterner,
        enums: &EnumTable,
        out: &mut Vec<Function>,
        synthesized: &mut Vec<crate::ir::Class>,
    ) {
        for stmt in stmts {
            match &stmt.kind {
                // A namespace scopes names, not storage — the label each
                // method is lowered under already carries the resolved
                // namespace, so this walk only has to reach the declarations
                // inside the block.
                StmtKind::NamespaceDecl(NamespaceDecl {
                    body: Some(block), ..
                }) => walk(
                    &block.stmts,
                    src,
                    exprs,
                    checked_types,
                    enums,
                    out,
                    synthesized,
                ),
                // An `interface`'s default and private method bodies (ADR
                // 0043 § 2/§ 3) are ordinary compiled methods — the interface
                // is where they are *declared*, which is all that differs.
                // Every bodiless member is skipped by the same `m.body`
                // check an `abstract` class method already went through.
                StmtKind::ClassDecl(nvs_syntax::ast::ClassDecl { members, .. })
                | StmtKind::InterfaceDecl(nvs_syntax::ast::InterfaceDecl { members, .. }) => {
                    for member in members {
                        match &member.kind {
                            ClassMemberKind::Method(m) => {
                                if m.body.is_none() {
                                    continue; // `abstract` — nothing to lower
                                }
                                let Some(label) = exprs.method_label(m.name) else {
                                    continue;
                                };
                                // `rule:iteration/generators`: a body containing `yield` is
                                // a generator, and becomes a state class and
                                // its own functions rather than one function
                                // — see `lower_generator`.
                                let body = m
                                    .body
                                    .as_ref()
                                    .expect("just checked this declaration has one");
                                if nvs_syntax::ast::is_generator_body(body) {
                                    let (fns, classes) =
                                        lower_generator(label, m, src, exprs, checked_types, enums);
                                    out.extend(fns);
                                    synthesized.extend(classes);
                                    continue;
                                }
                                let lowered =
                                    lower_method(label, m, src, exprs, checked_types, enums);
                                out.push(lowered.function);
                                out.extend(lowered.closures);
                                synthesized.extend(lowered.classes);
                            }
                            // `rule:classes/property-hooks`'s property hooks are compiled the
                            // same way, under the label `nvs_types` recorded
                            // for the hook itself — see
                            // `lower_property_hook`. A bodiless hook (an
                            // abstract `get;`) is skipped for exactly the
                            // reason an `abstract` method is.
                            ClassMemberKind::Property(p) => {
                                for hook in p.hooks.iter().flatten() {
                                    if hook.body.is_none() {
                                        continue;
                                    }
                                    let Some(label) = exprs.method_label(hook.span) else {
                                        continue;
                                    };
                                    let lowered = lower_property_hook(
                                        label,
                                        p,
                                        hook,
                                        src,
                                        exprs,
                                        checked_types,
                                        enums,
                                    );
                                    out.push(lowered.function);
                                    out.extend(lowered.closures);
                                    synthesized.extend(lowered.classes);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let mut functions = Vec::new();
    let mut synthesized = Vec::new();
    for file in files {
        walk(
            file.stmts,
            file.src,
            exprs,
            checked_types,
            enums,
            &mut functions,
            &mut synthesized,
        );
    }
    assert!(
        !files.is_empty(),
        "a program is its entry file plus whatever that reached"
    );
    for (i, file) in files.iter().enumerate() {
        // Position zero is the runtime's entry point and keeps the name it
        // was handed; every other file is entered from the `require` site
        // that named it, under the label that site computes from the same
        // `SourceId`. That is also the whole of what [`ScriptRole`] asks, so
        // the two are decided together rather than re-derived.
        let (name, role) = if i == 0 {
            (script.to_owned(), ScriptRole::Entry)
        } else {
            (file_script_label(file.src.id()), ScriptRole::Required)
        };
        let lowered = lower_script(
            &name,
            file.stmts,
            file.src,
            exprs,
            checked_types,
            enums,
            role,
        );
        functions.push(lowered.function);
        functions.extend(lowered.closures);
        synthesized.extend(lowered.classes);
    }
    // The functions with no source text — see
    // `synthesized_exception_constructors`. Emitted unconditionally: the
    // exception tree is in every program's class table, so a unit that omitted
    // these would be one where `new LogicError(…)` names a missing target.
    functions.extend(synthesized_exception_constructors());

    // Copied straight across rather than recomputed: `nvs-types` already
    // resolved the slot order and the supertype set against the class graph,
    // which this crate cannot see — see `crate::ir::Class`.
    let mut classes: Vec<crate::ir::Class> = layouts
        .iter()
        .map(|(label, layout)| {
            let (field_reprs, secret_fields) = field_slots(label, layout, exprs, checked_types);
            crate::ir::Class {
                label: label.to_owned(),
                fields: layout.fields.clone(),
                // `rule:types/erased-member-access`'s erased write reaches *any* class, not just a
                // shape literal's synthesized one, so every layout carries its
                // slots' representations — see `field_slots`, which answers the
                // `secret` bit off the same join.
                field_reprs,
                secret_fields,
                // `rule:security/reflection-enforces-visibility`'s visibility bit, copied across with the slot
                // order it is aligned to — `nvs_types::layout` decided it where
                // the declaration's keyword still exists.
                public_fields: layout.public_fields.clone(),
                // The bit beside it that tells a `protected` slot from a
                // `private` one, copied for the same reason and aligned to the
                // same order.
                protected_fields: layout.protected_fields.clone(),
                // The declared type beside it, aligned to the same slot order
                // and copied for the same reason: the spelling lives where the
                // declaration does.
                field_types: layout.field_types.clone(),
                // The class's constants, already flattened over its ancestors
                // and already folded — the front end is the only layer that
                // still has the declaration's right-hand side to fold.
                constants: layout.constants.clone(),
                // The attach sites beside them, own-only and already folded,
                // for the same reason: the front end is the only layer that
                // still has the `#[...]` payload to fold.
                attributes: layout.attributes.clone(),
                conforms: layout.conforms.clone(),
                methods: layout.methods.clone(),
                // The accessors beside them, on the same terms: the front end
                // is the only layer that ever saw the `{ get; set; }` block.
                hooks: layout.hooks.clone(),
                // `rule:core-classes/derive-attribute`'s field list, joined to this class's slot order — the
                // one place both tables are in hand. A field the layout has no
                // slot for is dropped rather than mis-indexed: `nvs_types::derive`
                // has already reported the declaration that caused it, and
                // guessing a slot here would write another property's value under
                // this one's key.
                codec: codec_fields(exprs.codec(label), layout),
                // The row half of the same join, off the same layout — see
                // `crate::ir::Class::db_codec` for why the two lists are two
                // and not one read twice.
                db_codec: codec_fields(exprs.db_codec(label), layout),
                // Whichever format the class declares, and the same number if
                // it declares both: the arity belongs to the `constructor` and
                // not to either mapping.
                ctor_arity: exprs
                    .codec(label)
                    .or_else(|| exprs.db_codec(label))
                    .map_or(0, |codec| codec.ctor_arity),
                // Every declared default that lands in one of this class's slots:
                // its own first, then each ancestor's, so a subclass redeclaring a
                // property wins the slot the two share. A label with no slot for
                // the name is skipped rather than mis-indexed, for the same reason
                // the codec above skips one.
                defaults: property_defaults(label, layout, exprs),
                is_closure: false,
            }
        })
        .collect();
    // `rule:iteration/generators`'s generator state classes have no source declaration and
    // therefore no `nvs_types::layout` entry — `nvs-ir` synthesizes both the
    // class and its methods, so it is the one thing here that adds to the
    // table rather than copying it.
    classes.extend(synthesized);
    // The label every closure's environment class conforms to, emitted
    // unconditionally for `synthesized_exception_constructors`' reason: a unit
    // that left it out would be one where `$x is callable` names a descriptor
    // the unit does not declare instead of answering `false`. It declares no
    // field and no method, the whole of what it carries being its own identity.
    // See `CLOSURE_MARKER`.
    classes.push(crate::ir::Class {
        label: CLOSURE_MARKER.to_owned(),
        fields: Vec::new(),
        field_reprs: Vec::new(),
        secret_fields: Vec::new(),
        public_fields: Vec::new(),
        protected_fields: Vec::new(),
        constants: Vec::new(),
        attributes: Vec::new(),
        field_types: Vec::new(),
        conforms: Vec::new(),
        methods: Vec::new(),
        hooks: Vec::new(),
        codec: Vec::new(),
        db_codec: Vec::new(),
        ctor_arity: 0,
        defaults: Vec::new(),
        // The marker every closure conforms to is not itself a closure:
        // nothing is ever an instance of it.
        is_closure: false,
    });
    // One marker supertype per written `callable(...)` signature some `is` in
    // this program tested, each carrying no field and no method for
    // `CLOSURE_MARKER`'s reason: the whole of what one holds is its own
    // identity, which is what the descriptor walk behind
    // `$x is callable(int): string` compares. Which closures conform is in
    // `nvs_types::callables`, because the relation is `is_assignable` itself
    // and needs a `ClassGraph`, a `SignatureTable` and a mutable interner —
    // none of which lowering holds; this reads the answer back off the table
    // and the edges go on each literal's class in `super::closure`.
    classes.extend(exprs.callable_sig_markers().map(|marker| crate::ir::Class {
        label: marker.to_owned(),
        fields: Vec::new(),
        field_reprs: Vec::new(),
        secret_fields: Vec::new(),
        public_fields: Vec::new(),
        protected_fields: Vec::new(),
        constants: Vec::new(),
        attributes: Vec::new(),
        field_types: Vec::new(),
        conforms: Vec::new(),
        methods: Vec::new(),
        hooks: Vec::new(),
        codec: Vec::new(),
        db_codec: Vec::new(),
        ctor_arity: 0,
        defaults: Vec::new(),
        is_closure: false,
    }));
    // A shape reached as a *field* has no call site to be lowered at, so the
    // class it decodes into is collected off the derived codecs here. A label a
    // literal in this unit also wrote is one class, which the dedup below
    // decides, and it keeps the literal's own slot representations because that
    // entry comes first.
    let mut shape_codecs = shape_codecs(exprs, &mut classes);
    for (label, _) in layouts.iter() {
        if let Some(codec) = exprs.codec(label) {
            nested_shapes(codec, &mut shape_codecs, &mut classes);
        }
    }
    // Sorted and deduped on the key for the reason the collection itself is not
    // ordered: the table is walked to publish one relocation symbol per entry,
    // and two sites writing one contract are one table.
    shape_codecs.sort_by(|a, b| a.key.cmp(&b.key));
    shape_codecs.dedup_by(|a, b| a.key == b.key);
    // The table behind `iter()` is a hash map, so its order varies run to run.
    // Sorting here is what makes a lowered `Program` — and therefore the
    // `--dump-ir` listing and every snapshot taken of it — reproducible for an
    // unchanged file, the same stability `crate::ids` already guarantees for a
    // statement id.
    classes.sort_by(|a, b| a.label.cmp(&b.label));
    // A shape literal's synthesized class is keyed on the shape, not the site
    // — see `shape_class_label` — so two bodies both writing `{x: 1, y: 2}`
    // each record one. Nothing else here can repeat a label: `layouts` is a
    // map, and a closure's and a generator's class are named for the site.
    //
    // Collapsing the repeats **merges** their slot representations on
    // `Lowering::record_shape_class`'s terms, rather than keeping whichever
    // one sorted first: that function already merges the records one frame
    // makes, and a label is program-wide, so two frames writing `{x: 1}` and
    // `{x: "s"}` have to reach the same degraded tag that one frame writing
    // both does. Keeping the first instead would let the frame a class was
    // sorted out of decide what every other frame's writes are checked
    // against.
    classes.dedup_by(|a, b| {
        if a.label != b.label {
            return false;
        }
        for (have, found) in b.field_reprs.iter_mut().zip(&a.field_reprs) {
            if *have != *found {
                *have = Ty::Tagged;
            }
        }
        true
    });
    // `rule:classes/delegation-by-field`'s `implements I by $field;` forwards: one synthesized
    // method each, and one row each in the delegating class's method table,
    // which is what a receiver typed as the *interface* dispatches through.
    // `nvs_types::Delegation` is where every one of these was decided; this
    // adds the two things only a lowered program has, the function and the
    // row. A name the table already answers is left alone — § 4's "a class may
    // still write its own method with the same name as a delegated one", and
    // an inherited body or an `rule:classes/interface-default-methods` interface default on the same
    // terms.
    for delegation in exprs.delegations() {
        let Some(class) = classes
            .iter_mut()
            .find(|class| class.label == delegation.class)
        else {
            continue;
        };
        if class
            .methods
            .iter()
            .any(|(name, _, _, _, _, _)| *name == delegation.method)
        {
            continue;
        }
        // `rule:classes/an-unwritten-property-read-throws` on the forward's own read: a `lateinit` delegate
        // field (`rule:classes/lateinit`) is the one E0720 admits that no constructor is
        // obliged to fill, and dispatching on what it then holds is what
        // `delegation_forward`'s guard exists to stop.
        let never_written = exprs.is_lateinit_property(&delegation.class, &delegation.field);
        let Some(function) = delegation_forward(delegation, checked_types, never_written) else {
            continue;
        };
        // Public: § 4 synthesizes a forward for an *interface* member, and an
        // interface has no other visibility to inherit. Neither parameter
        // roster is filled, which is
        // `nvs_types::layout::ClassLayout::methods`' empty case:
        // `nvs_types::Delegation` carries the forwarded member's parameter
        // types positionally as checked ids rather than as the text a
        // declaration wrote, and nothing anywhere names them — the forward is
        // written by no one.
        class.methods.push((
            delegation.method.clone(),
            delegation.class.clone(),
            true,
            false,
            Vec::new(),
            Vec::new(),
        ));
        functions.push(function);
    }

    crate::ir::Program {
        functions,
        classes,
        statics: static_props(layouts, exprs, checked_types),
        shape_codecs,
        enums: enum_shapes(enums),
    }
}

/// Every enum the checker resolved, as the list `nvs-codegen` hands to
/// `nvs_runtime::ClassTable::define_enum` — `rule:enums/reflection`'s whole
/// carriage from the front end to the runtime.
///
/// The table's own entries, unfiltered: a program can ask
/// `Core\Reflect\EnumInfo::of` for a `Core` enum as readily as for one of its
/// own, and the seeded rows are the only source either way. Sorted by label so
/// a unit's roster does not move with a hash map's iteration order, which a
/// golden test would otherwise read as a change.
fn enum_shapes(enums: &EnumTable) -> Vec<crate::ir::Enum> {
    let mut shapes: Vec<_> = enums
        .iter()
        .map(|(qname, info)| crate::ir::Enum {
            label: qname.to_string(),
            unsigned: info.backing == nvs_types::enums::EnumBacking::Uint,
            cases: info
                .cases
                .iter()
                .map(|(name, value)| {
                    let widened = match *value {
                        nvs_types::enums::EnumValue::Int(signed) => i128::from(signed),
                        nvs_types::enums::EnumValue::Uint(unsigned) => i128::from(unsigned),
                    };
                    (name.clone(), widened)
                })
                .collect(),
        })
        .collect();
    shapes.sort_by(|a, b| a.label.cmp(&b.label));
    shapes
}

/// Every inline shape the unit wrote as a type argument, as the table
/// `nvs-codegen` materializes and [`crate::ir::InstKind::ShapeCodecConst`]
/// names an entry of, plus every shape reached through one of their fields.
///
/// Read off the checker's own record of what each call site wrote, rather than
/// collected out of the functions that lowered: a contract is a fact about the
/// program and a lowering walks one function at a time, so two sites writing
/// one shape would otherwise each carry a list and be merged anyway. The
/// ordering and the merge are [`lower_program`]'s, which is where the shapes
/// reached through a *class's* fields join these.
fn shape_codecs(
    exprs: &ExprTypeTable,
    classes: &mut Vec<crate::ir::Class>,
) -> Vec<crate::ir::ShapeCodec> {
    let mut out: Vec<crate::ir::ShapeCodec> = Vec::new();
    for codec in exprs.shape_codecs() {
        let fields = shape_codec_fields(codec);
        out.push(crate::ir::ShapeCodec {
            key: shape_codec_key(&fields),
            fields,
        });
        nested_shapes(codec, &mut out, classes);
    }
    out
}

/// [`lower_program`] over a program of exactly one file — the shape a
/// self-contained script and every fixture in this crate's own tests has.
///
/// # Panics
///
/// The same way [`lower_program`] does.
#[must_use]
pub fn lower_file(
    script: &str,
    stmts: &[Stmt],
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    layouts: &ClassLayoutTable,
) -> crate::ir::Program {
    lower_program(
        script,
        &[nvs_types::ProgramFile { src, stmts }],
        exprs,
        checked_types,
        enums,
        layouts,
    )
}

/// Lowers `m` — which must have a body, and whose body must stay within this
/// slice's supported statement/expression shapes (see the crate docs) — to a
/// [`Function`] named `name`. `exprs`/`checked_types` are the
/// `nvs_types::check_program` run's own typed-expression table and type
/// interner — the source of truth for a call's/`new`'s resolved target (see
/// [`ExprInfo`] and the crate docs' "design choices" section).
///
/// `Function::params`' index 0 is always the implicit receiver (`$this`),
/// seeded into `Env` here before any explicit parameter — see
/// [`Function::params`](crate::ir::Function::params)'s own doc comment and
/// the crate docs' design-choices section for why every lowered method
/// carries it unconditionally.
///
/// # Panics
///
/// Panics, naming the unsupported shape, if `m` has no body or its body
/// leaves this slice's scope. This is not a diagnostic: callers are expected
/// to have already run `nvs_types::check_program` — with the very `exprs`/
/// `checked_types` passed here — and to only route programs within scope
/// through this function until lowering widens.
#[must_use]
pub fn lower_method(
    name: &str,
    m: &MethodMember,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> Lowered {
    let ret_ty = m
        .return_type
        .as_ref()
        .map_or(Ty::Void, |t| lower_decl_type(t, exprs, checked_types));
    let mut low = Lowering::new(name, Some(name), src, ret_ty, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    let mut env = Env::default();
    let mut param_tys = Vec::new();

    // Reserved safepoint poll site (recursion) — see `InstKind::Safepoint`'s
    // own doc comment for why function entry is a fixed site and why this
    // slice reserves only the shape, not a functional check.
    low.emit_safepoint(entry);

    // The implicit receiver, always parameter index 0 — seeded
    // unconditionally, the same way `nvs_types::check.rs`'s `check_method`
    // seeds `$this` into its own `LocalScope` regardless of a `static`
    // modifier (see that function's own comment for why: a static method's
    // body referencing `$this` is a distinct, unrelated diagnostic, not this
    // crate's concern). `nvs-ir` never lowers a free function — every
    // `MethodMember` it reaches belongs to a class per `rule:classes/no-free-functions-or-constants` — so there is
    // no case where a receiver truly doesn't exist. This is the "implicit
    // first parameter" shape from the crate docs' design-choices section,
    // chosen over a receiver-only special case so `ExprKind::MethodCall`'s
    // `$this`/an arbitrary receiver both flow through the ordinary
    // `ExprKind::Variable`/`Env` lookup path with no new machinery.
    //
    // A `static` method has no `$this` to put there, so that slot carries the
    // *called* class instead — late static binding's whole mechanism, and the
    // reason it costs no second parameter and no second call convention. See
    // `nvs_runtime::object`'s module docs, which own the decision.
    let is_static = m.modifiers.contains(&Modifier::Static);
    let recv_ty = if is_static { Ty::ClassDesc } else { Ty::Object };
    let (this_v, _) = low.emit(entry, recv_ty, InstKind::Param(0));
    param_tys.push(recv_ty);
    if is_static {
        low.lsb = Some(this_v);
    } else {
        env.insert("this".to_owned(), (this_v, Ty::Object));
        low.this = Some(this_v);
    }

    for (i, p) in m.params.iter().enumerate() {
        let decl_ty = p.ty.as_ref().unwrap_or_else(|| {
            panic!("`rule:types/declaration`: every parameter has a declared type")
        });
        // `...$rest`'s declared type is what each trailing *argument* is
        // checked against; the slot itself receives the one array
        // `Lowering::lower_variadic_tail` collected them into, which is what
        // the caller has always passed. `nvs_types::check` binds the body's
        // own name at `array<` that `>` for the same reason.
        let ty = match p.variadic {
            true => Ty::Array,
            false => lower_decl_type(decl_ty, exprs, checked_types),
        };
        // +1: index 0 is always the implicit receiver seeded above.
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        // `inout $x` — the incoming slot holds the address of one caller-staged
        // `Value` cell rather than a value of the declared type, so the
        // binding is a `Ty::Ref` and the declared type is remembered as the
        // *pointee*: every read of the parameter becomes an
        // `InstKind::RefLoad` at that type and every write an
        // `InstKind::RefStore`. See `Ty::Ref` for the whole representation.
        let bound_ty = if p.inout {
            low.ref_locals.insert(pname.clone(), ty);
            Ty::Ref
        } else {
            ty
        };
        let (v, _) = low.emit(entry, bound_ty, InstKind::Param(index));
        env.insert(pname, (v, bound_ty));
        param_tys.push(bound_ty);
    }

    // PHP 8's constructor promotion: the parameter *is* the property, so the
    // store the author did not write is emitted here, before the body, in
    // declaration order. See `promoted_stores`.
    if let Some(class) = name.rsplit_once("::").and_then(|(class, method)| {
        (method == "constructor" && !is_static).then_some(class.to_owned())
    }) {
        promoted_stores(&mut low, entry, &class, m, src, &env);
    }

    let body = m.body.as_ref().expect(
        "lower_method requires a method with a body — nothing to lower for an abstract one",
    );
    low.lower_stmts(&body.stmts, &mut cur, &mut env);
    // No explicit final `return` — the fallback seals whatever block is still
    // open. Nothing transfers out on this path (there is no return value), so
    // every refcounted local still live here gets released, same as an
    // explicit `return;`.
    if !low.is_terminated(cur) {
        low.release_all_locals(cur, &env, None);
        low.seal(cur, Terminator::Return(None));
    }

    let pending = std::mem::take(&mut low.closures);
    // Beside the closures, and out the same channel: see `Lowering::callables`.
    let callables = std::mem::take(&mut low.callables);
    // Beside the closures, and out the same channel: see `Lowering::shapes`.
    let shapes = std::mem::take(&mut low.shapes);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, mut classes) =
        drain_closures(pending, callables, src, exprs, checked_types, enums);
    classes.extend(shapes);
    Lowered {
        function: Function {
            name: name.to_owned(),
            params: param_tys,
            ret: ret_ty,
            blocks,
            entry,
            stmt_spans,
            edge_spans,
        },
        closures,
        classes,
    }
}

/// Emits the field store each promoted constructor parameter stands for —
/// `constructor(public int $n)` being exactly `constructor(int $n) { $this->n
/// = $n; }` with the assignment supplied here rather than written.
///
/// Emitted into the entry block, ahead of the body, so a constructor that
/// reads `$this->n` reads what it was passed, and in declaration order, which
/// is the order the author would have written the assignments in.
///
/// Two halves of [`Lowering::lower_reassignment`]'s property policy apply and
/// one does not. The **retain** does: a compiled method owns its parameters
/// and releases each at every exit, so the slot needs a reference of its own.
/// The **coercion** does not, because a promoted parameter's declared type is
/// the property's declared type — one declaration, one representation. Nor
/// does the **release of the old value**: this is the first store this slot
/// ever sees, the instance having been allocated by the `new` that reached
/// this call, so there is nothing there to drop.
///
/// A `static` method cannot be a constructor and is not asked; a parameter
/// with no visibility keyword declares nothing
/// ([`nvs_syntax::ast::Param::is_promoted`]).
fn promoted_stores(
    low: &mut Lowering<'_>,
    entry: BlockId,
    class: &str,
    m: &MethodMember,
    src: &SourceFile,
    env: &Env,
) {
    let this = low
        .this
        .expect("an instance method's receiver is seeded before its parameters");
    for p in m.params.iter().filter(|p| p.is_promoted()) {
        let field = strip_sigil(span_text(src, p.name)).to_owned();
        let Some(&(v, ty)) = env.get(&field) else {
            continue; // the binding above skipped it; nothing to store
        };
        if ty.is_refcounted() {
            low.emit_retain(entry, v);
        }
        low.emit_field_set(entry, this, class.to_owned(), field, v);
    }
}

/// Lowers one of `p`'s `rule:classes/property-hooks` property hooks to a [`Function`] named `name` — which must be the
/// label `nvs_types::signatures::hook_label` spelled for it, since a `set`
/// hook's short form recovers the declaring class's own label back out of it.
///
/// A hook is an ordinary compiled function, deliberately: it takes the same
/// implicit receiver in parameter slot 0 that [`lower_method`] gives every
/// method, it releases that receiver at every exit under the same
/// convention, and it is reached through the same [`InstKind::Call`]. That is
/// the whole reason a hooked property access costs no new instruction, no new
/// calling convention and no dispatch table entry — see
/// [`ExprInfo::HookedProperty`], which is where a *read* or *write* turns
/// into a call to one of these.
///
/// Everything the two accessors differ in:
///
/// - **`get`** returns the property's declared type; **`set`** returns
///   nothing and takes the incoming value as parameter slot 1, named by the
///   declaration or, left implicit, by `nvs_types::HOOK_VALUE_PARAM`.
/// - **The short `=> expr;` body** means "return this" for `get` and "store
///   this" for `set`, matching PHP 8.4. The store is emitted here rather than
///   desugared into the AST, because there is no AST node to desugar into.
/// - A `set` hook's store names the *backing slot* directly. Inside a hook,
///   the property is always its own storage — `nvs_types::Ctx::current_hook`
///   is what keeps the checker from recording a re-entrant
///   [`ExprInfo::HookedProperty`] for it.
///
/// # Panics
///
/// The same way [`lower_method`] does, plus: if `name` is not a hook label
/// and the hook needs the declaring class back out of it.
#[must_use]
pub fn lower_property_hook(
    name: &str,
    p: &nvs_syntax::ast::PropertyMember,
    hook: &nvs_syntax::ast::PropertyHook,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> Lowered {
    use nvs_syntax::ast::{PropertyHookBody, PropertyHookKind};

    let prop_ty = lower_decl_type(&p.ty, exprs, checked_types);
    let is_set = hook.kind == PropertyHookKind::Set;
    let ret_ty = if is_set { Ty::Void } else { prop_ty };
    let mut low = Lowering::new(name, Some(name), src, ret_ty, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    let mut env = Env::default();
    let mut param_tys = vec![Ty::Object];

    low.emit_safepoint(entry);
    let (this_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    env.insert("this".to_owned(), (this_v, Ty::Object));
    low.this = Some(this_v);

    if is_set {
        let (pname, pty) = match &hook.param {
            Some(param) => (
                strip_sigil(span_text(src, param.name)).to_owned(),
                param
                    .ty
                    .as_ref()
                    .map_or(prop_ty, |t| lower_decl_type(t, exprs, checked_types)),
            ),
            None => (nvs_types::HOOK_VALUE_PARAM.to_owned(), prop_ty),
        };
        let (v, _) = low.emit(entry, pty, InstKind::Param(1));
        env.insert(pname, (v, pty));
        param_tys.push(pty);
    }

    match &hook.body {
        Some(PropertyHookBody::Block(block)) => low.lower_stmts(&block.stmts, &mut cur, &mut env),
        Some(PropertyHookBody::Expr(e)) if is_set => {
            let class = nvs_types::signatures::hook_label_class(name)
                .unwrap_or_else(|| {
                    panic!(
                        "nvs-ir: `{name}` is not a hook label, so the class whose slot a \
                         `set => expr;` hook stores into cannot be recovered from it"
                    )
                })
                .to_owned();
            let field = strip_sigil(span_text(src, p.name)).to_owned();
            let (v, _) = low.lower_expr(e, Some(prop_ty), &mut env, &mut cur);
            if prop_ty.is_refcounted() && low.aliasing_read(e) {
                low.emit_retain(cur, v);
            }
            if prop_ty.is_refcounted() {
                let (old_v, _) = low.emit(
                    cur,
                    prop_ty,
                    InstKind::FieldGet {
                        object: this_v,
                        class: class.clone(),
                        field: field.clone(),
                    },
                );
                low.emit_release(cur, old_v);
            }
            low.emit_field_set(cur, this_v, class, field, v);
        }
        Some(PropertyHookBody::Expr(e)) => {
            let (v, ty) = low.lower_expr(e, Some(ret_ty), &mut env, &mut cur);
            if ty.is_refcounted() && low.aliasing_read(e) {
                low.emit_retain(cur, v);
            }
            if !low.is_terminated(cur) {
                low.release_all_locals(cur, &env, None);
                low.seal(cur, Terminator::Return(Some(v)));
            }
        }
        None => unreachable!("lower_property_hook is only called for a hook with a body"),
    }
    if !low.is_terminated(cur) {
        low.release_all_locals(cur, &env, None);
        low.seal(cur, Terminator::Return(None));
    }

    let pending = std::mem::take(&mut low.closures);
    // Beside the closures, and out the same channel: see `Lowering::callables`.
    let callables = std::mem::take(&mut low.callables);
    // Beside the closures, and out the same channel: see `Lowering::shapes`.
    let shapes = std::mem::take(&mut low.shapes);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, mut classes) =
        drain_closures(pending, callables, src, exprs, checked_types, enums);
    classes.extend(shapes);
    Lowered {
        function: Function {
            name: name.to_owned(),
            params: param_tys,
            ret: ret_ty,
            blocks,
            entry,
            stmt_spans,
            edge_spans,
        },
        closures,
        classes,
    }
}

/// Who enters a script frame — which is the whole of what a file that runs
/// out of statements answers with.
///
/// The two ADRs disagree here on purpose, and neither value is derivable
/// from the other at run time: by the time a `spawn script` child's answer
/// reaches `nvs_host::Isolate`'s join it is one `Value`, and an explicit
/// `return 1` is indistinguishable from a fall-through. So the difference is
/// spelled once, where the frame is lowered and its caller is still known.
///
/// - [`ScriptRole::Required`] — `rule:statements/a-require-expression-is-mixed`'s `1`, which is PHP's own answer for an `include` of a file that
///   never `return`s, kept for the construct PHP has.
/// - [`ScriptRole::Entry`] — `null`, "the value every Novis function without
///   a `return` produces"
///   ([ADR 0006](/docs/decisions/0006.md)
///   § *Decision*, which names it *deliberately not* `require`'s `1`). The
///   entry frame is the one a `spawn script` child is, so this is the child's
///   answer; under `nvs run` nothing reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRole {
    /// The program's own entry file — position zero, and what an isolate runs.
    Entry,
    /// A file some `require` site named, entered under that site's label.
    Required,
}

/// Lowers a file's own top-level statements into one synthesized function
/// — `rule:statements/storage-that-outlives-a-call`'s "the
/// script body is a function, so its variables are locals". `name` is the
/// label the listing/a future codegen symbol table uses; the caller picks
/// it, exactly as for [`lower_method`].
///
/// Everything that differs from [`lower_method`]:
///
/// - **No implicit receiver.** A script frame has no `$this`, so parameter
///   index 0 is not reserved and `params` is empty — matching
///   `nvs_types::check`'s own script frame, which seeds `$this` only when
///   there is an enclosing class.
/// - **The return representation is [`Ty::Tagged`], and `role` decides what
///   running out of statements hands back.** A top-level `return` hands a
///   value back to whatever entered the file, and
///   `rule:statements/require-is-the-only-inclusion-construct`
///   types that boundary `mixed`. Where the statements run out instead, the
///   seal is [`ScriptRole`]'s answer — the tagged `1` for a `require`, `null`
///   for the entry frame — and never the `Terminator::Return(None)`
///   `lower_method` uses, a `Ty::Tagged` frame owing its caller a value on
///   every exit.
///
/// Declarations are skipped rather than lowered: a class's methods are
/// lowered separately, one [`lower_method`] call each. A
/// `namespace X { ... }` block's *own* top-level statements are not skipped
/// — a namespace scopes names, not storage, so they belong to this same one
/// frame, which is exactly how `nvs_types::check::check_stmts` threads its
/// own [`Env`] through them.
pub fn lower_script(
    name: &str,
    stmts: &[Stmt],
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
    role: ScriptRole,
) -> Lowered {
    let ret_ty = Ty::Tagged;
    let mut low = Lowering::new(name, None, src, ret_ty, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    let mut env = Env::default();

    // The same reserved function-entry safepoint poll site `lower_method`
    // emits — a script body is a function, so it is a fixed site for the same
    // reason.
    low.emit_safepoint(entry);

    low.lower_script_stmts(stmts, &mut cur, &mut env);
    if !low.is_terminated(cur) {
        low.release_all_locals(cur, &env, None);
        // Whoever entered the frame decides what running out of statements
        // means, and the two ADRs disagree on purpose — see [`ScriptRole`].
        // Either way the seal is a value rather than the
        // `Terminator::Return(None)` `lower_method` uses: a `Ty::Tagged`
        // frame always hands a value back, and the one place that would
        // otherwise be free is exactly the one those ADRs pin.
        let (raw, raw_ty) = match role {
            ScriptRole::Required => (low.emit(cur, Ty::Int, InstKind::ConstInt(1)).0, Ty::Int),
            ScriptRole::Entry => (low.emit(cur, Ty::Null, InstKind::ConstNull).0, Ty::Null),
        };
        let sealed = low.coerce(cur, raw, raw_ty, Ty::Tagged, &mut env);
        low.seal(cur, Terminator::Return(Some(sealed)));
    }

    let pending = std::mem::take(&mut low.closures);
    // Beside the closures, and out the same channel: see `Lowering::callables`.
    let callables = std::mem::take(&mut low.callables);
    // Beside the closures, and out the same channel: see `Lowering::shapes`.
    let shapes = std::mem::take(&mut low.shapes);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, mut classes) =
        drain_closures(pending, callables, src, exprs, checked_types, enums);
    classes.extend(shapes);
    Lowered {
        function: Function {
            name: name.to_owned(),
            params: Vec::new(),
            ret: ret_ty,
            blocks,
            entry,
            stmt_spans,
            edge_spans,
        },
        closures,
        classes,
    }
}

/// Builds one function's basic blocks incrementally — see the module docs.
pub(crate) struct Lowering<'a> {
    ids: IdGen,
    src: &'a SourceFile,
    ret_ty: Ty,
    /// Where a call's/`new`'s resolved target is read back from — see
    /// [`ExprInfo`] and the crate docs' "design choices" section.
    exprs: &'a ExprTypeTable,
    /// The same `nvs_types::check_program` run's type interner — needed to
    /// translate a [`TypeId`] recorded in `exprs` into this crate's own
    /// [`Ty`] via [`erase_checked_ty`].
    checked_types: &'a TypeInterner,
    /// The same run's [`EnumTable`]: every declared enum's backing type and
    /// its cases' constant values.
    ///
    /// Threaded beside `checked_types` because a case's *value* is the one
    /// thing the checker's type does not carry —
    /// [`CheckedTy::EnumCase`](nvs_types::ty::Ty::EnumCase) names the enum and
    /// the case, and `rule:types/enum-case-type`'s membership test needs the integer that
    /// pair stands for. [`ExprInfo::EnumCase`] answers the same question, but
    /// only for a case written as an *expression*; a case named in a **type**
    /// has no expression to record one against.
    enums: &'a EnumTable,
    /// Parallel to `block_insts`/`block_terms`: the [`BlockId`] each was
    /// created with, in creation order. [`IdGen::next_block`] hands out ids
    /// sequentially from zero, so a block's id and its position in these
    /// vectors always coincide — that equality is what lets
    /// `is_terminated`/`seal`/`emit` index straight off `BlockId::index`
    /// rather than carrying a separate lookup table.
    block_ids: Vec<BlockId>,
    block_insts: Vec<Vec<Inst>>,
    block_terms: Vec<Option<Terminator>>,
    /// The stack of enclosing `while` loops currently being lowered,
    /// innermost last — see [`LoopFrame`]'s own doc comment for how
    /// [`Self::lower_while`]/[`Self::lower_break`]/[`Self::lower_continue`]
    /// use it.
    loop_stack: Vec<LoopFrame>,
    /// The stack of enclosing `try` regions currently being lowered, innermost
    /// last — see [`TryFrame`].
    try_stack: Vec<TryFrame<'a>>,
    /// Every refcounted value this frame owns that is **in flight inside an
    /// expression** — built here, with no other owner, and not yet released.
    ///
    /// The counterpart of [`Env`] for the half of a frame's references that
    /// has no name: a `Core` member's freshly materialized `string` argument
    /// lives here between [`Self::account_for_arg`] and
    /// [`Self::release_temporaries_since`], and nothing else can find it.
    /// [`Self::landing_block`] releases the whole stack on the error path,
    /// which is what makes the two edges agree — a throw abandons the
    /// expression, so every temporary it was holding is this frame's to drop
    /// whether control leaves the frame or lands in a `catch` inside it.
    ///
    /// Strictly stack-disciplined: a nested call completes, and drops its own
    /// entries, before the call around it accounts for its result. So a
    /// caller brackets its temporaries with [`Self::temporaries_mark`] and
    /// [`Self::release_temporaries_since`] rather than tracking values.
    ///
    /// A [`ArgOwnership::Transferred`] argument is here too, under the other
    /// [`TemporaryKind`]: the callee's own exit sweep releases it once the
    /// call is emitted, but the window between staging one and *reaching*
    /// that call belongs to this frame — `f($a, g())` retains `$a` for the
    /// transfer and then lets `g` throw. So a transferred entry is released
    /// on the error edge exactly like an owned one, and *forgotten* rather
    /// than released on the normal one, by the
    /// [`Self::forget_transferred_since`] each transferring call site runs
    /// immediately *before* it emits its call — the instruction from which
    /// the callee owns them on both of its own edges.
    owned_temporaries: Vec<(ValueId, TemporaryKind)>,
    /// The address half of an assignment target, already lowered, keyed by the
    /// span of the sub-expression that produced it — what
    /// [`Self::lower_read_modify_write`] splits out of the `$t = $t ⊕ e`
    /// rewrite so `Box::make()->count += 1` calls `make()` **once** where the
    /// rewrite reads its receiver twice.
    ///
    /// [`Self::lower_expr`] consults this before it looks at an expression's
    /// kind at all, so a staged sub-expression lowers to the value already in
    /// hand and runs nothing a second time. The implicit `1` an increment
    /// carries rides here too: it has no source span to build an
    /// [`ExprKind::Int`] from, so it is emitted at the target's own
    /// representation and staged under [`Self::synthetic_span`].
    ///
    /// An entry is a **borrow**, not an owner: whatever it names is either a
    /// durable slot's value or one this frame already put on
    /// [`Self::owned_temporaries`] for the length of the statement. That is
    /// why [`Self::aliasing_read`] answers `true` for a staged span — a
    /// consumer that wants to own the value retains it, and none of them
    /// releases it.
    ///
    /// Searched back to front and truncated rather than removed by key, so a
    /// nested read-modify-write (`$a[$i++] += 1`) sees its own innermost
    /// entry. Empty outside a statement's own target, and never more than a
    /// target's depth long.
    staged_targets: Vec<(Span, ValueId, Ty)>,
    /// The representation of every local this frame has **declared without an
    /// initializer** — `int $x;`, whose type `rule:types/var-inference` fixes at the
    /// declaration while its first value arrives on some later line.
    ///
    /// [`Env`] holds a name only once it has a value, so this is the only
    /// place a declaration with nothing to bind can put the one thing it does
    /// decide. Without it the *first* assignment would take its
    /// representation from the right-hand side instead, and `?string $s; $s =
    /// "x";` would bind a [`Ty::Str`] where every later narrowing expects the
    /// one tagged slot the declaration gave it.
    ///
    /// Read only where `Env` has no entry, so a bound local's own entry
    /// always wins, and never removed: Novis has no shadowing and a name is
    /// declared once per frame (`rule:types/declaration`), which is also why one flat map
    /// per frame is the whole scoping rule.
    declared_tys: FxHashMap<String, Ty>,
    /// This frame's late-static-binding class as a [`Ty::ClassDesc`] value,
    /// once something has asked for one — see [`Self::lsb`], which is the only
    /// thing that sets it after [`lower_method`] seeds a `static` method's
    /// parameter 0 here.
    lsb: Option<ValueId>,
    /// This frame's `$this`, for an *instance* method — the value
    /// [`Self::lsb`] loads the late-static-binding class out of. `None` for a
    /// static method and for the script frame.
    this: Option<ValueId>,
    /// The block [`Self::lsb`] appends its one load to. Always the function's
    /// entry block, so the value dominates every use no matter which block
    /// asked for it.
    entry: Option<BlockId>,
    /// This function's own `Class::method` label, for the backtrace frame
    /// [`Terminator::Propagate`] carries. The caller of [`lower_method`]/
    /// [`lower_script`] picks the spelling; this is the same string.
    fn_label: String,
    /// The `Class::member` enclosing every statement this frame lowers, or
    /// `None` for a script frame, whose [`Self::fn_label`] names a frame
    /// rather than a member. [`Self::source`] is what reads it.
    member: Option<String>,
    /// The span of the statement currently being lowered — the position half
    /// of that same backtrace frame.
    ///
    /// A cache of the span [`Self::lower_stmt`] just handed
    /// [`IdGen::next_stmt`], not a second table: `rule:testing/debug-probes`'s per-statement id
    /// already owns it, and [`Function::stmt_spans`] is where it ends up.
    cur_stmt_span: Span,
    /// How many `foreach` statements this frame has lowered so far — the
    /// suffix that keeps two nested loops' reserved [`Env`] names apart. See
    /// [`Self::lower_foreach`] for why those names exist and why the `#` in
    /// them cannot collide with a local.
    foreach_seq: u32,
    /// [`Self::foreach_seq`]'s counterpart for `switch`, which reserves one
    /// [`Env`] name of its own for the subject it compares against every case
    /// — see [`Self::lower_switch`].
    switch_seq: u32,
    /// Every `inout $x` parameter this frame declares, by name, mapped to the
    /// *pointee's* representation — the declared type its `Env` entry cannot
    /// carry, since that entry holds [`Ty::Ref`] instead (see [`Ty::Ref`] and
    /// [`lower_method`]).
    ///
    /// A side table rather than a payload on [`Ty::Ref`] because a [`Ty`] is
    /// `Copy` and one variant carrying another would end that; and a frame
    /// field rather than an `Env` one because it never changes after
    /// [`lower_method`] seeds it, so the `Env` clones a landing block and a
    /// loop header take would only copy it needlessly.
    ref_locals: FxHashMap<String, Ty>,
    /// Every `foreach (… as inout $v)` whose body is being lowered right now,
    /// innermost last — see [`InoutElement`] and [`Lowering::lower_foreach`].
    ///
    /// A stack rather than a map because two nested by-reference loops may be
    /// in scope at once and only the innermost binding of a name is visible;
    /// pushed before the body is lowered and popped after, so it is empty for
    /// every frame that has no such loop open and the write-through hook
    /// costs one `is_empty` there.
    inout_elements: Vec<InoutElement>,
    /// By-reference arguments staged for the calls currently being lowered,
    /// awaiting their copy-back — see [`Ty::Ref`] and
    /// [`Self::flush_ref_writebacks`].
    ///
    /// # Why this is a frame field rather than a return value
    ///
    /// The copy-back re-points the argument's *holder*, which for a local
    /// means rebinding it in [`Env`], and [`Self::lower_call_args`] hands its
    /// caller a `LoweredArgs` with nowhere to carry one. So the staging is
    /// parked here and drained by the call site itself, once, immediately
    /// after the call has returned — every argument of one call written back
    /// at one point rather than at whichever operand happened to be lowered
    /// last.
    ///
    /// **A stack, because calls nest.** `Foo::a($n, Bar::b($n))` stages `$n`
    /// for `a` before `b`'s argument list is lowered at all, so a call site
    /// takes [`Self::pending_refs_mark`] before it lowers its arguments and
    /// hands that mark back to [`Self::flush_ref_writebacks`] after it has
    /// emitted its call: the inner call writes back only what *it* staged, and
    /// the outer one's staging is still standing when its own call is emitted.
    ///
    /// That mark is the rule for *where* a copy-back lands, and it is where
    /// PHP puts it — at the call. So a read of the holder sequenced after the
    /// call and still inside the same statement
    /// (`Adder::bump($n) . " then " . $n`) sees the written-back value, and a
    /// call with an `inout $x` argument lowers in any expression position at all
    /// rather than only as a bare statement or a plain assignment's right-hand
    /// side. [`Self::lower_stmts`] asserts this list is empty once a statement
    /// has been lowered — an internal-consistency check on the call sites
    /// rather than a refusal of the program.
    ///
    /// **What that does not buy is PHP's operand order**, and it is not meant
    /// to. Novis evaluates a binary operator's operands strictly left to right,
    /// so `$n + Adder::bump($n)` reads the left `$n` *before* the call and
    /// answers `5 + 7`; PHP compiles that left operand to a CV read at the
    /// `ADD` itself, after the call, and answers `7 + 7`. PHP's own manual
    /// leaves the evaluation order of an expression's operands undefined, so
    /// there is no specified behaviour here to be compatible with, and
    /// left-to-right is the order every other side effect in an Novis expression
    /// already happens in.
    pending_refs: Vec<StagedRef>,
    /// `rule:iteration/generators`'s state class, while this frame is a generator's
    /// `advance()` — `None` for every other function there is. See
    /// [`lower_generator`], which owns the whole transform.
    generator: Option<GenFrame>,
    /// Every `rule:types/closure-literal` `fn` literal met in this body so far, in source order,
    /// each awaiting a function of its own — see [`lower_closure`]. Drained
    /// by whichever entry point built this frame, since a
    /// [`crate::ir::Function`] has nowhere to carry a second one.
    closures: Vec<PendingClosure>,
    /// Every `rule:types/callable-is-a-closure` first-class callable met in this body so far, in
    /// source order, each awaiting the forwarding thunk that gives it the one
    /// closure representation there is — see [`lower_callable`]. Travels out
    /// beside [`Self::closures`], for that field's reason.
    ///
    /// Not deduplicated: two sites naming the same member get two labels,
    /// because a `Function` is keyed on its label across the whole compiled
    /// unit and two files may each write `Foo::bar(...)`.
    callables: Vec<PendingCallable>,
    /// One synthesized class per distinct `rule:types/object-literal` shape literal this
    /// body writes — see [`Lowering::lower_object_literal`], which builds
    /// them, and [`shape_class_label`], which names them.
    ///
    /// Travels out beside `closures` for the same reason: a class is
    /// something only [`lower_file`] can hold, and an expression met in the
    /// middle of a body has nowhere else to put one. Deduplicated by label as
    /// it is filled, so a body writing `{x: 1}` in a loop records one entry.
    shapes: Vec<crate::ir::Class>,
}

/// One by-reference argument staged at a call site, and where its written-back
/// value has to land afterwards — see [`Lowering::pending_refs`].
struct StagedRef {
    /// What the copy-back re-points.
    holder: RefHolder,
    /// The [`Ty::Ref`] the callee was handed.
    slot: ValueId,
    /// The pointee's representation — the parameter's declared type, which
    /// `nvs_types`' `check_inout_arg` has already proven is exactly the
    /// holder's own.
    ty: Ty,
}

/// The storage a by-reference argument names, resolved at staging time rather
/// than carried as an AST reference.
///
/// Resolved eagerly for two reasons. A property's receiver must be evaluated
/// exactly once — staging reads the field and the copy-back writes it, and
/// re-lowering the receiver expression for the second would evaluate it twice.
/// And a [`Lowering`]'s one lifetime parameter is the source file's, not the
/// AST's, so parking a borrowed `Expr` in [`Lowering::pending_refs`] would
/// need a second one.
///
/// These are exactly the shapes [`is_aliasing_read`] recognises as durable
/// storage, and exactly the ones `nvs_types`' `check_inout_arg` accepts.
pub(crate) enum RefHolder {
    /// A bare local — the `Env` name it is bound under.
    Local(String),
    /// A compile-time-known property, with its receiver already lowered.
    Field {
        /// The receiver, evaluated once at staging time.
        object: ValueId,
        /// The declaring class's label.
        class: String,
        /// The property's own name.
        field: String,
    },
}

/// One open `foreach (… as inout $v)`: what a write to `$v` inside its body has to
/// write *through*.
///
/// The whole by-reference loop is this record plus
/// [`Lowering::write_through_element`], which is what
/// [`Lowering::lower_foreach`]'s own doc comment calls write-through: every
/// rebinding of `$v` also stores the new value into the entry `$v` came from,
/// rather than one copy-back at the end of the iteration. Both names are
/// [`Env`] names rather than values because the [`Env`] is what survives a
/// header phi, a landing block's clone and a generator's spill/reload — the
/// array's own binding is re-pointed by every write (`rule:types/arrays`'s
/// separation), so reading a stale [`ValueId`] here would write into the
/// array the loop *started* on.
struct InoutElement {
    /// The `$v` binding's name, as an assignment target names it.
    binding: String,
    /// The `Env` name the array being walked is bound under — the subject
    /// variable's own, since the loop takes no second reference to it.
    array: String,
    /// The `Env` name the current entry's cursor slot is bound under, a
    /// `Ty::Int` rebound at the top of every iteration.
    slot: String,
}

/// Which edge an entry on [`Lowering::owned_temporaries`] is released on.
///
/// The two differ on the **normal** edge only: both are this frame's to drop
/// while the expression is still in flight, and
/// [`Lowering::landing_block`] therefore sweeps the whole stack without
/// reading this at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TemporaryKind {
    /// Released on both edges — a value this frame built and no callee took:
    /// a `Core` member's materialized argument, a `.`'s partial result, a
    /// `match` subject. [`Lowering::release_temporaries_since`] is its normal
    /// edge.
    Owned,
    /// Released on the error edge only. The callee of an
    /// [`ArgOwnership::Transferred`] argument releases it from the moment the
    /// call is emitted, so releasing it here as well would be a double drop —
    /// [`Lowering::forget_transferred_since`] takes it off the stack instead.
    Transferred,
}

/// What [`Lowering::lower_call_args`] produced: the values to pass.
///
/// What this frame still *owes a release for* is deliberately not here. A
/// temporary has to be findable from the moment it exists — a later argument's
/// call can throw before this struct is ever returned — so
/// [`Lowering::account_for_arg`] pushes it onto
/// [`Lowering::owned_temporaries`] instead, and the caller brackets the whole
/// call with [`Lowering::temporaries_mark`] and
/// [`Lowering::release_temporaries_since`].
#[derive(Default)]
pub(crate) struct LoweredArgs {
    /// The argument values, positional.
    values: Vec<ValueId>,
}

/// One resolved signature's argument-shape, as [`Lowering::lower_call_args`]
/// needs it — owned rather than borrowed because every call site has to clone
/// it out of `self.exprs` before touching `self` mutably anyway.
pub(crate) struct ArgSig {
    /// Each parameter's declared type, positional.
    param_tys: Vec<TypeId>,
    /// Which parameters are declared `inout $x`, positional.
    inout: Vec<bool>,
    /// Whether the last parameter is `...$x` — in which case `param_tys`'
    /// last entry is the type *each* trailing argument is checked against,
    /// and `Lowering::lower_variadic_tail` turns all of them into the one
    /// array that parameter actually receives.
    variadic: bool,
    /// Each parameter's evaluated default, positional — `None` for one every
    /// call has to supply. See [`Lowering::lower_call_args`] for what an
    /// omitted trailing argument becomes.
    defaults: Vec<Option<nvs_types::ConstArg>>,
    /// Which parameter each **written** argument fills, in written order —
    /// one entry per `nvs_syntax::ast::Arg`, copied off
    /// [`nvs_types::expr_table::ResolvedCall::arg_slots`].
    ///
    /// This crate cannot re-derive it and the field exists for that reason
    /// alone: a `name:` resolves against `MethodSig::param_names`, which no
    /// pass below the checker holds. For an all-positional list it is the
    /// identity mapping and says nothing new, which is why
    /// [`Lowering::lower_call_args`] reads it uniformly rather than branching
    /// on whether the call wrote a name at all.
    arg_slots: Vec<ArgSlot>,
    /// Whether the callee is a Tier 0 `Core` member reached through the ADR
    /// 0002 helper convention, rather than a compiled Novis function.
    ///
    /// One thing turns on it, and it is a real difference rather than a
    /// convenience: a helper's parameter slot is a whole `nvs_runtime::Value`
    /// — a tag plus a payload — and `nvs-codegen`'s `emit_helper` writes each
    /// argument's tag from the *argument's* own representation, never from the
    /// parameter's. So a `Core` parameter whose declared type has no single IR
    /// representation (a union — `hasKey(array<T> $a, int|string $key)`) is
    /// still perfectly lowerable: the argument keeps its own type and the
    /// helper decodes by tag. A compiled Novis function's parameter slot is
    /// typed, so the same declaration there is not lowerable at all, and
    /// [`Lowering::lower_call_args`] keeps panicking for it.
    ///
    /// The asymmetry is only in *parameter* position. A `Core` member that
    /// *returned* a union would need the caller to hold a value of a
    /// representation [`Ty::Tagged`]'s own doc comment records as still open,
    /// so the registry has none.
    helper: bool,
}

impl ArgSig {
    /// The shape of a resolved call's own signature, called through the
    /// ordinary Novis call convention.
    fn of(call: &nvs_types::expr_table::ResolvedCall) -> Self {
        Self {
            param_tys: call.param_tys.clone(),
            inout: call.inout.clone(),
            variadic: call.variadic,
            defaults: call.defaults.clone(),
            arg_slots: call.arg_slots.clone(),
            helper: false,
        }
    }

    /// The same shape, for a callee reached through the helper convention —
    /// see [`Self::helper`].
    fn of_helper(call: &nvs_types::expr_table::ResolvedCall) -> Self {
        Self {
            helper: true,
            ..Self::of(call)
        }
    }

    /// The IR type to lower the argument at `index` against, or `None` when
    /// the parameter has no single representation *and* the callee can take
    /// the argument at whatever representation it arrives in — which is a
    /// helper and only a helper (see [`Self::helper`]).
    ///
    /// # Panics
    ///
    /// Panics through [`erase_checked_ty`] for a `Core` options bag, which is
    /// never one argument and never reaches here: [`Lowering::lower_fixed_arg`]
    /// reads that parameter's checked type and flattens it a slot at a time
    /// before asking for an expectation at all.
    fn expectation(&self, index: usize, checked_types: &TypeInterner) -> Option<Ty> {
        let pty = self.param_tys[index];
        if self.helper && matches!(checked_types.get(pty), CheckedTy::Union(_)) {
            return None;
        }
        Some(erase_checked_ty(pty, checked_types))
    }

    /// Whether the argument at `index` binds by reference. Never true past the
    /// recorded parameters, and never consulted for a variadic tail at all:
    /// `lower_call_args` collects that tail into one array, which is a value
    /// and not a holder, so there is no position-onward rule to apply here the
    /// way `nvs_types::signatures::MethodSig::is_inout` has one. `rule:core-api/shape-rules` R7
    /// keeps it that way for `Core` — nothing there is by-reference.
    fn is_inout(&self, index: usize) -> bool {
        self.inout.get(index).copied().unwrap_or(false)
    }
}

impl<'a> Lowering<'a> {
    /// A frame's lowering state. `name` is the backtrace frame's label and
    /// `member` the `Class::member` enclosing its statements — the same string
    /// for a method, and `None` against a frame name for a script.
    pub(crate) fn new(
        name: &str,
        member: Option<&str>,
        src: &'a SourceFile,
        ret_ty: Ty,
        exprs: &'a ExprTypeTable,
        checked_types: &'a TypeInterner,
        enums: &'a EnumTable,
    ) -> Self {
        Self {
            ids: IdGen::new(),
            exprs,
            checked_types,
            enums,
            src,
            ret_ty,
            block_ids: Vec::new(),
            block_insts: Vec::new(),
            block_terms: Vec::new(),
            loop_stack: Vec::new(),
            try_stack: Vec::new(),
            owned_temporaries: Vec::new(),
            staged_targets: Vec::new(),
            declared_tys: FxHashMap::default(),
            lsb: None,
            this: None,
            entry: None,
            fn_label: name.to_owned(),
            member: member.map(str::to_owned),
            cur_stmt_span: Span::at(src.id(), 0),
            foreach_seq: 0,
            switch_seq: 0,
            ref_locals: FxHashMap::default(),
            inout_elements: Vec::new(),
            pending_refs: Vec::new(),
            generator: None,
            closures: Vec::new(),
            callables: Vec::new(),
            shapes: Vec::new(),
        }
    }
    /// Records the synthesized class a shape literal named — see
    /// [`Self::shapes`].
    ///
    /// `reprs` is what each field's own initializer lowered to, in the same
    /// sorted order as `fields`, and it is the whole of what
    /// `nvs_runtime::nvs_object_slot_set` gets to check a write against.
    ///
    /// A label already recorded is **merged**, not skipped: `$shape{x}` is
    /// named for its field names alone (see [`shape_class_label`]), so
    /// `{x: 1}` and `{x: "s"}` are one class with two disagreeing slot types.
    /// A slot the two spell differently degrades to [`Ty::Tagged`], which
    /// `nvs-codegen` reads as "no fixed tag, do not check" — picking whichever
    /// literal was seen first would instead reject the other one's own writes.
    pub(crate) fn record_shape_class(
        &mut self,
        label: String,
        fields: Vec<String>,
        reprs: Vec<Ty>,
    ) {
        let field_count = fields.len();
        if let Some(class) = self.shapes.iter_mut().find(|class| class.label == label) {
            for (have, found) in class.field_reprs.iter_mut().zip(&reprs) {
                if *have != *found {
                    *have = Ty::Tagged;
                }
            }
            return;
        }
        self.shapes.push(crate::ir::Class {
            label,
            fields,
            field_reprs: reprs,
            // A shape literal's field type is *inferred* from its initializer
            // rather than declared, and `rule:security/secret-qualifier` puts the qualifier on a
            // declaration — so no slot here is `secret`, and a `{token:
            // $secret}` literal is `nvs_stdlib::debug`'s own known gap rather
            // than a bit this could set.
            secret_fields: vec![false; field_count],
            // `rule:types/object-literal` gives a shape literal no visibility keyword to write
            // and no class to be private to: every slot was written by the
            // literal that built it and every one is readable, which is the one
            // answer `Core\Reflect`'s walk can give a shape — and none is
            // `protected`, there being no class for one to be protected from.
            public_fields: vec![true; field_count],
            protected_fields: vec![false; field_count],
            // Empty rather than one entry per slot, which is `ir::Class`'s
            // "nothing told this class": a shape literal declares no type to
            // spell, its slots being typed by what was written into them.
            field_types: Vec::new(),
            // A shape literal has no declaration body, so it declares no
            // constant and carries no attach site — empty here is the fact and
            // not an omission.
            constants: Vec::new(),
            attributes: Vec::new(),
            // `rule:types/object-literal`: a shape literal's class has no methods, no
            // supertypes and no `implements`, it carries no attribute, and
            // every one of its slots is written by the literal that built it
            // — so there is nothing for a codec, a constructor arity or a
            // declared default to say.
            conforms: Vec::new(),
            methods: Vec::new(),
            hooks: Vec::new(),
            codec: Vec::new(),
            db_codec: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
            is_closure: false,
        });
    }
    /// The synthesized class a `Core` member answering a shape returns,
    /// recorded with the **result** shape's per-slot representations.
    ///
    /// `rule:concurrency/all-answers-a-typed-shape`'s `Core\Task::all` is why
    /// this exists. Its result carries the argument literal's own descriptor,
    /// because a shape class is named for its field names alone
    /// ([`shape_class_label`]) and both sides spell the same ones — but the
    /// literal's slots each hold a *closure*, so a class recorded from it
    /// alone promises [`Ty::Object`] on every slot and refuses the awaited
    /// `int` the field is declared to answer with. Recording the result's own
    /// representations against that same label **merges**
    /// ([`Self::record_shape_class`]): a slot the two spell differently
    /// degrades to [`Ty::Tagged`], which is "no fixed tag, do not check", and
    /// one they agree on keeps its tag. `nvs_runtime::object`'s module doc
    /// § *What a shape write checks* owns the mechanism, as case 4.
    ///
    /// A member answering anything else records nothing, so the ordinary
    /// `Core` call path pays one interner read.
    pub(crate) fn record_core_result_shape(&mut self, return_ty: TypeId) {
        // Copied out first: the fields below borrow the interner for as long
        // as they are read, and `record_shape_class` needs `self`.
        let checked_types = self.checked_types;
        let CheckedTy::Shape(fields) = checked_types.get(return_ty) else {
            return;
        };
        // `nvs_types::ty::TypeInterner::shape` interns a shape's fields in
        // sorted name order, which is the order the class's slots count
        // through — [`Self::lower_object_literal`] owns why the two sides have
        // to agree.
        let names: Vec<String> = fields.iter().map(|field| field.name.clone()).collect();
        let reprs: Vec<Ty> = fields
            .iter()
            .map(|field| erase_checked_ty(field.ty, checked_types))
            .collect();
        self.record_shape_class(shape_class_label(&names), names, reprs);
    }
    pub(crate) fn new_block(&mut self) -> BlockId {
        let id = self.ids.next_block();
        self.block_ids.push(id);
        self.block_insts.push(Vec::new());
        self.block_terms.push(None);
        if self.entry.is_none() {
            self.entry = Some(id);
        }
        id
    }
    /// This frame's late-static-binding class, as a [`Ty::ClassDesc`].
    ///
    /// A `static` method already has it in parameter 0 ([`lower_method`]). An
    /// *instance* method loads it from `$this` — one load at
    /// `nvs_runtime::OBJ_CLASS_OFFSET`, emitted into the **entry block** and
    /// cached, so it dominates every block that could ask and so a method
    /// whose body never says `static` pays nothing at all.
    ///
    /// Appending to the entry block after other blocks exist is safe by
    /// construction: a block's terminator is a separate field from its
    /// instruction list, so a later append still lands before the entry
    /// block's own terminator, and the only operand is parameter 0.
    ///
    /// # Panics
    ///
    /// Panics for a frame with neither — the script frame. `nvs_types` refuses
    /// `static::`/`new static()` outside a class (`E_UNDEFINED_CLASS`), so
    /// lowering never reaches this on one.
    pub(crate) fn lsb(&mut self) -> ValueId {
        if let Some(v) = self.lsb {
            return v;
        }
        let this = self.this.unwrap_or_else(|| {
            panic!(
                "nvs-ir: `{}` names `static` but has neither a receiver nor a called class — \
                 nvs_types is expected to have refused that outside a class",
                self.fn_label
            )
        });
        let entry = self
            .entry
            .expect("a frame asking for its called class has at least one block");
        let (v, _) = self.emit(entry, Ty::ClassDesc, InstKind::ClassDescOf { object: this });
        self.lsb = Some(v);
        v
    }
    pub(crate) fn is_terminated(&self, b: BlockId) -> bool {
        self.block_terms[b.index() as usize].is_some()
    }
    /// Gives `b` its terminator, exactly once.
    ///
    /// # Panics
    ///
    /// Panics if `b` already has one — every call site only ever seals a
    /// block it just confirmed (via [`Self::is_terminated`]) is still open.
    pub(crate) fn seal(&mut self, b: BlockId, term: Terminator) {
        let slot = &mut self.block_terms[b.index() as usize];
        assert!(
            slot.is_none(),
            "nvs-ir: bb{} sealed twice — bug in control-flow lowering",
            b.index()
        );
        *slot = Some(term);
    }
    /// The three constants a call on
    /// `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`' roster carries ahead of
    /// its receiver: the descriptor of what its type argument named, whether
    /// that was written as an `array<...>` of one, and — where it named an
    /// inline shape — the wire contract that shape's descriptor cannot hold.
    /// A written class emits the third slot as
    /// [`InstKind::ShapeCodecConst`]'s `None`, whose own docs say why the slot
    /// is there either way.
    ///
    /// Emitted from one place for [`written_class_label`]'s reason: three call
    /// paths reach this ABI and none of them may spell it differently. They
    /// emit it *before* the receiver is opened, so a `?->` guard's branch
    /// cannot come between a constant and its use; none of the three is
    /// refcounted, so none is retained or released.
    ///
    /// A shape also **registers its class here**, because a unit that hydrates
    /// a `{n: int}` it never spells has no shape literal to synthesize one and
    /// the descriptor constant above would resolve to nothing. Every slot it
    /// claims is [`Ty::Tagged`]: what fills them is a native decoder rather
    /// than a lowered write, so this registration has no tag to promise, and
    /// where a literal of the same field names also runs in this unit
    /// [`Self::record_shape_class`]'s merge lands on that same answer.
    ///
    /// # Panics
    ///
    /// Panics naming the member when the checker recorded a written shape but
    /// filed no contract under its span, which is one run's two halves
    /// disagreeing rather than anything a program wrote.
    pub(crate) fn written_type_constants(
        &mut self,
        b: BlockId,
        call: &nvs_types::expr_table::ResolvedCall,
    ) -> [ValueId; 3] {
        let class = written_class_label(call);
        // Copied out of the field so the table's borrow is the checker run's
        // and not this frame's: `record_shape_class` below needs `self` back.
        let exprs = self.exprs;
        let contract = call.written_shape.as_ref().map(|shape| {
            let codec = exprs.shape_codec(shape.codec).unwrap_or_else(|| {
                panic!(
                    "nvs-ir: `{}::{}` was written with the inline shape `{}`, and nvs_types \
                     filed no wire contract under its type argument's span — did this program \
                     pass nvs_types::check_program with the same table?",
                    call.class, call.method, shape.label
                )
            });
            let fields = shape_codec_fields(codec);
            let names: Vec<String> = codec
                .fields
                .iter()
                .map(|field| field.property.clone())
                .collect();
            let reprs = vec![Ty::Tagged; names.len()];
            self.record_shape_class(shape.label.clone(), names, reprs);
            shape_codec_key(&fields)
        });
        let (desc, _) = self.emit(b, Ty::ClassDesc, InstKind::ClassDescConst { class });
        let (list, _) = self.emit(b, Ty::Bool, InstKind::ConstBool(call.written_class_is_list));
        let (codec, _) = self.emit(
            b,
            Ty::ClassDesc,
            InstKind::ShapeCodecConst { shape: contract },
        );
        [desc, list, codec]
    }

    /// The constant a member on `nvs_stdlib::registry::SOURCE_MEMBERS`
    /// takes as its argument 0: where this call site is, as
    /// [`InstKind::SourceConst`] carries it.
    ///
    /// Ahead of everything the call itself wrote, receiver included, for the
    /// reason that roster's twin gives — what a producer is handed here is a
    /// constant *of* the call rather than a value in it, and one position for
    /// every producer beats a position that moves. `dump` is variadic, so a
    /// trailing slot would be a slot no reader of `args` can point at.
    ///
    /// Not refcounted, so nothing retains or releases it, and emitted before
    /// the receiver is opened so that a `?->` guard's branch cannot come
    /// between a constant and its use — both for [`Self::written_type_constants`]'s
    /// reasons exactly.
    pub(crate) fn producer_source(&mut self, b: BlockId) -> ValueId {
        let source = Some(self.source());
        self.emit(b, Ty::ClassDesc, InstKind::SourceConst { source })
            .0
    }

    /// The constant a member on `nvs_stdlib::registry::CALL_SITE_MEMBERS` takes
    /// as its **last** argument: the class this frame's statements are inside,
    /// carried as the enclosing `Class::member` half of the same
    /// [`InstKind::SourceConst`] a producer is handed.
    ///
    /// One instruction for both rosters rather than a second constant carrying
    /// the class alone, because the datum is already in this one and a helper
    /// reading it goes through `nvs_runtime::source::of_operand` either way. A
    /// descriptor address would be smaller and is what
    /// [`Self::written_type_constants`] hands over — but a *call site* is not a
    /// class reference: a script frame has none at all, and a class the unit
    /// declares no descriptor for would be a compile refusal where the answer
    /// wanted is "no class".
    ///
    /// Emitted before the receiver is opened, and not refcounted, for
    /// [`Self::producer_source`]'s reasons exactly.
    pub(crate) fn call_site(&mut self, b: BlockId) -> ValueId {
        self.producer_source(b)
    }

    /// The constant a member on `nvs_stdlib::registry::PREPARED_MEMBERS` takes
    /// as its argument 0: what the checker prepared out of the literal the call
    /// at `span` was written with, as [`InstKind::PreparedConst`] carries it.
    ///
    /// The whole of this crate's half of that channel. `nvs_types` files what
    /// it prepared under the **call's** span — which is the only address this
    /// frame holds — and the fact is read out of the same table every other
    /// resolved fact about the call comes from, so a checking run and a lowering
    /// run cannot disagree about which pattern was prepared.
    ///
    /// A call the fold did not read emits the zero word rather than no
    /// instruction: the slot is part of the member's ABI, and a path that
    /// skipped it would leave the helper reading past its own arguments.
    /// Emitted before the receiver is opened and not refcounted, for
    /// [`Self::producer_source`]'s reasons exactly.
    pub(crate) fn prepared_constant(&mut self, b: BlockId, span: nvs_diagnostics::Span) -> ValueId {
        let fact = self
            .exprs
            .prepared(span)
            .map(|prepared| match prepared.fact {
                nvs_types::expr_table::PreparedFact::RegexTier(nvs_types::RegexTier::Linear) => {
                    Prepared::RegexLinear
                }
                nvs_types::expr_table::PreparedFact::RegexTier(
                    nvs_types::RegexTier::Backtracking,
                ) => Prepared::RegexBacktracking,
                nvs_types::expr_table::PreparedFact::CldrPattern => Prepared::CldrPattern,
            });
        self.emit(b, Ty::Int, InstKind::PreparedConst { fact }).0
    }
    pub(crate) fn emit(&mut self, b: BlockId, ty: Ty, kind: InstKind) -> (ValueId, Ty) {
        let v = self.ids.next_value();
        self.block_insts[b.index() as usize].push(Inst {
            result: Some(v),
            ty: Some(ty),
            kind,
            on_error: None,
            raise_site: None,
        });
        (v, ty)
    }
    /// [`Self::emit`] for a call-shaped instruction: the same append, plus
    /// `rule:errors/propagation`'s error edge to
    /// a landing block built for this exact program point.
    ///
    /// Every instruction that returns a status goes through here, and nothing
    /// else does — see [`Inst::on_error`]'s own doc comment for that split.
    pub(crate) fn emit_fallible(
        &mut self,
        b: BlockId,
        ty: Ty,
        kind: InstKind,
        env: &Env,
    ) -> (ValueId, Ty) {
        let landing = self.landing_block(env);
        let v = self.ids.next_value();
        self.block_insts[b.index() as usize].push(Inst {
            result: Some(v),
            ty: Some(ty),
            kind,
            on_error: Some(landing),
            raise_site: None,
        });
        (v, ty)
    }
    /// [`Self::emit_fallible`] for an instruction `nvs-codegen` raises
    /// **inline** rather than through a callee: the same error edge, and
    /// beside it the site that raise renders itself from
    /// ([`crate::ir::Inst::raise_site`]).
    ///
    /// The checked arithmetic rows are the whole of what comes through here,
    /// and the site is [`Self::source`] — the very datum a `throw` in this
    /// statement would be compiled with, so whichever of the two raises, the
    /// exception names the same place
    /// (`rule:errors/a-record-names-where-it-was-produced`).
    pub(crate) fn emit_raising(
        &mut self,
        b: BlockId,
        ty: Ty,
        kind: InstKind,
        env: &Env,
    ) -> (ValueId, Ty) {
        let landing = self.landing_block(env);
        let site = self.source();
        let v = self.ids.next_value();
        self.block_insts[b.index() as usize].push(Inst {
            result: Some(v),
            ty: Some(ty),
            kind,
            on_error: Some(landing),
            raise_site: Some(site),
        });
        (v, ty)
    }
    /// A fresh landing block for a failure raised at this program point: what
    /// this frame owes on the error path, and where control goes next.
    ///
    /// One block per call site rather than one shared per region, because a
    /// `catch` handler's phis need a distinct predecessor per site — see
    /// [`Inst::on_error`].
    ///
    /// The two exits release different things, and the asymmetry is the whole
    /// point:
    ///
    /// * **Propagating** out of the frame, every refcounted local still live
    ///   here is dropped, exactly the sweep [`Self::release_all_locals`]
    ///   already performs at an ordinary `return`. Without it a throw would
    ///   leak every one of them.
    /// * **Reaching a `catch` in this same frame** releases nothing the
    ///   handler still names: it and everything after it name those locals,
    ///   and the binding each one has on the exception path travels through
    ///   this site's entry in [`TryFrame::edges`] into the handler's phis.
    ///   That exit is taken on a `THROWN` alone, so it comes with a second
    ///   block for every other non-`OK` status — [`Terminator::Catch`]'s
    ///   `onward` — which performs exactly the sweep the propagating exit
    ///   above does, since such a status leaves the frame without the handler
    ///   ever naming a thing.
    ///
    /// **The catchable exit is a block of its own**, and that block rather
    /// than this one is what goes on [`TryFrame::edges`]. A landing site is
    /// the one place two exits leave through a single block, so anything
    /// written into it lands on both — and [`Self::merge_envs`] writes into
    /// its incoming blocks. See the body for the double release that avoids.
    ///
    /// That block is also where [`InstKind::SeedRaiseSite`] goes, carrying
    /// [`Self::source`] in [`Inst::raise_site`]: reaching a handler in this
    /// frame is the one exception path that pushes no frame label, so it is
    /// the one place a failure raised with no site of its own — a helper's
    /// fault — would be caught naming nowhere.
    ///
    /// [`Self::owned_temporaries`] is released on **both** exits, ahead of
    /// either, and that is the one thing the asymmetry does not reach: a
    /// temporary has no `Env` entry for a handler to find it through, so the
    /// exception path is the last place anything can drop it. The values it
    /// holds are defined in the block that raised, which dominates this one,
    /// so the releases need no phi of their own.
    ///
    /// The sweep covers what a producer actually **staged**, and reads no
    /// [`TemporaryKind`]: a transferred argument is as much this frame's to
    /// drop on the way to a call it never reached as an owned one is.
    pub(crate) fn landing_block(&mut self, env: &Env) -> BlockId {
        let b = self.new_block();
        for (v, _) in self.owned_temporaries.clone() {
            self.emit_release(b, v);
        }
        // The innermost frame that actually has a handler — not simply the
        // innermost frame. See [`TryFrame::handler`] for the one shape that
        // sits on the stack without being a destination.
        let innermost = self
            .try_stack
            .iter()
            .rposition(|frame| frame.handler.is_some());
        match innermost.map(|at| (at, self.try_stack[at].handler)) {
            Some((at, Some(handler))) => {
                // The catchable exit gets a block of its own, and it is that
                // one — never `b` — the region records as an incoming edge.
                // `Self::merge_envs` writes into an incoming block
                // (`Self::release_merged_away`), and what it writes there is
                // owed on the way *into* the handler alone: a name the
                // dispatch drops is still swept by `onward` below on the way
                // out of the frame. Recording `b` would put that release ahead
                // of a terminator both exits leave through, so a `FATAL` raised
                // under a `try` holding any conditionally-bound refcounted
                // name — a `foreach`'s own reserved binding is the one every
                // program has — would release it twice.
                let caught = self.new_block();
                // The one edge on which nothing ever names where the failure
                // happened: no frame is unwound out of a throw caught in its
                // own frame, and a helper's fault renders no site of its own.
                // The statement whose call failed is that site, and the
                // instruction writes only where nothing else already did —
                // `InstKind::SeedRaiseSite` owns the rest.
                let site = self.source();
                self.block_insts[caught.index() as usize].push(Inst {
                    result: None,
                    ty: None,
                    kind: InstKind::SeedRaiseSite,
                    on_error: None,
                    raise_site: Some(site),
                });
                self.seal(caught, Terminator::Jump(handler));
                self.try_stack[at].edges.push((caught, env.clone()));
                // The uncatchable-status exit, and the one place this frame's
                // locals are dropped on a path that neither returns nor enters
                // a handler — see `Terminator::Catch`'s own doc comment.
                let onward = self.new_block();
                self.release_all_locals(onward, env, None);
                let frame = self.frame_label();
                self.seal(onward, Terminator::Propagate { frame });
                self.seal(
                    b,
                    Terminator::Catch {
                        handler: caught,
                        onward,
                    },
                );
            }
            Some((_, None)) => unreachable!("rposition only matches a frame with a handler"),
            None => {
                self.release_all_locals(b, env, None);
                let frame = self.frame_label();
                self.seal(b, Terminator::Propagate { frame });
            }
        }
        b
    }
    /// This frame's backtrace label, `Class::method() at <file>:<line>`.
    ///
    /// `<file>` is the source's name exactly as it was opened — the path as
    /// given on the command line — so the rendered trace is byte-for-byte
    /// identical on every platform. `<line>` is the enclosing statement's,
    /// from the span `rule:testing/debug-probes`'s per-statement id already carries.
    ///
    /// Both halves of that position come from [`Self::source`], so the record
    /// `rule:errors/a-record-names-where-it-was-produced` asks a producer for
    /// and this string cannot disagree about where a statement is.
    ///
    /// **The rendered string is a backtrace spelling, and no record is built
    /// from it.** An envelope carries content rather than presentation
    /// (`rule:errors/diagnostic-record`), so a reader wanting the file alone
    /// would have to parse it back apart. Its leading half is a *frame* name
    /// rather than a member — a script frame's is `script` or
    /// [`file_script_label`]'s `file#<id>$script`, exactly where
    /// [`Self::member`] is `None`. And it exists only where a landing block
    /// does, reaching compiled code through [`Terminator::Propagate`] alone,
    /// so a producer called from a statement that needs no error path has no
    /// constant here at all and gets one emitted at its own call.
    pub(crate) fn frame_label(&self) -> String {
        let at = self.source();
        format!("{}() at {}:{}", self.fn_label, at.file, at.line)
    }
    /// Where the statement being lowered is, in
    /// `rule:errors/a-record-names-where-it-was-produced`'s three parts: the
    /// file as the program named it, its one-based line, and the enclosing
    /// `Class::member` where there is one.
    ///
    /// **This is the one derivation of that datum**, and both readers the rule
    /// names come off it — a record's envelope, through the constant emitted at
    /// the producer's own call, and [`Self::frame_label`], by rendering it. Two
    /// spellings that agree today is the thing the rule exists to rule out, so
    /// nothing else builds one.
    ///
    /// No second position table is read for it: the line is the one the span
    /// `rule:testing/debug-probes`'s per-statement id already carries, and the
    /// member is [`Self::member`], which a script frame has none of.
    pub(crate) fn source(&self) -> Source {
        let (line, _) = self.src.line_col(self.cur_stmt_span.start);
        Source {
            file: self.src.name().to_owned(),
            line: u32::try_from(line + 1).expect("far more lines than a source file can hold"),
            member: self.member.clone(),
        }
    }
    /// Appends a reserved [`InstKind::Safepoint`] marker to `b` — see that
    /// variant's own doc comment for the call sites this has (function entry,
    /// a loop's back edge) and why it defines no value.
    pub(crate) fn emit_safepoint(&mut self, b: BlockId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Safepoint,
            on_error: None,
            raise_site: None,
        });
    }
    /// Appends an [`InstKind::Retain`] on `v` to `b` — see the module docs'
    /// refcounting-policy section for when a call site actually wants one;
    /// this just emits the instruction unconditionally, since every caller
    /// has already checked [`Ty::is_refcounted`] itself.
    pub(crate) fn emit_retain(&mut self, b: BlockId, v: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Retain { operand: v },
            on_error: None,
            raise_site: None,
        });
    }
    /// Appends an [`InstKind::Release`] on `v` to `b` — see [`Self::emit_retain`].
    pub(crate) fn emit_release(&mut self, b: BlockId, v: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Release { operand: v },
            on_error: None,
            raise_site: None,
        });
    }
    /// Reconciles the representation an expression produced with the one the
    /// position it lands in declares — the **one** place
    /// [`InstKind::Tag`]/[`InstKind::Untag`] are emitted.
    ///
    /// The two representations can differ for exactly one reason: the checker
    /// accepted an assignment, an argument or a `return` whose declared type
    /// is wider than the value's own. That is [`Ty::Tagged`] for every wider
    /// type but one (see its own doc comment), so most of this is widening
    /// into it or narrowing back out of it; any mismatch not in the table
    /// below is a bug in this crate rather than a conversion, and is left
    /// alone for the instruction that consumes it to reject.
    ///
    /// The exception is `rule:types/conversion`'s implicit `int`/`uint` → `float`
    /// widening, whose target is a machine representation rather than a tag,
    /// and which **can fail** — so unlike the two tag instructions it is a
    /// helper call carrying an error edge, and it is why this takes `env`.
    ///
    /// Ownership is unchanged in every direction: `Tag`/`Untag` transfer the
    /// operand's reference to their result and the numeric rows have no
    /// reference to transfer, so no caller needs a retain or a release around
    /// one, and the [`is_aliasing_read`]-keyed retain every boundary already
    /// emits still applies exactly once — to whichever of the two values that
    /// boundary ends up storing.
    pub(crate) fn coerce(
        &mut self,
        cur: BlockId,
        v: ValueId,
        from: Ty,
        to: Ty,
        env: &mut Env,
    ) -> ValueId {
        match (from, to) {
            (a, b) if a == b => v,
            // `rule:types/conversion`'s one implicit conversion: "`int` or `uint`
            // widening into a `float` position", which is exactly what a
            // declared type wider than the value's own is here. It is the
            // *same* conversion `$n as float` performs — exact, or throwing
            // above 2^53 where an `f64` stops representing every integer —
            // and § 2 says so outright, so it is the same fallible helper and
            // not a `fcvt_from_sint` this function could emit on its own.
            //
            // That is why this one takes `env`, and why every caller passes
            // it: a conversion carrying `rule:errors/propagation`'s error edge needs the
            // frame's landing block, which only the environment names.
            (Ty::Int | Ty::Uint, Ty::Float) => {
                let helper = if from == Ty::Int {
                    Helper::IntToFloat
                } else {
                    Helper::UintToFloat
                };
                self.emit_fallible(
                    cur,
                    Ty::Float,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                    env,
                )
                .0
            }
            // The union `rule:types/arithmetic` gives integer `/` reaches a declared
            // `float` the same way, but its representation is already
            // [`Ty::Tagged`] rather than an integer one, so the row is the
            // one that reads the runtime tag. `nvs_types` is what decided the
            // position accepts it (`expr::assign::is_assignable`); this only
            // performs it.
            (Ty::Tagged, Ty::Float) => {
                self.emit_fallible(
                    cur,
                    Ty::Float,
                    InstKind::HelperCall {
                        helper: Helper::TaggedToFloat,
                        args: vec![v],
                    },
                    env,
                )
                .0
            }
            // `rule:types/class-reference`'s `?class<T>`: `null` into a class-reference
            // position is the *null descriptor*, which is the zero word
            // relabelled — see [`Ty::ClassDesc`] for why that representation
            // holds `null` at all, and [`InstKind::Reinterpret`] for why a
            // relabelling is free. Ahead of the tagging rows below only in
            // reading order; neither target overlaps.
            (Ty::Null, Ty::ClassDesc) => {
                let (zero, _) = self.emit(cur, Ty::Int, InstKind::ConstInt(0));
                self.emit(cur, Ty::ClassDesc, InstKind::Reinterpret { operand: zero })
                    .0
            }
            (_, Ty::Tagged) => self.emit(cur, Ty::Tagged, InstKind::Tag { operand: v }).0,
            (Ty::Tagged, _) => self.emit(cur, to, InstKind::Untag { operand: v }).0,
            _ => v,
        }
    }
    /// The machine word behind a [`Ty::ClassDesc`] operand, as a [`Ty::Int`] a
    /// `BinOp` has a row for — and the zero word for the literal `null`, which
    /// is the same value a missed [`InstKind::ClassDescIn`] answers with.
    ///
    /// The one question every site asking "is this `?class<T>` null?" reduces
    /// to. [`Ty::ClassDesc`]'s own doc comment lists those sites and owns why
    /// they cannot read `Ty::Tagged` off the operand instead.
    pub(crate) fn class_desc_word(&mut self, v: ValueId, ty: Ty, cur: BlockId) -> ValueId {
        match ty {
            Ty::Null => self.emit(cur, Ty::Int, InstKind::ConstInt(0)).0,
            _ => {
                self.emit(cur, Ty::Int, InstKind::Reinterpret { operand: v })
                    .0
            }
        }
    }
    /// Appends an [`InstKind::RefStore`] to `b` — see that variant's own doc
    /// comment for the release-the-old half it performs itself, and
    /// [`Ty::Ref`] for the invariant the pair maintains.
    pub(crate) fn emit_ref_store(&mut self, b: BlockId, slot: ValueId, value: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::RefStore { slot, value },
            on_error: None,
            raise_site: None,
        });
    }
    /// Appends an [`InstKind::FieldSet`] to `b` — see
    /// [`Self::lower_reassignment`]'s property-target arm for the retain/
    /// release policy wrapped around this.
    pub(crate) fn emit_field_set(
        &mut self,
        b: BlockId,
        object: ValueId,
        class: String,
        field: String,
        value: ValueId,
    ) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::FieldSet {
                object,
                class,
                field,
                value,
            },
            on_error: None,
            raise_site: None,
        });
    }
    /// Appends an [`InstKind::StaticSet`] to `b` — see
    /// [`Self::lower_store`]'s static-property arm for the retain/release
    /// policy wrapped around this, which is [`Self::emit_field_set`]'s
    /// unchanged.
    pub(crate) fn emit_static_set(
        &mut self,
        b: BlockId,
        class: String,
        name: String,
        value: ValueId,
    ) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::StaticSet { class, name, value },
            on_error: None,
            raise_site: None,
        });
    }
    /// Appends an [`InstKind::ArraySet`] to `b`, yielding the array that now
    /// holds the entry — see that variant's own doc comment for the
    /// consume-one-reference-yield-one protocol, and
    /// [`Self::lower_reassignment`]'s `Index`-target arm for the retain policy
    /// wrapped around this and for where the result is written back to.
    pub(crate) fn emit_array_set(
        &mut self,
        b: BlockId,
        array: ValueId,
        key: ValueId,
        value: ValueId,
    ) -> ValueId {
        self.emit(b, Ty::Array, InstKind::ArraySet { array, key, value })
            .0
    }
    /// Appends an [`InstKind::ArrayAppend`] to `b`, yielding the array that
    /// now holds the entry — see [`Self::emit_array_set`], whose protocol this
    /// shares.
    ///
    /// Unlike that one it goes through [`Self::emit_fallible`]: an append is
    /// the one array write with an outcome other than success, since PHP
    /// refuses one whose next integer key is already live
    /// (`nvs_runtime::array`'s *the append is the one array write with a fault
    /// channel*). Neither operand needs anything of the landing block — the
    /// refusal releases the value it was handed and leaves the array's
    /// reference where the frame's own slot already names it.
    pub(crate) fn emit_array_append(
        &mut self,
        b: BlockId,
        array: ValueId,
        value: ValueId,
        env: &mut Env,
    ) -> ValueId {
        self.emit_fallible(b, Ty::Array, InstKind::ArrayAppend { array, value }, env)
            .0
    }
    /// Binds `name` to `(v, ty)` in `env` — every `var`/typed local
    /// declaration and every plain reassignment goes through here, `source`
    /// being the already-lowered right-hand-side expression. This is the
    /// declare/reassign half of the module docs' refcounting policy (the
    /// function-exit half is [`Self::release_all_locals`]):
    ///
    /// - If `ty` [`Ty::is_refcounted`] and `source` [`is_aliasing_read`] (a
    ///   bare `$other` variable read, or a compile-time-known property read),
    ///   this bind creates a *second* durable owner of a value some other
    ///   binding already owns — retain `v` first. A fresh literal, `new`, or
    ///   a call's own result already has exactly one natural owner, so none
    ///   of those need a retain here.
    /// - If `name` already held a refcounted value — an overwrite, not a
    ///   fresh declaration — release the *old* value, always *after* the
    ///   retain above so a self-assignment (`$x = $x;`) never observes a
    ///   transient zero refcount.
    pub(crate) fn bind_local(
        &mut self,
        cur: BlockId,
        env: &mut Env,
        name: String,
        v: ValueId,
        ty: Ty,
        source: &Expr,
    ) {
        let aliasing = self.aliasing_read(source);
        self.bind_local_value(cur, env, name, v, ty, aliasing);
    }
    /// The value half of [`Self::lower_store`]: `(value, representation,
    /// whether it aliases storage another owner keeps)`.
    pub(crate) fn lower_stored(
        &mut self,
        stored: &Stored<'_>,
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty, bool) {
        match stored {
            Stored::Expr(e) => {
                let (v, ty) = self.lower_expr(e, expected, env, cur);
                (v, ty, self.aliasing_read(e))
            }
            Stored::Value(v, ty) => (*v, *ty, false),
        }
    }
    /// [`Self::bind_local`] for a value with no right-hand-side expression to
    /// judge — a read-modify-write's combined result, which is a fresh
    /// producer by construction (see [`Self::lower_read_modify_write`]).
    pub(crate) fn bind_local_value(
        &mut self,
        cur: BlockId,
        env: &mut Env,
        name: String,
        v: ValueId,
        ty: Ty,
        aliasing: bool,
    ) {
        if ty.is_refcounted() && aliasing {
            self.emit_retain(cur, v);
        }
        if let Some(&(old_v, old_ty)) = env.get(&name)
            && old_ty.is_refcounted()
        {
            self.emit_release(cur, old_v);
        }
        // `$v = e` where `$v` is a `foreach (… as inout $v)` binding writes the
        // entry too — see `Self::write_through_element`, and note that the
        // header's own per-iteration binding does not come through here.
        self.write_through_element(cur, env, &name, v, ty);
        env.insert(name, (v, ty));
    }
    /// Re-points whatever holds `base` at `written`, the array an
    /// [`InstKind::ArraySet`]/[`InstKind::ArrayAppend`] just yielded.
    ///
    /// **No retain and no release.** Those two instructions consume one
    /// reference to the array they were given and produce one to the array
    /// they yield (see [`InstKind::ArraySet`]'s own doc comment); the
    /// reference consumed is the holder's, and the one produced replaces it in
    /// the same slot. When nothing else held the array the two are the same
    /// reference to the same allocation and this is a pure bookkeeping change
    /// with no runtime cost at all — which is the whole point of `rule:types/arrays`'s copy-on-write being a *write*-side cost.
    ///
    /// The holders that can be written back to are exactly the ones
    /// [`is_aliasing_read`] already recognises as durable storage: a bare
    /// local, a compile-time-known property, and a static property. That the
    /// list is the same list is the whole reason the paragraph above holds —
    /// a durable slot's read hands over no reference of its own, so the one
    /// the array instruction consumed was the holder's and the one it produced
    /// belongs back in the same place. A nested subscript
    /// (`$grid[0][1] = 5`) never reaches here as `base` at all:
    /// [`Self::lower_reassignment`] flattens the whole chain first and hands
    /// this its *root*, having already separated and re-pointed every level
    /// in between. Any other base is a write into a temporary and has no
    /// holder to speak of, so it panics naming itself rather than silently
    /// dropping the separation.
    ///
    /// The property arm's own `else` is unreachable, and its message carries
    /// the proof: a `PropertyAccess` span carries a `HookedProperty`, a
    /// `ShapeProperty`, a `Property` or nothing, and only a `Property`
    /// survives to here. A *hooked* property (`rule:classes/property-hooks`) is a pair of accessors
    /// rather than a slot and an *erased* one (`rule:types/erased-member-access`) is resolved by
    /// name at run time, so neither could be written back to at all;
    /// `nvs_types::expr::assign`'s `check_write_target` refuses both where the
    /// write is written, as `E0478` — PHP's own "indirect modification of
    /// overloaded property" — and `E0480`. Nothing recorded means the access
    /// was diagnosed instead, and a body holding a diagnostic is never
    /// lowered. That is the whole reason this function needs no rule for any
    /// of the others.
    pub(crate) fn write_back_array(
        &mut self,
        base: &Expr,
        written: ValueId,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        // `($a)["0"] = "y"` writes `$a["0"]`, here and in PHP alike: the holder
        // is whatever the parentheses wrap. Every caller flattens through them
        // as well, so this is the belt to that braces.
        let base = base.unparenthesized();
        match &base.kind {
            ExprKind::Variable(name_span) => {
                let name = strip_sigil(span_text(self.src, *name_span)).to_owned();
                // `$a[] = e` where `$a` is an `inout` parameter: the holder is
                // the caller's slot, reached through the reference the
                // parameter is bound to, so the separated array goes back
                // through that slot — the store `Self::lower_store`'s
                // reference arm makes, minus its retain and release, because
                // the reference the `ArraySet` consumed was the slot's and the
                // one it produced replaces it (the paragraph above). The `Env`
                // entry stays the reference: re-pointing it at the array would
                // leave the caller's slot holding the consumed reference and
                // every later read of `$a` on a binding that is not the slot,
                // which is one write the caller never sees and one release
                // too many at the end of the frame.
                if let Some(&(slot, Ty::Ref)) = env.get(&name) {
                    let pointee = self.pointee_of(&name);
                    let written = self.coerce(*cur, written, Ty::Array, pointee, env);
                    self.emit_ref_store(*cur, slot, written);
                    return;
                }
                // A narrowed `?array<T>` root was *read* at `Ty::Array`
                // (`Lowering::untag_narrowed`), but the local it is written
                // back into is still the one tagged slot its declaration gave
                // it, and the narrowing ends with the guard. Re-tagging keeps
                // the binding's representation the one every path out of the
                // guard agrees on; the `Tag` transfers the separated array's
                // reference exactly as this arm's plain store does.
                let held = env.get(&name).map(|&(_, ty)| ty);
                let (written, ty) = if held == Some(Ty::Tagged) {
                    (
                        self.coerce(*cur, written, Ty::Array, Ty::Tagged, env),
                        Ty::Tagged,
                    )
                } else {
                    (written, Ty::Array)
                };
                // `$v[0] = e` where `$v` is a `foreach (… as inout $v)` binding:
                // the separated row is what the entry now holds.
                self.write_through_element(*cur, env, &name, written, ty);
                env.insert(name, (written, ty));
            }
            ExprKind::PropertyAccess { object, .. } => {
                let Some(ExprInfo::Property { class, name, .. }) = self.exprs.lookup(base.span)
                else {
                    unreachable!(
                        "nvs-ir reaches an element write back through the property at {:?} \
                         carrying anything but an `ExprInfo::Property` only if a gate above \
                         it let one through, and none can — this is an invariant, not a gap. \
                         A `PropertyAccess` span carries exactly one of three entries or \
                         none: `nvs_types::expr::members::check_property_member` records \
                         `HookedProperty`, `ShapeProperty` or `Property` on every access it \
                         returns a resolved type from, and reports a diagnostic on every \
                         path it records nothing on, so a body reaching here at all had an \
                         entry. `nvs_types::expr::assign::check_write_target` then refuses \
                         the first two where the write is written — `E0478` for `rule:classes/property-observer` \
                         § 1's pair of accessors, `E0480` for `rule:types/erased-member-access`'s erased \
                         receiver — for all four write spellings alike, leaving `Property` \
                         as the only entry an element write's holder can still carry",
                        base.span
                    );
                };
                let class_label = class.to_string();
                let field_name = name.clone();
                let (object_v, receiver_ty) = self.lower_expr(object, None, env, cur);
                // A narrowed `?T` receiver is still one tagged slot wide, so it
                // arrives here as a `Ty::Tagged` — see
                // `Lowering::untag_receiver`, which every other write through a
                // property already goes through. Without it the `FieldSet`
                // below stores through a 128-bit "pointer" and cranelift
                // rejects the function rather than anything panicking here.
                let (object_v, _) = self.untag_receiver(object_v, receiver_ty, *cur);
                self.emit_field_set(*cur, object_v, class_label, field_name, written);
            }
            // `Class::$prop[k] = v` — `Self::static_property_of` answers the
            // slot's whole identity out of the typed-expression table, so
            // there is no receiver to evaluate and nothing that could run user
            // code a second time; the store is the field arm above minus every
            // step that needs one.
            ExprKind::StaticPropertyAccess { .. } => {
                let (class, name, _) = self.static_property_of(base);
                self.emit_static_set(*cur, class, name, written);
            }
            other => guarded_by!(
                code::E_ELEMENT_WRITE_ROOT_NOT_A_PLACE,
                "nvs-ir reached an array-element write back through {other:?}. \
                 `nvs_types::expr::assign::check_write_target` refuses every root but a bare \
                 local, a compile-time-known property and a static property where the write is \
                 written, because `rule:types/arrays`'s copy-on-write separation has to be \
                 written back to whatever holds the array, so this body was not checked with \
                 the same table"
            ),
        }
    }
    /// The declared type behind the `inout $name` parameter `name` — the pointee
    /// representation [`Self::ref_locals`] remembers, which the parameter's
    /// own `Env` entry cannot carry (it holds [`Ty::Ref`]).
    ///
    /// # Panics
    ///
    /// Panics for a name whose `Env` entry is a [`Ty::Ref`] that
    /// [`lower_method`] never registered — an internal inconsistency, since
    /// the two are written together and nothing else produces a [`Ty::Ref`]
    /// binding.
    pub(crate) fn pointee_of(&self, name: &str) -> Ty {
        *self.ref_locals.get(name).unwrap_or_else(|| {
            panic!(
                "nvs-ir: `${name}` is bound as a `Ty::Ref` but no pointee representation was \
                 recorded for it — bug in lower_method's by-reference parameter binding"
            )
        })
    }
    /// The top of [`Self::pending_refs`], taken by a call site before it
    /// lowers its argument list and handed back to
    /// [`Self::flush_ref_writebacks`] once its call has returned.
    ///
    /// See [`Self::pending_refs`] for why a call site needs a mark rather than
    /// draining the whole list: an enclosing call's arguments are already
    /// staged by the time a nested one is lowered.
    pub(crate) fn pending_refs_mark(&self) -> usize {
        self.pending_refs.len()
    }
    /// Emits the copy-back of every by-reference argument staged since `mark`,
    /// in staging order, and pops those entries off [`Self::pending_refs`].
    ///
    /// Each one is an [`InstKind::RefLoad`] out of the slot the call may have
    /// written, then a re-point of the holder ([`Self::write_back_holder`]).
    /// Called by the call site itself, into the block the call returned into,
    /// so the write lands where PHP's does — at the call, not at the enclosing
    /// statement. See [`Self::pending_refs`] for why `mark` is what keeps a
    /// nested call from flushing its caller's staging, and [`Ty::Ref`] for the
    /// whole representation.
    pub(crate) fn flush_ref_writebacks(&mut self, mark: usize, env: &mut Env, cur: BlockId) {
        for staged in self.pending_refs.split_off(mark) {
            let (v, _) = self.emit(cur, staged.ty, InstKind::RefLoad { slot: staged.slot });
            self.write_back_holder(&staged.holder, v, staged.ty, env, cur);
        }
    }
    /// Re-points `holder` at `written`, the value an [`InstKind::RefLoad`]
    /// just read back out of a by-reference argument's staged slot after the
    /// call returned.
    ///
    /// This is the copy-back half of [`Ty::Ref`]'s representation, and it
    /// **releases the holder's previous value** (when `ty` is
    /// [`Ty::is_refcounted`]) without retaining `written`: the slot owned one
    /// reference from the moment [`Self::lower_call_args`] staged it, and that
    /// reference transfers into the holder here. The staging retain and this
    /// release are the balanced pair — see [`Ty::Ref`]'s refcounting section,
    /// which owns the whole policy and the one path (a throwing callee) that
    /// does not reach this.
    ///
    /// Deliberately not [`Self::write_back_array`]: that one re-points a
    /// holder at a copy-on-write separation and owns *no* release at all,
    /// because [`InstKind::ArraySet`] consumed the very reference it replaces.
    /// Folding the two into one function would mean a flag deciding which
    /// ownership rule applies, which is the thing worth keeping apart.
    pub(crate) fn write_back_holder(
        &mut self,
        holder: &RefHolder,
        written: ValueId,
        ty: Ty,
        env: &mut Env,
        cur: BlockId,
    ) {
        match holder {
            RefHolder::Local(name) => {
                if let Some(&(old_v, old_ty)) = env.get(name)
                    && old_ty.is_refcounted()
                {
                    self.emit_release(cur, old_v);
                }
                // `f(inout $v)` where `$v` is a `foreach (… as inout $v)` binding: what
                // the callee wrote back reaches the entry too, the same way
                // an assignment to it does.
                self.write_through_element(cur, env, name, written, ty);
                env.insert(name.clone(), (written, ty));
            }
            RefHolder::Field {
                object,
                class,
                field,
            } => {
                if ty.is_refcounted() {
                    let (old_v, _) = self.emit(
                        cur,
                        ty,
                        InstKind::FieldGet {
                            object: *object,
                            class: class.clone(),
                            field: field.clone(),
                        },
                    );
                    self.emit_release(cur, old_v);
                }
                self.emit_field_set(cur, *object, class.clone(), field.clone(), written);
            }
        }
    }
    /// Whether `e` reads storage some durable slot still owns, so a value
    /// taken from it needs a retain before anything else can own it too —
    /// [`is_aliasing_read`]'s syntactic judgment, plus the one case that
    /// judgment cannot make from syntax alone.
    ///
    /// `$obj->prop` *looks* like a slot read at every hooked and unhooked
    /// property alike, but a property with an `rule:classes/property-hooks` `get` hook is a
    /// **call**: its result is a fresh, already-owned value, exactly like any
    /// other call's, and retaining it would leak one reference per read. Only
    /// `nvs_types`' resolution can tell the two apart, which is why this is a
    /// method on the lowering rather than a free function over the AST —
    /// every retain decision in this file goes through it.
    ///
    /// The second case a syntactic judgment cannot make is the *base*: a slot
    /// is only durable if whatever holds it is. `$m->make()->name` reads a
    /// field of an object nothing else owns, so
    /// [`Lowering::lower_property_access`] retains the value it reads and
    /// releases the object underneath it — the whole expression is a fresh
    /// producer, exactly like the call inside it, and a consumer that
    /// retained it again would leak one reference per read. `$a["k"]->name`
    /// is still an aliasing read, because the base is.
    ///
    /// `$m->rows()["0"]` is the same shape over an array's element rather than
    /// an object's slot, and [`Lowering::lower_index`] answers it identically,
    /// so the recursion below covers both spellings of "reads a slot of
    /// something nothing else owns".
    pub(crate) fn aliasing_read(&self, e: &Expr) -> bool {
        // A staged sub-expression of an assignment target is a borrow of a
        // value this frame's own temporaries stack (or a durable slot) already
        // owns — see `Self::staged_targets`. It answers `true` whatever its
        // syntax was, which is what stops the second read of a rewritten
        // `$t = $t ⊕ e` releasing a receiver the first read still needs.
        if self.staged(e.span).is_some() {
            return true;
        }
        // Parentheses group and never change what an expression *is*
        // (`nvs_syntax::ast::Expr::unparenthesized`), so every question below
        // is asked of what they wrap: `($a)` aliases the local exactly as `$a`
        // does, and answering `false` here for one would make
        // `array<string> $b = ($a);` release the array twice. The staged
        // lookup runs on both spans because a rewritten target's staged
        // sub-expression is recorded under the span it was *written* with.
        let e = e.unparenthesized();
        if self.staged(e.span).is_some() {
            return true;
        }
        // `rule:types/closure-self-name`'s self-name reads the invoke's own receiver, a binding
        // this frame's `Env` holds for the whole body — so a call through it
        // borrows exactly as `$f(...)` borrows the local `$f`, and answering
        // `false` here would have the call release a receiver the rest of the
        // body still needs. Asked before the syntactic judgment because a bare
        // name is not one of its shapes, and only the checker's record tells
        // this one from the `E0319` every other bare name is.
        if matches!(self.exprs.lookup(e.span), Some(ExprInfo::ClosureSelf)) {
            return true;
        }
        if !is_aliasing_read(&e.kind) {
            return false;
        }
        if matches!(
            self.exprs.lookup(e.span),
            Some(ExprInfo::HookedProperty { get: Some(_), .. })
        ) {
            return false;
        }
        match &e.kind {
            ExprKind::PropertyAccess { object, .. } => self.aliasing_read(object),
            ExprKind::Index { base, .. } => self.aliasing_read(base),
            _ => true,
        }
    }
    /// Releases every refcounted local still live in `env`, in a fixed
    /// (name-sorted) order for deterministic output — the function-exit half
    /// of the module docs' refcounting policy (see [`Self::bind_local`] for
    /// the declare/reassign half). `except`, when given, is the one local
    /// name whose value is transferring out as the function's own return
    /// value rather than being dropped here — `Self::lower_stmt`'s
    /// `StmtKind::Return` arm passes it exactly when the returned expression
    /// is itself a bare `$name` read, the only shape that can currently
    /// alias a still-live local; anything else (a fresh literal, or no
    /// return value at all) has no local to exclude, so every live local is
    /// released.
    ///
    /// A `static` method's parameter 0 needs no exclusion here: it is a
    /// [`Ty::ClassDesc`], which is not refcounted and never enters `env` at
    /// all (see [`lower_method`]).
    pub(crate) fn release_all_locals(&mut self, cur: BlockId, env: &Env, except: Option<&str>) {
        let mut names: Vec<&String> = env.keys().collect();
        names.sort();
        for name in names {
            if Some(name.as_str()) == except {
                continue;
            }
            let &(v, ty) = env.get(name).expect("just listed from env.keys()");
            if ty.is_refcounted() {
                self.emit_release(cur, v);
            }
        }
    }
    /// Consumes the builder, pairing up every block's id, instructions and
    /// terminator.
    ///
    /// # Panics
    ///
    /// Panics if any block was never sealed — see the module docs for why
    /// every block this lowering creates is expected to reach a terminator
    /// one way or another; hitting this would be a lowering bug, not a
    /// malformed input program.
    pub(crate) fn finish(self) -> (Vec<BasicBlock>, Vec<Span>, Vec<Span>) {
        let (stmt_spans, edge_spans) = self.ids.into_spans();
        let blocks = self
            .block_ids
            .into_iter()
            .zip(self.block_insts)
            .zip(self.block_terms)
            .map(|((id, insts), term)| BasicBlock {
                id,
                insts,
                term: term.unwrap_or_else(|| {
                    panic!(
                        "nvs-ir: bb{} was never sealed with a terminator",
                        id.index()
                    )
                }),
            })
            .collect();
        (blocks, stmt_spans, edge_spans)
    }
}

/// Reads a numeric-literal span's text with `_` digit separators stripped.
fn clean_digits(src: &SourceFile, span: nvs_diagnostics::Span) -> String {
    span_text(src, span).chars().filter(|&c| c != '_').collect()
}

/// A fractional literal's text as `rule:types/decimal`'s `(mantissa, scale)`, with any exponent folded into the scale, or
/// `None` where either bound is exceeded.
///
/// Read from the *digits* rather than through an `f64`, which is what makes
/// § 2's placing rule worth having: `19.99 as decimal` is exact to the full 29
/// significant digits instead of recovering only the ~17 an `f64` round-trips.
/// Trailing zeros are kept, because § 4 makes scale observable in rendering.
///
/// Deliberately a second implementation of `nvs_types::expr`'s
/// `decimal_literal_overflow`, for the reason that function's own doc comment
/// gives for `int_literal_digits`: the dependency runs the other way, and the
/// checker has to *report* an out-of-range literal where this only has to
/// build an in-range one. A literal this refuses has already been diagnosed
/// there, so the caller panics rather than folding a wrong value.
pub(crate) fn decimal_literal_parts(text: &str) -> Option<(u128, u8)> {
    let (numeric, exponent) = match text.split_once(['e', 'E']) {
        Some((numeric, exponent)) => (numeric, exponent.parse::<i32>().ok()?),
        None => (text, 0),
    };
    let (whole, fraction) = numeric.split_once('.').unwrap_or((numeric, ""));
    let mut digits = format!("{whole}{fraction}");
    let scale = i32::try_from(fraction.len()).ok()? - exponent;
    if scale < 0 {
        // A positive exponent wider than the fractional part is an integer:
        // shift the point right by padding the mantissa instead.
        digits.push_str(&"0".repeat(usize::try_from(scale.unsigned_abs()).ok()?));
    }
    let mantissa = digits.parse::<u128>().ok()?;
    let scale = u8::try_from(scale.max(0)).ok()?;
    (mantissa <= DECIMAL_MAX_MANTISSA && scale <= DECIMAL_MAX_SCALE).then_some((mantissa, scale))
}

/// `rule:types/decimal`'s mantissa bound, restated here for the same reason
/// [`decimal_literal_parts`] is: `nvs_runtime::decimal`, which owns it, is not
/// a dependency of this crate.
const DECIMAL_MAX_MANTISSA: u128 = (1u128 << 96) - 1;

/// `rule:types/decimal`'s scale bound — see [`DECIMAL_MAX_MANTISSA`].
const DECIMAL_MAX_SCALE: u8 = 28;

/// Cooks a plain, non-interpolated string literal's span — `nvs_syntax::ast::ExprKind::Str`'s
/// own doc comment: a single-quoted string, or a double-quoted/heredoc/nowdoc string with no
/// interpolation in it — into its runtime bytes.
///
/// All three spellings live in [`nvs_types::string_lit::cook_string_literal`], which owns the
/// escape grammar and the flexible-heredoc strip; this wrapper exists only so the call sites
/// below keep reading as one local name. Sharing rather than duplicating is the same call
/// `crate::string_lit`'s own module docs already record for the double-quoted half: the checker
/// has to diagnose exactly the cooking that happens here, so one routine has to perform both —
/// unlike [`int_literal_digits`], where the crate boundary runs the other way.
fn cook_str_literal(src: &SourceFile, span: nvs_diagnostics::Span) -> String {
    nvs_types::string_lit::cook_string_literal(src, span)
}

/// Splits a cooked integer-literal span into the radix its prefix names and
/// the digit run to parse against it — `nvs_syntax::Lexer::lex_number` emits
/// one `IntLiteral` token for all four forms (`0x…`/`0o…`/`0b…`, or a plain
/// decimal run; a legacy leading-zero octal spelling like PHP's `0755` is
/// deliberately *not* one of them, so `0755` lexes as decimal 755 with no
/// prefix to strip), and this is the one place that distinction has to be
/// undone before `str::from_str_radix` can parse the value.
/// `nvs_types::expr::literals::infer_int_literal` range-checks the same digits (mirroring this
/// function to do so, since this crate has no reverse dependency on that one) and reports `rule:types/arithmetic`'s diagnostic before lowering ever runs — see its doc comment — so [`Lowering::lower_int_literal`] can
/// treat an out-of-range literal as unreachable input, the same "trusts `nvs_types::check_program`
/// already ran" contract every other panic in this crate relies on.
fn int_literal_digits(src: &SourceFile, span: nvs_diagnostics::Span) -> (u32, String) {
    let digits = clean_digits(src, span);
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(rest) = digits.strip_prefix(prefix) {
            return (radix, rest.to_owned());
        }
    }
    (10, digits)
}

/// One `foreach` binding's declared representation.
///
/// # Panics
///
/// Panics naming `which` binding it was for a header that declares no type at
/// all. `rule:types/grammar`.2 makes both bindings' types mandatory and
/// `nvs_syntax`'s parser already reported the omission (the `None` here is the
/// error-recovery placeholder [`ForeachBinding::ty`]'s own doc comment
/// describes), so lowering never runs on such a file.
fn binding_ty(
    binding: &ForeachBinding,
    which: &str,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> Ty {
    let ty = binding.ty.as_ref().unwrap_or_else(|| {
        panic!(
            "nvs-ir: a `foreach` {which} binding reached lowering with no declared type — \
             nvs_syntax already reports that omission, so this file should never have been \
             lowered"
        )
    });
    lower_decl_type(ty, exprs, checked_types)
}

/// Lowers a *declared* type straight off the AST — every scalar atom, plus
/// `TypeAtom::Name(_)` (a plain class/interface/enum name) as
/// [`Ty::Object`], `TypeAtom::Array(_)` (bare `array` or `array<T>`) as
/// [`Ty::Array`], and `TypeAtom::Mixed` as [`Ty::Tagged`]. A plain name needs
/// no resolution to lower this way: `rule:types/declaration` already requires it to be
/// spelled out in full, and this crate erases class identity entirely (see
/// [`Ty::Object`]'s own doc comment), so "is this atom a class name at all"
/// is the only question that matters here — which class doesn't need
/// answering until a call/`new` on it does, via [`erase_checked_ty`] instead.
/// `array<T>`'s own type argument is discarded the same way — [`Ty::Array`]'s
/// own doc comment explains why no lowering decision needs it at this level.
///
/// `self`/`static`/`parent` lower to [`Ty::Object`] alongside a plain name,
/// and need no enclosing-class context to do it: each resolves to *some*
/// class, and this crate erases which one. That erasure is exactly why the
/// three cannot be left here — the class a `static` return type names is
/// decided at the call site, not the declaration, and nothing about that
/// question is a *representation* question. `new static()`'s and
/// `static::m()`'s actual class travels as a value instead
/// ([`Ty::ClassDesc`]), which is what [`Lowering::lsb`] produces. `rule:types/class-reference`'s
/// `class<T>` is the one annotation that names that representation directly,
/// and it answers the *representation* question without answering the identity
/// one either — see [`Ty::ClassDesc`].
///
/// # Panics
///
/// Every atom `rule:types/grammar` has is answered by name, so neither panic
/// here is a shape this crate refuses. `TypeAtom::Member(..)` is the one atom
/// the AST cannot answer alone and is guaranteed by
/// [`ExprTypeTable::declared_ty`](nvs_types::expr_table::ExprTypeTable::declared_ty),
/// whose entry the shortcut above takes first; the trailing arm exists because
/// `nvs_syntax::ast::TypeKind` and `TypeAtom` are `#[non_exhaustive]`, and a
/// spelling landing in it is a bug in the change that added it.
pub(crate) fn lower_decl_type(
    ty: &Type,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> Ty {
    // The checker already resolved this exact annotation and recorded the
    // answer (`ExprTypeTable::declared_ty`) — take it whenever it exists, so a
    // name-shaped atom whose meaning depends on resolution comes out right.
    // `rule:enums/closed-integer-type`'s enum is the case that forces this: `Rank $r` is an integer
    // binding and `Dog $d` is an object one, and nothing in the AST tells the
    // two apart. The match below stays as the answer for an annotation the
    // checker never visited, where every atom is its own answer anyway.
    if let Some(id) = exprs.declared_ty(ty.span) {
        return erase_checked_ty(id, checked_types);
    }
    match &ty.kind {
        TypeKind::Atom(TypeAtom::SelfTy | TypeAtom::StaticTy | TypeAtom::Parent) => Ty::Object,
        TypeKind::Atom(TypeAtom::Bool) => Ty::Bool,
        TypeKind::Atom(TypeAtom::Int) => Ty::Int,
        TypeKind::Atom(TypeAtom::Uint) => Ty::Uint,
        TypeKind::Atom(TypeAtom::Float) => Ty::Float,
        TypeKind::Atom(TypeAtom::Decimal) => Ty::Decimal,
        TypeKind::Atom(TypeAtom::Void) => Ty::Void,
        TypeKind::Atom(TypeAtom::String) => Ty::Str,
        TypeKind::Atom(TypeAtom::Bytes) => Ty::Bytes,
        // `rule:types/literal-types` again, for the one annotation shape that can be
        // answered without resolution. `TypeAtom::Member(..)` cannot: whether
        // it erases to a string, an int or an enum tag is exactly the question
        // the checker answered, so it takes the `declared_ty` shortcut above
        // or it is a bug.
        TypeKind::Atom(TypeAtom::StringLiteral(_)) => Ty::Str,
        TypeKind::Atom(TypeAtom::IntLiteral(_)) => Ty::Int,
        // `rule:types/grammar`'s two `bool` singletons, which `nvs_types::ty::Ty::True`
        // records are that same rule read on `bool`'s two values — so they
        // erase to `bool`'s representation exactly as the two atoms above
        // erase to theirs.
        TypeKind::Atom(TypeAtom::True | TypeAtom::False) => Ty::Bool,
        // `object` — `rule:types/grammar`'s opaque top of every class type, which is
        // the same pointer a named class is. See `erase_checked_ty`, which is
        // the arm this annotation actually takes whenever the checker visited
        // it.
        TypeKind::Atom(TypeAtom::Name(..) | TypeAtom::Object) => Ty::Object,
        // `rule:types/class-reference`'s class reference. Its argument is erased exactly the
        // way a class name is erased one arm above — see [`Ty::ClassDesc`] for
        // why nothing below this boundary asks `class<T>` what `T` was.
        TypeKind::Atom(TypeAtom::ClassRef(_)) => Ty::ClassDesc,
        // `rule:types/property-key`'s property key, whose values are names — see
        // [`erase_checked_ty`]'s own arm, which is the one this annotation
        // takes whenever the checker visited it, for what the argument costs
        // and where the set it erases went.
        TypeKind::Atom(TypeAtom::PropertyKey(_)) => Ty::Str,
        // `rule:types/callable-absorbs-closure`'s one closure type. Its *representation* is an object
        // — see the `ExprKind::Fn` arm of `Lowering::lower_expr`, which
        // synthesizes one class per literal to hold the captured environment
        // — so it erases here exactly the way a class name does.
        TypeKind::Atom(TypeAtom::Callable) => Ty::Object,
        // `rule:types/callable-signature`'s written signature erases to the
        // same object for the same reason: what a signature buys is a proof
        // the checker holds, and no part of it has a representation below this
        // boundary.
        TypeKind::Atom(TypeAtom::CallableSig { .. }) => Ty::Object,
        TypeKind::Atom(TypeAtom::Array(_)) => Ty::Array,
        // `mixed` — `rule:types/grammar`. See `Ty::Tagged`'s own doc comment for
        // exactly how much this representation does and doesn't do yet: a
        // local/parameter/return/call-argument round-trips, nothing else.
        TypeKind::Atom(TypeAtom::Mixed) => Ty::Tagged,
        TypeKind::Paren(inner) => lower_decl_type(inner, exprs, checked_types),
        // `rule:security/tainted-qualifier` and `rule:security/secret-qualifier`:
        // a qualifier is a bit on the *checker's* type and adds no runtime
        // representation at all, so each qualified spelling erases to the base
        // it shares a tag and an allocation with — the same answer
        // [`erase_checked_ty`]'s own arm gives the checked type, and that arm
        // owns what the erasure costs and where the one decision that needs a
        // qualifier back is recorded instead.
        TypeKind::Atom(
            TypeAtom::TaintedString | TypeAtom::SecretString | TypeAtom::SecretTaintedString,
        ) => Ty::Str,
        TypeKind::Atom(
            TypeAtom::TaintedBytes | TypeAtom::SecretBytes | TypeAtom::SecretTaintedBytes,
        ) => Ty::Bytes,
        // `rule:types/object-literal`'s shape is an ordinary refcounted
        // instance with no methods and no name, so its representation is the
        // object pointer a class already is — see [`erase_checked_ty`]'s arm
        // for what the erasure drops and why no site below this boundary wants
        // the field list.
        TypeKind::Atom(TypeAtom::Shape(_)) => Ty::Object,
        // `null` alone is one value with one representation.
        TypeKind::Atom(TypeAtom::Null) => Ty::Null,
        // `never` is return-only (`rule:types/grammar`) and a frame that cannot
        // come back hands its caller nothing, which is the representation
        // `void` already is. [`erase_checked_ty`]'s own `Never` arm carries why
        // the difference between the two is control flow and is not encoded in
        // a representation.
        TypeKind::Atom(TypeAtom::Never) => Ty::Void,
        // `iterable` is PHP's `array|Traversable` (`rule:iteration/two-interfaces`):
        // two runtime shapes, so the tagged representation every other
        // multi-shape position uses. Deliberately not `Ty::Object`, for the
        // reason [`erase_checked_ty`]'s arm gives — an `array<T>` is not an
        // object pointer.
        TypeKind::Atom(TypeAtom::Iterable) => Ty::Tagged,
        // All three admit more than one runtime shape, so all three are tagged
        // — see `Ty::Tagged`. Reached only for an annotation the checker never
        // visited; everything it did visit takes the `declared_ty` shortcut
        // above and goes through `erase_checked_ty`, which is the *narrower*
        // answer since `rule:types/literal-types`'s fold lives there: it folds `"a"|"b"`
        // back to the one representation its members share, which needs the
        // resolved members and so cannot be answered from the AST alone. An
        // intersection folds the same way there and is tagged here for a
        // second reason as well: `nvs_types::expr::assign` has no arm making
        // anything assignable to one, so the answer is wrong for no value.
        TypeKind::Nullable(_) | TypeKind::Union(_) | TypeKind::Intersection(_) => Ty::Tagged,
        // `rule:enums/closed-integer-type`'s `Suit::Hearts` written as a type,
        // and the one atom this match cannot answer: whether it erases to an
        // `int`, a `string` or an enum's tag is the resolution the checker
        // performed, and the AST carries the name alone. Every annotation the
        // checker visited is answered by the `declared_ty` shortcut above, so
        // arriving here means the table lost an entry it records — a bug in
        // whatever built it rather than a representation this crate is
        // missing.
        TypeKind::Atom(TypeAtom::Member(..)) => panic!(
            "nvs-ir: a member-constant type annotation reached lowering with no \
             `ExprTypeTable::declared_ty` entry, which the checker records for every annotation \
             it visits — this is a bug"
        ),
        // `nvs_syntax::ast::TypeKind` and `TypeAtom` are both `#[non_exhaustive]`,
        // so this arm is where a spelling added to the grammar lands rather
        // than a row missing above: every atom `rule:types/grammar` has is
        // answered by name, and `lower_decl_type_answers_every_type_atom_the_grammar_has`
        // counts them. A new one arriving here is a bug in the change that
        // added it.
        other => {
            panic!("nvs-ir: the type grammar grew {other:?} without an answer — this is a bug")
        }
    }
}

/// The per-slot constants recorded for the `Ty::CoreShape` parameter at
/// `index` — what a slot the written literal does not fill passes.
/// `nvs_types::core_lib` synthesizes exactly one entry per such parameter, so
/// a bag or a shape parameter always has one.
///
/// Two variants carry it and the split is the parameter's own optionality,
/// which that function's docs own: an omittable bag records its fills as the
/// parameter's own default (`ConstArg::Options`, `rule:core-api/shape-rules` R2), and a shape
/// parameter a call must write records them without becoming optional
/// (`ConstArg::RequiredShape`, `rule:core-api/shape-flattens-at-the-abi`). Both flatten identically here —
/// the whole point of the merged list is that a call site reads one order.
///
/// # Panics
///
/// Panics if that parameter's recorded default is absent or is neither
/// variant, which would mean the signature table and the parameter type
/// disagree about what the parameter is.
fn shape_fills(
    defaults: &[Option<nvs_types::ConstArg>],
    index: usize,
) -> &[(String, nvs_types::ConstArg)] {
    match defaults.get(index) {
        Some(Some(
            nvs_types::ConstArg::Options(fills) | nvs_types::ConstArg::RequiredShape(fills),
        )) => fills,
        other => panic!(
            "nvs-ir: parameter {index} is a shape but its recorded default is {other:?} — \
             nvs_types::core_lib is trusted to record one fill list per bag and per shape"
        ),
    }
}

/// The label the class synthesized for an `rule:types/object-literal` shape literal carries
/// — `$shape{x,y}` for `{x: 1, y: 2}`, from the field names **already
/// sorted**.
///
/// A label, not a name: it identifies the class in [`crate::ir::Program`]'s
/// table and nowhere else, and it is never a symbol, since a shape class has
/// no methods to emit one for. `$` cannot start an Novis identifier, so no
/// declaration can collide with it — the same guarantee the `Owner$fn0` a
/// closure's environment class carries relies on.
///
/// Keyed on the sorted field names alone, so two literals with the same
/// fields share one class whatever their field *types* are: a class carries
/// slot names, not slot types, and both sides count slots the same way.
///
/// So a wire contract, which is per-field types and nothing else, is not this
/// class's to carry — the crate's module docs own where it rides instead, and
/// why widening this label to name the types was refused.
///
/// The string itself is [`nvs_types::derive::shape_class_label`]: a call site
/// writing a shape as its type argument names this same class one crate
/// earlier, when the checker records what that site wrote, so the two spellings
/// of one label are one function.
pub(crate) fn shape_class_label(sorted_fields: &[String]) -> String {
    nvs_types::derive::shape_class_label(sorted_fields)
}

/// The key one inline shape's wire contract is filed and named by — the
/// counterpart of [`shape_class_label`] for the half a class label cannot
/// carry, and what [`crate::ir::InstKind::ShapeCodecConst`] holds.
///
/// It renders **every column that makes two contracts different**, so two call
/// sites writing the same fields at the same wire types share one
/// `nvs_runtime::ShapeCodec` and two writing `{n: int}` and `{n: string}` — one
/// class, by that label's own rule — get one table each. Nothing outside this
/// compile reads the spelling: `nvs-codegen` mangles it into a relocation
/// symbol, and both ends of that relocation derive the name from a key built
/// here, so what it looks like is this function's business alone.
pub(crate) fn shape_codec_key(fields: &[nvs_types::CodecField]) -> String {
    let mut out = String::from("$codec{");
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&field.key);
        // `rule:types/shape-type`'s own marker for the field that may be
        // absent, which is `rule:core-api/required-optional-and-nullable`'s
        // first column and independent of the second one below.
        if !field.required {
            out.push('?');
        }
        out.push(':');
        out.push_str(&format!("{:?}", field.ty));
        if let Some(element) = &field.element {
            // Every level of it: `array<Tag>` and `array<array<Tag>>` are two
            // contracts and decode differently, so a key that rendered only
            // the outermost element would file them under one table.
            push_element_key(element, &mut out);
        }
        if let Some(class) = &field.class {
            out.push('@');
            out.push_str(class);
        }
        // Two enums erase to one `CodecTy::Enum` and are told apart by the
        // roster alone, so a key that dropped it would decode one enum's
        // document against another's accepted values.
        if let Some(cases) = &field.cases {
            out.push_str(&format!("({cases:?})"));
        }
        if field.nullable {
            out.push_str("|null");
        }
    }
    out.push('}');
    out
}

/// One list field's element chain, appended to the key [`shape_codec_key`] is
/// building — `<Str>`, `<List<Str>>`, one pair of angle brackets per level.
fn push_element_key(element: &nvs_types::CodecElement, out: &mut String) {
    out.push('<');
    out.push_str(&format!("{:?}", element.ty));
    if let Some(inner) = &element.element {
        push_element_key(inner, out);
    }
    out.push('>');
}

/// One inline shape's [`nvs_types::derive::DerivedCodec`], as the field list
/// `nvs_runtime::ShapeCodec` holds.
///
/// [`codec_fields`]'s counterpart, and shorter than it for one reason: a class's
/// field list is joined against `nvs_types::layout`'s slot order, and a shape
/// has no layout to join against — the class synthesized for it lays its slots
/// out in the sorted field-name order this list is already in, so a field's
/// index **is** its slot and its constructor parameter alike.
fn shape_codec_fields(codec: &nvs_types::derive::DerivedCodec) -> Vec<nvs_types::CodecField> {
    codec
        .fields
        .iter()
        .enumerate()
        .map(|(slot, field)| nvs_types::CodecField {
            key: field.key.clone(),
            slot,
            param: slot,
            ty: field.ty,
            element: field.element.clone(),
            // The label and the roster ride down untouched, for
            // [`codec_fields`]'s reasons: `nvs-codegen` is the first place a
            // descriptor exists, and a case list is already the values
            // themselves.
            class: field.class.clone(),
            cases: field.cases.clone(),
            shape: nested_shape_key(field),
            nullable: field.nullable,
            required: field.required,
            // A shape declares no constructor, so there is no default to carry
            // — `nvs_runtime::CodecField::default`.
            default: None,
        })
        .collect()
}

/// The [`nvs_types::CodecField::shape`] a derived field carries: the key of the
/// table holding an inline shape field's own per-field wire types, and `None`
/// for every other wire type.
///
/// The class half of the same erasure needs nothing here — `nvs_types::derive`
/// already named it, and it rides down as the field's `class` label like a
/// nested class's does.
fn nested_shape_key(field: &nvs_types::derive::DerivedField) -> Option<String> {
    let nested = field.shape.as_deref()?;
    Some(shape_codec_key(&shape_codec_fields(nested)))
}

/// Every inline shape reached through a field of `codec`, and through one of
/// theirs: the contract each decodes against, and the class each decodes into.
///
/// A shape written as a **type argument** is registered where its call site
/// lowers ([`Lowering::written_type_constants`]). One reached as a *field* has
/// no site to lower at — a class may declare `{n: int}` in a program that never
/// writes a literal of it — so both halves are collected here instead, which is
/// what leaves `nvs-codegen` two resolvable pointers rather than one label and a
/// null.
fn nested_shapes(
    codec: &nvs_types::derive::DerivedCodec,
    codecs: &mut Vec<crate::ir::ShapeCodec>,
    classes: &mut Vec<crate::ir::Class>,
) {
    for field in &codec.fields {
        let Some(nested) = field.shape.as_deref() else {
            continue;
        };
        let fields = shape_codec_fields(nested);
        let names: Vec<String> = nested
            .fields
            .iter()
            .map(|nested_field| nested_field.property.clone())
            .collect();
        let field_count = names.len();
        classes.push(crate::ir::Class {
            label: shape_class_label(&names),
            fields: names,
            // `Ty::Tagged` on every slot, on [`Lowering::record_shape_class`]'s
            // terms: what fills these is a native decoder rather than a lowered
            // write, so there is no tag to promise, and a literal of the same
            // field names merges onto that same answer.
            field_reprs: vec![Ty::Tagged; field_count],
            secret_fields: vec![false; field_count],
            public_fields: vec![true; field_count],
            protected_fields: vec![false; field_count],
            field_types: Vec::new(),
            constants: Vec::new(),
            attributes: Vec::new(),
            conforms: Vec::new(),
            methods: Vec::new(),
            hooks: Vec::new(),
            codec: Vec::new(),
            db_codec: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
            is_closure: false,
        });
        codecs.push(crate::ir::ShapeCodec {
            key: shape_codec_key(&fields),
            fields,
        });
        // A shape's own field may be a shape, and the contract it decodes
        // against is one more table to publish.
        nested_shapes(nested, codecs, classes);
    }
}

/// The class label a member on `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` is
/// handed as its first argument, from what the checker recorded at the call
/// site — the one place all three call paths read it, so they cannot disagree
/// about what a missing one means.
///
/// A written **shape** answers the class synthesized for it, which is the same
/// descriptor a `{n: 1}` literal of those field names builds; what that label
/// cannot carry is the field *types*, and
/// [`Lowering::written_type_constants`] is where the contract holding them is
/// emitted beside it.
///
/// # Panics
///
/// Panics naming the member when the checker recorded neither a written class
/// nor a written shape, which is one run's two halves disagreeing rather than
/// anything a program wrote.
pub(crate) fn written_class_label(call: &nvs_types::expr_table::ResolvedCall) -> String {
    if let Some(shape) = &call.written_shape {
        return shape.label.clone();
    }
    call.written_class
        .as_ref()
        .unwrap_or_else(|| {
            panic!(
                "nvs-ir: `{}::{}` needs the class written at its call site, and nvs_types \
                 recorded none — did this program pass nvs_types::check_program with the \
                 same table?",
                call.class, call.method
            )
        })
        .to_string()
}

/// Translates an already-*checked* type — a [`TypeId`] recorded in an
/// [`ExprInfo::Call`]/[`ExprInfo::New`]/[`ExprInfo::Property`] entry, naming a
/// call's resolved parameter/return type or a property's declared field type
/// — into this crate's own [`Ty`]. Distinct from [`lower_decl_type`], which
/// reads a type straight off the AST instead: this one exists because a
/// resolved call's parameter/return types (and a property's field type) come
/// from `nvs_types`' own interner, not from a `Type` AST node this crate can
/// lower directly (there may be no local `Type` node at all, e.g. an
/// inherited method's parameter declared on a different class's source).
/// `Class` erases to [`Ty::Object`], same as [`lower_decl_type`]'s `Name`
/// case — see that variant's own doc comment for why identity doesn't need to
/// survive this translation. `Enum` does *not* join it: `rule:enums/closed-integer-type` makes an enum
/// a closed integer type, so it lowers to [`Ty::Enum`] carrying the backing
/// type `nvs_types::ty::Ty::Enum` already knows (see that variant for why the
/// backing rides in the checker's type rather than in a side table). `String`
/// erases to [`Ty::Str`] and `Bytes` to [`Ty::Bytes`] — the same
/// representations [`lower_decl_type`] already gives a local/parameter/return
/// type spelled directly in source — see [`crate::lower`]'s module docs for
/// the retain policy this needs at a call-argument/return/property-field
/// boundary, which [`Lowering::bind_local`], [`Lowering::lower_call_args`] and
/// `StmtKind::Return`'s own arm all apply via [`is_aliasing_read`].
///
/// Every row of the checker's type answers here, so nothing this function is
/// asked has to be asked as a question: a union folds by erasing each member
/// (`rule:types/literal-types`'s "zero additional runtime representation"),
/// and a member that once had no row is what made that fold an [`Option`].
///
/// # Panics
///
/// Neither panic is a shape this crate refuses. [`CheckedTy::CoreShape`] is a
/// `Core` options bag, which is not one value at all — the arm names what
/// flattens it before any type is erased — and the trailing arm exists because
/// `nvs_types::ty::Ty` is `#[non_exhaustive]`, so a row added upstream lands
/// there rather than going unnoticed.
///
/// Everything else erases. `object` is [`Ty::Object`] with
/// [`CheckedTy::Class`], [`CheckedTy::Callable`] and [`CheckedTy::Shape`];
/// [`CheckedTy::Never`] is [`Ty::Void`], the representation of "the caller
/// receives nothing", and that arm owns why the call site keeps its ordinary
/// fall-through; `mixed` is [`Ty::Tagged`] — see that variant's own doc
/// comment for exactly how much this boundary does and doesn't do with one
/// yet. A **union** is [`Ty::Tagged`] unless every member erases to one and the
/// same representation, in which case it is that one.
pub(crate) fn erase_checked_ty(id: TypeId, checked_types: &TypeInterner) -> Ty {
    match checked_types.get(id) {
        CheckedTy::Bool => Ty::Bool,
        CheckedTy::Int => Ty::Int,
        CheckedTy::Uint => Ty::Uint,
        CheckedTy::Float => Ty::Float,
        CheckedTy::Decimal => Ty::Decimal,
        CheckedTy::Void => Ty::Void,
        CheckedTy::String => Ty::Str,
        CheckedTy::Bytes => Ty::Bytes,
        // `rule:security/tainted-qualifier` and `rule:security/secret-qualifier`: `tainted` and `secret` are two
        // independent bits on the *checker's* type and add **zero** runtime
        // representation, exactly as `rule:types/literal-types`'s literal types do above. So
        // every qualified atom erases to the base it shares a tag and an
        // allocation with, and everything below this boundary sees a plain
        // `string` or `bytes`.
        //
        // What that costs is one thing, and it is paid for: a lowering
        // decision that genuinely depends on a qualifier cannot read it back
        // here. `rule:security/secret-comparison-is-constant-time`'s constant-time `==` is the only such decision,
        // and the checker records it at the comparison instead
        // (`nvs_types::expr_table::ExprInfo::SecretEquality`).
        CheckedTy::TaintedString | CheckedTy::SecretString | CheckedTy::SecretTaintedString => {
            Ty::Str
        }
        CheckedTy::TaintedBytes | CheckedTy::SecretBytes | CheckedTy::SecretTaintedBytes => {
            Ty::Bytes
        }
        // A shape joins them: `rule:types/object-literal` makes a shape value an ordinary
        // refcounted instance with no methods and no name of its own, so its
        // representation is the object pointer a class already has. What the
        // erasure drops is the field list, and nothing below this boundary
        // wants it — a field read carries its own slot index, resolved where
        // the type still existed (`InstKind::SlotGet`).
        //
        // Plain `object` is the same representation with the field list never
        // present in the first place — `rule:types/grammar`'s opaque top of every
        // class type. A member access through one is `rule:types/erased-member-access`'s fully
        // erased half: the checker records the field's *name* and nothing
        // else, and `InstKind::SlotGet`/`SlotSet` find it on the concrete
        // descriptor or throw.
        //
        // A `callable` carrying a written signature erases here too, and to the
        // same pointer: `rule:types/callable-signature` moves the *check* to
        // where the call is written, so what is left below this boundary is the
        // closure object the bare type already lowered to, with nothing about
        // its parameters left to represent.
        // `Core\Task::all`'s own parameter joins them, and for the same
        // reason: `rule:concurrency/all-answers-a-typed-shape` accepts a shape
        // and nothing else, so the value below this boundary is that shape's
        // object pointer. It is the one checker type here that survives
        // substitution — it is what the argument is *checked against* — so it
        // reaches this boundary rather than being rewritten before it.
        CheckedTy::Class(..)
        | CheckedTy::Callable
        | CheckedTy::CallableSig { .. }
        | CheckedTy::Shape(_)
        | CheckedTy::ShapeOfCallables(_)
        | CheckedTy::Object => Ty::Object,
        // `rule:types/class-reference`: a class reference's value is the run-time descriptor
        // `new static` already carries, so it erases to that representation and
        // its argument goes the way `Class`'s identity goes one arm above.
        CheckedTy::ClassRef(_) => Ty::ClassDesc,
        // `rule:types/property-key`: **a key is a name**, so its values are exactly the
        // `string`s `T`'s public properties are declared under and the
        // representation is the one a `string` already has — no descriptor
        // field, no tag of its own, and § 2's `property<T>` → `string` row
        // free by construction.
        //
        // The argument is dropped here the way `class<T>`'s is one arm above,
        // and it costs the same thing in the same place: the *set* a key may
        // hold does not survive, so § 2's checked rows cannot re-derive it
        // below this boundary. They do not have to — the checker records the
        // resolved roster at the conversion
        // (`nvs_types::expr_table::ExprInfo::PropertyKey`) and
        // `Lowering::property_key_set` reads it back, which is the same
        // "record it where the type still existed" the erased member access one
        // arm above already relies on.
        CheckedTy::PropertyKey(_) => Ty::Str,
        // `rule:types/literal-types`: a literal type and an enum-case type add **zero**
        // runtime representation. Each erases to the base it shares a tag and
        // payload with, so the singleton-ness stops at this boundary and
        // nothing below it learns a new type -- which is the whole of what
        // that section promises.
        CheckedTy::StringLiteral(_) => Ty::Str,
        CheckedTy::IntLiteral(_) => Ty::Int,
        // `true` and `false` are the same rule on `bool`'s two values (ADR
        // 0007 § 3's atoms, read by `nvs_types::ty::Ty::True`), so they erase
        // to `bool`. The union arm below then folds `true|false` back to one
        // `Ty::Bool` for free.
        CheckedTy::True | CheckedTy::False => Ty::Bool,
        CheckedTy::Enum(_, backing) | CheckedTy::EnumCase(_, backing, _) => {
            Ty::Enum(match backing {
                nvs_types::EnumBacking::Int => EnumRepr::Int,
                nvs_types::EnumBacking::Uint => EnumRepr::Uint,
            })
        }
        // The element `TypeId` is discarded — same erasure `lower_decl_type`
        // already gives `TypeAtom::Array(_)`, see `Ty::Array`'s own doc
        // comment for why this crate has no lowering decision that needs it.
        CheckedTy::Array(_) => Ty::Array,
        CheckedTy::Mixed => Ty::Tagged,
        // `null` alone is one value with one representation; anything that
        // admits *more* than one runtime shape is tagged. `?T` reaches here as
        // `Union([Null, T])` — the checker has no separate nullable type — so
        // the two arms below are the whole of `rule:expressions/nullable-conversion`'s representation.
        CheckedTy::Null => Ty::Null,
        // `rule:types/literal-types` again, and the whole of what it means: there is no
        // second representation, so a union whose members all erase to one
        // `Ty` **is** that `Ty`. `"a"|"b"` is a `Ty::Str`, `1|2` a `Ty::Int`
        // and `Mode::Read|Mode::Write` the enum's own tag — the erasure the
        // three atom arms above already perform, applied once more to the
        // union that collects them. Everything else still admits more than
        // one runtime shape and is tagged, `?T` (`Union([Null, T])`) included:
        // `Ty::Null` and `Ty::Str` are two representations, not one.
        CheckedTy::Union(members) => shared_erasure(members, checked_types),
        // An intersection takes the *same* rule, for the same reason and with
        // a different argument for it. A union is one of its members at run
        // time; an intersection is all of them at once — but either way the
        // question here is only how many runtime shapes the position admits,
        // and a value satisfying `Marker&Other` is one object pointer exactly
        // as a value satisfying `Marker|Other` is. So `A&B` over two class
        // atoms is `Ty::Object`, and a spellable-but-uninhabitable pair like
        // `int&string` is `Ty::Tagged` rather than a panic — the answer that
        // is wrong for no value, since no value reaches it.
        //
        // `rule:types/grammar` admits the type and `nvs_types::expr::assign` has no
        // arm making anything assignable to one, so nothing can inhabit an
        // intersection today; this arm is what lets the *declaration* compile
        // rather than panic below a checker that accepted it.
        CheckedTy::Intersection(members) => shared_erasure(members, checked_types),
        // `iterable` is PHP's `array|Traversable` and Novis keeps that reading
        // (`rule:iteration/two-interfaces`): two runtime shapes, so the representation is the
        // tagged one `mixed` and every other multi-shape position already
        // uses. Deliberately **not** `Ty::Object` — an `array<T>` is not an
        // object pointer, so the object erasure would be a correctness trap
        // waiting for the day `assign` grows the arm that makes an array
        // assignable here. Nothing is assignable to `iterable` today, so this
        // arm is likewise the declaration's, not any value's.
        CheckedTy::Iterable => Ty::Tagged,
        // `never` is the one atom whose *value* question has no value in it:
        // `rule:types/grammar` makes it return-only, and a frame that cannot come back
        // hands its caller nothing. That is the representation `void` already
        // is, so `never` erases to it — the caller of a `never` member reads no
        // slot, exactly as the caller of a `void` one does.
        //
        // **The difference between the two is control flow, and it is not
        // encoded here.** A `never` callee leaves its frame by throwing or by
        // exiting, and *that* is the terminator: the call site takes `rule:errors/propagation`'s
        // unwind edge to its landing pad, which the lowering already emits for
        // every call. The alternative — sealing the call site's fall-through
        // with an unreachable on the strength of the annotation — was rejected
        // rather than merely not written. It encodes the same fact a second
        // time, and it converts any hole in the checker's "this body leaves the
        // frame" analysis from a missed refusal into unreachable code that
        // executes. Erasing to `void` is wrong for no program: a `never` member
        // that did return would fall through to the caller's next instruction,
        // which is the safe answer rather than the undefined one.
        //
        // That hole is not hypothetical, which is what settled the choice.
        // `nvs_types::check::check_every_path_returns` exempts `never`
        // alongside `void`, so a body declaring `never` that simply falls off
        // its end compiles today and comes back; and `nvs_types::returns`
        // walks statements syntactically, so a *call* to a `never` member does
        // not yet count as leaving the frame the way a `throw` does. Both are
        // refusals the checker owes and neither is a representation question,
        // so both are open there rather than worked around here.
        CheckedTy::Never => Ty::Void,
        // A type variable reaches a lowering through exactly one door, and it
        // is not a call: `rule:classes/delegation-by-field`'s synthesized
        // forward declares the *interface's* own parameter and return types
        // (`crate::lower::call::delegation_forward`), and where that interface
        // is one of `rule:iteration/two-interfaces`' compiler-owned generics
        // those types are still the variables `nvs_types::signatures::MethodSig`
        // substitutes at a call site. So the forward's slots are the one
        // position here whose type is genuinely unconstrained, and `Ty::Tagged`
        // is what every such position already is — the same answer `mixed`
        // gets, which is also what an unbound variable substitutes to.
        CheckedTy::TypeVar(_) => Ty::Tagged,
        // A `Core` options bag is the one checker type with no representation
        // here, because it is not one argument: `rule:core-api/shape-flattens-at-the-abi`
        // makes it one per slot of the merged field list, and
        // `Lowering::lower_fixed_arg` reads the parameter's *checked* type to
        // send it to `Lowering::lower_options_arg` before `ArgSig::expectation`
        // erases anything.
        // `rule:core-api/shape-parameter` keeps the type spellable in the `Core`
        // registry alone, so no annotation, no written `callable` signature, no
        // union member and no interface a class may delegate can name one, and
        // no value of it ever arrives. Reaching this is a bug in whatever
        // lowered a `Core` parameter as a value.
        CheckedTy::CoreShape(_) => panic!(
            "nvs-ir: a `Core` shape parameter was erased as one value — \
             `Lowering::lower_options_arg` flattens it into one argument per merged slot before \
             its type is asked for; this is a bug"
        ),
        // `nvs_types::ty::Ty` is `#[non_exhaustive]`, so this arm is where a
        // row added to the checker's type lands rather than a row missing
        // above: every one it has today is answered by name, and
        // `erase_checked_ty_answers_every_checked_type_a_value_can_have` counts
        // them. A new one arriving here is a bug in the change that added it.
        other => {
            panic!("nvs-ir: the checker's type grew {other:?} without an erasure — this is a bug")
        }
    }
}

/// The one runtime shape a member list admits, or [`Ty::Tagged`] where it
/// admits more than one.
///
/// Shared by the union and intersection arms of [`erase_checked_ty`]: both ask
/// the same question — how many representations does this position hold — and
/// `rule:types/literal-types`'s "there is no second representation" is what makes the answer
/// a fold rather than a case analysis.
fn shared_erasure(members: &[TypeId], checked_types: &TypeInterner) -> Ty {
    let mut shared: Option<Ty> = None;
    for member in members {
        match (erase_checked_ty(*member, checked_types), shared) {
            (ty, None) => shared = Some(ty),
            (ty, Some(seen)) if ty == seen => {}
            // `rule:types/class-reference`'s `?class<T>` — the one pair of *different*
            // erasures that is still one representation, and the reason this
            // fold is not simply "every member erases the same way". A
            // descriptor is an address and no class lives at address zero, so
            // [`Ty::ClassDesc`] has a spare value that means "no class" and
            // `null` is spelled with it. See that variant's own doc comment
            // for the sites that then have to ask a `ClassDesc` whether it is
            // null rather than reading `Ty::Tagged` off the operand.
            (Ty::Null, Some(Ty::ClassDesc)) | (Ty::ClassDesc, Some(Ty::Null)) => {
                shared = Some(Ty::ClassDesc);
            }
            _ => return Ty::Tagged,
        }
    }
    shared.unwrap_or(Ty::Tagged)
}

/// What an assignment stores, once its target's address is settled.
///
/// [`Lowering::lower_store`] lowers the target either way; only where the
/// value comes from differs. A read-modify-write (`$x += 1`, `$x++`) has
/// combined its value before the store runs and has no expression left to
/// hand over — see [`Lowering::lower_read_modify_write`].
pub(crate) enum Stored<'a> {
    /// `$t = e;` — lowered against the target's declared representation, and
    /// judged by [`Lowering::aliasing_read`] like any other read.
    Expr(&'a Expr),
    /// A value already in hand. Always a **fresh producer**: the only thing
    /// that builds one is a binary operator's result, which no durable slot
    /// holds yet.
    Value(ValueId, Ty),
}

pub(crate) fn is_aliasing_read(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Variable(_)
            | ExprKind::PropertyAccess { .. }
            | ExprKind::Index { .. }
            // `Class::$prop` reads a slot the *request* owns, which outlives
            // every frame that can read it — so it is an aliasing read with no
            // base to recurse into, the one shape here that is durable by
            // construction rather than by whatever it was read out of.
            | ExprKind::StaticPropertyAccess { .. }
    )
}

/// The one method an `rule:types/closure-literal`
/// closure's environment class answers, as the method table spells it.
pub(crate) const FN_INVOKE: &str = "invoke";

/// The label every closure's environment class conforms to, and that no class a
/// program declares does: `rule:types/callable-is-a-closure` makes `callable` a
/// question about one shape of value, and a `conforms` edge is the one thing a
/// descriptor already carries that answers it — so `$x is callable` is the
/// descriptor walk `instanceof` emits rather than a mechanism of its own.
///
/// A label and not a name, on [`shape_class_label`]'s terms: `$` cannot start an
/// Novis identifier, so no declaration can collide with it and no source can
/// name this class to implement it by hand.
///
/// The descriptor is emitted into every program whether or not the file writes a
/// closure ([`lower_program`]), for the exception tree's reason: a walk needs
/// something to compare against before it can answer `false`.
///
/// **Cost:** one field-less, method-less descriptor per compiled unit — once per
/// unit, never per request or per task.
pub(crate) const CLOSURE_MARKER: &str = "$closure";

/// The reserved **first** field of every closure's environment class: how many
/// parameters [`FN_INVOKE`] declares, not counting the receiver.
///
/// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
/// § 2 hands every `Core\Arr` callback `($value, $key)` and lets it "declare
/// fewer parameters" — so a native caller has to know how many the closure
/// actually wants before it can pass, and retain, the right number. A
/// descriptor carries no arity, so the closure object carries it, in a slot
/// whose index `nvs_runtime::CLOSURE_ARITY_SLOT` restates and
/// `nvs-codegen`'s `a_closure_object_carries_its_own_arity_in_slot_zero`
/// holds the two together.
///
/// One 16-byte slot per closure, per evaluation of the literal — bought
/// against a second class-descriptor field that every non-closure class would
/// carry too.
pub(crate) const FN_ARITY: &str = "fn#arity";

/// The reserved **second** field of every closure's environment class: which
/// runtime tag each parameter of [`FN_INVOKE`] requires, packed one nibble
/// per parameter into the slot's `int` payload, parameter 0 in the least
/// significant nibble.
///
/// # Why the object carries it
///
/// `rule:types/closure-literal`
/// gives `callable` no parameter list, so **no checker can compare a call site
/// against the body it will reach** — and the compiled `invoke` reads argument
/// slot *i* at its own declared representation, which turns a mismatch into an
/// arbitrary dereference rather than a fault. That is a priority-1 hole, so
/// the one party that still knows the declared types — this lowering, at the
/// literal — writes them down for the one party that can act on them:
/// `nvs_runtime::call_closure`, which compares before it passes.
///
/// A nibble is the `nvs_runtime::Tag` discriminant the argument must carry,
/// so the reader needs no table of its own; [`param_tag_nibble`] is the map
/// and `nvs-codegen`'s `param_tag_nibbles_are_the_runtime_tag_bytes` holds it
/// against `nvs_codegen::ty::tag_of`, which is the same fact one crate over.
/// [`FN_PARAM_TAG_ANY`] is the one nibble that is not a tag: a `mixed`, `?T`
/// or union parameter is `Ty::Tagged`, whose representation *is* a tag byte
/// chosen at run time, so nothing about the argument can be wrong.
///
/// # The bound, and what happens past it
///
/// Sixteen parameters fit ([`FN_PARAM_TAGS_CAPACITY`]). A closure declaring
/// more gets no nibble for its seventeenth onward, and `nvs_runtime` refuses
/// the *call* rather than passing a parameter it cannot check — fail-closed,
/// under this repository's priority ordering, and unreachable from a callback
/// the spec describes, which is handed two arguments.
///
/// One further 16-byte slot per closure, per evaluation of the literal, beside
/// [`FN_ARITY`]'s — priority 5 spent on priority 1, and bought against a
/// per-parameter slot, which would cost the same at two parameters and more at
/// every count above.
pub(crate) const FN_PARAM_TAGS: &str = "fn#params";

/// How many parameters [`FN_PARAM_TAGS`] describes: one nibble each, in the
/// 64 bits of a slot's `int` payload.
pub(crate) const FN_PARAM_TAGS_CAPACITY: usize = 16;

/// The reserved **third** field of a *first-class callable*'s class: what the
/// target declares its parameters to be **called**, comma-separated in
/// declaration order — and absent from a `fn` literal's class, which is the
/// whole of how a native reader tells the two apart.
///
/// # Why the object carries it
///
/// [ADR 0006](/docs/decisions/0006.md) § *Decision* binds
/// an isolate's `args:` to its entry's parameters **by name**, and
/// `rule:concurrency/an-upgrade-is-spawn-shaped` makes
/// `Core\Socket::upgrade(Chat::run(...), args: {…})` one of those entries. The
/// `spawn script Class::method` construct has the names as a constant this
/// lowering writes into the call (`Lowering::spawn_method_entry`); a `Core`
/// member's entry is one ordinary argument with no constant beside it, so the
/// names have to ride on the value. [`FN_ARITY`] and [`FN_PARAM_TAGS`] are the
/// precedent and the reason is theirs: this lowering is the last party that
/// can see a declaration, and the party that needs the fact runs much later.
///
/// # Why only a first-class callable's class
///
/// `nvs_types::expr::isolate` admits exactly a path or a `Class::method(...)`
/// where an entry is expected and refuses an `fn` literal with `E0802`, so a
/// literal's names could never be read — and paying a slot per closure
/// evaluation for a field nothing reads is the wrong trade under this
/// repository's priority ordering. The absence is *load-bearing* rather than
/// an omission: `nvs_runtime::closure_param_names` asks the descriptor for a
/// field of this name, so a `fn` literal's closure answers `None` instead of
/// having its first capture read as a name list.
///
/// Placed third, ahead of [`FCC_RECV`](closure::FCC_RECV), so
/// the index `nvs_runtime::CLOSURE_PARAM_NAMES_SLOT` restates is a *hint* that
/// hits on the first probe for every callable that has one.
///
/// One 16-byte slot per written `(...)`, per evaluation of it, holding an
/// interned constant — and nothing at all on the `fn` literals every
/// `Core\Arr` callback is.
pub(crate) const FN_PARAM_NAMES: &str = "fn#names";

/// The [`FN_PARAM_TAGS`] nibble for a parameter no argument can be wrong for.
///
/// Deliberately not a `nvs_runtime::Tag` discriminant, and parked at the
/// **top** of the nibble rather than one past the roster's end so that it
/// stays that way: the discriminant just past the tags is `rule:classes/an-unwritten-property-read-throws`'s never-written
/// storage state, and a nibble chosen as "one past the last tag" is a nibble
/// that collides the next time the roster grows. `nvs-codegen`'s
/// `the_any_nibble_denotes_no_tag_at_all` is the guard.
pub const FN_PARAM_TAG_ANY: u8 = 15;

/// The [`FN_PARAM_TAGS`] nibble a parameter represented as `ty` requires.
///
/// The whole of this map is "the tag a value of that representation carries",
/// which is `nvs_codegen::ty::tag_of` one crate over — `nvs-ir` cannot name
/// `nvs_runtime::Tag` (it does not depend on it) and neither can `nvs-runtime`
/// name this, so the two are held together by a test in `nvs-codegen`, which
/// sees both. That is the same shape [`FN_ARITY`] and
/// `nvs_runtime::CLOSURE_ARITY_SLOT` already stand in.
///
/// Exhaustive on purpose: a new [`Ty`] variant is a decision about what a
/// closure parameter of that representation admits, and this is where it gets
/// taken rather than defaulted.
///
/// Because the key is a representation and not a declared type, a `?T`
/// parameter checks exactly as much as a `mixed` one does — nothing: both
/// erase to [`Ty::Tagged`], so both take [`FN_PARAM_TAG_ANY`] and `?string $k`
/// accepts an `int` argument as readily as `mixed $k` does. That is the answer
/// rather than a hole. A nibble names one tag, so `T`-or-null cannot be
/// spelled here at all, and it does not need to be: a body lowered against a
/// tagged slot reads the tag at every use, which is the guarantee the check
/// exists to give the bodies that do not.
/// `tests/conformance/core/arr-a-mixed-or-nullable-callback-parameter-is-unchecked.nvst`
/// pins both halves from Novis.
///
/// [`Ty::Object`]'s nibble is the one row where the representation is not the
/// whole answer. Every class name erases onto it, so this word admits a
/// closure declaring the wrong class — and a named-class binding, unlike ADR
/// 0036 § 4's erased receiver, is read and written at a *fixed offset* against
/// its label, so admitting one is a type confusion rather than a wrong answer.
/// The label four bits have no room for is checked at the closure's **entry**
/// instead, one `instanceof` per class-declared parameter
/// (`lower::closure::check_param_class`); `docs/adr/README.md` § *Decisions
/// taken at project start* owns why that boundary pays rather than every
/// named-class property access, and
/// `tests/conformance/core/out-a-callback-parameter-naming-a-class-checks-the-argument-class-at-entry.nvst`
/// pins both lines — this one around objecthood, that one around ancestry.
pub fn param_tag_nibble(ty: Ty) -> u8 {
    match ty {
        // `Ref` and `ClassDesc` ride in the payload of an otherwise-`null`
        // slot, exactly as `tag_of` says; neither is writable as a parameter's
        // declared type, and an `inout $x` parameter is refused before it gets here.
        Ty::Null | Ty::Ref | Ty::ClassDesc => 0,
        Ty::Bool => 1,
        // `rule:enums/closed-integer-type`'s enum travels as its backing integer, tag included, so
        // these two rows are why an enum and its backing type are one
        // representation here and two enums over one backing are as well —
        // `tests/conformance/core/arr-a-callback-enum-parameter-is-its-backing-integer.nvst`
        // pins the agreement, and the `int`/`uint` split that keeps it exact.
        Ty::Int | Ty::Enum(EnumRepr::Int) => 2,
        Ty::Uint | Ty::Enum(EnumRepr::Uint) => 3,
        Ty::Float => 4,
        Ty::Str => 5,
        Ty::Array => 6,
        Ty::Object => 7,
        Ty::Decimal => 10,
        Ty::Bytes => 11,
        // `Tagged` accepts every tag by construction. `Void` is not a value
        // and cannot be written in a parameter position at all, so it has no
        // argument to judge either.
        Ty::Tagged | Ty::Void => FN_PARAM_TAG_ANY,
    }
}

/// The [`FN_PARAM_TAGS`] word for a parameter list already lowered to its
/// representations: one [`param_tag_nibble`] each, in declaration order, least
/// significant first.
///
/// The one implementation of that packing. Its two callers pack the same word
/// for two readers that must agree — `lower::closure`'s
/// `param_tags_word` writes it into a closure object for
/// `nvs_runtime::call_closure`, and `nvs-codegen` writes it into a
/// `nvs_runtime::MethodRow` for the erased call `rule:types/erased-member-access` defers — and both
/// readers are one `check_param_tags`, so two packings would be two chances
/// for the shift or the capacity to be read differently.
///
/// Parameters past [`FN_PARAM_TAGS_CAPACITY`] contribute no nibble, which is
/// what makes the reader refuse the call rather than pass an argument it
/// cannot judge. The receiver is **not** one of these: a caller fills the
/// declared parameters and the callee's own slot 0 is its receiver, so both
/// callers hand over the explicit list alone.
pub fn pack_param_tags(params: impl IntoIterator<Item = Ty>) -> u64 {
    let mut word: u64 = 0;
    for (i, ty) in params.into_iter().take(FN_PARAM_TAGS_CAPACITY).enumerate() {
        word |= u64::from(param_tag_nibble(ty)) << (i * 4);
    }
    word
}

/// How many `array<…>` levels one [`array_element_tags`] word describes: a
/// [`param_tag_nibble`] each, packed into a `u64`.
///
/// `nvs_types::expr::operators` refuses a target that nests deeper
/// (`E0711`) — that module's `ARRAY_ELEMENT_TAG_LEVELS` is the same `64 / 4`
/// and the rule's one home — so a deeper annotation never reaches this crate.
pub const ARRAY_ELEMENT_TAG_LEVELS: usize = u64::BITS as usize / 4;

/// The word `rule:types/conversion`'s `array<T> as array<U>` row carries to the runtime:
/// one [`param_tag_nibble`] per level of `U`, outermost first. `array<int>`
/// is one nibble, `2`; `array<array<int>>` is two, `6` then `2`; and
/// `array<mixed>` is [`FN_PARAM_TAG_ANY`], the "every tag" nibble that makes
/// the walk a formality.
///
/// **The row's check is a tag per element and nothing wider**, which is the
/// same four bits a closure parameter's entry check compares
/// ([`param_tag_nibble`]'s own doc comment) and the reason the two share this
/// encoding rather than inventing a second one. A helper argument is stored as
/// an `nvs_runtime::Value`, so an integer is what can travel; a class
/// descriptor cannot, which is exactly why an element type naming a class is
/// refused a phase up rather than lowered here.
///
/// A nibble of `Ty::Array` means "and the next nibble describes *its*
/// elements", so the chain is self-terminating: every array level writes the
/// level below it, and a leaf writes nothing after itself.
///
/// `None` where the target is not an array at all, where a level's type is one
/// no tag decides — a class, a shape, a `callable`, an enum, a literal type, a
/// union — or where it nests past [`ARRAY_ELEMENT_TAG_LEVELS`]. The roster is
/// `nvs_types::expr::operators`' `reject_uncheckable_element_type`, which owns
/// the rule and refuses each of those where it is written (`E0711`), so the
/// only `None` that survives to here is the one that refusal exempts: the
/// **identical type**, `array<Foo> as array<Foo>`, which is
/// [`Lowering::convert`](crate::lower::Lowering::convert)'s free row and wants
/// no walk at all. That is why the caller falls through rather than asserting.
fn array_element_tags(id: TypeId, checked_types: &TypeInterner) -> Option<u64> {
    // `rule:expressions/nullable-conversion`'s `as ?array<U>` asks the same question of the same target, and
    // the `T` node inside the sugar records no checked type of its own
    // (`Lowering::nullable_target_atoms` says why) — so the `?T` is unwrapped
    // here, where the interned `Union([Null, T])` still has it.
    let id = match checked_types.get(id) {
        CheckedTy::Union(members) => {
            let mut named = members
                .iter()
                .filter(|member| !matches!(checked_types.get(**member), CheckedTy::Null));
            let only = *named.next()?;
            if named.next().is_some() {
                return None;
            }
            only
        }
        _ => id,
    };
    let CheckedTy::Array(element) = checked_types.get(id) else {
        return None;
    };
    let mut current = *element;
    let mut word = 0u64;
    for level in 0..ARRAY_ELEMENT_TAG_LEVELS {
        // The allowlist is spelled out rather than taken from
        // `erase_checked_ty`, because what a level needs is not "does this
        // have a representation" but "is its representation the *whole* of
        // it": every type below admits exactly the values one tag admits, and
        // one that admits fewer would be asserted rather than checked.
        let repr = match checked_types.get(current) {
            CheckedTy::Null => Ty::Null,
            CheckedTy::Bool => Ty::Bool,
            CheckedTy::Int => Ty::Int,
            CheckedTy::Uint => Ty::Uint,
            CheckedTy::Float => Ty::Float,
            CheckedTy::Decimal => Ty::Decimal,
            CheckedTy::String => Ty::Str,
            CheckedTy::Bytes => Ty::Bytes,
            CheckedTy::Mixed => Ty::Tagged,
            CheckedTy::Array(_) => Ty::Array,
            _ => return None,
        };
        word |= u64::from(param_tag_nibble(repr)) << (level * 4);
        match checked_types.get(current) {
            CheckedTy::Array(inner) => current = *inner,
            _ => return Some(word),
        }
    }
    None
}

/// One lowered body, plus everything the `rule:types/closure-literal` closures inside it
/// synthesized.
///
/// A closure literal is an *expression*, so it is met in the middle of
/// lowering some other function's body — but what it produces is a whole
/// second function and a class, neither of which that body can hold. This is
/// how they travel back out to [`lower_file`], which is the only thing that
/// owns a [`crate::ir::Program`].
#[derive(Debug)]
pub struct Lowered {
    /// The body that was asked for.
    pub function: Function,
    /// One `invoke` per `fn` literal in it, transitively — a closure written
    /// inside another closure's body is in here too.
    pub closures: Vec<Function>,
    /// The captured-environment class each of those is a method of.
    pub classes: Vec<crate::ir::Class>,
}

#[cfg(test)]
mod tests;
