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
    AssignOp, BinaryOp, CallArgs, Expr, ExprKind, MethodMember, Stmt, StmtKind, StringPart, Type,
    TypeAtom, TypeKind, UnaryOp as AstUnaryOp,
};
use mwl_types::expr_table::{ExprInfo, ExprTypeTable};
use mwl_types::ty::{Ty as CheckedTy, TypeId, TypeInterner};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ids::{BlockId, IdGen, ValueId};
use crate::ir::{BasicBlock, BinOp, Function, Helper, Inst, InstKind, Terminator, UnOp};
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
        });
    }

    /// Appends an [`InstKind::ArraySet`] to `b` — see
    /// [`Self::lower_reassignment`]'s `Index`-target arm for the retain
    /// policy wrapped around this.
    fn emit_array_set(&mut self, b: BlockId, array: ValueId, key: ValueId, value: ValueId) {
        self.block_insts[b.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::ArraySet { array, key, value },
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

    /// `$x = expr;` or `$obj->prop = expr;` as a bare expression statement —
    /// SSA renaming needs no join logic here, only a fresh binding in `env`
    /// (a local target) or a [`InstKind::FieldSet`] (a property target).
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
        match &target.kind {
            ExprKind::Variable(name_span) => {
                let lname = strip_sigil(span_text(self.src, *name_span)).to_owned();
                let expected = env.get(&lname).map(|&(_, t)| t);
                let (v, ty) = self.lower_expr(value, expected, env, cur);
                self.bind_local(cur, env, lname, v, ty, value);
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
                let (object_v, _) = self.lower_expr(object, None, env, cur);
                let (v, _) = self.lower_expr(value, Some(field_ty), env, cur);
                if field_ty.is_refcounted() && is_aliasing_read(&value.kind) {
                    self.emit_retain(cur, v);
                }
                if field_ty.is_refcounted() {
                    let (old_v, _) = self.emit(
                        cur,
                        field_ty,
                        InstKind::FieldGet {
                            object: object_v,
                            class: class_label.clone(),
                            field: field_name.clone(),
                        },
                    );
                    self.emit_release(cur, old_v);
                }
                self.emit_field_set(cur, object_v, class_label, field_name, v);
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
            // (`index` is `None`) — PHP's append syntax — still panics: it
            // needs its own "next available integer key" counter this crate
            // has no representation for yet.
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
                let Some(index) = index else {
                    panic!(
                        "mwl-ir does not yet lower `$a[] = expr;` append syntax; see the crate \
                         docs' known gaps"
                    );
                };
                let (array_v, _) = self.lower_expr(base, None, env, cur);
                let (key_v, key_aliasing) = self.lower_array_key(index, env, cur);
                if key_aliasing {
                    self.emit_retain(cur, key_v);
                }
                let (v, _) = self.lower_expr(value, Some(elem_ty), env, cur);
                if elem_ty.is_refcounted() && is_aliasing_read(&value.kind) {
                    self.emit_retain(cur, v);
                }
                self.emit_array_set(cur, array_v, key_v, v);
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
            // `[...]`/legacy `array(...)` — see `InstKind::ArrayNew`'s own
            // doc comment for the full policy this mirrors and its known
            // gaps. This slice only lowers a *positional* literal: every
            // `ArrayItem` must supply no explicit `key =>`, no `...spread`
            // and no `&value` — each unsupported shape panics naming itself
            // rather than guessing at a runtime conversion this crate can't
            // yet synthesize (`mwl_types::expr::check_array_literal` itself
            // has no key-normalization/rejection logic yet either). A
            // positional element's own key is simply its index, auto-
            // numbered from `0` exactly like PHP's own `[$a, $b]` shorthand.
            // Each element that's itself `Ty::is_refcounted` and
            // `is_aliasing_read` is retained before the array durably owns
            // it, the same policy `Self::lower_call_args` already applies at
            // a call-argument boundary; the array literal's own result needs
            // no retain — a fresh producer, same as `new`/a call's result.
            ExprKind::ArrayLiteral(items) => {
                let mut entries = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    assert!(
                        item.key.is_none() && !item.spread && !item.by_ref,
                        "mwl-ir only lowers a positional array-literal element — an explicit \
                         `key =>`, a `...spread`, or a `&value` element is a known gap"
                    );
                    let (v, ty) = self.lower_expr(&item.value, None, env, cur);
                    if ty.is_refcounted() && is_aliasing_read(&item.value.kind) {
                        self.emit_retain(cur, v);
                    }
                    entries.push((i.to_string(), v));
                }
                self.emit(cur, Ty::Array, InstKind::ArrayNew { entries })
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
            other => panic!(
                "mwl-ir's control-flow slice only lowers literals, locals, unary/binary \
                 operators, `new`, a static or instance method call, property access, an array \
                 literal, and an array-element read — got {other:?}; see the crate docs' known \
                 gaps"
            ),
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
                    Ty::Str | Ty::Bytes | Ty::Void | Ty::Object | Ty::Array => {
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
    /// policy. A `float`, `bool`, or `null` subscript is a compile-time
    /// rejection ADR 0007 § 5 also names, but `mwl_types::expr::check_expr`'s
    /// `Index` arm doesn't enforce it yet — the same known gap
    /// [`ir::InstKind::ArrayNew`]'s own doc comment already names for an
    /// array literal's explicit `key =>` — so this panics naming the case
    /// rather than guessing at a conversion PHP itself doesn't define for
    /// those types.
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
                "mwl-ir only lowers an int/uint/string array-subscript key — got {other:?}; \
                 mwl_types doesn't yet reject a float/bool/null subscript (ADR 0007 § 5), so \
                 this crate can't trust it was rejected upstream; see the crate docs' known gaps"
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

/// Lowers a *declared* type straight off the AST — every scalar atom, plus
/// `TypeAtom::Name(_)` (a plain class/interface/enum name) as
/// [`Ty::Object`] and `TypeAtom::Array(_)` (bare `array` or `array<T>`) as
/// [`Ty::Array`]. A plain name needs no resolution to lower this way: ADR
/// 0007 § 1 already requires it to be spelled out in full, and this crate
/// erases class identity entirely (see [`Ty::Object`]'s own doc comment), so
/// "is this atom a class name at all" is the only question that matters here
/// — which class doesn't need answering until a call/`new` on it does, via
/// [`lower_checked_ty`] instead. `array<T>`'s own type argument is discarded
/// the same way — [`Ty::Array`]'s own doc comment explains why no lowering
/// decision needs it at this level. `self`/`static`/`parent` are not handled:
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
        TypeKind::Atom(TypeAtom::Bytes) => Ty::Bytes,
        TypeKind::Atom(TypeAtom::Name(_)) => Ty::Object,
        TypeKind::Atom(TypeAtom::Array(_)) => Ty::Array,
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
        CheckedTy::Bytes => Ty::Bytes,
        CheckedTy::Class(_) | CheckedTy::Enum(_) => Ty::Object,
        // The element `TypeId` is discarded — same erasure `lower_decl_type`
        // already gives `TypeAtom::Array(_)`, see `Ty::Array`'s own doc
        // comment for why this crate has no lowering decision that needs it.
        CheckedTy::Array(_) => Ty::Array,
        other => panic!(
            "mwl-ir only lowers a resolved call's bool/int/uint/float/void/string/bytes/array/\
             class/enum parameter or return type — got {other:?}; see the crate docs' known gaps"
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
    // instruction it needs. Every fixture below is a *positional* literal —
    // no explicit `key =>`, no `...spread`, no `&value` — since those still
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

    #[test]
    #[should_panic(expected = "known gap")]
    fn an_explicit_keyed_array_element_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(): void {\n    array $a = [\"k\" => 1];\n  }\n}\n",
        );
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

    #[test]
    #[should_panic(expected = "known gaps")]
    fn writing_base_append_syntax_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a): void {\n    $a[] = 1;\n  }\n}\n",
        );
    }

    /// A `bool` subscript isn't one of ADR 0007 § 5's three legal key
    /// source types (`int`/`uint`/`string`) — `mwl_types` doesn't reject it
    /// at check time yet (the same known gap `InstKind::ArrayNew`'s own doc
    /// comment already names for an array literal's explicit `key =>`), so
    /// `Lowering::lower_array_key` is the one that panics naming it, rather
    /// than guessing at a conversion PHP itself doesn't define for that type.
    #[test]
    #[should_panic(expected = "known gaps")]
    fn a_bool_subscript_key_is_still_out_of_scope() {
        lower_first_method(
            "<?mwl\nclass T {\n  function m(array<int> $a, bool $b): void {\n    $a[$b] = 1;\n  }\n}\n",
        );
    }
}
