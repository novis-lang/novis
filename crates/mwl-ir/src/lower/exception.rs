//! ADR 0002's throw/catch lowering: the landing pads, the `finally` ladder, and the synthesized `Throwable` constructor.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

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
    /// # Panics
    ///
    /// Panics naming the operand's representation if it is not a
    /// [`Ty::Object`] — the checker has already refused throwing anything but
    /// a `Throwable` subclass, so anything else here is a lowering bug.
    pub(super) fn lower_throw(&mut self, inner: &Expr, env: &mut Env, cur: &mut BlockId) {
        let (v, ty) = self.lower_expr(inner, None, env, cur);
        assert!(
            matches!(ty, Ty::Object),
            "mwl-ir lowers `throw` only for an exception object — got representation {ty:?}; \
             see the crate docs' known gaps"
        );
        if self.aliasing_read(inner) {
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
    pub(super) fn write_throw_location(&mut self, cur: BlockId, thrown: ValueId) {
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
    ///   `catch` clause whose *own body* then throws: the frame the clause
    ///   body is lowered under names no handler, so that throw reaches the
    ///   enclosing region directly. PHP runs the `finally` first. A `return`
    ///   out of a clause body *does* run it — that frame carries `finally`
    ///   for exactly that reason.
    ///
    /// A `break`/`continue` out of a protected region *does* run the pending
    /// `finally` of every region it leaves, bounded by the loop it targets —
    /// [`Self::run_finallys_above`].
    pub(super) fn lower_try(
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
    pub(super) fn lower_catch_clauses(
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
            // A `return` inside the clause body still owes this region's
            // `finally` — so the body is lowered under a frame that carries
            // it. The frame names no handler: a throw from a `catch` body is
            // the enclosing region's, not this clause list's own.
            self.try_stack.push(crate::lower::TryFrame {
                handler: None,
                edges: Vec::new(),
                finally,
            });
            self.lower_stmts(&clause.body.stmts, &mut handler_cur, &mut handler_env);
            self.try_stack
                .pop()
                .expect("just pushed this clause's own frame above");
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
    pub(super) fn run_pending_finallys(&mut self, cur: &mut BlockId, env: &mut Env) {
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
    pub(super) fn run_finallys_above(&mut self, depth: usize, cur: &mut BlockId, env: &mut Env) {
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
    pub(super) fn catch_clause_type(&self, clause: &CatchClause) -> String {
        span_text(self.src, clause.ty.span)
            .trim()
            .trim_start_matches('\\')
            .to_owned()
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
pub(super) const THROWABLE_ROOT: &str = "Throwable";

/// `Throwable::$message`.
pub(super) const MESSAGE_FIELD: &str = "message";

/// `Throwable::$previous`.
pub(super) const PREVIOUS_FIELD: &str = "previous";

/// `Throwable::$backtrace`.
pub(super) const BACKTRACE_FIELD: &str = "backtrace";

/// `Throwable::$location`.
pub(super) const LOCATION_FIELD: &str = "location";

/// The label the synthesized root constructor is compiled under — the target
/// a `new LogicError("…")` and a `parent::constructor(…)` in a user subclass
/// both resolve to.
pub(super) const THROWABLE_CTOR: &str = "Throwable::constructor";

/// `ParseError`, the one class below the root that declares a property —
/// [ADR 0071](../../../../docs/adr/0071-derived-codecs.md) § 5's `issues`.
/// `mwl_hir::errors::OWN_PROPERTIES` is that roster's home; this crate depends
/// on neither `mwl-hir` nor `mwl-types`, so it restates the two names it needs.
pub(super) const PARSE_ERROR: &str = "ParseError";

/// `ParseError::$issues`.
pub(super) const ISSUES_FIELD: &str = "issues";

/// The MWL functions with no source text: one constructor per exception class
/// that declares state of its own.
///
/// They cannot be written in MWL — `backtrace` is grown by the runtime as a
/// throw propagates, so a source declaration would need a body with no legal
/// spelling (`mwl_hir::errors` owns that reasoning). What each does is small
/// enough to build by hand: store the message and the `previous` the options
/// bag flattened into parameter 2, start an empty backtrace, and put a
/// placeholder in `location` that [`Lowering::write_throw_location`]
/// overwrites at the `throw`.
///
/// Parameter 2 is `Ty::Tagged` because spec § 10 types the option
/// `Throwable|null`, and a bag omitted whole flattens to that option's own
/// `null` default — so the slot is written on every path and ADR 0022's
/// definite assignment holds without a branch here.
///
/// `ParseError` gets a second one rather than inheriting the root's, because
/// [ADR 0071](../../../../docs/adr/0071-derived-codecs.md) § 5 gives it an
/// `issues` list declared `array<Issue>`: ADR 0022 makes every property
/// definitely assigned, and a slot the root's constructor never touches would
/// read `null` out of a type that cannot be one. It writes all five slots
/// rather than chaining, which costs three duplicated instructions and buys
/// not needing a call at all on a path that allocates an exception.
pub(super) fn synthesized_exception_constructors() -> Vec<Function> {
    vec![
        exception_constructor(THROWABLE_ROOT, &[]),
        exception_constructor(PARSE_ERROR, &[ISSUES_FIELD]),
    ]
}

/// One such constructor: the root's four slots, then one empty `array` per
/// name in `extra` — which is every property `class` declares beyond them.
///
/// The receiver, the message and the `previous` option are all *transferred*
/// to this frame by the call convention, so all three are released at the exit
/// — each field takes its own reference first. A tagged `null` retains and
/// releases as a no-op, which `mwl_runtime::mwl_value_retain` decides at run
/// time rather than this lowering deciding it here.
fn exception_constructor(class: &str, extra: &[&str]) -> Function {
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
    };
    let defines = |result: ValueId, ty: Ty, kind: InstKind| Inst {
        result: Some(result),
        ty: Some(ty),
        kind,
        on_error: None,
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
    for field in extra {
        let value = ids.next_value();
        insts.push(defines(value, Ty::Array, empty_array()));
        insts.push(store(field, value));
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
