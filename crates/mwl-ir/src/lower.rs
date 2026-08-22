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
    AssignOp, BinaryOp, Expr, ExprKind, MethodMember, Stmt, StmtKind, Type, TypeAtom, TypeKind,
    UnaryOp as AstUnaryOp,
};
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
/// [`Function`] named `name`.
///
/// # Panics
///
/// Panics, naming the unsupported shape, if `m` has no body or its body
/// leaves this slice's scope. This is not a diagnostic: callers are expected
/// to have already run `mwl_types::check_program` and to only route programs
/// within scope through this function until lowering widens.
#[must_use]
pub fn lower_method(name: &str, m: &MethodMember, src: &SourceFile) -> Function {
    let ret_ty = m.return_type.as_ref().map_or(Ty::Void, lower_scalar_type);
    let mut low = Lowering::new(src, ret_ty);
    let entry = low.new_block();
    let mut cur = entry;
    let mut env = Env::default();
    let mut param_tys = Vec::new();

    // Reserved safepoint poll site (recursion) — see `InstKind::Safepoint`'s
    // own doc comment for why function entry is one of the two fixed sites
    // and why this slice reserves only the shape, not a functional check.
    low.emit_safepoint(entry);

    for (i, p) in m.params.iter().enumerate() {
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
        let ty = lower_scalar_type(decl_ty);
        let index = u32::try_from(i).expect("far more parameters than a call could ever take");
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
    if !low.is_terminated(cur) {
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
    fn new(src: &'a SourceFile, ret_ty: Ty) -> Self {
        Self {
            ids: IdGen::new(),
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
                let expected = lower_scalar_type(decl_ty);
                let (v, _) = self.lower_expr(value, Some(expected), env, *cur);
                env.insert(
                    strip_sigil(span_text(self.src, *local_name)).to_owned(),
                    (v, expected),
                );
            }
            StmtKind::Expr(e) => self.lower_reassignment(e, env, *cur),
            StmtKind::Return(value) => {
                let v = value
                    .as_ref()
                    .map(|v| self.lower_expr(v, Some(self.ret_ty), env, *cur).0);
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
            panic!(
                "mwl-ir's control-flow slice only lowers a plain `$x = expr;` reassignment as \
                 an expression statement — got {:?}; see the crate docs' known gaps",
                e.kind
            );
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
        env.insert(lname, (v, ty));
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
                let digits = clean_digits(self.src, *span);
                if expected == Some(Ty::Uint) {
                    let n: u64 = digits.parse().unwrap_or_else(|_| {
                        panic!("mwl-ir: integer literal `{digits}` doesn't fit a `uint`")
                    });
                    self.emit(cur, Ty::Uint, InstKind::ConstUint(n))
                } else {
                    let n: i64 = digits.parse().unwrap_or_else(|_| {
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
            other => panic!(
                "mwl-ir's control-flow slice only lowers literals, locals, and unary/binary \
                 operators over them — got {other:?}; see the crate docs' known gaps"
            ),
        }
    }
}

/// Reads a numeric-literal span's text with `_` digit separators stripped.
fn clean_digits(src: &SourceFile, span: mwl_diagnostics::Span) -> String {
    span_text(src, span).chars().filter(|&c| c != '_').collect()
}

fn lower_scalar_type(ty: &Type) -> Ty {
    match &ty.kind {
        TypeKind::Atom(TypeAtom::Bool) => Ty::Bool,
        TypeKind::Atom(TypeAtom::Int) => Ty::Int,
        TypeKind::Atom(TypeAtom::Uint) => Ty::Uint,
        TypeKind::Atom(TypeAtom::Float) => Ty::Float,
        TypeKind::Atom(TypeAtom::Void) => Ty::Void,
        TypeKind::Paren(inner) => lower_scalar_type(inner),
        other => panic!(
            "mwl-ir's first slice only lowers bool/int/uint/float/void types — got {other:?}; \
             see the crate docs' known gaps"
        ),
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use mwl_diagnostics::{Diagnostics, SourceId, SourceMap};
    use mwl_syntax::ast::{ClassMemberKind, StmtKind as TopStmtKind};
    use mwl_syntax::parse_file;

    use super::*;
    use crate::print::print_function;

    /// Parses `src`, pulls out `T`'s first method, and lowers it —
    /// `src` is expected to already be a program `mwl_types::check_program`
    /// would accept with no errors (this crate does not re-check it).
    fn lower_first_method(src: &str) -> (Function, SourceMap, SourceId) {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");

        let decl = stmts
            .iter()
            .find_map(|s| match &s.kind {
                TopStmtKind::ClassDecl(decl) => Some(decl),
                _ => None,
            })
            .expect("fixture must declare a class");
        let method = decl
            .members
            .iter()
            .find_map(|m| match &m.kind {
                ClassMemberKind::Method(method) => Some(method),
                _ => None,
            })
            .expect("fixture class must declare a method");

        let name = span_text(map.file(file), method.name).to_owned();
        let f = lower_method(&name, method, map.file(file));
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
}
