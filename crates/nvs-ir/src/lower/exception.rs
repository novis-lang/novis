//! `rule:errors/propagation`'s throw/catch lowering: the landing pads, the `finally` ladder, and the synthesized `Throwable` constructor.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. The methods are
//! `pub(crate)` so they reach across these modules and no further.

use super::*;

impl<'a> Lowering<'a> {
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
    /// # A tagged operand
    ///
    /// A [`Ty::Tagged`] operand arrives here on purpose. `nvs_types`'
    /// `expr::members::reject_unthrowable` refuses a type that can hold no
    /// object and a class outside spec § 10's tree, and passes `mixed`,
    /// `object` and every union — `?Throwable` among them — because the tree
    /// is what `catch` matches on at run time. [`Self::guard_throwable`] asks
    /// the two questions that leaves, where the value is.
    ///
    /// # Panics
    ///
    /// Panics naming the operand's representation if it is neither
    /// [`Ty::Object`] nor [`Ty::Tagged`]. Those two are what
    /// `reject_unthrowable`'s `can_hold_an_object` predicate lets past, so a
    /// third is a bug in this crate and not a shape a program can write.
    pub(crate) fn lower_throw(&mut self, inner: &Expr, env: &mut Env, cur: &mut BlockId) {
        let (v, ty) = self.lower_expr(inner, None, env, cur);
        assert!(
            matches!(ty, Ty::Object | Ty::Tagged),
            "`throw`'s operand is a `Ty::Object` or a `Ty::Tagged`, the pair \
             `nvs_types::expr::members::reject_unthrowable` admits — representation {ty:?} means \
             this is a bug in nvs-ir"
        );
        let borrowed = self.aliasing_read(inner);
        let v = self.guard_throwable(v, ty, borrowed, inner.span, env, cur);
        if borrowed {
            self.emit_retain(*cur, v);
        }
        let source = self.throw_source(*cur);
        let landing = self.landing_block(env);
        self.seal(
            *cur,
            Terminator::Throw {
                value: v,
                source,
                landing,
            },
        );
    }
    /// The two answers a [`Ty::Tagged`] `throw` operand's type did not settle,
    /// asked in front of the raise: whether the tag is an object at all —
    /// [`Self::split_on_object_tag`], shared with `clone`'s own guard — and
    /// whether that object is inside spec § 10's tree. `cur` is left on the
    /// block where both hold, and the value handed back is the object itself.
    ///
    /// A [`Ty::Object`] operand is handed straight back. Its class is the
    /// checker's own answer, and `reject_unthrowable` has already refused
    /// every named one outside the tree.
    ///
    /// **The wording is PHP's, word for word.**
    /// `rule:php-migration/every-divergence-is-deliberate-and-listed` lists no
    /// divergence here, so a program that catches one of these reads what it
    /// reads in PHP. The class carrying it is [`LOGIC_ERROR`] — spec § 10's
    /// "a bug in the program", the entry [`Lowering::lower_match`]'s unmatched
    /// subject already raises — because the closed tree has no `Error` of
    /// PHP's own. A closure takes the second message rather than the first:
    /// [`InstKind::TagIs`] compares one tag byte, and a closure carries the
    /// object tag every other instance does
    /// (`rule:types/callable-is-a-closure`), so what refuses it is the
    /// `Throwable` test below.
    ///
    /// **What it spends** (`rule:programs/memory-priority`): one tag compare
    /// and one descriptor walk, on the tagged operand alone, and no allocation
    /// on the path where both hold.
    ///
    /// `borrowed` is [`Self::aliasing_read`]'s answer for the operand, and it
    /// is what decides the release on either raise: a fresh operand is this
    /// frame's to discharge before the block leaves, and [`InstKind::Untag`]
    /// carries that obligation onto the narrowed value with the reference.
    fn guard_throwable(
        &mut self,
        v: ValueId,
        ty: Ty,
        borrowed: bool,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        if ty != Ty::Tagged {
            return v;
        }
        let (object, not_an_object) = self.split_on_object_tag(v, span, cur);
        if !borrowed {
            self.emit_release(not_an_object, v);
        }
        self.raise_logic_error(not_an_object, "Can only throw objects", env);

        let (in_the_tree, _) = self.emit(
            *cur,
            Ty::Bool,
            InstKind::ClassTest {
                value: object,
                class: TestedClass::Named(THROWABLE_ROOT.to_owned()),
            },
        );
        let inside = self.new_block();
        let outside = self.new_block();
        let then_edge = self.ids.next_edge(span);
        let else_edge = self.ids.next_edge(span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: in_the_tree,
                then_block: inside,
                then_edge,
                else_block: outside,
                else_edge,
            },
        );
        if !borrowed {
            self.emit_release(outside, object);
        }
        self.raise_logic_error(
            outside,
            "Cannot throw objects that do not implement Throwable",
            env,
        );
        *cur = inside;
        object
    }
    /// The one question a [`Ty::Tagged`] operand's type leaves an operator that
    /// needs an object: whether the tag is one. `cur` is left on the block where
    /// it is, holding the [`InstKind::Untag`] whose value is handed back, and
    /// the block handed back beside it is the one the caller owes its own
    /// refusal — which is the whole of what the two operators differ in.
    /// [`Self::guard_throwable`] asks `throw`'s second question there and
    /// [`Lowering::guard_cloneable`] renders the tag into PHP's `clone()`
    /// message.
    ///
    /// **What it spends** (`rule:programs/memory-priority`): one tag compare on
    /// the tagged operand, and no allocation on either side of the branch.
    pub(crate) fn split_on_object_tag(
        &mut self,
        v: ValueId,
        span: Span,
        cur: &mut BlockId,
    ) -> (ValueId, BlockId) {
        let (is_object, _) = self.emit(
            *cur,
            Ty::Bool,
            InstKind::TagIs {
                operand: v,
                repr: Ty::Object,
            },
        );
        let an_object = self.new_block();
        let not_an_object = self.new_block();
        let then_edge = self.ids.next_edge(span);
        let else_edge = self.ids.next_edge(span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: is_object,
                then_block: an_object,
                then_edge,
                else_block: not_an_object,
                else_edge,
            },
        );
        let (object, _) = self.emit(an_object, Ty::Object, InstKind::Untag { operand: v });
        *cur = an_object;
        (object, not_an_object)
    }
    /// Builds a [`LOGIC_ERROR`] carrying `message` in `block` and seals the
    /// block on the raise of it.
    ///
    /// Argument 2 is the `{previous}` bag flattened to its own `null` default,
    /// widened into the [`Ty::Tagged`] slot spec § 10's `Throwable|null`
    /// erases to — the list [`Lowering::lower_match`]'s own throw builds by
    /// hand, for the same reason. No retain: the instance is built here and
    /// has exactly one owner, which [`Terminator::Throw`] takes.
    fn raise_logic_error(&mut self, block: BlockId, message: &str, env: &mut Env) {
        let (message, _) = self.emit(block, Ty::Str, InstKind::ConstStr(message.to_owned()));
        let (absent, _) = self.emit(block, Ty::Null, InstKind::ConstNull);
        let absent = self.coerce(block, absent, Ty::Null, Ty::Tagged, env);
        let (exception, _) = self.emit_fallible(
            block,
            Ty::Object,
            InstKind::New {
                class: LOGIC_ERROR.to_owned(),
                ctor: Some(THROWABLE_CTOR.to_owned()),
                args: vec![message, absent],
            },
            env,
        );
        let source = self.throw_source(block);
        let landing = self.landing_block(env);
        self.seal(
            block,
            Terminator::Throw {
                value: exception,
                source,
                landing,
            },
        );
    }
    /// `Core\Script::finish()` — the one `Core` row that is *raised* rather
    /// than called, and the whole of what makes the fourth ending an ending
    /// every `finally` observes.
    ///
    /// The value raised is a fresh instance of `nvs_types::CORE_SCRIPT_FINISH_CLASS`:
    /// a second, parentless root of the exception tree, which declares no
    /// property and no constructor and which nothing extends. That buys both
    /// halves at once against [`super::TryFrame`] as it already stands — the
    /// throw path is the path a `finally` body lives on, so every enclosing
    /// region's finally-and-re-raise block runs, while the `catch` dispatch's
    /// self-or-ancestor walk is false for every arm in the tree and hands the
    /// same reference onward. `rule:errors/propagation`.
    ///
    /// No retain: the instance is built here and has exactly one owner, which
    /// [`Terminator::Throw`] takes — [`Self::lower_throw`]'s own operand rule,
    /// on the side of it that is a fresh `new`.
    ///
    /// What follows is dead by construction and still lowered, for the reason
    /// [`super::Lowering::lower_exit`] gives: a `never`-typed expression owes
    /// its caller a value, so the sealed block is followed by a fresh one and
    /// the value handed back is a constant nothing reads.
    pub(crate) fn lower_finish(&mut self, env: &mut Env, cur: &mut BlockId) -> (ValueId, Ty) {
        let (marker, _) = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::New {
                class: nvs_types::CORE_SCRIPT_FINISH_CLASS.to_owned(),
                ctor: None,
                args: Vec::new(),
            },
            env,
        );
        let source = self.throw_source(*cur);
        let landing = self.landing_block(env);
        self.seal(
            *cur,
            Terminator::Throw {
                value: marker,
                source,
                landing,
            },
        );
        *cur = self.new_block();
        self.emit(*cur, Ty::Int, InstKind::ConstInt(0))
    }
    /// The carrier a `throw` in `cur` hands `nvs_runtime::nvs_raise`, which
    /// fills the raised object's `location` from it.
    ///
    /// **The throw site, not the construction site**, and deliberately so: the
    /// backtrace beside it holds the frames the exception *unwound out of*
    /// rather than a snapshot taken at `new` (see `nvs_runtime::throwable`'s
    /// own docs for why `rule:errors/propagation`'s checked-return convention makes that the
    /// cheap shape), so a `location` naming the construction site would be the
    /// one field disagreeing with everything around it.
    ///
    /// It is [`Self::producer_source`]'s constant unchanged — the very datum a
    /// record producer takes as its argument 0, which is what lets
    /// `rule:errors/a-record-names-where-it-was-produced` have one
    /// construction with two readers. Nothing is stored from here: the slot is
    /// one `set_field` in the runtime, where the raise already reaches the
    /// object, rather than a read, a release and a store emitted into every
    /// frame that can throw.
    pub(crate) fn throw_source(&mut self, cur: BlockId) -> Option<ValueId> {
        Some(self.producer_source(cur))
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
    /// [`InstKind::ClassTest`] against its declared class plus a branch —
    /// the same test `$e is T` compiles to, which is why nothing here
    /// needs a second mechanism. `catch (Throwable $e)` is not special-cased:
    /// every exception class descends from `Throwable`, so its test
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
    /// A `break`/`continue` out of a protected region *does* run the pending
    /// `finally` of every region it leaves, bounded by the loop it targets —
    /// [`Self::run_finallys_above`].
    pub(crate) fn lower_try(
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
            handler: Some(handler_block),
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

        // The phis first, then the binding: `nvs-codegen` requires a block's
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
    /// The class-test chain a `try`'s clauses lower to, plus the
    /// re-raise that ends it — see [`Self::lower_try`] for the shape and why
    /// it needs no mechanism of its own.
    pub(crate) fn lower_catch_clauses(
        &mut self,
        dispatch: BlockId,
        thrown: ValueId,
        catches: &'a [CatchClause],
        finally: Option<&'a Block>,
        join: &mut CatchJoin<'_>,
    ) {
        let mut test_block = dispatch;
        for clause in catches {
            let caught = self.caught_class_label(&clause.ty);
            let (cond, _) = self.emit(
                test_block,
                Ty::Bool,
                InstKind::ClassTest {
                    value: thrown,
                    class: TestedClass::Named(caught),
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
            // Every exit from the clause body owes this region's `finally`, so
            // the body is lowered under a frame carrying it: a `return` takes
            // it through `run_pending_finallys`, and a *throw* takes it
            // through the frame's handler. That handler is this region's own
            // finally-and-re-raise block rather than the dispatch above — a
            // clause does not catch what its own body raises — and with no
            // `finally` there is nothing owed at all, so the frame names no
            // handler and such a throw goes straight to the enclosing region.
            let reraise = finally.map(|_| self.new_block());
            self.try_stack.push(crate::lower::TryFrame {
                handler: reraise,
                edges: Vec::new(),
                finally,
            });
            self.lower_stmts(&clause.body.stmts, &mut handler_cur, &mut handler_env);
            let clause_frame = self
                .try_stack
                .pop()
                .expect("just pushed this clause's own frame above");
            if let Some(block) = finally
                && let Some(reraise) = reraise
            {
                self.lower_finally_and_reraise(
                    reraise,
                    block,
                    &clause_frame.edges,
                    bound.as_deref(),
                    &join.dispatch_env,
                );
            }
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
                    source: None,
                    landing,
                },
            );
        }
    }
    /// Builds the block a throw out of a `catch` clause's own body lands in:
    /// this region's `finally`, then the very same reference handed onward to
    /// the enclosing region.
    ///
    /// PHP's own order, and the one thing that makes a `catch` body no
    /// different from any other exit out of the region: the `finally` runs
    /// before the new exception leaves the frame, and a `finally` that throws
    /// on its way *replaces* it, because its own `Terminator::Throw` is never
    /// reached.
    ///
    /// `edges` is the clause frame's — one landing block per failing call in
    /// the body — and they are folded into this block's phis by the same
    /// [`Self::merge_envs`] the dispatch block's own edges go through. The
    /// clause binding is released here, exactly as the clause's completing
    /// path already releases it before lowering its own copy of the `finally`:
    /// the first exception's reference is this frame's, and the second one is
    /// what [`InstKind::TakeThrown`] hands back.
    ///
    /// The block is built even when `edges` is empty — a clause body that
    /// cannot throw at all — because every block this lowering creates has to
    /// reach a terminator ([`Self::finish`]), and a dead handler block is what
    /// [`Self::lower_try`] already leaves behind for a `try` body in the same
    /// position.
    pub(crate) fn lower_finally_and_reraise(
        &mut self,
        reraise: BlockId,
        finally: &'a Block,
        edges: &[(BlockId, Env)],
        bound: Option<&str>,
        dispatch_env: &Env,
    ) {
        // The phis first, then the take: `nvs-codegen` requires a block's phis
        // to be its leading run.
        let mut env = self.merge_envs(reraise, edges, dispatch_env);
        let (thrown, _) = self.emit(reraise, Ty::Object, InstKind::TakeThrown);
        let mut cur = reraise;
        if let Some(name) = bound
            && let Some(&(v, _)) = env.get(name)
        {
            self.emit_release(cur, v);
            env.remove(name);
        }
        self.lower_stmts(&finally.stmts, &mut cur, &mut env);
        if !self.is_terminated(cur) {
            let landing = self.landing_block(&env);
            self.seal(
                cur,
                Terminator::Throw {
                    value: thrown,
                    source: None,
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
    pub(crate) fn run_pending_finallys(&mut self, cur: &mut BlockId, env: &mut Env) {
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
    /// Lowers a copy of the `finally` body of every protected region *above*
    /// `depth` on [`Self::try_stack`], innermost first — what a `break` or a
    /// `continue` owes before it leaves the regions between it and its loop.
    ///
    /// The same duplicate-at-each-exit shape as
    /// [`Self::run_pending_finallys`], and the same pop-before-lowering rule,
    /// bounded rather than exhaustive: `depth` is
    /// [`LoopFrame::try_depth`](super::LoopFrame::try_depth), the stack height
    /// the loop's body started at, so a region enclosing the *whole* loop is
    /// left alone. Stops early if a `finally` body itself terminates the
    /// block — a `return` inside one wins, and the caller must re-check
    /// [`Self::is_terminated`] before sealing its own jump.
    pub(crate) fn run_finallys_above(&mut self, depth: usize, cur: &mut BlockId, env: &mut Env) {
        let mut saved = Vec::new();
        while self.try_stack.len() > depth {
            let frame = self
                .try_stack
                .pop()
                .expect("the loop condition just proved the stack is deeper than `depth`");
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
    /// `rule:expressions/catch-lowers-to-block-form`: `expr catch (T $e) => value` — the block form's lowering with a
    /// value on every edge that reaches the join.
    ///
    /// Everything structural is [`Self::lower_try`]'s, for the reasons that
    /// function's own doc comment gives: a [`TryFrame`] brackets the guarded
    /// expression so each failing call inside it records its own landing
    /// block, [`InstKind::TakeThrown`] takes the pending exception once at the
    /// top of the handler, an arm is one [`InstKind::ClassTest`] against its
    /// class plus a branch, and what no arm matched is handed straight back to
    /// a [`Terminator::Throw`] carrying the very same reference. What the
    /// expression form adds is the join: the guard and every completing arm
    /// carry a value into one [`InstKind::Phi`], widened into a shared
    /// representation by the [`Self::join_representations`] a `match` already
    /// joins its arms through. § 6's "written to one temporary" needs no
    /// temporary in SSA — the phi *is* it.
    ///
    /// What differs from the block form follows from an arm being an expression
    /// rather than a block:
    ///
    /// - **No `finally`.** The expression form has no spelling for one
    ///   (`rule:expressions/catch-expression-precedence`), so no frame pushed here carries one and no exit out of this
    ///   region owes one. Every place [`Self::lower_catch_clauses`] lowers a
    ///   copy of a `finally` body is simply absent, which is also why no arm
    ///   body needs a frame of its own.
    /// - **A `throw` arm contributes no edge**, so its type never reaches the
    ///   join. `throw` in expression position hands back a placeholder value
    ///   out of a fresh unreachable block ([`Self::lower_expr`]'s own arm), and
    ///   letting that block into the phi would feed the placeholder's
    ///   representation to [`Self::join_representations`] — a `string` guard
    ///   beside a `throw` arm would join at [`Ty::Tagged`], which is not the
    ///   type the checker gave the expression (`nvs_types::expr::check_expr`
    ///   drops a `never` arm from § 4's union for the same reason). A `throw`
    ///   body therefore goes through [`Self::lower_throw`] directly, which
    ///   seals the arm's own block and opens no successor at all.
    /// - **The binding is released on the arm's completing edge**, exactly
    ///   where a clause body's is and for [`Self::lower_try`]'s reason: this
    ///   crate's [`Env`] is flat, so a name still holding a reference at the
    ///   join would be dropped by [`Self::merge_envs`] and never released. On
    ///   a `throw` arm the release is the landing block's instead, since that
    ///   path leaves through [`Self::landing_block`] rather than through the
    ///   join.
    ///
    /// # Panics
    ///
    /// Panics if `arms` is empty, which no parse produces — `nvs_syntax`
    /// builds an [`ExprKind::Catch`] only after reading at least one `catch`.
    pub(crate) fn lower_catch(
        &mut self,
        guarded: &Expr,
        arms: &[CatchArm],
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            !arms.is_empty(),
            "an arm-less `catch` expression reached lowering — `nvs_syntax`'s `parse_catch` \
             produces `ExprKind::Catch` only once it has read a `catch` keyword"
        );
        let handler_block = self.new_block();
        let merge_block = self.new_block();
        let pre_env = env.clone();

        self.try_stack.push(TryFrame {
            handler: Some(handler_block),
            edges: Vec::new(),
            finally: None,
        });
        let mut guard_env = pre_env.clone();
        let mut guard_cur = *cur;
        let (guard_v, guard_ty) =
            self.lower_expr(guarded, expected, &mut guard_env, &mut guard_cur);
        let frame = self
            .try_stack
            .pop()
            .expect("just pushed this expression's own frame above");
        // The whole expression hands its value to a caller that will treat it
        // as fresh — `is_aliasing_read` names no `catch` — so a guard that is
        // itself a borrow owes the retain here, the same one a `match` arm
        // body owes ([`Self::lower_match`]).
        if guard_ty.is_refcounted() && self.aliasing_read(guarded) {
            self.emit_retain(guard_cur, guard_v);
        }
        let mut branches: Vec<(BlockId, ValueId, Ty)> = vec![(guard_cur, guard_v, guard_ty)];
        let mut incoming: Vec<(BlockId, Env)> = vec![(guard_cur, guard_env)];

        // The phis first, then the take: `nvs-codegen` requires a block's phis
        // to be its leading run, and `merge_envs` appends.
        let dispatch_env = self.merge_envs(handler_block, &frame.edges, &pre_env);
        let (thrown_v, _) = self.emit(handler_block, Ty::Object, InstKind::TakeThrown);
        let mut test_block = handler_block;
        for arm in arms {
            let caught = self.caught_class_label(&arm.ty);
            let (cond, _) = self.emit(
                test_block,
                Ty::Bool,
                InstKind::ClassTest {
                    value: thrown_v,
                    class: TestedClass::Named(caught),
                },
            );
            let taken = self.new_block();
            let next = self.new_block();
            let then_edge = self.ids.next_edge(arm.body.span);
            let else_edge = self.ids.next_edge(arm.body.span);
            self.seal(
                test_block,
                Terminator::Branch {
                    cond,
                    then_block: taken,
                    then_edge,
                    else_block: next,
                    else_edge,
                },
            );
            test_block = next;

            let mut arm_env = dispatch_env.clone();
            let mut arm_cur = taken;
            let bound = arm
                .var
                .map(|span| strip_sigil(span_text(self.src, span)).to_owned());
            match &bound {
                Some(name) => {
                    arm_env.insert(name.clone(), (thrown_v, Ty::Object));
                }
                // `catch (IOError) => …` names nothing, so the reference
                // `TakeThrown` produced has no slot to live in.
                None => self.emit_release(taken, thrown_v),
            }
            if let ExprKind::Throw(inner) = &arm.body.unparenthesized().kind {
                self.lower_throw(inner, &mut arm_env, &mut arm_cur);
                continue;
            }
            let (v, ty) = self.lower_expr(&arm.body, expected, &mut arm_env, &mut arm_cur);
            if ty.is_refcounted() && self.aliasing_read(&arm.body) {
                self.emit_retain(arm_cur, v);
            }
            if let Some(name) = &bound
                && let Some(&(bound_v, _)) = arm_env.get(name)
            {
                self.emit_release(arm_cur, bound_v);
                arm_env.remove(name);
            }
            branches.push((arm_cur, v, ty));
            incoming.push((arm_cur, arm_env));
        }

        // Nothing matched — the very same reference goes back to the context,
        // so the exception leaves this frame exactly as it arrived, `location`
        // included: handing a reference onward is not a second throw site, and
        // a `None` source is what leaves the first one's answer standing.
        let landing = self.landing_block(&dispatch_env);
        self.seal(
            test_block,
            Terminator::Throw {
                value: thrown_v,
                source: None,
                landing,
            },
        );

        // No branch is sealed until every one has a type: a branch that has to
        // widen into the representation the whole expression joins at needs
        // that widening in its own block, ahead of its jump.
        let ty = self.join_representations(&mut branches, env);
        for &(block, _, _) in &branches {
            self.seal(block, Terminator::Jump(merge_block));
        }
        *env = self.merge_envs(merge_block, &incoming, &pre_env);
        let (result, _) = self.emit(
            merge_block,
            ty,
            InstKind::Phi {
                incoming: branches.iter().map(|&(b, v, _)| (b, v)).collect(),
            },
        );
        *cur = merge_block;
        (result, ty)
    }
    /// The class label a `catch` clause or an
    /// `rule:expressions/catch-expression` arm tests against, as the rendered
    /// `QName` `nvs_types::layout` keys the descriptor table by.
    ///
    /// The checker records the answer, which is the route
    /// [`InstKind::ClassTest`] already takes for `$x is C`: every written
    /// annotation goes through `nvs_types::lower::lower_type`, which persists
    /// the resolved [`TypeId`] under the type's own span
    /// (`nvs_types::expr_table::ExprTypeTable::declared_ty`). So a name that
    /// means something only in its file — one an import brought into scope, or
    /// one written inside a `namespace` block — arrives here already resolved,
    /// where the source text would name a class neither descriptor table
    /// holds. `QName` is destructured rather than named, for
    /// `super::closure::declared_class`'s reason: `nvs-hir` is a
    /// dev-dependency of this crate.
    ///
    /// The written text is the fallback for a clause the checker recorded no
    /// type for, which is a lowering run without a checking pass in front of
    /// it.
    pub(crate) fn caught_class_label(&self, ty: &Type) -> String {
        if let Some(CheckedTy::Class(qname, _)) = self
            .exprs
            .declared_ty(ty.span)
            .map(|id| self.checked_types.get(id))
        {
            return qname.to_string();
        }
        span_text(self.src, ty.span).trim().to_owned()
    }
}

/// Whether reading `kind` produces a *borrowed* reference to storage some
/// other binding still owns, rather than a freshly constructed value with
/// exactly one natural owner — the same "is this a copy or a fresh value"
/// judgment [`Lowering::bind_local`]'s own doc comment already describes for
/// a bare variable read, shared with a call argument
/// ([`Lowering::lower_call_args`]) and a returned expression
/// (`StmtKind::Return`'s own arm). A plain local (`ExprKind::Variable`), a
/// compile-time-known property read (`ExprKind::PropertyAccess`), and a
/// compile-time-known array-element read (`ExprKind::Index`) all borrow
/// storage that keeps its own reference after this read — a local's own slot,
/// the object's field, or the array's own entry (`rule:types/arrays`'s copy-on-write
/// value semantics) — so copying any of them into a new durable slot needs a
/// retain. A fresh literal, `new`, or a call's own result already has exactly
/// one natural owner and needs none.
/// The root exception class's label — the one `nvs_types::layout` keys its own
/// slots under, and the one every `FieldGet`/`FieldSet` on an exception
/// resolves through.
///
/// A slot resolved against the root is valid for every subclass
/// (`nvs_runtime::object` lays a subclass's slots after its parent's), so
/// lowering never has to know which exception class it actually holds. The
/// tree's own home is `nvs_hir::errors`; this crate depends on neither
/// `nvs-hir` nor `nvs-types`' name resolution, so it restates the one label it
/// needs — `nvs-codegen`'s
/// `the_runtime_and_the_compiler_agree_on_every_throwable_slot` holds those
/// copies together.
pub(crate) const THROWABLE_ROOT: &str = "Throwable";

/// `Throwable::$message`.
pub(crate) const MESSAGE_FIELD: &str = "message";

/// `Throwable::$previous`.
pub(crate) const PREVIOUS_FIELD: &str = "previous";

/// `Throwable::$backtrace`.
pub(crate) const BACKTRACE_FIELD: &str = "backtrace";

/// `Throwable::$location`.
pub(crate) const LOCATION_FIELD: &str = "location";

/// The label the synthesized root constructor is compiled under — the target
/// a `new LogicError("…")` and a `parent::constructor(…)` in a user subclass
/// both resolve to.
pub(crate) const THROWABLE_CTOR: &str = "Throwable::constructor";

/// `LogicError`, spec § 10's class for a caller that broke a contract it could
/// have checked — the one exception this crate raises out of a body it
/// synthesized itself, from [`lower_generator_current`]'s protocol guard.
/// Restated here for [`PARSE_ERROR`]'s reason.
pub(crate) const LOGIC_ERROR: &str = "LogicError";

/// `ParseError`, a class below the root that declares a property of its own —
/// `rule:core-classes/derive-reports-every-field`'s `issues`.
/// `nvs_hir::errors::OWN_PROPERTIES` is that roster's home; this crate depends
/// on neither `nvs-hir` nor `nvs-types`, so it restates the names it needs.
pub(crate) const PARSE_ERROR: &str = "ParseError";

/// `ParseError::$issues`.
pub(crate) const ISSUES_FIELD: &str = "issues";

/// `Core\Db\RolledBack`, another class below the root that declares a
/// property — spec § 18's `reason`, thrown by
/// `rule:core-classes/db-transactions`'s
/// `Transaction::rollBack`. Restated here for [`PARSE_ERROR`]'s reason.
pub(crate) const ROLLED_BACK: &str = "Core\\Db\\RolledBack";

/// `Core\Db\RolledBack::$reason`.
pub(crate) const REASON_FIELD: &str = "reason";

/// `Core\Db\DbError`, another such class — spec § 18's `kind`, written by
/// every refusal `rule:core-classes/db-error` gives a
/// normalised kind. Restated here for [`PARSE_ERROR`]'s reason.
pub(crate) const DB_ERROR: &str = "Core\\Db\\DbError";

/// `Core\Db\DbError::$kind`.
pub(crate) const KIND_FIELD: &str = "kind";

/// The `Core\Db\ErrorKind::Other` ordinal — `nvs_stdlib::db::ERROR_KIND` is
/// that roster's home and this crate depends on it no more than it depends on
/// `nvs-hir`, so the one value it needs is restated for [`PARSE_ERROR`]'s
/// reason. `nvs_types::error_lib`'s `the_kind_property_names_a_registered_enum`
/// pins the ordinal against that roster, being the one crate that can see both.
pub(crate) const ERROR_KIND_OTHER: i64 = 10;

/// The Novis functions with no source text: one constructor per exception class
/// that declares state of its own.
///
/// They cannot be written in Novis — `backtrace` is grown by the runtime as a
/// throw propagates, so a source declaration would need a body with no legal
/// spelling (`nvs_hir::errors` owns that reasoning). What each does is small
/// enough to build by hand: store the message and the `previous` the options
/// bag flattened into parameter 2, start an empty backtrace, and put a
/// placeholder in `location` that `nvs_runtime::nvs_raise` overwrites from the
/// carrier [`Lowering::throw_source`] hands it at the `throw`.
///
/// Parameter 2 is `Ty::Tagged` because spec § 10 types the option
/// `Throwable|null`, and a bag omitted whole flattens to that option's own
/// `null` default — so the slot is written on every path and `rule:classes/definite-property-initialization`'s
/// definite assignment holds without a branch here.
///
/// `ParseError`, `Core\Db\DbError` and `Core\Db\RolledBack` each get one of
/// their own rather than inheriting the root's, because each declares a
/// property the root's constructor never touches — `rule:classes/definite-property-initialization` makes every
/// property definitely assigned, and such a slot would read `null` out of a
/// type that cannot be one. Each writes every slot itself rather than chaining,
/// which duplicates a handful of stores and buys not needing a call at all on a
/// path that allocates an exception.
///
/// **What each extra slot is initialized to is [`ExtraInit`]'s decision**, and
/// each differs: `ParseError::$issues` starts empty, because a `ParseError`
/// raised by hand has no field list to report and
/// `rule:core-classes/derive-reports-every-field`'s decoder fills
/// it from native code. `Core\Db\RolledBack::$reason` starts as **the message**,
/// because spec § 18 gives that class nothing else to carry: the one string a
/// caller passes is the reason, so `new Core\Db\RolledBack("cart is empty")`
/// and `rule:core-classes/db-transactions`'s `rollBack("cart is empty")` agree without the thrower
/// having to write a second slot. `Core\Db\DbError::$kind` starts at
/// [`ERROR_KIND_OTHER`], which is the honest answer for an error no server
/// classified: `rule:core-classes/db-error` defines `Other` as the condition a driver's own
/// code table does not name, and a `DbError` a program constructed itself has
/// no code table behind it at all. `nvs_stdlib::db`'s `statement_failure`
/// overwrites the slot on the path that *does* have one.
pub(crate) fn synthesized_exception_constructors() -> Vec<Function> {
    vec![
        exception_constructor(THROWABLE_ROOT, &[]),
        exception_constructor(PARSE_ERROR, &[(ISSUES_FIELD, ExtraInit::EmptyArray)]),
        exception_constructor(
            DB_ERROR,
            &[(KIND_FIELD, ExtraInit::EnumCase(ERROR_KIND_OTHER))],
        ),
        exception_constructor(ROLLED_BACK, &[(REASON_FIELD, ExtraInit::Message)]),
    ]
}

/// What [`exception_constructor`] stores into one property a subclass declares
/// beyond the root's own.
///
/// A closed set rather than a value the caller builds: every one of these has
/// to be a definite assignment `rule:classes/definite-property-initialization` accepts *and* a representation the
/// class's seeded type admits (`nvs_types::error_lib::own_properties` is where
/// that type is), so another initializer is a deliberate addition here rather
/// than an instruction written at a call site.
#[derive(Clone, Copy)]
enum ExtraInit {
    /// A fresh empty `array`.
    EmptyArray,
    /// The `$message` parameter, retained a second time — the slot is a second
    /// durable owner of the same string.
    Message,
    /// One enum case, by its ordinal. `rule:enums/one-backing-type` represents an enum as its
    /// backing integer and [`Ty::Enum`] is that representation, so there is
    /// nothing to retain and nothing to release: the slot owns a scalar.
    EnumCase(i64),
}

/// One such constructor: the root's own slots, then one per `(field, init)`
/// pair in `extra` — which is every property `class` declares beyond them.
///
/// The receiver, the message and the `previous` option are all *transferred*
/// to this frame by the call convention, so all three are released at the exit
/// — each field takes its own reference first. A tagged `null` retains and
/// releases as a no-op, which `nvs_runtime::nvs_value_retain` decides at run
/// time rather than this lowering deciding it here.
fn exception_constructor(class: &str, extra: &[(&str, ExtraInit)]) -> Function {
    let mut ids = IdGen::default();
    let block = ids.next_block();
    let this = ids.next_value();
    let message = ids.next_value();
    let previous = ids.next_value();
    let backtrace = ids.next_value();
    let location = ids.next_value();

    let plain = |kind: InstKind| Inst {
        result: None,
        ty: None,
        kind,
        on_error: None,
        raise_site: None,
    };
    let defines = |result: ValueId, ty: Ty, kind: InstKind| Inst {
        result: Some(result),
        ty: Some(ty),
        kind,
        on_error: None,
        raise_site: None,
    };
    // Every store names `class` rather than the declaring one: a slot is
    // resolved against the *layout* of the label written here, and a subclass's
    // layout holds every inherited field at the ancestor's own index.
    let store = |field: &str, value: ValueId| {
        plain(InstKind::FieldSet {
            object: this,
            class: class.to_owned(),
            field: field.to_owned(),
            value,
        })
    };
    let empty_array = || InstKind::ArrayNew {
        entries: Vec::new(),
    };

    let mut insts = vec![
        plain(InstKind::Safepoint),
        defines(this, Ty::Object, InstKind::Param(0)),
        defines(message, Ty::Str, InstKind::Param(1)),
        plain(InstKind::Retain { operand: message }),
        store(MESSAGE_FIELD, message),
        defines(previous, Ty::Tagged, InstKind::Param(2)),
        plain(InstKind::Retain { operand: previous }),
        store(PREVIOUS_FIELD, previous),
        defines(backtrace, Ty::Array, empty_array()),
        store(BACKTRACE_FIELD, backtrace),
        defines(location, Ty::Str, InstKind::ConstStr(String::new())),
        store(LOCATION_FIELD, location),
    ];
    for &(field, init) in extra {
        match init {
            ExtraInit::EmptyArray => {
                let value = ids.next_value();
                insts.push(defines(value, Ty::Array, empty_array()));
                insts.push(store(field, value));
            }
            // The slot takes its own reference, exactly as `MESSAGE_FIELD`
            // above does — the frame still releases the transferred `message`
            // at the exit, and both fields outlive it.
            ExtraInit::Message => {
                insts.push(plain(InstKind::Retain { operand: message }));
                insts.push(store(field, message));
            }
            ExtraInit::EnumCase(ordinal) => {
                let value = ids.next_value();
                insts.push(defines(
                    value,
                    Ty::Enum(EnumRepr::Int),
                    InstKind::ConstInt(ordinal),
                ));
                insts.push(store(field, value));
            }
        }
    }
    insts.push(plain(InstKind::Release { operand: message }));
    insts.push(plain(InstKind::Release { operand: previous }));
    insts.push(plain(InstKind::Release { operand: this }));

    let (stmt_spans, edge_spans) = ids.into_spans();
    Function {
        name: format!("{class}::constructor"),
        params: vec![Ty::Object, Ty::Str, Ty::Tagged],
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
