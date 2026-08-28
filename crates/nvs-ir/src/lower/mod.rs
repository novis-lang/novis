//! Lowers one already-checked method body to a [`crate::ir::Function`] — see
//! the crate's own module docs for which statement and expression shapes are
//! covered, and why this trusts its input rather than re-checking it.
//!
//! # Layout
//!
//! One `impl Lowering` split across this directory, which Rust allows for an
//! inherent impl inside a single crate. Each module's methods are
//! `pub(super)`, reaching exactly as far as `lower/` and no further — the
//! visibility they had when this was one 9,000-line file. The split is for
//! collision surface: `for`, `switch`, `match`, `$fn(...)` and ADR 0043's
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
//! | [`generator`] | ADR 0053 § 4's state machine — the frame, the spills, the three synthesized methods |
//! | [`call`] | argument ownership, options-bag flattening, an `inout $x` argument staged and written back |
//! | [`closure`] | ADR 0031 closure literals and their captured-environment class |
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
//! reserved this session.

use nvs_diagnostics::{SourceFile, SourceId, Span};
use nvs_syntax::ast::{
    ArrayItem, AssignOp, BinaryOp, Block, CallArgs, CatchClause, ClassMemberKind,
    DestructureElement, DestructureTarget, Expr, ExprKind, FnBody, FnExpr, ForeachBinding,
    IncDecOp, MatchArm, MethodMember, Modifier, NamespaceDecl, NewTarget, ObjectLiteralField, Stmt,
    StmtKind, StringPart, SwitchCase, Type, TypeAtom, TypeKind, UnaryOp as AstUnaryOp,
};
use nvs_types::EnumTable;
use nvs_types::expr_table::{ArgSlot, ExprInfo, ExprTypeTable, ForeachDrive};
use nvs_types::layout::ClassLayoutTable;
use nvs_types::ty::{Ty as CheckedTy, TypeId, TypeInterner};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{BlockId, EdgeId, IdGen, ValueId};
use crate::ir::{AbsentKey, BasicBlock, BinOp, Function, Helper, Inst, InstKind, Terminator, UnOp};
use crate::ty::{EnumRepr, Ty};
use crate::{span_text, strip_sigil};

// One `impl Lowering` split across this directory — see each module's own
// header. Rust allows that for an inherent impl inside one crate, so this is
// a move and nothing else; the methods there are `pub(super)`, which reaches
// exactly as far as `lower/` and no further.
mod call;
mod closure;
mod control;
mod convert;
mod exception;
mod expr;
mod generator;
mod operator;
mod stmt;

// `call`, `control`, `expr` and `stmt` only add methods to the one
// `impl Lowering` below, so they export nothing to import. The three named
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
enum ArgOwnership {
    /// The callee owns it — an Novis method or constructor, whose parameter is
    /// bound into its own `Env` like a local and released at its exit sweep.
    /// The caller therefore retains an aliasing refcounted argument first, so
    /// the pair balances.
    Transferred,
    /// The callee borrows it — every ADR 0002 helper, including a `Core`
    /// member, which receives a `&[Value]` and releases nothing. No retain,
    /// and the caller keeps owning what it passed; `nvs_stdlib`'s own docs own
    /// why ADR 0063's purity rule is what makes that safe.
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
struct CatchJoin<'e> {
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

/// The class labelled `label`'s declared property defaults, resolved against
/// its own flattened slot order — [`crate::ir::Class::defaults`].
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
    }
    image
        .into_iter()
        .enumerate()
        .filter_map(|(slot, value)| Some((slot, value?)))
        .collect()
}

/// One evaluated constant, as the runtime's own smaller vocabulary spells it —
/// `None` for a variant a written property declaration cannot reach, which is
/// unreachable rather than lossy for [`property_defaults`]' own reason.
fn field_default(value: &nvs_types::ConstArg) -> Option<nvs_types::FieldDefault> {
    use nvs_types::{ConstArg, FieldDefault};

    match value {
        ConstArg::Bool(v) => Some(FieldDefault::Bool(*v)),
        ConstArg::Int(v) => Some(FieldDefault::Int(*v)),
        ConstArg::Uint(v) => Some(FieldDefault::Uint(*v)),
        ConstArg::Float(v) => Some(FieldDefault::Float(*v)),
        ConstArg::Str(s) => Some(FieldDefault::Str(s.clone())),
        ConstArg::EmptyArray => Some(FieldDefault::EmptyArray),
        ConstArg::Null | ConstArg::Bytes(_) | ConstArg::Options(_) | ConstArg::Built { .. } => None,
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
                    // The same fallback `field_reprs` takes, for the same
                    // reason: a type this crate does not represent is
                    // `Ty::Tagged`, which reads and writes the whole 16 bytes
                    // rather than a payload half it cannot name.
                    repr: exprs
                        .property_types(label)
                        .iter()
                        .find(|(property, _)| property == name)
                        .and_then(|(_, ty)| erase_checked_ty(*ty, checked_types))
                        .unwrap_or(Ty::Tagged),
                    default_value: default.as_ref().and_then(field_default),
                })
        })
        .collect();
    statics.sort_by(|a, b| (&a.class, &a.name).cmp(&(&b.class, &b.name)));
    statics
}

/// What each of `label`'s field slots is declared to hold, in slot order —
/// [`crate::ir::Class::field_reprs`], and the whole of what ADR 0036 § 4's
/// erased **write** check has to go on.
///
/// The join is [`property_defaults`]'s exactly: own class first, then every
/// ancestor, so a subclass redeclaring a property wins the slot the two
/// share. What differs is the fallback. A default a class does not write is
/// simply absent, but every slot needs a *representation* for the vector to
/// stay aligned with [`nvs_types::ClassLayout::fields`], so a slot nothing
/// claims — a class whose signature never reached
/// [`ExprTypeTable::property_types`], or a type this crate does not represent
/// — is [`Ty::Tagged`], which `nvs-codegen` maps to "unchecked" rather than
/// to a tag that would refuse a legal write.
fn field_reprs(
    label: &str,
    layout: &nvs_types::ClassLayout,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> Vec<Ty> {
    let mut image: Vec<Option<Ty>> = vec![None; layout.fields.len()];
    let chain = std::iter::once(label).chain(layout.conforms.iter().map(String::as_str));
    for owner in chain {
        for (property, ty) in exprs.property_types(owner) {
            let Some(slot) = layout.slot_of(property) else {
                continue;
            };
            if image[slot].is_some() {
                continue;
            }
            image[slot] = Some(erase_checked_ty(*ty, checked_types).unwrap_or(Ty::Tagged));
        }
    }
    image
        .into_iter()
        .map(|repr| repr.unwrap_or(Ty::Tagged))
        .collect()
}

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
/// `require` and variables do not (ADR 0021 § *Decision*).
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
/// ([ADR 0043](../../../docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)
/// § 2) and a default is an ordinary compiled method under the interface's own
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
                                // ADR 0053 § 4: a body containing `yield` is
                                // a generator, and becomes three functions
                                // and a state class rather than one function
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
                            // ADR 0014 § 1's property hooks are compiled the
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
        // `SourceId`.
        let name = if i == 0 {
            script.to_owned()
        } else {
            file_script_label(file.src.id())
        };
        let lowered = lower_script(&name, file.stmts, file.src, exprs, checked_types, enums);
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
        .map(|(label, layout)| crate::ir::Class {
            label: label.to_owned(),
            fields: layout.fields.clone(),
            // ADR 0036 § 4's erased write reaches *any* class, not just a
            // shape literal's synthesized one, so every layout carries its
            // slots' representations — see `field_reprs`.
            field_reprs: field_reprs(label, layout, exprs, checked_types),
            conforms: layout.conforms.clone(),
            methods: layout.methods.clone(),
            // ADR 0071's field list, joined to this class's slot order — the
            // one place both tables are in hand. A field the layout has no
            // slot for is dropped rather than mis-indexed: `nvs_types::derive`
            // has already reported the declaration that caused it (a promoted
            // parameter is that module's gap 2), and guessing a slot here
            // would write another property's value under this one's key.
            codec: exprs.codec(label).map_or_else(Vec::new, |codec| {
                codec
                    .fields
                    .iter()
                    .filter_map(|field| {
                        Some(nvs_types::CodecField {
                            key: field.key.clone(),
                            slot: layout.slot_of(&field.property)?,
                            // A field whose declaration named no constructor
                            // parameter is dropped for the same reason a
                            // slotless one is: `nvs_types::derive` has already
                            // reported it, and inventing a position would pass
                            // this field's value as another parameter.
                            param: field.param?,
                            ty: field.ty,
                            nullable: field.nullable,
                        })
                    })
                    .collect()
            }),
            ctor_arity: exprs.codec(label).map_or(0, |codec| codec.ctor_arity),
            // Every declared default that lands in one of this class's slots:
            // its own first, then each ancestor's, so a subclass redeclaring a
            // property wins the slot the two share. A label with no slot for
            // the name is skipped rather than mis-indexed, for the same reason
            // the codec above skips one.
            defaults: property_defaults(label, layout, exprs),
        })
        .collect();
    // ADR 0053 § 4's generator state classes have no source declaration and
    // therefore no `nvs_types::layout` entry — `nvs-ir` synthesizes both the
    // class and its two methods, so it is the one thing here that adds to the
    // table rather than copying it.
    classes.extend(synthesized);
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
    classes.dedup_by(|a, b| a.label == b.label);
    // ADR 0043 § 4's `implements I by $field;` forwards: one synthesized
    // method each, and one row each in the delegating class's method table,
    // which is what a receiver typed as the *interface* dispatches through.
    // `nvs_types::Delegation` is where every one of these was decided; this
    // adds the two things only a lowered program has, the function and the
    // row. A name the table already answers is left alone — § 4's "a class may
    // still write its own method with the same name as a delegated one", and
    // an inherited body or an ADR 0043 § 2 interface default on the same
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
            .any(|(name, _)| *name == delegation.method)
        {
            continue;
        }
        let Some(function) = delegation_forward(delegation, checked_types) else {
            continue;
        };
        class
            .methods
            .push((delegation.method.clone(), delegation.class.clone()));
        functions.push(function);
    }

    crate::ir::Program {
        functions,
        classes,
        statics: static_props(layouts, exprs, checked_types),
    }
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
    let mut low = Lowering::new(name, src, ret_ty, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    let mut env = Env::default();
    let mut param_tys = Vec::new();

    // Reserved safepoint poll site (recursion) — see `InstKind::Safepoint`'s
    // own doc comment for why function entry is one of the two fixed sites
    // and why this slice reserves only the shape, not a functional check.
    low.emit_safepoint(entry);

    // The implicit receiver, always parameter index 0 — seeded
    // unconditionally, the same way `nvs_types::check.rs`'s `check_method`
    // seeds `$this` into its own `LocalScope` regardless of a `static`
    // modifier (see that function's own comment for why: a static method's
    // body referencing `$this` is a distinct, unrelated diagnostic, not this
    // crate's concern). `nvs-ir` never lowers a free function — every
    // `MethodMember` it reaches belongs to a class per ADR 0011 — so there is
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
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
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
    // No explicit final `return` — the same fallback the straight-line slice
    // always had, now expressed as sealing whatever block is still open.
    // Nothing transfers out on this path (there is no return value), so
    // every refcounted local still live here gets released, same as an
    // explicit `return;`.
    if !low.is_terminated(cur) {
        low.release_all_locals(cur, &env, None);
        low.seal(cur, Terminator::Return(None));
    }

    let pending = std::mem::take(&mut low.closures);
    // Beside the closures, and out the same channel: see `Lowering::shapes`.
    let shapes = std::mem::take(&mut low.shapes);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, mut classes) = drain_closures(pending, src, exprs, checked_types, enums);
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

/// Lowers one of `p`'s [ADR 0014](../../../docs/adr/0014-property-observer.md)
/// § 1 property hooks to a [`Function`] named `name` — which must be the
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
/// The two accessors differ in exactly three places:
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
    let mut low = Lowering::new(name, src, ret_ty, exprs, checked_types, enums);
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
    // Beside the closures, and out the same channel: see `Lowering::shapes`.
    let shapes = std::mem::take(&mut low.shapes);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, mut classes) = drain_closures(pending, src, exprs, checked_types, enums);
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

/// Lowers a file's own top-level statements into one synthesized function
/// — [ADR 0008](../../../docs/adr/0008-static-and-global.md) § 2's "the
/// script body is a function, so its variables are locals". `name` is the
/// label the listing/a future codegen symbol table uses; the caller picks
/// it, exactly as for [`lower_method`].
///
/// Two things differ from [`lower_method`], and nothing else does:
///
/// - **No implicit receiver.** A script frame has no `$this`, so parameter
///   index 0 is not reserved and `params` is empty — matching
///   `nvs_types::check`'s own script frame, which seeds `$this` only when
///   there is an enclosing class.
/// - **The return representation is [`Ty::Tagged`].** A top-level `return`
///   hands a value back to whatever `require`d the file, and
///   [ADR 0021](../../../docs/adr/0021-single-file-inclusion-construct.md)
///   types that boundary `mixed`. A file that never returns falls through to
///   a seal handing back the tagged `1` § 3 names for that case, which is
///   PHP's own answer — not the `Terminator::Return(None)` `lower_method`
///   uses, a `Ty::Tagged` frame owing its caller a value on every exit.
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
) -> Lowered {
    let ret_ty = Ty::Tagged;
    let mut low = Lowering::new(name, src, ret_ty, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    let mut env = Env::default();

    // The same reserved function-entry safepoint poll site `lower_method`
    // emits — a script body is a function, so it is one of the two fixed
    // sites for the same reason.
    low.emit_safepoint(entry);

    low.lower_script_stmts(stmts, &mut cur, &mut env);
    if !low.is_terminated(cur) {
        low.release_all_locals(cur, &env, None);
        // ADR 0021 § 3 names the value of a file that never `return`s: `1`,
        // which is PHP's own answer for a `require` of such a file. So the
        // fall-through seal is a tagged integer rather than the
        // `Terminator::Return(None)` `lower_method` uses — a `Ty::Tagged`
        // frame always hands a value back, and the one place that would
        // otherwise be free is exactly the one the ADR pins.
        let (one, _) = low.emit(cur, Ty::Int, InstKind::ConstInt(1));
        let one = low.coerce(cur, one, Ty::Int, Ty::Tagged, &mut env);
        low.seal(cur, Terminator::Return(Some(one)));
    }

    let pending = std::mem::take(&mut low.closures);
    // Beside the closures, and out the same channel: see `Lowering::shapes`.
    let shapes = std::mem::take(&mut low.shapes);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, mut classes) = drain_closures(pending, src, exprs, checked_types, enums);
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
struct Lowering<'a> {
    ids: IdGen,
    src: &'a SourceFile,
    ret_ty: Ty,
    /// Where a call's/`new`'s resolved target is read back from — see
    /// [`ExprInfo`] and the crate docs' "design choices" section.
    exprs: &'a ExprTypeTable,
    /// The same `nvs_types::check_program` run's type interner — needed to
    /// translate a [`TypeId`] recorded in `exprs` into this crate's own
    /// [`Ty`] via [`lower_checked_ty`].
    checked_types: &'a TypeInterner,
    /// The same run's [`EnumTable`]: every declared enum's backing type and
    /// its cases' constant values.
    ///
    /// Threaded beside `checked_types` because a case's *value* is the one
    /// thing the checker's type does not carry —
    /// [`CheckedTy::EnumCase`](nvs_types::ty::Ty::EnumCase) names the enum and
    /// the case, and ADR 0047 § 3's membership test needs the integer that
    /// pair stands for. [`ExprInfo::EnumCase`] answers the same question, but
    /// only for a case written as an *expression*; a case named in a **type**
    /// has no expression to record one against.
    enums: &'a EnumTable,
    /// Parallel to `block_insts`/`block_terms`: the [`BlockId`] each was
    /// created with, in creation order. [`IdGen::next_block`] hands out ids
    /// sequentially from zero, so a block's id and its position in these
    /// three vectors always coincide — that equality is what lets
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
    /// # Known gap
    ///
    /// A [`ArgOwnership::Transferred`] argument is not here — the callee's own
    /// exit sweep releases it, including on the callee's throwing edge — so
    /// the window between staging one and reaching the call still leaks if a
    /// *later* argument throws (`f($a, g())`, where `$a` was retained for the
    /// transfer and `g` throws). Closing it means a second entry kind on this
    /// stack, released on the error edge and *forgotten* on the normal one,
    /// plus one such forget at each of the three transferring call sites.
    owned_temporaries: Vec<ValueId>,
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
    /// initializer** — `int $x;`, whose type ADR 0037 fixes at the
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
    /// declared once per frame (ADR 0007 § 1), which is also why one flat map
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
    /// The span of the statement currently being lowered — the position half
    /// of that same backtrace frame.
    ///
    /// A cache of the span [`Self::lower_stmt`] just handed
    /// [`IdGen::next_stmt`], not a second table: ADR 0018's per-statement id
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
    /// side. [`Self::lower_stmts`] still asserts this list is empty once a
    /// statement has been lowered, which is now an internal-consistency check
    /// on the call sites rather than a refusal of the program.
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
    /// ADR 0053 § 4's state class, while this frame is a generator's
    /// `advance()` — `None` for every other function there is. See
    /// [`lower_generator`], which owns the whole transform.
    generator: Option<GenFrame>,
    /// Every ADR 0031 `fn` literal met in this body so far, in source order,
    /// each awaiting a function of its own — see [`lower_closure`]. Drained
    /// by whichever entry point built this frame, since a
    /// [`crate::ir::Function`] has nowhere to carry a second one.
    closures: Vec<PendingClosure>,
    /// One synthesized class per distinct ADR 0036 § 2 shape literal this
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
/// These are exactly the two shapes [`is_aliasing_read`] recognises as durable
/// storage, and exactly the two `nvs_types`' `check_inout_arg` accepts.
enum RefHolder {
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
/// array's own binding is re-pointed by every write (ADR 0007 § 5's
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
struct LoweredArgs {
    /// The argument values, positional.
    values: Vec<ValueId>,
}

/// One resolved signature's argument-shape, as [`Lowering::lower_call_args`]
/// needs it — owned rather than borrowed because every call site has to clone
/// it out of `self.exprs` before touching `self` mutably anyway.
struct ArgSig {
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
    /// Panics through [`lower_checked_ty`] for a parameter type this crate has
    /// no lowering for on a *non*-helper callee — the unchanged behaviour, and
    /// the one that has to stay: a compiled Novis function's parameter slot is
    /// typed, so there is nothing to fall back to.
    fn expectation(&self, index: usize, checked_types: &TypeInterner) -> Option<Ty> {
        let pty = self.param_tys[index];
        if self.helper && matches!(checked_types.get(pty), CheckedTy::Union(_)) {
            return None;
        }
        Some(lower_checked_ty(pty, checked_types))
    }

    /// Whether the argument at `index` binds by reference. Never true past the
    /// recorded parameters, and never consulted for a variadic tail at all:
    /// `lower_call_args` collects that tail into one array, which is a value
    /// and not a holder, so there is no position-onward rule to apply here the
    /// way `nvs_types::signatures::MethodSig::is_inout` has one. ADR 0063 R7
    /// keeps it that way for `Core` — nothing there is by-reference.
    fn is_inout(&self, index: usize) -> bool {
        self.inout.get(index).copied().unwrap_or(false)
    }
}

impl<'a> Lowering<'a> {
    pub(super) fn new(
        name: &str,
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
            cur_stmt_span: Span::at(src.id(), 0),
            foreach_seq: 0,
            switch_seq: 0,
            ref_locals: FxHashMap::default(),
            inout_elements: Vec::new(),
            pending_refs: Vec::new(),
            generator: None,
            closures: Vec::new(),
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
    pub(super) fn record_shape_class(
        &mut self,
        label: String,
        fields: Vec<String>,
        reprs: Vec<Ty>,
    ) {
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
            // ADR 0036 § 5: a shape literal's class has no methods, no
            // supertypes and no `implements`, it carries no attribute, and
            // every one of its slots is written by the literal that built it
            // — so there is nothing for a codec, a constructor arity or a
            // declared default to say.
            conforms: Vec::new(),
            methods: Vec::new(),
            codec: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
        });
    }
    pub(super) fn new_block(&mut self) -> BlockId {
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
    pub(super) fn lsb(&mut self) -> ValueId {
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
    pub(super) fn is_terminated(&self, b: BlockId) -> bool {
        self.block_terms[b.index() as usize].is_some()
    }
    /// Gives `b` its terminator, exactly once.
    ///
    /// # Panics
    ///
    /// Panics if `b` already has one — every call site only ever seals a
    /// block it just confirmed (via [`Self::is_terminated`]) is still open.
    pub(super) fn seal(&mut self, b: BlockId, term: Terminator) {
        let slot = &mut self.block_terms[b.index() as usize];
        assert!(
            slot.is_none(),
            "nvs-ir: bb{} sealed twice — bug in control-flow lowering",
            b.index()
        );
        *slot = Some(term);
    }
    pub(super) fn emit(&mut self, b: BlockId, ty: Ty, kind: InstKind) -> (ValueId, Ty) {
        let v = self.ids.next_value();
        self.block_insts[b.index() as usize].push(Inst {
            result: Some(v),
            ty: Some(ty),
            kind,
            on_error: None,
        });
        (v, ty)
    }
    /// [`Self::emit`] for a call-shaped instruction: the same append, plus
    /// [ADR 0002](../../../docs/adr/0002-error-propagation.md)'s error edge to
    /// a landing block built for this exact program point.
    ///
    /// Every instruction that returns a status goes through here, and nothing
    /// else does — see [`Inst::on_error`]'s own doc comment for that split.
    pub(super) fn emit_fallible(
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
    ///   already performs at an ordinary `return`. This is `nvs-codegen`'s
    ///   known gap 3 — a backend that leaks on every throw — closed.
    /// * **Reaching a `catch` in this same frame** releases nothing: the
    ///   handler and everything after it still name those locals, and the
    ///   binding each one has on the exception path travels through this
    ///   block's own entry in [`TryFrame::edges`] into the handler's phis.
    ///
    /// [`Self::owned_temporaries`] is released on **both** exits, ahead of
    /// either, and that is the one thing the asymmetry does not reach: a
    /// temporary has no `Env` entry for a handler to find it through, so the
    /// exception path is the last place anything can drop it. The values it
    /// holds are defined in the block that raised, which dominates this one,
    /// so the releases need no phi of their own.
    ///
    /// # Known gap
    ///
    /// The sweep covers what a producer actually **staged**: a call's
    /// arguments and receiver ([`Self::account_for_arg`]), and the operands
    /// and partial results of `.`, an interpolation and an `echo`. A site
    /// that still releases a fresh value inline instead — a normalized
    /// subscript key, a `match` subject — leaks it if something between the
    /// two throws. Each is one `own_temporary`/`release_temporaries_since`
    /// pair away, not a second mechanism. [`Self::owned_temporaries`] names
    /// the one hole that is *not* shaped like that.
    pub(super) fn landing_block(&mut self, env: &Env) -> BlockId {
        let b = self.new_block();
        for v in self.owned_temporaries.clone() {
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
                self.try_stack[at].edges.push((b, env.clone()));
                self.seal(b, Terminator::Catch { handler });
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
    /// from the span ADR 0018's per-statement id already carries.
    pub(super) fn frame_label(&self) -> String {
        let (line, _) = self.src.line_col(self.cur_stmt_span.start);
        format!("{}() at {}:{}", self.fn_label, self.src.name(), line + 1)
    }
    /// Appends a reserved [`InstKind::Safepoint`] marker to `b` — see that
    /// variant's own doc comment for the two call sites this has today
    /// (function entry, a loop's back edge) and why it defines no value.
    pub(super) fn emit_safepoint(&mut self, b: BlockId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Safepoint,
            on_error: None,
        });
    }
    /// Appends an [`InstKind::Retain`] on `v` to `b` — see the module docs'
    /// refcounting-policy section for when a call site actually wants one;
    /// this just emits the instruction unconditionally, since every caller
    /// has already checked [`Ty::is_refcounted`] itself.
    pub(super) fn emit_retain(&mut self, b: BlockId, v: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Retain { operand: v },
            on_error: None,
        });
    }
    /// Appends an [`InstKind::Release`] on `v` to `b` — see [`Self::emit_retain`].
    pub(super) fn emit_release(&mut self, b: BlockId, v: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Release { operand: v },
            on_error: None,
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
    /// The exception is ADR 0007 § 2's implicit `int`/`uint` → `float`
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
    pub(super) fn coerce(
        &mut self,
        cur: BlockId,
        v: ValueId,
        from: Ty,
        to: Ty,
        env: &mut Env,
    ) -> ValueId {
        match (from, to) {
            (a, b) if a == b => v,
            // ADR 0007 § 2's one implicit conversion: "`int` or `uint`
            // widening into a `float` position", which is exactly what a
            // declared type wider than the value's own is here. It is the
            // *same* conversion `$n as float` performs — exact, or throwing
            // above 2^53 where an `f64` stops representing every integer —
            // and § 2 says so outright, so it is the same fallible helper and
            // not a `fcvt_from_sint` this function could emit on its own.
            //
            // That is why this one takes `env`, and why every caller passes
            // it: a conversion carrying ADR 0002's error edge needs the
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
            // The union ADR 0007 § 4 gives integer `/` reaches a declared
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
            (_, Ty::Tagged) => self.emit(cur, Ty::Tagged, InstKind::Tag { operand: v }).0,
            (Ty::Tagged, _) => self.emit(cur, to, InstKind::Untag { operand: v }).0,
            _ => v,
        }
    }
    /// Appends an [`InstKind::RefStore`] to `b` — see that variant's own doc
    /// comment for the release-the-old half it performs itself, and
    /// [`Ty::Ref`] for the invariant the pair maintains.
    pub(super) fn emit_ref_store(&mut self, b: BlockId, slot: ValueId, value: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::RefStore { slot, value },
            on_error: None,
        });
    }
    /// Appends an [`InstKind::FieldSet`] to `b` — see
    /// [`Self::lower_reassignment`]'s property-target arm for the retain/
    /// release policy wrapped around this.
    pub(super) fn emit_field_set(
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
        });
    }
    /// Appends an [`InstKind::StaticSet`] to `b` — see
    /// [`Self::lower_store`]'s static-property arm for the retain/release
    /// policy wrapped around this, which is [`Self::emit_field_set`]'s
    /// unchanged.
    pub(super) fn emit_static_set(
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
        });
    }
    /// Appends an [`InstKind::ArraySet`] to `b`, yielding the array that now
    /// holds the entry — see that variant's own doc comment for the
    /// consume-one-reference-yield-one protocol, and
    /// [`Self::lower_reassignment`]'s `Index`-target arm for the retain policy
    /// wrapped around this and for where the result is written back to.
    pub(super) fn emit_array_set(
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
    pub(super) fn emit_array_append(
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
    pub(super) fn bind_local(
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
    pub(super) fn lower_stored(
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
    pub(super) fn bind_local_value(
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
    /// with no runtime cost at all — which is the whole point of ADR 0007
    /// § 5's copy-on-write being a *write*-side cost.
    ///
    /// Exactly three holders can be written back to, which are the three
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
    /// `ShapeProperty`, a `Property` or nothing, and only the third survives
    /// to here. A *hooked* property (ADR 0014 § 1) is a pair of accessors
    /// rather than a slot and an *erased* one (ADR 0036 § 4) is resolved by
    /// name at run time, so neither could be written back to at all;
    /// `nvs_types::expr::assign`'s `check_write_target` refuses both where the
    /// write is written, as `E0478` — PHP's own "indirect modification of
    /// overloaded property" — and `E0480`. Nothing recorded means the access
    /// was diagnosed instead, and a body holding a diagnostic is never
    /// lowered. That is the whole reason this function needs no rule for any
    /// of the three.
    pub(super) fn write_back_array(
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
                         the first two where the write is written — `E0478` for ADR 0014 \
                         § 1's pair of accessors, `E0480` for ADR 0036 § 4's erased \
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
            other => panic!(
                "nvs-ir lowers an array-element write only through a bare local, a \
                 compile-time-known property or a static property, because ADR 0007 § 5's \
                 copy-on-write separation has to be written back to whatever holds the array — \
                 not through {other:?}, which `nvs_types::expr::assign::check_write_target` \
                 refuses as `E0700` where it is written, so this body was not checked with the \
                 same table"
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
    pub(super) fn pointee_of(&self, name: &str) -> Ty {
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
    pub(super) fn pending_refs_mark(&self) -> usize {
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
    pub(super) fn flush_ref_writebacks(&mut self, mark: usize, env: &mut Env, cur: BlockId) {
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
    pub(super) fn write_back_holder(
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
    /// property alike, but a property with an ADR 0014 § 1 `get` hook is a
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
    pub(super) fn aliasing_read(&self, e: &Expr) -> bool {
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
        // does, and answering `false` here for one is what made
        // `array<string> $b = ($a);` release the array twice. The staged
        // lookup runs on both spans because a rewritten target's staged
        // sub-expression is recorded under the span it was *written* with.
        let e = e.unparenthesized();
        if self.staged(e.span).is_some() {
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
    pub(super) fn release_all_locals(&mut self, cur: BlockId, env: &Env, except: Option<&str>) {
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
    pub(super) fn finish(self) -> (Vec<BasicBlock>, Vec<Span>, Vec<Span>) {
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

/// A fractional literal's text as [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md)
/// § 1's `(mantissa, scale)`, with any exponent folded into the scale, or
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

/// ADR 0054 § 1's mantissa bound, restated here for the same reason
/// [`decimal_literal_parts`] is: `nvs_runtime::decimal`, which owns it, is not
/// a dependency of this crate.
const DECIMAL_MAX_MANTISSA: u128 = (1u128 << 96) - 1;

/// ADR 0054 § 1's scale bound — see [`DECIMAL_MAX_MANTISSA`].
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
/// function to do so, since this crate has no reverse dependency on that one) and reports ADR 0007
/// § 4's diagnostic before lowering ever runs — see its doc comment — so [`Lowering::lower_int_literal`] can
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
/// all. ADR 0007 § 3.2 makes both bindings' types mandatory and
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
/// no resolution to lower this way: ADR 0007 § 1 already requires it to be
/// spelled out in full, and this crate erases class identity entirely (see
/// [`Ty::Object`]'s own doc comment), so "is this atom a class name at all"
/// is the only question that matters here — which class doesn't need
/// answering until a call/`new` on it does, via [`lower_checked_ty`] instead.
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
/// ([`Ty::ClassDesc`]), which is what [`Lowering::lsb`] produces.
fn lower_decl_type(ty: &Type, exprs: &ExprTypeTable, checked_types: &TypeInterner) -> Ty {
    // The checker already resolved this exact annotation and recorded the
    // answer (`ExprTypeTable::declared_ty`) — take it whenever it exists, so a
    // name-shaped atom whose meaning depends on resolution comes out right.
    // ADR 0010's enum is the case that forces this: `Rank $r` is an integer
    // binding and `Dog $d` is an object one, and nothing in the AST tells the
    // two apart. The match below stays as the answer for an annotation the
    // checker never visited, where every atom is its own answer anyway.
    if let Some(id) = exprs.declared_ty(ty.span) {
        return lower_checked_ty(id, checked_types);
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
        // ADR 0047 § 5 again, for the one annotation shape that can be
        // answered without resolution. `TypeAtom::Member(..)` cannot: whether
        // it erases to a string, an int or an enum tag is exactly the question
        // the checker answered, so it takes the `declared_ty` shortcut above
        // or it is a bug.
        TypeKind::Atom(TypeAtom::StringLiteral(_)) => Ty::Str,
        TypeKind::Atom(TypeAtom::IntLiteral(_)) => Ty::Int,
        // ADR 0007 § 3's two `bool` singletons, which `nvs_types::ty::Ty::True`
        // records are that same rule read on `bool`'s two values — so they
        // erase to `bool`'s representation exactly as the two atoms above
        // erase to theirs.
        TypeKind::Atom(TypeAtom::True | TypeAtom::False) => Ty::Bool,
        // `object` — ADR 0007 § 3's opaque top of every class type, which is
        // the same pointer a named class is. See `erase_checked_ty`, which is
        // the arm this annotation actually takes whenever the checker visited
        // it.
        TypeKind::Atom(TypeAtom::Name(..) | TypeAtom::Object) => Ty::Object,
        // ADR 0031 § 4's one closure type. Its *representation* is an object
        // — see the `ExprKind::Fn` arm of `Lowering::lower_expr`, which
        // synthesizes one class per literal to hold the captured environment
        // — so it erases here exactly the way a class name does.
        TypeKind::Atom(TypeAtom::Callable) => Ty::Object,
        TypeKind::Atom(TypeAtom::Array(_)) => Ty::Array,
        // `mixed` — ADR 0007 § 3. See `Ty::Tagged`'s own doc comment for
        // exactly how much this representation does and doesn't do yet: a
        // local/parameter/return/call-argument round-trips, nothing else.
        TypeKind::Atom(TypeAtom::Mixed) => Ty::Tagged,
        TypeKind::Paren(inner) => lower_decl_type(inner, exprs, checked_types),
        // Both admit more than one runtime shape, so both are tagged — see
        // `Ty::Tagged`. Reached only for an annotation the checker never
        // visited; everything it did visit takes the `declared_ty` shortcut
        // above and goes through `lower_checked_ty`, which is the *narrower*
        // answer since ADR 0047 § 5's fold landed there: it folds `"a"|"b"`
        // back to the one representation its members share, which needs the
        // resolved members and so cannot be answered from the AST alone.
        TypeKind::Nullable(_) | TypeKind::Union(_) => Ty::Tagged,
        other => panic!(
            "nvs-ir only lowers bool/int/uint/float/void/string/bytes/array/`?T`/a union/`object`/\
             a plain class name as a declared type — got {other:?}; see the crate docs' known gaps"
        ),
    }
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
/// survive this translation. `Enum` does *not* join it: ADR 0010 makes an enum
/// a closed integer type, so it lowers to [`Ty::Enum`] carrying the backing
/// type `nvs_types::ty::Ty::Enum` already knows (see that variant for why the
/// backing rides in the checker's type rather than in a side table). `String` erases to [`Ty::Str`] and
/// `Bytes` to [`Ty::Bytes`] — the same representations [`lower_decl_type`]
/// already gives a local/parameter/return type spelled directly in source —
/// see [`crate::lower`]'s module docs for the retain policy this now needs at
/// a call-argument/return/property-field boundary, which
/// [`Lowering::bind_local`], [`Lowering::lower_call_args`] and
/// `StmtKind::Return`'s own arm all apply via [`is_aliasing_read`].
///
/// # Panics
///
/// Panics naming the unsupported shape for anything outside this slice's
/// scope: either qualified (`tainted`/`secret`) string or bytes variant,
/// `object`, an intersection, or either of
/// `never`/`iterable` — neither of these
/// has an IR representation yet (see the crate docs' known gaps). `mixed`
/// erases to [`Ty::Tagged`] — see that variant's own doc comment for exactly
/// how much this boundary does and doesn't do with one yet. A **union** never
/// panics: it is [`Ty::Tagged`] unless every member erases to one and the same
/// representation, in which case it is that one — ADR 0047 § 5's "zero
/// additional runtime representation", which is why a member outside this
/// scope is asked through [`erase_checked_ty`] rather than asserted.
/// The per-option defaults recorded for the options-bag parameter at `index` —
/// `nvs_types::core_lib` synthesizes exactly one `ConstArg::Options` entry per
/// bag, so a bag parameter always has one.
///
/// # Panics
///
/// Panics if that parameter's recorded default is absent or is not a bag,
/// which would mean the signature table and the parameter type disagree about
/// what the parameter is.
fn options_defaults(
    defaults: &[Option<nvs_types::ConstArg>],
    index: usize,
) -> &[(String, nvs_types::ConstArg)] {
    match defaults.get(index) {
        Some(Some(nvs_types::ConstArg::Options(options))) => options,
        other => panic!(
            "nvs-ir: parameter {index} is an options bag but its recorded default is {other:?} \
             — nvs_types::core_lib is trusted to record one `ConstArg::Options` per bag"
        ),
    }
}

/// The label the class synthesized for an ADR 0036 § 2 shape literal carries
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
pub(super) fn shape_class_label(sorted_fields: &[String]) -> String {
    format!("$shape{{{}}}", sorted_fields.join(","))
}

fn lower_checked_ty(id: TypeId, checked_types: &TypeInterner) -> Ty {
    erase_checked_ty(id, checked_types).unwrap_or_else(|| {
        panic!(
            "nvs-ir only lowers a resolved call's bool/int/uint/float/void/string/bytes/array/\
             class/object/shape/enum/mixed/null/union parameter or return type — got {:?}; see \
             the crate docs' known gaps",
            checked_types.get(id)
        )
    })
}

/// [`lower_checked_ty`], as a question rather than an assertion: `None` where
/// the checker's type has no representation in this crate yet.
///
/// Split out for the [`CheckedTy::Union`] arm alone. Folding a union to the
/// one representation its members share means asking each member for its own,
/// and a member outside this slice's scope must answer that question rather
/// than panic — a `object|A` union is still [`Ty::Tagged`], the same answer it
/// gave before the fold existed, not a new internal error.
fn erase_checked_ty(id: TypeId, checked_types: &TypeInterner) -> Option<Ty> {
    Some(match checked_types.get(id) {
        CheckedTy::Bool => Ty::Bool,
        CheckedTy::Int => Ty::Int,
        CheckedTy::Uint => Ty::Uint,
        CheckedTy::Float => Ty::Float,
        CheckedTy::Decimal => Ty::Decimal,
        CheckedTy::Void => Ty::Void,
        CheckedTy::String => Ty::Str,
        CheckedTy::Bytes => Ty::Bytes,
        // ADR 0024 § 1 and ADR 0033 § 1: `tainted` and `secret` are two
        // independent bits on the *checker's* type and add **zero** runtime
        // representation, exactly as ADR 0047 § 5's literal types do above. So
        // all six qualified atoms erase to the base they share a tag and an
        // allocation with, and everything below this boundary sees a plain
        // `string` or `bytes`.
        //
        // What that costs is one thing, and it is paid for: a lowering
        // decision that genuinely depends on a qualifier cannot read it back
        // here. ADR 0033 § 5's constant-time `==` is the only such decision,
        // and the checker records it at the comparison instead
        // (`nvs_types::expr_table::ExprInfo::SecretEquality`).
        CheckedTy::TaintedString | CheckedTy::SecretString | CheckedTy::SecretTaintedString => {
            Ty::Str
        }
        CheckedTy::TaintedBytes | CheckedTy::SecretBytes | CheckedTy::SecretTaintedBytes => {
            Ty::Bytes
        }
        // A shape joins them: ADR 0036 § 2 makes a shape value an ordinary
        // refcounted instance with no methods and no name of its own, so its
        // representation is the object pointer a class already has. What the
        // erasure drops is the field list, and nothing below this boundary
        // wants it — a field read carries its own slot index, resolved where
        // the type still existed (`InstKind::SlotGet`).
        //
        // Plain `object` is the same representation with the field list never
        // present in the first place — ADR 0007 § 3's opaque top of every
        // class type. A member access through one is ADR 0036 § 4's fully
        // erased half: the checker records the field's *name* and nothing
        // else, and `InstKind::SlotGet`/`SlotSet` find it on the concrete
        // descriptor or throw.
        CheckedTy::Class(..) | CheckedTy::Callable | CheckedTy::Shape(_) | CheckedTy::Object => {
            Ty::Object
        }
        // ADR 0047 § 5: a literal type and an enum-case type add **zero**
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
        // the two arms below are the whole of ADR 0066's representation.
        CheckedTy::Null => Ty::Null,
        // ADR 0047 § 5 again, and the whole of what it means: there is no
        // second representation, so a union whose members all erase to one
        // `Ty` **is** that `Ty`. `"a"|"b"` is a `Ty::Str`, `1|2` a `Ty::Int`
        // and `Mode::Read|Mode::Write` the enum's own tag — the erasure the
        // three atom arms above already perform, applied once more to the
        // union that collects them. Everything else still admits more than
        // one runtime shape and is tagged, `?T` (`Union([Null, T])`) included:
        // `Ty::Null` and `Ty::Str` are two representations, not one.
        CheckedTy::Union(members) => {
            let mut shared: Option<Ty> = None;
            for member in members {
                match (erase_checked_ty(*member, checked_types), shared) {
                    (Some(ty), None) => shared = Some(ty),
                    (Some(ty), Some(seen)) if ty == seen => {}
                    _ => return Some(Ty::Tagged),
                }
            }
            shared.unwrap_or(Ty::Tagged)
        }
        _ => return None,
    })
}

/// What an assignment stores, once its target's address is settled.
///
/// [`Lowering::lower_store`] lowers the target either way; only where the
/// value comes from differs. A read-modify-write (`$x += 1`, `$x++`) has
/// combined its value before the store runs and has no expression left to
/// hand over — see [`Lowering::lower_read_modify_write`].
pub(super) enum Stored<'a> {
    /// `$t = e;` — lowered against the target's declared representation, and
    /// judged by [`Lowering::aliasing_read`] like any other read.
    Expr(&'a Expr),
    /// A value already in hand. Always a **fresh producer**: the only thing
    /// that builds one is a binary operator's result, which no durable slot
    /// holds yet.
    Value(ValueId, Ty),
}

fn is_aliasing_read(kind: &ExprKind) -> bool {
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

/// The one method an [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)
/// closure's environment class answers, as the method table spells it.
pub(crate) const FN_INVOKE: &str = "invoke";

/// The reserved **first** field of every closure's environment class: how many
/// parameters [`FN_INVOKE`] declares, not counting the receiver.
///
/// [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
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
/// [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md) § 1
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

/// The [`FN_PARAM_TAGS`] nibble for a parameter no argument can be wrong for.
///
/// Deliberately not a `nvs_runtime::Tag` discriminant — the roster runs to
/// eleven, so twelve is free and can never be mistaken for a tag a value
/// actually carries.
pub const FN_PARAM_TAG_ANY: u8 = 12;

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
        // ADR 0010's enum travels as its backing integer, tag included, so
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

/// How many `array<…>` levels one [`array_element_tags`] word describes: a
/// [`param_tag_nibble`] each, packed into a `u64`.
///
/// `nvs_types::expr::operators` refuses a target that nests deeper
/// (`E0711`) — that module's `ARRAY_ELEMENT_TAG_LEVELS` is the same `64 / 4`
/// and the rule's one home — so a deeper annotation never reaches this crate.
pub const ARRAY_ELEMENT_TAG_LEVELS: usize = u64::BITS as usize / 4;

/// The word ADR 0007 § 2's `array<T> as array<U>` row carries to the runtime:
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
    // ADR 0066's `as ?array<U>` asks the same question of the same target, and
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

/// One lowered body, plus everything the ADR 0031 closures inside it
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
