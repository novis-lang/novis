//! Statement lowering — the dispatch every statement kind goes through, plus assignment and `unset`.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

impl<'a> Lowering<'a> {
    /// [`lower_script`]'s own walk: like [`Self::lower_stmts`], but over a
    /// file's top level, where a *declaration* is not part of the frame at
    /// all (a class's methods are lowered separately, one [`lower_method`]
    /// call each) and a `namespace X { ... }` block's body is — a namespace
    /// scopes names, not storage, so its statements share this one frame.
    /// The unbraced `namespace X;` form declares no statements of its own,
    /// so it is skipped like any other declaration; the statements that
    /// follow it are siblings and are reached by the ordinary loop.
    pub(super) fn lower_script_stmts(
        &mut self,
        stmts: &'a [Stmt],
        cur: &mut BlockId,
        env: &mut Env,
    ) {
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
    pub(super) fn lower_stmts(&mut self, stmts: &'a [Stmt], cur: &mut BlockId, env: &mut Env) {
        for stmt in stmts {
            if self.is_terminated(*cur) {
                break;
            }
            self.lower_stmt(stmt, cur, env);
            // A by-reference argument staged inside this statement must have
            // been copied back by now — a leftover means the call appeared in
            // an expression position no `flush_ref_writebacks` call site
            // covers, and silently dropping the write-back would be a wrong
            // program rather than an unsupported one. See
            // `Self::pending_refs`' known gap.
            assert!(
                self.pending_refs.is_empty(),
                "mwl-ir lowers a call with a `&$x` argument only as a bare expression \
                 statement or as a plain assignment's right-hand side — the one at {:?} is in \
                 neither position, and its write-back has nowhere to land; see the crate \
                 docs' known gaps",
                stmt.span
            );
        }
    }
    pub(super) fn lower_stmt(&mut self, stmt: &'a Stmt, cur: &mut BlockId, env: &mut Env) {
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
                let expected = lower_decl_type(decl_ty, self.exprs, self.checked_types);
                let (v, actual) = self.lower_expr(value, Some(expected), env, cur);
                let v = self.coerce(*cur, v, actual, expected, env);
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
                let (v, ty) = self.lower_expr(value, None, env, cur);
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
            // ADR 0053 § 5: a generator's body has no return value, so
            // `return;` means "the sequence ends here" — the same exit
            // running off the end takes. `mwl_types` reports E0447 for a
            // `return expr;` in one, which is why this ignores `value`
            // rather than lowering it.
            StmtKind::Return(_) if self.generator.is_some() => {
                self.run_pending_finallys(cur, env);
                if !self.is_terminated(*cur) {
                    self.finish_generator(*cur, env);
                }
            }
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
                    let (rv, rty) = self.lower_expr(value_expr, Some(ret_ty), env, cur);
                    if except.is_none() && rty.is_refcounted() && self.aliasing_read(value_expr) {
                        self.emit_retain(*cur, rv);
                    }
                    Some(self.coerce(*cur, rv, rty, ret_ty, env))
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
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => self.lower_for(init, cond, step, body, cur, env),
            StmtKind::Foreach {
                subject,
                key,
                value,
                value_by_ref,
                body,
            } => self.lower_foreach(subject, key.as_ref(), value, *value_by_ref, body, cur, env),
            StmtKind::Switch { subject, cases } => self.lower_switch(subject, cases, cur, env),
            // ADR 0028 § 3 leaves exactly one `unset` target standing — an
            // array element — and `mwl_types::expr::check_unset_target`
            // already rejected a declared property, so anything else reaching
            // here is a shape this slice does not lower.
            StmtKind::Unset(targets) => {
                for target in targets {
                    self.lower_unset(target, env, cur);
                }
            }
            StmtKind::Break(level) => self.lower_break(level, cur, env),
            StmtKind::Continue(level) => self.lower_continue(level, cur, env),
            StmtKind::Echo(operands) => self.lower_echo(operands, cur, env),
            StmtKind::Try {
                body,
                catches,
                finally,
            } => self.lower_try(body, catches, finally.as_ref(), cur, env),
            other => panic!(
                "mwl-ir's control-flow slice only lowers a typed local declaration, a plain \
                 reassignment, `echo`, `unset`, `return`, a nested block, `if`, `while`, `for`, \
                 `foreach`, `switch`, `try`/`catch`, `throw` and a loop-scoped \
                 `break`/`continue` — got {other:?}; see the crate docs' known gaps"
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
    pub(super) fn lower_expr_stmt(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        match &e.kind {
            ExprKind::Assign {
                op: AssignOp::Assign,
                by_ref: false,
                ..
            } => self.lower_reassignment(e, env, cur),
            ExprKind::Assign {
                op,
                target,
                value,
                by_ref: false,
            } if op.binary_op().is_some() => {
                let binop = op.binary_op().expect("guarded by the arm's own condition");
                self.lower_compound_assignment(e, binop, target, value, env, cur);
            }
            ExprKind::MethodCall { .. } | ExprKind::StaticCall { .. } | ExprKind::New { .. } => {
                let (v, ty) = self.lower_expr(e, None, env, cur);
                if ty.is_refcounted() {
                    self.emit_release(*cur, v);
                }
                // Every `&$x` argument the call staged is copied back here —
                // the statement boundary is where an `&mut Env` exists at all.
                // See `Self::pending_refs`.
                self.flush_ref_writebacks(env, *cur);
            }
            // PHP 8 makes `throw` an expression; MWL keeps that grammar and
            // lowers only the statement position, which is the one place it
            // has a block to seal. `$x = throw …;` falls through to
            // `lower_expr`'s own unsupported-shape panic.
            ExprKind::Throw(inner) => self.lower_throw(inner, env, cur),
            // ADR 0053 § 4's suspension point. Only the statement position
            // is lowered, for `throw`'s reason above: `yield` produces
            // nothing a surrounding expression could consume (§ 5 gives a
            // generator no `send()`), so there is no other position worth
            // having.
            ExprKind::Yield {
                key: None,
                value: Some(v),
            } => self.lower_yield(v, env, cur),
            // ADR 0021's statement form, lowered to nothing. The `require`
            // graph is walked at compile time (`mwl_hir::resolve_program`),
            // so by the time this runs the target's declarations are already
            // in the same `crate::ir::Program` as this file's and there is
            // nothing left for the site to do. The value form —
            // `$c = require 'config.mwl';`, § 3's `mixed` — is a separate
            // question and still a gap; see the crate docs.
            ExprKind::Require { .. } => {}
            // ADR 0036 § 2's parenthesized reading, and every other one.
            // `mwl_syntax`'s `parse_statement_inner` commits a
            // statement-initial `{` to a *block*, so a discarded shape
            // literal has to be written `({a: 1});` — which arrives here
            // wrapped. Parentheses say nothing about what a statement means,
            // so this unwraps and dispatches again rather than duplicating
            // any arm above.
            // ADR 0007 § 4's `± 1`, over the target's own numeric type. Both
            // spellings are the same statement — see `Self::lower_incdec_stmt`
            // for why the prefix/postfix distinction has nothing to say here.
            ExprKind::PreIncDec { op, expr: target }
            | ExprKind::PostIncDec { op, expr: target } => {
                self.lower_incdec_stmt(e, *op, target, env, cur);
            }
            ExprKind::Paren(inner) => self.lower_expr_stmt(inner, env, cur),
            // A value built and immediately discarded. `new` above is the
            // same shape and the same one line of accounting: the literal is
            // a fresh producer, so this frame owns the only reference to it
            // and releases it here. Worth lowering at all only because § 2
            // names it — the statement is a no-op with an allocation in it,
            // and the checker has already reported anything wrong inside.
            ExprKind::ObjectLiteral(_) => {
                let (v, ty) = self.lower_expr(e, None, env, cur);
                if ty.is_refcounted() {
                    self.emit_release(*cur, v);
                }
            }
            other => panic!(
                "mwl-ir's control-flow slice only lowers a plain `$x = expr;` reassignment, a \
                 bare call/`new`, or a discarded shape literal as an expression statement — \
                 got {other:?}; see the crate docs' known gaps"
            ),
        }
    }
    /// `$x ⊕= e;` — rewritten into the `$x = $x ⊕ e;` it means and handed
    /// straight to [`Self::lower_reassignment`], so every target that crate
    /// can already assign to (a local, a compile-time-known property, an
    /// array element) gains its compound form at once, with that function's
    /// retain-before-release ordering unchanged. [`AssignOp::binary_op`] is
    /// the one home of the operator pairing; `mwl_types::expr`'s
    /// `check_compound_assign` types the same rewrite, so the synthesized
    /// [`ExprKind::Binary`] node carries the *assignment's* own span — the
    /// span the checker recorded any [`ExprInfo`] under (`??=` records an
    /// [`ExprInfo::Coalesce`] there) — while both operands keep their real
    /// ones.
    ///
    /// The rewrite lowers the target expression twice, once as the read and
    /// once as the write, which is why [`is_reevaluable_target`] gates it:
    /// for a local, `$this`, or a property/element path over those, a second
    /// evaluation is the same load and observes the same value, but
    /// `f()->count += 1` would call `f()` twice where PHP calls it once.
    ///
    /// **`.=` on a plain `string` local is the one exception**, and it takes
    /// [`Self::lower_string_append`] instead. The rewrite is correct for it —
    /// it is what this function did until the append existed — but its
    /// `InstKind::Concat` can only build a fresh buffer and copy the whole
    /// accumulation into it, so `$out .= $piece` in a loop is quadratic in the
    /// number of appends. Every other target keeps the rewrite, because a
    /// property or an element already needs the write-back the rewrite
    /// performs; see [`InstKind::StrAppend`].
    ///
    /// # Panics
    ///
    /// Panics naming the target when it is not one [`is_reevaluable_target`]
    /// accepts.
    pub(super) fn lower_compound_assignment(
        &mut self,
        e: &Expr,
        op: BinaryOp,
        target: &Expr,
        value: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        if op == BinaryOp::Concat
            && let ExprKind::Variable(name_span) = &target.kind
        {
            let name = strip_sigil(span_text(self.src, *name_span)).to_owned();
            // A `Ty::Ref` binding (`&$x`) names the caller's slot rather than
            // an SSA value, so it is not a holder this can re-point; it falls
            // through to the rewrite, which stores through the address.
            if let Some(&(current, Ty::Str)) = env.get(&name) {
                self.lower_string_append(name, current, value, env, cur);
                return;
            }
        }
        self.lower_read_modify_write(e.span, target, op, Some(value), env, cur);
    }
    /// `$x++;` / `--$x;` — ADR 0007 § 4's `± 1` over the target's own numeric
    /// type, through the same read-modify-write `$x += 1;` takes.
    ///
    /// **Prefix and postfix are the same statement.** The two differ only in
    /// which of the read-modify-write's two values the surrounding
    /// *expression* sees, and an expression statement sees neither — so this
    /// does not distinguish them, and `tests/conformance/lang/`'s
    /// `an-increment-answers-the-same-in-either-position` is what holds them
    /// together. An increment used *as a value* (`$y = $x++;`) is the same
    /// unlowered shape a nested assignment (`$y = ($x = 5);`) is, and for the
    /// same reason: [`Self::lower_expr`] is handed an `&Env` and so has no
    /// binding it could re-point.
    pub(super) fn lower_incdec_stmt(
        &mut self,
        e: &Expr,
        op: IncDecOp,
        target: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        let op = match op {
            IncDecOp::Inc => BinaryOp::Add,
            IncDecOp::Dec => BinaryOp::Sub,
        };
        self.lower_read_modify_write(e.span, target, op, None, env, cur);
    }
    /// `$t ⊕= e;` and `$t++;` alike: read the target, combine, write it back,
    /// answering `(old, new)` for a position that wants one of them.
    ///
    /// The combine is still the `$t = $t ⊕ e` rewrite — this crate has one
    /// binary-operator lowering and it takes an [`Expr`] — but the target's
    /// **address** is computed before the rewrite is built rather than by it:
    /// every sub-expression that cannot be re-read is lowered once and staged
    /// ([`Self::stage_target_address`]), so `Box::make()->count += 1` calls
    /// `make()` once where the rewrite writes its receiver down twice. The
    /// read itself is staged the same way, which is also how an increment
    /// learns the representation to emit its `1` at.
    ///
    /// # Panics
    ///
    /// Panics naming the target when it is not one
    /// [`Self::reevaluable_target`] accepts even after staging — a nullsafe
    /// path, or a call in a position staging does not reach.
    pub(super) fn lower_read_modify_write(
        &mut self,
        span: Span,
        target: &Expr,
        op: BinaryOp,
        rhs: Option<&Expr>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty, ValueId, Ty) {
        let temporaries = self.temporaries_mark();
        let addresses = self.staged_mark();
        self.stage_target_address(target, env, cur);
        assert!(
            self.reevaluable_target(target),
            "mwl-ir lowers a compound assignment by rewriting it to `$x = $x op e`, which reads \
             the target twice, so its target must be a local, `$this`, or a property/element \
             path over those — got {:?}; see the crate docs' known gaps",
            target.kind
        );
        let reads = self.staged_mark();
        let (old, old_ty) = self.lower_expr(target, None, env, cur);
        self.stage(target.span, old, old_ty);
        // An increment's `1` has no source span to build an `ExprKind::Int`
        // from, so it is emitted here — at the representation the read just
        // reported, since ADR 0007 § 4 gives `int ⊕ int` and `uint ⊕ uint`
        // their own rows and refuses the mixed pair — and staged like the rest
        // of the address. `one` exists only to outlive the borrow below.
        let one;
        let rhs = match rhs {
            Some(rhs) => rhs,
            None => {
                let span = self.synthetic_span();
                let v = self.emit_const_one(*cur, old_ty);
                self.stage(span, v, old_ty);
                one = Expr {
                    kind: ExprKind::Int(span),
                    span,
                };
                &one
            }
        };
        let combined = Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(target.clone()),
                rhs: Box::new(rhs.clone()),
            },
            span,
        };
        let (new, new_ty) = self.lower_expr(&combined, Some(old_ty), env, cur);
        // The read and the `1` are dropped before the write: the store lowers
        // the target's own sub-expressions again — which is what keeps a plain
        // `=` emitting exactly what it always did — and those are the entries
        // that have to still be standing when it does.
        self.unstage_to(reads);
        self.lower_store(target, &Stored::Value(new, new_ty), env, cur);
        self.unstage_to(addresses);
        self.release_temporaries_since(temporaries, *cur);
        // `$x += Foo::bar($n);` — the right-hand side may have staged a `&$n`
        // argument, exactly as a plain assignment's does. See
        // `Self::pending_refs`.
        self.flush_ref_writebacks(env, *cur);
        (old, old_ty, new, new_ty)
    }
    /// The `1` an increment adds or subtracts, at the target's own
    /// representation.
    ///
    /// ADR 0007 § 4 gives each numeric type its own arithmetic row and
    /// refuses a mixed-signedness pair outright, so the literal is emitted as
    /// the operand it will be paired with rather than as a default `int`.
    ///
    /// Anything else takes `int`, which is what a written `$x += 1` puts on
    /// the right — so a target `mwl_types` could not pin down (a `mixed`, a
    /// `?int`) is refused in the same place, and with the same message, that
    /// spelling is already refused in. Every target it *can* pin down and that
    /// is not numeric is E0474 and never reaches here.
    fn emit_const_one(&mut self, cur: BlockId, ty: Ty) -> ValueId {
        let (ty, kind) = match ty {
            Ty::Uint => (Ty::Uint, InstKind::ConstUint(1)),
            Ty::Float => (Ty::Float, InstKind::ConstFloat(1.0)),
            Ty::Decimal => (
                Ty::Decimal,
                InstKind::ConstDecimal {
                    negative: false,
                    mantissa: 1,
                    scale: 0,
                },
            ),
            _ => (Ty::Int, InstKind::ConstInt(1)),
        };
        self.emit(cur, ty, kind).0
    }
    /// Lowers, once, every sub-expression of `target` the `$t = $t ⊕ e`
    /// rewrite would otherwise evaluate twice, and stages the results — see
    /// [`Self::staged_targets`].
    ///
    /// Only what [`Self::reevaluable_target`] refuses is staged, so a target
    /// that already lowered before this existed emits exactly the instructions
    /// it emitted then. A refcounted one goes on [`Self::owned_temporaries`]:
    /// it is a fresh producer with no other owner — that is precisely why it
    /// could not be re-read — so this frame owes its release, on the throwing
    /// edge as much as the normal one.
    fn stage_target_address(&mut self, target: &Expr, env: &mut Env, cur: &mut BlockId) {
        match &target.kind {
            ExprKind::PropertyAccess { object, .. } => self.stage_address_of(object, env, cur),
            ExprKind::Index { base, index } => {
                self.stage_address_of(base, env, cur);
                if let Some(index) = index
                    && !self.pure_key(index)
                {
                    self.stage_value_of(index, env, cur);
                }
            }
            _ => {}
        }
    }
    /// One receiver or base: staged when it cannot be re-read, walked into
    /// when it can — so a `$this->a->b` path stages nothing at all.
    fn stage_address_of(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        if self.reevaluable_target(e) {
            self.stage_target_address(e, env, cur);
        } else {
            self.stage_value_of(e, env, cur);
        }
    }
    fn stage_value_of(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        let aliasing = self.aliasing_read(e);
        let (v, ty) = self.lower_expr(e, None, env, cur);
        if ty.is_refcounted() && !aliasing {
            self.own_temporary(v);
        }
        self.stage(e.span, v, ty);
    }

    /// `$s .= e;` where `$s` is a plain `Ty::Str` local — one
    /// [`InstKind::StrAppend`] that appends into `$s`'s own buffer whenever
    /// nothing else holds it, rather than the rewrite's `Concat` copying the
    /// whole accumulation into a fresh one.
    ///
    /// **No retain and no release of either operand**, which is
    /// [`Self::write_back_array`]'s bookkeeping exactly: the instruction
    /// consumes the local's one reference and yields the one that replaces it
    /// in the same `Env` slot, so `env.insert` is the entire write-back.
    /// [`InstKind::StrAppend`]'s own doc comment owns that protocol.
    ///
    /// The suffix goes through [`Self::concat_operand`], so a scalar is
    /// converted to a `string` first and staged on
    /// [`Self::owned_temporaries`] when no durable slot owns it — the same
    /// treatment, and the same throwing-edge safety, [`Self::lower_concat`]
    /// gives its two operands.
    fn lower_string_append(
        &mut self,
        name: String,
        target: ValueId,
        value: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        let mark = self.temporaries_mark();
        let (suffix, aliasing) = self.concat_operand(value, env, cur);
        if !aliasing {
            self.own_temporary(suffix);
        }
        let (appended, _) = self.emit(*cur, Ty::Str, InstKind::StrAppend { target, suffix });
        self.release_temporaries_since(mark, *cur);
        env.insert(name, (appended, Ty::Str));
    }
    /// `$x = expr;` or `$obj->prop = expr;` as a bare expression statement —
    /// SSA renaming needs no join logic here, only a fresh binding in `env`
    /// (a local target) or a [`InstKind::FieldSet`] (a property target).
    pub(super) fn lower_reassignment(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        let ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
            by_ref: false,
        } = &e.kind
        else {
            unreachable!("Self::lower_expr_stmt only routes a plain `AssignOp::Assign` here");
        };
        self.lower_store(target, &Stored::Expr(value), env, cur);
        // `$x = Foo::bar($n);` — the right-hand side may have staged a `&$n`
        // argument, whose copy-back belongs to this statement. See
        // `Self::pending_refs`.
        self.flush_ref_writebacks(env, *cur);
    }
    /// The write half of [`Self::lower_reassignment`]: everything about
    /// *where* the value lands, with what lands there left to [`Stored`].
    ///
    /// Split out because a read-modify-write has no right-hand-side
    /// expression to hand this — its value is already in a register by the
    /// time the store runs ([`Self::lower_read_modify_write`]). The target's
    /// own sub-expressions are still lowered *here*, at the point in the
    /// evaluation order they have always been lowered at, so a plain `=` emits
    /// exactly the instructions it did before the split; for the
    /// read-modify-write path they are staged
    /// ([`Self::staged_targets`]) and this second lowering costs nothing and
    /// runs nothing twice.
    pub(super) fn lower_store(
        &mut self,
        target: &Expr,
        stored: &Stored<'_>,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        match &target.kind {
            ExprKind::Variable(name_span) => {
                let lname = strip_sigil(span_text(self.src, *name_span)).to_owned();
                // A `&$x` parameter names the caller's staged slot, not an SSA
                // binding: the write is a store through the address, so SSA
                // renaming has nothing to do and `env` is left alone. The
                // retain/load-old/release/store sequence is `Self::bind_local`'s
                // own policy against a slot instead of an `Env` entry — see
                // `Ty::Ref` and `InstKind::RefStore`.
                if let Some(&(slot, Ty::Ref)) = env.get(&lname) {
                    let pointee = self.pointee_of(&lname);
                    let (v, _, aliasing) = self.lower_stored(stored, Some(pointee), env, cur);
                    if pointee.is_refcounted() && aliasing {
                        self.emit_retain(*cur, v);
                    }
                    if pointee.is_refcounted() {
                        let (old_v, _) = self.emit(*cur, pointee, InstKind::RefLoad { slot });
                        self.emit_release(*cur, old_v);
                    }
                    self.emit_ref_store(*cur, slot, v);
                } else {
                    let expected = env.get(&lname).map(|&(_, t)| t);
                    let (v, ty, aliasing) = self.lower_stored(stored, expected, env, cur);
                    // ADR 0037 fixes a local's type at its declaration, so an
                    // existing binding's representation wins over whatever the
                    // right-hand side produced -- otherwise a `?int` local
                    // reassigned an `int` would silently change shape, and the
                    // next phi over it would merge two representations.
                    let (v, ty) = match expected {
                        Some(want) => (self.coerce(*cur, v, ty, want, env), want),
                        None => (v, ty),
                    };
                    self.bind_local_value(*cur, env, lname, v, ty, aliasing);
                }
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
                // A `set` hook (ADR 0014 § 1) makes the write a call, exactly
                // the way a `get` hook makes the read one — same receiver
                // slot, same ownership convention, and the assigned value as
                // the accessor's one ordinary argument. A property with only
                // a `get` hook still writes its own slot: MWL's hooked
                // properties are always backed, so there is a slot to write
                // (`mwl_types::signatures::PropertyHooks` owns that
                // decision).
                //
                // An ADR 0036 § 4 shape target is neither: it has no
                // declaring class to name and no hook to call, so it takes
                // the name-keyed write its own read mirrors and leaves before
                // the class machinery below.
                if let Some(ExprInfo::ShapeProperty { name, slot, ty }) =
                    self.exprs.lookup(target.span)
                {
                    let field = super::expr::ShapeField {
                        name: name.clone(),
                        slot: *slot,
                        ty: *ty,
                    };
                    self.lower_shape_property_assign(object, &field, stored, env, cur);
                    return;
                }
                let (class, name, ty, set) = match self.exprs.lookup(target.span) {
                    Some(ExprInfo::Property { class, name, ty }) => (class, name, *ty, None),
                    Some(ExprInfo::HookedProperty {
                        class,
                        name,
                        ty,
                        set,
                        ..
                    }) => (class, name, *ty, set.clone()),
                    _ => panic!(
                        "mwl-ir: a property assignment target at {:?} has no resolved declaring \
                         class recorded in the typed-expression table — either it wasn't checked \
                         with the same table, or its receiver erased to a plain `object`, which \
                         ADR 0036 § 4's erased half still does not lower (see the crate docs' \
                         known gaps)",
                        target.span
                    ),
                };
                let field_ty = lower_checked_ty(ty, self.checked_types);
                let class_label = class.to_string();
                let field_name = name.clone();
                if let Some(label) = set {
                    let (object_v, receiver_ty) = self.lower_expr(object, None, env, cur);
                    // A narrowed `?T` receiver arrives tagged — see
                    // `Lowering::untag_receiver`, which the read side reaches
                    // through `open_nullsafe`.
                    let (object_v, receiver_ty) = self.untag_receiver(object_v, receiver_ty, *cur);
                    if receiver_ty.is_refcounted() && self.aliasing_read(object) {
                        self.emit_retain(*cur, object_v);
                    }
                    let (v, vty, aliasing) = self.lower_stored(stored, Some(field_ty), env, cur);
                    if field_ty.is_refcounted() && aliasing {
                        self.emit_retain(*cur, v);
                    }
                    let v = self.coerce(*cur, v, vty, field_ty, env);
                    self.emit_fallible(
                        *cur,
                        Ty::Void,
                        InstKind::Call {
                            target: label,
                            receiver: Some(object_v),
                            args: vec![v],
                        },
                        env,
                    );
                } else {
                    let (object_v, receiver_ty) = self.lower_expr(object, None, env, cur);
                    let (object_v, _) = self.untag_receiver(object_v, receiver_ty, *cur);
                    let (v, vty, aliasing) = self.lower_stored(stored, Some(field_ty), env, cur);
                    if field_ty.is_refcounted() && aliasing {
                        self.emit_retain(*cur, v);
                    }
                    let v = self.coerce(*cur, v, vty, field_ty, env);
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
            //
            // A *nested* target (`$grid[0][1] = v`) is the same write one
            // level down, and the separation has to run at every level: the
            // target is flattened to its root and one key per level, the root
            // and each key are lowered exactly once (PHP evaluates neither
            // twice), the chain is descended with `InstKind::ArrayGet`, and
            // the `ArraySet`s are then emitted back up with the *outermost*
            // last, so the value handed to `Self::write_back_array` is the
            // root array that now holds every re-pointed row. Each key is used
            // twice — borrowed by the `ArrayGet`, stored by the `ArraySet` —
            // but only the store takes a reference, so the single-level
            // `if key_aliasing { retain }` still applies exactly once per key.
            // The descent itself is `Helper::ArrayRowForWrite` rather than an
            // `InstKind::ArrayGet`, because a row has to arrive owning a
            // reference (the `ArraySet` below it consumes one) *and* an
            // absent key has to auto-vivify the way PHP's does — see that
            // helper's own doc comment, and this crate's module docs for what
            // the resulting refcount means for the copy.
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
                // `$grid[0][1]` flattens to the root `$grid` and the levels
                // `$grid[0]` (whose key is `0`) and the target itself (whose
                // key is `1`, and which is not in `levels`).
                let mut levels: Vec<&Expr> = Vec::new();
                let mut root = base;
                while let ExprKind::Index {
                    base: inner,
                    index: Some(_),
                } = &root.kind
                {
                    levels.push(root);
                    root = inner;
                }
                levels.reverse();
                let (root_v, _) = self.lower_expr(root, None, env, cur);
                // Every key, then the value: left to right, each exactly
                // once, and every retain deferred until nothing that can
                // throw is left to lower.
                let mut inner_keys: Vec<(ValueId, bool)> = Vec::with_capacity(levels.len());
                for level in &levels {
                    let ExprKind::Index {
                        index: Some(key), ..
                    } = &level.kind
                    else {
                        unreachable!("only an `Index` carrying a subscript is pushed onto `levels`")
                    };
                    let (key_v, _key_ty, key_aliasing) = self.lower_array_key(key, env, cur);
                    inner_keys.push((key_v, key_aliasing));
                }
                let outer_key = match index {
                    None => None,
                    Some(index) => {
                        let (key_v, _key_ty, key_aliasing) = self.lower_array_key(index, env, cur);
                        Some((key_v, key_aliasing))
                    }
                };
                let (v, _, aliasing) = self.lower_stored(stored, Some(elem_ty), env, cur);
                for &(key_v, key_aliasing) in &inner_keys {
                    if key_aliasing {
                        self.emit_retain(*cur, key_v);
                    }
                }
                if let Some((key_v, true)) = outer_key {
                    self.emit_retain(*cur, key_v);
                }
                if elem_ty.is_refcounted() && aliasing {
                    self.emit_retain(*cur, v);
                }
                // Down the chain. Each row arrives owning one reference —
                // `Helper::ArrayRowForWrite`'s whole job, since the
                // `ArraySet` below consumes one and a plain
                // `InstKind::ArrayGet` would both borrow and answer an absent
                // key with a null.
                let mut arrays: Vec<ValueId> = Vec::with_capacity(levels.len() + 1);
                arrays.push(root_v);
                for (level, &(key_v, _)) in levels.iter().zip(&inner_keys) {
                    let row_ty = self.row_ty_of(level);
                    let array = *arrays.last().expect("pushed the root above");
                    let (row, _) = self.emit(
                        *cur,
                        row_ty,
                        InstKind::HelperCall {
                            helper: Helper::ArrayRowForWrite,
                            args: vec![array, key_v],
                        },
                    );
                    arrays.push(row);
                }
                // And back up, innermost first, so each level is handed the
                // separated array the level below just produced.
                let innermost = *arrays.last().expect("pushed the root above");
                let mut written = match outer_key {
                    None => self.emit_array_append(*cur, innermost, v, env),
                    Some((key_v, _)) => self.emit_array_set(*cur, innermost, key_v, v),
                };
                for (i, &(key_v, _)) in inner_keys.iter().enumerate().rev() {
                    written = self.emit_array_set(*cur, arrays[i], key_v, written);
                }
                self.write_back_array(root, written, env, cur);
            }
            other => panic!(
                "mwl-ir's control-flow slice only lowers reassignment to a plain local, a \
                 compile-time-known property, or a compile-time-known array element, not \
                 {other:?}"
            ),
        }
    }
    /// The representation of one intermediate level of a nested
    /// array-element write target — `$grid[0]` in `$grid[0][1] = v` — read
    /// out of the typed-expression table exactly as the write's own element
    /// type is, since the checker records an [`ExprInfo::Index`] entry per
    /// `Index` node whether it was reached as a read or as a target.
    ///
    /// # Panics
    ///
    /// Panics when the level has no entry (its base erased to `mixed`), and
    /// when its element type is not an array — a level that is being
    /// subscripted again has to be one, so anything else means the checker
    /// accepted a target this crate has no separation rule for.
    fn row_ty_of(&self, level: &Expr) -> Ty {
        let Some(ExprInfo::Index { elem_ty }) = self.exprs.lookup(level.span) else {
            panic!(
                "mwl-ir: an intermediate level of a nested array-index assignment target at \
                 {:?} has no resolved element type recorded in the typed-expression table — \
                 either it wasn't checked with the same table, or its base erased to `mixed` \
                 (an unresolved array), which this crate does not yet lower (see the crate \
                 docs' known gaps)",
                level.span
            );
        };
        let row_ty = lower_checked_ty(*elem_ty, self.checked_types);
        assert!(
            row_ty == Ty::Array,
            "mwl-ir: an intermediate level of a nested array-index assignment target at {:?} \
             lowered to {row_ty:?} rather than an array, so there is nothing for the level \
             above it to write back into — mwl_types::check_program is trusted to have \
             rejected subscripting a non-array",
            level.span
        );
        row_ty
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
    pub(super) fn lower_unset(&mut self, target: &Expr, env: &mut Env, cur: &mut BlockId) {
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
        let (key_v, key_aliasing) = self.lower_rendered_array_key(index, env, cur);
        let (written, _) = self.emit(
            *cur,
            Ty::Array,
            InstKind::ArrayUnset {
                array: array_v,
                key: key_v,
            },
        );
        if !key_aliasing {
            self.emit_release(*cur, key_v);
        }
        self.write_back_array(base, written, env, cur);
    }
    /// Whether lowering `e` twice observes the same value and runs no side
    /// effect the second time — the precondition
    /// [`Self::lower_read_modify_write`]'s rewrite needs, since `$t ⊕= e`
    /// becomes `$t = $t ⊕ e` with the target appearing on both sides.
    ///
    /// A **staged** sub-expression qualifies whatever it was written as: its
    /// second lowering is a lookup in [`Self::staged_targets`] and runs
    /// nothing at all, which is the whole reason that table exists. Beyond
    /// that, a local read (`$this` is one, spelled as an ordinary variable)
    /// and a property or element path built over those are the shapes that
    /// qualify: each is a load, and a `get` hook (ADR 0014 § 1) still runs
    /// exactly once because the write side of a property assignment never
    /// reads its own target back through the hook. A nullsafe path, or
    /// anything else that can run user code where staging did not reach it, is
    /// refused rather than silently duplicated.
    fn reevaluable_target(&self, e: &Expr) -> bool {
        if self.staged(e.span).is_some() {
            return true;
        }
        match &e.kind {
            ExprKind::Variable(_) => true,
            ExprKind::PropertyAccess {
                object,
                nullsafe: false,
                ..
            } => self.reevaluable_target(object),
            ExprKind::Index { base, index } => {
                self.reevaluable_target(base) && index.as_ref().is_none_or(|i| self.pure_key(i))
            }
            _ => false,
        }
    }
    /// Whether an array key expression can be lowered twice — the same
    /// question [`Self::reevaluable_target`] asks of a target, widened by the
    /// literal forms a key is usually written as.
    fn pure_key(&self, e: &Expr) -> bool {
        matches!(e.kind, ExprKind::Int(_) | ExprKind::Str(_)) || self.reevaluable_target(e)
    }
}
