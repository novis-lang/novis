//! Statement lowering — the dispatch every statement kind goes through, plus assignment and `unset`.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. The methods
//! are `pub(crate)`, so they reach across these modules and no further.

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
    pub(crate) fn lower_script_stmts(
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
                // `rule:programs/no-runtime-autoload`'s `autoload` map is read by `nvs_hir` while the
                // `require`/autoload graph is being built, long before any of
                // this — so at file scope it is a declaration like the rest of
                // this list and emits nothing, in the entry point exactly as
                // in the bootstrap file a conformance case usually puts it.
                | StmtKind::AutoloadDecl(_)
                | StmtKind::TopLevelFunction(_)
                | StmtKind::TopLevelConst(_) => {}
                _ => self.lower_stmt(stmt, cur, env),
            }
        }
    }
    /// Lowers a statement list into `cur`, stopping early once `cur` is
    /// sealed (dead code after a `return` inside the list is simply never
    /// lowered — nothing downstream needs it modeled).
    pub(crate) fn lower_stmts(&mut self, stmts: &'a [Stmt], cur: &mut BlockId, env: &mut Env) {
        for stmt in stmts {
            if self.is_terminated(*cur) {
                break;
            }
            self.lower_stmt(stmt, cur, env);
            // Every `inout $x` argument is copied back at its own call, so the
            // list is empty again by the time the statement ends. A leftover
            // means some call site lowered an argument list without flushing
            // its own `pending_refs_mark` — an internal inconsistency rather
            // than an unsupported program. See `Self::pending_refs`.
            assert!(
                self.pending_refs.is_empty(),
                "nvs-ir: an `inout $x` argument staged inside the statement at {:?} was never \
                 copied back — every site that lowers an argument list is expected to flush \
                 its own staging mark once its call has returned; see \
                 `lower::Lowering::pending_refs`",
                stmt.span
            );
        }
    }
    pub(crate) fn lower_stmt(&mut self, stmt: &'a Stmt, cur: &mut BlockId, env: &mut Env) {
        let stmt_id = self.ids.next_stmt(stmt.span);
        self.cur_stmt_span = stmt.span;
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::StmtMarker(stmt_id),
            on_error: None,
            raise_site: None,
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
            // `rule:types/var-inference`: `var $x = expr;` — no declared type at all, so there
            // is no `expected` to check `value` against. `lower_expr` already
            // synthesizes a type from the expression alone whenever `expected`
            // is `None` (a bare integer literal defaults to `Ty::Int`, etc.) —
            // exactly the same rule `nvs_types::locals::check_stmt`'s own
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
            // `int $x;` — a declaration with no initializer, which `rule:types/declaration` makes a complete statement: the type is fixed here and the
            // value arrives on some later line. There is nothing to emit,
            // because `nvs_types::locals`' definite-assignment pass is what
            // guarantees no path reads the name before an assignment reaches
            // it (reading one that may not have is `E0403`) — so the whole
            // statement *is* the type, remembered for the assignment that
            // will bind it. See `Lowering::declared_tys` for why remembering
            // it is not optional.
            StmtKind::LocalDecl {
                ty: Some(decl_ty),
                name: local_name,
                value: None,
            } => {
                let declared = lower_decl_type(decl_ty, self.exprs, self.checked_types);
                let lname = strip_sigil(span_text(self.src, *local_name)).to_owned();
                self.declared_tys.insert(lname, declared);
            }
            // A bare `;`. PHP parses one wherever a statement is legal and it
            // does nothing there; so does this, and the `StmtMarker` above
            // has already given it the source position a debugger would want.
            StmtKind::Empty => {}
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
            // `rule:iteration/one-way-only`: a generator's body has no return value, so
            // `return;` means "the sequence ends here" — the same exit
            // running off the end takes. `nvs_types` reports E0447 for a
            // `return expr;` in one, which is why this ignores `value`
            // rather than lowering it.
            StmtKind::Return(_) if self.generator.is_some() => {
                self.run_pending_finallys(cur, env);
                if !self.is_terminated(*cur) {
                    self.finish_generator(*cur, env);
                }
            }
            StmtKind::Return(value) => {
                // `return $local;` hands the binding's own reference straight
                // out rather than retaining it here and releasing it below —
                // `release_all_locals` skips the name instead. An `inout $x`
                // parameter is the one binding that cannot play: it is a
                // `Ty::Ref` cell, so `release_all_locals` was never going to
                // release it (the caller's copy-back owns that reference), and
                // exempting it would only lose the retain the read below owes,
                // handing the caller a value with no owner at all. See
                // `Ty::Ref` and `Lowering::pending_refs`.
                let except = value.as_ref().and_then(|v| {
                    if let ExprKind::Variable(span) = &v.kind {
                        let name = strip_sigil(span_text(self.src, *span)).to_owned();
                        match env.get(&name) {
                            Some(&(_, Ty::Ref)) => None,
                            _ => Some(name),
                        }
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
            StmtKind::If { arms, else_ } => {
                self.lower_if(arms, else_.as_deref(), cur, env);
            }
            StmtKind::While { cond, body } => self.lower_while(cond, body, cur, env),
            StmtKind::DoWhile { body, cond } => self.lower_do_while(body, cond, cur, env),
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
                value_inout,
                body,
            } => self.lower_foreach(subject, key.as_ref(), value, *value_inout, body, cur, env),
            StmtKind::Switch { subject, cases } => self.lower_switch(subject, cases, cur, env),
            // `rule:classes/unset-is-refused-on-a-property` leaves exactly one `unset` target standing — an
            // array element — and `nvs_types::expr::check_unset_target`
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
            StmtKind::InlineHtml(span) => self.lower_inline_html(*span, cur, env),
            StmtKind::Destructure { target, value } => {
                self.lower_destructure(target, value, cur, env);
            }
            // `rule:types/grammar`.3's destructuring lowers one arm above, and
            // what is left is the roster: every `StmtKind` a body can hold is
            // spelled out here, and each remaining shape is one the front end
            // already refuses where it is written. The arm names the code that
            // does the refusing rather than claiming a lowering gap, so the
            // guarantee is checked against a conformance case expecting it
            // (`crates/nvs-ir/tests/refusals.rs`).
            //
            // `var $x;` — `var` reads its type off the initializer and has
            // nothing else to read one off, so the parser requires the `=`
            // where it would be (`rule:types/var-inference`). The `LocalDecl`
            // shapes that carry a type, a value or both lower above.
            StmtKind::LocalDecl {
                ty: None,
                value: None,
                ..
            } => guarded_by!(
                code::E_EXPECTED_TOKEN,
                "`var` with no initializer has no type to lower — the parser requires the `=`"
            ),
            // `global`, `goto` and a function-scope `static` are each reported
            // at the statement the parser built, so none reaches a compilation
            // that lowers (`rule:statements/no-function-static-and-no-global`
            // for the two storage shapes).
            StmtKind::Global(_) => guarded_by!(
                code::E_GLOBAL_UNSUPPORTED,
                "`global` reaches outside its own frame, which nothing that lowers does"
            ),
            StmtKind::StaticLocal { .. } => guarded_by!(
                code::E_STATIC_LOCAL_UNSUPPORTED,
                "a function-scope `static` is not a storage class this compiler has"
            ),
            StmtKind::Goto(_) => guarded_by!(
                code::E_GOTO_UNSUPPORTED,
                "`goto` names an edge no structured control-flow graph carries"
            ),
            // A free function or constant is refused at every scope by
            // `nvs_hir::members` (`rule:classes/no-free-functions-or-constants`).
            StmtKind::TopLevelFunction(_) => guarded_by!(
                code::E_TOPLEVEL_FUNCTION_UNSUPPORTED,
                "every function is a method, so a free one never reaches a body"
            ),
            StmtKind::TopLevelConst(_) => guarded_by!(
                code::E_TOPLEVEL_CONST_UNSUPPORTED,
                "every constant belongs to a class, so a free one never reaches a body"
            ),
            // The declarations. At file scope `lower_script_stmts` above skips
            // them; anywhere else `nvs_types::locals`' walk, which is reached
            // only from inside a body, reports this code. The decision is in
            // `docs/adr/README.md` § *Decisions taken at project start*, since
            // PHP's "declared when the statement runs" has no reading a static
            // table built before any code runs can give it.
            StmtKind::ClassDecl(_)
            | StmtKind::InterfaceDecl(_)
            | StmtKind::EnumDecl(_)
            | StmtKind::NamespaceDecl(_)
            | StmtKind::UseDecl(_)
            | StmtKind::AutoloadDecl(_)
            | StmtKind::TypeAliasDecl(_) => guarded_by!(
                code::E_NESTED_TYPE_DECLARATION_UNSUPPORTED,
                "a declaration inside a body is refused there — only file scope carries one"
            ),
            // The last two are engine invariants rather than refusals, so
            // neither has a code to name. `Error` is error recovery's
            // placeholder, and a compilation that reported a parse error never
            // reaches lowering. The wildcard reaches no variant at all: it is
            // there because `StmtKind` is `#[non_exhaustive]`, which makes Rust
            // ask for one however many arms a cross-crate match spells out.
            StmtKind::Error => unreachable!(
                "a parse error was reported before lowering, so its recovery placeholder \
                 never reaches this dispatch"
            ),
            _ => unreachable!(
                "every `StmtKind` has an arm above; this one is `#[non_exhaustive]`'s, and \
                 nothing the parser builds occupies it"
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
    /// Every arm below is a shape with accounting of its own; anything else
    /// is evaluated for its effects and discarded by the last arm, so this
    /// dispatch refuses nothing — an expression with no lowering is named by
    /// [`Self::lower_expr`]'s own dispatch instead, which points at the
    /// expression rather than at the statement wrapping it.
    pub(crate) fn lower_expr_stmt(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
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
                self.lower_compound_assignment(e, binop, op.defaults(), target, value, env, cur);
            }
            ExprKind::MethodCall { .. } | ExprKind::StaticCall { .. } | ExprKind::New { .. } => {
                let (v, ty) = self.lower_expr(e, None, env, cur);
                if ty.is_refcounted() {
                    self.emit_release(*cur, v);
                }
            }
            // PHP 8 makes `throw` an expression, and Novis keeps that grammar in
            // both positions: this arm is the statement one, where the sealed
            // block *is* the end of the statement, and `lower_expr`'s own
            // `Throw` arm is `$n ?? throw …`, which adds the unreachable
            // continuation block an expression's caller still writes into.
            ExprKind::Throw(inner) => self.lower_throw(inner, env, cur),
            // `exit;` / `exit(…);` — the whole construct is the one helper
            // call `Self::lower_exit` emits. The `never`-typed value it hands
            // back is a dead `ConstInt`, and `Ty::Int` is not refcounted, so
            // there is nothing to discard here.
            ExprKind::Exit(arg) => {
                self.lower_exit(arg.as_deref(), env, cur);
            }
            // `rule:iteration/generators`'s suspension point. Only the statement position
            // is lowered, for `throw`'s reason above: `yield` produces
            // nothing a surrounding expression could consume (§ 5 gives a
            // generator no `send()`), so there is no other position worth
            // having.
            ExprKind::Yield {
                key: None,
                value: Some(v),
            } => self.lower_yield(v, env, cur),
            // `rule:statements/require-is-the-only-inclusion-construct`'s statement form: a call to the required file's own
            // script frame. The graph is walked at compile time
            // (`nvs_hir::resolve_program`), so the target's *declarations*
            // are already in this same `crate::ir::Program` and nothing here
            // resolves a name — what the site still owes is the required
            // file's own top-level statements, which run here, in source
            // order, exactly where the `require` is written.
            //
            // The frame is the file's, not this one's: declarations cross a
            // `require` and variables do not (`rule:statements/a-required-file-shares-declarations-not-locals`), which
            // is what `nvs_types::locals` already checks each file's body
            // under. It is called every time the statement is reached, PHP's
            // own answer for `require` as opposed to `require_once` — the
            // walk loads each file once, but that is a *compile*-time fact.
            //
            // A path with no target is one no file was compiled for, and the
            // site throws there (`Self::lower_unloaded_require`). A literal
            // naming nothing loadable, or closing a cycle, is a diagnostic and
            // never reaches lowering. The value form —
            // `$c = require 'config.nvs';` — is this same call with its result
            // kept, one file over in `Self::lower_expr`.
            ExprKind::Require { path } => {
                if let Some(target) = self.exprs.require_target(path.span) {
                    // The frame returns whatever the file's own `return`
                    // handed back, or § 3's `1` where it never returned at
                    // all, and the statement form reads neither — but the
                    // value is still this frame's to release, exactly as a
                    // discarded anonymous object's is.
                    let (v, ty) = self.emit_fallible(
                        *cur,
                        Ty::Tagged,
                        InstKind::Call {
                            target: crate::lower::file_script_label(target),
                            receiver: None,
                            args: Vec::new(),
                        },
                        env,
                    );
                    if ty.is_refcounted() {
                        self.emit_release(*cur, v);
                    }
                } else {
                    self.lower_unloaded_require(path, env, cur);
                }
            }
            // `rule:types/anonymous-object`'s parenthesized reading, and every other one.
            // `nvs_syntax`'s `parse_statement_inner` commits a
            // statement-initial `{` to a *block*, so a discarded shape
            // literal has to be written `({a: 1});` — which arrives here
            // wrapped. Parentheses say nothing about what a statement means,
            // so this unwraps and dispatches again rather than duplicating
            // any arm above.
            // `rule:types/arithmetic`'s `± 1`, over the target's own numeric type. Both
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
            ExprKind::AnonObject(_) => {
                let (v, ty) = self.lower_expr(e, None, env, cur);
                if ty.is_refcounted() {
                    self.emit_release(*cur, v);
                }
            }
            // `print "…";` — the same write `echo` emits, with the `1` PHP
            // answers left unread. See `Self::lower_print`.
            ExprKind::Print(operand) => {
                self.lower_print(operand, env, cur);
            }
            // `isset($a);` on its own line answers a [`Ty::Bool`] nobody
            // reads, which is why it is here rather than left to the panic:
            // its operands are still *evaluated*, so a subscript key writing
            // a local (`isset($a[$i++]);`) writes it exactly where PHP does.
            // Nothing to release — see `Self::lower_isset_operand`, which
            // already accounts for every operand it reads.
            ExprKind::Isset(operands) => {
                self.lower_isset(operands, env, cur);
            }
            // `empty($x);` — the same discarded `Ty::Bool`, and here for the
            // same reason: `Self::lower_not` still evaluates the operand, so
            // `empty($a[$i++]);` writes `$i` exactly where PHP writes it, and
            // it already releases a fresh operand itself.
            ExprKind::Empty(operand) => {
                self.lower_not(operand, env, cur);
            }
            // Every other expression, evaluated for what it does with its
            // value discarded — `$a[$i++];`, `$obj->prop;`, `$x;`, `1 + 2;`,
            // a discarded `match`. Each one runs for its effects, which is
            // the same rule the `isset`, `empty` and
            // `AnonObject` arms above already spell out one shape at a
            // time; those stay because each carries an accounting note of its
            // own, not because this could not cover them.
            //
            // Refusing an effect-free one instead — the "expression result
            // unused" a stricter language reports — is not taken, and the
            // reason is that "has no effect" is not a property this slice can
            // decide: a property read runs the hook `rule:classes/property-hooks` gives it, a
            // subscript key runs whatever the key expression does, and a call
            // is buried inside half of these. So every expression statement
            // is evaluated, and the residue that genuinely does nothing costs
            // one dead instruction the backend drops.
            //
            // The release is `Self::aliasing_read`'s judgment, not a blanket
            // one: `$x;` hands back the local's own value and releasing that
            // would be a release of a reference this frame never took, while
            // `$m->rows()["0"];` is a fresh producer whose only reference is
            // the one being discarded here. A shape with no lowering at all
            // is still refused one level down, by `lower_expr`'s own
            // dispatch, which names the expression rather than the statement
            // wrapping it.
            _ => {
                let (v, ty) = self.lower_expr(e, None, env, cur);
                if ty.is_refcounted() && !self.aliasing_read(e) {
                    self.emit_release(*cur, v);
                }
            }
        }
    }
    /// `$x ⊕= e;` — rewritten into the `$x = $x ⊕ e;` it means and handed
    /// straight to [`Self::lower_reassignment`], so every target that crate
    /// can already assign to (a local, a compile-time-known property, an
    /// array element) gains its compound form at once, with that function's
    /// retain-before-release ordering unchanged. [`AssignOp::binary_op`] is
    /// the one home of the operator pairing; `nvs_types::expr`'s
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
    /// [`Self::lower_string_append`] instead. The rewrite would be correct for
    /// it, but its
    /// `InstKind::Concat` can only build a fresh buffer and copy the whole
    /// accumulation into it, so `$out .= $piece` in a loop is quadratic in the
    /// number of appends. Every other target keeps the rewrite, because a
    /// property or an element already needs the write-back the rewrite
    /// performs; see [`InstKind::StrAppend`]. A `??.=` takes it too: a local
    /// whose representation is a plain string can never be `null`, so the
    /// operator is the plain `.=` there.
    ///
    /// # Panics
    ///
    /// Panics naming the target when it is not one [`is_reevaluable_target`]
    /// accepts.
    #[expect(
        clippy::too_many_arguments,
        reason = "the assignment's own parts plus the lowering context; a struct would buy its \
                  one call site nothing"
    )]
    pub(crate) fn lower_compound_assignment(
        &mut self,
        e: &Expr,
        op: BinaryOp,
        defaults: bool,
        target: &Expr,
        value: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        if op == BinaryOp::Concat
            && let ExprKind::Variable(name_span) = &target.kind
        {
            let name = strip_sigil(span_text(self.src, *name_span)).to_owned();
            // A `Ty::Ref` binding (`inout $x`) names the caller's slot rather than
            // an SSA value, so it is not a holder this can re-point; it falls
            // through to the rewrite, which stores through the address.
            if let Some(&(current, Ty::Str)) = env.get(&name) {
                self.lower_string_append(name, current, value, env, cur);
                return;
            }
        }
        self.lower_read_modify_write(e.span, target, op, defaults, Some(value), false, env, cur);
    }
    /// `$x++;` / `--$x;` — `rule:types/arithmetic`'s `± 1` over the target's own numeric
    /// type, through the same read-modify-write `$x += 1;` takes.
    ///
    /// **Prefix and postfix are the same statement.** The two differ only in
    /// which of the read-modify-write's two values the surrounding
    /// *expression* sees, and an expression statement sees neither — so this
    /// does not distinguish them, and `tests/conformance/lang/`'s
    /// `an-increment-answers-the-same-in-either-position` is what holds them
    /// together. Used *as a value* (`$y = $x++;`) the two do differ, and
    /// [`Self::lower_expr`]'s own arms pick between them — see
    /// [`Self::lower_incdec`], which both positions share.
    pub(crate) fn lower_incdec_stmt(
        &mut self,
        e: &Expr,
        op: IncDecOp,
        target: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        self.lower_incdec(e, op, target, env, cur);
    }
    /// `$x++` / `--$x` in either position: `rule:types/arithmetic`'s `± 1` over the
    /// target's own numeric type, answering `(old, old_ty, new, new_ty)`.
    ///
    /// The whole read-modify-write is [`Self::lower_read_modify_write`], the
    /// one `$x += 1;` already takes, so the target's address is computed once
    /// however the increment was written; all this adds is `rule:types/arithmetic`'s
    /// choice of operator. Which of the two values a caller keeps is the only
    /// difference between the prefix and postfix spellings, and between an
    /// expression statement (neither) and a value position (one).
    pub(crate) fn lower_incdec(
        &mut self,
        e: &Expr,
        op: IncDecOp,
        target: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty, ValueId, Ty) {
        let op = match op {
            IncDecOp::Inc => BinaryOp::Add,
            IncDecOp::Dec => BinaryOp::Sub,
        };
        self.lower_read_modify_write(e.span, target, op, false, None, false, env, cur)
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
    /// The assertion below has **no shape that reaches it**, and the proof is
    /// three gates deep rather than one, which is why it is asserted rather
    /// than assumed. `nvs_syntax`'s `Parser::require_write_target` admits
    /// exactly a variable, a subscript, a property and a static property as a
    /// write target (`E0105` for anything else, an increment included), and
    /// `nvs_types::expr::assign::check_write_target` then refuses the nullsafe
    /// property (`E0479`) and the element write whose root is not a place
    /// (`E0700`). Of what survives, a variable and a `Class::$prop` are
    /// re-readable on their own, and the two composite shapes each carry
    /// exactly one level that is not — the property's receiver, the
    /// subscript's base and key — which [`Self::stage_target_address`] has
    /// just staged. A target that trips it therefore reports a gate that
    /// admitted a shape it does not model, not a lowering gap.
    ///
    /// `extra_owner` is [`Self::lower_store`]'s, passed straight through: the
    /// `new` half is the value that landed in the target, so a value position
    /// consuming it needs a reference of its own.
    ///
    /// `defaults` is `??+=`, `??-=` and `??.=`
    /// (`rule:expressions/defaulting-assignment`): the read is guarded as
    /// under `??=`, and the operator's left operand is `$t ?? d` rather than
    /// `$t`, with `d` a constant staged at the target's representation. That
    /// `??` is lowered by [`Self::lower_coalesce`] from the entry
    /// `nvs_types` recorded under [`AssignOp::defaulted_read_span`].
    #[expect(
        clippy::too_many_arguments,
        reason = "the rewrite's own inputs plus the two `lower_store` needs; a struct would buy \
                  its three call sites nothing"
    )]
    pub(crate) fn lower_read_modify_write(
        &mut self,
        span: Span,
        target: &Expr,
        op: BinaryOp,
        defaults: bool,
        rhs: Option<&Expr>,
        extra_owner: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty, ValueId, Ty) {
        let temporaries = self.temporaries_mark();
        let addresses = self.staged_mark();
        self.stage_target_address(target, env, cur);
        assert!(
            self.reevaluable_target(target),
            "nvs-ir stages a compound assignment's target address before rewriting it to \
             `$x = $x op e`, which leaves every target the two phases in front of this admit \
             re-evaluable — a local and a `Class::$prop` on their own, a property or an element \
             path because the one level of it that is not was just staged. {:?} arrived anyway, \
             so `nvs_syntax`'s `Parser::require_write_target` or \
             `nvs_types::expr::assign::check_write_target` admitted a shape it does not model; \
             see this function's own doc comment for the whole proof",
            target.kind
        );
        let reads = self.staged_mark();
        // `??=` and the three defaulting operators read their target the way
        // `??` reads its left operand, so an absent key at any level gives
        // `null`. Only the read is guarded: the write below lowers the same
        // levels as the plain `=` does, which builds a row that is not there.
        let outer = std::mem::replace(
            &mut self.reading_guarded_target,
            op == BinaryOp::Coalesce || defaults,
        );
        let (old, old_ty) = self.lower_expr(target, None, env, cur);
        self.reading_guarded_target = outer;
        self.stage(target.span, old, old_ty);
        // An increment's `1` has no source span to build an `ExprKind::Int`
        // from, so it is emitted here — at the representation the read just
        // reported, since `rule:types/arithmetic` gives `int ⊕ int` and `uint ⊕ uint`
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
        let (lhs, hint) = if defaults {
            self.defaulted_read(target, op, rhs.span, *cur)
        } else {
            (target.clone(), old_ty)
        };
        let combined = Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs.clone()),
            },
            span,
        };
        let (new, new_ty) = self.lower_expr(&combined, Some(hint), env, cur);
        // The read and the `1` are dropped before the write: the store lowers
        // the target's own sub-expressions again — which is where a plain `=`
        // lowers its own — and those are the entries that have to still be
        // standing when it does.
        self.unstage_to(reads);
        let (new, new_ty) =
            self.lower_store(target, &Stored::Value(new, new_ty), extra_owner, env, cur);
        self.unstage_to(addresses);
        self.release_temporaries_since(temporaries, *cur);
        (old, old_ty, new, new_ty)
    }
    /// The `$t ?? d` a defaulting assignment applies its operator to, and the
    /// representation of `d`, which is the operator's left operand whenever
    /// `$t` is `null` or absent.
    ///
    /// `d` is the zero of the representation `nvs_types` recorded for `$t`
    /// without `null`. A tagged one is a `mixed` or a union target, and
    /// takes the `int` `0`, or the `string` `""` under `.`, which is what the
    /// checker joined that union with. The zero is emitted here and staged
    /// under [`Self::synthetic_span`], as an increment's `1` is.
    fn defaulted_read(
        &mut self,
        target: &Expr,
        op: BinaryOp,
        value: Span,
        cur: BlockId,
    ) -> (Expr, Ty) {
        let read_span = AssignOp::defaulted_read_span(target.span, value);
        let non_null = match self.exprs.lookup(read_span) {
            Some(ExprInfo::Coalesce { non_null, .. }) => {
                erase_checked_ty(*non_null, self.checked_types)
            }
            _ => panic!(
                "nvs-ir: a defaulting assignment at {read_span:?} has no recorded `??` — \
                 `nvs_types::expr::assign::defaulted_read` records one under \
                 `AssignOp::defaulted_read_span` for every one it checks"
            ),
        };
        let (ty, kind) = match non_null {
            Ty::Int => (Ty::Int, InstKind::ConstInt(0)),
            Ty::Uint => (Ty::Uint, InstKind::ConstUint(0)),
            Ty::Float => (Ty::Float, InstKind::ConstFloat(0.0)),
            Ty::Decimal => (
                Ty::Decimal,
                InstKind::ConstDecimal {
                    negative: false,
                    mantissa: 0,
                    scale: 0,
                },
            ),
            Ty::Str => (Ty::Str, InstKind::ConstStr(String::new())),
            _ if op == BinaryOp::Concat => (Ty::Str, InstKind::ConstStr(String::new())),
            _ => (Ty::Int, InstKind::ConstInt(0)),
        };
        let (zero, _) = self.emit(cur, ty, kind);
        let zero_span = self.synthetic_span();
        self.stage(zero_span, zero, ty);
        let read = Expr {
            kind: ExprKind::Binary {
                op: BinaryOp::Coalesce,
                lhs: Box::new(target.clone()),
                rhs: Box::new(Expr {
                    kind: ExprKind::Int(zero_span),
                    span: zero_span,
                }),
            },
            span: read_span,
        };
        (read, ty)
    }
    /// The `1` an increment adds or subtracts, at the target's own
    /// representation.
    ///
    /// `rule:types/arithmetic` gives each numeric type its own arithmetic row and
    /// refuses a mixed-signedness pair outright, so the literal is emitted as
    /// the operand it will be paired with rather than as a default `int`.
    ///
    /// Anything else takes `int`, which is what a written `$x += 1` puts on
    /// the right — so a target `nvs_types` could not pin down (a `mixed`, a
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
    /// Only what [`Self::reevaluable_target`] refuses is staged, so a
    /// re-readable target emits exactly the instructions it would with no
    /// staging at all. A refcounted one goes on [`Self::owned_temporaries`]:
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
    ///
    /// A property or an element level is **always** walked into rather than
    /// staged, whether or not it is re-readable on its own. What has to be
    /// evaluated once is the receiver underneath it; the level itself names a
    /// *slot*, and staging its value would hide that slot from
    /// [`Self::write_back_array`], which re-points the holder by lowering the
    /// receiver again. Staging the read instead of the receiver would make
    /// `$b->self()->rows["k"] .= "x"` call `self()` twice — once for the read
    /// and once for the write-back — where PHP calls it once.
    fn stage_address_of(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        match &e.kind {
            ExprKind::PropertyAccess {
                nullsafe: false, ..
            }
            | ExprKind::Index { .. } => {
                self.stage_target_address(e, env, cur);
            }
            _ if self.reevaluable_target(e) => self.stage_target_address(e, env, cur),
            _ => self.stage_value_of(e, env, cur),
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
    /// `$s = $s . e …;` where `$s` is a plain `Ty::Str` local — one
    /// [`InstKind::Concat`] that writes into `$s`'s own buffer whenever nothing
    /// else holds it, rather than allocating a fresh one and copying the whole
    /// accumulation into it.
    ///
    /// **No retain and no release of the local**, which is
    /// [`Self::lower_string_append`]'s bookkeeping exactly: the instruction
    /// consumes the binding's one reference and yields the one that replaces it
    /// in the same `Env` slot, so `env.insert` is the entire write-back.
    /// [`InstKind::Concat`]'s own doc comment owns that protocol, and the
    /// runtime is what decides whether the buffer is reused — a `$s` a second
    /// binding also holds is copied out of, exactly as a shared `.=` target is.
    ///
    /// The leading operand is not lowered again: the binding's current value
    /// *is* it, and it is already `Ty::Str`, so it needs neither
    /// [`Self::concat_operand`]'s conversion nor the retain a durable slot's
    /// piece takes at every other `Concat` site. Nothing is consumed before the
    /// instruction, which cannot throw, so a later operand that does leaves the
    /// binding owning exactly what it owned before.
    ///
    /// Statement position only, for [`Self::lower_compound_assignment`]'s
    /// reason: this re-points the holder and has no value to hand back, and
    /// `$t = ($s = $s . $x)` is rare enough that the general path — correct for
    /// it — is the right trade against a second concatenation lowering.
    fn lower_string_self_concat(
        &mut self,
        name: String,
        current: ValueId,
        value: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        let mut operands = Vec::new();
        Self::flatten_concat(value, &mut operands);
        let mark = self.temporaries_mark();
        let mut pieces = Vec::with_capacity(operands.len());
        pieces.push(current);
        for operand in &operands[1..] {
            let (v, aliasing) = self.concat_operand(operand, env, cur);
            if !aliasing {
                self.own_temporary(v);
            }
            pieces.push(v);
        }
        let (joined, _) = self.emit(*cur, Ty::Str, InstKind::Concat { pieces });
        self.release_temporaries_since(mark, *cur);
        env.insert(name, (joined, Ty::Str));
    }
    /// `$x = expr;` or `$obj->prop = expr;` as a bare expression statement —
    /// SSA renaming needs no join logic here, only a fresh binding in `env`
    /// (a local target) or a [`InstKind::FieldSet`] (a property target).
    ///
    /// The one shape that does not reach [`Self::lower_store`] is
    /// `$s = $s . e …` on a plain `Ty::Str` local, which
    /// [`Self::lower_string_self_concat`] hands the binding's own reference to
    /// so the concatenation can write into its buffer — the `=` spelling of
    /// what [`Self::lower_compound_assignment`] already recognises in `.=`.
    pub(crate) fn lower_reassignment(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) {
        let ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
            by_ref: false,
        } = &e.kind
        else {
            unreachable!("Self::lower_expr_stmt only routes a plain `AssignOp::Assign` here");
        };
        if let ExprKind::Variable(name_span) = &target.kind {
            let name = strip_sigil(span_text(self.src, *name_span)).to_owned();
            // A `Ty::Ref` binding (`inout $x`) names the caller's slot rather
            // than an SSA value, so it is not a holder this can re-point — the
            // same exclusion the `.=` spelling makes.
            if let Some(&(current, Ty::Str)) = env.get(&name)
                && self.concat_spine_opens_with(value, &name)
            {
                self.lower_string_self_concat(name, current, value, env, cur);
                return;
            }
        }
        self.lower_store(target, &Stored::Expr(value), false, env, cur);
    }
    /// The same assignment in **value** position — `int $b = ($a = 2);`, and
    /// the right-associative chain `$a = $b = 0;` where the inner one is the
    /// outer one's right-hand side.
    ///
    /// The answer is the value **written**, at the target's own declared
    /// representation rather than at whatever the right-hand side produced:
    /// `nvs_types::expr::assign::check_assign` types the whole expression as
    /// the target's declared type, so a `float $f = ($n = 1);` over an `int`
    /// `$n` sees the `int` the binding took, and every representation
    /// question below has one answer. [`Self::lower_store`] is what already
    /// knows that value, per target kind, which is why it hands it back.
    ///
    /// A compound spelling (`$e += 4`, `$s .= "x"`) answers PHP's own choice,
    /// the value *after* the operation, and takes
    /// [`Self::lower_read_modify_write`] — the identical rewrite the statement
    /// form takes, so the target's address is still computed exactly once.
    /// The one thing it does not reuse is
    /// [`Self::lower_compound_assignment`]'s `.=`-on-a-`string`-local fast
    /// path: [`Self::lower_string_append`] re-points the holder in place and
    /// has no value to hand back, and a `.=` in value position is rare enough
    /// that the general rewrite — which is correct for it — is the right trade
    /// against a second append lowering.
    ///
    /// **Unlike an increment, this owes a retain.** `rule:types/arithmetic` leaves an
    /// increment only non-refcounted targets, but an assignment's target is
    /// any declared type at all, and the value that landed is owned by the
    /// binding, the field, the slot or the array entry it landed in. This
    /// expression is a *fresh producer* to everything above it —
    /// [`is_aliasing_read`] does not list [`ExprKind::Assign`], so no consumer
    /// will retain it — so it hands back a second owner, which is
    /// `lower_store`'s `extra_owner` and not a retain emitted here; that
    /// function's doc comment owns why the difference matters.
    ///
    /// No [`Self::flush_ref_writebacks`] call, and none is owed: an `inout $n`
    /// argument staged inside the right-hand side is copied back at its own
    /// call, before this assignment's value is even in hand.
    ///
    /// # Panics
    ///
    /// Panics for `$a = &$b`, which no position lowers — the statement form
    /// reaches [`Self::lower_expr_stmt`]'s own unsupported-shape panic by the
    /// same route.
    pub(crate) fn lower_assign_expr(
        &mut self,
        e: &Expr,
        op: AssignOp,
        target: &Expr,
        value: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        match op.binary_op() {
            Some(binop) => {
                let (_, _, new, new_ty) = self.lower_read_modify_write(
                    e.span,
                    target,
                    binop,
                    op.defaults(),
                    Some(value),
                    true,
                    env,
                    cur,
                );
                (new, new_ty)
            }
            None => self.lower_store(target, &Stored::Expr(value), true, env, cur),
        }
    }
    /// The write half of [`Self::lower_reassignment`]: everything about
    /// *where* the value lands, with what lands there left to [`Stored`].
    ///
    /// Split out because a read-modify-write has no right-hand-side
    /// expression to hand this — its value is already in a register by the
    /// time the store runs ([`Self::lower_read_modify_write`]). The target's
    /// own sub-expressions are lowered *here*, at their own point in the
    /// evaluation order, so a plain `=` emits exactly the instructions that
    /// order calls for and nothing more; for the
    /// read-modify-write path they are staged
    /// ([`Self::staged_targets`]) and this second lowering costs nothing and
    /// runs nothing twice.
    ///
    /// Hands back **the value that landed**, at the target's own declared
    /// representation rather than at whatever the right-hand side produced —
    /// which is what an assignment in *value* position answers
    /// ([`Self::lower_assign_expr`]). A statement-position caller ignores it.
    ///
    /// `extra_owner` asks for that value to arrive owning one reference of
    /// the caller's own, and is a parameter rather than a retain the caller
    /// emits afterwards because one arm has no "afterwards" to emit it in: an
    /// `rule:classes/property-hooks` `set` hook is a **call**, the argument convention
    /// transfers the reference to it, and what the hook then does with the
    /// value is the hook's business — so a retain emitted after that call can
    /// be reading a value the hook already released. Every other arm leaves
    /// the target itself owning the value and would be safe either way; they
    /// take the same parameter so the rule is one rule.
    pub(crate) fn lower_store(
        &mut self,
        target: &Expr,
        stored: &Stored<'_>,
        extra_owner: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        match &target.kind {
            ExprKind::Variable(name_span) => {
                let lname = strip_sigil(span_text(self.src, *name_span)).to_owned();
                // An `inout $x` parameter names the caller's staged slot, not an SSA
                // binding: the write is a store through the address, so SSA
                // renaming has nothing to do and `env` is left alone. The
                // retain/load-old/release/store sequence is `Self::bind_local`'s
                // own policy against a slot instead of an `Env` entry — see
                // `Ty::Ref` and `InstKind::RefStore`.
                if let Some(&(slot, Ty::Ref)) = env.get(&lname) {
                    let pointee = self.pointee_of(&lname);
                    let (v, vty, aliasing) = self.lower_stored(stored, Some(pointee), env, cur);
                    // Into the slot's representation first, so the retain
                    // below lands on the value that is stored: a scalar
                    // variable written into a `mixed` slot arrives as its own
                    // `Int`/`Float`/`Bool`, and a refcount on that is the
                    // internal error `nvs-codegen` reserves for exactly this.
                    let v = self.coerce(*cur, v, vty, pointee, env);
                    if pointee.is_refcounted() && aliasing {
                        self.emit_retain(*cur, v);
                    }
                    if pointee.is_refcounted() && extra_owner {
                        self.emit_retain(*cur, v);
                    }
                    if pointee.is_refcounted() {
                        let (old_v, _) = self.emit(*cur, pointee, InstKind::RefLoad { slot });
                        self.emit_release(*cur, old_v);
                    }
                    self.emit_ref_store(*cur, slot, v);
                    (v, pointee)
                } else {
                    // A local declared without an initializer has no `Env`
                    // entry until this assignment makes one, and its declared
                    // type is still what the value has to arrive as — see
                    // `Lowering::declared_tys`, which is why the fallback is
                    // an `or_else` and not the other order.
                    let expected = env
                        .get(&lname)
                        .map(|&(_, t)| t)
                        .or_else(|| self.declared_tys.get(&lname).copied());
                    let (v, ty, aliasing) = self.lower_stored(stored, expected, env, cur);
                    // `rule:types/var-inference` fixes a local's type at its declaration, so an
                    // existing binding's representation wins over whatever the
                    // right-hand side produced -- otherwise a `?int` local
                    // reassigned an `int` would silently change shape, and the
                    // next phi over it would merge two representations.
                    let (v, ty) = match expected {
                        Some(want) => (self.coerce(*cur, v, ty, want, env), want),
                        None => (v, ty),
                    };
                    if ty.is_refcounted() && extra_owner {
                        self.emit_retain(*cur, v);
                    }
                    self.bind_local_value(*cur, env, lname, v, ty, aliasing);
                    (v, ty)
                }
            }
            // `$obj->prop = expr;` — the receiver's declaring class comes
            // from `self.exprs`, exactly like `ExprKind::PropertyAccess`'s
            // own read-side lowering in `Self::lower_expr`: `check_assign`'s
            // general (non-plain-local) arm in `nvs_types::expr` routes the
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
                object,
                nullsafe,
                property,
            } => {
                // A *nullsafe* target never arrives here, and never will:
                // `?->` yields `null` where the receiver is `null` and `null`
                // is not a place, so `nvs_types::expr::assign`'s
                // `check_write_target` refuses `$a?->b = v` as `E0479` where
                // it is written — PHP's own "can't use nullsafe operator in
                // write context". The alternative was never a lowering rule
                // but an assignment that silently does nothing on one path,
                // which is why this asserts the checker's answer rather than
                // naming a shape still to be lowered.
                assert!(
                    !*nullsafe,
                    "nvs-ir: a nullsafe property assignment target (`?->`) at {:?} reached \
                     lowering, so it wasn't checked with the same rules — \
                     `nvs_types::expr::assign`'s `check_write_target` refuses that as `E0479`",
                    target.span
                );
                // A `set` hook (`rule:classes/property-hooks`) makes the write a call, exactly
                // the way a `get` hook makes the read one — same receiver
                // slot, same ownership convention, and the assigned value as
                // the accessor's one ordinary argument. A property with only
                // a `get` hook still writes its own slot: Novis's hooked
                // properties are always backed, so there is a slot to write
                // (`nvs_types::signatures::PropertyHooks` owns that
                // decision).
                //
                // An `rule:types/erased-member-access` shape target is neither: it has no
                // declaring class to name and no hook to call, so it takes
                // the name-keyed write its own read mirrors and leaves before
                // the class machinery below. That arm is also the erased
                // receiver's — a plain `object` and a `mixed` both record the
                // same variant — so § 4's write half, incoming-value check
                // included, is here rather than in the `panic!` below, which
                // has no reachable target at all.
                // `guarded` is a read's question and is never set on a write
                // target — a name the concrete class does not carry is a
                // throw on this side whatever the read side was written
                // under, which is `nvs_runtime::nvs_object_slot_set`'s own
                // "never creates a field" half.
                if let Some(ExprInfo::ShapeProperty { name, slot, ty, .. }) =
                    self.exprs.lookup(target.span)
                {
                    let field = super::expr::ShapeField {
                        name: name.clone(),
                        slot: *slot,
                        ty: *ty,
                    };
                    return self.lower_shape_property_assign(
                        object,
                        &field,
                        stored,
                        extra_owner,
                        env,
                        cur,
                    );
                }
                // `rule:types/property-key-access`'s `$obj->$key = v`, whose name arrives as a
                // value: § 5 makes it `rule:types/erased-member-access`'s checked erased store with
                // the name taken from the key, so it leaves before the class
                // machinery below for the shape target's reason and one more —
                // there is no name here to ask about a hook with.
                if let Some(ExprInfo::KeyedProperty { ty, .. }) = self.exprs.lookup(target.span) {
                    let ty = *ty;
                    return self.lower_keyed_property_assign(
                        object,
                        property,
                        ty,
                        stored,
                        extra_owner,
                        env,
                        cur,
                    );
                }
                let (class, name, ty, set, observer) = match self.exprs.lookup(target.span) {
                    Some(ExprInfo::Property {
                        class,
                        name,
                        ty,
                        observer,
                        ..
                    }) => (class, name, *ty, None, observer.clone()),
                    Some(ExprInfo::HookedProperty {
                        class,
                        name,
                        ty,
                        set,
                        observer,
                        ..
                    }) => (class, name, *ty, set.clone(), observer.clone()),
                    _ => panic!(
                        "nvs-ir: a property assignment target at {:?} has no entry in the \
                          typed-expression table, so it was not checked with the same table — \
                          `nvs_types::expr::members::check_property_member` records one for every \
                          access it returns from and refuses the rest, and its own doc comment \
                          carries that proof",
                        target.span
                    ),
                };
                let field_ty = erase_checked_ty(ty, self.checked_types);
                let class_label = class.to_string();
                let field_name = name.clone();
                let observed_name = name.clone();
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
                    // Coerce first, retain second: the retain has to land on
                    // the value that is stored, and a scalar variable written
                    // into a `mixed` field arrives as its own `Int`/`Float`/
                    // `Bool` until this widens it.
                    let v = self.coerce(*cur, v, vty, field_ty, env);
                    if field_ty.is_refcounted() && aliasing {
                        self.emit_retain(*cur, v);
                    }
                    // Before the call, not after it: the argument convention
                    // transfers this reference to the hook, so once the call
                    // has run there is no value here left to retain.
                    if field_ty.is_refcounted() && extra_owner {
                        self.emit_retain(*cur, v);
                    }
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
                    // `rule:classes/property-observer-pipeline`: the observer is told "the value the hook
                    // actually committed, not necessarily the caller's
                    // original argument", so this reads the backing slot back
                    // rather than reusing `v`. A hooked property is always
                    // backed (`nvs_types::signatures::PropertyHooks` owns that
                    // decision), so there is always a slot to read, and a
                    // `set` hook that commits somewhere else has committed
                    // nothing here — which is what the slot then says.
                    if let Some(calls) = observer {
                        let (committed, _) = self.emit(
                            *cur,
                            field_ty,
                            InstKind::FieldGet {
                                object: object_v,
                                class: class_label,
                                field: field_name,
                            },
                        );
                        self.emit_observer_call(
                            *cur,
                            object_v,
                            receiver_ty,
                            &observed_name,
                            committed,
                            field_ty,
                            "onPropertySet",
                            calls.set,
                            env,
                        );
                    }
                    (v, field_ty)
                } else {
                    let (object_v, receiver_ty) = self.lower_expr(object, None, env, cur);
                    let (object_v, receiver_ty) = self.untag_receiver(object_v, receiver_ty, *cur);
                    let (v, vty, aliasing) = self.lower_stored(stored, Some(field_ty), env, cur);
                    // Coerce first, retain second — the arm above says why.
                    let v = self.coerce(*cur, v, vty, field_ty, env);
                    if field_ty.is_refcounted() && aliasing {
                        self.emit_retain(*cur, v);
                    }
                    if field_ty.is_refcounted() && extra_owner {
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
                    // `rule:classes/property-observer-pipeline`'s second step. With no `set` hook the slot
                    // store *is* the commit, so `v` is exactly the committed
                    // value and no read-back is owed.
                    if let Some(calls) = observer {
                        self.emit_observer_call(
                            *cur,
                            object_v,
                            receiver_ty,
                            &observed_name,
                            v,
                            field_ty,
                            "onPropertySet",
                            calls.set,
                            env,
                        );
                    }
                    (v, field_ty)
                }
            }
            // `Class::$prop = expr;` — the same sequence the field arm below
            // ends with, minus every step that needs a receiver: retain the
            // new value if it is an aliasing read, read the slot's previous
            // value back and release it, then store. Retain before release,
            // so `Class::$p = Class::$p;` never observes a transient zero.
            ExprKind::StaticPropertyAccess { .. } => {
                let (class, name, slot_ty) = self.static_property_of(target);
                let (v, vty, aliasing) = self.lower_stored(stored, Some(slot_ty), env, cur);
                // Coerce first, retain second — the property arms say why.
                let v = self.coerce(*cur, v, vty, slot_ty, env);
                if slot_ty.is_refcounted() && aliasing {
                    self.emit_retain(*cur, v);
                }
                if slot_ty.is_refcounted() && extra_owner {
                    self.emit_retain(*cur, v);
                }
                if slot_ty.is_refcounted() {
                    let (old_v, _) = self.emit(
                        *cur,
                        slot_ty,
                        InstKind::StaticGet {
                            class: class.clone(),
                            name: name.clone(),
                        },
                    );
                    self.emit_release(*cur, old_v);
                }
                self.emit_static_set(*cur, class, name, v);
                (v, slot_ty)
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
            // (`rule:types/arrays`'s copy-on-write separation produces a different
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
            //
            // An intermediate level may itself be an append (`$g[][0] = 1;`),
            // PHP's "start a fresh row and write into it". It has no key to
            // lower and nothing to descend into, so its row is an empty
            // `InstKind::ArrayNew` and the climb stores it back with an
            // `InstKind::ArrayAppend` — which is why the fresh row's key is
            // never named anywhere here. The *read* spelling `$a[]` is
            // `E0481` in `nvs_types`, and a base that declares no element
            // type is `E0482` there, so neither reaches this arm from source.
            ExprKind::Index { base, index } => {
                let Some(ExprInfo::Index { elem_ty, .. }) = self.exprs.lookup(target.span) else {
                    panic!(
                        "nvs-ir: an array-index assignment target at {:?} has no resolved \
                         element type recorded in the typed-expression table — `nvs_types` \
                         refuses a base that declares none as `E0482`, so this body was not \
                         checked with the same table",
                        target.span
                    );
                };
                let elem_ty = erase_checked_ty(*elem_ty, self.checked_types);
                // `$grid[0][1]` flattens to the root `$grid` and the levels
                // `$grid[0]` (whose key is `0`) and the target itself (whose
                // key is `1`, and which is not in `levels`).
                let mut levels: Vec<&Expr> = Vec::new();
                let mut root = base.unparenthesized();
                while let ExprKind::Index { base: inner, .. } = &root.kind {
                    levels.push(root);
                    root = inner.unparenthesized();
                }
                levels.reverse();
                // `E0700` leaves the root a place, so the only thing under it
                // that can run user code is a receiver — and
                // `Self::write_back_array` lowers that receiver a *second*
                // time, to re-point the slot it names. Staging it here is what
                // keeps `$b->self()->rows["k"] = "y"` calling `self()` once,
                // the count PHP has. A write arriving from
                // `Self::lower_read_modify_write` has already staged it, and
                // `Self::stage_target_address` then finds the entry and adds
                // nothing; the release below is the same bracket that function
                // puts around the whole rewrite, one scope in.
                let temporaries = self.temporaries_mark();
                let addresses = self.staged_mark();
                self.stage_target_address(root, env, cur);
                let (root_v, _) = self.lower_expr(root, None, env, cur);
                // Every key, then the value: left to right, each exactly
                // once, and every retain deferred until nothing that can
                // throw is left to lower. An intermediate level may itself be
                // an *append* (`$g[][0] = 1;`) and then has no key at all —
                // the `None`s below are that level, and the descent and the
                // climb each have one arm for it.
                let mut inner_keys: Vec<Option<(ValueId, bool)>> = Vec::with_capacity(levels.len());
                for level in &levels {
                    let ExprKind::Index { index, .. } = &level.kind else {
                        unreachable!("only an `Index` is pushed onto `levels`")
                    };
                    let key = match index {
                        None => None,
                        Some(key) => {
                            let (key_v, _key_ty, key_aliasing) =
                                self.lower_array_key(key, env, cur);
                            Some((key_v, key_aliasing))
                        }
                    };
                    inner_keys.push(key);
                }
                let outer_key = match index {
                    None => None,
                    Some(index) => {
                        let (key_v, _key_ty, key_aliasing) = self.lower_array_key(index, env, cur);
                        Some((key_v, key_aliasing))
                    }
                };
                let (v, vty, aliasing) = self.lower_stored(stored, Some(elem_ty), env, cur);
                // Into the element's representation before any retain, for
                // the reason the `inout` arm above states: `$mixed[] = $n`
                // with `int $n` is otherwise a refcount on a plain integer.
                let v = self.coerce(*cur, v, vty, elem_ty, env);
                for key in &inner_keys {
                    if let Some((key_v, true)) = *key {
                        self.emit_retain(*cur, key_v);
                    }
                }
                if let Some((key_v, true)) = outer_key {
                    self.emit_retain(*cur, key_v);
                }
                if elem_ty.is_refcounted() && aliasing {
                    self.emit_retain(*cur, v);
                }
                if elem_ty.is_refcounted() && extra_owner {
                    self.emit_retain(*cur, v);
                }
                // Down the chain. Each row arrives owning one reference —
                // `Helper::ArrayRowForWrite`'s whole job, since the
                // `ArraySet` below consumes one and an `InstKind::ArrayGet`
                // would both borrow and *throw* on an absent key, where a
                // write has to build the row.
                let mut arrays: Vec<ValueId> = Vec::with_capacity(levels.len() + 1);
                arrays.push(root_v);
                for (level, key) in levels.iter().zip(&inner_keys) {
                    let row_ty = self.row_ty_of(level);
                    let array = *arrays.last().expect("pushed the root above");
                    let (row, _) = match *key {
                        Some((key_v, _)) => self.emit_fallible(
                            *cur,
                            row_ty,
                            InstKind::HelperCall {
                                helper: Helper::ArrayRowForWrite,
                                args: vec![array, key_v],
                            },
                            env,
                        ),
                        // An append level has nothing to descend *into*: PHP
                        // starts a fresh row and the climb below appends it,
                        // which is why its key is never named. The empty
                        // `ArrayNew` already arrives owning one reference, so
                        // this is the same ownership `ArrayRowForWrite` hands
                        // back.
                        None => self.emit(
                            *cur,
                            row_ty,
                            InstKind::ArrayNew {
                                entries: Vec::new(),
                            },
                        ),
                    };
                    arrays.push(row);
                }
                // And back up, innermost first, so each level is handed the
                // separated array the level below just produced.
                let innermost = *arrays.last().expect("pushed the root above");
                let mut written = match outer_key {
                    None => self.emit_array_append(*cur, innermost, v, env),
                    Some((key_v, _)) => self.emit_array_set(*cur, innermost, key_v, v),
                };
                for (i, key) in inner_keys.iter().enumerate().rev() {
                    written = match *key {
                        Some((key_v, _)) => self.emit_array_set(*cur, arrays[i], key_v, written),
                        None => self.emit_array_append(*cur, arrays[i], written, env),
                    };
                }
                self.write_back_array(root, written, env, cur);
                self.unstage_to(addresses);
                self.release_temporaries_since(temporaries, *cur);
                (v, elem_ty)
            }
            other => unreachable!(
                "nvs-ir reaches an assignment target of kind {other:?} only if both gates above \
                 it let one through, and neither can — this is an invariant, not a gap. \
                 `nvs_syntax`'s `is_assignable` admits a local, a subscript, a property and a \
                 static property and refuses every other kind where it is written (`E0105`), for \
                 `=`, `⊕=` and an increment alike; \
                 `nvs_types::expr::assign::check_write_target` then refuses a subscript chain \
                 whose root is not a place (`E0700`). `ExprKind::Error` is the one kind those \
                 two admit and this match does not, and a body holding a parse error is never \
                 lowered"
            ),
        }
    }
    /// The representation of one intermediate level of a nested
    /// array-element write target — `$grid[0]` in `$grid[0][1] = v` — read
    /// out of the typed-expression table exactly as the write's own element
    /// type is, since the checker records an [`ExprInfo::Index`] entry per
    /// `Index` node whether it was reached as a read or as a target.
    ///
    /// Neither of the two refusals below has a reachable target, and the
    /// reason is one fact each, both owned by `nvs_types::expr`'s
    /// `ExprKind::Index` arm — which is the only producer of an
    /// [`ExprInfo::Index`] entry there is.
    ///
    /// **No entry at all.** That arm records one exactly where the base's type
    /// is a `Ty::Array`, and every path it records nothing on has reported a
    /// diagnostic: `E0482` for a base with no element type, or nothing extra
    /// because the base itself already reported, or nothing extra because the
    /// chain's root is a write target `check_write_target` is refusing in the
    /// same breath (`E0479`, `E0478`, `E0480`, `E0700`). A body holding a
    /// diagnostic is never lowered, so a level reaching here has an entry.
    ///
    /// **An entry whose element type is not an array.** *Intermediate* is what
    /// makes this hold: the level above was checked with this one as its base,
    /// and it resolved — otherwise the paragraph above would have stopped the
    /// program — so this level's type was a `Ty::Array` there. This level's
    /// type *is* the `elem_ty` recorded here, with one exception that cannot
    /// arise: a `??`-guarded read is typed with its `null` dropped, and
    /// `nvs_types::Env::coalesce_guarded` is filled only from a `??`'s own left
    /// operand, which is a read. `??=` guards its target's read through a mark
    /// in the typed-expression table that only the read consults
    /// ([`Self::reading_guarded_target`]), so its levels' entries are the
    /// ordinary ones — `array<?array<int>> $g; $g["0"]["1"] ??= 5;` is `E0482`
    /// like the plain `=` it is spelled out of.
    ///
    /// # Panics
    ///
    /// On either, as an invariant this crate asserts rather than a gap it
    /// leaves open.
    fn row_ty_of(&self, level: &Expr) -> Ty {
        let Some(ExprInfo::Index { elem_ty, .. }) = self.exprs.lookup(level.span) else {
            unreachable!(
                "nvs-ir reaches an intermediate level of a nested array-index assignment \
                 target at {:?} with no resolved element type recorded in the typed-expression \
                 table only if `nvs_types::expr`'s `ExprKind::Index` arm both declined to \
                 record one and reported nothing, and it never does — see this function's own \
                 doc comment for which path leaves which diagnostic",
                level.span
            );
        };
        let row_ty = erase_checked_ty(*elem_ty, self.checked_types);
        assert!(
            row_ty == Ty::Array,
            "nvs-ir: an intermediate level of a nested array-index assignment target at {:?} \
             lowered to {row_ty:?} rather than an array, so there is nothing for the level \
             above it to write back into — the level above it was checked with this one as \
             its base and resolved, which is only possible where this one is an array",
            level.span
        );
        row_ty
    }
    /// `unset($a[$k]);` — the one `unset` target `rule:classes/unset-is-refused-on-a-property` leaves
    /// standing, lowered to [`InstKind::ArrayUnset`] and written back through
    /// the same [`Self::write_back_array`] an element *write* uses, since
    /// removing an entry separates a shared array exactly the way writing one
    /// does.
    ///
    /// The key is borrowed rather than stored, which inverts
    /// [`Self::lower_reassignment`]'s `Index`-target retain: an aliasing key
    /// needs nothing, and a freshly converted one (an `int` subscript's
    /// decimal-string form) is this frame's own single-owner temporary and is
    /// released once the removal has read it. A level's key on a *nested*
    /// target is the other way round again, because the climb stores it.
    ///
    /// **A nested target flattens exactly as a nested write does**
    /// ([`Self::lower_store`]'s `Index` arm), down to the one root that has a
    /// holder and back up storing each separated level into the one above it.
    /// The descent differs in a single decision and it is the whole reason
    /// this walk is written out rather than shared: a level is read with
    /// [`AbsentKey::Throws`] rather than vivified, since a removal that first
    /// creates the row it removes from is an entry neither PHP nor Novis puts
    /// there.
    ///
    /// # Panics
    ///
    /// A [`guarded_by!`] naming `E0234` for any other `unset` operand:
    /// `nvs_types::expr::check_unset_target` is the one home of what an operand
    /// may be, and it refuses every other spelling where it is written — a
    /// declared property as `E0413`
    /// (`rule:classes/unset-is-refused-on-a-property`), and everything from a
    /// bare local to a subscript of a temporary as that code.
    pub(crate) fn lower_unset(&mut self, target: &Expr, env: &mut Env, cur: &mut BlockId) {
        let ExprKind::Index {
            base,
            index: Some(index),
        } = &target.kind
        else {
            guarded_by!(
                code::E_UNSET_TARGET_NOT_AN_ELEMENT,
                "nvs-ir reached `unset` on {:?}. `rule:classes/unset-is-refused-on-a-property` \
                 leaves one operand standing — an element of an array a local, a property or a \
                 static property holds, including at depth — and \
                 `nvs_types::expr::check_unset_target` refuses every other spelling where it is \
                 written, a declared property as `E0413` and the rest as this code",
                target.kind
            );
        };
        // `$grid["r"]["1"]` flattens to the root `$grid` and the levels
        // `$grid["r"]` (whose key is `"r"`) and the target itself (whose key
        // is `"1"`, and which is not in `levels`) — the same flatten
        // [`Self::lower_store`]'s `Index` arm does, and for the same reason:
        // only the root has a holder to be written back to.
        let mut levels: Vec<&Expr> = Vec::new();
        let mut root = base.unparenthesized();
        while let ExprKind::Index { base: inner, .. } = &root.kind {
            levels.push(root);
            root = inner.unparenthesized();
        }
        levels.reverse();
        let (root_v, root_ty) = self.lower_expr(root, None, env, cur);
        assert!(
            root_ty == Ty::Array,
            "nvs-ir: an `unset` target's root lowered to {root_ty:?} rather than an array — \
             `nvs_types` refuses a base that declares no element type as `E0482`, so this body \
             was not checked with the same table"
        );
        // Every key, left to right and each exactly once, before anything is
        // retained or descended into. A freshly built one — a rendered
        // subscript, a literal — is this frame's own reference with nowhere
        // else to be found, and the descent below is fallible at every level,
        // so each goes on `Self::owned_temporaries` for as long as something
        // can throw over it.
        let keys_mark = self.temporaries_mark();
        let mut inner_keys: Vec<(ValueId, bool)> = Vec::with_capacity(levels.len());
        for level in &levels {
            let ExprKind::Index {
                index: Some(key), ..
            } = &level.kind
            else {
                panic!(
                    "nvs-ir: an `unset` target's level at {:?} is the append spelling `$a[]`, \
                     which `nvs_types` reports as `E0481` wherever it is not a plain `=`'s own \
                     target",
                    level.span
                );
            };
            let (key_v, key_ty, key_aliasing) = self.lower_array_key(key, env, cur);
            if key_ty.is_refcounted() && !key_aliasing {
                self.own_temporary(key_v);
            }
            inner_keys.push((key_v, key_aliasing));
        }
        let (key_v, key_aliasing) = self.lower_rendered_array_key(index, env, cur);
        if !key_aliasing {
            self.own_temporary(key_v);
        }
        // Down the chain, borrowing each row. A level is an ordinary element
        // *read* — [`AbsentKey::Throws`], so `unset($g["nope"]["1"])` throws
        // exactly as `$g["nope"]["1"]` would (`rule:types/absent-storage-is-never-a-zero-value`, PHP being
        // silent there instead). Deliberately **not**
        // [`Helper::ArrayRowForWrite`], whose absent-key answer is to
        // vivify: a removal that first creates the row it is removing from
        // would leave an entry behind that neither language puts there.
        // Nothing is retained yet, so a throw out of any level drops nothing.
        let mut arrays: Vec<ValueId> = Vec::with_capacity(levels.len() + 1);
        arrays.push(root_v);
        for (level, (level_key, _)) in levels.iter().zip(&inner_keys) {
            let row_ty = self.row_ty_of(level);
            let array = *arrays.last().expect("pushed the root above");
            let (row, _) = self.emit_fallible(
                *cur,
                row_ty,
                InstKind::ArrayGet {
                    array,
                    key: *level_key,
                    absent: AbsentKey::Throws,
                },
                env,
            );
            arrays.push(row);
        }
        // Nothing below throws, which is what makes this the first safe place
        // to own anything: one reference per row, because the removal and
        // every store on the climb each consume one. It is also where the keys
        // stop needing the stack — from here each is either released by hand
        // (the removal's, which only borrowed it) or consumed by an
        // `InstKind::ArraySet` on the climb, and an entry surviving into
        // either would be a second release of the same reference.
        self.forget_temporaries_since(keys_mark);
        for row in arrays.iter().skip(1) {
            self.emit_retain(*cur, *row);
        }
        let innermost = *arrays.last().expect("pushed the root above");
        let (mut written, _) = self.emit(
            *cur,
            Ty::Array,
            InstKind::ArrayUnset {
                array: innermost,
                key: key_v,
            },
        );
        if !key_aliasing {
            self.emit_release(*cur, key_v);
        }
        // And back up, innermost first, so each level is handed the separated
        // array the level below just produced. A level's key is *stored* by
        // the [`InstKind::ArraySet`] here, unlike the borrow the removal above
        // took, so an aliasing one is retained for the slot it lands in.
        for (i, (level_key, level_aliasing)) in inner_keys.iter().enumerate().rev() {
            if *level_aliasing {
                self.emit_retain(*cur, *level_key);
            }
            written = self.emit_array_set(*cur, arrays[i], *level_key, written);
        }
        self.write_back_array(root, written, env, cur);
    }
    /// `rule:types/grammar`.3's `[int $a, string $b] = $pair;` — a run of element
    /// reads off one subject, and nothing else at all.
    ///
    /// Every leaf is the subscript it is spelled out of: `$pair[0]`,
    /// `$pair[1]`, or `$pair[k]` where the element writes a `k =>`. So a
    /// missing key **throws** exactly as that subscript would
    /// ([`AbsentKey::Throws`], `rule:types/absent-storage-is-never-a-zero-value`'s divergence — PHP warns
    /// and binds `null`), a leaf binds exactly as
    /// [`StmtKind::LocalDecl`]'s own arm binds an initializer, and the
    /// element read is at the *element's* representation rather than the
    /// leaf's: `nvs_types::locals` records that type under the leaf's own
    /// span as the same [`ExprInfo::Index`] entry a subscript gets, and
    /// [`Self::coerce`] takes it from there to the declared one, which is
    /// how `[float $f] = $ints;` widens where `rule:types/conversion` says it does.
    ///
    /// **The subject is lowered once**, whatever the pattern's depth, and a
    /// nested target reads through the borrowed element rather than a copy
    /// of it. A fresh producer (`[int $a] = rows();`) is staged on the
    /// owned-temporaries stack for the whole statement, so a throw out of
    /// any element read drops it on the way — [`Self::lower_index`]'s rule,
    /// one statement wider — and so is a rendered key, which is why the
    /// mark is taken before the subject and released only at the end.
    ///
    /// A leaf's *position* is its index in the pattern, `[, int $b]`'s
    /// skipped slot included, and a `k =>` element occupies one like any
    /// other rather than renumbering what follows it. PHP refuses to mix the
    /// two spellings in one pattern at all, so nothing observable rides on
    /// which answer this gives the mixture.
    pub(crate) fn lower_destructure(
        &mut self,
        target: &'a DestructureTarget,
        value: &'a Expr,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let mark = self.temporaries_mark();
        let (subject_v, subject_ty) = self.lower_expr(value, None, env, cur);
        assert!(
            subject_ty == Ty::Array,
            "nvs-ir destructures only an `array<T>` — got {subject_ty:?}; a value that names no \
             element type is `E0482` at `nvs_types::locals`, so this body was not checked"
        );
        if !self.aliasing_read(value) {
            self.own_temporary(subject_v);
        }
        self.lower_destructure_target(target, subject_v, cur, env);
        self.release_temporaries_since(mark, *cur);
    }
    /// One level of [`Self::lower_destructure`]'s pattern, against the array
    /// that level takes apart. Recurses for a nested target, whose own
    /// subject is the element just read.
    fn lower_destructure_target(
        &mut self,
        target: &'a DestructureTarget,
        subject: ValueId,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        for (position, element) in target.elements.iter().enumerate() {
            match element {
                DestructureElement::Skip => {}
                DestructureElement::Leaf {
                    key,
                    ty,
                    name,
                    span,
                    ..
                } => {
                    let decl_ty = ty.as_ref().expect(
                        "nvs_types reports E0101 for a destructuring leaf with no declared type",
                    );
                    let declared = lower_decl_type(decl_ty, self.exprs, self.checked_types);
                    let elem_ty = self.destructured_element_ty(*span);
                    let v =
                        self.read_destructured(subject, key.as_ref(), position, elem_ty, cur, env);
                    let (v, elem_ty) = self.widen_marked_binding(*span, v, elem_ty, env, *cur);
                    let v = self.coerce(*cur, v, elem_ty, declared, env);
                    let lname = strip_sigil(span_text(self.src, *name)).to_owned();
                    // The read borrows the entry the array still owns, so the
                    // binding takes a reference of its own — the same
                    // `is_aliasing_read` answer `Self::bind_local` computes
                    // for a `$a = $pair["0"];` written out by hand.
                    self.bind_local_value(*cur, env, lname, v, declared, true);
                }
                DestructureElement::Nested { key, target, .. } => {
                    let nested = self.read_destructured(
                        subject,
                        key.as_ref(),
                        position,
                        Ty::Array,
                        cur,
                        env,
                    );
                    self.lower_destructure_target(target, nested, cur, env);
                }
                _ => {}
            }
        }
    }
    /// The representation a destructuring leaf's element read answers in,
    /// which is the element type of the array being taken apart and not the
    /// leaf's own declared type — see [`Self::lower_destructure`].
    fn destructured_element_ty(&self, span: Span) -> Ty {
        let Some(ExprInfo::Index { elem_ty, .. }) = self.exprs.lookup(span) else {
            panic!(
                "nvs-ir: a destructuring leaf at {span:?} has no element type recorded in the \
                 typed-expression table — `nvs_types::locals` records one for every leaf it \
                 accepts, so this body was not checked with the same table"
            );
        };
        erase_checked_ty(*elem_ty, self.checked_types)
    }
    /// One element read out of a destructuring subject: `subject[key]`, or
    /// `subject[position]` where the element writes no key.
    ///
    /// A rendered key is staged rather than released here, for
    /// [`Self::lower_index`]'s reason — the read below can throw, and the
    /// landing block releases the stack the whole statement opened.
    fn read_destructured(
        &mut self,
        subject: ValueId,
        key: Option<&'a Expr>,
        position: usize,
        result_ty: Ty,
        cur: &mut BlockId,
        env: &mut Env,
    ) -> ValueId {
        let key_v = match key {
            Some(key) => {
                let (key_v, key_ty, key_aliasing) = self.lower_array_key(key, env, cur);
                if key_ty.is_refcounted() && !key_aliasing {
                    self.own_temporary(key_v);
                }
                key_v
            }
            None => {
                let index = i64::try_from(position)
                    .expect("a destructuring pattern has far fewer than i64::MAX elements");
                self.emit(*cur, Ty::Int, InstKind::ConstInt(index)).0
            }
        };
        self.emit_fallible(
            *cur,
            result_ty,
            InstKind::ArrayGet {
                array: subject,
                key: key_v,
                absent: AbsentKey::Throws,
            },
            env,
        )
        .0
    }
    /// Whether lowering `e` twice observes the same value and runs no side
    /// effect the second time — the precondition
    /// [`Self::lower_read_modify_write`]'s rewrite needs, since `$t ⊕= e`
    /// becomes `$t = $t ⊕ e` with the target appearing on both sides.
    ///
    /// A **staged** sub-expression qualifies whatever it was written as: its
    /// second lowering is a lookup in [`Self::staged_targets`] and runs
    /// nothing at all, which is the whole reason that table exists. Beyond
    /// that, a local read (`$this` is one, spelled as an ordinary variable), a
    /// `Class::$prop` static and a property or element path built over those
    /// are the shapes that
    /// qualify: each is a load, and a `get` hook (`rule:classes/property-hooks`) still runs
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
            // `Class::$prop` is re-readable for free and is the one target
            // here that needs no staging at all: `Self::static_property_of`
            // answers the whole `(declaring class, name, representation)`
            // triple out of the typed-expression table, so the class part is
            // a *name* rather than a sub-expression — there is nothing to
            // evaluate once, and nothing that could run user code a second
            // time. `self::`/`static::`/`parent::` included, since name
            // resolution has already turned each of them into the declaring
            // class's label.
            ExprKind::StaticPropertyAccess { .. } => true,
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
