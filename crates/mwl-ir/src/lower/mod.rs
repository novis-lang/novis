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
//! | [`call`] | argument ownership, options-bag flattening, a `&$x` argument staged and written back |
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
//! `mwl_types::check_program` already required it to be definitely assigned
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

use mwl_diagnostics::{SourceFile, Span};
use mwl_syntax::ast::{
    ArrayItem, AssignOp, BinaryOp, Block, CallArgs, CatchClause, ClassMemberKind,
    DestructureElement, DestructureTarget, Expr, ExprKind, FnBody, FnExpr, ForeachBinding,
    IncDecOp, MatchArm, MethodMember, Modifier, NamespaceDecl, NewTarget, ObjectLiteralField, Stmt,
    StmtKind, StringPart, SwitchCase, Type, TypeAtom, TypeKind, UnaryOp as AstUnaryOp,
};
use mwl_types::EnumTable;
use mwl_types::expr_table::{ArgSlot, ExprInfo, ExprTypeTable, ForeachDrive};
use mwl_types::layout::ClassLayoutTable;
use mwl_types::ty::{Ty as CheckedTy, TypeId, TypeInterner};
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
mod exception;
mod expr;
mod generator;
mod stmt;

// `call`, `control`, `expr` and `stmt` only add methods to the one
// `impl Lowering` below, so they export nothing to import. The three named
// here also carry free items this module and its siblings call.
use self::{closure::*, exception::*, generator::*};

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
    /// comment records why MWL takes the meaning PHP's own warning points at.
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
/// The two conventions MWL has, and the one thing that differs between
/// lowering an [`InstKind::Call`] and an [`InstKind::CoreCall`] beyond which
/// instruction is emitted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ArgOwnership {
    /// The callee owns it — an MWL method or constructor, whose parameter is
    /// bound into its own `Env` like a local and released at its exit sweep.
    /// The caller therefore retains an aliasing refcounted argument first, so
    /// the pair balances.
    Transferred,
    /// The callee borrows it — every ADR 0002 helper, including a `Core`
    /// member, which receives a `&[Value]` and releases nothing. No retain,
    /// and the caller keeps owning what it passed; `mwl_stdlib`'s own docs own
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
    /// `None` for the frame pushed around a `catch` clause's *own body*: that
    /// body still owes this region's `finally` on the way out
    /// ([`Lowering::run_pending_finallys`]), but a throw inside it belongs to
    /// the enclosing region rather than to the clause list it is already
    /// running — so [`Lowering::landing_block`] looks straight past it.
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
/// The constant is translated here rather than in `mwl-codegen` because
/// `mwl_types::ConstArg` is everything a *signature* can carry and
/// `mwl_types::FieldDefault` is only what a written property declaration can
/// reach: a variant outside that set is unreachable rather than lossy, since
/// `mwl_types::defaults::eval_property_default` cannot produce one.
fn property_defaults(
    label: &str,
    layout: &mwl_types::ClassLayout,
    exprs: &ExprTypeTable,
) -> Vec<(usize, mwl_types::FieldDefault)> {
    use mwl_types::FieldDefault;

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
fn field_default(value: &mwl_types::ConstArg) -> Option<mwl_types::FieldDefault> {
    use mwl_types::{ConstArg, FieldDefault};

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
/// stay aligned with [`mwl_types::ClassLayout::fields`], so a slot nothing
/// claims — a class whose signature never reached
/// [`ExprTypeTable::property_types`], or a type this crate does not represent
/// — is [`Ty::Tagged`], which `mwl-codegen` maps to "unchecked" rather than
/// to a tag that would refuse a legal write.
fn field_reprs(
    label: &str,
    layout: &mwl_types::ClassLayout,
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

/// Lowers a whole checked **program**: every class method that has a body in
/// any of `files`, plus the *entry* file's own top-level statements as one
/// script frame named `script`.
///
/// This is what a caller with a resolved `require`/`autoload` graph wants —
/// [`lower_file`] is the one-file spelling, and [`lower_method`] and
/// [`lower_script`] stay public for the narrower "lower exactly this one
/// thing" cases the tests use.
///
/// **`files[0]` is the entry point, and it is the only file that gets a
/// script frame.** N files cannot all be `script`, and ADR 0021's "no
/// isolation" is already honoured by the compile-time walk rather than at run
/// time: a `require` statement lowers to nothing (`lower_expr_stmt`), because
/// the target's declarations are in this same program by the time the site is
/// reached. So a non-entry file contributes its *declarations* only, and a
/// bare top-level statement written in a `require`d file is silently not run
/// — the crate's own module doc records it as a known gap, since making it
/// run means giving each file a frame and calling it from the `require`
/// site, which is the isolation question ADR 0006 owns.
/// `mwl_hir::resolve_program`'s entry-first order is what makes indexing
/// position zero right.
///
/// Each method is named with the `Class::method` label
/// `mwl_types::expr_table::ExprTypeTable::method_label` recorded for its
/// declaration, which is the *same* label a call's
/// [`InstKind::Call::target`](crate::ir::InstKind::Call) is rendered from —
/// see that accessor's own doc comment for why the label is spelled in
/// `mwl-types` rather than here. A method whose declaration has no recorded
/// label is skipped: nothing can call it by a name that was never resolved,
/// so lowering it would only produce an unreachable function.
///
/// Interfaces and enums are not walked. An `interface` method may carry a
/// body ([ADR 0043](../../../docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)),
/// but reaching one needs the `by`-delegation resolution and the dispatch
/// that land with M4's object model; there is nothing to call it from today.
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
    files: &[mwl_types::ProgramFile<'_>],
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
                StmtKind::ClassDecl(mwl_syntax::ast::ClassDecl { members, .. })
                | StmtKind::InterfaceDecl(mwl_syntax::ast::InterfaceDecl { members, .. }) => {
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
                                if mwl_syntax::ast::is_generator_body(body) {
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
                            // same way, under the label `mwl_types` recorded
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
    let entry = files
        .first()
        .expect("a program has at least its entry file");
    let lowered = lower_script(script, entry.stmts, entry.src, exprs, checked_types, enums);
    functions.push(lowered.function);
    functions.extend(lowered.closures);
    synthesized.extend(lowered.classes);
    // The functions with no source text — see
    // `synthesized_exception_constructors`. Emitted unconditionally: the
    // exception tree is in every program's class table, so a unit that omitted
    // these would be one where `new LogicError(…)` names a missing target.
    functions.extend(synthesized_exception_constructors());

    // Copied straight across rather than recomputed: `mwl-types` already
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
            // slot for is dropped rather than mis-indexed: `mwl_types::derive`
            // has already reported the declaration that caused it (a promoted
            // parameter is that module's gap 2), and guessing a slot here
            // would write another property's value under this one's key.
            codec: exprs.codec(label).map_or_else(Vec::new, |codec| {
                codec
                    .fields
                    .iter()
                    .filter_map(|field| {
                        Some(mwl_types::CodecField {
                            key: field.key.clone(),
                            slot: layout.slot_of(&field.property)?,
                            // A field whose declaration named no constructor
                            // parameter is dropped for the same reason a
                            // slotless one is: `mwl_types::derive` has already
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
    // therefore no `mwl_types::layout` entry — `mwl-ir` synthesizes both the
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
        &[mwl_types::ProgramFile { src, stmts }],
        exprs,
        checked_types,
        enums,
        layouts,
    )
}

/// Lowers `m` — which must have a body, and whose body must stay within this
/// slice's supported statement/expression shapes (see the crate docs) — to a
/// [`Function`] named `name`. `exprs`/`checked_types` are the
/// `mwl_types::check_program` run's own typed-expression table and type
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
/// to have already run `mwl_types::check_program` — with the very `exprs`/
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
    // unconditionally, the same way `mwl_types::check.rs`'s `check_method`
    // seeds `$this` into its own `LocalScope` regardless of a `static`
    // modifier (see that function's own comment for why: a static method's
    // body referencing `$this` is a distinct, unrelated diagnostic, not this
    // crate's concern). `mwl-ir` never lowers a free function — every
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
    // `mwl_runtime::object`'s module docs, which own the decision.
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
        // the caller has always passed. `mwl_types::check` binds the body's
        // own name at `array<` that `>` for the same reason.
        let ty = match p.variadic {
            true => Ty::Array,
            false => lower_decl_type(decl_ty, exprs, checked_types),
        };
        // +1: index 0 is always the implicit receiver seeded above.
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        // `&$x` — the incoming slot holds the address of one caller-staged
        // `Value` cell rather than a value of the declared type, so the
        // binding is a `Ty::Ref` and the declared type is remembered as the
        // *pointee*: every read of the parameter becomes an
        // `InstKind::RefLoad` at that type and every write an
        // `InstKind::RefStore`. See `Ty::Ref` for the whole representation.
        let bound_ty = if p.by_ref {
            low.ref_locals.insert(pname.clone(), ty);
            Ty::Ref
        } else {
            ty
        };
        let (v, _) = low.emit(entry, bound_ty, InstKind::Param(index));
        env.insert(pname, (v, bound_ty));
        param_tys.push(bound_ty);
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

/// Lowers one of `p`'s [ADR 0014](../../../docs/adr/0014-property-observer.md)
/// § 1 property hooks to a [`Function`] named `name` — which must be the
/// label `mwl_types::signatures::hook_label` spelled for it, since a `set`
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
///   declaration or, left implicit, by `mwl_types::HOOK_VALUE_PARAM`.
/// - **The short `=> expr;` body** means "return this" for `get` and "store
///   this" for `set`, matching PHP 8.4. The store is emitted here rather than
///   desugared into the AST, because there is no AST node to desugar into.
/// - A `set` hook's store names the *backing slot* directly. Inside a hook,
///   the property is always its own storage — `mwl_types::Ctx::current_hook`
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
    p: &mwl_syntax::ast::PropertyMember,
    hook: &mwl_syntax::ast::PropertyHook,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> Lowered {
    use mwl_syntax::ast::{PropertyHookBody, PropertyHookKind};

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
            None => (mwl_types::HOOK_VALUE_PARAM.to_owned(), prop_ty),
        };
        let (v, _) = low.emit(entry, pty, InstKind::Param(1));
        env.insert(pname, (v, pty));
        param_tys.push(pty);
    }

    match &hook.body {
        Some(PropertyHookBody::Block(block)) => low.lower_stmts(&block.stmts, &mut cur, &mut env),
        Some(PropertyHookBody::Expr(e)) if is_set => {
            let class = mwl_types::signatures::hook_label_class(name)
                .unwrap_or_else(|| {
                    panic!(
                        "mwl-ir: `{name}` is not a hook label, so the class whose slot a \
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
///   `mwl_types::check`'s own script frame, which seeds `$this` only when
///   there is an enclosing class.
/// - **The return representation is [`Ty::Tagged`].** A top-level `return`
///   hands a value back to whatever `require`d the file, and
///   [ADR 0021](../../../docs/adr/0021-single-file-inclusion-construct.md)
///   types that boundary `mixed`. A file that never returns falls through to
///   the same `Terminator::Return(None)` seal `lower_method` uses.
///
/// Declarations are skipped rather than lowered: a class's methods are
/// lowered separately, one [`lower_method`] call each. A
/// `namespace X { ... }` block's *own* top-level statements are not skipped
/// — a namespace scopes names, not storage, so they belong to this same one
/// frame, which is exactly how `mwl_types::check::check_stmts` threads its
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
    /// The same `mwl_types::check_program` run's type interner — needed to
    /// translate a [`TypeId`] recorded in `exprs` into this crate's own
    /// [`Ty`] via [`lower_checked_ty`].
    checked_types: &'a TypeInterner,
    /// The same run's [`EnumTable`]: every declared enum's backing type and
    /// its cases' constant values.
    ///
    /// Threaded beside `checked_types` because a case's *value* is the one
    /// thing the checker's type does not carry —
    /// [`CheckedTy::EnumCase`](mwl_types::ty::Ty::EnumCase) names the enum and
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
    /// always wins, and never removed: MWL has no shadowing and a name is
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
    /// Every `&$x` parameter this frame declares, by name, mapped to the
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
    /// Every `foreach (… as &$v)` whose body is being lowered right now,
    /// innermost last — see [`ByRefElement`] and [`Lowering::lower_foreach`].
    ///
    /// A stack rather than a map because two nested by-reference loops may be
    /// in scope at once and only the innermost binding of a name is visible;
    /// pushed before the body is lowered and popped after, so it is empty for
    /// every frame that has no such loop open and the write-through hook
    /// costs one `is_empty` there.
    by_ref_elements: Vec<ByRefElement>,
    /// By-reference arguments staged for the call currently being lowered,
    /// awaiting their copy-back — see [`Ty::Ref`] and
    /// [`Self::flush_ref_writebacks`].
    ///
    /// # Why this is a frame field rather than a return value
    ///
    /// The copy-back re-points the argument's *holder*, which for a local
    /// means rebinding it in [`Env`]. The staging is parked here and drained
    /// at the enclosing statement, so every argument of one call is written
    /// back at one point rather than at whichever operand happened to be
    /// lowered last.
    ///
    /// **Known gap.** A read of the holder that is sequenced *after* the call
    /// but still inside the same statement (`$n + Adder::bump($n)`) therefore
    /// sees the pre-call value, where PHP would see the written-back one.
    /// [`Self::lower_stmt`] asserts this list is empty once a statement has
    /// been lowered, so such a program panics naming the shape rather than
    /// silently losing the write. This used to be a *scoping* limit —
    /// [`Self::lower_expr`] held an `&Env` and had no binding it could
    /// re-point — and is not any more: it holds an `&mut Env` since an
    /// increment started lowering in value position. What closing it now
    /// needs is a rule for *where* the copy-back lands, which is the
    /// sequence-point question this list currently answers by deferring.
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
    /// `mwl_types`' `check_by_ref_arg` has already proven is exactly the
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
/// storage, and exactly the two `mwl_types`' `check_by_ref_arg` accepts.
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

/// One open `foreach (… as &$v)`: what a write to `$v` inside its body has to
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
struct ByRefElement {
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
    /// Which parameters are declared `&$x`, positional.
    by_ref: Vec<bool>,
    /// Whether the last parameter is `...$x` — in which case `param_tys`'
    /// last entry is the type *each* trailing argument is checked against,
    /// and `Lowering::lower_variadic_tail` turns all of them into the one
    /// array that parameter actually receives.
    variadic: bool,
    /// Each parameter's evaluated default, positional — `None` for one every
    /// call has to supply. See [`Lowering::lower_call_args`] for what an
    /// omitted trailing argument becomes.
    defaults: Vec<Option<mwl_types::ConstArg>>,
    /// Which parameter each **written** argument fills, in written order —
    /// one entry per `mwl_syntax::ast::Arg`, copied off
    /// [`mwl_types::expr_table::ResolvedCall::arg_slots`].
    ///
    /// This crate cannot re-derive it and the field exists for that reason
    /// alone: a `name:` resolves against `MethodSig::param_names`, which no
    /// pass below the checker holds. For an all-positional list it is the
    /// identity mapping and says nothing new, which is why
    /// [`Lowering::lower_call_args`] reads it uniformly rather than branching
    /// on whether the call wrote a name at all.
    arg_slots: Vec<ArgSlot>,
    /// Whether the callee is a Tier 0 `Core` member reached through the ADR
    /// 0002 helper convention, rather than a compiled MWL function.
    ///
    /// One thing turns on it, and it is a real difference rather than a
    /// convenience: a helper's parameter slot is a whole `mwl_runtime::Value`
    /// — a tag plus a payload — and `mwl-codegen`'s `emit_helper` writes each
    /// argument's tag from the *argument's* own representation, never from the
    /// parameter's. So a `Core` parameter whose declared type has no single IR
    /// representation (a union — `hasKey(array<T> $a, int|string $key)`) is
    /// still perfectly lowerable: the argument keeps its own type and the
    /// helper decodes by tag. A compiled MWL function's parameter slot is
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
    /// ordinary MWL call convention.
    fn of(call: &mwl_types::expr_table::ResolvedCall) -> Self {
        Self {
            param_tys: call.param_tys.clone(),
            by_ref: call.by_ref.clone(),
            variadic: call.variadic,
            defaults: call.defaults.clone(),
            arg_slots: call.arg_slots.clone(),
            helper: false,
        }
    }

    /// The same shape, for a callee reached through the helper convention —
    /// see [`Self::helper`].
    fn of_helper(call: &mwl_types::expr_table::ResolvedCall) -> Self {
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
    /// the one that has to stay: a compiled MWL function's parameter slot is
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
    /// way `mwl_types::signatures::MethodSig::is_by_ref` has one. ADR 0063 R7
    /// keeps it that way for `Core` — nothing there is by-reference.
    fn is_by_ref(&self, index: usize) -> bool {
        self.by_ref.get(index).copied().unwrap_or(false)
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
            by_ref_elements: Vec::new(),
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
    /// `mwl_runtime::mwl_object_slot_set` gets to check a write against.
    ///
    /// A label already recorded is **merged**, not skipped: `$shape{x}` is
    /// named for its field names alone (see [`shape_class_label`]), so
    /// `{x: 1}` and `{x: "s"}` are one class with two disagreeing slot types.
    /// A slot the two spell differently degrades to [`Ty::Tagged`], which
    /// `mwl-codegen` reads as "no fixed tag, do not check" — picking whichever
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
    /// `mwl_runtime::OBJ_CLASS_OFFSET`, emitted into the **entry block** and
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
    /// Panics for a frame with neither — the script frame. `mwl_types` refuses
    /// `static::`/`new static()` outside a class (`E_UNDEFINED_CLASS`), so
    /// lowering never reaches this on one.
    pub(super) fn lsb(&mut self) -> ValueId {
        if let Some(v) = self.lsb {
            return v;
        }
        let this = self.this.unwrap_or_else(|| {
            panic!(
                "mwl-ir: `{}` names `static` but has neither a receiver nor a called class — \
                 mwl_types is expected to have refused that outside a class",
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
            "mwl-ir: bb{} sealed twice — bug in control-flow lowering",
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
    ///   already performs at an ordinary `return`. This is `mwl-codegen`'s
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
            // one that reads the runtime tag. `mwl_types` is what decided the
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
    /// (`mwl_runtime::array`'s *the append is the one array write with a fault
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
        // `$v = e` where `$v` is a `foreach (… as &$v)` binding writes the
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
    /// `mwl_types::expr::assign`'s `check_write_target` refuses both where the
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
                // `$v[0] = e` where `$v` is a `foreach (… as &$v)` binding:
                // the separated row is what the entry now holds.
                self.write_through_element(*cur, env, &name, written, ty);
                env.insert(name, (written, ty));
            }
            ExprKind::PropertyAccess { object, .. } => {
                let Some(ExprInfo::Property { class, name, .. }) = self.exprs.lookup(base.span)
                else {
                    unreachable!(
                        "mwl-ir reaches an element write back through the property at {:?} \
                         carrying anything but an `ExprInfo::Property` only if a gate above \
                         it let one through, and none can — this is an invariant, not a gap. \
                         A `PropertyAccess` span carries exactly one of three entries or \
                         none: `mwl_types::expr::members::check_property_member` records \
                         `HookedProperty`, `ShapeProperty` or `Property` on every access it \
                         returns a resolved type from, and reports a diagnostic on every \
                         path it records nothing on, so a body reaching here at all had an \
                         entry. `mwl_types::expr::assign::check_write_target` then refuses \
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
                "mwl-ir lowers an array-element write only through a bare local, a \
                 compile-time-known property or a static property, because ADR 0007 § 5's \
                 copy-on-write separation has to be written back to whatever holds the array — \
                 not through {other:?}, which `mwl_types::expr::assign::check_write_target` \
                 refuses as `E0700` where it is written, so this body was not checked with the \
                 same table"
            ),
        }
    }
    /// The declared type behind the `&$name` parameter `name` — the pointee
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
                "mwl-ir: `${name}` is bound as a `Ty::Ref` but no pointee representation was \
                 recorded for it — bug in lower_method's by-reference parameter binding"
            )
        })
    }
    /// Emits every staged by-reference argument's copy-back, in staging order,
    /// and clears the list.
    ///
    /// Each one is an [`InstKind::RefLoad`] out of the slot the call may have
    /// written, then a re-point of the holder ([`Self::write_back_holder`]).
    /// Called at the end of the *statement* that contained the call, which is
    /// where an `&mut Env` exists at all — see [`Self::pending_refs`] for what
    /// that costs and [`Ty::Ref`] for the whole representation.
    pub(super) fn flush_ref_writebacks(&mut self, env: &mut Env, cur: BlockId) {
        for staged in std::mem::take(&mut self.pending_refs) {
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
                // `f(&$v)` where `$v` is a `foreach (… as &$v)` binding: what
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
    /// `mwl_types`' resolution can tell the two apart, which is why this is a
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
        // (`mwl_syntax::ast::Expr::unparenthesized`), so every question below
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
                        "mwl-ir: bb{} was never sealed with a terminator",
                        id.index()
                    )
                }),
            })
            .collect();
        (blocks, stmt_spans, edge_spans)
    }
}

/// Reads a numeric-literal span's text with `_` digit separators stripped.
fn clean_digits(src: &SourceFile, span: mwl_diagnostics::Span) -> String {
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
/// Deliberately a second implementation of `mwl_types::expr`'s
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
/// [`decimal_literal_parts`] is: `mwl_runtime::decimal`, which owns it, is not
/// a dependency of this crate.
const DECIMAL_MAX_MANTISSA: u128 = (1u128 << 96) - 1;

/// ADR 0054 § 1's scale bound — see [`DECIMAL_MAX_MANTISSA`].
const DECIMAL_MAX_SCALE: u8 = 28;

/// Cooks a plain, non-interpolated string literal's span — `mwl_syntax::ast::ExprKind::Str`'s
/// own doc comment: a single-quoted string, or a double-quoted/heredoc/nowdoc string with no
/// interpolation in it — into its runtime bytes.
///
/// All three spellings live in [`mwl_types::string_lit::cook_string_literal`], which owns the
/// escape grammar and the flexible-heredoc strip; this wrapper exists only so the call sites
/// below keep reading as one local name. Sharing rather than duplicating is the same call
/// `crate::string_lit`'s own module docs already record for the double-quoted half: the checker
/// has to diagnose exactly the cooking that happens here, so one routine has to perform both —
/// unlike [`int_literal_digits`], where the crate boundary runs the other way.
fn cook_str_literal(src: &SourceFile, span: mwl_diagnostics::Span) -> String {
    mwl_types::string_lit::cook_string_literal(src, span)
}

/// Splits a cooked integer-literal span into the radix its prefix names and
/// the digit run to parse against it — `mwl_syntax::Lexer::lex_number` emits
/// one `IntLiteral` token for all four forms (`0x…`/`0o…`/`0b…`, or a plain
/// decimal run; a legacy leading-zero octal spelling like PHP's `0755` is
/// deliberately *not* one of them, so `0755` lexes as decimal 755 with no
/// prefix to strip), and this is the one place that distinction has to be
/// undone before `str::from_str_radix` can parse the value.
/// `mwl_types::expr::literals::infer_int_literal` range-checks the same digits (mirroring this
/// function to do so, since this crate has no reverse dependency on that one) and reports ADR 0007
/// § 4's diagnostic before lowering ever runs — see its doc comment — so [`Lowering::lower_int_literal`] can
/// treat an out-of-range literal as unreachable input, the same "trusts `mwl_types::check_program`
/// already ran" contract every other panic in this crate relies on.
fn int_literal_digits(src: &SourceFile, span: mwl_diagnostics::Span) -> (u32, String) {
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
/// `mwl_syntax`'s parser already reported the omission (the `None` here is the
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
            "mwl-ir: a `foreach` {which} binding reached lowering with no declared type — \
             mwl_syntax already reports that omission, so this file should never have been \
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
        // ADR 0007 § 3's two `bool` singletons, which `mwl_types::ty::Ty::True`
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
            "mwl-ir only lowers bool/int/uint/float/void/string/bytes/array/`?T`/a union/`object`/\
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
/// from `mwl_types`' own interner, not from a `Type` AST node this crate can
/// lower directly (there may be no local `Type` node at all, e.g. an
/// inherited method's parameter declared on a different class's source).
/// `Class` erases to [`Ty::Object`], same as [`lower_decl_type`]'s `Name`
/// case — see that variant's own doc comment for why identity doesn't need to
/// survive this translation. `Enum` does *not* join it: ADR 0010 makes an enum
/// a closed integer type, so it lowers to [`Ty::Enum`] carrying the backing
/// type `mwl_types::ty::Ty::Enum` already knows (see that variant for why the
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
/// `mwl_types::core_lib` synthesizes exactly one `ConstArg::Options` entry per
/// bag, so a bag parameter always has one.
///
/// # Panics
///
/// Panics if that parameter's recorded default is absent or is not a bag,
/// which would mean the signature table and the parameter type disagree about
/// what the parameter is.
fn options_defaults(
    defaults: &[Option<mwl_types::ConstArg>],
    index: usize,
) -> &[(String, mwl_types::ConstArg)] {
    match defaults.get(index) {
        Some(Some(mwl_types::ConstArg::Options(options))) => options,
        other => panic!(
            "mwl-ir: parameter {index} is an options bag but its recorded default is {other:?} \
             — mwl_types::core_lib is trusted to record one `ConstArg::Options` per bag"
        ),
    }
}

/// The label the class synthesized for an ADR 0036 § 2 shape literal carries
/// — `$shape{x,y}` for `{x: 1, y: 2}`, from the field names **already
/// sorted**.
///
/// A label, not a name: it identifies the class in [`crate::ir::Program`]'s
/// table and nowhere else, and it is never a symbol, since a shape class has
/// no methods to emit one for. `$` cannot start an MWL identifier, so no
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
            "mwl-ir only lowers a resolved call's bool/int/uint/float/void/string/bytes/array/\
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
        // (`mwl_types::expr_table::ExprInfo::SecretEquality`).
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
        // 0007 § 3's atoms, read by `mwl_types::ty::Ty::True`), so they erase
        // to `bool`. The union arm below then folds `true|false` back to one
        // `Ty::Bool` for free.
        CheckedTy::True | CheckedTy::False => Ty::Bool,
        CheckedTy::Enum(_, backing) | CheckedTy::EnumCase(_, backing, _) => {
            Ty::Enum(match backing {
                mwl_types::EnumBacking::Int => EnumRepr::Int,
                mwl_types::EnumBacking::Uint => EnumRepr::Uint,
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
/// whose index `mwl_runtime::CLOSURE_ARITY_SLOT` restates and
/// `mwl-codegen`'s `a_closure_object_carries_its_own_arity_in_slot_zero`
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
/// `mwl_runtime::call_closure`, which compares before it passes.
///
/// A nibble is the `mwl_runtime::Tag` discriminant the argument must carry,
/// so the reader needs no table of its own; [`param_tag_nibble`] is the map
/// and `mwl-codegen`'s `param_tag_nibbles_are_the_runtime_tag_bytes` holds it
/// against `mwl_codegen::ty::tag_of`, which is the same fact one crate over.
/// [`FN_PARAM_TAG_ANY`] is the one nibble that is not a tag: a `mixed`, `?T`
/// or union parameter is `Ty::Tagged`, whose representation *is* a tag byte
/// chosen at run time, so nothing about the argument can be wrong.
///
/// # The bound, and what happens past it
///
/// Sixteen parameters fit ([`FN_PARAM_TAGS_CAPACITY`]). A closure declaring
/// more gets no nibble for its seventeenth onward, and `mwl_runtime` refuses
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
/// Deliberately not a `mwl_runtime::Tag` discriminant — the roster runs to
/// eleven, so twelve is free and can never be mistaken for a tag a value
/// actually carries.
pub const FN_PARAM_TAG_ANY: u8 = 12;

/// The [`FN_PARAM_TAGS`] nibble a parameter represented as `ty` requires.
///
/// The whole of this map is "the tag a value of that representation carries",
/// which is `mwl_codegen::ty::tag_of` one crate over — `mwl-ir` cannot name
/// `mwl_runtime::Tag` (it does not depend on it) and neither can `mwl-runtime`
/// name this, so the two are held together by a test in `mwl-codegen`, which
/// sees both. That is the same shape [`FN_ARITY`] and
/// `mwl_runtime::CLOSURE_ARITY_SLOT` already stand in.
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
/// `tests/conformance/core/arr-a-mixed-or-nullable-callback-parameter-is-unchecked.mwlt`
/// pins both halves from MWL.
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
/// `tests/conformance/core/out-a-callback-parameter-naming-a-class-checks-the-argument-class-at-entry.mwlt`
/// pins both lines — this one around objecthood, that one around ancestry.
pub fn param_tag_nibble(ty: Ty) -> u8 {
    match ty {
        // `Ref` and `ClassDesc` ride in the payload of an otherwise-`null`
        // slot, exactly as `tag_of` says; neither is writable as a parameter's
        // declared type, and a `&$x` parameter is refused before it gets here.
        Ty::Null | Ty::Ref | Ty::ClassDesc => 0,
        Ty::Bool => 1,
        // ADR 0010's enum travels as its backing integer, tag included, so
        // these two rows are why an enum and its backing type are one
        // representation here and two enums over one backing are as well —
        // `tests/conformance/core/arr-a-callback-enum-parameter-is-its-backing-integer.mwlt`
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
/// `mwl_types::expr::operators` refuses a target that nests deeper
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
/// an `mwl_runtime::Value`, so an integer is what can travel; a class
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
/// `mwl_types::expr::operators`' `reject_uncheckable_element_type`, which owns
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
mod tests {
    use insta::assert_snapshot;
    use mwl_diagnostics::{Diagnostics, SourceId, SourceMap};
    use mwl_syntax::ast::{ClassMemberKind, StmtKind as TopStmtKind};
    use mwl_syntax::parse_file;

    use super::*;
    use crate::print::{print_function, print_program};

    /// Parses `src`, actually runs it through `mwl_hir::resolve_file` and
    /// `mwl_types::check_program` (unlike this crate's earlier slices, which
    /// only trusted a fixture *would* pass — now that lowering a call/`new`
    /// needs a real [`ExprTypeTable`], a fixture needs a real check run to
    /// produce one), pulls out `T`'s first method, and lowers it.
    fn lower_first_method(src: &str) -> (Function, SourceMap, SourceId) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = mwl_hir::resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut checked_types = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        let files = [mwl_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }];
        let enums =
            mwl_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

        let decl = stmts
            .iter()
            .find_map(|s| match &s.kind {
                TopStmtKind::ClassDecl(decl)
                    if span_text(map.file(file), decl.name.span) == "T" =>
                {
                    Some(decl)
                }
                _ => None,
            })
            .expect("fixture must declare a class `T`");
        let method = decl
            .members
            .iter()
            .find_map(|m| match &m.kind {
                ClassMemberKind::Method(method) => Some(method),
                _ => None,
            })
            .expect("fixture class must declare a method");

        let name = span_text(map.file(file), method.name).to_owned();
        let f = lower_method(
            &name,
            method,
            map.file(file),
            &exprs,
            &checked_types,
            &enums,
        );
        (f.function, map, file)
    }

    /// Parses, resolves and checks `src` exactly as [`lower_first_method`]
    /// does, then lowers the file's *own* top-level statements through
    /// [`lower_script`] instead of pulling a method out of a class.
    fn lower_script_src(src: &str) -> (Function, SourceMap, SourceId) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = mwl_hir::resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut checked_types = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        let files = [mwl_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }];
        let enums =
            mwl_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

        let f = lower_script(
            "<script>",
            &stmts,
            map.file(file),
            &exprs,
            &checked_types,
            &enums,
        );
        (f.function, map, file)
    }

    /// The whole file lowered — every function and every class, which is
    /// what a generator needs: one declaration becomes three functions plus a
    /// synthesized class, and a snapshot of any one of them alone would hide
    /// how they fit together.
    fn lower_program(src: &str) -> (crate::ir::Program, SourceMap, SourceId) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = mwl_hir::resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut checked_types = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        let files = [mwl_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }];
        let enums =
            mwl_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let layouts = mwl_types::layout::build_class_layouts(&files, &module.graph);
        let p = lower_file(
            "<script>",
            &stmts,
            map.file(file),
            &exprs,
            &checked_types,
            &enums,
            &layouts,
        );
        (p, map, file)
    }

    /// The acceptance program of `docs/agent/loop-goal.md`, lowered: one
    /// synthesized frame, no receiver parameter, a `ConstStr` handed straight
    /// to `Helper::EchoStr`, and the literal released right after the write
    /// reads it (nothing else ever owns it).
    #[test]
    fn a_script_body_echoes_a_string_literal() {
        let (f, map, file) = lower_script_src("<?mwl\necho \"Hello, World!\";\n");
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `throw new LogicError(…)` allocates an ordinary object, runs the
    /// synthesized root constructor on it, stamps the throw site into
    /// `location`, hands it to the context, and enters a landing block that
    /// names the frame the throw is leaving.
    #[test]
    fn a_throw_builds_an_exception_object_and_enters_its_landing_block() {
        let (f, map, file) = lower_script_src("<?mwl\nthrow new LogicError(\"boom\");\n");
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Every failing call inside a `try` reaches the same dispatch block,
    /// each through its own landing block — which is what keeps a handler
    /// phi's predecessors distinct. The exception is taken once by
    /// `take.thrown`, tested by one `instanceof` per clause, and the clause's
    /// binding is released where its body ends.
    #[test]
    fn a_try_gives_every_protected_call_its_own_landing_block() {
        let (f, map, file) = lower_script_src(
            "<?mwl\nclass T {\n  public static function go(): void { }\n}\n\
             try {\n  T::go();\n  echo \"fine\";\n} catch (Throwable $e) {\n  \
             echo $e->message;\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Two clauses on one `try`: an `instanceof` chain in source order, and a
    /// re-raise of the very same reference when neither matches.
    #[test]
    fn two_catch_clauses_lower_to_an_instanceof_chain_ending_in_a_rethrow() {
        let (f, map, file) = lower_script_src(
            "<?mwl\nclass T {\n  public static function go(): void { }\n}\n\
             try {\n  T::go();\n} catch (LogicError $a) {\n  echo \"logic\";\n\
             } catch (IOError $b) {\n  echo \"io\";\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `try { return … } finally { … }` — the `finally` body is lowered once
    /// per exit, so the `return` runs its own copy before leaving the frame
    /// and the exception path runs a second one before re-raising.
    #[test]
    fn a_finally_is_lowered_once_per_exit_out_of_the_protected_region() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
  function m(): int {
    try {
      return T::inner();
    } finally {
      echo \"done\";
    }
  }
  static function inner(): int { return 1; }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Outside a `try`, a landing block releases the frame's live refcounted
    /// locals before the status travels onward — `mwl-codegen`'s known gap 3,
    /// stated as IR rather than left to the backend.
    #[test]
    fn a_propagating_landing_block_releases_the_frames_live_strings() {
        let (f, map, file) = lower_script_src(
            "<?mwl\nclass T {\n  public static function go(): void { }\n}\n\
             string $s = \"held\";\nT::go();\necho $s;\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `Core` member borrows its arguments, so a materialized `string`
    /// literal is this frame's own temporary — and the helper is exactly the
    /// thing that can throw while it is in flight.
    /// [`Lowering::landing_block`] releases the whole
    /// [`Lowering::owned_temporaries`] stack on the error edge, and this pins
    /// the harder half: the edge that reaches a `catch` in this same frame,
    /// where the locals sweep deliberately releases nothing. A temporary has
    /// no `Env` entry for the handler to find it through, so the landing block
    /// is the last place anything can drop it.
    #[test]
    fn a_landing_block_releases_the_call_temporaries_still_in_flight() {
        let (f, map, file) = lower_script_src(
            "<?mwl\ntry {\n  bytes $b = Core\\Encoding::fromHex(\"ff\");\n}\
             \ncatch (Throwable $e) {\n  echo \"caught\";\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A file-scope local is an ordinary local of the synthesized frame
    /// (ADR 0008 § 2) — declared, reassigned and read with exactly the
    /// machinery a method body already uses. `echo` of an `int` converts
    /// through the same `Helper::IntToString` `.` concatenation uses.
    #[test]
    fn a_script_body_local_is_an_ordinary_local() {
        let (f, map, file) = lower_script_src("<?mwl\nint $n = 1;\n$n = $n + 2;\necho $n;\n");
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A declaration is skipped by the script frame — `T`'s method is
    /// `lower_method`'s job, not this walk's — while a
    /// `namespace X { ... }` block's own statements are not: a namespace
    /// scopes names, not storage, so they share this one frame.
    #[test]
    fn a_script_body_skips_declarations_and_enters_a_namespace_block() {
        let (f, map, file) = lower_script_src(
            "<?mwl\nclass T {\n  function m(): void { }\n}\nnamespace A { echo \"in-ns\"; }\necho \"after\";\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    #[test]
    fn straight_line_arithmetic_and_return() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function add(int $a, int $b): int {\n    int $sum = $a + $b;\n    return $sum;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// ADR 0007 § 4's "either operand a `float`" row is settled *here*, by a
    /// conversion emitted ahead of the operator, and not by `mwl-codegen`
    /// repairing a `BinOp` whose two operands disagree — that crate's "a
    /// `BinOp` has one representation" invariant stays intact, and its
    /// mismatch refusal stays a genuine internal error.
    ///
    /// The conversion is the checked one `$n as float` writes
    /// (`Helper::IntToFloat`/`UintToFloat`), so it carries ADR 0002's error
    /// edge — ADR 0007 § 2 names this as the language's one implicit
    /// conversion and says it throws above 2^53 rather than rounding.
    ///
    /// Read off the rendering rather than snapshotted: what is pinned is the
    /// *order* of two instructions and which of them can throw, and a
    /// snapshot would go red for an unrelated renumbering while saying
    /// nothing about either.
    #[test]
    fn a_mixed_numeric_pair_converts_before_the_operator() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\nclass T {\n",
            "  function widen(int $i, uint $u, float $f): float {\n",
            "    float $a = $i + $f;\n",
            "    float $b = $f * $u;\n",
            "    return $a + $b;\n",
            "  }\n}\n",
        ));
        let text = print_function(&f, map.file(file));
        let at = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle} is not in the lowering: {text}"))
        };
        assert!(
            at("int_to_float") < at("= add "),
            "the `int` side widened after its operator: {text}"
        );
        assert!(
            at("uint_to_float") < at("= mul "),
            "the `uint` side widened after its operator: {text}"
        );
        for line in text.lines() {
            if line.contains("int_to_float") {
                assert!(
                    line.contains(" ! bb"),
                    "the implicit widening lost its error edge: {line}"
                );
            }
        }
    }

    /// The other side of that bound: ADR 0007 § 4's *ordering* rows are
    /// exact in the mathematical integers, so a mixed numeric pair under
    /// `<` is answered by `Helper::NumericLt` and pays no conversion at
    /// all — a widening there would raise `ArithmeticError` past 2^53 for
    /// a pair that orders perfectly well.
    #[test]
    fn a_mixed_numeric_comparison_pays_no_widening() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\nclass T {\n",
            "  function order(int $i, float $f): bool {\n",
            "    return $i < $f;\n",
            "  }\n}\n",
        ));
        let text = print_function(&f, map.file(file));
        assert!(text.contains("numeric_lt"), "{text}");
        assert!(
            !text.contains("int_to_float"),
            "an ordering row widened an operand that can throw: {text}"
        );
    }

    /// ADR 0007 § 4's six bitwise rows all reach an instruction, and only the
    /// two that PHP can refuse carry an error edge.
    ///
    /// Read off the rendering rather than snapshotted, because what is being
    /// pinned is *which* rows are fallible — a snapshot would go red for any
    /// unrelated renumbering and say nothing about that.
    #[test]
    fn every_bitwise_operator_lowers() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\nclass T {\n",
            "  function mix(int $a, int $b): int {\n",
            "    int $out = ((($a & $b) | 8) ^ 1) << 2;\n",
            "    return ($out >> 1) + ~$a;\n",
            "  }\n}\n",
        ));
        let text = print_function(&f, map.file(file));
        for name in ["band", "bor", "bxor", "shl", "shr", "bnot"] {
            assert!(text.contains(name), "{name} did not lower: {text}");
        }
        // A shift's *count* is the only thing PHP refuses here — a negative
        // one throws `ArithmeticError` — so `<<` and `>>` take ADR 0002's edge
        // while the three total operators and `~` do not.
        for line in text.lines() {
            let fallible = line.contains(" ! bb");
            for (name, expected) in [
                ("= band ", false),
                ("= bor ", false),
                ("= bxor ", false),
                ("= bnot ", false),
                ("= shl ", true),
                ("= shr ", true),
            ] {
                if line.contains(name) {
                    assert_eq!(fallible, expected, "{name}error edge: {line}");
                }
            }
        }
    }

    /// `$x op= e` is `$x = $x op e`, so the five bitwise compound forms need no
    /// lowering of their own — `lower_compound_assignment`'s rewrite is what
    /// gives them one, and this is the test that says so rather than a second
    /// table in the lowering.
    #[test]
    fn a_bitwise_compound_assignment_lowers_through_its_binary_form() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\nclass T {\n",
            "  function mask(int $x): int {\n",
            "    $x &= 3;\n    $x |= 8;\n    $x ^= 1;\n    $x <<= 2;\n    $x >>= 1;\n",
            "    return $x;\n",
            "  }\n}\n",
        ));
        let text = print_function(&f, map.file(file));
        for name in ["band", "bor", "bxor", "shl", "shr"] {
            assert!(
                text.contains(name),
                "the compound form of {name} did not lower: {text}"
            );
        }
    }

    /// `$x++` and `++$x` are the same *statement*: the operator's position
    /// decides which of the read-modify-write's two values a surrounding
    /// expression sees, and an expression statement sees neither. So all four
    /// spellings go through the one [`Lowering::lower_incdec_stmt`] and two
    /// adds and two subs is the whole shape.
    ///
    /// Counted rather than snapshotted, for
    /// [`a_bitwise_compound_assignment_lowers_through_its_binary_form`]'s
    /// reason: a snapshot would go red for a renumbering while saying nothing
    /// about the two spellings agreeing.
    #[test]
    fn an_increment_lowers_in_either_position() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\nclass T {\n",
            "  function step(int $x): int {\n",
            "    $x++;\n    ++$x;\n    $x--;\n    --$x;\n",
            "    return $x;\n",
            "  }\n}\n",
        ));
        let text = print_function(&f, map.file(file));
        assert_eq!(text.matches("= add ").count(), 2, "{text}");
        assert_eq!(text.matches("= sub ").count(), 2, "{text}");
    }

    /// The `$t = $t ⊕ e` rewrite writes its target down twice, so a target
    /// with a call in it would *call* twice — `f()->count += 1` incrementing
    /// the field of one object and then discarding a second one.
    /// [`Lowering::lower_read_modify_write`] lowers the address once and
    /// stages it ([`Lowering::staged_targets`]), which is what this counts:
    /// one call, and the field read and written off the value it produced.
    #[test]
    fn a_compound_assignment_evaluates_its_target_once() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\nclass T {\n",
            "  public int $count = 0;\n",
            "  function bump(): void {\n",
            "    $this->box()->count += 1;\n",
            "  }\n",
            "  function box(): T { return $this; }\n",
            "}\n",
        ));
        let text = print_function(&f, map.file(file));
        assert_eq!(text.matches("call T::box").count(), 1, "{text}");
        assert_eq!(text.matches("field.get").count(), 1, "{text}");
        assert_eq!(text.matches("field.set").count(), 1, "{text}");
    }

    /// ADR 0007 § 4 gives `**` a row for every numeric representation but the
    /// `decimal` [ADR 0054](../../../../docs/adr/0054-decimal-scalar-type.md)
    /// § 3 refuses, and lists it beside `+`, `-` and `*` — so the two integer
    /// rows throw and the `float` one, being `f64::powf`, cannot.
    ///
    /// Counted rather than snapshotted, and for the reason
    /// [`every_bitwise_operator_lowers`] gives: what is pinned is *which* rows
    /// carry ADR 0002's edge, and a snapshot would go red for a renumbering
    /// while saying nothing about that. `**=` is in the same body because it
    /// has no lowering of its own — `lower_compound_assignment`'s rewrite is
    /// what gives it one — so the count is what says it arrived.
    #[test]
    fn a_power_operator_lowers_over_every_numeric_row() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\nclass T {\n",
            "  function rows(int $i, uint $u, float $x): float {\n",
            "    int $a = $i ** 3;\n",
            "    uint $b = $u ** $u;\n",
            "    float $c = $x ** 2.0;\n",
            "    $a **= 2;\n",
            "    return $c;\n",
            "  }\n}\n",
        ));
        let text = print_function(&f, map.file(file));
        let powers: Vec<&str> = text
            .lines()
            .filter(|line| line.contains("= pow "))
            .collect();
        assert_eq!(
            powers.len(),
            4,
            "one `pow` per row and one for `**=`: {text}"
        );
        let fallible = powers.iter().filter(|line| line.contains(" ! bb")).count();
        assert_eq!(
            fallible, 3,
            "the two `int` rows and the `uint` one throw, the `float` one does not: {text}"
        );
    }

    /// A plain local reassignment gets a fresh SSA value rather than mutating
    /// the one already bound to `$n` — the point of routing even
    /// straight-line reassignment through `lower_reassignment`.
    #[test]
    fn reassignment_produces_a_fresh_ssa_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function bump(int $n): int {\n    int $out = $n;\n    $out = $out + 1;\n    return $out;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Unary negation/not and a comparison operator, over a `uint`-defaulted
    /// bare literal (ADR 0007 § 4) — exercises the operators the arithmetic
    /// test above doesn't.
    #[test]
    fn unary_and_comparison_operators() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function check(int $n): bool {\n    bool $neg = -$n < 0;\n    return !$neg;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `uint` local initialized from a bare integer literal takes the
    /// literal as `uint`, not `int` — ADR 0007 § 4's target-directed rule,
    /// mirrored from `mwl_types::expr::infer`.
    #[test]
    fn a_bare_literal_targeting_uint_is_lowered_as_uint() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): uint {\n    uint $n = 1;\n    return $n;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if`/`else`, both branches reassigning the same local — the plain
    /// two-predecessor merge, needing one real phi.
    #[test]
    fn if_else_merges_with_a_phi() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function pick(bool $c, int $a, int $b): int {\n    int $r = 0;\n    if ($c) {\n      $r = $a;\n    } else {\n      $r = $b;\n    }\n    return $r;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if` with no `else` — the false edge lands on the merge block
    /// directly, carrying the pre-branch environment.
    #[test]
    fn if_with_no_else_merges_the_implicit_edge() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function clamp(int $n): int {\n    int $r = $n;\n    if ($r < 0) {\n      $r = 0;\n    }\n    return $r;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Both branches of an `if` `return` — the merge block has no real
    /// predecessor and is dead, but still needs a well-formed terminator.
    #[test]
    fn if_else_both_returning_leaves_a_dead_merge_block() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function abs(int $n): int {\n    if ($n < 0) {\n      return -$n;\n    } else {\n      return $n;\n    }\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `while` loop reassigning two pre-existing locals in its body — each
    /// needs its own loop-header phi, patched with the back-edge value.
    #[test]
    fn while_loop_carries_locals_through_a_header_phi() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function sum(int $n): int {\n    int $total = 0;\n    int $i = 0;\n    while ($i < $n) {\n      $total = $total + $i;\n      $i = $i + 1;\n    }\n    return $total;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if ($n)` with an `int` parameter — ADR 0035's truthy table for a
    /// scalar condition, converted through the new `Helper::IntTruthy`
    /// rather than requiring `$n` already be `bool`.
    #[test]
    fn an_int_condition_converts_through_a_truthy_helper() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(int $n): bool {\n    if ($n) {\n      return true;\n    }\n    return false;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `while ($s)` with a `string` parameter — the same table's `string`
    /// row (`Helper::StrTruthy`), exercised through `while` rather than
    /// `if` to confirm `Lowering::lower_while` routes through the same
    /// `Lowering::lower_truthy_cond` helper.
    #[test]
    fn a_string_while_condition_converts_through_a_truthy_helper() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(string $s): void {\n    while ($s) {\n      $s = \"\";\n    }\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if ($a)` with an `array<int>` parameter — ADR 0035's "empty is
    /// falsy, regardless of element type" row, via `Helper::ArrayTruthy`.
    /// `$a` is a bare variable read (`is_aliasing_read`), so no release
    /// follows the helper call — the array is still the parameter's own
    /// slot, released normally at scope exit.
    #[test]
    fn an_array_condition_converts_through_a_truthy_helper() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): bool {\n    if ($a) {\n      return true;\n    }\n    return false;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if (self::make())` where `make` returns a fresh `array<int>` — unlike
    /// the parameter case above, this array has no other owner once the
    /// truthy check reads it, so `Lowering::lower_truthy_cond` must release
    /// it right after, the same "release a fresh value once its one and only
    /// use is done" precedent `Self::concat_operand`'s own caller already
    /// sets for `.` concatenation.
    #[test]
    fn a_fresh_array_condition_is_released_after_the_truthy_check() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): bool {\n    if (self::make()) {\n      return true;\n    }\n    return false;\n  }\n  static function make(): array<int> {\n    return [1];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if ($f)` with a class-typed parameter — ADR 0035 § 4 makes a class
    /// instance always truthy, so this needs no `HelperCall` at all: it
    /// folds straight to a fresh `const.bool true`.
    #[test]
    fn an_object_condition_is_always_truthy() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(Foo $f): bool {\n    if ($f) {\n      return true;\n    }\n    return false;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `Ty::Tagged`'s first slice: a `mixed`-typed parameter, returned
    /// straight back through the bare-`$name`-return transfer-out path — the
    /// narrowest possible round-trip, exercising `lower_decl_type`'s new
    /// `TypeAtom::Mixed` arm for both the parameter and the return type with
    /// `Ty::is_refcounted` correctly reporting `false` (no retain/release
    /// appears anywhere in the snapshot).
    #[test]
    fn a_mixed_parameter_round_trips_through_return() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function pick(mixed $x): mixed {\n    return $x;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `mixed $y = $x;` — an explicitly `mixed`-typed local declared from a
    /// `mixed` parameter, then returned. Exercises `Lowering::bind_local`
    /// with a `Ty::Tagged` binding: still no retain, since `Ty::Tagged` is not
    /// `is_refcounted`, and the local correctly excludes itself from
    /// `release_all_locals`'s exit sweep by transferring out on `return`,
    /// exactly like any other bare-variable return.
    #[test]
    fn a_typed_mixed_local_round_trips() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function pick(mixed $x): mixed {\n    mixed $y = $x;\n    return $y;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `var $y = $x;` (ADR 0037) with a `mixed`-typed initializer — `var`'s
    /// own inference path (`lower_expr` with `expected: None`) picks up
    /// `Ty::Tagged` from the initializer exactly the way it already does for
    /// any other representation, needing no `var`-specific handling.
    #[test]
    fn a_var_local_infers_mixed_from_a_mixed_initializer() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function pick(mixed $x): mixed {\n    var $y = $x;\n    return $y;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Passing a `mixed` local as a call argument round-trips too — the
    /// callee's own `mixed` parameter is just another local, released at the
    /// callee's own (trivial, no-op) exit, mirroring every other
    /// representation's call-argument boundary.
    #[test]
    fn passing_a_mixed_local_as_a_call_argument_round_trips() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(mixed $x): mixed {\n    return $this->identity($x);\n  }\n  function identity(mixed $v): mixed {\n    return $v;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `mixed` condition is ADR 0035 § 2's last table row: the dispatch it
    /// names moves into `Helper::ValueTruthy`, which reads the operand's tag
    /// and applies whichever of the rows above it names. This used to be a
    /// `#[should_panic]` guard over exactly this source — `Ty::Tagged` gave
    /// the value a representation to exist in without giving the table a way
    /// to read it — so what the snapshot pins is one helper call where a
    /// panic stood, and no untag anywhere: an unchecked one over an `int`
    /// payload is a pointer the next instruction would dereference.
    #[test]
    fn a_mixed_condition_dispatches_the_truthy_table_on_the_tag() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(mixed $x): bool {\n    if ($x) {\n      return true;\n    }\n    return false;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `bytes` is the one row ADR 0035 § 2 does not take from PHP, which has
    /// no such type: falsy iff empty, dropping the one-octet `"0"` case that
    /// exists for a `string` only because PHP reads one as a possible number.
    /// The snapshot pins that a declared `bytes` reaches `Helper::BytesTruthy`
    /// rather than `Helper::StrTruthy`, which is the whole of the difference —
    /// the two heap shapes are identical, so a mis-routed operand would still
    /// run and answer wrongly on one input.
    #[test]
    fn a_bytes_condition_takes_its_own_truthy_helper() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bytes $b): bool {\n    if ($b) {\n      return true;\n    }\n    return false;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `for` loop: the initializer runs before the header, the step runs in
    /// a block of its own between the body and the header, and the loop
    /// variable's header phi is patched from that step block — see
    /// [`Lowering::lower_for`].
    #[test]
    fn for_loop_runs_its_step_in_a_block_between_body_and_header() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function sum(int $n): int {\n    int $total = 0;\n    int $i = 0;\n    for ($i = 0; $i < $n; $i += 1) {\n      $total += $i;\n    }\n    return $total;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `continue` in a `for` jumps to the step block rather than the header,
    /// so the step still runs on that path — the one structural difference
    /// between [`Lowering::lower_for`] and [`Lowering::lower_while`].
    #[test]
    fn for_loop_continue_reaches_the_step_block() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function count(int $n): int {\n    int $hits = 0;\n    int $i = 0;\n    for ($i = 0; $i < $n; $i += 1) {\n      if ($i == 2) {\n        continue;\n      }\n      $hits += 1;\n    }\n    return $hits;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `for` whose body always returns: nothing reaches the step block, so
    /// it is sealed with the environment the loop was entered with rather
    /// than a merge of edges that do not exist. See [`Lowering::lower_for`].
    #[test]
    fn for_loop_whose_body_always_returns_leaves_a_dead_step_block() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function head(int $n): int {\n    int $i = 0;\n    for ($i = 0; $i < $n; $i += 1) {\n      return $i;\n    }\n    return -1;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `switch`: one subject, an equality chain over the labels, and a
    /// `default` reached by the chain's own fall-off — see
    /// [`Lowering::lower_switch`].
    #[test]
    fn switch_lowers_to_an_equality_chain_ending_at_the_default() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function rank(int $n): int {\n    switch ($n) {\n      case 1:\n        return 10;\n      case 2:\n        return 20;\n      default:\n        return 0;\n    }\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Fallthrough is the absence of a `break`, so a body that reaches its end
    /// jumps into the next body block rather than past the switch, and a
    /// `break` jumps to the after-block the frame carries.
    #[test]
    fn switch_falls_through_a_body_with_no_break() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function pick(int $n): int {\n    int $hits = 0;\n    switch ($n) {\n      case 1:\n      case 2:\n        $hits += 1;\n        break;\n      default:\n        $hits += 9;\n    }\n    return $hits;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `string` subject is retained for the length of the switch and
    /// released once in the after-block, the same shape
    /// [`Lowering::lower_foreach`] gives the array it walks.
    #[test]
    fn a_switch_over_a_string_holds_one_reference_to_its_subject() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function tier(string $s): int {\n    int $out = 0;\n    switch ($s) {\n      case \"a\":\n        $out = 1;\n        break;\n    }\n    return $out;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `continue` written inside a `switch` inside a loop continues the
    /// **loop** — the switch's frame carries no continue target, so
    /// [`Lowering::lower_continue`] walks past it. PHP's own bare `continue`
    /// there means `break`; see [`Lowering::lower_switch`] for why MWL takes
    /// the meaning PHP's warning points at instead.
    #[test]
    fn continue_inside_a_switch_reaches_the_enclosing_loops_header() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function count(int $n): int {\n    int $hits = 0;\n    int $i = 0;\n    while ($i < $n) {\n      $i += 1;\n      switch ($i) {\n        case 2:\n          continue;\n        default:\n          $hits += 1;\n      }\n    }\n    return $hits;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `match` is [`Lowering::lower_switch`]'s chain producing a value: each
    /// arm ends in a jump to one merge block, and the arms join in a phi the
    /// way a ternary's two branches do.
    #[test]
    fn match_arms_join_in_a_phi_at_one_merge_block() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function name(int $n): string {\n    return match ($n) {\n      1, 2 => \"low\",\n      default => \"high\",\n    };\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// With no `default` arm the chain's fall-off raises a `LogicError`
    /// instead of reaching the merge — `mwl_hir::errors`' closed tree has no
    /// `UnhandledMatchError` to raise.
    #[test]
    fn a_match_with_no_default_throws_where_the_chain_runs_out() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function name(int $n): string {\n    return match ($n) {\n      1 => \"one\",\n    };\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `new Foo(1)` with a resolved one-parameter constructor — the class's
    /// own resolved target and the constructor's argument both come from the
    /// typed-expression table (`ExprInfo::New`), not from re-deriving `Foo`'s
    /// signature by hand.
    #[test]
    fn new_with_a_resolved_constructor() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  function constructor(int $x) {}\n}\nclass T {\n  function make(): Foo {\n    return new Foo(1);\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `new Foo()` against a class with no explicit `constructor` — the
    /// `ExprInfo::New` entry's `ctor` is `None`, so lowering emits an empty
    /// argument list rather than looking one up.
    #[test]
    fn new_with_no_declared_constructor() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {}\nclass T {\n  function make(): Foo {\n    return new Foo();\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `self::make()` — a static call with no receiver, resolved to `T`'s own
    /// method via the typed-expression table.
    #[test]
    fn a_self_static_call_with_a_scalar_return() {
        // `m` declared first, forward-referencing `make` — `lower_first_method`
        // lowers `T`'s *first* method, and MWL resolves a same-class method
        // call regardless of declaration order (its signature table is built
        // in a pass ahead of body-checking; see `mwl_types::signatures`).
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    return self::make(1);\n  }\n  static function make(int $n): int {\n    return $n;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A local declared with a class type, initialized from `new` and
    /// returned — exercises `lower_decl_type`'s `TypeAtom::Name` arm
    /// alongside `ExprInfo::New`.
    #[test]
    fn a_class_typed_local_initialized_from_new() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {}\nclass T {\n  function make(): Foo {\n    Foo $x = new Foo();\n    return $x;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$this->a(1)` — the implicit receiver seeded by `lower_method` (SSA
    /// value 0, parameter index 0) flows into `InstKind::Call`'s `receiver`
    /// field via the same `ExprKind::Variable`/`Env` lookup any other local
    /// uses; nothing about `MethodCall`'s own lowering is `$this`-specific.
    #[test]
    fn a_this_method_call() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    return $this->a(1);\n  }\n  function a(int $x): int {\n    return $x;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->greet()` — an instance call through a receiver that isn't
    /// `$this` at all, on a class with no explicit parameters, to exercise
    /// the general `object` lowering path rather than only the `$this`
    /// special case.
    #[test]
    fn an_instance_method_call_through_a_local_receiver() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj->greet();\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj?->greet()` on a receiver that may be `null` — one `is.null` over
    /// the tagged receiver, the call in the arm where it isn't, and a `null`
    /// in the arm where it is, merged by a `phi`. See
    /// `Lowering::open_nullsafe`.
    #[test]
    fn a_nullsafe_method_call_on_a_nullable_receiver() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(?Foo $obj): ?int {\n    return $obj?->greet();\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// The same call on a receiver whose *representation* rules `null` out
    /// costs nothing: no branch, no tag test, exactly the instructions `->`
    /// emits — which is also why `mwl_types` gives it no `null` in its type.
    #[test]
    fn a_nullsafe_method_call_on_a_receiver_that_cannot_be_null() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj?->greet();\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if ($obj != null) { $obj->greet(); }` — the receiver's slot is still
    /// one tagged value, so the plain `->` reads it back with a single
    /// unchecked `untag` and no test of its own. The `!== null` in the
    /// condition is the *only* tag test, and it is one `is.null` rather than
    /// a comparison against a `null` constant. See
    /// `Lowering::untag_receiver`.
    #[test]
    fn a_narrowed_receiver_untags_once_with_no_guard_of_its_own() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(?Foo $obj): int {\n    if ($obj != null) {\n      return $obj->greet();\n    }\n    return 0;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$this->count` — a property access through the implicit receiver,
    /// resolved to its declaring class via `ExprInfo::Property`.
    #[test]
    fn a_this_property_access() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  public int $count = 0;\n  function m(): int {\n    return $this->count;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->count` — a property access through a receiver that isn't
    /// `$this`, to exercise the general `object` lowering path rather than
    /// only the `$this` special case.
    #[test]
    fn a_property_access_through_a_local_receiver() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj->count;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj?->count` — the same guard a nullsafe *call* opens, wrapped
    /// around a field read instead: the `field.get` runs only in the arm
    /// where the tag says the receiver is not `null`.
    #[test]
    fn a_nullsafe_property_access_on_a_nullable_receiver() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(?Foo $obj): ?int {\n    return $obj?->count;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$i->path` on an ADR 0036 § 4 shape receiver — no class, no label and
    /// no layout table: one `slot.get` keyed on the field's *name*, carrying
    /// its position in the shape's sorted field list (which puts `path` after
    /// `message`) as the runtime's hint. Fallible, so it has a landing block
    /// of its own: § 4 makes a name the concrete receiver does not carry a
    /// catchable throw. The receiver is a parameter, so it is an aliasing
    /// read and the value read out of its slot is retained by whoever keeps
    /// it, exactly as for a class field.
    #[test]
    fn a_shape_property_access_reads_its_slot_by_name() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m({path: string, message: string} $i): string {\n    return $i->path;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A property access through a plain-`object` receiver is ADR 0036 § 4's
    /// *fully* erased half: there is no declaring class and no layout either,
    /// so what the checker records is the written name alone and the read
    /// lowers to the same name-keyed `SlotGet` a shape's does — with a hint
    /// of slot 0, which the runtime's own by-name search corrects, and a
    /// result at `Ty::Tagged` because nothing knows what the field holds.
    #[test]
    fn a_property_access_through_a_plain_object_receiver_reads_by_name() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(object $o): mixed {\n    return $o->x;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `var $n = 1;` (ADR 0037) — no declared type at all, so the local's
    /// type is whatever `lower_expr` synthesizes from the initializer alone,
    /// exactly as `mwl_types::locals::check_stmt`'s own `var` arm fixes it.
    /// A bare integer literal with no `expected` type defaults to `int`
    /// (ADR 0007 § 4), so `$n` ends up `int` here even though nothing in the
    /// source spells that out.
    #[test]
    fn a_var_local_infers_its_type_from_the_initializer() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    var $n = 1;\n    return $n;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A hex/octal/binary integer literal cooks to the same value its
    /// decimal spelling would — `mwl-syntax`'s lexer accepts all three
    /// prefixed forms as one `IntLiteral` token (see
    /// `crates/mwl-syntax/src/lexer.rs`'s `lex_number`), and until this
    /// session `mwl-ir` only cooked a plain decimal run, so `0x1F` would have
    /// panicked as "doesn't fit an `int`" rather than lowering to `31`.
    #[test]
    fn multi_base_integer_literals_cook_to_the_same_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    int $hex = 0x1F;\n    int $oct = 0o17;\n    int $bin = 0b101;\n    return $hex + $oct + $bin;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `string` literal cooks to `InstKind::ConstStr` — a single-quoted
    /// literal only unescapes `\\`/`\'`, a double-quoted one additionally
    /// unescapes `\n`/`\t`/`\"`. Neither local is ever aliased or returned,
    /// so both get exactly one release at the implicit `void` fallback
    /// return — no retain anywhere in this fixture.
    #[test]
    fn string_literals_cook_their_escapes_and_release_at_scope_exit() {
        let (f, map, file) = lower_first_method(
            r#"<?mwl
class T {
  function m(): void {
    string $single = 'it\'s a \\ test';
    string $double = "line1\nline2\t\"quoted\"";
  }
}
"#,
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A double-quoted literal's numeric escapes (`\101` octal, `\x2A` hex,
    /// `\u{1F600}` a multi-byte Unicode codepoint) now cook to the actual
    /// byte/codepoint they name, delegated to
    /// `mwl_types::string_lit::cook_double_quoted_text` — see
    /// `cook_str_literal`'s own doc comment for why this crate shares that
    /// routine with the checker rather than duplicating it.
    #[test]
    fn numeric_escapes_cook_to_their_byte_or_codepoint() {
        let (f, map, file) = lower_first_method(
            r#"<?mwl
class T {
  function m(): void {
    string $s = "\101\x2A\u{1F600}";
  }
}
"#,
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `"pre$mid post"` — a double-quoted literal with one interpolation
    /// site surrounded by literal text on both sides — lowers to the same
    /// single three-piece `InstKind::Concat` a written-out
    /// `"pre" . $mid . " post"` does, per
    /// `Lowering::lower_interpolated_parts`, so the whole string is one
    /// allocation rather than a fold's two. `$mid`'s own read is an aliasing
    /// one, so it's left unreleased (its slot still owns it); the two literal
    /// text pieces are fresh and released once the `Concat` has read them,
    /// ending with the whole method's own `void` exit releasing `$s`.
    #[test]
    fn interpolated_string_with_text_on_both_sides_is_one_concat() {
        let (f, map, file) = lower_first_method(
            r#"<?mwl
class T {
  function m(): void {
    string $mid = "middle";
    string $s = "pre$mid post";
  }
}
"#,
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `"$x"` alone — no literal text around the one interpolation site —
    /// is `ExprKind::Interpolated`'s degenerate single-part case:
    /// `mwl_syntax::parser::collapse_string_parts` still picks `Interpolated`
    /// over `Str` (the one part isn't `StringPart::Text`), but no
    /// `InstKind::Concat` ever runs to copy `$x`'s value into a fresh
    /// buffer. `Lowering::lower_interpolated_parts` has to retain `$x`'s
    /// value itself in exactly this shape — the snapshot should show a
    /// retain on `$x`'s own value immediately after it's read, with no
    /// `Concat` instruction anywhere in the function, and the usual single
    /// release of `$s` (now the sole owner of that retained reference,
    /// alongside `$x`'s own still-live slot) at the implicit return.
    #[test]
    fn interpolated_string_with_only_a_variable_retains_it() {
        let (f, map, file) = lower_first_method(
            r#"<?mwl
class T {
  function m(): void {
    string $x = "hello";
    string $s = "$x";
  }
}
"#,
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A heredoc with no interpolation site used at all collapses to a plain
    /// `ExprKind::Str` (`mwl_syntax::parser::collapse_string_parts`), just
    /// like a double-quoted literal — the same `InstKind::ConstStr` shape,
    /// cooked through `cook_heredoc_str` instead of the quote-delimited
    /// branch. Its closing marker is flush left, so PHP 7.3's
    /// flexible-indentation strip is a no-op here; the numeric escape still
    /// cooks, since a heredoc runs the same escape grammar a double-quoted
    /// literal does.
    #[test]
    fn a_flush_left_heredoc_cooks_like_a_double_quoted_literal() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    string $s = <<<EOT\nline1\\nline2\nEOT;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// The closing marker's own indentation is stripped from every body
    /// line — PHP 7.3's "flexible heredoc" rule. The cooked `ConstStr`
    /// should show `"hello\nworld"` with no leading spaces baked in, even
    /// though the source itself indents both body lines and the marker to
    /// match this function's own brace nesting.
    #[test]
    fn an_indented_heredoc_strips_the_closing_markers_indentation() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    string $s = <<<EOT\n        hello\n        world\n        EOT;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A nowdoc (`<<<'EOT'`) applies no escape grammar at all — unlike the
    /// flush-left heredoc fixture above, `\n` here must cook to two literal
    /// characters, backslash and `n`, not a newline — while still getting
    /// its closing marker's indentation stripped exactly like a heredoc
    /// does.
    #[test]
    fn a_nowdoc_strips_indentation_but_applies_no_escapes() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    string $s = <<<'EOT'\n        raw \\n text\n        EOT;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A heredoc with an interpolation site lowers through the exact same
    /// `InstKind::Concat` over every piece that `lower_interpolated_parts`
    /// already builds for a double-quoted literal — the only difference is
    /// each `Text` run
    /// getting dedented first. The middle line picks up right after `$x`'s
    /// interpolation site, so it has to be recognized as a fresh line of
    /// its own for the indentation strip to apply to it at all.
    #[test]
    fn an_indented_interpolated_heredoc_strips_indentation_from_every_run() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    string $x = \"hi\";\n    string $s = <<<EOT\n        pre $x\n        post\n        EOT;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `string $b = $a;` aliases `$a`'s already-owned value rather than
    /// constructing a fresh one — `Lowering::bind_local` retains it. `return
    /// $b;` then transfers `$b`'s reference out directly (excluded from
    /// `Lowering::release_all_locals`'s sweep), leaving exactly one release
    /// for `$a`'s slot — one retain, one release, never zero and never two,
    /// for a value that in fact has exactly one owner (the caller) once this
    /// function returns.
    #[test]
    fn assigning_one_string_local_to_another_retains_the_shared_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function pick(): string {\n    string $a = \"hello\";\n    string $b = $a;\n    return $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Reassigning a `string` local to a fresh literal releases the value it
    /// previously held — `$x`'s `\"a\"` is released the moment `\"b\"`
    /// overwrites it, well before the function's own exit sweep releases
    /// `\"b\"` in turn.
    #[test]
    fn reassigning_a_string_local_releases_its_previous_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    string $x = \"a\";\n    $x = \"b\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Passing a `string` local as a call argument retains it first —
    /// `Lowering::lower_call_args`'s own aliasing check — since the callee's
    /// own parameter is bound like any other local and released at the
    /// callee's exit (not visible in this snapshot, since `lower_first_method`
    /// only lowers `T`'s first method). `take` returns `int`, not `string`,
    /// so the call's own result needs no refcount treatment — this fixture
    /// isolates the argument-side retain from the return-side question the
    /// two tests below cover. Net effect on `$s`'s own slot: one retain right
    /// before the call, one release at `m`'s own exit sweep.
    #[test]
    fn passing_a_string_local_as_a_call_argument_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    string $s = \"hi\";\n    return self::take($s);\n  }\n  static function take(string $x): int {\n    return 1;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `var $s = $obj->name;` — a `string`-typed property read is an
    /// aliasing read exactly like a bare variable read
    /// (`lower::is_aliasing_read`), so binding it to a new local retains the
    /// field's own value; `$obj` itself is `Ty::Object`, not yet refcounted,
    /// so only `$s`'s slot is released at the exit sweep.
    #[test]
    fn binding_a_string_property_read_to_a_local_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public string $name = \"hi\";\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    var $s = $obj->name;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `return $obj->name;` — a property read has no local slot for
    /// `Lowering::release_all_locals` to exclude the way a bare `$name`
    /// return does, so `StmtKind::Return`'s own arm retains it explicitly
    /// instead: exactly one retain, no release, leaving the caller with
    /// exactly one owned reference once this function returns.
    #[test]
    fn returning_a_string_property_read_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public string $name = \"hi\";\n}\nclass T {\n  function m(): string {\n    Foo $obj = new Foo();\n    return $obj->name;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `return self::make();` where `make` returns `string` — a call's own
    /// result is a fresh-like producer, same as `new` or a literal
    /// (`lower::is_aliasing_read` is `false` for `ExprKind::StaticCall`), so
    /// returning it directly needs no retain at all: it already has exactly
    /// one owner, which just transfers out to the caller.
    #[test]
    fn returning_a_string_returning_calls_result_needs_no_retain() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): string {\n    return self::make();\n  }\n  static function make(): string {\n    return \"hi\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `self::helper();` with no assignment at all — the ordinary way to
    /// invoke a `void`-returning method. `Lowering::lower_expr_stmt` now
    /// routes a bare call/`new` expression statement through `lower_expr`
    /// for its side effect alone; `helper` returns `void`, so there is
    /// nothing to release afterward.
    #[test]
    fn a_bare_void_call_used_as_a_statement_lowers_with_no_release() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    self::helper();\n  }\n  static function helper(): void {}\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `self::make();` with no assignment, where `make` returns `string` —
    /// the call's `string` result is refcounted and nothing ever binds it, so
    /// `Lowering::lower_expr_stmt` releases it immediately, right after the
    /// call, rather than leaking it: exactly one release, no retain (a call's
    /// own result is a fresh producer, per `is_aliasing_read`).
    #[test]
    fn a_bare_call_used_as_a_statement_releases_a_discarded_string_result() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    self::make();\n  }\n  static function make(): string {\n    return \"hi\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `new Foo();` with no assignment at all — a bare `new` used purely for
    /// a constructor's side effect. The fresh instance has exactly one owner
    /// and nothing ever binds it, so `Lowering::lower_expr_stmt` releases it
    /// straight after the construction, the same way it already did a
    /// discarded `string` result above.
    #[test]
    fn a_bare_new_used_as_a_statement_releases_the_discarded_instance() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  function constructor() {}\n}\nclass T {\n  function m(): void {\n    new Foo();\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->greet();` with no assignment — a bare *instance* call as a
    /// statement, not just a static one, to make sure `lower_expr_stmt`'s
    /// dispatch isn't accidentally `StaticCall`-only.
    #[test]
    fn a_bare_instance_call_used_as_a_statement_lowers_too() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  function greet(): void {}\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    $obj->greet();\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->name = "new";` — a `string`-typed property write from a fresh
    /// literal. `Lowering::lower_reassignment`'s property-target arm reads
    /// the field's previous value back with a `FieldGet` and releases it, but
    /// needs no retain of the new value: a literal already has exactly one
    /// natural owner (`is_aliasing_read` is `false` for `ExprKind::Str`),
    /// same as any other durable-slot bind.
    #[test]
    fn writing_a_fresh_string_literal_to_a_property_releases_its_previous_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public string $name = \"orig\";\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    $obj->name = \"new\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->make()->name` — a field read whose base is a *temporary*. The
    /// call hands back the only reference to the object, and `FieldGet`
    /// borrows out of its slot, so the read retains its own result before the
    /// base is released: the two together make the whole expression a fresh
    /// producer, which is what `Lowering::aliasing_read` reports it as.
    #[test]
    fn a_field_read_off_a_temporary_retains_its_result_and_releases_the_base() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public string $name = \"orig\";\n}\nclass Maker {\n  function make(): Foo { return new Foo(); }\n}\nclass T {\n  function m(): void {\n    Maker $obj = new Maker();\n    string $s = $obj->make()->name;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->rows()["k"]` — an *element* read whose base is a temporary, the
    /// same shape as the field read above one storage kind along.
    /// `InstKind::ArrayGet` borrows out of the array the call handed back, so
    /// the read retains its own result before the array is released, and the
    /// whole expression is a fresh producer.
    #[test]
    fn an_index_read_off_a_temporary_retains_its_result_and_releases_the_base() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Maker {\n  function rows(): array<string> { return [\"k\" => \"v\"]; }\n}\nclass T {\n  function m(): void {\n    Maker $obj = new Maker();\n    string $s = $obj->rows()[\"k\"];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->name = $s;` — writing an aliasing local into a property retains
    /// the new value first (same order `Lowering::bind_local` uses for a
    /// local target), then reads and releases the field's previous value —
    /// retain before release, so a self-assignment through the same slot
    /// would never observe a transient zero refcount.
    #[test]
    fn writing_a_string_local_to_a_property_retains_it_before_releasing_the_old_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public string $name = \"orig\";\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    string $s = \"hi\";\n    $obj->name = $s;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$this->name = "new";` — a property write through the implicit
    /// receiver, exercising the same `$this`/`Env` lookup path
    /// `Lowering::lower_expr`'s `PropertyAccess` read arm already shares with
    /// an ordinary local receiver.
    #[test]
    fn writing_through_this_lowers_too() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  public string $name = \"orig\";\n  function m(): void {\n    $this->name = \"new\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A property write through a plain-`object` receiver takes the same
    /// name-keyed `SlotSet` the read side's `SlotGet` mirrors — the whole of
    /// ADR 0036 § 4's erased write. The value is widened to `Ty::Tagged`
    /// first: the field's real type is the receiving class's to state, and
    /// `mwl_runtime::mwl_object_slot_set` is where it is checked.
    #[test]
    fn writing_through_a_plain_object_receiver_writes_by_name() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(object $o): void {\n    $o->x = 1;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `"a" . "b"` — two fresh literal operands lower to a single
    /// `InstKind::Concat`, with no retain of either operand (each is only
    /// read to build the new buffer, exactly the way `InstKind::FieldGet`
    /// reads its `object` receiver without retaining it). Each is a fresh,
    /// non-aliasing value with no durable slot of its own — a bare `Str`
    /// literal isn't `is_aliasing_read` — so each gets exactly one release
    /// right after `Concat` reads it, the same "release a fresh value once
    /// its one and only use is done" precedent a bare call/`new` statement
    /// already sets; the concatenation's own result needs no retain or
    /// release at all when it's returned directly — a fresh producer, same
    /// as a literal or a call's result, transferring straight out.
    #[test]
    fn concatenating_two_string_literals_needs_no_retain_of_either_operand() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): string {\n    return \"a\" . \"b\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a . $b` — both operands are aliasing reads of an existing local, but
    /// `InstKind::Concat` only *reads* them to build the new buffer; neither
    /// local's own slot is retained on the way in, since concatenation never
    /// becomes a second durable owner of either operand the way binding one
    /// to a new local would. `$a`/`$b`'s own slots still get their ordinary
    /// one release each at `m`'s exit sweep, and the concatenation's own
    /// result — bound to `$c` here, an aliasing read of nothing — needs no
    /// retain either, only the release `release_all_locals` gives every
    /// refcounted local still live at return.
    #[test]
    fn concatenating_two_string_locals_reads_them_without_retaining() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    string $a = \"x\";\n    string $b = \"y\";\n    string $c = $a . $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `1 . "x"` — an `int` operand on the `.` side that `mwl_types::expr::
    /// check_expr`'s own `require_stringable` happily accepts (PHP-style
    /// implicit to-string) converts through a new `InstKind::HelperCall`
    /// (`Helper::IntToString`) before reaching `InstKind::Concat`. Both the
    /// helper-call result and the `"x"` literal are fresh, non-aliasing
    /// values with no durable slot of their own, so both get released right
    /// after `Concat` reads them — the concatenation's own result is
    /// returned directly and needs no release at all.
    #[test]
    fn concatenating_an_int_literal_with_a_string_uses_a_helper_call() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): string {\n    return 1 . \"x\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$flag . "!"` — a `bool` local read (an aliasing read of its own
    /// slot) converts through `Helper::BoolToString`; the conversion result
    /// is still a fresh, non-aliasing `Ty::Str` value (the `bool` itself was
    /// never refcounted, so there was nothing to alias into the conversion),
    /// so it's released right after `Concat` reads it, same as the `int`
    /// case above. `$flag`'s own slot needs no release from `Concat` at all
    /// — it isn't `Ty::is_refcounted`, so `release_all_locals` skips it too.
    #[test]
    fn concatenating_a_bool_local_with_a_string_uses_a_helper_call() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $flag): string {\n    return $flag . \"!\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj . "x"` where `$obj`'s class implements `Stringable` — ADR 0028
    /// § 1's implicit conversion, desugared to the `toString()`
    /// `mwl_types::expr::operators::require_stringable` resolved under the
    /// operand's own span. A `.` operand is not itself a call expression, so
    /// there is no `ExprInfo::Call` for it the way an actual
    /// `$obj->toString()` site would have; the checker's own side map
    /// (`ExprTypeTable::to_string_call`) is what carries the target across.
    ///
    /// It dispatches through `InstKind::ClassDescOf`/`CallVirtual` so an
    /// override wins, carries ADR 0002's error edge because a `toString` body
    /// may throw, and retains `$n` first — the parameter's slot still owns it,
    /// and the callee releases every refcounted parameter at its own exit.
    #[test]
    fn concatenating_a_stringable_object_operand_calls_its_to_string() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Name implements Stringable {\n  public function toString(): string { return \"x\"; }\n}\nclass T {\n  public function m(Name $n): string {\n    return $n . \"x\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj as string` — ADR 0007 § 2's explicit spelling of the very same
    /// conversion, reaching the very same `Self::lower_to_string_call` rather than
    /// getting a second answer of its own, exactly as `as bool` reuses
    /// ADR 0035's truthy table.
    #[test]
    fn converting_a_stringable_object_to_string_calls_its_to_string() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Name implements Stringable {\n  public function toString(): string { return \"x\"; }\n}\nclass T {\n  public function m(Name $n): string {\n    return $n as string;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    // `bytes` is the mechanical follow-on to `string` the crate docs named:
    // same `Ty::Bytes` representation, same `Lowering::bind_local`/
    // `lower_call_args`/`release_all_locals`/`lower_reassignment` insertion
    // points `Ty::Str` already uses. `mwl-syntax`'s grammar has no `bytes`
    // literal syntax at all (no `b"..."` form), so unlike the `string` tests
    // above, every fixture below sources its `bytes` value from a parameter
    // or a property read rather than a literal — both already-covered
    // `is_aliasing_read` shapes, so this still exercises the same policy a
    // literal-sourced fixture would.

    /// `bytes $b = $a;` aliases the parameter `$a`'s already-owned value —
    /// `Lowering::bind_local` retains it, exactly like the `string` analog
    /// above. `return $b;` transfers `$b`'s reference out directly (excluded
    /// from `Lowering::release_all_locals`'s sweep), leaving exactly one
    /// release for `$a`'s own slot at the exit sweep.
    #[test]
    fn a_bytes_parameter_bound_to_a_local_transfers_out_on_return() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function pick(bytes $a): bytes {\n    bytes $b = $a;\n    return $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `bytes $x = $a; $x = $b;` — reassigning a `bytes` local to a second
    /// aliasing parameter retains the new value first, then releases the
    /// value `$x` previously held, the same order `Lowering::bind_local`
    /// always uses. At the exit sweep every local still live — `$a`, `$b` and
    /// `$x` (now aliasing `$b`'s storage) — gets its own release: `$a`'s
    /// storage ends up released twice in total (once when `$x` moves off it,
    /// once for `$a`'s own slot), which is correct rather than a double free
    /// — two live slots (`$a`, and `$x` before the reassignment) really did
    /// hold two independent references to it.
    #[test]
    fn reassigning_a_bytes_local_retains_the_new_value_and_releases_the_old() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bytes $a, bytes $b): void {\n    bytes $x = $a;\n    $x = $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Passing a `bytes` local as a call argument retains it first —
    /// `Lowering::lower_call_args`'s aliasing check, exactly mirroring the
    /// `string` analog above. `take` returns `int`, isolating the
    /// argument-side retain from any return-side question.
    #[test]
    fn passing_a_bytes_local_as_a_call_argument_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bytes $s): int {\n    return self::take($s);\n  }\n  static function take(bytes $x): int {\n    return 1;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `var $s = $obj->data;` — a `bytes`-typed property read is an aliasing
    /// read exactly like a `string` one, so binding it to a new local retains
    /// the field's own value. `Foo`'s `bytes` property has no literal default
    /// available (see this block's own note), so its constructor assigns it
    /// from a `bytes` parameter instead — ADR 0022's definite-initialization
    /// obligation either way.
    #[test]
    fn binding_a_bytes_property_read_to_a_local_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public bytes $data;\n  function constructor(bytes $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(bytes $seed): void {\n    Foo $obj = new Foo($seed);\n    var $s = $obj->data;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->data = $other;` — writing an aliasing `bytes` local into a
    /// property retains the new value first, then reads and releases the
    /// field's previous value, the same order `Lowering::lower_reassignment`'s
    /// property-target arm always uses for a refcounted field.
    #[test]
    fn writing_a_bytes_local_to_a_property_retains_it_before_releasing_the_old_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public bytes $data;\n  function constructor(bytes $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(bytes $seed, bytes $other): void {\n    Foo $obj = new Foo($seed);\n    $obj->data = $other;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    // `array<T>` is the fourteenth slice — `Ty::Array` is a bare, opaque
    // representation exactly like `Ty::Object` (see that variant's own doc
    // comment), and `InstKind::ArrayNew` is the fixed-shape literal
    // instruction it needs. The fixtures immediately below are all
    // *positional* literals — no explicit `key =>` — which keep the single-
    // `ArrayNew` shape; the explicit-`key =>` fixtures further down cover the
    // `ArrayNew` (empty) + `ArraySet`* shape, which a `...spread` element
    // takes too. `&value` never reaches here at all, `mwl_types` refusing
    // it as `E0483`.

    /// `[]` — an empty array literal lowers to `InstKind::ArrayNew` with no
    /// entries at all, still a well-formed fresh `Ty::Array` value.
    #[test]
    fn an_empty_array_literal_lowers_with_no_entries() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): array {\n    return [];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `[1, 2, 3]` — three fresh, non-aliasing `int` elements, auto-numbered
    /// `"0"`/`"1"`/`"2"`. None of them is `Ty::is_refcounted`, so no retain is
    /// emitted for any entry — only the array's own slot gets a release at
    /// `m`'s exit sweep.
    #[test]
    fn a_literal_with_fresh_scalar_elements_needs_no_retain() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [1, 2, 3];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `[$s]` — a `string` local read is an aliasing read
    /// (`lower::is_aliasing_read`), so the element is retained before the
    /// array durably owns it, the same policy `Lowering::lower_call_args`
    /// already applies at a call-argument boundary. `$s`'s own slot still
    /// gets its ordinary release at `m`'s exit sweep, alongside the array's.
    #[test]
    fn a_literal_with_an_aliasing_element_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    string $s = \"hi\";\n    array $a = [$s];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `["k" => 1]` — a literal with an explicit `key =>` element lowers to
    /// an empty `ArrayNew` plus one `ArraySet`: `"k"` is a fresh `ConstStr`
    /// (a source-literal string is never an aliasing read), so it needs no
    /// retain of its own, mirroring the value `1`.
    #[test]
    fn a_string_literal_keyed_array_element_lowers_to_array_new_then_array_set() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [\"k\" => 1];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `[5 => "a"]` — an explicit `int` key travels to the `InstKind::ArraySet`
    /// chain unrendered, exactly as an `$arr[$i]` subscript does: this is the
    /// same `Lowering::lower_array_key` on both sides, which is why the two
    /// can never drift.
    #[test]
    fn an_int_literal_keyed_array_element_carries_the_integer_unrendered() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [5 => \"a\"];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `[$k => 1]` — a `string` local used as an explicit key is an aliasing
    /// read, so it is retained before the array durably owns it, exactly the
    /// policy `Lowering::lower_reassignment`'s `Index`-target arm already
    /// gives `$a[$k] = 1;`.
    #[test]
    fn a_dynamic_string_keyed_array_element_retains_the_key() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(string $k): void {\n    array $a = [$k => 1];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `[1, "k" => 2, 3]` — a positional element mixed with an explicit key
    /// still numbers from "how many positional elements came before it" —
    /// `"0"`, then `"1"` for the trailing `3` — not PHP's real "continues
    /// from the highest int key used so far" rule (see
    /// `ir::InstKind::ArrayNew`'s own doc comment for why that's a
    /// deliberate, documented simplification rather than a bug).
    #[test]
    fn a_positional_element_after_an_explicit_key_keeps_its_own_position_counter() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [1, \"k\" => 2, 3];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `[...$a]` — one `InstKind::ArraySpread` per spread element, over the
    /// same empty-`ArrayNew` shape an explicit key already takes. The subject
    /// is a local read, so it is *borrowed* and no retain is emitted beside
    /// the copy: what the destination ends up owning is a fresh reference per
    /// entry, and the runtime takes it.
    #[test]
    fn a_spread_element_copies_the_subject_into_the_literal() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [1];\n    array $b = [...$a];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `[0, ...$a, 2]` — a keyless element of a literal that contains a
    /// spread is an `InstKind::ArrayAppend`, not a lowering-time index: how
    /// many entries the spread contributed is the subject's own run-time
    /// length. The fixture two above is the contrast — with no spread in the
    /// literal, that counter is still this pass's.
    #[test]
    fn a_keyless_element_beside_a_spread_appends_instead_of_numbering() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [1];\n    array $b = [0, ...$a, 2];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A spread whose subject is a *fresh producer* — a call's result rather
    /// than a local read — is staged on `Lowering::owned_temporaries` and
    /// released once the copy has been emitted, because
    /// `InstKind::ArraySpread` borrows its subject rather than consuming it.
    /// The array under construction is on that same stack throughout, which
    /// is what gives the copy's own error edge something to release.
    #[test]
    fn a_spread_of_a_call_result_releases_the_subject_after_the_copy() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $b = [...self::rows()];\n  }\n  static function rows(): array {\n    return [1];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Passing an `array` local as a call argument retains it first —
    /// `Lowering::lower_call_args`'s aliasing check, exactly mirroring the
    /// `string`/`bytes` analogs above. `lower_checked_ty`'s new
    /// `CheckedTy::Array(_) => Ty::Array` arm is what makes this boundary
    /// work with no new insertion point of its own.
    #[test]
    fn passing_an_array_local_as_a_call_argument_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    array $a = [1];\n    return self::take($a);\n  }\n  static function take(array $x): int {\n    return 1;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `var $s = $obj->data;` — an `array`-typed property read is an
    /// aliasing read exactly like `string`/`bytes`, so binding it to a new
    /// local retains the field's own value. `Foo`'s `array` property has no
    /// literal default available in a property initializer the way a scalar
    /// one would, so its constructor assigns it from an `array` parameter
    /// instead — ADR 0022's definite-initialization obligation either way.
    /// The constructor call itself also exercises an array literal
    /// (`[1]`) passed as a resolved call argument, not just a local bind.
    #[test]
    fn binding_an_array_property_read_to_a_local_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public array $data;\n  function constructor(array $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo([1]);\n    var $s = $obj->data;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$obj->data = $other;` — writing an aliasing `array` local into a
    /// property retains the new value first, then reads and releases the
    /// field's previous value, the same order
    /// `Lowering::lower_reassignment`'s property-target arm always uses for
    /// a refcounted field.
    #[test]
    fn writing_an_array_local_to_a_property_retains_it_before_releasing_the_old_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass Foo {\n  public array $data;\n  function constructor(array $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo([1]);\n    array $other = [2];\n    $obj->data = $other;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a[0]` through an `array<int>` parameter — the simplest array-access
    /// read: a fresh, non-refcounted `int` element, and a literal `int` key
    /// that reaches `InstKind::ArrayGet` as the `int` it already was, with no
    /// `helper.int_to_string` and therefore no key allocation and no release
    /// of one either. ADR 0007 § 5 still says the key *is* `"0"`; `mwl-ir`'s
    /// module doc § *an array key is a `string`, and an `int` subscript no
    /// longer spells it* is why the decimal is no longer rendered to reach
    /// it, and codegen picks `mwl_array_get_index` off this operand's `Ty`.
    #[test]
    fn reading_an_int_element_through_a_literal_key_carries_the_integer_unrendered() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): int {\n    return $a[0];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a[$i]` with a `uint` subscript — one of the two subscripts
    /// `Lowering::lower_array_key` still renders, because the runtime's index
    /// ABI is an `i64` and a `uint` above `i64::MAX` has no `i64` spelling
    /// naming the same key. So `helper.uint_to_string` is still in this
    /// output, and the fresh key it produces is still released right after
    /// the borrow — the shape the `int` case above no longer has.
    #[test]
    fn reading_an_element_through_a_uint_subscript_still_renders_the_decimal() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a, uint $i): int {\n    return $a[$i];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$out .= $piece;` on a plain `string` local — one `str.append` and
    /// nothing else. No `concat`, because there is no fresh buffer to build;
    /// no retain of the suffix, which `$piece`'s own slot still owns; and no
    /// release of the old `$out`, because `InstKind::StrAppend` consumes that
    /// reference and yields the one the binding is re-pointed at. That is
    /// `InstKind::ArraySet`'s protocol, which that variant's doc comment owns.
    #[test]
    fn appending_to_a_string_local_appends_in_place_rather_than_concatenating() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(string $piece): string {\n    var $out = \"\";\n    \
             $out .= $piece;\n    return $out;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$this->p .= "x";` keeps the `$x = $x . e` rewrite: a property target
    /// already needs the `FieldSet` write-back the rewrite performs, so
    /// `Lowering::lower_string_append`'s one-slot bookkeeping does not reach
    /// it and `concat` is still what runs. An `int` suffix on a `string`
    /// local is the same story one operand along — it is converted through
    /// `helper.int_to_string` first, then appended.
    #[test]
    fn appending_to_a_property_keeps_the_concat_rewrite() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  public string $p = \"\";\n  function m(): void {\n    \
             $this->p .= \"x\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `unset($a[0]);` — the other subscript that still renders.
    /// `InstKind::ArrayUnset` has no index-shaped runtime primitive beside
    /// it, so `Lowering::lower_rendered_array_key` forces the decimal here
    /// rather than letting codegen discover it cannot. Contrast
    /// `writing_an_int_element_through_a_literal_key_carries_the_integer_unrendered`,
    /// which is the same literal key one instruction along.
    #[test]
    fn unsetting_an_element_through_an_int_key_still_renders_the_decimal() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): void {\n    unset($a[0]);\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a[$k]` where both the array's element and the key are `string`
    /// locals — the key is already `Ty::Str` and is a bare variable read
    /// (`is_aliasing_read`), so it needs no conversion and, unlike the
    /// literal-key case above, is *not* released after the read (`$k`'s own
    /// slot still owns it). Binding the `array<string>` element itself to
    /// `$s` retains it first, since `ExprKind::Index` is now one of
    /// `is_aliasing_read`'s recognized shapes — exactly the same policy a
    /// property read already gets.
    #[test]
    fn reading_a_string_element_through_a_string_local_key_retains_the_result() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<string> $a, string $k): void {\n    var $s = $a[$k];\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a[0] = 5;` through an `array<int>` parameter — the simplest
    /// array-element write: a fresh, non-refcounted `int` value (no retain)
    /// and a literal `int` key carried unrendered exactly the way the read
    /// side carries it, with no old-value get/release pair at all
    /// (`InstKind::ArraySet`'s own doc comment explains why an ordinary
    /// new-or-existing-key write bundles that into one instruction rather
    /// than splitting it like `FieldSet` does).
    #[test]
    fn writing_an_int_element_through_a_literal_key_carries_the_integer_unrendered() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): void {\n    $a[0] = 5;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a[$k] = $v;` where the array's element, the key, and the new value
    /// are all `string` locals — both the key and the value are aliasing
    /// reads of their own slots, so both get retained before
    /// `InstKind::ArraySet` runs; `$a`/`$k`/`$v` each still get their
    /// ordinary release at `m`'s exit sweep.
    #[test]
    fn writing_a_string_element_through_a_string_local_key_retains_both_key_and_value() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<string> $a, string $k, string $v): void {\n    $a[$k] = $v;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a[] = 1;` — PHP's append syntax lowers to `InstKind::ArrayAppend`
    /// with no key at all, unlike every other `Index`-target write: a fresh,
    /// non-refcounted `int` value needs no retain, mirroring
    /// `writing_an_int_element_through_a_literal_key_carries_the_integer_unrendered`
    /// but with no `lower_array_key` call in the output at all, since there is
    /// no key to lower.
    ///
    /// It is the one array write with an error edge (` ! bb1`), so the snapshot
    /// also carries a landing block — see `Lowering::emit_array_append`.
    #[test]
    fn appending_a_fresh_int_value_needs_no_retain() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): void {\n    $a[] = 1;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a[] = $v;` where the appended value is a `string` local — an
    /// aliasing read of `$v`'s own slot, so it is retained before
    /// `InstKind::ArrayAppend` runs, the same policy
    /// `writing_a_string_element_through_a_string_local_key_retains_both_key_and_value`
    /// already gives an explicit key's value; `$a`/`$v` each still get their
    /// ordinary release at `m`'s exit sweep.
    ///
    /// The landing block releases both locals and nothing more: the refusal
    /// path inside `mwl_runtime::mwl_array_append` releases the extra reference
    /// the retain above staged, and leaves the array's where this frame's own
    /// slot still names it.
    #[test]
    fn appending_an_aliasing_string_value_retains_it() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<string> $a, string $v): void {\n    $a[] = $v;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// Two writes into one local: the second reads the array the *first*
    /// yielded, not the one the parameter arrived as, and the exit sweep
    /// releases the last one only. That chain is ADR 0007 § 5's copy-on-write
    /// separation being written back — see `Lowering::write_back_array` — and
    /// it is the whole reason `InstKind::ArraySet` defines a value.
    #[test]
    fn a_second_write_reads_the_array_the_first_one_yielded() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): void {\n    $a[0] = 5;\n    $a[1] = 6;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$this->rows[$k] = $v;` — the other holder a separation can be written
    /// back to. The yielded array goes straight into the property slot with a
    /// bare `field.set` and no retain or release: the reference the write
    /// consumed was the slot's own, and the one it produced replaces it there.
    #[test]
    fn writing_an_element_through_a_property_base_stores_the_result_back() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  public array<int> $rows;\n  function m(): void {\n    $this->rows[0] = 5;\n  }\n  function constructor() { $this->rows = []; }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$grid[0][1] = 5;` — a nested subscript separates *every* level of the
    /// chain and writes each one back in turn, which the snapshot reads as
    /// one `array_row_for_write` descending and two `array.set`s climbing
    /// back out, outermost last. The row helper is what makes the descent's
    /// ownership uniform (see `ir::Helper::ArrayRowForWrite`), so there is no
    /// retain beside it and no branch for the absent key.
    #[test]
    fn writing_through_a_nested_subscript_separates_every_level() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<array<int>> $g): void {\n    $g[0][1] = 5;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A `bool` subscript isn't one of ADR 0007 § 5's three legal key source
    /// types (`int`/`uint`/`string`) — `mwl_types::expr::check_array_key_type`
    /// now rejects it at check time (see `mwl_types::check`'s own
    /// `a_bool_key_array_literal_is_diagnosed`-style fixtures for the
    /// diagnostic side), so `lower_first_method`'s own `check_program` call
    /// already fails the fixture before lowering ever runs —
    /// `Lowering::lower_array_key`'s `other` panic arm is unreachable for
    /// this input now, not the thing this test demonstrates.
    #[test]
    #[should_panic(expected = "fixture failed to check")]
    fn a_bool_subscript_key_is_rejected_before_lowering_even_runs() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a, bool $b): void {\n    $a[$b] = 1;\n  }\n}\n",
        );
    }

    /// `$a && $b` — ADR 0035's short-circuit `&&`, the nineteenth slice's
    /// first new form: `$a`'s own truthy test branches straight to a merge
    /// block carrying `const.bool false` when falsy, only evaluating `$b`
    /// (through its own truthy test) on the truthy path — `Lowering::
    /// lower_and`'s branch/`Phi`-merge shape.
    #[test]
    fn and_short_circuits_to_a_phi() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $a, bool $b): bool {\n    return $a && $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a || $b` — `Lowering::lower_or`'s mirror of `and_short_circuits_to_a_phi`:
    /// the short-circuit edge (truthy `$a`) carries `const.bool true` instead,
    /// and `$b` is only evaluated when `$a` is falsy.
    #[test]
    fn or_short_circuits_to_a_phi() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $a, bool $b): bool {\n    return $a || $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `!$s` with a `string` operand — fixes a latent bug the seventeenth
    /// slice's own table left behind: unary `!` previously passed its
    /// operand's own type straight through as the result type (only
    /// coincidentally correct for the one existing fixture, which negated an
    /// already-`bool` local) instead of always producing `Ty::Bool` per ADR
    /// 0035. `$s` converts through `Helper::StrTruthy` first, then negates —
    /// `$s` is a bare parameter read (`is_aliasing_read`), so no release
    /// follows the helper call, same as any other truthy-tested aliasing
    /// read.
    #[test]
    fn not_converts_a_non_bool_operand_through_the_truthy_table_then_negates() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(string $s): bool {\n    return !$s;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `!($a && $b)` — `!`'s operand is itself a short-circuit `&&`, composing
    /// through `Lowering::lower_not`'s own `Lowering::lower_expr` call.
    #[test]
    fn not_composes_with_a_short_circuit_and() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $a, bool $b): bool {\n    return !($a && $b);\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `if ($a && $b)` — an `if`'s own condition is itself a short-circuit
    /// `&&`, exercising `Lowering::lower_if`'s updated call into
    /// `Lowering::lower_truthy_cond` with a mutable `cur` that `&&`'s own
    /// branch/merge shape gets to redirect before the `if`'s own `Branch`
    /// terminator is sealed.
    #[test]
    fn if_condition_short_circuits_with_and() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $a, bool $b): bool {\n    if ($a && $b) {\n      return true;\n    }\n    return false;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `while ($a || $b)` — exercises the loop-header adjustment
    /// `Lowering::lower_while` needed to host a branching condition at all:
    /// the header phi for `$a` still lives in the fixed loop-header block,
    /// but the loop's own `Branch` terminator now seals onto `cond_end`
    /// (wherever `||`'s own merge block ended up), not the header block
    /// itself.
    #[test]
    fn while_condition_short_circuits_with_or() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $a, bool $b): void {\n    while ($a || $b) {\n      $a = false;\n    }\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$c ? $a : $b` with both branches the same `int` — `Lowering::
    /// lower_ternary`'s ordinary (non-elvis) shape: `cond`'s own value is
    /// released once `truthy_convert` reads it (nothing reuses it, unlike
    /// elvis), and the two branches join through a fresh `Phi`.
    #[test]
    fn ternary_with_matching_branch_types_merges_with_a_phi() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $c, int $a, int $b): int {\n    return $c ? $a : $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$a ?: $b` (elvis) where `$a` is a non-refcounted `int` — the truthy
    /// path reuses `$a`'s own value as the ternary's result with neither a
    /// retain nor a release, since `Ty::Int` isn't `is_refcounted` at all;
    /// this is the "nothing to own" half of elvis's reuse rule.
    #[test]
    fn elvis_with_a_non_refcounted_condition_needs_no_retain() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(int $a, int $b): int {\n    return $a ?: $b;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$s ?: $d` (elvis) where `$s` is a `string` parameter — `$s` is an
    /// aliasing read (its own parameter slot still owns it), so reusing it as
    /// the truthy path's value needs a retain (a second, independent owner:
    /// the ternary's own result) rather than the release every other
    /// truthy-tested position would apply here.
    #[test]
    fn elvis_retains_an_aliased_refcounted_condition() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(string $s, string $d): string {\n    return $s ?: $d;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `self::make() ?: \"x\"` (elvis) where `cond` is a *fresh*, non-aliasing
    /// `string` (a call's own result) — the opposite corner from
    /// `elvis_retains_an_aliased_refcounted_condition`: reusing it needs
    /// neither a retain nor a release, since it already has exactly one
    /// owner, which simply transfers to become the ternary's result.
    #[test]
    fn elvis_transfers_a_fresh_refcounted_condition_with_no_retain_or_release() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(): string {\n    return self::make() ?: \"x\";\n  }\n  static function make(): string {\n    return \"y\";\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A ternary whose `then`/`else` branches lower to two different
    /// `crate::ty::Ty` representations (`int` vs `string`) — each branch is
    /// tagged in its **own** block, ahead of its jump, and the phi carries
    /// `Ty::Tagged`, which is exactly what `erase_checked_ty` gives the union
    /// the checker already typed the whole expression as. See
    /// `Lowering::join_representations` for why that is the erasure rather
    /// than a promotion of one side into the other.
    #[test]
    fn a_ternary_with_mismatched_branch_types_joins_at_the_tagged_representation() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $c): mixed {\n    mixed $r = $c ? 1 : \"x\";\n    return $r;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// The same rule over more than two branches: a `match` whose arms lower
    /// to `int`, `float` and `string` tags each of them in its own arm block
    /// and joins at one `Ty::Tagged` phi — `Lowering::lower_match` shares
    /// `Lowering::join_representations` with the ternary rather than owning a
    /// second rule.
    #[test]
    fn match_arms_in_three_representations_join_at_the_tagged_representation() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(int $k): mixed {\n    mixed $r = match ($k) { 1 => 1, 2 => 2.5, default => \"x\" };\n    return $r;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A short-circuiting `&&` nested inside a **call argument** — the
    /// position that had no `&mut BlockId` to redirect and so panicked, which
    /// was this crate's known gap 5. `Lowering::lower_expr` now owns one, so
    /// the argument's own branch/merge is spliced into the caller's block
    /// chain and the call is emitted in whichever block the merge ended in.
    #[test]
    fn a_short_circuit_and_nested_in_a_call_argument_composes() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $a, bool $b): void {\n    self::take($a && $b);\n  }\n  static function take(bool $x): void {}\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A plain `break;` inside a nested `if`, with no reassignment along the
    /// break path that would ever differ from the loop's own steady-state
    /// value — `Lowering::merge_envs` degenerates to a plain clone (the same
    /// `[(_, only)]` shape a break-free loop already produced) rather than a
    /// spurious phi, but the CFG itself gains the extra break block/edge:
    /// this is mostly a shape test confirming `break` lowers to a `Jump`
    /// straight to the after-block at all, before the next test exercises a
    /// case where the merge actually needs a fresh phi.
    #[test]
    fn while_loop_with_a_plain_break_exits_early() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(int $n): int {\n    int $i = 0;\n    while ($i < $n) {\n      if ($i == 3) {\n        break;\n      }\n      $i = $i + 1;\n    }\n    return $i;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `break` after a reassignment the loop's own back edge never sees
    /// (`$r = $i;` runs every iteration, but the `break` fires before the
    /// bottom-of-body value the header phi's back edge would otherwise
    /// carry) — the after-block's own environment now needs a real
    /// `Lowering::merge_envs`-built phi for `$r`, combining the condition's
    /// ordinary false edge (the loop-steady-state phi value) with the
    /// break's own edge (that iteration's fresher value), not just a plain
    /// clone of `header_env` the way a break-free loop always produced.
    #[test]
    fn break_merges_a_differing_value_into_the_after_block() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(int $n): int {\n    int $i = 0;\n    int $r = 0;\n    while ($i < $n) {\n      $r = $i;\n      if ($i == 3) {\n        break;\n      }\n      $i = $i + 1;\n    }\n    return $r;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `continue` inside a nested `if` adds a *third* incoming edge to
    /// `$sum`'s header phi, alongside the pre-loop edge and the body's own
    /// fall-through back edge — `$sum` is skipped (via `continue`) on the
    /// iteration where `$i == 3`, so that edge's value genuinely differs
    /// from the fall-through edge's, confirming
    /// `Lowering::lower_while`'s combined `back_edges` (fall-through plus
    /// every recorded `continue`) all reach the same phi, not just the
    /// fall-through edge alone.
    #[test]
    fn continue_adds_another_incoming_edge_to_the_header_phi() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(int $n): int {\n    int $i = 0;\n    int $sum = 0;\n    while ($i < $n) {\n      $i = $i + 1;\n      if ($i == 3) {\n        continue;\n      }\n      $sum = $sum + $i;\n    }\n    return $sum;\n  }\n}\n",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `break 2;` targets the *outer* loop's after-block, not the inner
    /// one's — the whole point of a level, and the half a jump could get
    /// wrong while still lowering to something well-formed.
    ///
    /// Read off the rendering rather than snapshotted: what is pinned is
    /// which block the jump names, and a snapshot would go red for any
    /// unrelated renumbering while saying nothing about that. `break;` in the
    /// same position is lowered beside it, so the assertion is that the two
    /// name **different** blocks rather than that either names a particular
    /// one.
    #[test]
    fn a_multi_level_break_leaves_the_loop_its_level_names() {
        let nested = |keyword: &str| {
            let (f, map, file) = lower_first_method(&format!(
                "<?mwl\nclass T {{\n  function m(int $n): void {{\n    while ($n > 0) {{\n      \
                 while ($n > 0) {{\n        {keyword};\n      }}\n      echo \"inner-done\";\n    \
                 }}\n  }}\n}}\n",
            ));
            print_function(&f, map.file(file))
        };
        let one = nested("break");
        let two = nested("break 2");
        assert_ne!(
            one, two,
            "`break 2` lowered to the same jump a `break` does"
        );
    }

    /// `continue 2` written inside a `switch` inside one loop is PHP's own
    /// idiomatic spelling for "continue the enclosing loop", and it lowers to
    /// exactly the back edge a bare `continue` there already takes — the
    /// `switch` counts as a level and the walk outward finds the loop.
    ///
    /// `mwl_ir::lower::Lowering::lower_continue` is where both steps live and
    /// why; `docs/adr/README.md`'s paragraph on `continue` inside a `switch`
    /// is the decision's home.
    #[test]
    fn a_continue_level_walks_out_of_a_switch_to_the_loop() {
        let inside_switch = |keyword: &str| {
            let (f, map, file) = lower_first_method(&format!(
                "<?mwl\nclass T {{\n  function m(int $n): void {{\n    while ($n > 0) {{\n      \
                 $n = $n - 1;\n      switch ($n) {{\n        case 1:\n          {keyword};\n      \
                   default:\n          echo \"tick\";\n      }}\n    }}\n  }}\n}}\n",
            ));
            // Without the `; stmt @l:c-l:c` markers, which differ only
            // because `continue 2` is two characters longer than `continue`.
            print_function(&f, map.file(file))
                .lines()
                .filter(|l| !l.trim_start().starts_with("; stmt"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(
            inside_switch("continue"),
            inside_switch("continue 2"),
            "`continue 2` inside a `switch` did not reach the loop a bare `continue` does"
        );
    }

    /// Runs the whole front end over `src` and lowers the file — every class
    /// method, the script frame, and the class table.
    fn lower_whole_file(src: &str) -> crate::ir::Program {
        lower_whole_file_with_src(src).0
    }

    /// [`lower_whole_file`], keeping the source map so a test can render one
    /// of the lowered functions with [`print_function`].
    fn lower_whole_file_with_src(src: &str) -> (crate::ir::Program, SourceMap, SourceId) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = mwl_hir::resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut checked_types = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        let files = [mwl_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }];
        let enums =
            mwl_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let layouts = mwl_types::build_class_layouts(&files, &module.graph);
        let program = lower_file(
            "<script>",
            &stmts,
            map.file(file),
            &exprs,
            &checked_types,
            &enums,
            &layouts,
        );
        (program, map, file)
    }

    /// The class table is carried straight through from `mwl-types`, sorted
    /// by label so an unchanged file lowers identically every time.
    #[test]
    fn a_lowered_file_carries_its_class_table_in_label_order() {
        let program = lower_whole_file(concat!(
            "<?mwl\n",
            "interface Greets { public function greet(): string; }\n",
            "class Animal {\n",
            "  public int $legs;\n",
            "  function constructor(int $legs) { $this->legs = $legs; }\n",
            "}\n",
            "class Dog extends Animal implements Greets {\n",
            "  public string $name;\n",
            "  function constructor(string $name) {\n",
            "    parent::constructor(4);\n",
            "    $this->name = $name;\n",
            "  }\n",
            "  public function greet(): string { return $this->name; }\n",
            "}\n",
        ));

        // The seeded exception tree is in every program's table, so the
        // declared classes are checked by name rather than by position.
        let declared: Vec<&str> = program
            .classes
            .iter()
            .map(|class| class.label.as_str())
            .filter(|label| ["Animal", "Dog", "Greets"].contains(label))
            .collect();
        assert_eq!(declared, ["Animal", "Dog", "Greets"]);

        let by_label = |label: &str| {
            program
                .classes
                .iter()
                .find(|class| class.label == label)
                .unwrap_or_else(|| panic!("{label} should be in the class table"))
        };
        let dog = by_label("Dog");
        assert_eq!(dog.fields, ["legs", "name"]);
        assert_eq!(dog.conforms, ["Animal", "Greets"]);
        assert!(by_label("Greets").fields.is_empty());
    }

    /// Source with a `get`- and a `set`-hooked property, plus one ordinary
    /// one — the shape every hook test below reads.
    const HOOKED: &str = concat!(
        "<?mwl\n",
        "class Counter {\n",
        "  public int $hits;\n",
        "  public int $doubled {\n",
        "    get => $this->hits * 2;\n",
        "    set(int $v) { $this->hits = $v; }\n",
        "  }\n",
        "  function constructor(int $hits) { $this->hits = $hits; }\n",
        "  public function read(): int { return $this->doubled; }\n",
        "  public function write(int $n): void { $this->doubled = $n; }\n",
        "}\n",
    );

    /// ADR 0014 § 1's hooks are ordinary compiled functions, each under the
    /// label `mwl_types::signatures::hook_label` spells — the same one the
    /// access site's `InstKind::Call` names, which is why nothing here needs a
    /// dispatch table entry.
    #[test]
    fn each_property_hook_is_lowered_as_its_own_function() {
        let program = lower_whole_file(HOOKED);
        let names: Vec<&str> = program
            .functions
            .iter()
            .map(|f| f.name.as_str())
            .filter(|n| n.contains("$doubled"))
            .collect();
        assert_eq!(names, ["Counter::$doubled::get", "Counter::$doubled::set"]);

        let by_name = |name: &str| {
            program
                .functions
                .iter()
                .find(|f| f.name == name)
                .unwrap_or_else(|| panic!("{name} should have been lowered"))
        };
        // `get` takes only the receiver and returns the property's type;
        // `set` takes the incoming value in slot 1 and returns nothing.
        let get = by_name("Counter::$doubled::get");
        assert_eq!(get.params, [Ty::Object]);
        assert_eq!(get.ret, Ty::Int);
        let set = by_name("Counter::$doubled::set");
        assert_eq!(set.params, [Ty::Object, Ty::Int]);
        assert_eq!(set.ret, Ty::Void);
    }

    /// The point of the whole slice: `$this->doubled` is a **call**, not a
    /// field read. Before this landed it lowered to a `FieldGet` on a slot
    /// nothing ever wrote, which is why `examples/hooks.mwl` printed `0`.
    #[test]
    fn reading_a_get_hooked_property_calls_the_hook_instead_of_reading_the_slot() {
        let (program, map, file) = lower_whole_file_with_src(HOOKED);
        let read = program
            .functions
            .iter()
            .find(|f| f.name == "Counter::read")
            .expect("`read` should have been lowered");
        let text = print_function(read, map.file(file));
        assert!(text.contains("Counter::$doubled::get"), "{text}");
        assert!(!text.contains("field.get"), "{text}");
    }

    /// The write side, same shape: the assigned value is the accessor's one
    /// ordinary argument.
    #[test]
    fn writing_a_set_hooked_property_calls_the_hook_instead_of_writing_the_slot() {
        let (program, map, file) = lower_whole_file_with_src(HOOKED);
        let write = program
            .functions
            .iter()
            .find(|f| f.name == "Counter::write")
            .expect("`write` should have been lowered");
        let text = print_function(write, map.file(file));
        assert!(text.contains("Counter::$doubled::set"), "{text}");
        assert!(!text.contains("field.set"), "{text}");
    }

    /// Inside `$doubled`'s own hooks the property is its backing slot, never
    /// a re-entrant call — that is what lets a hook transform a stored value
    /// and still terminate. `mwl_types::Ctx::current_hook` is the rule; this
    /// is the lowering that proves it, on the one hook that touches its own
    /// property.
    #[test]
    fn a_hook_body_reaching_its_own_property_touches_the_slot_directly() {
        let (program, map, file) = lower_whole_file_with_src(concat!(
            "<?mwl\n",
            "class Box {\n",
            "  public int $n {\n",
            "    get => $this->n + 1;\n",
            "    set(int $v) { $this->n = $v * 2; }\n",
            "  }\n",
            "  function constructor() { $this->n = 1; }\n",
            "}\n",
        ));
        for (name, expected) in [("Box::$n::get", "field.get"), ("Box::$n::set", "field.set")] {
            let f = program
                .functions
                .iter()
                .find(|f| f.name == name)
                .unwrap_or_else(|| panic!("{name} should have been lowered"));
            let text = print_function(f, map.file(file));
            assert!(text.contains(expected), "{name}: {text}");
            // Read for a `call` naming it rather than for the name anywhere:
            // the signature line names the function, and so does the frame
            // label in an ADR 0002 landing block's `propagate` — which the
            // getter's `+ 1` now has, since ADR 0007 § 4 gives integer
            // arithmetic an overflow edge.
            let recursed = text
                .lines()
                .any(|line| line.contains("call") && line.contains(name));
            assert!(!recursed, "{name} recursed into itself: {text}");
        }
    }

    /// `foreach` over an `array<T>` with both bindings — the cursor's header
    /// phi, the retained loop-owned array reference under its reserved `Env`
    /// name, the step at the *top* of the body, and the per-iteration release
    /// of the key on the back edge. See `Lowering::lower_foreach`.
    #[test]
    fn a_foreach_walks_an_array_through_a_cursor_it_owns_a_reference_to() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
  function m(array<int> $a): void {
    foreach ($a as string $k => int $v) {
      echo $k;
    }
  }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A refcounted value binding is retained where a scalar one is not — the
    /// binding is a durable slot and `InstKind::ArrayValueAt` hands it a
    /// borrow, the same split `InstKind::ArrayGet` already has.
    #[test]
    fn a_refcounted_foreach_value_binding_is_retained_for_its_iteration() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
  function m(array<string> $a): void {
    foreach ($a as string $v) {
      echo $v;
    }
  }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `foreach` over an `Iterator<T>` — ADR 0053 § 3's third shape. Every
    /// member call is a `call.virtual`, never a static `call`: the interface
    /// declares both without a body, so there is no compiled function to
    /// name. The cursor is retained before each one, since a receiver is
    /// parameter 0 and MWL transfers an argument's reference to the callee.
    #[test]
    fn a_foreach_over_a_cursor_drives_advance_then_current_virtually() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
  function m(Iterator<int> $c): void {
    foreach ($c as int $v) {
      echo $v;
    }
  }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `foreach` over an `Iterable<T>` — the same loop with one `iterate()`
    /// ahead of it. The subject's own reference is *transferred* into that
    /// call rather than retained for it, which is why the subject never
    /// enters the `Env` and the loop's after-block releases the cursor
    /// instead.
    #[test]
    fn a_foreach_over_an_iterable_calls_iterate_once_before_the_loop() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
  function m(Iterable<int> $it): void {
    foreach ($it as int $v) {
      echo $v;
    }
  }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// ADR 0053 § 4's state-machine transform, end to end: the factory that
    /// runs no user code, the entry switch, one resumption arm per `yield`,
    /// and the spill/reload pair that lets a value cross a suspension that
    /// SSA cannot carry it across. `$limit` gets a loop-header phi despite
    /// never being reassigned — see `seed_generator_loop_carried`.
    #[test]
    fn a_generator_lowers_to_a_factory_a_state_class_and_a_resumption_switch() {
        let (p, map, file) = lower_program(
            "<?mwl
class G {
  static function upTo(int $limit): Iterator<int> {
    var $i = 1;
    while ($i <= $limit) {
      yield $i;
      $i = $i + 1;
    }
  }
}
",
        );
        assert_snapshot!(print_program(&p, map.file(file)));
    }

    /// ADR 0031's `fn` literal, lowered: the literal site allocates the
    /// captured-environment object and stores a *retained* snapshot of each
    /// capture into it, and the body becomes that class's one `invoke`, which
    /// reads every capture back out of parameter 0. See `lower_closure`,
    /// which owns the representation.
    #[test]
    fn a_closure_lowers_to_a_captured_environment_object_and_an_invoke_method() {
        let (p, map, file) = lower_program(
            "<?mwl
string $tag = \"t\";
int $bump = 1;
var $f = fn(int $n): string => $tag;
echo $bump;
",
        );
        assert_snapshot!(print_program(&p, map.file(file)));
    }

    /// A closure capturing an enclosing `&$x` parameter, which is the one
    /// capture whose `Env` entry is an address rather than a value: ADR 0031
    /// § 2 captures by value, so the field takes a `ref.load` snapshot of the
    /// cell at the literal, at the declared pointee type, and then the same
    /// retain every refcounted capture already takes. Both halves are visible
    /// here on purpose — the load alone would leave the environment object
    /// sharing the caller's one reference, and the field's `str` type is what
    /// says `invoke` reads a value rather than the caller's address, which is
    /// what makes the closure safe to outlive the call that staged the cell.
    #[test]
    fn a_closure_capturing_a_by_reference_parameter_snapshots_the_cell() {
        let (p, map, file) = lower_program(
            "<?mwl
class T {
  static function make(string &$s): callable {
    return fn (): string => $s;
  }
}
",
        );
        assert_snapshot!(print_program(&p, map.file(file)));
    }

    /// A refcounted element and a refcounted local both survive a
    /// suspension: the field takes its own reference on the way in and the
    /// resume block takes one on the way back out, so the two never share.
    #[test]
    fn a_generator_parks_a_refcounted_local_and_element_in_its_state_object() {
        let (p, map, file) = lower_program(
            "<?mwl
class G {
  static function two(): Iterator<string> {
    var $tag = \"t\";
    yield $tag;
    yield $tag;
  }
}
",
        );
        assert_snapshot!(print_program(&p, map.file(file)));
    }

    /// `unset($a[$k]);` — `InstKind::ArrayUnset` written back through the same
    /// holder a write uses, with the literal key released afterwards because
    /// the removal only borrows it.
    #[test]
    fn unsetting_an_element_writes_the_separated_array_back() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
  function m(array<int> $a): void {
    unset($a[\"k\"]);
  }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `foreach (… as &$v)` writes back through the array it is walking, which
    /// is the one shape copy-on-write separation has to be told *not* to
    /// separate. The snapshot is where that shows: no retain of the subject on
    /// the way in and no release after the loop, an `array.key_at`/`array.set`
    /// pair at the write rather than at the end of the iteration, and the
    /// subject's own binding carrying a header phi over what the last
    /// iteration wrote.
    #[test]
    fn a_by_reference_foreach_writes_through_to_the_array_it_walks() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
  function m(array<int> $a): void {
    foreach ($a as int &$v) {
      $v = $v + 1;
    }
  }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A Tier 0 `Core` member call: `core.call` naming the symbol
    /// `mwl_stdlib::registry` registered, with **no retain** on the array
    /// argument even though it is a refcounted aliasing read — a `Core` member
    /// borrows what it is handed. Contrast the ordinary static call in
    /// `a_self_static_call_with_a_scalar_return`'s snapshot, which retains.
    #[test]
    fn a_core_member_call_lowers_to_a_symbol_and_borrows_its_argument() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\n",
            "class T {\n",
            "  function m(array<int> $a): uint {\n",
            "    return Core\\Arr::count($a);\n",
            "  }\n",
            "}\n",
        ));
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A refcounted local declared *inside* a loop body is released on the
    /// back edge, not carried out of the loop: the next iteration restarts
    /// from the header environment, so nothing after this point could ever
    /// reach it. Without the release it leaked one reference per iteration —
    /// found by `examples/report.mwl`'s valgrind leg, which was the first
    /// fixture to declare one. See [`Lowering::end_iteration`].
    #[test]
    fn a_local_declared_in_a_loop_body_is_released_on_the_back_edge() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\n",
            "class T {\n",
            "  function m(array<string> $words): void {\n",
            "    foreach ($words as string $w) {\n",
            "      var $key = Core\\Str::lower($w);\n",
            "      echo $key;\n",
            "    }\n",
            "  }\n",
            "}\n",
        ));
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// The same obligation one level down: a local declared in one `if` branch
    /// is bound on that edge only, so the merge drops it — and the edge that
    /// bound it is the last place its reference is reachable. See
    /// [`Lowering::release_merged_away`].
    #[test]
    fn a_local_declared_in_one_if_branch_is_released_where_the_branches_merge() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\n",
            "class T {\n",
            "  function m(bool $c): void {\n",
            "    if ($c) {\n",
            "      var $s = Core\\Str::lower(\"AA\");\n",
            "      echo $s;\n",
            "    } else {\n",
            "      echo \"no\";\n",
            "    }\n",
            "  }\n",
            "}\n",
        ));
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// ADR 0063 R2's options bag, flattened: `Core\Arr::range` takes two
    /// positional arguments and one bag declaring one option, and both calls
    /// below emit a `core.call` with **three** arguments — the written
    /// `{step: 3}` in the first, the materialized default `1` in the second.
    /// The bag itself never appears in the IR at all, which is the property
    /// that keeps `mwl-codegen` and the ADR 0002 helper convention from
    /// learning that options exist.
    #[test]
    fn an_options_bag_flattens_into_one_argument_per_option() {
        let (f, map, file) = lower_first_method(concat!(
            "<?mwl\n",
            "class T {\n",
            "  function m(): void {\n",
            "    echo Core\\Arr::count(Core\\Arr::range(1, 10, {step: 3}));\n",
            "    echo Core\\Arr::count(Core\\Arr::range(1, 10));\n",
            "  }\n",
            "}\n",
        ));
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// `$x instanceof Name` — one `instanceof` naming the *resolved* class
    /// label the checker recorded, with no retain of the receiver.
    #[test]
    fn an_instanceof_names_the_class_the_checker_resolved() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class Animal {
}
class T {
  function m(Animal $a): bool {
    return $a instanceof Animal;
  }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// The dynamic `$x instanceof $name` form has no class to name, so the
    /// checker reports `E0496` and records nothing. This crate never sees such
    /// a program — the fixture reaches lowering only because these tests skip
    /// the diagnostics gate — so the miss is an internal-consistency panic
    /// rather than the hole its wording used to describe.
    #[test]
    #[should_panic(expected = "E0496")]
    fn a_dynamic_instanceof_records_nothing_to_lower() {
        lower_first_method(
            "<?mwl
class Animal {
}
class T {
  function m(Animal $a, string $n): bool {
    return $a instanceof $n;
  }
}
",
        );
    }

    /// A `mixed` subject keeps its [`crate::ir::Ty::Tagged`] representation all
    /// the way into the instruction: `mwl-codegen` calls
    /// `mwl_value_instanceof` for it, which reads the tag rather than
    /// dereferencing an unchecked payload. Every subject whose *declared* type
    /// can hold no object is `E0497` at the checker, so no third
    /// representation reaches here.
    #[test]
    fn an_instanceof_over_a_mixed_subject_keeps_its_tag() {
        let (f, _, _) = lower_first_method(
            "<?mwl
class Animal {
}
class T {
  function m(mixed $a): bool {
    return $a instanceof Animal;
  }
}
",
        );
        // Slot 0 is the receiver every lowered method carries; the declared
        // `mixed` parameter behind it is what the test walks, and lowering it
        // at all is the assertion — this fixture used to trip an assert
        // demanding a `Ty::Object` subject.
        assert_eq!(f.params.get(1), Some(&Ty::Tagged), "{:?}", f.params);
        assert!(
            f.blocks
                .iter()
                .flat_map(|b| &b.insts)
                .any(|i| matches!(i.kind, InstKind::InstanceOf { .. })),
            "the fixture lowers one `instanceof`"
        );
    }

    /// Integer `%` is the one operator carrying an
    /// [`Inst::on_error`](crate::ir::Inst::on_error) edge: ADR 0007 § 4 makes
    /// a zero divisor throw, and `mwl-codegen` raises that inline rather than
    /// through a helper, so the frame's cleanup path has to exist at the
    /// operator itself. `%` over floats gets none, which is the half of this
    /// worth holding — an error edge that appeared on every `BinOp` would be
    /// a landing block per arithmetic expression.
    #[test]
    fn an_integer_modulo_carries_an_error_edge_and_a_float_one_does_not() {
        let (f, _, _) = lower_script_src("<?mwl\nint $a = 7;\nint $b = 2;\nint $q = $a % $b;\n");
        let modulo = f
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .find(|i| matches!(i.kind, InstKind::BinOp { op: BinOp::Mod, .. }))
            .expect("the fixture lowers one `%`");
        assert!(modulo.on_error.is_some(), "{modulo:?}");

        let (f, _, _) =
            lower_script_src("<?mwl\nfloat $a = 7.0;\nfloat $b = 2.0;\nfloat $q = $a % $b;\n");
        let modulo = f
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .find(|i| matches!(i.kind, InstKind::BinOp { op: BinOp::Mod, .. }))
            .expect("the fixture lowers one `%`");
        assert!(modulo.on_error.is_none(), "{modulo:?}");
    }

    /// A file declaring no class still lowers, and still carries both rosters
    /// no source declares — `mwl_hir::errors`' exception tree, with its one
    /// synthesized constructor, and `mwl_hir::interfaces`' four global
    /// interfaces — the `hello.mwl` shape. Nothing in the file references any
    /// of them and they are emitted anyway: a descriptor has to exist before
    /// `$x instanceof Stringable` has anything to test against, and a class
    /// implementing one only keeps the edge if the label it names is in this
    /// list (`mwl_types::layout::build_class_layouts`).
    #[test]
    fn a_file_with_no_class_still_carries_every_compiler_declared_class() {
        let program = lower_whole_file("<?mwl\necho \"hi\";\n");
        let labels: Vec<&str> = program
            .classes
            .iter()
            .map(|class| class.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "ArithmeticError",
                "Comparable",
                "IOError",
                "Iterable",
                "Iterator",
                "LogicError",
                "ParseError",
                "RecursionError",
                "RuntimeError",
                "Stringable",
                "Throwable",
                "TimeoutError",
            ]
        );
        let functions: Vec<&str> = program.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            functions,
            [
                "<script>",
                "Throwable::constructor",
                "ParseError::constructor"
            ]
        );
    }

    /// The root's four slots are the ones `mwl_runtime::throwable` reaches by
    /// index, and every other exception class inherits them at the same
    /// indices — the property that lets the runtime append a backtrace frame
    /// to a value it knows nothing else about.
    #[test]
    fn every_exception_class_carries_the_root_s_four_slots_at_the_same_indices() {
        let program = lower_whole_file("<?mwl\nclass MyError extends IOError {}\n");
        for label in ["Throwable", "IOError", "MyError"] {
            let class = program
                .classes
                .iter()
                .find(|c| c.label == label)
                .unwrap_or_else(|| panic!("{label} should be in the class table"));
            assert_eq!(
                class.fields,
                ["message", "previous", "backtrace", "location"],
                "{label}"
            );
        }
    }

    /// ADR 0010 § 3: a case is an integer constant inlined at its use site —
    /// `Rank::Gold` is a `ConstInt 2` and nothing else, with no storage, no
    /// descriptor and no allocation. ADR 0010 § 5's first row then makes
    /// `as int` a free `Reinterpret`.
    #[test]
    fn an_enum_case_lowers_to_a_constant_and_as_int_is_free() {
        let (f, map, file) = lower_script_src(
            "<?mwl
enum Rank { Bronze, Silver, Gold }
int $g = Rank::Gold as int;
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// ADR 0010 § 2's `: uint` backing reaches the IR: the case constant is a
    /// `ConstUint` at `enum:uint`, and `as uint` is the free row again.
    #[test]
    fn a_uint_backed_enum_case_lowers_to_a_uint_constant() {
        let (f, map, file) = lower_script_src(
            "<?mwl
enum P: uint { Read = 0b001, Write = 0b010 }
uint $w = P::Write as uint;
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// An enum-typed *binding* is an integer binding, not an object one —
    /// ADR 0010 § 6. The parameter's representation is what proves it: a
    /// `Ty::Object` here would mean a refcounted receiver slot, a retain and a
    /// release, none of which an enum has.
    #[test]
    fn an_enum_typed_parameter_is_an_integer_parameter() {
        let (f, _map, _file) = lower_first_method(
            "<?mwl
enum Rank { Bronze, Gold }
class T { public function f(Rank $r): int { return $r as int; } }
",
        );
        assert_eq!(f.params, [Ty::Object, Ty::Enum(EnumRepr::Int)]);
        assert_eq!(f.ret, Ty::Int);
    }

    /// ADR 0035 § 4: an enum case is *always* truthy, never judged by its
    /// backing value. `Rank::Bronze` is backed by `0`, so a representation
    /// that erased it to `Ty::Int` would emit `Helper::IntTruthy` here and
    /// come back `false`.
    #[test]
    fn an_enum_condition_folds_to_true_rather_than_testing_its_backing_value() {
        let (f, map, file) = lower_script_src(
            "<?mwl
enum Rank { Bronze, Gold }
if (Rank::Bronze) { echo \"y\"; }
",
        );
        let text = print_function(&f, map.file(file));
        assert!(text.contains("const.bool true"), "{text}");
        assert!(!text.contains("helper.int_truthy"), "{text}");
    }

    /// ADR 0007 § 2's total rows, reached through `as` rather than through
    /// `.`: a scalar to `string` reuses the same `Helper` conversion, and a
    /// value to `bool` reuses ADR 0035's truthy table.
    #[test]
    fn as_string_and_as_bool_reuse_the_conversions_that_already_exist() {
        let (f, map, file) = lower_script_src(
            "<?mwl
string $s = 7 as string;
bool $b = 0 as bool;
",
        );
        let text = print_function(&f, map.file(file));
        assert!(text.contains("helper.int_to_string"), "{text}");
        assert!(text.contains("helper.int_truthy"), "{text}");
    }

    /// ADR 0007 § 2's checked rows go through a fallible helper — the same
    /// call shape a method call has, error edge included, because either one
    /// can throw. The error edge is what this asserts: a checked conversion
    /// that skipped it would drop the throw on the floor.
    #[test]
    fn a_checked_conversion_row_carries_adr_0002_s_error_edge() {
        let (f, map, file) = lower_script_src(
            "<?mwl
uint $u = 1;
int $n = $u as int;
",
        );
        let text = print_function(&f, map.file(file));
        // `! bbN` is how `crate::print` renders `Inst::on_error`.
        assert!(
            text.lines()
                .any(|l| l.contains("helper.uint_to_int") && l.contains(" ! bb")),
            "{text}"
        );
    }

    /// ADR 0013 § 2: ordering two objects *is* a `Comparable::compareTo` call
    /// followed by a comparison of its `int` against zero — never a comparison
    /// of the two values, which for objects would be two heap pointers.
    #[test]
    fn ordering_two_objects_calls_compare_to_and_tests_its_result_against_zero() {
        let (f, map, file) = lower_script_src(
            "<?mwl
class P implements Comparable {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function compareTo(self $other): int { return $this->n - $other->n; }
}
var $a = new P(1);
var $b = new P(2);
if ($a < $b) { echo \"lt\"; }
",
        );
        let text = print_function(&f, map.file(file));
        assert!(text.contains("::compareTo"), "{text}");
        assert!(text.contains("const.int 0"), "{text}");
        assert!(
            text.lines()
                .any(|l| l.contains("lt v") && l.contains("bool")),
            "{text}"
        );
    }

    /// `<=>` is the call's own result: `compareTo` already returns exactly
    /// what the spaceship operator means, so there is no second comparison.
    #[test]
    fn the_spaceship_operator_is_the_compare_to_result_itself() {
        let (f, map, file) = lower_script_src(
            "<?mwl
class P implements Comparable {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function compareTo(self $other): int { return $this->n - $other->n; }
}
var $a = new P(1);
int $c = $a <=> $a;
",
        );
        let text = print_function(&f, map.file(file));
        assert!(text.contains("::compareTo"), "{text}");
        assert!(!text.contains("const.int 0"), "{text}");
    }

    /// ADR 0023 § 1: one instruction, a fresh object with one owner, and no
    /// hook — `__clone` is one of the magic methods MWL does not have.
    #[test]
    fn clone_lowers_to_one_instruction_with_no_hook_call() {
        let (f, map, file) = lower_script_src(
            "<?mwl
class P { public int $n; public function constructor(int $n) { $this->n = $n; } }
var $a = new P(1);
var $b = clone $a;
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// ADR 0010 § 5's integer *into* an enum: the conversion is free, and
    /// what it costs is the check in front of it — one comparison per case of
    /// the declaration, throwing with every case named. The accepted set is
    /// built from the declaration rather than from the site, which is the
    /// whole difference between this and ADR 0047 § 3's named subset.
    #[test]
    fn converting_into_an_enum_tests_every_case_of_the_declaration() {
        let (f, map, file) = lower_script_src(
            "<?mwl
enum Rank { Bronze, Gold }
int $n = 1;
Rank $r = $n as Rank;
",
        );
        let text = print_function(&f, map.file(file));
        assert!(text.contains("reinterpret"), "{text}");
        assert!(
            text.contains("`Rank::Bronze`, `Rank::Gold`"),
            "the throw names every case, in the declaration's own order: {text}"
        );
    }

    // ------------------------------------------------------------------
    // By-reference parameters -- `Ty::Ref` owns the representation these
    // pin down, and its refcounting section owns the retain/release pairing
    // the third one exists to make visible.
    // ------------------------------------------------------------------

    /// The caller's half: the holder is read, staged into a one-cell slot
    /// whose address is the argument, and copied back out of that slot once
    /// the call returns -- rebinding the local, so everything after the call
    /// reads the written-back value. `int` is not refcounted, so the whole
    /// thing costs a store, a load and no refcount traffic at all.
    #[test]
    fn a_by_reference_argument_is_staged_and_copied_back() {
        let (f, map, file) = lower_script_src(
            "<?mwl
class Adder {
               public static function bump(int &$slot): void { $slot = $slot + 5; }
}
             int $n = 1;
Adder::bump($n);
echo $n;
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// The callee's half: the parameter is a `ref`, every read of it is a
    /// `ref.load` and every write a `ref.store`. Nothing is released at the
    /// frame's exit -- a `Ty::Ref` is not `Ty::is_refcounted`, so
    /// `release_all_locals` skips it, which is what keeps the caller's
    /// staged reference the caller's.
    #[test]
    fn a_by_reference_parameter_reads_and_writes_through_its_slot() {
        let (f, map, file) = lower_first_method(
            "<?mwl
class T {
               public static function bump(int &$slot): void { $slot = $slot + 5; }
}
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// A refcounted pointee, where the policy is actually visible: one
    /// staging `retain` before `ref.slot`, and one `release` of the holder's
    /// previous value at the copy-back. The pair balances, which is what
    /// makes the slot's own reference transfer into the holder rather than
    /// leak or double-free.
    #[test]
    fn a_refcounted_by_reference_argument_balances_its_staging_retain() {
        let (f, map, file) = lower_script_src(
            "<?mwl
class Shout {
               public static function upper(string &$s): void { $s = $s . \"!\"; }
}
             string $msg = \"hi\";
Shout::upper($msg);
echo $msg;
",
        );
        assert_snapshot!(print_function(&f, map.file(file)));
    }

    /// ADR 0033 § 5: `==` over two `secret` operands is the constant-time
    /// helper, and an unqualified pair of the same representation is still
    /// the ordinary `BinOp::Eq` that `mwl-codegen` turns into `mwl_str_eq`.
    ///
    /// Both halves are asserted in one fixture on purpose. The qualifier
    /// spends no representation (`erase_checked_ty`), so the *only* thing
    /// separating these two comparisons in the IR is which helper the arm
    /// picked — a version of this test that pinned the `secret` pair alone
    /// would still pass if the lowering had started sending every `string`
    /// comparison through the constant-time row, which is a real regression:
    /// it would spend § 5's ≈+8 ns on every string `==` in the language.
    #[test]
    fn a_secret_equality_lowers_to_the_constant_time_helper() {
        let (f, map, file) = lower_script_src(
            "<?mwl
secret string $token = \"a\";
secret string $given = \"b\";
bool $secretly = $token == $given;
string $plain = \"a\";
string $other = \"b\";
bool $openly = $plain == $other;
",
        );
        let text = print_function(&f, map.file(file));
        assert_eq!(text.matches("helper.secret_eq").count(), 1, "{text}");
        // `= eq `, not `eq `: `helper.secret_eq v0, v1` ends in the shorter
        // one, so the loose spelling counts the constant-time call twice and
        // the assertion below can never fail.
        assert_eq!(text.matches("= eq ").count(), 1, "{text}");
    }

    /// The `bytes` base of the same rule, and `!=` — ADR 0033 § 1 puts the
    /// qualifier on both bases, and `Helper::SecretEq` answers `!=` under a
    /// `UnOp::Not` rather than through a second helper, the arrangement
    /// `Helper::NumericEq` already uses. A `!=` that had grown its own
    /// short-circuiting row would be the same timing oracle § 5 closes.
    #[test]
    fn a_secret_bytes_inequality_is_the_same_helper_under_a_not() {
        let (f, map, file) = lower_script_src(
            "<?mwl
secret bytes $mac = \"a\" as bytes;
secret bytes $sent = \"b\" as bytes;
bool $differ = $mac != $sent;
",
        );
        let text = print_function(&f, map.file(file));
        assert!(text.contains("helper.secret_eq"), "{text}");
        assert!(text.contains("not "), "{text}");
    }
}
