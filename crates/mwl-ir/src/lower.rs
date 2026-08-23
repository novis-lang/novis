//! Lowers one already-checked method body to a [`crate::ir::Function`] — see
//! the crate's own module docs for exactly which statement/expression shapes
//! this slice covers, and why it trusts its input rather than re-checking it.
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
//! already have been rejected there.
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
    AssignOp, BinaryOp, Block, CallArgs, CatchClause, ClassMemberKind, Expr, ExprKind,
    ForeachBinding, MethodMember, Modifier, NamespaceDecl, Stmt, StmtKind, StringPart, Type,
    TypeAtom, TypeKind, UnaryOp as AstUnaryOp,
};
use mwl_types::expr_table::{ExprInfo, ExprTypeTable};
use mwl_types::layout::ClassLayoutTable;
use mwl_types::ty::{Ty as CheckedTy, TypeId, TypeInterner};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{BlockId, IdGen, ValueId};
use crate::ir::{BasicBlock, BinOp, Function, Helper, Inst, InstKind, Terminator, UnOp};
use crate::ty::Ty;
use crate::{span_text, strip_sigil};

/// A local's current SSA binding: which value it holds, and at what
/// representation type.
type Env = FxHashMap<String, (ValueId, Ty)>;

/// One loop's exit points, gathered while its body is lowered.
/// [`Lowering::lower_while`]/[`Lowering::lower_foreach`] push one of these
/// onto [`Lowering::loop_stack`] before lowering the body and pop it back off
/// once lowering returns; [`Lowering::lower_break`]/[`Lowering::lower_continue`]
/// read the top frame's `after_block`/`header_block` and record their own
/// `(block, env)` pair into it. The loop then folds `continue_edges` in
/// alongside the body's own fall-through exit when patching the header's
/// phis, and `break_edges` in alongside the condition's false edge when
/// building the loop's own after-block environment — see
/// [`Lowering::lower_while`]'s own doc comment for exactly how both are
/// combined.
struct LoopFrame {
    /// Where a `continue` jumps — the loop header, re-running the condition.
    header_block: BlockId,
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
    /// The reserved `Env` names a `foreach` keeps its own bookkeeping under —
    /// its retained reference to the array being walked, and its cursor. They
    /// are dropped from a `break` edge's recorded environment so nothing after
    /// the loop can see them; see [`Lowering::lower_foreach`] for why they
    /// live in the `Env` at all.
    loop_private: Vec<String>,
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
    handler: BlockId,
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

/// Lowers a whole checked file: every class method that has a body, plus the
/// file's own top-level statements as one script frame named `script`.
///
/// This is what a caller with a file in hand wants — [`lower_method`] and
/// [`lower_script`] stay public for the narrower "lower exactly this one
/// thing" cases the tests use.
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
/// The same way [`lower_method`]/[`lower_script`] do — naming the shape this
/// slice does not lower. See [`lower_method`]'s own note.
#[must_use]
pub fn lower_file(
    script: &str,
    stmts: &[Stmt],
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    layouts: &ClassLayoutTable,
) -> crate::ir::Program {
    fn walk(
        stmts: &[Stmt],
        src: &SourceFile,
        exprs: &ExprTypeTable,
        checked_types: &TypeInterner,
        out: &mut Vec<Function>,
    ) {
        for stmt in stmts {
            match &stmt.kind {
                // A namespace scopes names, not storage — the label each
                // method is lowered under already carries the resolved
                // namespace, so this walk only has to reach the declarations
                // inside the block.
                StmtKind::NamespaceDecl(NamespaceDecl {
                    body: Some(block), ..
                }) => walk(&block.stmts, src, exprs, checked_types, out),
                StmtKind::ClassDecl(decl) => {
                    for member in &decl.members {
                        let ClassMemberKind::Method(m) = &member.kind else {
                            continue;
                        };
                        if m.body.is_none() {
                            continue; // `abstract` — nothing to lower
                        }
                        let Some(label) = exprs.method_label(m.name) else {
                            continue;
                        };
                        out.push(lower_method(label, m, src, exprs, checked_types));
                    }
                }
                _ => {}
            }
        }
    }

    let mut functions = Vec::new();
    walk(stmts, src, exprs, checked_types, &mut functions);
    functions.push(lower_script(script, stmts, src, exprs, checked_types));
    // The one function with no source text — see
    // `synthesized_throwable_constructor`. Emitted unconditionally: the
    // exception tree is in every program's class table, so a unit that omitted
    // this would be one where `new LogicError(…)` names a missing target.
    functions.push(synthesized_throwable_constructor());

    // Copied straight across rather than recomputed: `mwl-types` already
    // resolved the slot order and the supertype set against the class graph,
    // which this crate cannot see — see `crate::ir::Class`.
    let mut classes: Vec<crate::ir::Class> = layouts
        .iter()
        .map(|(label, layout)| crate::ir::Class {
            label: label.to_owned(),
            fields: layout.fields.clone(),
            conforms: layout.conforms.clone(),
        })
        .collect();
    // The table behind `iter()` is a hash map, so its order varies run to run.
    // Sorting here is what makes a lowered `Program` — and therefore the
    // `--dump-ir` listing and every snapshot taken of it — reproducible for an
    // unchanged file, the same stability `crate::ids` already guarantees for a
    // statement id.
    classes.sort_by(|a, b| a.label.cmp(&b.label));

    crate::ir::Program { functions, classes }
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
) -> Function {
    let ret_ty = m.return_type.as_ref().map_or(Ty::Void, lower_decl_type);
    let mut low = Lowering::new(name, src, ret_ty, exprs, checked_types);
    let entry = low.new_block();
    let mut cur = entry;
    let mut env = Env::default();
    let mut param_tys = Vec::new();

    // Reserved safepoint poll site (recursion) — see `InstKind::Safepoint`'s
    // own doc comment for why function entry is one of the two fixed sites
    // and why this slice reserves only the shape, not a functional check.
    low.emit_safepoint(entry);

    // The implicit receiver (`$this`), always parameter index 0 — seeded
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
    let (this_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    env.insert("this".to_owned(), (this_v, Ty::Object));
    param_tys.push(Ty::Object);
    // ...but a `static` method's caller fills that slot with `null`, so this
    // frame holds no reference to release at scope exit. See
    // `Lowering::borrowed`, which is what that costs: one skipped release, no
    // second parameter shape and no second call convention.
    if m.modifiers.contains(&Modifier::Static) {
        low.borrowed = Some("this".to_owned());
    }

    for (i, p) in m.params.iter().enumerate() {
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
        let ty = lower_decl_type(decl_ty);
        // +1: index 0 is always the implicit receiver seeded above.
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let (v, _) = low.emit(entry, ty, InstKind::Param(index));
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        env.insert(pname, (v, ty));
        param_tys.push(ty);
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

    let (blocks, stmt_spans, edge_spans) = low.finish();
    Function {
        name: name.to_owned(),
        params: param_tys,
        ret: ret_ty,
        blocks,
        entry,
        stmt_spans,
        edge_spans,
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
/// - **The return representation is [`Ty::Mixed`].** A top-level `return`
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
) -> Function {
    let ret_ty = Ty::Mixed;
    let mut low = Lowering::new(name, src, ret_ty, exprs, checked_types);
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

    let (blocks, stmt_spans, edge_spans) = low.finish();
    Function {
        name: name.to_owned(),
        params: Vec::new(),
        ret: ret_ty,
        blocks,
        entry,
        stmt_spans,
        edge_spans,
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
    /// The one local name this frame holds *without* owning a reference to,
    /// or `None`.
    ///
    /// Exactly one thing is ever in it: a `static` method's `$this`. Every
    /// lowered method carries a receiver at parameter index 0 whether or not
    /// it has one (see [`lower_method`]), and a static call fills that slot
    /// with `null` — so the frame was handed no reference there and must not
    /// release one at scope exit. Every other refcounted parameter *was*
    /// retained by its caller, which is what makes the release correct for all
    /// of them.
    borrowed: Option<String>,
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
}

impl<'a> Lowering<'a> {
    fn new(
        name: &str,
        src: &'a SourceFile,
        ret_ty: Ty,
        exprs: &'a ExprTypeTable,
        checked_types: &'a TypeInterner,
    ) -> Self {
        Self {
            ids: IdGen::new(),
            exprs,
            checked_types,
            src,
            ret_ty,
            block_ids: Vec::new(),
            block_insts: Vec::new(),
            block_terms: Vec::new(),
            loop_stack: Vec::new(),
            try_stack: Vec::new(),
            borrowed: None,
            fn_label: name.to_owned(),
            cur_stmt_span: Span::at(src.id(), 0),
            foreach_seq: 0,
        }
    }

    fn new_block(&mut self) -> BlockId {
        let id = self.ids.next_block();
        self.block_ids.push(id);
        self.block_insts.push(Vec::new());
        self.block_terms.push(None);
        id
    }

    fn is_terminated(&self, b: BlockId) -> bool {
        self.block_terms[b.index() as usize].is_some()
    }

    /// Gives `b` its terminator, exactly once.
    ///
    /// # Panics
    ///
    /// Panics if `b` already has one — every call site only ever seals a
    /// block it just confirmed (via [`Self::is_terminated`]) is still open.
    fn seal(&mut self, b: BlockId, term: Terminator) {
        let slot = &mut self.block_terms[b.index() as usize];
        assert!(
            slot.is_none(),
            "mwl-ir: bb{} sealed twice — bug in control-flow lowering",
            b.index()
        );
        *slot = Some(term);
    }

    fn emit(&mut self, b: BlockId, ty: Ty, kind: InstKind) -> (ValueId, Ty) {
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
    fn emit_fallible(&mut self, b: BlockId, ty: Ty, kind: InstKind, env: &Env) -> (ValueId, Ty) {
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
    /// # Known gap
    ///
    /// The sweep covers the frame's **locals**, not a temporary still in
    /// flight inside the expression being evaluated — the fresh string a
    /// half-built `.` concatenation is holding when its next operand's call
    /// throws has no `Env` entry to be found through, and leaks. Closing it
    /// needs an owned-temporaries stack threaded through
    /// [`Self::lower_expr`], which is a widening of this policy rather than a
    /// different one.
    fn landing_block(&mut self, env: &Env) -> BlockId {
        let b = self.new_block();
        match self.try_stack.last().map(|frame| frame.handler) {
            Some(handler) => {
                self.try_stack
                    .last_mut()
                    .expect("just read the top frame above")
                    .edges
                    .push((b, env.clone()));
                self.seal(b, Terminator::Catch { handler });
            }
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
    fn frame_label(&self) -> String {
        let (line, _) = self.src.line_col(self.cur_stmt_span.start);
        format!("{}() at {}:{}", self.fn_label, self.src.name(), line + 1)
    }

    /// Appends a reserved [`InstKind::Safepoint`] marker to `b` — see that
    /// variant's own doc comment for the two call sites this has today
    /// (function entry, a loop's back edge) and why it defines no value.
    fn emit_safepoint(&mut self, b: BlockId) {
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
    fn emit_retain(&mut self, b: BlockId, v: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Retain { operand: v },
            on_error: None,
        });
    }

    /// Appends an [`InstKind::Release`] on `v` to `b` — see [`Self::emit_retain`].
    fn emit_release(&mut self, b: BlockId, v: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Release { operand: v },
            on_error: None,
        });
    }

    /// Appends an [`InstKind::FieldSet`] to `b` — see
    /// [`Self::lower_reassignment`]'s property-target arm for the retain/
    /// release policy wrapped around this.
    fn emit_field_set(
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

    /// Appends an [`InstKind::ArraySet`] to `b`, yielding the array that now
    /// holds the entry — see that variant's own doc comment for the
    /// consume-one-reference-yield-one protocol, and
    /// [`Self::lower_reassignment`]'s `Index`-target arm for the retain policy
    /// wrapped around this and for where the result is written back to.
    fn emit_array_set(
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
    fn emit_array_append(&mut self, b: BlockId, array: ValueId, value: ValueId) -> ValueId {
        self.emit(b, Ty::Array, InstKind::ArrayAppend { array, value })
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
    fn bind_local(
        &mut self,
        cur: BlockId,
        env: &mut Env,
        name: String,
        v: ValueId,
        ty: Ty,
        source: &Expr,
    ) {
        if ty.is_refcounted() && is_aliasing_read(&source.kind) {
            self.emit_retain(cur, v);
        }
        if let Some(&(old_v, old_ty)) = env.get(&name)
            && old_ty.is_refcounted()
        {
            self.emit_release(cur, old_v);
        }
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
    /// Exactly two holders can be written back to today, which are the two
    /// [`is_aliasing_read`] already recognises as durable storage: a bare
    /// local, and a compile-time-known property. A nested subscript
    /// (`$grid[0][1] = 5`) would have to separate every level of the chain and
    /// write each back in turn, so it panics naming itself rather than
    /// silently dropping the outer levels' separation; so does any other base,
    /// which is a write into a temporary and has no holder to speak of.
    fn write_back_array(&mut self, base: &Expr, written: ValueId, env: &mut Env, cur: BlockId) {
        match &base.kind {
            ExprKind::Variable(name_span) => {
                let name = strip_sigil(span_text(self.src, *name_span)).to_owned();
                env.insert(name, (written, Ty::Array));
            }
            ExprKind::PropertyAccess { object, .. } => {
                let Some(ExprInfo::Property { class, name, .. }) = self.exprs.lookup(base.span)
                else {
                    panic!(
                        "mwl-ir: an array-index assignment whose base is the property at {:?} \
                         has no resolved declaring class recorded in the typed-expression table \
                         — either it wasn't checked with the same table, or its receiver erased \
                         to a shape/plain `object` (ADR 0036 § 4), which this crate does not yet \
                         lower (see the crate docs' known gaps)",
                        base.span
                    );
                };
                let class_label = class.to_string();
                let field_name = name.clone();
                let (object_v, _) = self.lower_expr(object, None, env, cur);
                self.emit_field_set(cur, object_v, class_label, field_name, written);
            }
            other => panic!(
                "mwl-ir lowers an array-element write only through a bare local or a \
                 compile-time-known property, because ADR 0007 § 5's copy-on-write separation \
                 has to be written back to whatever holds the array — not through {other:?}; \
                 see the crate docs' known gaps"
            ),
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
    /// [`Self::borrowed`] is skipped for a second, unrelated reason: that
    /// slot is one the frame never owned a reference to at all.
    fn release_all_locals(&mut self, cur: BlockId, env: &Env, except: Option<&str>) {
        let mut names: Vec<&String> = env.keys().collect();
        names.sort();
        for name in names {
            if Some(name.as_str()) == except || self.borrowed.as_deref() == Some(name.as_str()) {
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
    fn finish(self) -> (Vec<BasicBlock>, Vec<Span>, Vec<Span>) {
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

    /// [`lower_script`]'s own walk: like [`Self::lower_stmts`], but over a
    /// file's top level, where a *declaration* is not part of the frame at
    /// all (a class's methods are lowered separately, one [`lower_method`]
    /// call each) and a `namespace X { ... }` block's body is — a namespace
    /// scopes names, not storage, so its statements share this one frame.
    /// The unbraced `namespace X;` form declares no statements of its own,
    /// so it is skipped like any other declaration; the statements that
    /// follow it are siblings and are reached by the ordinary loop.
    fn lower_script_stmts(&mut self, stmts: &'a [Stmt], cur: &mut BlockId, env: &mut Env) {
        for stmt in stmts {
            if self.is_terminated(*cur) {
                break;
            }
            match &stmt.kind {
                StmtKind::NamespaceDecl(NamespaceDecl {
                    body: Some(block), ..
                }) => self.lower_script_stmts(&block.stmts, cur, env),
                StmtKind::NamespaceDecl(_)
                | StmtKind::UseDecl(_)
                | StmtKind::ClassDecl(_)
                | StmtKind::InterfaceDecl(_)
                | StmtKind::EnumDecl(_)
                | StmtKind::TypeAliasDecl(_)
                | StmtKind::TopLevelFunction(_)
                | StmtKind::TopLevelConst(_) => {}
                _ => self.lower_stmt(stmt, cur, env),
            }
        }
    }

    /// Lowers a statement list into `cur`, stopping early once `cur` is
    /// sealed (dead code after a `return` inside the list is simply never
    /// lowered — nothing downstream needs it modeled).
    fn lower_stmts(&mut self, stmts: &'a [Stmt], cur: &mut BlockId, env: &mut Env) {
        for stmt in stmts {
            if self.is_terminated(*cur) {
                break;
            }
            self.lower_stmt(stmt, cur, env);
        }
    }

    fn lower_stmt(&mut self, stmt: &'a Stmt, cur: &mut BlockId, env: &mut Env) {
        let stmt_id = self.ids.next_stmt(stmt.span);
        self.cur_stmt_span = stmt.span;
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::StmtMarker(stmt_id),
            on_error: None,
        });
        match &stmt.kind {
            StmtKind::LocalDecl {
                ty: Some(decl_ty),
                name: local_name,
                value: Some(value),
            } => {
                let expected = lower_decl_type(decl_ty);
                let (v, _) = self.lower_expr_top(value, Some(expected), env, cur);
                let lname = strip_sigil(span_text(self.src, *local_name)).to_owned();
                self.bind_local(*cur, env, lname, v, expected, value);
            }
            // ADR 0037: `var $x = expr;` — no declared type at all, so there
            // is no `expected` to check `value` against. `lower_expr` already
            // synthesizes a type from the expression alone whenever `expected`
            // is `None` (a bare integer literal defaults to `Ty::Int`, etc.) —
            // exactly the same rule `mwl_types::locals::check_stmt`'s own
            // `None` arm applies (`check_expr(value, None, ...)` before fixing
            // the binding to whatever came back), so this just reuses that
            // path and binds the local to the type `lower_expr` returns
            // instead of a type read off the AST. The parser/checker both
            // guarantee `value` is present whenever `ty` is `None` — see
            // `StmtKind::LocalDecl`'s own doc comment — and a bare
            // array-literal initializer is already a checker error (ADR
            // 0037's own refused shape), so it never reaches this arm.
            StmtKind::LocalDecl {
                ty: None,
                name: local_name,
                value: Some(value),
            } => {
                let (v, ty) = self.lower_expr_top(value, None, env, cur);
                let lname = strip_sigil(span_text(self.src, *local_name)).to_owned();
                self.bind_local(*cur, env, lname, v, ty, value);
            }
            StmtKind::Expr(e) => self.lower_expr_stmt(e, env, cur),
            // See `Self::release_all_locals`'s own doc comment for why a
            // bare `$name` return expression is excluded from the exit
            // sweep rather than retained: its value transfers out instead of
            // being copied. A property read has no local slot to exclude
            // from that sweep at all — the field's own storage keeps its
            // reference regardless of what this function does — so it needs
            // an explicit retain here instead, the same `is_aliasing_read`
            // judgment `Self::bind_local`/`Self::lower_call_args` already
            // apply at their own boundary.
            StmtKind::Return(value) => {
                let except = value.as_ref().and_then(|v| {
                    if let ExprKind::Variable(span) = &v.kind {
                        Some(strip_sigil(span_text(self.src, *span)).to_owned())
                    } else {
                        None
                    }
                });
                let ret_ty = self.ret_ty;
                let v = if let Some(value_expr) = value.as_ref() {
                    let (rv, rty) = self.lower_expr_top(value_expr, Some(ret_ty), env, cur);
                    if except.is_none() && rty.is_refcounted() && is_aliasing_read(&value_expr.kind)
                    {
                        self.emit_retain(*cur, rv);
                    }
                    Some(rv)
                } else {
                    None
                };
                // Every enclosing `finally` runs before the frame is left —
                // see `run_pending_finallys` for why each exit lowers its own
                // copy of the body.
                self.run_pending_finallys(cur, env);
                if !self.is_terminated(*cur) {
                    self.release_all_locals(*cur, env, except.as_deref());
                    self.seal(*cur, Terminator::Return(v));
                }
            }
            StmtKind::Block(b) => self.lower_stmts(&b.stmts, cur, env),
            StmtKind::If { cond, then, else_ } => {
                self.lower_if(cond, then, else_.as_deref(), cur, env);
            }
            StmtKind::While { cond, body } => self.lower_while(cond, body, cur, env),
            StmtKind::Foreach {
                subject,
                key,
                value,
                value_by_ref,
                body,
            } => self.lower_foreach(subject, key.as_ref(), value, *value_by_ref, body, cur, env),
            // ADR 0028 § 3 leaves exactly one `unset` target standing — an
            // array element — and `mwl_types::expr::check_unset_target`
            // already rejected a declared property, so anything else reaching
            // here is a shape this slice does not lower.
            StmtKind::Unset(targets) => {
                for target in targets {
                    self.lower_unset(target, env, *cur);
                }
            }
            StmtKind::Break(level) => self.lower_break(level, cur, env),
            StmtKind::Continue(level) => self.lower_continue(level, cur, env),
            StmtKind::Echo(operands) => self.lower_echo(operands, *cur, env),
            StmtKind::Try {
                body,
                catches,
                finally,
            } => self.lower_try(body, catches, finally.as_ref(), cur, env),
            other => panic!(
                "mwl-ir's control-flow slice only lowers a typed local declaration, a plain \
                 reassignment, `echo`, `unset`, `return`, a nested block, `if`, `while`, \
                 `foreach`, `try`/`catch`, `throw` and a loop-scoped `break`/`continue` — got \
                 {other:?}; see the crate docs' known gaps"
            ),
        }
    }

    /// An expression used as its own statement — either `$x = expr;` (routed
    /// to [`Self::lower_reassignment`]) or a call/`new` invoked purely for
    /// its side effect with no assignment at all, e.g. `doSomething();`, the
    /// ordinary way to invoke a `void`-returning method. The latter shares no
    /// machinery with [`Self::bind_local`]'s declare/reassign policy — there
    /// is no local slot for the produced value to occupy, and nothing else in
    /// the function will ever bind or return it — so it is lowered for
    /// whatever it does and its result, if any, is released immediately when
    /// [`Ty::is_refcounted`] (a `void`-returning call has nothing to
    /// release). The receiver of a discarded instance call is not released
    /// here: [`Ty::Object`] isn't refcounted yet at all (see the crate docs'
    /// known gaps), independent of whether the call itself is a statement or
    /// bound to something.
    ///
    /// # Panics
    ///
    /// Panics naming the unsupported shape for anything outside this slice's
    /// scope: an expression statement that is neither a plain reassignment
    /// nor a bare call/`new`.
    fn lower_expr_stmt(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        match &e.kind {
            ExprKind::Assign {
                op: AssignOp::Assign,
                by_ref: false,
                ..
            } => self.lower_reassignment(e, env, cur),
            ExprKind::MethodCall { .. } | ExprKind::StaticCall { .. } | ExprKind::New { .. } => {
                let (v, ty) = self.lower_expr(e, None, env, *cur);
                if ty.is_refcounted() {
                    self.emit_release(*cur, v);
                }
            }
            // PHP 8 makes `throw` an expression; MWL keeps that grammar and
            // lowers only the statement position, which is the one place it
            // has a block to seal. `$x = throw …;` falls through to
            // `lower_expr`'s own unsupported-shape panic.
            ExprKind::Throw(inner) => self.lower_throw(inner, env, cur),
            other => panic!(
                "mwl-ir's control-flow slice only lowers a plain `$x = expr;` reassignment or a \
                 bare call/`new` as an expression statement — got {other:?}; see the crate \
                 docs' known gaps"
            ),
        }
    }

    /// `throw expr;` — evaluate the exception, hand its reference to the
    /// request context, and enter this point's landing block.
    ///
    /// The retain mirrors every other "copy into a second durable slot" site
    /// ([`Self::bind_local`], [`Self::lower_call_args`]): the context durably
    /// owns the exception once [`Terminator::Throw`] runs, so an aliasing
    /// operand (`throw $e;`) needs one and a fresh `new Exception(…)` — which
    /// already has exactly one owner — does not. The frame's own release of
    /// `$e` then happens in the landing block, which is why the two do not
    /// cancel out.
    ///
    /// # Panics
    ///
    /// Panics naming the operand's representation if it is not a
    /// [`Ty::Object`] — the checker has already refused throwing anything but
    /// a `Throwable` subclass, so anything else here is a lowering bug.
    fn lower_throw(&mut self, inner: &Expr, env: &mut Env, cur: &mut BlockId) {
        let (v, ty) = self.lower_expr_top(inner, None, env, cur);
        assert!(
            matches!(ty, Ty::Object),
            "mwl-ir lowers `throw` only for an exception object — got representation {ty:?}; \
             see the crate docs' known gaps"
        );
        if is_aliasing_read(&inner.kind) {
            self.emit_retain(*cur, v);
        }
        self.write_throw_location(*cur, v);
        let landing = self.landing_block(env);
        self.seal(*cur, Terminator::Throw { value: v, landing });
    }

    /// Fills `$e->location` with the site of the `throw` that is about to
    /// raise it.
    ///
    /// **The throw site, not the construction site**, and deliberately so: the
    /// backtrace beside it holds the frames the exception *unwound out of*
    /// rather than a snapshot taken at `new` (see `mwl_runtime::throwable`'s
    /// own docs for why ADR 0002's checked-return convention makes that the
    /// cheap shape), so a `location` naming the construction site would be the
    /// one field disagreeing with everything around it.
    ///
    /// The write goes through the *root* class label. `mwl_runtime::object`
    /// lays a subclass's slots after its parent's, so a slot resolved against
    /// `Throwable` is valid for every exception class there can be — which is
    /// the same property that lets the runtime reach `backtrace` at all.
    fn write_throw_location(&mut self, cur: BlockId, thrown: ValueId) {
        let (line, _) = self.src.line_col(self.cur_stmt_span.start);
        let rendered = format!("{}:{}", self.src.name(), line + 1);
        let (previous, _) = self.emit(
            cur,
            Ty::Str,
            InstKind::FieldGet {
                object: thrown,
                class: THROWABLE_ROOT.to_owned(),
                field: LOCATION_FIELD.to_owned(),
            },
        );
        self.emit_release(cur, previous);
        let (location, _) = self.emit(cur, Ty::Str, InstKind::ConstStr(rendered));
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::FieldSet {
                object: thrown,
                class: THROWABLE_ROOT.to_owned(),
                field: LOCATION_FIELD.to_owned(),
                value: location,
            },
            on_error: None,
        });
    }

    /// `try { … } catch (T $e) { … } finally { … }` — the protected region,
    /// its clause dispatch, its `finally`, and the join point after all of
    /// them.
    ///
    /// Structurally a [`Self::lower_while`] without a back edge: a
    /// [`TryFrame`] brackets the body so every failing call inside it records
    /// its own landing block as one incoming edge, and those edges are folded
    /// into the dispatch block's phis by the same [`Self::merge_envs`] the
    /// loop's `break` edges already go through.
    ///
    /// # How a clause is selected
    ///
    /// [`InstKind::TakeThrown`] takes the pending exception once, at the top
    /// of the dispatch block, and each clause is then one
    /// [`InstKind::InstanceOf`] against its declared class plus a branch —
    /// the same test `$e instanceof T` compiles to, which is why nothing here
    /// needs a second mechanism. `catch (Throwable $e)` is not special-cased:
    /// every exception class descends from `Throwable`, so its `instanceof`
    /// simply always answers true. If no clause matches, the taken reference
    /// is handed straight back to a [`Terminator::Throw`], so an unmatched
    /// exception leaves the frame carrying the same object it arrived with.
    ///
    /// A clause's binding is released at the end of its own body rather than
    /// living on past the `try`. That is narrower than PHP, where `$e` stays
    /// visible after it, and it follows from this crate's flat, unscoped
    /// [`Env`]: a name bound on only one incoming edge is dropped by
    /// `merge_envs`, so anything left holding a reference at that join would
    /// never be released at all.
    ///
    /// # Known gaps
    ///
    /// * A `finally` does **not** run when the exception path enters a
    ///   `catch` clause whose *own body* then throws: the frame for this
    ///   region is popped before a handler is lowered, so that throw reaches
    ///   the enclosing region directly. PHP runs the `finally` first.
    /// * A `break`/`continue` out of a protected region does not run a
    ///   pending `finally` either — [`Self::lower_break`] refuses that shape
    ///   outright rather than lowering it wrong.
    fn lower_try(
        &mut self,
        body: &'a Block,
        catches: &'a [CatchClause],
        finally: Option<&'a Block>,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let handler_block = self.new_block();
        let after_block = self.new_block();

        self.try_stack.push(TryFrame {
            handler: handler_block,
            edges: Vec::new(),
            finally,
        });
        let mut body_env = env.clone();
        let mut body_cur = *cur;
        self.lower_stmts(&body.stmts, &mut body_cur, &mut body_env);
        let frame = self
            .try_stack
            .pop()
            .expect("just pushed this region's own frame above");

        let mut after_incoming: Vec<(BlockId, Env)> = Vec::new();
        if !self.is_terminated(body_cur) {
            // The normal exit runs the `finally` before the join, exactly the
            // way every other exit runs its own copy.
            if let Some(block) = finally {
                self.lower_stmts(&block.stmts, &mut body_cur, &mut body_env);
            }
            if !self.is_terminated(body_cur) {
                self.seal(body_cur, Terminator::Jump(after_block));
                after_incoming.push((body_cur, body_env));
            }
        }

        // The phis first, then the binding: `mwl-codegen` requires a block's
        // phis to be its leading run, and `merge_envs` appends.
        let dispatch_env = self.merge_envs(handler_block, &frame.edges, env);
        let (thrown_v, _) = self.emit(handler_block, Ty::Object, InstKind::TakeThrown);
        let mut join = CatchJoin {
            after_block,
            dispatch_env,
            after_incoming: &mut after_incoming,
        };
        self.lower_catch_clauses(handler_block, thrown_v, catches, finally, &mut join);

        *env = self.merge_envs(after_block, &after_incoming, env);
        *cur = after_block;
    }

    /// The `instanceof`-chain dispatch a `try`'s clauses lower to, plus the
    /// re-raise that ends it — see [`Self::lower_try`] for the shape and why
    /// it needs no mechanism of its own.
    fn lower_catch_clauses(
        &mut self,
        dispatch: BlockId,
        thrown: ValueId,
        catches: &'a [CatchClause],
        finally: Option<&'a Block>,
        join: &mut CatchJoin<'_>,
    ) {
        let mut test_block = dispatch;
        for clause in catches {
            let caught = self.catch_clause_type(clause);
            let (cond, _) = self.emit(
                test_block,
                Ty::Bool,
                InstKind::InstanceOf {
                    value: thrown,
                    class: caught,
                },
            );
            let handler = self.new_block();
            let next = self.new_block();
            let then_edge = self.ids.next_edge(clause.body.span);
            let else_edge = self.ids.next_edge(clause.body.span);
            self.seal(
                test_block,
                Terminator::Branch {
                    cond,
                    then_block: handler,
                    then_edge,
                    else_block: next,
                    else_edge,
                },
            );

            let mut handler_env = join.dispatch_env.clone();
            let mut handler_cur = handler;
            let bound = clause
                .var
                .map(|span| strip_sigil(span_text(self.src, span)).to_owned());
            match &bound {
                Some(name) => {
                    handler_env.insert(name.clone(), (thrown, Ty::Object));
                }
                // `catch (Throwable) { … }` names nothing, so the reference
                // `TakeThrown` produced has no slot to live in.
                None => self.emit_release(handler, thrown),
            }
            self.lower_stmts(&clause.body.stmts, &mut handler_cur, &mut handler_env);
            if !self.is_terminated(handler_cur) {
                if let Some(name) = &bound
                    && let Some(&(v, _)) = handler_env.get(name)
                {
                    self.emit_release(handler_cur, v);
                    handler_env.remove(name);
                }
                if let Some(block) = finally {
                    self.lower_stmts(&block.stmts, &mut handler_cur, &mut handler_env);
                }
                if !self.is_terminated(handler_cur) {
                    self.seal(handler_cur, Terminator::Jump(join.after_block));
                    join.after_incoming.push((handler_cur, handler_env));
                }
            }
            test_block = next;
        }

        // Nothing matched — run the `finally` and hand the very same reference
        // back to the context, so the exception leaves this frame unchanged.
        let mut rethrow_env = join.dispatch_env.clone();
        let mut rethrow_cur = test_block;
        if let Some(block) = finally {
            self.lower_stmts(&block.stmts, &mut rethrow_cur, &mut rethrow_env);
        }
        if !self.is_terminated(rethrow_cur) {
            let landing = self.landing_block(&rethrow_env);
            self.seal(
                rethrow_cur,
                Terminator::Throw {
                    value: thrown,
                    landing,
                },
            );
        }
    }

    /// Lowers a copy of every enclosing region's `finally` body, innermost
    /// first — what a `return` inside a protected region owes before it
    /// leaves the frame.
    ///
    /// **Duplicated at each exit rather than shared.** A shared body would
    /// need either a subroutine call (JSR, which nothing in this IR has) or a
    /// dispatch on "where do I go afterwards", and every real compiler that
    /// tried the latter found the same thing: the exit paths are few and the
    /// bookkeeping is not. Lowering the AST twice costs nothing at run time
    /// and needs no new IR shape at all.
    ///
    /// Each frame is *popped* before its own `finally` is lowered, so a call
    /// inside that body reaches the next region out rather than looping back
    /// into the handler it is already running — and pushed back afterwards, so
    /// the caller's own bracketing is untouched.
    fn run_pending_finallys(&mut self, cur: &mut BlockId, env: &mut Env) {
        let mut saved = Vec::new();
        while let Some(frame) = self.try_stack.pop() {
            if let Some(block) = frame.finally {
                self.lower_stmts(&block.stmts, cur, env);
            }
            let done = self.is_terminated(*cur);
            saved.push(frame);
            if done {
                break;
            }
        }
        while let Some(frame) = saved.pop() {
            self.try_stack.push(frame);
        }
    }

    /// The class label a `catch` clause tests against.
    ///
    /// Deliberately the *written* text rather than a resolved `QName`: this
    /// crate never depends on `mwl-hir`, and
    /// [ADR 0015](../../../docs/adr/0015-no-name-aliasing.md) forbids import
    /// renaming, so a bare `LogicError` in source is the global `LogicError`
    /// and nothing else. A leading `\\` is stripped, since
    /// `mwl_types::layout` keys a class by its rendered `QName`, which never
    /// carries one.
    ///
    /// # Known gap
    ///
    /// A clause naming a class inside a `namespace` block resolves against the
    /// file's namespace at check time and against nothing here, so its label
    /// will not match the layout table's. That is the same missing resolution
    /// [`InstKind::InstanceOf`] avoided by having the checker record the
    /// answer, and the same fix applies — `mwl_types` recording a resolved
    /// `QName` per clause.
    fn catch_clause_type(&self, clause: &CatchClause) -> String {
        span_text(self.src, clause.ty.span)
            .trim()
            .trim_start_matches('\\')
            .to_owned()
    }

    /// `$x = expr;` or `$obj->prop = expr;` as a bare expression statement —
    /// SSA renaming needs no join logic here, only a fresh binding in `env`
    /// (a local target) or a [`InstKind::FieldSet`] (a property target).
    fn lower_reassignment(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        let ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
            by_ref: false,
        } = &e.kind
        else {
            unreachable!("Self::lower_expr_stmt only routes a plain `AssignOp::Assign` here");
        };
        match &target.kind {
            ExprKind::Variable(name_span) => {
                let lname = strip_sigil(span_text(self.src, *name_span)).to_owned();
                let expected = env.get(&lname).map(|&(_, t)| t);
                let (v, ty) = self.lower_expr_top(value, expected, env, cur);
                self.bind_local(*cur, env, lname, v, ty, value);
            }
            // `$obj->prop = expr;` — the receiver's declaring class comes
            // from `self.exprs`, exactly like `ExprKind::PropertyAccess`'s
            // own read-side lowering in `Self::lower_expr`: `check_assign`'s
            // general (non-plain-local) arm in `mwl_types::expr` routes the
            // target through the ordinary `check_property_access`, which
            // records the same `ExprInfo::Property` entry a read would, keyed
            // by the `PropertyAccess` expression's own span — i.e. `target.span`
            // here. Refcounting mirrors `Self::bind_local`'s local-slot policy,
            // adapted to a field with no `Env` entry to consult before the
            // overwrite: retain the new value first (if it's an aliasing read,
            // same `is_aliasing_read` judgment), *then* read the field's
            // previous value back with a `FieldGet` and release it — retain
            // before release, same order `bind_local` uses, so a
            // self-assignment (`$obj->prop = $obj->prop;`) never observes a
            // transient zero refcount.
            ExprKind::PropertyAccess {
                object, nullsafe, ..
            } => {
                assert!(
                    !*nullsafe,
                    "mwl-ir does not yet lower a nullsafe property assignment target (`?->`); \
                     see the crate docs' known gaps"
                );
                let Some(ExprInfo::Property { class, name, ty }) = self.exprs.lookup(target.span)
                else {
                    panic!(
                        "mwl-ir: a property assignment target at {:?} has no resolved declaring \
                         class recorded in the typed-expression table — either it wasn't checked \
                         with the same table, or its receiver erased to a shape/plain `object` \
                         (ADR 0036 § 4), which this crate does not yet lower (see the crate docs' \
                         known gaps)",
                        target.span
                    );
                };
                let field_ty = lower_checked_ty(*ty, self.checked_types);
                let class_label = class.to_string();
                let field_name = name.clone();
                let (object_v, _) = self.lower_expr(object, None, env, *cur);
                let (v, _) = self.lower_expr_top(value, Some(field_ty), env, cur);
                if field_ty.is_refcounted() && is_aliasing_read(&value.kind) {
                    self.emit_retain(*cur, v);
                }
                if field_ty.is_refcounted() {
                    let (old_v, _) = self.emit(
                        *cur,
                        field_ty,
                        InstKind::FieldGet {
                            object: object_v,
                            class: class_label.clone(),
                            field: field_name.clone(),
                        },
                    );
                    self.emit_release(*cur, old_v);
                }
                self.emit_field_set(*cur, object_v, class_label, field_name, v);
            }
            // `$arr[$i] = expr;` — the element's declared type comes from
            // `self.exprs`, exactly like the read side above (`check_assign`'s
            // general arm routes the target back through the same
            // `check_expr`/`ExprKind::Index` path, so it records the same
            // `ExprInfo::Index` entry a read would, keyed by `target.span`).
            // Unlike a property write, there is no previous value to read
            // back and release here — see `InstKind::ArraySet`'s own doc
            // comment for why an array key may or may not already be
            // present, so this bundles the whole replace-or-insert into one
            // instruction rather than a get/release pair. `base[] = expr;`
            // (`index` is `None`) — PHP's append syntax — lowers to
            // `InstKind::ArrayAppend` instead: see that variant's own doc
            // comment for why no key is lowered or even computed here at all,
            // unlike every other `Index`-target write.
            //
            // Both instructions *yield* the array that now holds the entry
            // (ADR 0007 § 5's copy-on-write separation produces a different
            // allocation), so the write is not finished until the base's
            // holder has been re-pointed at that result —
            // `Self::write_back_array` does exactly that, and its own doc
            // comment owns why no retain or release goes with it.
            ExprKind::Index { base, index } => {
                let Some(ExprInfo::Index { elem_ty }) = self.exprs.lookup(target.span) else {
                    panic!(
                        "mwl-ir: an array-index assignment target at {:?} has no resolved \
                         element type recorded in the typed-expression table — either it wasn't \
                         checked with the same table, or its base erased to `mixed` (an \
                         unresolved array), which this crate does not yet lower (see the crate \
                         docs' known gaps)",
                        target.span
                    );
                };
                let elem_ty = lower_checked_ty(*elem_ty, self.checked_types);
                let (array_v, _) = self.lower_expr(base, None, env, *cur);
                let written = match index {
                    None => {
                        let (v, _) = self.lower_expr_top(value, Some(elem_ty), env, cur);
                        if elem_ty.is_refcounted() && is_aliasing_read(&value.kind) {
                            self.emit_retain(*cur, v);
                        }
                        self.emit_array_append(*cur, array_v, v)
                    }
                    Some(index) => {
                        let (key_v, key_aliasing) = self.lower_array_key(index, env, *cur);
                        if key_aliasing {
                            self.emit_retain(*cur, key_v);
                        }
                        let (v, _) = self.lower_expr_top(value, Some(elem_ty), env, cur);
                        if elem_ty.is_refcounted() && is_aliasing_read(&value.kind) {
                            self.emit_retain(*cur, v);
                        }
                        self.emit_array_set(*cur, array_v, key_v, v)
                    }
                };
                self.write_back_array(base, written, env, *cur);
            }
            other => panic!(
                "mwl-ir's control-flow slice only lowers reassignment to a plain local, a \
                 compile-time-known property, or a compile-time-known array element, not \
                 {other:?}"
            ),
        }
    }

    /// `if (cond) then (else else_)?` — the module docs describe the
    /// merge-point construction this drives.
    ///
    /// # Panics
    ///
    /// Panics naming the case for a `cond` whose static type
    /// [`Self::lower_truthy_cond`] doesn't yet convert — see that method's
    /// own doc comment for exactly what's covered and what still isn't
    /// (`null`/`mixed`/a union).
    fn lower_if(
        &mut self,
        cond: &Expr,
        then: &'a Stmt,
        else_: Option<&'a Stmt>,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let cond_v = self.lower_truthy_cond(cond, env, cur);

        let merge_block = self.new_block();
        let then_block = self.new_block();
        let then_edge = self.ids.next_edge(then.span);
        let (else_block, else_edge) = match else_ {
            Some(else_stmt) => (self.new_block(), self.ids.next_edge(else_stmt.span)),
            // No `else`: the branch's false edge goes straight to the merge
            // block, carrying the pre-branch environment unchanged.
            None => (merge_block, self.ids.next_edge(cond.span)),
        };
        self.seal(
            *cur,
            Terminator::Branch {
                cond: cond_v,
                then_block,
                then_edge,
                else_block,
                else_edge,
            },
        );
        let pre_branch_block = *cur;

        let mut then_env = env.clone();
        let mut then_cur = then_block;
        self.lower_stmt(then, &mut then_cur, &mut then_env);
        let then_reaches_merge = !self.is_terminated(then_cur);
        if then_reaches_merge {
            self.seal(then_cur, Terminator::Jump(merge_block));
        }

        let (else_env, else_reaches_merge, else_cur) = if let Some(else_stmt) = else_ {
            let mut else_env = env.clone();
            let mut else_cur = else_block;
            self.lower_stmt(else_stmt, &mut else_cur, &mut else_env);
            let reaches = !self.is_terminated(else_cur);
            if reaches {
                self.seal(else_cur, Terminator::Jump(merge_block));
            }
            (else_env, reaches, else_cur)
        } else {
            (env.clone(), true, pre_branch_block)
        };

        let mut incoming = Vec::new();
        if then_reaches_merge {
            incoming.push((then_cur, then_env));
        }
        if else_reaches_merge {
            incoming.push((else_cur, else_env));
        }

        *env = self.merge_envs(merge_block, &incoming, env);
        *cur = merge_block;
    }

    /// `while (cond) body` — the module docs describe the loop-header phi
    /// construction this drives. A `break`/`continue` anywhere inside `body`
    /// (at any nesting depth reachable through `if`/nested `{}`) adds one
    /// more incoming edge to fold in: [`Self::lower_break`]/
    /// [`Self::lower_continue`] record their own `(block, env)` pair into
    /// the [`LoopFrame`] this method pushes before lowering `body` and pops
    /// once it returns — a `continue`'s edge joins the body's own
    /// fall-through exit when patching the header's phis below, and a
    /// `break`'s edge joins the condition's false edge when building the
    /// loop's own after-block environment, both through the same
    /// [`Self::merge_envs`]/phi-patch machinery a fall-through-only loop
    /// already used.
    ///
    /// # Panics
    ///
    /// Panics naming the case for a `cond` whose static type
    /// [`Self::lower_truthy_cond`] doesn't yet convert — see [`Self::lower_if`]'s
    /// panic doc, the same restriction applies here.
    fn lower_while(&mut self, cond: &Expr, body: &'a Stmt, cur: &mut BlockId, env: &mut Env) {
        let mut seen = FxHashSet::default();
        let mut reassigned = Vec::new();
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);

        let pre_block = *cur;
        let header_block = self.new_block();
        self.seal(pre_block, Terminator::Jump(header_block));

        // Seed a phi for every pre-existing local the body might touch,
        // carrying only the pre-loop incoming edge for now — the back edge
        // is patched in once the body's exit environment is known, below.
        let mut header_env = env.clone();
        let mut phi_slots: Vec<(String, usize)> = Vec::new();
        for name in &reassigned {
            let Some(&(pre_v, ty)) = env.get(name) else {
                continue;
            };
            let phi_v = self.ids.next_value();
            let inst_index = self.block_insts[header_block.index() as usize].len();
            self.block_insts[header_block.index() as usize].push(Inst {
                result: Some(phi_v),
                ty: Some(ty),
                kind: InstKind::Phi {
                    incoming: vec![(pre_block, pre_v)],
                },
                on_error: None,
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

        // `cond` may itself need to branch (a `&&`/`||`/ternary condition —
        // see `Self::lower_truthy_cond`), in which case the loop's own
        // `Branch` terminator belongs on whichever block that evaluation
        // actually ends in, not necessarily `header_block` itself. The header
        // phis above still physically live in `header_block` — cond
        // lowering only ever *appends* blocks after it, never touches those.
        let mut cond_end = header_block;
        let cond_v = self.lower_truthy_cond(cond, &header_env, &mut cond_end);

        let body_block = self.new_block();
        let after_block = self.new_block();
        let body_edge = self.ids.next_edge(body.span);
        let after_edge = self.ids.next_edge(cond.span);
        self.seal(
            cond_end,
            Terminator::Branch {
                cond: cond_v,
                then_block: body_block,
                then_edge: body_edge,
                else_block: after_block,
                else_edge: after_edge,
            },
        );

        self.loop_stack.push(LoopFrame {
            header_block,
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned: Vec::new(),
            loop_private: Vec::new(),
        });
        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        self.lower_stmt(body, &mut body_cur, &mut body_env);
        let frame = self
            .loop_stack
            .pop()
            .expect("just pushed this loop's own frame above");

        // Every edge that loops back to the header: the body's own
        // fall-through exit (if it reaches one) plus one more per `continue`
        // recorded while lowering the body above.
        let mut back_edges: Vec<(BlockId, Env)> = Vec::new();
        if !self.is_terminated(body_cur) {
            // Reserved safepoint poll site (loop back edge) — see
            // `InstKind::Safepoint`'s own doc comment. Placed on the actual
            // back edge, not the loop header, so a body that never reaches
            // it (e.g. it always `return`s) polls zero times per skipped
            // iteration, same as a functional poll would. A `continue`'s own
            // back edge already got its own poll in `Self::lower_continue`.
            self.emit_safepoint(body_cur);
            self.seal(body_cur, Terminator::Jump(header_block));
            back_edges.push((body_cur, body_env));
        }
        back_edges.extend(frame.continue_edges);
        for (name, inst_index) in &phi_slots {
            for (block, back_env) in &back_edges {
                let &(back_v, _) = back_env.get(name).unwrap_or_else(|| {
                    panic!(
                        "mwl-ir: `{name}` was reassigned in a while body per the syntactic scan \
                         but is missing from a back edge's exit environment — bug in \
                         collect_reassigned_locals"
                    )
                });
                let inst = &mut self.block_insts[header_block.index() as usize][*inst_index];
                let InstKind::Phi { incoming } = &mut inst.kind else {
                    unreachable!("phi_slots only ever indexes a Phi instruction");
                };
                incoming.push((*block, back_v));
            }
        }
        // If no back edge exists at all (e.g. the body always `return`s and
        // never `continue`s), each header phi keeps its single pre-loop
        // incoming edge — a degenerate but valid phi, since no optimizer
        // exists yet to fold a single-input phi away.

        // The loop's own exit environment: the condition's ordinary false
        // edge (carrying `header_env` unchanged, since `lower_truthy_cond`
        // only ever reads `header_env`, never mutates it) plus one more
        // incoming edge per `break` recorded above. `Self::merge_envs`
        // degenerates to a plain clone with no new phi at all when there is
        // no `break` to fold in, exactly the prior "loop exit is always
        // `header_env`" behavior.
        let mut after_incoming: Vec<(BlockId, Env)> = vec![(cond_end, header_env.clone())];
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &header_env);
        *cur = after_block;
    }

    /// `foreach ($subject as $k => $v) body` over an `array<T>` — ADR 0007
    /// § 5's insertion order, walked by the cursor
    /// [`InstKind::ArrayNextSlot`] steps.
    ///
    /// Structurally [`Self::lower_while`] with a synthesized condition, and it
    /// reuses that method's whole phi/`break`/`continue` machinery unchanged.
    /// Three things are its own:
    ///
    /// * **The loop owns a second reference to the array**, retained here when
    ///   the subject [`is_aliasing_read`]s an existing slot (a fresh subject —
    ///   a call's result, a literal — already has exactly one owner, this
    ///   frame's). That reference is what makes PHP's by-value `foreach` fall
    ///   out of copy-on-write rather than needing a snapshot: a write to the
    ///   same array inside the body now sees a refcount above one and
    ///   *separates*, leaving the cursor walking what the loop started on. It
    ///   is released once, in the loop's own after-block.
    /// * **The array reference and the cursor live in the [`Env`]** under
    ///   reserved `foreach#N`/`foreach#N$cursor` names. A `#` can never appear
    ///   in an MWL identifier, so neither can collide with a local. Being
    ///   ordinary `Env` members is what gets them for free: the cursor gets
    ///   its loop-header phi through the same seeding
    ///   [`Self::lower_while`] gives any reassigned local, and both are swept
    ///   by [`Self::release_all_locals`] on a `return` or a throw out of the
    ///   body. [`LoopFrame::loop_private`] then hides them from everything
    ///   after the loop.
    /// * **The key and value bindings are owned for one iteration each**, and
    ///   released at every point an iteration ends — see
    ///   [`LoopFrame::iteration_owned`]. The cursor is advanced at the *top*
    ///   of the body rather than the bottom precisely so that a `continue`'s
    ///   back edge needs no step of its own.
    ///
    /// # Panics
    ///
    /// Panics naming the case for a subject that is not an `array<T>` (ADR
    /// 0053's `Iterable`/`Iterator` are a separate lowering, over a
    /// user-visible interface rather than these primitives), for `&$v` by
    /// reference, for a key binding declared as anything but `string` (ADR
    /// 0007 § 5 makes every stored key a `string`; an `int` key binding needs
    /// a string-to-int conversion nothing lowers yet), and for a binding with
    /// no declared type at all, which `mwl_types` already diagnosed.
    #[expect(
        clippy::too_many_arguments,
        reason = "the arguments are one `StmtKind::Foreach`'s own fields plus \
                  the `(cur, env)` pair every lowering method threads"
    )]
    fn lower_foreach(
        &mut self,
        subject: &Expr,
        key: Option<&ForeachBinding>,
        value: &ForeachBinding,
        value_by_ref: bool,
        body: &'a Stmt,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        assert!(
            !value_by_ref,
            "mwl-ir does not yet lower `foreach (… as &$v)`: a by-reference value binding writes \
             back through the array it is walking, which is the one shape ADR 0007 § 5's \
             copy-on-write separation has to be told not to separate; see the crate docs' known \
             gaps"
        );
        let value_ty = binding_ty(value, "value");
        let key_binding = key.map(|k| {
            let ty = binding_ty(k, "key");
            assert!(
                ty == Ty::Str,
                "mwl-ir lowers a `foreach` key binding only at `string`, ADR 0007 § 5's one \
                 stored key type — got {ty:?}, which would need a string-to-key conversion this \
                 crate does not have; see the crate docs' known gaps"
            );
            strip_sigil(span_text(self.src, k.name)).to_owned()
        });
        let value_name = strip_sigil(span_text(self.src, value.name)).to_owned();

        let (array_v, array_ty) = self.lower_expr_top(subject, None, env, cur);
        assert!(
            array_ty == Ty::Array,
            "mwl-ir lowers `foreach` only over an `array<T>` — got {array_ty:?}; ADR 0053's \
             `Iterable`/`Iterator` subjects are their own lowering (see the crate docs' known \
             gaps)"
        );
        if is_aliasing_read(&subject.kind) {
            self.emit_retain(*cur, array_v);
        }

        let seq = self.foreach_seq;
        self.foreach_seq += 1;
        let array_name = format!("foreach#{seq}");
        let cursor_name = format!("foreach#{seq}$cursor");
        env.insert(array_name.clone(), (array_v, Ty::Array));
        let (zero_v, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
        env.insert(cursor_name.clone(), (zero_v, Ty::Int));

        // From here on this is `Self::lower_while`'s shape, with the cursor
        // prepended to the loop-carried names so it gets the same header phi
        // every reassigned local does.
        let mut seen = FxHashSet::default();
        let mut reassigned = vec![cursor_name.clone()];
        seen.insert(cursor_name.clone());
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);

        let pre_block = *cur;
        let header_block = self.new_block();
        self.seal(pre_block, Terminator::Jump(header_block));

        let mut header_env = env.clone();
        let mut phi_slots: Vec<(String, usize)> = Vec::new();
        for name in &reassigned {
            let Some(&(pre_v, ty)) = env.get(name) else {
                continue;
            };
            let phi_v = self.ids.next_value();
            let inst_index = self.block_insts[header_block.index() as usize].len();
            self.block_insts[header_block.index() as usize].push(Inst {
                result: Some(phi_v),
                ty: Some(ty),
                kind: InstKind::Phi {
                    incoming: vec![(pre_block, pre_v)],
                },
                on_error: None,
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

        let cursor_v = header_env[&cursor_name].0;
        let (slot_v, _) = self.emit(
            header_block,
            Ty::Int,
            InstKind::ArrayNextSlot {
                array: array_v,
                from: cursor_v,
            },
        );
        let (exhausted_at, _) = self.emit(header_block, Ty::Int, InstKind::ConstInt(0));
        let (more_v, _) = self.emit(
            header_block,
            Ty::Bool,
            InstKind::BinOp {
                op: BinOp::GtEq,
                lhs: slot_v,
                rhs: exhausted_at,
            },
        );

        let body_block = self.new_block();
        let after_block = self.new_block();
        let body_edge = self.ids.next_edge(body.span);
        let after_edge = self.ids.next_edge(subject.span);
        self.seal(
            header_block,
            Terminator::Branch {
                cond: more_v,
                then_block: body_block,
                then_edge: body_edge,
                else_block: after_block,
                else_edge: after_edge,
            },
        );

        let mut iteration_owned = Vec::new();
        if let Some(name) = &key_binding {
            iteration_owned.push(name.clone());
        }
        iteration_owned.push(value_name.clone());
        self.loop_stack.push(LoopFrame {
            header_block,
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned,
            loop_private: vec![array_name.clone(), cursor_name.clone()],
        });

        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        let (one_v, _) = self.emit(body_cur, Ty::Int, InstKind::ConstInt(1));
        let (next_v, _) = self.emit(
            body_cur,
            Ty::Int,
            InstKind::BinOp {
                op: BinOp::Add,
                lhs: slot_v,
                rhs: one_v,
            },
        );
        body_env.insert(cursor_name.clone(), (next_v, Ty::Int));
        if let Some(name) = &key_binding {
            let (k_v, _) = self.emit(
                body_cur,
                Ty::Str,
                InstKind::ArrayKeyAt {
                    array: array_v,
                    slot: slot_v,
                },
            );
            body_env.insert(name.clone(), (k_v, Ty::Str));
        }
        let (v_v, _) = self.emit(
            body_cur,
            value_ty,
            InstKind::ArrayValueAt {
                array: array_v,
                slot: slot_v,
            },
        );
        if value_ty.is_refcounted() {
            self.emit_retain(body_cur, v_v);
        }
        body_env.insert(value_name, (v_v, value_ty));

        self.lower_stmt(body, &mut body_cur, &mut body_env);
        let mut back_edges: Vec<(BlockId, Env)> = Vec::new();
        if !self.is_terminated(body_cur) {
            self.end_iteration(body_cur, &mut body_env);
            self.emit_safepoint(body_cur);
            self.seal(body_cur, Terminator::Jump(header_block));
            back_edges.push((body_cur, body_env));
        }
        let frame = self
            .loop_stack
            .pop()
            .expect("just pushed this loop's own frame above");
        back_edges.extend(frame.continue_edges);
        for (name, inst_index) in &phi_slots {
            for (block, back_env) in &back_edges {
                let &(back_v, _) = back_env.get(name).unwrap_or_else(|| {
                    panic!(
                        "mwl-ir: `{name}` was reassigned in a foreach body per the syntactic \
                         scan but is missing from a back edge's exit environment — bug in \
                         collect_reassigned_locals"
                    )
                });
                let inst = &mut self.block_insts[header_block.index() as usize][*inst_index];
                let InstKind::Phi { incoming } = &mut inst.kind else {
                    unreachable!("phi_slots only ever indexes a Phi instruction");
                };
                incoming.push((*block, back_v));
            }
        }

        let mut exit_env = header_env.clone();
        exit_env.remove(&array_name);
        exit_env.remove(&cursor_name);
        let mut after_incoming: Vec<(BlockId, Env)> = vec![(header_block, exit_env.clone())];
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &exit_env);
        self.emit_release(after_block, array_v);
        *cur = after_block;
    }

    /// `unset($a[$k]);` — the one `unset` target ADR 0028 § 3 leaves
    /// standing, lowered to [`InstKind::ArrayUnset`] and written back through
    /// the same [`Self::write_back_array`] an element *write* uses, since
    /// removing an entry separates a shared array exactly the way writing one
    /// does.
    ///
    /// The key is borrowed rather than stored, which inverts
    /// [`Self::lower_reassignment`]'s `Index`-target retain: an aliasing key
    /// needs nothing, and a freshly converted one (an `int` subscript's
    /// decimal-string form) is this frame's own single-owner temporary and is
    /// released once the removal has read it.
    ///
    /// # Panics
    ///
    /// Panics naming the shape for any other `unset` operand. A declared
    /// property is already a `mwl_types` diagnostic (ADR 0028 § 3), and a bare
    /// local has no meaning in MWL at all — every binding is typed and
    /// definitely assigned, so there is no "make this name undefined again".
    fn lower_unset(&mut self, target: &Expr, env: &mut Env, cur: BlockId) {
        let ExprKind::Index {
            base,
            index: Some(index),
        } = &target.kind
        else {
            panic!(
                "mwl-ir lowers `unset` only on an array element with an explicit subscript — \
                 got {:?}; see the crate docs' known gaps",
                target.kind
            );
        };
        let (array_v, array_ty) = self.lower_expr(base, None, env, cur);
        assert!(
            array_ty == Ty::Array,
            "mwl-ir: an `unset` target's base lowered to {array_ty:?} rather than an array — \
             either it erased to `mixed`, which this crate does not yet lower, or \
             mwl_types::check_program accepted something it should not have"
        );
        let (key_v, key_aliasing) = self.lower_array_key(index, env, cur);
        let (written, _) = self.emit(
            cur,
            Ty::Array,
            InstKind::ArrayUnset {
                array: array_v,
                key: key_v,
            },
        );
        if !key_aliasing {
            self.emit_release(cur, key_v);
        }
        self.write_back_array(base, written, env, cur);
    }

    /// `break;`/`break 1;` — jumps straight to the enclosing loop's exit
    /// block, recording the current block and environment as one more
    /// incoming edge [`Self::lower_while`] folds into its own after-block
    /// merge once the body it's nested in finishes lowering. See
    /// [`Self::loop_exit_level`] for what `level` is allowed to be.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::loop_stack`] is empty — `mwl_types` does not yet
    /// reject a `break` outside any loop itself (see the crate docs' known
    /// gaps), so this is the one place that still gets checked, defensively,
    /// before building a jump with nothing to target.
    fn lower_break(&mut self, level: &Option<Expr>, cur: &mut BlockId, env: &Env) {
        self.loop_exit_level(level, "break");
        let after_block = self
            .loop_stack
            .last()
            .unwrap_or_else(|| {
                panic!(
                    "mwl-ir: `break` reached lowering with no enclosing loop on the loop stack \
                     — mwl_types should have already rejected this; see the crate docs' known \
                     gaps"
                )
            })
            .after_block;
        // A `break` ends the iteration it is in, so it owes exactly what a
        // back edge owes — see `LoopFrame::iteration_owned`. It additionally
        // hides the loop's own bookkeeping names from everything after the
        // loop, which the condition's own false edge never carries either.
        let mut exit_env = env.clone();
        self.end_iteration(*cur, &mut exit_env);
        for name in &self
            .loop_stack
            .last()
            .expect("just read the same stack above")
            .loop_private
        {
            exit_env.remove(name);
        }
        self.loop_stack
            .last_mut()
            .expect("just read the same stack above")
            .break_edges
            .push((*cur, exit_env));
        self.seal(*cur, Terminator::Jump(after_block));
    }

    /// Releases the innermost loop's per-iteration bindings and drops them
    /// from `env` — the one thing every point an iteration ends has in common
    /// (the body's fall-through back edge, a `continue`, a `break`).
    ///
    /// A no-op for a `while` loop, whose [`LoopFrame::iteration_owned`] is
    /// empty; see that field's own doc comment for the whole policy.
    fn end_iteration(&mut self, cur: BlockId, env: &mut Env) {
        let owned = self
            .loop_stack
            .last()
            .map(|frame| frame.iteration_owned.clone())
            .unwrap_or_default();
        for name in owned {
            if let Some((v, ty)) = env.remove(&name)
                && ty.is_refcounted()
            {
                self.emit_release(cur, v);
            }
        }
    }

    /// `continue;`/`continue 1;` — a loop back edge exactly like the body's
    /// own fall-through exit, so it gets the same safepoint poll and the
    /// same header-phi patching, folded in by [`Self::lower_while`] once the
    /// body it's nested in finishes lowering. See [`Self::loop_exit_level`]
    /// for what `level` is allowed to be.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::loop_stack`] is empty — see [`Self::lower_break`]'s
    /// panic doc, the same defensive check applies here.
    fn lower_continue(&mut self, level: &Option<Expr>, cur: &mut BlockId, env: &Env) {
        self.loop_exit_level(level, "continue");
        let header_block = self
            .loop_stack
            .last()
            .unwrap_or_else(|| {
                panic!(
                    "mwl-ir: `continue` reached lowering with no enclosing loop on the loop \
                     stack — mwl_types should have already rejected this; see the crate docs' \
                     known gaps"
                )
            })
            .header_block;
        // The next iteration rebinds a `foreach` header's key/value from
        // scratch, so this back edge ends the current one — see
        // `LoopFrame::iteration_owned`.
        let mut back_env = env.clone();
        self.end_iteration(*cur, &mut back_env);
        self.emit_safepoint(*cur);
        self.loop_stack
            .last_mut()
            .expect("just read the same stack above")
            .continue_edges
            .push((*cur, back_env));
        self.seal(*cur, Terminator::Jump(header_block));
    }

    /// Validates a `break`/`continue` statement's optional level operand —
    /// PHP allows `break N;`/`continue N;` to unwind `N` nested loops at
    /// once. Only `None` (defaults to level 1) or a literal `1` is accepted
    /// today: a non-literal level has no compile-time meaning to resolve,
    /// and `N > 1` would need every enclosing [`LoopFrame`] on
    /// [`Self::loop_stack`] up to the `N`-th, not just the innermost one, to
    /// become that statement's target — a real widening [`Self::lower_while`]
    /// doesn't do yet, left for whenever a fixture actually nests loops this
    /// deeply.
    ///
    /// # Panics
    ///
    /// Panics naming the gap for a non-literal level or one greater than 1.
    fn loop_exit_level(&self, level: &Option<Expr>, keyword: &str) {
        let Some(level_expr) = level else {
            return;
        };
        let ExprKind::Int(span) = &level_expr.kind else {
            panic!(
                "mwl-ir does not yet lower a `{keyword}` with a non-literal level; see the \
                 crate docs' known gaps"
            );
        };
        let (radix, digits) = int_literal_digits(self.src, *span);
        let n = u64::from_str_radix(&digits, radix).unwrap_or_else(|_| {
            panic!("mwl-ir: `{keyword}` level literal `{digits}` doesn't fit a u64")
        });
        assert!(
            n == 1,
            "mwl-ir does not yet lower `{keyword} {n}` — a multi-level {keyword}; see the crate \
             docs' known gaps"
        );
    }

    /// Merges the environments reaching a join block into one, inserting a
    /// [`InstKind::Phi`] for any local whose value differs across incoming
    /// edges. See the module docs for why a name missing from some incoming
    /// environment can be silently dropped rather than treated as an error.
    fn merge_envs(
        &mut self,
        merge_block: BlockId,
        incoming: &[(BlockId, Env)],
        pre_branch_env: &Env,
    ) -> Env {
        match incoming {
            [] => {
                // Every incoming edge terminated (e.g. both an `if`'s
                // branches always `return`) — control never reaches
                // `merge_block`. Anything lowered after it is dead code;
                // falling back to the pre-branch environment keeps lowering
                // total without needing a dedicated "unreachable" terminator
                // for what is, structurally, still a well-typed block.
                pre_branch_env.clone()
            }
            [(_, only)] => only.clone(),
            _ => {
                let mut merged = Env::default();
                // Sorted rather than left in `FxHashMap`'s bucket order: a
                // phi's id must depend only on source order (see the crate's
                // `ids` module docs on why), never on hash-table internals.
                let mut names: Vec<String> = incoming[0].1.keys().cloned().collect();
                names.sort_unstable();
                for name in names {
                    if !incoming[1..].iter().all(|(_, e)| e.contains_key(&name)) {
                        // Not bound on every incoming edge — per the module
                        // docs, checked input never uses such a name past
                        // this point, so it needs no entry in the merged env.
                        continue;
                    }
                    let values: Vec<(BlockId, ValueId, Ty)> = incoming
                        .iter()
                        .map(|(b, e)| {
                            let &(v, t) = &e[&name];
                            (*b, v, t)
                        })
                        .collect();
                    let ty = values[0].2;
                    let first_v = values[0].1;
                    if values.iter().all(|&(_, v, _)| v == first_v) {
                        merged.insert(name, (first_v, ty));
                    } else {
                        let (phi_v, _) = self.emit(
                            merge_block,
                            ty,
                            InstKind::Phi {
                                incoming: values.iter().map(|&(b, v, _)| (b, v)).collect(),
                            },
                        );
                        merged.insert(name, (phi_v, ty));
                    }
                }
                merged
            }
        }
    }

    /// A permissive syntactic over-approximation of "which existing locals
    /// might `stmt` reassign" — recurses into `{}`/`if`/`while` since a
    /// reassignment nested inside any of those still needs a loop-header
    /// phi. Anything else is ignored here; if it turns out to be an
    /// unsupported shape, [`Self::lower_stmt`] panics on it when actually
    /// lowering the body, same as always. A `LocalDecl` that *shadows* an
    /// outer local of the same name inside a nested block is not
    /// distinguished from a reassignment of the outer binding — the flat,
    /// unscoped `Env` this crate uses has no notion of nested lexical
    /// scopes yet; see the crate docs' known gaps.
    ///
    /// `out` collects each name once, in first-occurrence source order —
    /// deliberately not a `HashSet`'s own iteration order, which depends on
    /// hash-bucket layout rather than the source alone. The order it's
    /// walked in decides which [`ValueId`]/phi a header phi gets, and the
    /// crate's `ids` module docs already commit to every id depending only
    /// on source order.
    fn collect_reassigned_locals(
        &self,
        stmt: &Stmt,
        seen: &mut FxHashSet<String>,
        out: &mut Vec<String>,
    ) {
        match &stmt.kind {
            StmtKind::Expr(e) => {
                if let ExprKind::Assign {
                    op: AssignOp::Assign,
                    target,
                    by_ref: false,
                    ..
                } = &e.kind
                    && let Some(name) = self.rebound_local(target)
                    && seen.insert(name.clone())
                {
                    out.push(name);
                }
            }
            // `unset($a[$k]);` re-points `$a` at the separated array exactly
            // the way `$a[$k] = …;` does — see `Self::lower_unset` — so it
            // needs the identical loop-header phi.
            StmtKind::Unset(targets) => {
                for target in targets {
                    if let Some(name) = self.rebound_local(target)
                        && seen.insert(name.clone())
                    {
                        out.push(name);
                    }
                }
            }
            StmtKind::Block(b) => {
                for s in &b.stmts {
                    self.collect_reassigned_locals(s, seen, out);
                }
            }
            StmtKind::If { then, else_, .. } => {
                self.collect_reassigned_locals(then, seen, out);
                if let Some(e) = else_ {
                    self.collect_reassigned_locals(e, seen, out);
                }
            }
            StmtKind::While { body, .. } | StmtKind::Foreach { body, .. } => {
                self.collect_reassigned_locals(body, seen, out);
            }
            _ => {}
        }
    }

    /// Which local, if any, an assignment or `unset` target re-points — the
    /// name [`Self::collect_reassigned_locals`] owes a loop-header phi.
    ///
    /// A bare `$x` is the obvious one. An array element (`$a[$k]`, `$a[]`) is
    /// the less obvious one and matters just as much: ADR 0007 § 5's
    /// copy-on-write separation produces a *different* allocation, which
    /// [`Self::write_back_array`] stores back into the base's own local slot.
    /// Missing that phi would leave a loop body writing into the value the
    /// loop was entered with on every iteration instead of the one the last
    /// iteration produced — which is only invisible while the array is solely
    /// owned and therefore never actually separates.
    ///
    /// A property target (`$obj->p`, `$obj->items[$k]`) rebinds no local at
    /// all: the write goes to the object's own slot, which no phi describes.
    fn rebound_local(&self, target: &Expr) -> Option<String> {
        match &target.kind {
            ExprKind::Variable(name_span) => {
                Some(strip_sigil(span_text(self.src, *name_span)).to_owned())
            }
            ExprKind::Paren(inner) | ExprKind::Index { base: inner, .. } => {
                self.rebound_local(inner)
            }
            _ => None,
        }
    }

    fn lower_expr(
        &mut self,
        expr: &Expr,
        expected: Option<Ty>,
        env: &Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        match &expr.kind {
            // `(expr)` is fully transparent — `mwl_types::expr::check_expr`'s
            // own `ExprKind::Paren` arm just recurses with the same
            // `expected`, and this does the same for lowering. Needed for
            // `!($a && $b)`-shaped input at all: `!` binds tighter than
            // `&&`/`||` in the grammar, so writing "not (a and b)" requires
            // the explicit parens, which the parser keeps as their own node
            // rather than discarding.
            ExprKind::Paren(inner) => self.lower_expr(inner, expected, env, cur),
            ExprKind::Bool(b) => self.emit(cur, Ty::Bool, InstKind::ConstBool(*b)),
            // ADR 0007 § 4, mirroring `mwl_types::expr::infer`'s own rule: a
            // bare integer literal means `uint` exactly where that's the
            // expected type, `int` otherwise. `mwl_types::expr::infer`'s own
            // `ExprKind::Int` arm now enforces ADR 0007 § 4's magnitude rule
            // at check time — too large for `int` is only legal where `uint`
            // is expected, and too large even for `uint`'s full `u64` range
            // is a diagnostic regardless — so `lower_method`'s usual "trusts
            // its input already passed `mwl_types::check_program`" contract
            // (see the crate docs) covers this too: the `unwrap_or_else`
            // panics below are unreachable for anything the checker accepted,
            // the same defensive-invariant shape as `Env::get`'s own panic on
            // an undeclared local just above.
            ExprKind::Int(span) => {
                let (radix, digits) = int_literal_digits(self.src, *span);
                if expected == Some(Ty::Uint) {
                    let n: u64 = u64::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                        panic!("mwl-ir: integer literal `{digits}` doesn't fit a `uint`")
                    });
                    self.emit(cur, Ty::Uint, InstKind::ConstUint(n))
                } else {
                    let n: i64 = i64::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                        panic!("mwl-ir: integer literal `{digits}` doesn't fit an `int`")
                    });
                    self.emit(cur, Ty::Int, InstKind::ConstInt(n))
                }
            }
            ExprKind::Float(span) => {
                let digits = clean_digits(self.src, *span);
                let n: f64 = digits
                    .parse()
                    .unwrap_or_else(|_| panic!("mwl-ir: float literal `{digits}` failed to parse"));
                self.emit(cur, Ty::Float, InstKind::ConstFloat(n))
            }
            // A fresh `Ty::Str` value with exactly one natural owner — see
            // `Self::bind_local`'s doc comment for why a value produced here
            // never needs a retain of its own, only whatever consumes it.
            ExprKind::Str(span) => {
                let s = cook_str_literal(self.src, *span);
                self.emit(cur, Ty::Str, InstKind::ConstStr(s))
            }
            // A double-quoted- or heredoc-sourced `Interpolated` both lower
            // to the same `InstKind::Concat` chain a written-out `.`
            // expression already does — see `Self::lower_interpolated_parts`'s
            // own doc comment for the one subtlety plain N-ary `.`-folding
            // wouldn't force into the open on its own, and for how a heredoc's
            // own flexible-indentation strip fits into that fold. Never a
            // nowdoc: `mwl_syntax::parser::collapse_string_parts` only ever
            // reaches `Interpolated` when at least one interpolation site
            // was used, which a nowdoc's body can never contain.
            ExprKind::Interpolated(parts) => {
                let raw = span_text(self.src, expr.span);
                assert!(
                    raw.starts_with('"') || raw.starts_with("<<<"),
                    "mwl-ir only lowers a double-quoted or heredoc-sourced Interpolated string — \
                     got {raw:?}; see the crate docs' known gaps"
                );
                self.lower_interpolated_parts(parts, expr.span, env, cur)
            }
            ExprKind::Variable(span) => {
                let name = strip_sigil(span_text(self.src, *span));
                let &(v, ty) = env.get(name).unwrap_or_else(|| {
                    panic!(
                        "mwl-ir: undeclared local `${name}` — lower_method trusts its input \
                         already passed mwl_types::check_program"
                    )
                });
                (v, ty)
            }
            // `!` always produces `Ty::Bool` via ADR 0035's truthy table
            // (`Self::negate_truthy`), regardless of `inner`'s own type — a
            // separate arm from the plain arithmetic/bitwise unary operators
            // below, which just pass their operand's own type straight
            // through. `inner` is lowered with the plain, non-branching
            // `Self::lower_expr` here (this arm has no `&mut BlockId` to
            // redirect) — a nested `&&`/`||`/ternary `inner` still panics via
            // that call's own arms; `Self::lower_not` is the top-level
            // sibling that supports composing with those.
            ExprKind::Unary {
                op: AstUnaryOp::Not,
                expr: inner,
            } => {
                let (v, ty) = self.lower_expr(inner, None, env, cur);
                let r = self.negate_truthy(v, ty, is_aliasing_read(&inner.kind), cur);
                (r, Ty::Bool)
            }
            ExprKind::Unary { op, expr: inner } => {
                let (v, ty) = self.lower_expr(inner, expected, env, cur);
                let uop = match op {
                    AstUnaryOp::Neg => UnOp::Neg,
                    other => panic!(
                        "mwl-ir's control-flow slice only lowers unary `-`/`!` — got {other:?}; \
                         see the crate docs' known gaps"
                    ),
                };
                self.emit(
                    cur,
                    ty,
                    InstKind::UnOp {
                        op: uop,
                        operand: v,
                    },
                )
            }
            // `.` concatenation is not `InstKind::BinOp` — it allocates a
            // fresh buffer rather than computing a native scalar result, so
            // it gets its own arm (and its own `InstKind::Concat`) ahead of
            // the scalar-operator table below. Each operand goes through
            // `Self::concat_operand` first, which converts a scalar through
            // a new `InstKind::HelperCall` when it isn't already `Ty::Str` —
            // a `Stringable`-object operand (also accepted by
            // `mwl_types::expr::check_expr`'s own `require_stringable`) still
            // panics there, since it needs a resolved `toString` call this
            // crate can't synthesize yet. `concat_operand` also reports
            // whether the value it hands back aliases storage a durable slot
            // still owns; an operand that doesn't (a literal, a nested
            // `Concat`'s own result, or a freshly converted `HelperCall`
            // result) is released right after this `Concat` reads it, since
            // nothing else ever will — the same "release a fresh value once
            // its one and only use is done" precedent `Self::lower_expr_stmt`
            // already sets for a bare call/`new` statement.
            ExprKind::Binary {
                op: BinaryOp::Concat,
                lhs,
                rhs,
            } => {
                let (lv, l_alias) = self.concat_operand(lhs, env, cur);
                let (rv, r_alias) = self.concat_operand(rhs, env, cur);
                let result = self.emit(cur, Ty::Str, InstKind::Concat { lhs: lv, rhs: rv });
                if !l_alias {
                    self.emit_release(cur, lv);
                }
                if !r_alias {
                    self.emit_release(cur, rv);
                }
                result
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let (lv, lty) = self.lower_expr(lhs, expected, env, cur);
                let (rv, _) = self.lower_expr(rhs, Some(lty), env, cur);
                let (bop, ty) = match op {
                    BinaryOp::Add => (BinOp::Add, lty),
                    BinaryOp::Sub => (BinOp::Sub, lty),
                    BinaryOp::Mul => (BinOp::Mul, lty),
                    BinaryOp::Div => (BinOp::Div, lty),
                    BinaryOp::Mod => (BinOp::Mod, lty),
                    BinaryOp::Eq | BinaryOp::Identical => (BinOp::Eq, Ty::Bool),
                    BinaryOp::NotEq | BinaryOp::NotIdentical => (BinOp::NotEq, Ty::Bool),
                    BinaryOp::Lt => (BinOp::Lt, Ty::Bool),
                    BinaryOp::LtEq => (BinOp::LtEq, Ty::Bool),
                    BinaryOp::Gt => (BinOp::Gt, Ty::Bool),
                    BinaryOp::GtEq => (BinOp::GtEq, Ty::Bool),
                    other => panic!(
                        "mwl-ir's control-flow slice only lowers arithmetic/equality/ordering \
                         operators — got {other:?}; see the crate docs' known gaps"
                    ),
                };
                // A comparison only *reads* its operands, so a refcounted one
                // that no durable slot owns — a string literal in
                // `$key === "bad"` is the shape this exists for — is released
                // right after the instruction reads it, exactly the rule the
                // `Concat` arm above applies to its own fresh operands.
                let result = self.emit(
                    cur,
                    ty,
                    InstKind::BinOp {
                        op: bop,
                        lhs: lv,
                        rhs: rv,
                    },
                );
                if lty.is_refcounted() {
                    if !is_aliasing_read(&lhs.kind) {
                        self.emit_release(cur, lv);
                    }
                    if !is_aliasing_read(&rhs.kind) {
                        self.emit_release(cur, rv);
                    }
                }
                result
            }
            // `new Target(...)` — the constructed class and its resolved
            // constructor (if any) come from `self.exprs`, not from `target`
            // itself: `target` may be `self`/`static`/`parent`, which this
            // crate has no enclosing-class context to resolve on its own
            // (see `lower_decl_type`'s doc comment).
            ExprKind::New { args, .. } => {
                let Some(ExprInfo::New { class, ctor, .. }) = self.exprs.lookup(expr.span) else {
                    panic!(
                        "mwl-ir: `new` at {:?} has no resolved class recorded in the \
                         typed-expression table — did this program pass \
                         mwl_types::check_program with the same table?",
                        expr.span
                    );
                };
                let target_label = class.to_string();
                // The declaring class, not the constructed one: `new Dog(...)`
                // on a `Dog` with no `constructor` of its own invokes
                // `Animal::constructor`. Only `mwl_types` resolved that, so
                // the label is carried rather than re-derived downstream.
                let ctor_label = ctor
                    .as_ref()
                    .map(|call| format!("{}::{}", call.class, call.method));
                let arg_values = match ctor {
                    Some(call) => {
                        let param_tys = call.param_tys.clone();
                        let variadic = call.variadic;
                        let checked_types = self.checked_types;
                        self.lower_call_args(
                            args,
                            &param_tys,
                            variadic,
                            checked_types,
                            ArgOwnership::Transferred,
                            env,
                            cur,
                        )
                    }
                    None => {
                        let CallArgs::List(list) = args else {
                            panic!(
                                "mwl-ir: `new {target_label}(...)` has no resolved constructor \
                                 but wasn't called with a plain argument list — {args:?}"
                            );
                        };
                        assert!(
                            list.is_empty(),
                            "mwl-ir: `new {target_label}(...)` has no resolved constructor but \
                             was called with arguments — mwl_types doesn't yet enforce a \
                             zero-arity check here (see its own known gaps), so this crate \
                             cannot trust it was rejected upstream"
                        );
                        Vec::new()
                    }
                };
                self.emit_fallible(
                    cur,
                    Ty::Object,
                    InstKind::New {
                        class: target_label,
                        ctor: ctor_label,
                        args: arg_values,
                    },
                    env,
                )
            }
            // `$obj->method(...)`/`$this->method(...)` — the receiver is
            // lowered like any other expression (for `$this`, that's just an
            // `Env` lookup, since `lower_method` already seeded it as the
            // implicit parameter 0); the resolved target itself still comes
            // from `self.exprs`, exactly like a static call/`new` below.
            ExprKind::MethodCall {
                object,
                method: _,
                nullsafe,
                args,
            } => {
                assert!(
                    !*nullsafe,
                    "mwl-ir does not yet lower a nullsafe method call (`?->`); see the crate \
                     docs' known gaps"
                );
                let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
                    panic!(
                        "mwl-ir: an instance method call at {:?} has no resolved target \
                         recorded in the typed-expression table — did this program pass \
                         mwl_types::check_program with the same table?",
                        expr.span
                    );
                };
                let target_label = format!("{}::{}", call.class, call.method);
                let param_tys = call.param_tys.clone();
                let variadic = call.variadic;
                let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
                let checked_types = self.checked_types;
                let (receiver_v, receiver_ty) = self.lower_expr(object, None, env, cur);
                // The receiver is parameter 0, so it is an ordinary argument
                // for ownership purposes: MWL's convention is that the caller
                // retains an aliasing argument and the callee releases every
                // refcounted parameter at scope exit (see
                // `Self::release_all_locals`). `$this->m()` and `$obj->m()`
                // both read an existing slot, so both need the retain
                // `Self::lower_call_args` already inserts for one.
                if receiver_ty.is_refcounted() && is_aliasing_read(&object.kind) {
                    self.emit_retain(cur, receiver_v);
                }
                let arg_values = self.lower_call_args(
                    args,
                    &param_tys,
                    variadic,
                    checked_types,
                    ArgOwnership::Transferred,
                    env,
                    cur,
                );
                self.emit_fallible(
                    cur,
                    return_ty,
                    InstKind::Call {
                        target: target_label,
                        receiver: Some(receiver_v),
                        args: arg_values,
                    },
                    env,
                )
            }
            // `parent::method(...)`/`self::method(...)`/`Class::method(...)`.
            //
            // Written like a static call, but *not* necessarily one: PHP's
            // `parent::constructor(...)` and `self::helper()` invoke an
            // instance method on the enclosing `$this` whenever the resolved
            // target is not declared `static`. So the receiver is decided by
            // `ResolvedCall::is_static` rather than by the `::` in the source
            // — passing `null` to a method that reads `$this` would be a
            // null-pointer write into a field slot, not a diagnostic.
            ExprKind::StaticCall { args, .. } => {
                let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
                    panic!(
                        "mwl-ir: a static call at {:?} has no resolved target recorded in the \
                         typed-expression table — did this program pass \
                         mwl_types::check_program with the same table?",
                        expr.span
                    );
                };
                // A Tier 0 `Core` member is native Rust behind a helper
                // symbol, not a compiled MWL function, so it takes a
                // different instruction and a different argument-ownership
                // rule — see `InstKind::CoreCall`, which owns both. Resolved
                // through the identical `ResolvedCall` up to this point,
                // which is the whole reason `mwl_types` seeds a signature
                // table rather than special-casing `Core` at each call site.
                if let Some(symbol) = mwl_types::core_symbol_of(&call.class, &call.method) {
                    let param_tys = call.param_tys.clone();
                    let variadic = call.variadic;
                    let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
                    let checked_types = self.checked_types;
                    let arg_values = self.lower_call_args(
                        args,
                        &param_tys,
                        variadic,
                        checked_types,
                        ArgOwnership::Borrowed,
                        env,
                        cur,
                    );
                    return self.emit_fallible(
                        cur,
                        return_ty,
                        InstKind::CoreCall {
                            symbol,
                            args: arg_values,
                        },
                        env,
                    );
                }
                let target_label = format!("{}::{}", call.class, call.method);
                let param_tys = call.param_tys.clone();
                let variadic = call.variadic;
                let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
                let checked_types = self.checked_types;
                let receiver = if call.is_static {
                    None
                } else {
                    // The enclosing frame's own `$this`, retained the same way
                    // an explicit `$obj->m()` receiver is — the callee will
                    // release it. A file-scope frame has none, which the
                    // checker has already refused for a non-static target.
                    let &(this_v, this_ty) = env.get("this").unwrap_or_else(|| {
                        panic!(
                            "mwl-ir: `{target_label}` is not static but is reached from a frame \
                             with no `$this` — mwl_types is expected to have refused that"
                        )
                    });
                    if this_ty.is_refcounted() {
                        self.emit_retain(cur, this_v);
                    }
                    Some(this_v)
                };
                let arg_values = self.lower_call_args(
                    args,
                    &param_tys,
                    variadic,
                    checked_types,
                    ArgOwnership::Transferred,
                    env,
                    cur,
                );
                self.emit_fallible(
                    cur,
                    return_ty,
                    InstKind::Call {
                        target: target_label,
                        receiver,
                        args: arg_values,
                    },
                    env,
                )
            }
            // `$obj->prop` — the receiver's declaring class comes from
            // `self.exprs`, exactly like a call's resolved target; a shape or
            // plain-`object` receiver (ADR 0036 § 4) has no such entry at
            // all, so this panics naming that case rather than lowering it —
            // see the crate docs' known gaps for why (the checker itself
            // defers the runtime-checked fallback to M4, with no IR/codegen
            // yet to throw from).
            ExprKind::PropertyAccess {
                object, nullsafe, ..
            } => {
                assert!(
                    !*nullsafe,
                    "mwl-ir does not yet lower a nullsafe property access (`?->`); see the \
                     crate docs' known gaps"
                );
                let Some(ExprInfo::Property { class, name, ty }) = self.exprs.lookup(expr.span)
                else {
                    panic!(
                        "mwl-ir: a property access at {:?} has no resolved declaring class \
                         recorded in the typed-expression table — either it wasn't checked with \
                         the same table, or its receiver erased to a shape/plain `object` (ADR \
                         0036 § 4), which this crate does not yet lower (see the crate docs' \
                         known gaps)",
                        expr.span
                    );
                };
                let field_ty = lower_checked_ty(*ty, self.checked_types);
                let class_label = class.to_string();
                let field_name = name.clone();
                let (object_v, _) = self.lower_expr(object, None, env, cur);
                self.emit(
                    cur,
                    field_ty,
                    InstKind::FieldGet {
                        object: object_v,
                        class: class_label,
                        field: field_name,
                    },
                )
            }
            // `[...]`/legacy `array(...)` — see `InstKind::ArrayNew`'s own
            // doc comment for the full policy this mirrors and its known
            // gaps. `...spread` and `&value` elements are still unsupported
            // — each panics naming itself rather than guessing at a merge/
            // reference representation this crate doesn't have yet. A
            // *purely positional* literal (no element has an explicit
            // `key =>`) keeps the original single-`ArrayNew` shape: each
            // element's key is simply its index, auto-numbered from `0`
            // exactly like PHP's own `[$a, $b]` shorthand, computed at
            // lowering time with no runtime key instruction at all. A
            // literal with at least one explicit `key =>` element instead
            // builds an empty array first and appends one `ArraySet` per
            // element in source order — seeing `crate::ir::InstKind::ArrayNew`'s
            // own doc comment for why that's the only shape general enough
            // to give an explicit key's (possibly runtime-computed) value a
            // place to live, and the one PHP behavior it deliberately doesn't
            // reproduce (a positional element's key numbering ignores any
            // explicit `int`/`uint` key elsewhere in the same literal, rather
            // than PHP's real "continues from the highest int key used so
            // far"). Each value that's itself `Ty::is_refcounted` and
            // `is_aliasing_read` is retained before the array durably owns
            // it, the same policy `Self::lower_call_args` already applies at
            // a call-argument boundary; an explicit key gets the identical
            // treatment via `Self::lower_array_key`'s own aliasing flag. The
            // array literal's own result needs no retain — a fresh producer,
            // same as `new`/a call's result.
            ExprKind::ArrayLiteral(items) => {
                assert!(
                    items.iter().all(|item| !item.spread && !item.by_ref),
                    "mwl-ir does not yet lower a `...spread` or `&value` array-literal element \
                     — see the crate docs' known gaps"
                );
                if items.iter().all(|item| item.key.is_none()) {
                    let mut entries = Vec::with_capacity(items.len());
                    for (i, item) in items.iter().enumerate() {
                        let (v, ty) = self.lower_expr(&item.value, None, env, cur);
                        if ty.is_refcounted() && is_aliasing_read(&item.value.kind) {
                            self.emit_retain(cur, v);
                        }
                        entries.push((i.to_string(), v));
                    }
                    self.emit(cur, Ty::Array, InstKind::ArrayNew { entries })
                } else {
                    let array = self.emit(
                        cur,
                        Ty::Array,
                        InstKind::ArrayNew {
                            entries: Vec::new(),
                        },
                    );
                    let mut next_index = 0usize;
                    // Each write yields the array the next one writes into —
                    // the same pointer every time here, since a literal under
                    // construction is solely owned, but threaded rather than
                    // assumed so the one protocol has no exception.
                    let mut array_v = array.0;
                    for item in items {
                        let (key_v, key_aliasing) = match &item.key {
                            Some(key) => self.lower_array_key(key, env, cur),
                            None => {
                                let key_str = next_index.to_string();
                                next_index += 1;
                                let (kv, _) = self.emit(cur, Ty::Str, InstKind::ConstStr(key_str));
                                (kv, false)
                            }
                        };
                        if key_aliasing {
                            self.emit_retain(cur, key_v);
                        }
                        let (v, ty) = self.lower_expr(&item.value, None, env, cur);
                        if ty.is_refcounted() && is_aliasing_read(&item.value.kind) {
                            self.emit_retain(cur, v);
                        }
                        array_v = self.emit_array_set(cur, array_v, key_v, v);
                    }
                    (array_v, Ty::Array)
                }
            }
            // `$arr[$i]` — the element's declared type comes from
            // `self.exprs`, exactly like a property access's declaring
            // class: a base that erased to `mixed` (ADR 0007 § 5's own
            // "nothing compile-time-known to read" case for an unresolved
            // array) has no `ExprInfo::Index` entry at all, so this panics
            // naming that case rather than lowering it. `base[]` (`index`
            // is `None`) has no meaning as a read at all — it is PHP's
            // append syntax, assignment-target-only — so it panics too.
            ExprKind::Index { base, index } => {
                let Some(index) = index else {
                    panic!(
                        "mwl-ir does not lower `$a[]` as a read expression — append syntax \
                         (`index` is `None`) is assignment-target-only; see the crate docs' \
                         known gaps"
                    );
                };
                let Some(ExprInfo::Index { elem_ty }) = self.exprs.lookup(expr.span) else {
                    panic!(
                        "mwl-ir: an array-index read at {:?} has no resolved element type \
                         recorded in the typed-expression table — either it wasn't checked with \
                         the same table, or its base erased to `mixed` (an unresolved array), \
                         which this crate does not yet lower (see the crate docs' known gaps)",
                        expr.span
                    );
                };
                let result_ty = lower_checked_ty(*elem_ty, self.checked_types);
                let (array_v, _) = self.lower_expr(base, None, env, cur);
                let (key_v, key_aliasing) = self.lower_array_key(index, env, cur);
                let result = self.emit(
                    cur,
                    result_ty,
                    InstKind::ArrayGet {
                        array: array_v,
                        key: key_v,
                    },
                );
                if !key_aliasing {
                    self.emit_release(cur, key_v);
                }
                result
            }
            // `$x instanceof Name` — the tested class comes from
            // `self.exprs`, exactly like a property access's declaring class,
            // because resolving a bare `Animal` to `Ns\Animal` needs the
            // namespace/import context this crate cannot see. The dynamic
            // form (`$x instanceof $name`) records nothing and is refused.
            ExprKind::InstanceOf { expr: inner, .. } => {
                let Some(ExprInfo::InstanceOf { class }) = self.exprs.lookup(expr.span) else {
                    panic!(
                        "mwl-ir: an `instanceof` at {:?} has no resolved class recorded in the \
                         typed-expression table — either it wasn't checked with the same table, \
                         or its right-hand side is the dynamic `$x instanceof $name` form, which \
                         this crate does not lower (see the crate docs' known gaps)",
                        expr.span
                    );
                };
                let class_label = class.to_string();
                let (value, ty) = self.lower_expr(inner, None, env, cur);
                assert!(
                    matches!(ty, Ty::Object),
                    "mwl-ir lowers `instanceof` only against an object receiver — got \
                     representation {ty:?}"
                );
                self.emit(
                    cur,
                    Ty::Bool,
                    InstKind::InstanceOf {
                        value,
                        class: class_label,
                    },
                )
            }
            other => panic!(
                "mwl-ir's control-flow slice only lowers literals, locals, unary/binary \
                 operators, `new`, a static or instance method call, property access, an array \
                 literal, an array-element read, and `instanceof` — got {other:?}; see the crate \
                 docs' known gaps"
            ),
        }
    }

    /// `echo $a, $b;` — writes each operand's bytes to standard output in
    /// order, with no separator and no escaping: `.claude/loop-goal.md`
    /// records that ADR 0024 § 5's auto-escaping sink is the HTTP *response*
    /// write, not this one, and that whether `echo` under a future
    /// `mwl serve` becomes that sink is an M7 decision this does not
    /// pre-empt.
    ///
    /// Each operand is converted to [`Ty::Str`] by [`Self::concat_operand`]
    /// — the same shared path `.` concatenation already uses, so a scalar
    /// goes through its own [`Helper`] conversion and a `Stringable`-object
    /// operand panics naming the identical gap — and then handed to one
    /// [`Helper::EchoStr`] [`InstKind::HelperCall`] each. That call defines
    /// no value, so it is pushed with `result: None` rather than emitted
    /// through [`Self::emit`].
    ///
    /// An operand `concat_operand` reports as non-aliasing (a literal, a
    /// nested `Concat`'s own result, or a freshly converted `HelperCall`
    /// result) is released right after the write reads it, since nothing
    /// else ever will — the same "release a fresh value once its one and
    /// only use is done" policy the [`ExprKind::Binary`] concatenation arm
    /// already applies. No safepoint is emitted: `echo` is neither of the
    /// two reserved sites (function entry, a loop's back edge).
    fn lower_echo(&mut self, operands: &[Expr], cur: BlockId, env: &Env) {
        for operand in operands {
            let (v, aliasing) = self.concat_operand(operand, env, cur);
            // The one conversion-free helper that can genuinely fail: a write
            // to the request's output. See `Inst::on_error` for why the
            // scalar-to-string conversions around it carry no landing block.
            let landing = self.landing_block(env);
            self.block_insts[cur.index() as usize].push(Inst {
                result: None,
                ty: None,
                kind: InstKind::HelperCall {
                    helper: Helper::EchoStr,
                    args: vec![v],
                },
                on_error: Some(landing),
            });
            if !aliasing {
                self.emit_release(cur, v);
            }
        }
    }

    /// Lowers one `.` operand and, if it isn't already [`Ty::Str`], converts
    /// it through a new [`InstKind::HelperCall`] — `mwl_types::expr::
    /// check_expr`'s own `require_stringable` already accepts a scalar or a
    /// `Stringable`-implementing object on either side of `.` (PHP-style
    /// implicit stringification); this crate can express the scalar half
    /// today (see [`crate::ir::Helper`]) but a `Stringable` object still has
    /// no resolved `toString` call to synthesize here (that identity isn't
    /// recorded anywhere `.` itself can read — a call's own resolved target
    /// only exists for an actual call *expression*, and a bare `.` operand
    /// isn't one), so it panics naming the case rather than guessing.
    ///
    /// Returns the resulting `Ty::Str` value together with whether it
    /// [`is_aliasing_read`] of storage a durable slot still owns. A scalar
    /// conversion is never an aliasing read regardless of where the scalar
    /// itself came from — the `Ty::Str` `HelperCall` produces is always a
    /// brand new buffer with exactly one owner, the conversion result
    /// itself, same as a literal or a call's own result.
    fn concat_operand(&mut self, expr: &Expr, env: &Env, cur: BlockId) -> (ValueId, bool) {
        let (v, ty) = self.lower_expr(expr, None, env, cur);
        match ty {
            Ty::Str => (v, is_aliasing_read(&expr.kind)),
            Ty::Bool | Ty::Int | Ty::Uint | Ty::Float => {
                let helper = match ty {
                    Ty::Bool => Helper::BoolToString,
                    Ty::Int => Helper::IntToString,
                    Ty::Uint => Helper::UintToString,
                    Ty::Float => Helper::FloatToString,
                    Ty::Str | Ty::Bytes | Ty::Void | Ty::Object | Ty::Array | Ty::Mixed => {
                        unreachable!("matched above")
                    }
                };
                let (sv, _) = self.emit(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                );
                (sv, false)
            }
            other => panic!(
                "mwl-ir only converts a scalar operand to `string` for `.` so far — got \
                 {other:?}; a `Stringable`-object operand needs a resolved `toString` call this \
                 crate can't synthesize yet, see the crate docs' known gaps"
            ),
        }
    }

    /// Converts an already-lowered `(v, ty)` pair through ADR 0035's truthy
    /// table, with no ownership decision attached — see [`Self::truthy_value`]
    /// for the usual "release a fresh, non-aliasing refcounted operand once
    /// its truthy test is done" wrapper every caller but
    /// [`Self::lower_ternary`]'s elvis arm wants; elvis needs the bare
    /// conversion on its own, since its truthy-path *value* is `v` itself
    /// (PHP only evaluates a `?:` condition once) and releasing it here would
    /// use-after-free that reuse.
    ///
    /// `Ty::Bool` passes straight through; `Ty::Int`/`Ty::Uint`/`Ty::Float`/
    /// `Ty::Str` each convert through their own [`Helper`] variant
    /// (`IntTruthy`/`UintTruthy`/`FloatTruthy`/`StrTruthy`); [`Ty::Array`]
    /// converts through [`Helper::ArrayTruthy`] (falsy iff empty, ADR 0035's
    /// table); and [`Ty::Object`] — a class instance or an enum case — needs
    /// no helper at all, since ADR 0035 § 4 makes either always truthy: this
    /// folds straight to a fresh [`InstKind::ConstBool`] `true` rather than
    /// emitting a call with nothing to inspect at runtime.
    ///
    /// # Panics
    ///
    /// Panics naming the case for anything outside this table: `Ty::Bytes`
    /// (no truthy row is named for it — ADR 0035's table only covers
    /// `string`, not the separate `bytes` type) or `Ty::Void`. The `null`
    /// case (a nullable type) still has no IR representation to convert
    /// *from* at all, so it can't actually reach this method for any program
    /// in scope today. `Ty::Mixed` still panics too: converting one through
    /// ADR 0035's table needs a runtime type-tag representation this crate
    /// still doesn't have.
    fn truthy_convert(&mut self, v: ValueId, ty: Ty, cur: BlockId) -> ValueId {
        match ty {
            Ty::Bool => v,
            Ty::Int | Ty::Uint | Ty::Float | Ty::Str => {
                let helper = match ty {
                    Ty::Int => Helper::IntTruthy,
                    Ty::Uint => Helper::UintTruthy,
                    Ty::Float => Helper::FloatTruthy,
                    Ty::Str => Helper::StrTruthy,
                    Ty::Bool | Ty::Void | Ty::Object | Ty::Array | Ty::Bytes | Ty::Mixed => {
                        unreachable!("matched above")
                    }
                };
                self.emit(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                )
                .0
            }
            Ty::Array => {
                self.emit(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper: Helper::ArrayTruthy,
                        args: vec![v],
                    },
                )
                .0
            }
            // A class instance or an enum case — ADR 0035 § 4, always
            // truthy, nothing to inspect at runtime.
            Ty::Object => self.emit(cur, Ty::Bool, InstKind::ConstBool(true)).0,
            other => panic!(
                "mwl-ir's truthy-condition slice only converts a `bool`, a scalar, `Ty::Array` \
                 or `Ty::Object` value — got {other:?}; a `null` value has no IR representation \
                 to convert from at all, and a `mixed`/union value needs a runtime type-tag \
                 representation this crate doesn't have yet, see the crate docs' known gaps"
            ),
        }
    }

    /// [`Self::truthy_convert`] plus the ownership half every truthy-tested
    /// position but elvis wants: a refcounted operand (`Ty::Str`/`Ty::Array`)
    /// that isn't [`is_aliasing_read`] — a fresh call/`new`/literal result
    /// whose only use is this truthy test — is released right after it's
    /// read, the same "release a fresh value once its one and only use is
    /// done" precedent [`Self::concat_operand`]'s own caller already sets for
    /// `.` concatenation; an aliasing read (a bare variable, a
    /// compile-time-known property or array-element read) still durably
    /// belongs to whatever slot it came from and needs no release here.
    fn truthy_value(&mut self, v: ValueId, ty: Ty, is_alias: bool, cur: BlockId) -> ValueId {
        let cond_v = self.truthy_convert(v, ty, cur);
        if ty.is_refcounted() && !is_alias {
            self.emit_release(cur, v);
        }
        cond_v
    }

    /// Lowers `cond` — an `if`/`while` condition, or `&&`/`||`'s own operand
    /// (see [`Self::lower_and`]/[`Self::lower_or`]) — through
    /// [`Self::lower_expr_top`] (so a nested `&&`/`||`/`!`/ternary composes,
    /// e.g. `if ($a && $b)`) and then [`Self::truthy_value`]'s table.
    /// `*cur` is updated to whichever block `cond`'s own evaluation ends in —
    /// unchanged unless `cond` itself needed to branch.
    ///
    /// # Panics
    ///
    /// See [`Self::truthy_convert`]'s own panic doc — the same restriction
    /// applies here.
    fn lower_truthy_cond(&mut self, cond: &Expr, env: &Env, cur: &mut BlockId) -> ValueId {
        let (v, ty) = self.lower_expr_top(cond, None, env, cur);
        self.truthy_value(v, ty, is_aliasing_read(&cond.kind), *cur)
    }

    /// Lowers `expr` in a position that owns a mutable `cur` — a local
    /// declaration's initializer, `return`'s value, a plain reassignment's
    /// right-hand side, or a condition under test
    /// ([`Self::lower_truthy_cond`]) — and so can redirect it if `expr` needs
    /// control flow of its own: `&&`/`||` ([`Self::lower_and`]/
    /// [`Self::lower_or`], ADR 0035's short-circuit truthy positions), `!`
    /// ([`Self::lower_not`], which recurses through here for its own operand
    /// so `!($a && $b)` composes), or a ternary/elvis branch
    /// ([`Self::lower_ternary`]). PHP's `and`/`or`/`xor` keyword operators
    /// have no lowering here because they no longer exist in the AST at
    /// all — ADR 0045 rejects them at parse time.
    ///
    /// Everywhere else `lower_expr` is called directly instead — a call
    /// argument, an array-literal element, a `.`-operand, a nested
    /// arithmetic/comparison operand — still panics naming the gap if it
    /// contains one of these forms, since those callers only ever own a
    /// fixed `cur: BlockId`, not a `&mut BlockId` they could redirect after a
    /// branch; see the crate docs' known gaps.
    fn lower_expr_top(
        &mut self,
        expr: &Expr,
        expected: Option<Ty>,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        match &expr.kind {
            // See `Self::lower_expr`'s own `ExprKind::Paren` arm — same
            // transparent unwrap, just recursing back through this method
            // instead so a parenthesized `&&`/`||`/`!`/ternary still composes
            // (e.g. `!($a && $b)`).
            ExprKind::Paren(inner) => self.lower_expr_top(inner, expected, env, cur),
            ExprKind::Binary {
                op: BinaryOp::And,
                lhs,
                rhs,
            } => (self.lower_and(lhs, rhs, env, cur), Ty::Bool),
            ExprKind::Binary {
                op: BinaryOp::Or,
                lhs,
                rhs,
            } => (self.lower_or(lhs, rhs, env, cur), Ty::Bool),
            ExprKind::Unary {
                op: AstUnaryOp::Not,
                expr: inner,
            } => (self.lower_not(inner, env, cur), Ty::Bool),
            ExprKind::Ternary { cond, then, else_ } => {
                self.lower_ternary(cond, then.as_deref(), else_, env, cur)
            }
            _ => self.lower_expr(expr, expected, env, *cur),
        }
    }

    /// `!expr` — ADR 0035's truthy table applied to `expr`, then negated;
    /// always produces [`Ty::Bool`] regardless of `expr`'s own type, unlike a
    /// plain arithmetic/bitwise unary operator. `expr` is lowered through
    /// [`Self::lower_expr_top`] so `!($a && $b)`/`!($a ? $b : $c)` compose the
    /// same way a bare `&&`/`||`/ternary does at a top-level position.
    fn lower_not(&mut self, inner: &Expr, env: &Env, cur: &mut BlockId) -> ValueId {
        let (v, ty) = self.lower_expr_top(inner, None, env, cur);
        self.negate_truthy(v, ty, is_aliasing_read(&inner.kind), *cur)
    }

    /// Shared by [`Self::lower_not`] (a top-level `!`, whose operand may
    /// itself branch) and [`Self::lower_expr`]'s own `!` arm (a nested `!`
    /// with no `&mut BlockId` to redirect, so its operand may not branch):
    /// [`Self::truthy_value`]'s conversion, then an [`InstKind::UnOp`]
    /// negating the resulting [`Ty::Bool`].
    fn negate_truthy(&mut self, v: ValueId, ty: Ty, is_alias: bool, cur: BlockId) -> ValueId {
        let b = self.truthy_value(v, ty, is_alias, cur);
        self.emit(
            cur,
            Ty::Bool,
            InstKind::UnOp {
                op: UnOp::Not,
                operand: b,
            },
        )
        .0
    }

    /// `lhs && rhs` — PHP's short-circuit `&&`: `rhs` is only evaluated when
    /// `lhs` is truthy. Lowered exactly like [`Self::lower_if`]'s own
    /// branch/merge shape, except the join point produces the expression's
    /// own [`Ty::Bool`] value via a fresh [`InstKind::Phi`] instead of
    /// merging named locals (an expression's own temporaries never live in
    /// [`Env`] — that's [`Self::merge_envs`]' business, not this one's).
    /// `lhs`/`rhs` each go through [`Self::lower_truthy_cond`], so either may
    /// itself be any type ADR 0035's table covers, and either may itself be a
    /// nested `&&`/`||`/`!`/ternary.
    fn lower_and(&mut self, lhs: &Expr, rhs: &Expr, env: &Env, cur: &mut BlockId) -> ValueId {
        let lhs_v = self.lower_truthy_cond(lhs, env, cur);
        let lhs_end = *cur;
        // Emitted in `lhs_end` before it's sealed below — this is the join's
        // incoming value for the short-circuit (falsy-`lhs`) edge.
        let short_v = self.emit(lhs_end, Ty::Bool, InstKind::ConstBool(false)).0;

        let rhs_block = self.new_block();
        let merge_block = self.new_block();
        let rhs_edge = self.ids.next_edge(rhs.span);
        let short_edge = self.ids.next_edge(lhs.span);
        self.seal(
            lhs_end,
            Terminator::Branch {
                cond: lhs_v,
                then_block: rhs_block,
                then_edge: rhs_edge,
                else_block: merge_block,
                else_edge: short_edge,
            },
        );

        let mut rhs_cur = rhs_block;
        let rhs_v = self.lower_truthy_cond(rhs, env, &mut rhs_cur);
        let rhs_end = rhs_cur;
        self.seal(rhs_end, Terminator::Jump(merge_block));

        let (result, _) = self.emit(
            merge_block,
            Ty::Bool,
            InstKind::Phi {
                incoming: vec![(lhs_end, short_v), (rhs_end, rhs_v)],
            },
        );
        *cur = merge_block;
        result
    }

    /// `lhs || rhs` — [`Self::lower_and`]'s mirror: `rhs` is only evaluated
    /// when `lhs` is falsy, and the short-circuit (truthy-`lhs`) edge carries
    /// `true` instead of `false`.
    fn lower_or(&mut self, lhs: &Expr, rhs: &Expr, env: &Env, cur: &mut BlockId) -> ValueId {
        let lhs_v = self.lower_truthy_cond(lhs, env, cur);
        let lhs_end = *cur;
        let short_v = self.emit(lhs_end, Ty::Bool, InstKind::ConstBool(true)).0;

        let rhs_block = self.new_block();
        let merge_block = self.new_block();
        let short_edge = self.ids.next_edge(lhs.span);
        let rhs_edge = self.ids.next_edge(rhs.span);
        self.seal(
            lhs_end,
            Terminator::Branch {
                cond: lhs_v,
                then_block: merge_block,
                then_edge: short_edge,
                else_block: rhs_block,
                else_edge: rhs_edge,
            },
        );

        let mut rhs_cur = rhs_block;
        let rhs_v = self.lower_truthy_cond(rhs, env, &mut rhs_cur);
        let rhs_end = rhs_cur;
        self.seal(rhs_end, Terminator::Jump(merge_block));

        let (result, _) = self.emit(
            merge_block,
            Ty::Bool,
            InstKind::Phi {
                incoming: vec![(lhs_end, short_v), (rhs_end, rhs_v)],
            },
        );
        *cur = merge_block;
        result
    }

    /// `cond ? then : else` (`then` is `None` for elvis, `cond ?: else`) —
    /// [`Self::lower_if`]'s branch/merge shape again, this time joining the
    /// expression's own value via a [`InstKind::Phi`] rather than merging
    /// named locals.
    ///
    /// `cond` is converted through [`Self::truthy_convert`] directly, not
    /// [`Self::lower_truthy_cond`]/[`Self::truthy_value`]: elvis's truthy
    /// path reuses `cond`'s own value (PHP evaluates a `?:` condition exactly
    /// once), so releasing it as part of the truthy test — the usual rule
    /// every other truthy-tested position wants — would use-after-free that
    /// reuse. Instead: a refcounted, non-aliasing `cond` is released once
    /// `then` is given (nothing left to reuse it for), or retained once more
    /// when `then` is omitted and `cond` *is* an aliasing read (its value is
    /// about to gain a second, independent owner — the ternary's own
    /// result) — the same [`is_aliasing_read`]-keyed retain
    /// [`Self::bind_local`]/[`Self::lower_call_args`] already apply at their
    /// own ownership-transfer boundaries. A fresh, non-aliasing `cond`
    /// reused by elvis needs neither: it already has exactly one owner,
    /// which simply becomes the ternary's result.
    ///
    /// The same retain question applies to every `then`/`else` branch, not
    /// just elvis's reused `cond`: every consumer of this method's own result
    /// ([`Self::bind_local`], `return`) treats it as an ordinary fresh value —
    /// [`is_aliasing_read`] never lists [`mwl_syntax::ast::ExprKind::Ternary`]
    /// — so this method has to guarantee that itself. A branch whose own
    /// expression [`is_aliasing_read`] (a bare variable, a compile-time-known
    /// property or array-element read) is retained right there, converting a
    /// still-slot-owned reference into the ternary's own independent one,
    /// exactly like [`Self::lower_interpolated_parts`]' own "single-part
    /// alias" case; a branch that's already fresh (a literal, `new`, a call
    /// result, or a nested `&&`/`||`/`!`/ternary — the last already guarantees
    /// its own freshness by this same rule) needs no retain, since ownership
    /// just transfers.
    ///
    /// # Panics
    ///
    /// Panics naming the case if `then`'s and `else`'s branches lower to two
    /// different [`Ty`] representations — the checker's own union of their
    /// static types has no IR representation this crate can fold into yet
    /// (see [`Ty::Mixed`]'s own doc comment on why a union isn't folded into
    /// it automatically). Otherwise see [`Self::truthy_convert`]'s own panic
    /// doc for `cond`'s own restriction.
    fn lower_ternary(
        &mut self,
        cond: &Expr,
        then: Option<&Expr>,
        else_: &Expr,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (cond_v, cond_ty) = self.lower_expr_top(cond, None, env, cur);
        let cond_is_alias = is_aliasing_read(&cond.kind);
        let truthy_v = self.truthy_convert(cond_v, cond_ty, *cur);
        let pre_block = *cur;
        if then.is_some() && cond_ty.is_refcounted() && !cond_is_alias {
            self.emit_release(pre_block, cond_v);
        }

        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();
        let then_edge = self.ids.next_edge(then.map_or(cond.span, |t| t.span));
        let else_edge = self.ids.next_edge(else_.span);
        self.seal(
            pre_block,
            Terminator::Branch {
                cond: truthy_v,
                then_block,
                then_edge,
                else_block,
                else_edge,
            },
        );

        let (then_v, then_ty, then_end) = match then {
            Some(then_expr) => {
                let mut then_cur = then_block;
                let (v, ty) = self.lower_expr_top(then_expr, None, env, &mut then_cur);
                if ty.is_refcounted() && is_aliasing_read(&then_expr.kind) {
                    self.emit_retain(then_cur, v);
                }
                (v, ty, then_cur)
            }
            None => {
                if cond_ty.is_refcounted() && cond_is_alias {
                    self.emit_retain(then_block, cond_v);
                }
                (cond_v, cond_ty, then_block)
            }
        };
        self.seal(then_end, Terminator::Jump(merge_block));

        let mut else_cur = else_block;
        let (else_v, else_ty) = self.lower_expr_top(else_, None, env, &mut else_cur);
        if else_ty.is_refcounted() && is_aliasing_read(&else_.kind) {
            self.emit_retain(else_cur, else_v);
        }
        self.seal(else_cur, Terminator::Jump(merge_block));

        assert_eq!(
            then_ty, else_ty,
            "mwl-ir's ternary/elvis slice only lowers a ternary whose branches share the same \
             IR-level type — got {then_ty:?} vs {else_ty:?}; a differing-branch-type ternary \
             erases to a union the checker already computed but this crate has no IR \
             representation to fold it into yet, see the crate docs' known gaps"
        );

        let (result, _) = self.emit(
            merge_block,
            then_ty,
            InstKind::Phi {
                incoming: vec![(then_end, then_v), (else_cur, else_v)],
            },
        );
        *cur = merge_block;
        (result, then_ty)
    }

    /// Lowers `ExprKind::Interpolated`'s parts into the single [`Ty::Str`]
    /// value they denote — a left-to-right fold of [`InstKind::Concat`],
    /// reusing [`Self::concat_operand`] per `StringPart::Expr` piece exactly
    /// the way `.`-concatenation's own two-operand arm does (a
    /// `Stringable`-object piece hits the identical "needs a resolved
    /// `toString`" panic that method's own doc comment already names as a
    /// shared, not-yet-lowerable case — this is not primarily a new gap, just
    /// the same one reached from a second syntax). A `StringPart::Text` piece
    /// cooks straight to a fresh [`InstKind::ConstStr`] via
    /// [`mwl_types::string_lit::cook_double_quoted_text`] — the same routine
    /// [`cook_str_literal`] delegates to for a plain double-quoted `Str`,
    /// since a `Text` run's escape grammar is identical either way (see that
    /// function's own doc comment) — unless `whole_span` opens with `<<<`
    /// (a heredoc; never a nowdoc, see this function's caller), in which
    /// case each `Text` run first goes through
    /// [`mwl_types::string_lit::dedent_heredoc_run`] against the one
    /// [`mwl_types::string_lit::heredoc_shape`] computed for the whole
    /// literal, exactly the way [`cook_heredoc_str`] dedents a `Str`-collapsed
    /// heredoc's own single run — `body_start` is true only for `parts`'
    /// own first entry, and `is_last_run` only for the last `StringPart::Text`
    /// entry (never an `Expr`: the body always ends in literal text, at
    /// minimum the one newline before the closing marker).
    ///
    /// The one shape plain N-ary `.`-folding wouldn't otherwise force into
    /// the open: an `Interpolated` with exactly one part that is itself an
    /// aliasing read (`"$x"` alone, no literal text around it and nothing
    /// else to concatenate against) never emits an `InstKind::Concat` at
    /// all, so nothing along the way copies `$x`'s value into a fresh
    /// buffer. Returning `$x`'s own `ValueId` unchanged would hand the
    /// caller a second durable owner of a slot's existing storage with no
    /// retain behind it — exactly the free-turns-into-a-dangling-reference
    /// bug [`Self::bind_local`]'s own retain-on-aliasing-source rule exists
    /// to avoid. `is_aliasing_read` deliberately does not list
    /// `ExprKind::Interpolated` at all (mirroring `ExprKind::Binary { op:
    /// Concat, .. }`, which never lists it either, since two or more parts
    /// always produce a real `Concat`'s fresh buffer) — so this function,
    /// not `Self::bind_local`, is the one place that single-part degenerate
    /// case has to convert a borrowed reference into an owned one, by
    /// retaining it directly before handing it back as though it were as
    /// fresh as every other shape this function can return.
    fn lower_interpolated_parts(
        &mut self,
        parts: &[StringPart],
        whole_span: mwl_diagnostics::Span,
        env: &Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            !parts.is_empty(),
            "mwl-syntax's collapse_string_parts only ever produces ExprKind::Interpolated for a \
             non-empty parts vec"
        );
        let is_heredoc = span_text(self.src, whole_span).starts_with("<<<");
        let indent = if is_heredoc {
            mwl_types::string_lit::heredoc_shape(self.src, whole_span)
                .0
                .indent
        } else {
            String::new()
        };
        let last_text_idx = is_heredoc
            .then(|| parts.iter().rposition(|p| matches!(p, StringPart::Text(_))))
            .flatten();
        let mut acc: Option<(ValueId, bool)> = None;
        for (i, part) in parts.iter().enumerate() {
            let piece = match part {
                StringPart::Text(span) => {
                    let s = if is_heredoc {
                        let mut issues = Vec::new(); // discarded: mwl_types::check_program already reported these
                        let dedented = mwl_types::string_lit::dedent_heredoc_run(
                            self.src,
                            &indent,
                            *span,
                            i == 0,
                            Some(i) == last_text_idx,
                            &mut issues,
                        );
                        mwl_types::string_lit::cook_double_quoted_text_str(&dedented, *span).0
                    } else {
                        mwl_types::string_lit::cook_double_quoted_text(self.src, *span).0
                    };
                    (self.emit(cur, Ty::Str, InstKind::ConstStr(s)).0, false)
                }
                StringPart::Expr(e) => self.concat_operand(e, env, cur),
            };
            acc = Some(match acc {
                None => piece,
                Some((lv, l_alias)) => {
                    let (rv, r_alias) = piece;
                    let (result, _) =
                        self.emit(cur, Ty::Str, InstKind::Concat { lhs: lv, rhs: rv });
                    if !l_alias {
                        self.emit_release(cur, lv);
                    }
                    if !r_alias {
                        self.emit_release(cur, rv);
                    }
                    (result, false)
                }
            });
        }
        let (v, alias) = acc.expect("checked non-empty above");
        if alias {
            // The single-part-alias degenerate case this function's own doc
            // comment names — no `Concat` ran, so `v` is still someone
            // else's storage; retain it to become this expression's own
            // single fresh owner.
            self.emit_retain(cur, v);
        }
        (v, Ty::Str)
    }

    /// Lowers `expr` — an `ExprKind::Index`'s subscript — and normalizes it
    /// to a [`Ty::Str`] key: ADR 0007 § 5's "every key is a `string`" rule,
    /// with an `int`/`uint` subscript normalized to its decimal-string form
    /// (`$a[8]` is `$a["8"]`) via the exact [`Helper::IntToString`]/
    /// [`Helper::UintToString`] conversion [`Self::concat_operand`] already
    /// uses for `.`'s scalar operand — reused verbatim rather than a new
    /// policy. Also used, identically, for an array literal's explicit
    /// `key =>` element (see [`ir::InstKind::ArrayNew`]'s own doc comment). A
    /// `float`, `bool`, or `null` key is a compile-time rejection
    /// `mwl_types::expr::check_array_key_type` now enforces at both call
    /// sites (an `Index` subscript and an array-literal explicit key alike),
    /// so the `other` arm below is an internal-invariant panic — unreachable
    /// for anything that already passed `mwl_types::check_program` — rather
    /// than a live known gap.
    ///
    /// Returns the resulting `Ty::Str` value together with whether it
    /// [`is_aliasing_read`]s storage a durable slot still owns — exactly the
    /// same second half [`Self::concat_operand`] returns, for the same
    /// reason: a plain `string` subscript passed through unchanged may still
    /// be a bare local/property/array read, while a freshly converted
    /// `int`/`uint` key is always a brand new buffer with exactly one owner.
    fn lower_array_key(&mut self, expr: &Expr, env: &Env, cur: BlockId) -> (ValueId, bool) {
        let (v, ty) = self.lower_expr(expr, None, env, cur);
        match ty {
            Ty::Str => (v, is_aliasing_read(&expr.kind)),
            Ty::Int | Ty::Uint => {
                let helper = if ty == Ty::Int {
                    Helper::IntToString
                } else {
                    Helper::UintToString
                };
                let (sv, _) = self.emit(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                );
                (sv, false)
            }
            other => panic!(
                "mwl-ir: an array key lowered to {other:?} — mwl_types::check_program is trusted \
                 to have already rejected a float/bool/null key (ADR 0007 § 5) at both the \
                 subscript and array-literal explicit-key sites, so this should be unreachable"
            ),
        }
    }

    /// Lowers a resolved call's/`new`'s positional argument list against
    /// `param_tys` — the already-resolved parameter types from
    /// `mwl_types::expr_table::ResolvedCall`. An argument whose expected type
    /// [`Ty::is_refcounted`] and whose source expression [`is_aliasing_read`]
    /// (a bare variable or a compile-time-known property read) is retained
    /// before the call — the callee's own parameter is bound into its `Env`
    /// exactly like a local (see [`lower_method`]) and released at its own
    /// exit by [`Lowering::release_all_locals`], so this retain is the
    /// caller-side half of a balanced pair, symmetric with what
    /// [`Lowering::bind_local`] already does for a local declaration. A
    /// fresh literal, `new`, or a call's own result passed directly as an
    /// argument needs no retain: it already has exactly one owner, which
    /// simply transfers into the callee's parameter slot.
    ///
    /// # Panics
    ///
    /// Panics naming the unsupported shape for anything outside this slice's
    /// scope: `variadic`, a named or spread argument (`mwl_types` itself
    /// doesn't fully positionally type-check these against a signature yet —
    /// see its own known gaps), or an argument count that doesn't exactly
    /// match `param_tys`' length (this crate trusts
    /// `mwl_types::check_program` already enforced arity for a non-variadic
    /// signature).
    #[expect(
        clippy::too_many_arguments,
        reason = "one resolved signature's three fields plus the interner they \
                  are read against, the ownership rule, and the `(env, cur)` \
                  pair every lowering method threads"
    )]
    fn lower_call_args(
        &mut self,
        args: &CallArgs,
        param_tys: &[TypeId],
        variadic: bool,
        checked_types: &TypeInterner,
        ownership: ArgOwnership,
        env: &Env,
        cur: BlockId,
    ) -> Vec<ValueId> {
        assert!(
            !variadic,
            "mwl-ir does not yet lower a call to a variadic signature; see the crate docs' \
             known gaps"
        );
        let CallArgs::List(list) = args else {
            panic!(
                "mwl-ir only lowers a plain positional argument list for a resolved call/`new` \
                 — got {args:?}; see the crate docs' known gaps"
            );
        };
        assert!(
            list.iter().all(|a| a.name.is_none() && !a.spread),
            "mwl-ir does not yet lower a named or spread call argument; see the crate docs' \
             known gaps"
        );
        assert_eq!(
            list.len(),
            param_tys.len(),
            "mwl-ir: a resolved call's argument count doesn't match its signature — this crate \
             trusts mwl_types::check_program already enforced this"
        );
        let mut out = Vec::with_capacity(list.len());
        for (arg, &pty) in list.iter().zip(param_tys) {
            let expected = lower_checked_ty(pty, checked_types);
            let (v, ty) = self.lower_expr(&arg.value, Some(expected), env, cur);
            if ownership == ArgOwnership::Transferred
                && ty.is_refcounted()
                && is_aliasing_read(&arg.value.kind)
            {
                self.emit_retain(cur, v);
            }
            out.push(v);
        }
        out
    }
}

/// Reads a numeric-literal span's text with `_` digit separators stripped.
fn clean_digits(src: &SourceFile, span: mwl_diagnostics::Span) -> String {
    span_text(src, span).chars().filter(|&c| c != '_').collect()
}

/// Cooks a plain, non-interpolated string literal's span — `mwl_syntax::ast::ExprKind::Str`'s
/// own doc comment: a single-quoted string, or a double-quoted/heredoc/nowdoc string with no
/// interpolation in it — into its runtime bytes.
///
/// A single-quoted literal only ever needs the two escapes `mwl-syntax`'s lexer recognizes there
/// (`\\` and `\'` — see `Lexer::lex_single_quoted`'s own
/// `single_quoted_string_only_escapes_backslash_and_quote` test), cooked inline below since
/// there is no invalid-UTF-8 case to guard against (both escapes are ASCII, and every other
/// character copies straight through from a source file that is already valid UTF-8) — nothing
/// worth sharing a routine for. A double-quoted literal instead delegates its whole inner span to
/// [`mwl_types::string_lit::cook_double_quoted_text`] — the full escape grammar (named escapes,
/// octal/hex byte escapes, `\u{...}` codepoints) plus its two failure modes (an out-of-range
/// `\u{...}`, or byte escapes that don't assemble into valid UTF-8) live there now, not here, so
/// `mwl_types::expr::infer`'s own `ExprKind::Str` arm can diagnose exactly the same cooking this
/// function performs — see that module's own docs for why the routine is shared rather than
/// duplicated the way [`int_literal_digits`] is. This function discards the returned issues:
/// `mwl_types::check_program` already reported them, the same "checker diagnoses, `mwl-ir` trusts"
/// split every other panic in this crate relies on.
///
/// A heredoc/nowdoc-sourced `Str` (a body with no interpolation site used at all — its span opens
/// with `<`, not a quote) instead delegates to [`cook_heredoc_str`]: PHP 7.3's "flexible heredoc"
/// indentation strip
/// ([`mwl_types::string_lit::heredoc_shape`]/[`mwl_types::string_lit::dedent_heredoc_run`]) runs
/// first, then the same double-quoted escape grammar as above — unless it's a nowdoc
/// (`mwl_types::string_lit::heredoc_is_nowdoc`), which applies no escapes at all, exactly like a
/// single-quoted literal minus even `\\`/`\'`.
fn cook_str_literal(src: &SourceFile, span: mwl_diagnostics::Span) -> String {
    let raw = span_text(src, span);
    if raw.starts_with("<<<") {
        return cook_heredoc_str(src, span, raw);
    }
    let quote = raw
        .chars()
        .next()
        .unwrap_or_else(|| panic!("mwl-ir: an empty string literal span at {span:?} — lexer bug?"));
    assert!(
        quote == '\'' || quote == '"',
        "mwl-ir only cooks a single-quoted, double-quoted or heredoc/nowdoc string literal — got \
         {raw:?}; see the crate docs' known gaps"
    );
    let inner_span = mwl_diagnostics::Span::new(span.file, span.start + 1, span.end - 1);
    if quote == '"' {
        return mwl_types::string_lit::cook_double_quoted_text(src, inner_span).0;
    }
    let inner = span_text(src, inner_span);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some(next) if next == quote => out.push(quote),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Cooks a heredoc/nowdoc literal collapsed to `ExprKind::Str` (no interpolation site anywhere in
/// its body) — `cook_str_literal`'s own doc comment for the shape this covers. `raw` is `span`'s
/// own text, already confirmed to start with `<<<` by the caller. Trusts
/// `mwl_types::check_program` already reported any `HeredocIndentIssue` this literal has, the same
/// way [`cook_str_literal`]'s own double-quoted branch trusts `CookIssue`s were already reported —
/// this function discards both.
fn cook_heredoc_str(src: &SourceFile, span: mwl_diagnostics::Span, raw: &str) -> String {
    let (shape, _issues) = mwl_types::string_lit::heredoc_shape(src, span);
    let mut issues = Vec::new();
    let dedented = mwl_types::string_lit::dedent_heredoc_run(
        src,
        &shape.indent,
        shape.body,
        true,
        true,
        &mut issues,
    );
    if mwl_types::string_lit::heredoc_is_nowdoc(raw) {
        dedented
    } else {
        mwl_types::string_lit::cook_double_quoted_text_str(&dedented, span).0
    }
}

/// Splits a cooked integer-literal span into the radix its prefix names and
/// the digit run to parse against it — `mwl_syntax::Lexer::lex_number` emits
/// one `IntLiteral` token for all four forms (`0x…`/`0o…`/`0b…`, or a plain
/// decimal run; a legacy leading-zero octal spelling like PHP's `0755` is
/// deliberately *not* one of them, so `0755` lexes as decimal 755 with no
/// prefix to strip), and this is the one place that distinction has to be
/// undone before `str::from_str_radix` can parse the value. `mwl_types::expr::infer`'s own
/// `ExprKind::Int` arm now range-checks the same digits (mirroring this function to do so, since
/// this crate has no reverse dependency on that one) and reports ADR 0007 § 4's diagnostic before
/// lowering ever runs — see that arm's doc comment — so `lower_expr`'s `ExprKind::Int` arm can
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
fn binding_ty(binding: &ForeachBinding, which: &str) -> Ty {
    let ty = binding.ty.as_ref().unwrap_or_else(|| {
        panic!(
            "mwl-ir: a `foreach` {which} binding reached lowering with no declared type — \
             mwl_syntax already reports that omission, so this file should never have been \
             lowered"
        )
    });
    lower_decl_type(ty)
}

/// Lowers a *declared* type straight off the AST — every scalar atom, plus
/// `TypeAtom::Name(_)` (a plain class/interface/enum name) as
/// [`Ty::Object`], `TypeAtom::Array(_)` (bare `array` or `array<T>`) as
/// [`Ty::Array`], and `TypeAtom::Mixed` as [`Ty::Mixed`]. A plain name needs
/// no resolution to lower this way: ADR 0007 § 1 already requires it to be
/// spelled out in full, and this crate erases class identity entirely (see
/// [`Ty::Object`]'s own doc comment), so "is this atom a class name at all"
/// is the only question that matters here — which class doesn't need
/// answering until a call/`new` on it does, via [`lower_checked_ty`] instead.
/// `array<T>`'s own type argument is discarded the same way — [`Ty::Array`]'s
/// own doc comment explains why no lowering decision needs it at this level.
/// `self`/`static`/`parent` are not handled: resolving those needs the
/// enclosing class, which this crate's straight-off-the-AST design (see the
/// crate docs) has never needed to track before now.
fn lower_decl_type(ty: &Type) -> Ty {
    match &ty.kind {
        TypeKind::Atom(TypeAtom::Bool) => Ty::Bool,
        TypeKind::Atom(TypeAtom::Int) => Ty::Int,
        TypeKind::Atom(TypeAtom::Uint) => Ty::Uint,
        TypeKind::Atom(TypeAtom::Float) => Ty::Float,
        TypeKind::Atom(TypeAtom::Void) => Ty::Void,
        TypeKind::Atom(TypeAtom::String) => Ty::Str,
        TypeKind::Atom(TypeAtom::Bytes) => Ty::Bytes,
        TypeKind::Atom(TypeAtom::Name(_)) => Ty::Object,
        TypeKind::Atom(TypeAtom::Array(_)) => Ty::Array,
        // `mixed` — ADR 0007 § 3. See `Ty::Mixed`'s own doc comment for
        // exactly how much this representation does and doesn't do yet: a
        // local/parameter/return/call-argument round-trips, nothing else.
        TypeKind::Atom(TypeAtom::Mixed) => Ty::Mixed,
        TypeKind::Paren(inner) => lower_decl_type(inner),
        other => panic!(
            "mwl-ir only lowers bool/int/uint/float/void/string/bytes/array/a plain class name \
             as a declared type — got {other:?}; see the crate docs' known gaps"
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
/// `Class`/`Enum` both erase to [`Ty::Object`], same as [`lower_decl_type`]'s
/// `Name` case — see that variant's own doc comment for why identity doesn't
/// need to survive this translation. `String` erases to [`Ty::Str`] and
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
/// `object`, a shape, a union/intersection, or any of
/// `never`/`true`/`false`/`iterable`/`callable`/`null` — none of these
/// have an IR representation yet (see the crate docs' known gaps). `mixed`
/// erases to [`Ty::Mixed`] — see that variant's own doc comment for exactly
/// how much this boundary does and doesn't do with one yet.
fn lower_checked_ty(id: TypeId, checked_types: &TypeInterner) -> Ty {
    match checked_types.get(id) {
        CheckedTy::Bool => Ty::Bool,
        CheckedTy::Int => Ty::Int,
        CheckedTy::Uint => Ty::Uint,
        CheckedTy::Float => Ty::Float,
        CheckedTy::Void => Ty::Void,
        CheckedTy::String => Ty::Str,
        CheckedTy::Bytes => Ty::Bytes,
        CheckedTy::Class(_) | CheckedTy::Enum(_) => Ty::Object,
        // The element `TypeId` is discarded — same erasure `lower_decl_type`
        // already gives `TypeAtom::Array(_)`, see `Ty::Array`'s own doc
        // comment for why this crate has no lowering decision that needs it.
        CheckedTy::Array(_) => Ty::Array,
        CheckedTy::Mixed => Ty::Mixed,
        other => panic!(
            "mwl-ir only lowers a resolved call's bool/int/uint/float/void/string/bytes/array/\
             class/enum/mixed parameter or return type — got {other:?}; see the crate docs' \
             known gaps"
        ),
    }
}

/// Whether reading `kind` produces a *borrowed* reference to storage some
/// other binding still owns, rather than a freshly constructed value with
/// exactly one natural owner — the same "is this a copy or a fresh value"
/// judgment [`Lowering::bind_local`]'s own doc comment already describes for
/// a bare variable read, now shared with a call argument
/// ([`Lowering::lower_call_args`]) and a returned expression
/// (`StmtKind::Return`'s own arm). A plain local (`ExprKind::Variable`), a
/// compile-time-known property read (`ExprKind::PropertyAccess`), and a
/// compile-time-known array-element read (`ExprKind::Index`) all borrow
/// storage that keeps its own reference after this read — a local's own slot,
/// the object's field, or the array's own entry (ADR 0007 § 5's copy-on-write
/// value semantics) — so copying any of them into a new durable slot needs a
/// retain. A fresh literal, `new`, or a call's own result already has exactly
/// one natural owner and needs none.
/// The root exception class's label — the one `mwl_types::layout` keys its
/// four slots under, and the one every `FieldGet`/`FieldSet` on an exception
/// resolves through.
///
/// A slot resolved against the root is valid for every subclass
/// (`mwl_runtime::object` lays a subclass's slots after its parent's), so
/// lowering never has to know which exception class it actually holds. The
/// tree's own home is `mwl_hir::errors`; this crate depends on neither
/// `mwl-hir` nor `mwl-types`' name resolution, so it restates the one label it
/// needs — `mwl-codegen`'s
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot` holds the
/// three copies together.
const THROWABLE_ROOT: &str = "Throwable";

/// `Throwable::$message`.
const MESSAGE_FIELD: &str = "message";
/// `Throwable::$backtrace`.
const BACKTRACE_FIELD: &str = "backtrace";
/// `Throwable::$location`.
const LOCATION_FIELD: &str = "location";

/// The label the synthesized root constructor is compiled under — the target
/// a `new LogicError("…")` and a `parent::constructor(…)` in a user subclass
/// both resolve to.
const THROWABLE_CTOR: &str = "Throwable::constructor";

/// The one MWL function with no source text: `Throwable`'s constructor.
///
/// It cannot be written in MWL — `backtrace` is grown by the runtime as a
/// throw propagates, so a source declaration would need a body with no legal
/// spelling (`mwl_hir::errors` owns that reasoning). What it does is small
/// enough to build by hand: store the message, start an empty backtrace, and
/// put a placeholder in `location` that [`Lowering::write_throw_location`]
/// overwrites at the `throw`. `previous` is left `null`, which is the only
/// value it can have until `mwl_types::signatures::MethodSig` can model an
/// optional parameter (`mwl_types::error_lib`'s own known gaps).
///
/// The receiver and the message are both *transferred* to this frame by the
/// call convention, so both are released at the exit — the field takes its own
/// reference to the message first.
fn synthesized_throwable_constructor() -> Function {
    let mut ids = IdGen::default();
    let block = ids.next_block();
    let this = ids.next_value();
    let message = ids.next_value();
    let backtrace = ids.next_value();
    let location = ids.next_value();

    let plain = |kind: InstKind| Inst {
        result: None,
        ty: None,
        kind,
        on_error: None,
    };
    let defines = |result: ValueId, ty: Ty, kind: InstKind| Inst {
        result: Some(result),
        ty: Some(ty),
        kind,
        on_error: None,
    };
    let store = |field: &str, value: ValueId| {
        plain(InstKind::FieldSet {
            object: this,
            class: THROWABLE_ROOT.to_owned(),
            field: field.to_owned(),
            value,
        })
    };

    let insts = vec![
        plain(InstKind::Safepoint),
        defines(this, Ty::Object, InstKind::Param(0)),
        defines(message, Ty::Str, InstKind::Param(1)),
        plain(InstKind::Retain { operand: message }),
        store(MESSAGE_FIELD, message),
        defines(
            backtrace,
            Ty::Array,
            InstKind::ArrayNew {
                entries: Vec::new(),
            },
        ),
        store(BACKTRACE_FIELD, backtrace),
        defines(location, Ty::Str, InstKind::ConstStr(String::new())),
        store(LOCATION_FIELD, location),
        plain(InstKind::Release { operand: message }),
        plain(InstKind::Release { operand: this }),
    ];

    let (stmt_spans, edge_spans) = ids.into_spans();
    Function {
        name: THROWABLE_CTOR.to_owned(),
        params: vec![Ty::Object, Ty::Str],
        ret: Ty::Void,
        blocks: vec![BasicBlock {
            id: block,
            insts,
            term: Terminator::Return(None),
        }],
        entry: block,
        stmt_spans,
        edge_spans,
    }
}

fn is_aliasing_read(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Variable(_) | ExprKind::PropertyAccess { .. } | ExprKind::Index { .. }
    )
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use mwl_diagnostics::{Diagnostics, SourceId, SourceMap};
    use mwl_syntax::ast::{ClassMemberKind, StmtKind as TopStmtKind};
    use mwl_syntax::parse_file;

    use super::*;
    use crate::print::print_function;

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
        mwl_types::check_program(
            &stmts,
            map.file(file),
            &module,
            &mut checked_types,
            &mut exprs,
            &mut diags,
        );
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
        let f = lower_method(&name, method, map.file(file), &exprs, &checked_types);
        (f, map, file)
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
        mwl_types::check_program(
            &stmts,
            map.file(file),
            &module,
            &mut checked_types,
            &mut exprs,
            &mut diags,
        );
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

        let f = lower_script("<script>", &stmts, map.file(file), &exprs, &checked_types);
        (f, map, file)
    }

    /// The acceptance program of `.claude/loop-goal.md`, lowered: one
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

    /// `Ty::Mixed`'s first slice: a `mixed`-typed parameter, returned
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
    /// with a `Ty::Mixed` binding: still no retain, since `Ty::Mixed` is not
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
    /// `Ty::Mixed` from the initializer exactly the way it already does for
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

    /// `Ty::Mixed` gives a `mixed`-typed condition an IR representation to
    /// exist at all, but not a way to convert it through ADR 0035's truthy
    /// table — that still needs a runtime type-tag representation this slice
    /// deliberately doesn't build (see `Ty::Mixed`'s own doc comment). Before
    /// this slice this case was unreachable for any in-scope program (no
    /// `mixed`-typed value could exist yet); now it's a live gap, so this
    /// documents the panic actually fires rather than merely being named as
    /// theoretical.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn a_mixed_condition_still_panics_naming_the_gap() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(mixed $x): bool {\n    if ($x) {\n      return true;\n    }\n    return false;\n  }\n}\n",
        );
    }

    #[test]
    #[should_panic(expected = "known gaps")]
    fn for_loops_are_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(): int {\n    int $i = 0;\n    for ($i = 0; $i < 1; $i = $i + 1) {}\n    return 0;\n  }\n}\n",
        );
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

    #[test]
    #[should_panic(expected = "known gaps")]
    fn a_nullsafe_method_call_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj?->greet();\n  }\n}\n",
        );
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

    #[test]
    #[should_panic(expected = "known gaps")]
    fn a_nullsafe_property_access_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj?->count;\n  }\n}\n",
        );
    }

    /// A property access through a plain-`object` receiver erases per ADR
    /// 0036 § 4 — `mwl_types` records no `ExprInfo::Property` entry for it,
    /// so lowering panics naming the case rather than reading a nonexistent
    /// declaring class.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn a_property_access_through_a_plain_object_receiver_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(object $o): mixed {\n    return $o->x;\n  }\n}\n",
        );
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
    /// two-`InstKind::Concat` shape a written-out `"pre" . $mid . " post"`
    /// would, per `Lowering::lower_interpolated_parts`. `$mid`'s own read is
    /// an aliasing one, so it's left unreleased by the first `Concat`
    /// (its slot still owns it); the two literal text pieces and both
    /// intermediate/final `Concat` results are all fresh and released once
    /// each side reads them, ending with the whole method's own `void`
    /// exit releasing `$s`.
    #[test]
    fn interpolated_string_with_text_on_both_sides_folds_left_to_right() {
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
    /// `InstKind::Concat` fold `lower_interpolated_parts` already builds for
    /// a double-quoted literal — the only difference is each `Text` run
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

    /// A property write through a plain-`object` receiver erases per ADR
    /// 0036 § 4 — `mwl_types` records no `ExprInfo::Property` entry for it,
    /// so lowering panics naming the case, the same way the read side already
    /// does for the identical receiver shape.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn writing_through_a_plain_object_receiver_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(object $o): void {\n    $o->x = 1;\n  }\n}\n",
        );
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

    /// `$obj . "x"` where `$obj`'s class implements `Stringable` — accepted
    /// by `mwl_types::expr::check_expr`'s `require_stringable` (ADR 0028
    /// § 1), but this crate has no way to synthesize the resolved
    /// `toString()` call `.` would need to desugar to: a `.` operand isn't
    /// itself a call expression, so there is no
    /// `mwl_types::expr_table::ExprInfo::Call` entry recorded for it the way
    /// an actual `$obj->toString()` call site would have. Lowering panics
    /// naming the case instead of guessing at a target.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn concatenating_a_stringable_object_operand_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass Name implements Stringable {\n  function toString(): string { return \"x\"; }\n}\nclass T {\n  function m(Name $n): string {\n    return $n . \"x\";\n  }\n}\n",
        );
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
    // `ArrayNew` (empty) + `ArraySet`* shape. `...spread`/`&value` still
    // panic naming the gap (see the `should_panic` fixtures at the end of
    // this block).

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

    /// `[5 => "a"]` — an `int` literal key normalizes to its decimal string
    /// via `Lowering::lower_array_key`'s existing `Helper::IntToString`
    /// conversion, the exact same helper an `$arr[$i]` subscript already
    /// reuses.
    #[test]
    fn an_int_literal_keyed_array_element_normalizes_the_key_to_a_string() {
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

    #[test]
    #[should_panic(expected = "known gap")]
    fn a_spread_array_element_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [1];\n    array $b = [...$a];\n  }\n}\n",
        );
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
    /// normalized to its decimal-string form through
    /// `Helper::IntToString` before `InstKind::ArrayGet` reads it. The
    /// converted key is a fresh, non-aliasing buffer nothing else will ever
    /// release, so `Lowering::lower_expr`'s `Index` arm releases it right
    /// after the read — the same "release a fresh value once its one and
    /// only use is done" policy `concat_operand`'s caller already applies.
    #[test]
    fn reading_an_int_element_through_a_literal_key_normalizes_it_to_a_string() {
        let (f, map, file) = lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): int {\n    return $a[0];\n  }\n}\n",
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

    /// `$a[]` is legal to *parse* in any expression position (`mwl-syntax`'s
    /// postfix-index parsing doesn't restrict an empty subscript to an
    /// assignment target), and `mwl_types::expr::check_expr`'s `Index` arm
    /// doesn't reject it as a read either — it simply skips checking a
    /// subscript that isn't there and still resolves the element type from
    /// the base. So this reaches `Lowering::lower_expr`'s own `Index` arm,
    /// which is the one that draws the "append is assignment-target-only"
    /// line and panics naming it.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn reading_base_append_syntax_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): int {\n    return $a[];\n  }\n}\n",
        );
    }

    /// `$a[0] = 5;` through an `array<int>` parameter — the simplest
    /// array-element write: a fresh, non-refcounted `int` value (no retain)
    /// and a literal `int` key normalized the same way the read side is,
    /// with no old-value get/release pair at all (`InstKind::ArraySet`'s own
    /// doc comment explains why an ordinary new-or-existing-key write bundles
    /// that into one instruction rather than splitting it like `FieldSet`
    /// does).
    #[test]
    fn writing_an_int_element_through_a_literal_key_normalizes_it_to_a_string() {
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
    /// `writing_an_int_element_through_a_literal_key_normalizes_it_to_a_string`
    /// but with no `lower_array_key`/`helper.int_to_string` conversion in the
    /// output at all, since there is no key to normalize.
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

    /// `$grid[0][1] = 5;` — a nested subscript would have to separate every
    /// level of the chain and write each one back in turn, so it panics
    /// naming itself rather than silently dropping the outer levels'
    /// separation. See `Lowering::write_back_array`.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn writing_through_a_nested_subscript_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(array<array<int>> $g): void {\n    $g[0][1] = 5;\n  }\n}\n",
        );
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
    /// lower_and`'s branch/`Phi`-merge shape, reached from `return`'s
    /// mutable `cur` via `Lowering::lower_expr_top`.
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
    /// through `Lowering::lower_not`'s own `Lowering::lower_expr_top` call
    /// rather than the plain, non-branching `Lowering::lower_expr` a nested
    /// `!` would otherwise be stuck with.
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
    /// `crate::ty::Ty` representations (`int` vs `string`) — the checker's
    /// own union of their static types has no IR representation this crate
    /// can fold into yet, so `Lowering::lower_ternary` panics naming the
    /// mismatch rather than guessing which side wins.
    #[test]
    #[should_panic(expected = "share the same IR-level type")]
    fn a_ternary_with_mismatched_branch_types_still_panics_naming_the_gap() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $c): mixed {\n    mixed $r = $c ? 1 : \"x\";\n    return $r;\n  }\n}\n",
        );
    }

    /// `&&`/`||`/`!`/ternary only compose at a position that already owns a
    /// mutable `cur` — a call argument still only has a fixed `cur: BlockId`,
    /// so `$a && $b` nested there still panics via the plain, non-branching
    /// `Lowering::lower_expr`'s existing arithmetic/equality/ordering-only
    /// `Binary` table, exactly as before this slice.
    #[test]
    #[should_panic(expected = "arithmetic/equality/ordering operators")]
    fn a_short_circuit_and_nested_in_a_call_argument_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(bool $a, bool $b): void {\n    self::take($a && $b);\n  }\n  static function take(bool $x): void {}\n}\n",
        );
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

    /// `break`/`continue` outside any loop at all reach lowering unrejected —
    /// `mwl_types` does not yet check loop nesting at all (see the crate
    /// docs' known gaps) — so `Lowering::loop_stack` being empty is the one
    /// place this still gets caught, defensively, rather than lowering a
    /// `Jump` to a block that was never created.
    #[test]
    #[should_panic(expected = "no enclosing loop")]
    fn break_outside_any_loop_panics_naming_the_gap() {
        lower_first_method("<?mwl\nclass T {\n  function m(): void {\n    break;\n  }\n}\n");
    }

    /// Same as above, for `continue`.
    #[test]
    #[should_panic(expected = "no enclosing loop")]
    fn continue_outside_any_loop_panics_naming_the_gap() {
        lower_first_method("<?mwl\nclass T {\n  function m(): void {\n    continue;\n  }\n}\n");
    }

    /// `break 2;`/`continue 2;` — a multi-level break/continue — still
    /// panics naming the gap: unwinding more than one enclosing loop would
    /// need every `LoopFrame` up to the `N`-th on `Lowering::loop_stack` to
    /// become the statement's target, not just the innermost one
    /// `Lowering::loop_exit_level` reads today.
    #[test]
    #[should_panic(expected = "multi-level break")]
    fn a_multi_level_break_still_panics_naming_the_gap() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(int $n): void {\n    while ($n > 0) {\n      while ($n > 0) {\n        break 2;\n      }\n    }\n  }\n}\n",
        );
    }

    /// Same as above, for `continue`.
    #[test]
    #[should_panic(expected = "multi-level continue")]
    fn a_multi_level_continue_still_panics_naming_the_gap() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(int $n): void {\n    while ($n > 0) {\n      continue 2;\n    }\n  }\n}\n",
        );
    }

    /// Runs the whole front end over `src` and lowers the file — every class
    /// method, the script frame, and the class table.
    fn lower_whole_file(src: &str) -> crate::ir::Program {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = mwl_hir::resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut checked_types = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        mwl_types::check_program(
            &stmts,
            map.file(file),
            &module,
            &mut checked_types,
            &mut exprs,
            &mut diags,
        );
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let layouts = mwl_types::build_class_layouts(&stmts, map.file(file), &module.graph);
        lower_file(
            "<script>",
            &stmts,
            map.file(file),
            &exprs,
            &checked_types,
            &layouts,
        )
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
    /// separate — out of scope, and named rather than mislowered.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn a_by_reference_foreach_value_binding_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl
class T {
  function m(array<int> $a): void {
    foreach ($a as int &$v) {
      echo $v;
    }
  }
}
",
        );
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
    /// checker records nothing and lowering refuses it rather than guessing.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn a_dynamic_instanceof_is_still_out_of_scope() {
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

    /// A file declaring no class still lowers, and still carries the
    /// exception tree plus its one synthesized constructor — the `hello.mwl`
    /// shape. Nothing in the file references either, and both are emitted
    /// anyway: `mwl_hir::errors`' classes exist in every program.
    #[test]
    fn a_file_with_no_class_still_carries_the_exception_tree() {
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
                "IOError",
                "LogicError",
                "ParseError",
                "RuntimeError",
                "Throwable",
                "TimeoutError",
            ]
        );
        let functions: Vec<&str> = program.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(functions, ["<script>", "Throwable::constructor"]);
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
}
