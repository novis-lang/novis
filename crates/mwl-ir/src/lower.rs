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
    AssignOp, BinaryOp, CallArgs, Expr, ExprKind, MethodMember, Stmt, StmtKind, Type, TypeAtom,
    TypeKind, UnaryOp as AstUnaryOp,
};
use mwl_types::expr_table::{ExprInfo, ExprTypeTable};
use mwl_types::ty::{Ty as CheckedTy, TypeId, TypeInterner};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{BlockId, IdGen, ValueId};
use crate::ir::{BasicBlock, BinOp, Function, Inst, InstKind, Terminator, UnOp};
use crate::ty::Ty;
use crate::{span_text, strip_sigil};

/// A local's current SSA binding: which value it holds, and at what
/// representation type.
type Env = FxHashMap<String, (ValueId, Ty)>;

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
    let mut low = Lowering::new(src, ret_ty, exprs, checked_types);
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
}

impl<'a> Lowering<'a> {
    fn new(
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
        });
        (v, ty)
    }

    /// Appends a reserved [`InstKind::Safepoint`] marker to `b` — see that
    /// variant's own doc comment for the two call sites this has today
    /// (function entry, a loop's back edge) and why it defines no value.
    fn emit_safepoint(&mut self, b: BlockId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Safepoint,
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
        });
    }

    /// Appends an [`InstKind::Release`] on `v` to `b` — see [`Self::emit_retain`].
    fn emit_release(&mut self, b: BlockId, v: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::Release { operand: v },
        });
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
    fn release_all_locals(&mut self, cur: BlockId, env: &Env, except: Option<&str>) {
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

    /// Lowers a statement list into `cur`, stopping early once `cur` is
    /// sealed (dead code after a `return` inside the list is simply never
    /// lowered — nothing downstream needs it modeled).
    fn lower_stmts(&mut self, stmts: &[Stmt], cur: &mut BlockId, env: &mut Env) {
        for stmt in stmts {
            if self.is_terminated(*cur) {
                break;
            }
            self.lower_stmt(stmt, cur, env);
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt, cur: &mut BlockId, env: &mut Env) {
        let stmt_id = self.ids.next_stmt(stmt.span);
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::StmtMarker(stmt_id),
        });
        match &stmt.kind {
            StmtKind::LocalDecl {
                ty: Some(decl_ty),
                name: local_name,
                value: Some(value),
            } => {
                let expected = lower_decl_type(decl_ty);
                let (v, _) = self.lower_expr(value, Some(expected), env, *cur);
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
                let (v, ty) = self.lower_expr(value, None, env, *cur);
                let lname = strip_sigil(span_text(self.src, *local_name)).to_owned();
                self.bind_local(*cur, env, lname, v, ty, value);
            }
            StmtKind::Expr(e) => self.lower_expr_stmt(e, env, *cur),
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
                let v = value.as_ref().map(|v| {
                    let (rv, rty) = self.lower_expr(v, Some(self.ret_ty), env, *cur);
                    if except.is_none() && rty.is_refcounted() && is_aliasing_read(&v.kind) {
                        self.emit_retain(*cur, rv);
                    }
                    rv
                });
                self.release_all_locals(*cur, env, except.as_deref());
                self.seal(*cur, Terminator::Return(v));
            }
            StmtKind::Block(b) => self.lower_stmts(&b.stmts, cur, env),
            StmtKind::If { cond, then, else_ } => {
                self.lower_if(cond, then, else_.as_deref(), cur, env);
            }
            StmtKind::While { cond, body } => self.lower_while(cond, body, cur, env),
            other => panic!(
                "mwl-ir's control-flow slice only lowers a typed local declaration, a plain \
                 reassignment, `return`, a nested block, `if` and `while` — got {other:?}; see \
                 the crate docs' known gaps"
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
    fn lower_expr_stmt(&mut self, e: &Expr, env: &mut Env, cur: BlockId) {
        match &e.kind {
            ExprKind::Assign {
                op: AssignOp::Assign,
                by_ref: false,
                ..
            } => self.lower_reassignment(e, env, cur),
            ExprKind::MethodCall { .. } | ExprKind::StaticCall { .. } | ExprKind::New { .. } => {
                let (v, ty) = self.lower_expr(e, None, env, cur);
                if ty.is_refcounted() {
                    self.emit_release(cur, v);
                }
            }
            other => panic!(
                "mwl-ir's control-flow slice only lowers a plain `$x = expr;` reassignment or a \
                 bare call/`new` as an expression statement — got {other:?}; see the crate \
                 docs' known gaps"
            ),
        }
    }

    /// `$x = expr;` as a bare expression statement — SSA renaming needs no
    /// join logic here, only a fresh binding in `env`.
    fn lower_reassignment(&mut self, e: &Expr, env: &mut Env, cur: BlockId) {
        let ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
            by_ref: false,
        } = &e.kind
        else {
            unreachable!("Self::lower_expr_stmt only routes a plain `AssignOp::Assign` here");
        };
        let ExprKind::Variable(name_span) = &target.kind else {
            panic!(
                "mwl-ir's control-flow slice only lowers reassignment to a plain local, not {:?}",
                target.kind
            );
        };
        let lname = strip_sigil(span_text(self.src, *name_span)).to_owned();
        let expected = env.get(&lname).map(|&(_, t)| t);
        let (v, ty) = self.lower_expr(value, expected, env, cur);
        self.bind_local(cur, env, lname, v, ty, value);
    }

    /// `if (cond) then (else else_)?` — the module docs describe the
    /// merge-point construction this drives.
    ///
    /// # Panics
    ///
    /// Panics if `cond` isn't statically `bool` — ADR 0035's full truthy
    /// conversion for a non-`bool` condition needs a runtime-helper call,
    /// which doesn't exist in the IR yet (see the crate docs' known gaps).
    fn lower_if(
        &mut self,
        cond: &Expr,
        then: &Stmt,
        else_: Option<&Stmt>,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let (cond_v, cond_ty) = self.lower_expr(cond, Some(Ty::Bool), env, *cur);
        assert!(
            cond_ty == Ty::Bool,
            "mwl-ir's control-flow slice only lowers a `bool`-typed `if`/`while` condition — a \
             truthy conversion of a non-`bool` value needs a runtime-helper call, a known gap"
        );

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
    /// construction this drives.
    ///
    /// # Panics
    ///
    /// Panics if `cond` isn't statically `bool` — see [`Self::lower_if`]'s
    /// panic doc, the same restriction applies here.
    fn lower_while(&mut self, cond: &Expr, body: &Stmt, cur: &mut BlockId, env: &mut Env) {
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
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

        let (cond_v, cond_ty) = self.lower_expr(cond, Some(Ty::Bool), &header_env, header_block);
        assert!(
            cond_ty == Ty::Bool,
            "mwl-ir's control-flow slice only lowers a `bool`-typed `if`/`while` condition — a \
             truthy conversion of a non-`bool` value needs a runtime-helper call, a known gap"
        );

        let body_block = self.new_block();
        let after_block = self.new_block();
        let body_edge = self.ids.next_edge(body.span);
        let after_edge = self.ids.next_edge(cond.span);
        self.seal(
            header_block,
            Terminator::Branch {
                cond: cond_v,
                then_block: body_block,
                then_edge: body_edge,
                else_block: after_block,
                else_edge: after_edge,
            },
        );

        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        self.lower_stmt(body, &mut body_cur, &mut body_env);
        if !self.is_terminated(body_cur) {
            // Reserved safepoint poll site (loop back edge) — see
            // `InstKind::Safepoint`'s own doc comment. Placed on the actual
            // back edge, not the loop header, so a body that never reaches
            // it (e.g. it always `return`s) polls zero times per skipped
            // iteration, same as a functional poll would.
            self.emit_safepoint(body_cur);
            self.seal(body_cur, Terminator::Jump(header_block));
            for (name, inst_index) in &phi_slots {
                let &(back_v, _) = body_env.get(name).unwrap_or_else(|| {
                    panic!(
                        "mwl-ir: `{name}` was reassigned in a while body per the syntactic scan \
                         but is missing from its exit environment — bug in \
                         collect_reassigned_locals"
                    )
                });
                let inst = &mut self.block_insts[header_block.index() as usize][*inst_index];
                let InstKind::Phi { incoming } = &mut inst.kind else {
                    unreachable!("phi_slots only ever indexes a Phi instruction");
                };
                incoming.push((body_cur, back_v));
            }
        }
        // If the body never reaches its own back edge (e.g. it always
        // `return`s), each header phi keeps its single pre-loop incoming
        // edge — a degenerate but valid phi, since no optimizer exists yet
        // to fold a single-input phi away.

        *env = header_env;
        *cur = after_block;
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
                    && let ExprKind::Variable(name_span) = &target.kind
                {
                    let name = strip_sigil(span_text(self.src, *name_span)).to_owned();
                    if seen.insert(name.clone()) {
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
            StmtKind::While { body, .. } => {
                self.collect_reassigned_locals(body, seen, out);
            }
            _ => {}
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
            ExprKind::Bool(b) => self.emit(cur, Ty::Bool, InstKind::ConstBool(*b)),
            // ADR 0007 § 4, mirroring `mwl_types::expr::infer`'s own rule: a
            // bare integer literal means `uint` exactly where that's the
            // expected type, `int` otherwise. Magnitude range-checking is a
            // known gap here, same as it already is there.
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
            ExprKind::Unary { op, expr: inner } => {
                let (v, ty) = self.lower_expr(inner, expected, env, cur);
                let uop = match op {
                    AstUnaryOp::Neg => UnOp::Neg,
                    AstUnaryOp::Not => UnOp::Not,
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
                self.emit(
                    cur,
                    ty,
                    InstKind::BinOp {
                        op: bop,
                        lhs: lv,
                        rhs: rv,
                    },
                )
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
                let arg_values = match ctor {
                    Some(call) => {
                        let param_tys = call.param_tys.clone();
                        let variadic = call.variadic;
                        let checked_types = self.checked_types;
                        self.lower_call_args(args, &param_tys, variadic, checked_types, env, cur)
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
                self.emit(
                    cur,
                    Ty::Object,
                    InstKind::New {
                        class: target_label,
                        args: arg_values,
                    },
                )
            }
            // `$obj->method(...)`/`$this->method(...)` — the receiver is
            // lowered like any other expression (for `$this`, that's just an
            // `Env` lookup, since `lower_method` already seeded it as the
            // implicit parameter 0); the resolved target itself still comes
            // from `self.exprs`, exactly like a static call/`new` below.
            ExprKind::MethodCall {
                object,
                nullsafe,
                args,
                ..
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
                let (receiver_v, _) = self.lower_expr(object, None, env, cur);
                let arg_values =
                    self.lower_call_args(args, &param_tys, variadic, checked_types, env, cur);
                self.emit(
                    cur,
                    return_ty,
                    InstKind::Call {
                        target: target_label,
                        receiver: Some(receiver_v),
                        args: arg_values,
                    },
                )
            }
            // `self::method(...)`/`Class::method(...)` — no receiver value.
            ExprKind::StaticCall { args, .. } => {
                let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
                    panic!(
                        "mwl-ir: a static call at {:?} has no resolved target recorded in the \
                         typed-expression table — did this program pass \
                         mwl_types::check_program with the same table?",
                        expr.span
                    );
                };
                let target_label = format!("{}::{}", call.class, call.method);
                let param_tys = call.param_tys.clone();
                let variadic = call.variadic;
                let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
                let checked_types = self.checked_types;
                let arg_values =
                    self.lower_call_args(args, &param_tys, variadic, checked_types, env, cur);
                self.emit(
                    cur,
                    return_ty,
                    InstKind::Call {
                        target: target_label,
                        receiver: None,
                        args: arg_values,
                    },
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
            other => panic!(
                "mwl-ir's control-flow slice only lowers literals, locals, unary/binary \
                 operators, `new`, a static or instance method call, and property access — got \
                 {other:?}; see the crate docs' known gaps"
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
    fn lower_call_args(
        &mut self,
        args: &CallArgs,
        param_tys: &[TypeId],
        variadic: bool,
        checked_types: &TypeInterner,
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
            if ty.is_refcounted() && is_aliasing_read(&arg.value.kind) {
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
/// `single_quoted_string_only_escapes_backslash_and_quote` test); a double-quoted literal
/// additionally cooks the common named escapes (`\n`, `\t`, `\r`, `\\`, `\"`, `\$`, `\0`). Any
/// other backslash sequence (a numeric escape like `\xHH`/`\u{...}`/octal, or any escape not
/// meaningful for the quote kind in play) is passed through literally rather than cooked — a
/// known gap, mirroring [`int_literal_digits`]'s own "not modeled this slice" magnitude gap for
/// integer literals. A heredoc/nowdoc-sourced `Str` — whose span doesn't open with a quote
/// character at all — isn't handled here either: this crate has no lowered fixture reaching one
/// yet, so it panics naming the gap rather than guessing a representation.
fn cook_str_literal(src: &SourceFile, span: mwl_diagnostics::Span) -> String {
    let raw = span_text(src, span);
    let quote = raw
        .chars()
        .next()
        .unwrap_or_else(|| panic!("mwl-ir: an empty string literal span at {span:?} — lexer bug?"));
    assert!(
        quote == '\'' || quote == '"',
        "mwl-ir does not yet cook a heredoc/nowdoc string literal (its span doesn't open with a \
         quote character) — got {raw:?}; see the crate docs' known gaps"
    );
    let inner = &raw[quote.len_utf8()..raw.len() - quote.len_utf8()];
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
            Some('n') if quote == '"' => out.push('\n'),
            Some('t') if quote == '"' => out.push('\t'),
            Some('r') if quote == '"' => out.push('\r'),
            Some('$') if quote == '"' => out.push('$'),
            Some('0') if quote == '"' => out.push('\0'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Splits a cooked integer-literal span into the radix its prefix names and
/// the digit run to parse against it — `mwl_syntax::Lexer::lex_number` emits
/// one `IntLiteral` token for all four forms (`0x…`/`0o…`/`0b…`, or a plain
/// decimal run; a legacy leading-zero octal spelling like PHP's `0755` is
/// deliberately *not* one of them, so `0755` lexes as decimal 755 with no
/// prefix to strip), and this is the one place that distinction has to be
/// undone before `str::from_str_radix` can parse the value. Full-magnitude
/// range-checking stays a known gap here, same as it already is in
/// `mwl_types::expr::infer`'s own doc comment for the decimal case — this
/// only widens which *bases* get cooked, not how large a value either arm
/// accepts.
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

/// Lowers a *declared* type straight off the AST — every scalar atom, plus
/// `TypeAtom::Name(_)` (a plain class/interface/enum name) as
/// [`Ty::Object`]. A plain name needs no resolution to lower this way: ADR
/// 0007 § 1 already requires it to be spelled out in full, and this crate
/// erases class identity entirely (see [`Ty::Object`]'s own doc comment), so
/// "is this atom a class name at all" is the only question that matters here
/// — which class doesn't need answering until a call/`new` on it does, via
/// [`lower_checked_ty`] instead. `self`/`static`/`parent` are not handled:
/// resolving those needs the enclosing class, which this crate's straight-off-
/// the-AST design (see the crate docs) has never needed to track before now.
fn lower_decl_type(ty: &Type) -> Ty {
    match &ty.kind {
        TypeKind::Atom(TypeAtom::Bool) => Ty::Bool,
        TypeKind::Atom(TypeAtom::Int) => Ty::Int,
        TypeKind::Atom(TypeAtom::Uint) => Ty::Uint,
        TypeKind::Atom(TypeAtom::Float) => Ty::Float,
        TypeKind::Atom(TypeAtom::Void) => Ty::Void,
        TypeKind::Atom(TypeAtom::String) => Ty::Str,
        TypeKind::Atom(TypeAtom::Name(_)) => Ty::Object,
        TypeKind::Paren(inner) => lower_decl_type(inner),
        other => panic!(
            "mwl-ir only lowers bool/int/uint/float/void/string/a plain class name as a declared \
             type — got {other:?}; see the crate docs' known gaps"
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
/// need to survive this translation. `String` erases to [`Ty::Str`], the same
/// representation [`lower_decl_type`] already gives a local/parameter/return
/// type spelled directly in source — see [`crate::lower`]'s module docs for
/// the retain policy this now needs at a call-argument/return/property-field
/// boundary, which [`Lowering::bind_local`], [`Lowering::lower_call_args`] and
/// `StmtKind::Return`'s own arm all apply via [`is_aliasing_read`].
///
/// # Panics
///
/// Panics naming the unsupported shape for anything outside this slice's
/// scope: `bytes`, either qualified (`tainted`/`secret`) string or bytes
/// variant, `array<T>`, `object`, a shape, a union/intersection, or any of
/// `mixed`/`never`/`true`/`false`/`iterable`/`callable`/`null` — none of these
/// have an IR representation yet (see the crate docs' known gaps).
fn lower_checked_ty(id: TypeId, checked_types: &TypeInterner) -> Ty {
    match checked_types.get(id) {
        CheckedTy::Bool => Ty::Bool,
        CheckedTy::Int => Ty::Int,
        CheckedTy::Uint => Ty::Uint,
        CheckedTy::Float => Ty::Float,
        CheckedTy::Void => Ty::Void,
        CheckedTy::String => Ty::Str,
        CheckedTy::Class(_) | CheckedTy::Enum(_) => Ty::Object,
        other => panic!(
            "mwl-ir only lowers a resolved call's bool/int/uint/float/void/string/class/enum \
             parameter or return type — got {other:?}; see the crate docs' known gaps"
        ),
    }
}

/// Whether reading `kind` produces a *borrowed* reference to storage some
/// other binding still owns, rather than a freshly constructed value with
/// exactly one natural owner — the same "is this a copy or a fresh value"
/// judgment [`Lowering::bind_local`]'s own doc comment already describes for
/// a bare variable read, now shared with a call argument
/// ([`Lowering::lower_call_args`]) and a returned expression
/// (`StmtKind::Return`'s own arm). A plain local (`ExprKind::Variable`) and a
/// compile-time-known property read (`ExprKind::PropertyAccess`) both borrow
/// storage that keeps its own reference after this read — a local's own slot,
/// or the object's field — so copying either into a new durable slot needs a
/// retain. A fresh literal, `new`, or a call's own result already has exactly
/// one natural owner and needs none.
fn is_aliasing_read(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Variable(_) | ExprKind::PropertyAccess { .. }
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
    /// a constructor's side effect. `Ty::Object` isn't refcounted yet (see
    /// the crate docs' known gaps), so `Lowering::lower_expr_stmt` lowers the
    /// construction but emits no release for it.
    #[test]
    fn a_bare_new_used_as_a_statement_lowers_with_no_release() {
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
}
