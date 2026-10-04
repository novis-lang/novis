//! Control flow — `if`, `while`, `for`, both `foreach` shapes, `switch`, `break`/`continue`, and the env merge every join needs.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. The methods
//! are `pub(crate)` so they reach across these modules and no further, which
//! is the reach a single-file `lower` would give them.

use super::*;

impl<'a> Lowering<'a> {
    /// `if (cond) then (elseif (cond2) then2)* (else else_)?` — the module
    /// docs describe the merge-point construction this drives.
    ///
    /// Each arm's `else` is the rest of the chain, with a merge block of its
    /// own. The arms are lowered in order first, each one's branch and
    /// `then` side, keeping a frame per arm. Then the frames are merged from
    /// the last arm back to the first. That is the order a nested `If` per
    /// arm would give, block for block, and it costs no stack per arm.
    ///
    /// # Panics
    ///
    /// Panics naming the case for a `cond` whose static type
    /// [`Self::lower_truthy_cond`] doesn't yet convert — see that method's
    /// own doc comment for exactly what's covered and what still isn't
    /// (`null`/`mixed`/a union).
    pub(crate) fn lower_if(
        &mut self,
        arms: &'a [nvs_syntax::ast::IfArm],
        else_: Option<&'a Stmt>,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        // The last block of an arm's `then` side and its environment, when it
        // reaches the merge.
        type ThenSide = Option<(BlockId, Env)>;
        // One per arm: its merge block, its `then` side, and the environment
        // its condition left behind.
        let mut frames: Vec<(BlockId, ThenSide, Env)> = Vec::with_capacity(arms.len());
        for (i, arm) in arms.iter().enumerate() {
            // Every arm after the first is a statement of its own to coverage
            // and to the statement counts, as a nested `If` would be.
            if i > 0 {
                let stmt_id = self.ids.next_stmt(arm.span);
                self.cur_stmt_span = arm.span;
                self.block_insts[cur.index() as usize].push(Inst {
                    result: None,
                    ty: None,
                    kind: InstKind::StmtMarker(stmt_id),
                    on_error: None,
                    raise_site: None,
                });
            }
            let cond_v = self.lower_truthy_cond(&arm.cond, env, cur);

            let merge_block = self.new_block();
            let then_block = self.new_block();
            let then_edge = self.ids.next_edge(arm.then.span);
            let rest_span = arms
                .get(i + 1)
                .map(|next| next.span)
                .or(else_.map(|s| s.span));
            let (else_block, else_edge) = match rest_span {
                Some(span) => (self.new_block(), self.ids.next_edge(span)),
                // No `else`: the branch's false edge goes straight to the merge
                // block, carrying the pre-branch environment unchanged.
                None => (merge_block, self.ids.next_edge(arm.cond.span)),
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

            let mut then_env = env.clone();
            let mut then_cur = then_block;
            self.lower_stmt(&arm.then, &mut then_cur, &mut then_env);
            let then_side = (!self.is_terminated(then_cur)).then(|| {
                self.seal(then_cur, Terminator::Jump(merge_block));
                (then_cur, then_env)
            });
            frames.push((merge_block, then_side, env.clone()));
            // The rest of the chain starts on the false edge. With no rest,
            // that edge leaves the branch's own block.
            if rest_span.is_some() {
                *cur = else_block;
            }
        }

        // `cur` and `env` are now the false side of the last arm. Only a real
        // `else` has a block of its own to close with a jump.
        let mut owns_its_block = false;
        if let Some(else_stmt) = else_ {
            self.lower_stmt(else_stmt, cur, env);
            owns_its_block = true;
        }
        while let Some((merge_block, then_side, entry_env)) = frames.pop() {
            let rest_reaches_merge = !owns_its_block || !self.is_terminated(*cur);
            if owns_its_block && rest_reaches_merge {
                self.seal(*cur, Terminator::Jump(merge_block));
            }
            let mut incoming = Vec::new();
            incoming.extend(then_side);
            if rest_reaches_merge {
                incoming.push((*cur, env.clone()));
            }
            *env = self.merge_envs(merge_block, &incoming, &entry_env);
            *cur = merge_block;
            owns_its_block = true;
        }
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
    pub(crate) fn lower_while(
        &mut self,
        cond: &Expr,
        body: &'a Stmt,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let mut seen = FxHashSet::default();
        let mut reassigned = Vec::new();
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);
        // The condition is re-evaluated on every iteration and may re-point a
        // local itself — `while ($i++ < 3)` is the whole loop counter — so it
        // owes a header phi exactly as the body does. Missing it leaves the
        // increment reading the pre-loop value forever.
        self.collect_reassigned_in_expr(cond, &mut seen, &mut reassigned);
        self.seed_generator_loop_carried(env, &mut seen, &mut reassigned);

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
            // An `inout $x` parameter's binding is an address that never changes:
            // writing to it stores *through* it rather than rebinding it
            // (`Ty::Ref`), so a header phi for one would carry the same value
            // on both edges and describe nothing.
            if ty == Ty::Ref {
                continue;
            }
            let phi_v = self.ids.next_value();
            let inst_index = self.block_insts[header_block.index() as usize].len();
            self.block_insts[header_block.index() as usize].push(Inst {
                result: Some(phi_v),
                ty: Some(ty),
                kind: InstKind::Phi {
                    incoming: vec![(pre_block, pre_v)],
                },
                on_error: None,
                raise_site: None,
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
        let cond_v = self.lower_truthy_cond(cond, &mut header_env, &mut cond_end);

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
            continue_target: Some(header_block),
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned: Vec::new(),
            carried: header_env.keys().cloned().collect(),
            loop_private: Vec::new(),
            try_depth: self.try_stack.len(),
        });
        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        self.lower_stmt(body, &mut body_cur, &mut body_env);
        // The body's own fall-through exit ends an iteration exactly as a
        // `continue` does, so it owes the same releases — see
        // `Self::end_iteration`. Emitted before the frame is popped, since
        // that is where the loop's carried set lives.
        let reaches_back_edge = !self.is_terminated(body_cur);
        if reaches_back_edge {
            self.end_iteration(body_cur, &mut body_env);
        }
        let frame = self
            .loop_stack
            .pop()
            .expect("just pushed this loop's own frame above");

        // Every edge that loops back to the header: the body's own
        // fall-through exit (if it reaches one) plus one more per `continue`
        // recorded while lowering the body above.
        let mut back_edges: Vec<(BlockId, Env)> = Vec::new();
        if reaches_back_edge {
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
                        "nvs-ir: `{name}` was reassigned in a while body per the syntactic scan \
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
        // edge (carrying `header_env` as the condition left it — an increment
        // written into the header, `while ($i++ < 3)`, has already re-pointed
        // it there) plus one more incoming edge per `break` recorded above.
        // With no `break` to fold in, `Self::merge_envs` degenerates to a
        // plain clone with no new phi at all, so the loop exit is simply
        // `header_env`.
        let mut after_incoming: Vec<(BlockId, Env)> = vec![(cond_end, header_env.clone())];
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &header_env);
        *cur = after_block;
    }
    /// `do body while (cond);` — [`Self::lower_while`] with the condition
    /// moved to the bottom, which changes what follows and nothing else.
    ///
    /// * **The header *is* the body's first block.** A `while` needs a block
    ///   of its own to evaluate the condition in before the body is entered;
    ///   a `do` enters the body unconditionally, so the loop-carried phis sit
    ///   at the top of the body itself and the pre-loop edge falls straight
    ///   into them. That is the whole of "the body runs before the condition
    ///   does" — there is no guard to skip.
    /// * **`continue` targets the condition, not the header.** PHP's `continue`
    ///   in a `do`/`while` re-tests the condition rather than restarting the
    ///   body, so the condition gets a block of its own that every way an
    ///   iteration can end flows into — the body's fall-through and one edge
    ///   per `continue` — exactly the role [`Self::lower_for`]'s step block
    ///   plays, and merged the same way. The header therefore sees **one**
    ///   back edge, from wherever the condition's evaluation ends.
    ///
    /// The safepoint poll sits with the condition for [`Self::lower_for`]'s
    /// reason: an iteration that never completes (the body always `return`s)
    /// polls zero times.
    ///
    /// # Panics
    ///
    /// Panics for the cases [`Self::lower_truthy_cond`] already names, the
    /// same restriction [`Self::lower_while`] carries.
    pub(crate) fn lower_do_while(
        &mut self,
        body: &'a Stmt,
        cond: &Expr,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let mut seen = FxHashSet::default();
        let mut reassigned = Vec::new();
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);
        // The condition is re-evaluated per iteration and may re-point a local
        // itself, so it owes a header phi just as the body does — see
        // `Self::lower_while`, which collects the same set for the same reason.
        self.collect_reassigned_in_expr(cond, &mut seen, &mut reassigned);
        self.seed_generator_loop_carried(env, &mut seen, &mut reassigned);

        let pre_block = *cur;
        let header_block = self.new_block();
        self.seal(pre_block, Terminator::Jump(header_block));

        // Seeded exactly as `Self::lower_while` seeds them, and patched below
        // once the condition block's exit environment is known — see that
        // method for why a `Ty::Ref` binding is skipped.
        let mut header_env = env.clone();
        let mut phi_slots: Vec<(String, usize)> = Vec::new();
        for name in &reassigned {
            let Some(&(pre_v, ty)) = env.get(name) else {
                continue;
            };
            if ty == Ty::Ref {
                continue;
            }
            let phi_v = self.ids.next_value();
            let inst_index = self.block_insts[header_block.index() as usize].len();
            self.block_insts[header_block.index() as usize].push(Inst {
                result: Some(phi_v),
                ty: Some(ty),
                kind: InstKind::Phi {
                    incoming: vec![(pre_block, pre_v)],
                },
                on_error: None,
                raise_site: None,
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

        let cond_block = self.new_block();
        let after_block = self.new_block();

        self.loop_stack.push(LoopFrame {
            continue_target: Some(cond_block),
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned: Vec::new(),
            carried: header_env.keys().cloned().collect(),
            loop_private: Vec::new(),
            try_depth: self.try_stack.len(),
        });
        // The body is lowered *into* the header, after the phis already
        // standing at its top — which is what makes this a `do` rather than a
        // `while` with the same blocks.
        let mut body_env = header_env.clone();
        let mut body_cur = header_block;
        self.lower_stmt(body, &mut body_cur, &mut body_env);
        let reaches_cond = !self.is_terminated(body_cur);
        if reaches_cond {
            self.end_iteration(body_cur, &mut body_env);
        }
        let frame = self
            .loop_stack
            .pop()
            .expect("just pushed this loop's own frame above");

        let mut cond_incoming: Vec<(BlockId, Env)> = Vec::new();
        if reaches_cond {
            self.seal(body_cur, Terminator::Jump(cond_block));
            cond_incoming.push((body_cur, body_env));
        }
        cond_incoming.extend(frame.continue_edges);

        let (cond_end, cond_env) = if cond_incoming.is_empty() {
            // Nothing reaches the condition — the body always `return`s,
            // throws or `break`s, so it is never tested. `Self::finish`
            // insists every block carries a terminator, and the header phis
            // still need an entry for this edge, so it jumps back with the
            // environment the loop was *entered* with: those values are
            // defined before the header, which is the one thing that stays
            // true on a block nothing can reach. `Self::lower_for` handles an
            // unreachable step block the same way and for the same reason.
            self.seal(cond_block, Terminator::Jump(header_block));
            (cond_block, env.clone())
        } else {
            let mut cond_env = self.merge_envs(cond_block, &cond_incoming, &header_env);
            let mut cond_end = cond_block;
            let cond_v = self.lower_truthy_cond(cond, &mut cond_env, &mut cond_end);
            // Reserved safepoint poll site (loop back edge) — see
            // `InstKind::Safepoint`'s own doc comment.
            self.emit_safepoint(cond_end);
            let body_edge = self.ids.next_edge(body.span);
            let after_edge = self.ids.next_edge(cond.span);
            self.seal(
                cond_end,
                Terminator::Branch {
                    cond: cond_v,
                    then_block: header_block,
                    then_edge: body_edge,
                    else_block: after_block,
                    else_edge: after_edge,
                },
            );
            (cond_end, cond_env)
        };

        for (name, inst_index) in &phi_slots {
            let &(back_v, _) = cond_env.get(name).unwrap_or_else(|| {
                panic!(
                    "nvs-ir: `{name}` was reassigned in a do/while body or condition per the \
                     syntactic scan but is missing from the back edge's exit environment — bug \
                     in collect_reassigned_locals"
                )
            });
            let inst = &mut self.block_insts[header_block.index() as usize][*inst_index];
            let InstKind::Phi { incoming } = &mut inst.kind else {
                unreachable!("phi_slots only ever indexes a Phi instruction");
            };
            incoming.push((cond_end, back_v));
        }

        // The loop's exit environment: the condition's false edge, plus one
        // more incoming edge per `break`. A condition nothing reaches
        // contributes no edge at all, and `Self::merge_envs` then degenerates
        // to the pre-loop environment on a block nothing can reach either.
        let mut after_incoming: Vec<(BlockId, Env)> = Vec::new();
        if !cond_incoming.is_empty() {
            after_incoming.push((cond_end, cond_env));
        }
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &header_env);
        *cur = after_block;
    }
    /// `for (init; cond; step) body` — [`Self::lower_while`]'s exact shape
    /// with two clauses bolted onto it, and one structural consequence.
    ///
    /// * **`init` runs once, before the header exists**, so it is lowered
    ///   straight into `cur` against the caller's own `env`: whatever it binds
    ///   or re-points is simply part of the environment the loop is entered
    ///   with, and needs no phi of its own beyond the one every reassigned
    ///   local already gets.
    /// * **`step` runs at the end of every iteration**, which is what makes a
    ///   `for` structurally different from a `while` rather than sugar over
    ///   one: `continue` has to run it too. So the loop gets a **step block**
    ///   between the body and the header, and that — not the header — is what
    ///   [`LoopFrame::header_block`] points a `continue` at. Every way an
    ///   iteration can end (the body's fall-through, each `continue`) is an
    ///   incoming edge to it, merged by the same [`Self::merge_envs`] a join
    ///   uses; the step clause is then lowered *once*, over the merged
    ///   environment, and the single back edge to the header leaves from it.
    /// * **The header phis therefore see one back edge**, from the step block,
    ///   rather than one per `continue` — the only place this differs from
    ///   [`Self::lower_while`]'s patch loop, which is otherwise identical.
    ///
    /// A `continue` still emits its own safepoint poll ([`Self::lower_continue`])
    /// and then reaches the step block's, so that path polls twice per
    /// iteration. A poll is a load and a predicted-not-taken branch
    /// ([`InstKind::Safepoint`]), and paying it twice on the rarer edge is
    /// cheaper than teaching `continue` which loop shape it sits in.
    ///
    /// * **A comma list in the condition clause decides on its last
    ///   expression**, which is PHP's rule. Every expression before that one is
    ///   lowered into the header as the statement it is, so an assignment
    ///   written there is visible to the test beside it and to the body, and it
    ///   runs on every trip through the header including the one that fails the
    ///   test. The phi seeding above walks the whole clause, so a local such an
    ///   expression writes carries across the back edge like any other.
    ///
    /// # Panics
    ///
    /// Panics for the cases [`Self::lower_expr_stmt`] and
    /// [`Self::lower_truthy_cond`] already name.
    pub(crate) fn lower_for(
        &mut self,
        init: &'a ForInit,
        cond: &[Expr],
        step: &[Expr],
        body: &'a Stmt,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        // `rule:iteration/for-counter-scope`: the declaration form lowers as the statement it is,
        // into the pre-header block — the same slot store the line above the
        // loop produced, at the same point.
        match init {
            ForInit::Decl(decl) => self.lower_stmt(decl, cur, env),
            ForInit::Exprs(exprs) => {
                for e in exprs {
                    self.lower_expr_stmt(e, env, cur);
                }
            }
        }

        let mut seen = FxHashSet::default();
        let mut reassigned = Vec::new();
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);
        // The step and the condition alike — see `Self::lower_while`, which
        // owes the same phi for the same reason.
        for e in step.iter().chain(cond) {
            self.collect_reassigned_in_expr(e, &mut seen, &mut reassigned);
        }
        self.seed_generator_loop_carried(env, &mut seen, &mut reassigned);

        let pre_block = *cur;
        let header_block = self.new_block();
        self.seal(pre_block, Terminator::Jump(header_block));

        // Seeded exactly as `Self::lower_while` seeds them, and patched below
        // once the step block's exit environment is known — see that method
        // for why a `Ty::Ref` binding is skipped.
        let mut header_env = env.clone();
        let mut phi_slots: Vec<(String, usize)> = Vec::new();
        for name in &reassigned {
            let Some(&(pre_v, ty)) = env.get(name) else {
                continue;
            };
            if ty == Ty::Ref {
                continue;
            }
            let phi_v = self.ids.next_value();
            let inst_index = self.block_insts[header_block.index() as usize].len();
            self.block_insts[header_block.index() as usize].push(Inst {
                result: Some(phi_v),
                ty: Some(ty),
                kind: InstKind::Phi {
                    incoming: vec![(pre_block, pre_v)],
                },
                on_error: None,
                raise_site: None,
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

        // An omitted condition is PHP's `for (;;)` — an unconditional loop,
        // spelled here as the constant the branch then tests, so the block
        // shape stays the one every other loop builds.
        let mut cond_end = header_block;
        let cond_v = match cond.split_last() {
            Some((test, before)) => {
                for e in before {
                    self.lower_expr_stmt(e, &mut header_env, &mut cond_end);
                }
                self.lower_truthy_cond(test, &mut header_env, &mut cond_end)
            }
            None => {
                self.emit(header_block, Ty::Bool, InstKind::ConstBool(true))
                    .0
            }
        };

        let body_block = self.new_block();
        let step_block = self.new_block();
        let after_block = self.new_block();
        let body_edge = self.ids.next_edge(body.span);
        let after_edge = self
            .ids
            .next_edge(cond.last().map_or(body.span, |c| c.span));
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
            continue_target: Some(step_block),
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned: Vec::new(),
            carried: header_env.keys().cloned().collect(),
            loop_private: Vec::new(),
            try_depth: self.try_stack.len(),
        });
        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        self.lower_stmt(body, &mut body_cur, &mut body_env);
        let reaches_step = !self.is_terminated(body_cur);
        if reaches_step {
            self.end_iteration(body_cur, &mut body_env);
        }
        let frame = self
            .loop_stack
            .pop()
            .expect("just pushed this loop's own frame above");

        let mut step_incoming: Vec<(BlockId, Env)> = Vec::new();
        if reaches_step {
            self.seal(body_cur, Terminator::Jump(step_block));
            step_incoming.push((body_cur, body_env));
        }
        step_incoming.extend(frame.continue_edges);

        let back_edges: Vec<(BlockId, Env)> = if step_incoming.is_empty() {
            // Nothing reaches the step block — the body always `return`s,
            // throws or `break`s, so no iteration ever completes. It still
            // needs a terminator (`Self::finish` insists every block has one),
            // and the header phis still need an entry for it, so it jumps
            // back with the environment the loop was *entered* with: those
            // values are defined before the header, which is the one thing
            // that stays true on a block nothing can reach.
            self.seal(step_block, Terminator::Jump(header_block));
            vec![(step_block, env.clone())]
        } else {
            let mut step_env = self.merge_envs(step_block, &step_incoming, &header_env);
            let mut step_cur = step_block;
            for e in step {
                self.lower_expr_stmt(e, &mut step_env, &mut step_cur);
            }
            // Reserved safepoint poll site (loop back edge) — placed here for
            // `Self::lower_while`'s reason, on the edge itself rather than the
            // header, so an iteration that never completes polls zero times.
            self.emit_safepoint(step_cur);
            self.seal(step_cur, Terminator::Jump(header_block));
            vec![(step_cur, step_env)]
        };

        for (name, inst_index) in &phi_slots {
            for (block, back_env) in &back_edges {
                let &(back_v, _) = back_env.get(name).unwrap_or_else(|| {
                    panic!(
                        "nvs-ir: `{name}` was reassigned in a for body or step per the syntactic \
                         scan but is missing from the back edge's exit environment — bug in \
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

        let mut after_incoming: Vec<(BlockId, Env)> = vec![(cond_end, header_env.clone())];
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &header_env);
        *cur = after_block;
    }
    /// `switch (subject) { case a: … default: … }` — one subject, an equality
    /// chain, and bodies that fall into each other because nothing separates
    /// them.
    ///
    /// **A chain of [`Terminator::Branch`]es, not [`Terminator::Switch`].**
    /// The general terminator exists (a generator's resumption dispatch is
    /// one), but it selects on an integer and a `case` label is any expression
    /// of the subject's type — `case "A":` is the shape `examples/match.nvs`
    /// actually writes, and a string comparison is a runtime call
    /// (`nvs_str_eq`), not a jump-table index. One shape that serves every
    /// subject type beats two that need the lowering to decide which it is;
    /// re-deriving a dense integer `switch` back into the jump table is an
    /// optimisation for the tier that has a cost model, not for this one.
    ///
    /// These follow from PHP's own semantics:
    ///
    /// * **The subject is evaluated once**, before any label is, and every
    ///   label is then compared against that value in source order. The
    ///   comparison is [`BinOp::Eq`] — PHP's `switch` compared loosely and its
    ///   `match` identically, but `rule:expressions/switch-match-equality` gives Novis one equality rule for
    ///   both, and both operands are statically the same Novis type here anyway,
    ///   which is the one condition under which PHP's two agreed; see
    ///   [`Self::lower_expr`]'s `Binary` arm, which lowers the operator the
    ///   same way for the same reason.
    /// * **`default` is the chain's fall-off, wherever it is written.** Every
    ///   label is tried first; only then does control reach the `default`
    ///   body, so a `default` written in the middle still runs last — and
    ///   still falls through into the case *after* it, which is why the body
    ///   blocks stay in source order while the test chain skips it.
    /// * **Fallthrough is the absence of a `break`**, so a body that reaches
    ///   its end jumps to the next body block rather than past the switch.
    ///   That edge is an ordinary join, merged by [`Self::merge_envs`] like
    ///   any other — which is also what releases a local the falling-through
    ///   case declared, since the next body's own label edge does not bind it.
    /// * **A `switch` is a `break` target without being a loop**, so it pushes
    ///   a [`LoopFrame`] whose [`LoopFrame::continue_target`] is `None`. A
    ///   `continue` inside one therefore continues the enclosing **loop**.
    ///   PHP instead counts a `switch` as a looping structure there, making a
    ///   bare `continue` behave as `break` — and warns, since PHP 7.3, that
    ///   you probably meant `continue 2`. Novis takes the meaning that warning
    ///   points at: the alternative is a keyword that silently means one thing
    ///   inside a `switch` and another everywhere else, which priority 4
    ///   (simplicity of the language surface) refuses to buy for a
    ///   compatibility PHP itself discourages.
    ///
    /// The subject's own reference is held exactly the way
    /// [`Self::lower_foreach`] holds the array it walks: retained if it
    /// [`is_aliasing_read`]s a slot, parked in the [`Env`] under a reserved
    /// `switch#N` name so a `return` or a throw out of a case body sweeps it
    /// ([`Self::release_all_locals`]), hidden from everything after the switch
    /// by [`LoopFrame::loop_private`], and released once in the after-block —
    /// which every path out of the switch reaches, so that release runs
    /// exactly once. It costs one refcount pair per `switch` over a
    /// refcounted subject, and nothing at all over an `int`/`bool` one.
    ///
    /// Every label is compared through [`Self::emit_equality`], the one
    /// equality lowering a written `==` over the same pair takes
    /// (`rule:expressions/switch-match-equality`) — so a label at a
    /// representation the subject does not share is that function's row for
    /// the pair rather than anything this chain decides for itself.
    ///
    /// # Panics
    ///
    /// Through [`Self::emit_equality`], guarded by `E0466`: a label no value
    /// of the subject's type could equal is refused where it is written, by
    /// `nvs_types::locals`' own `Switch` arm.
    pub(crate) fn lower_switch(
        &mut self,
        subject: &Expr,
        cases: &'a [SwitchCase],
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let (subj_v, subj_ty) = self.lower_expr(subject, None, env, cur);
        let subject_is_alias = self.aliasing_read(subject);
        // `switch ($x) {}` — PHP still evaluates the subject and then does
        // nothing with it, so this owes only the release a fresh value owes.
        if cases.is_empty() {
            if subj_ty.is_refcounted() && !subject_is_alias {
                self.emit_release(*cur, subj_v);
            }
            return;
        }
        let owns_subject = subj_ty.is_refcounted();
        if owns_subject && subject_is_alias {
            self.emit_retain(*cur, subj_v);
        }
        let seq = self.switch_seq;
        self.switch_seq += 1;
        let subject_name = format!("switch#{seq}");
        if owns_subject {
            env.insert(subject_name.clone(), (subj_v, subj_ty));
        }

        // An enum subject is compared on its backing integer, the same free
        // relabelling [`Lowering::lower_match`] makes above its own label
        // chain — that function's doc comment is the rule's home. The
        // subject's own value and type are untouched by it: they are what the
        // parked reserved binding and the releases below read.
        let (cmp_subj_v, cmp_subj_ty) = self.reinterpret_enum_to_backing(subj_v, subj_ty, cur);

        let body_blocks: Vec<BlockId> = cases.iter().map(|_| self.new_block()).collect();
        let after_block = self.new_block();
        let default_index = cases.iter().position(|c| c.cond.is_none());

        // The equality chain, lowered against the environment the switch was
        // entered with — a label runs before any body does, so no body's
        // bindings are in scope for one.
        let mut entry_env = env.clone();
        let mut entry_edges: Vec<Vec<(BlockId, Env)>> = vec![Vec::new(); cases.len()];
        let mut test_cur = *cur;
        for (i, case) in cases.iter().enumerate() {
            let Some(cond) = &case.cond else {
                continue;
            };
            // The label is in flight for its own comparison, which is the one
            // thing between building it and releasing it that can throw: the
            // rows `Self::emit_equality` answers a cross-representation pair
            // with carry `rule:errors/propagation`'s error edge. The subject
            // needs no such staging — it is parked in the `Env` under the
            // reserved name above, which a landing block sweeps.
            let label_mark = self.temporaries_mark();
            let (cond_v, cond_ty) =
                self.lower_expr(cond, Some(subj_ty), &mut entry_env, &mut test_cur);
            let label_owed = cond_ty.is_refcounted() && !self.aliasing_read(cond);
            if label_owed {
                self.own_temporary(cond_v);
            }
            // The label takes the subject's own relabelling, for the
            // subject's own reason; `cond_v` is still what the release below
            // reads.
            let (cmp_cond_v, cmp_cond_ty) =
                self.reinterpret_enum_to_backing(cond_v, cond_ty, &mut test_cur);
            // A label is the one comparison the language has, written without
            // the operator (`rule:expressions/switch-match-equality`), so it is
            // the lowering a written `==` over the pair takes — a label at a
            // representation the subject does not share included.
            let eq_v = self.emit_equality(
                BinaryOp::Eq,
                (cmp_subj_v, cmp_subj_ty),
                (cmp_cond_v, cmp_cond_ty),
                &mut entry_env,
                &mut test_cur,
            );
            // Past the comparison, so the label is back to being accounted for
            // by hand. A comparison only reads its operands, so a label that no
            // durable slot owns — `case "A":`, the common shape — is released
            // right after the instruction reads it, exactly the rule
            // `Self::lower_expr`'s own `Binary` arm applies.
            self.forget_temporaries_since(label_mark);
            if label_owed {
                self.emit_release(test_cur, cond_v);
            }
            let next = self.new_block();
            let hit_edge = self.ids.next_edge(case.span);
            let miss_edge = self.ids.next_edge(cond.span);
            self.seal(
                test_cur,
                Terminator::Branch {
                    cond: eq_v,
                    then_block: body_blocks[i],
                    then_edge: hit_edge,
                    else_block: next,
                    else_edge: miss_edge,
                },
            );
            entry_edges[i].push((test_cur, entry_env.clone()));
            test_cur = next;
        }
        // Nothing matched: the `default` body if there is one, otherwise past
        // the whole statement. The reserved subject name is dropped from the
        // latter edge for the same reason `Self::lower_break` drops it —
        // nothing after the switch may see it, and the after-block releases it.
        let mut exit_env = entry_env.clone();
        exit_env.remove(&subject_name);
        let mut after_incoming: Vec<(BlockId, Env)> = Vec::new();
        match default_index {
            Some(i) => {
                self.seal(test_cur, Terminator::Jump(body_blocks[i]));
                entry_edges[i].push((test_cur, entry_env.clone()));
            }
            None => {
                self.seal(test_cur, Terminator::Jump(after_block));
                after_incoming.push((test_cur, exit_env.clone()));
            }
        }

        self.loop_stack.push(LoopFrame {
            continue_target: None,
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned: Vec::new(),
            carried: entry_env.keys().cloned().collect(),
            loop_private: if owns_subject {
                vec![subject_name.clone()]
            } else {
                Vec::new()
            },
            try_depth: self.try_stack.len(),
        });

        let last_index = cases.len() - 1;
        let mut fall_in: Option<(BlockId, Env)> = None;
        for (i, case) in cases.iter().enumerate() {
            let block = body_blocks[i];
            let mut incoming = entry_edges[i].clone();
            incoming.extend(fall_in.take());
            let mut body_env = self.merge_envs(block, &incoming, &entry_env);
            let mut body_cur = block;
            self.lower_stmts(&case.body, &mut body_cur, &mut body_env);
            if self.is_terminated(body_cur) {
                continue;
            }
            if i < last_index {
                self.seal(body_cur, Terminator::Jump(body_blocks[i + 1]));
                fall_in = Some((body_cur, body_env));
            } else {
                // Running off the end of the last body leaves the switch, so
                // it owes exactly what a `break` owes: this case's own locals
                // released, and the reserved subject name hidden.
                self.end_iteration(body_cur, &mut body_env);
                body_env.remove(&subject_name);
                self.seal(body_cur, Terminator::Jump(after_block));
                after_incoming.push((body_cur, body_env));
            }
        }

        let frame = self
            .loop_stack
            .pop()
            .expect("just pushed this switch's own frame above");
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &exit_env);
        if owns_subject {
            self.emit_release(after_block, subj_v);
        }
        *cur = after_block;
    }
    /// `foreach ($subject as $k => $v) body` over an `array<T>` — `rule:types/arrays`'s insertion order, walked by the cursor
    /// [`InstKind::ArrayNextSlot`] steps.
    ///
    /// Structurally [`Self::lower_while`] with a synthesized condition, and it
    /// reuses that method's whole phi/`break`/`continue` machinery unchanged.
    /// What is its own:
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
    ///   in an Novis identifier, so neither can collide with a local. Being
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
    /// # `inout $v` inverts the loop's second reference
    ///
    /// A by-reference value binding writes each element back into the array
    /// being walked, so the loop must **not** hold a second reference: the
    /// one thing the extra reference buys — `rule:types/arrays` separating the array
    /// on the first write, leaving the cursor on the snapshot — is exactly
    /// what an `inout $v` loop must not do. So the by-reference shape drops the
    /// retain, walks the subject variable's *own* `Env` binding rather than a
    /// reserved `foreach#N` one (`nvs_types`' `check_foreach_inout` refuses
    /// every subject that is not a plain variable, so there is always one),
    /// and releases nothing after the loop, the local's own exit sweep being
    /// the single owner it always was.
    ///
    /// **Each write is a write-through, not a copy-back at the end of the
    /// iteration.** Every rebinding of `$v` — `$v = e`, `$v .= e`, `$v++`, a
    /// `inout $v` argument's own copy-back, an element write `$v[0] = e` — also
    /// stores the new value into the entry it came from, through
    /// [`Self::write_through_element`] and the [`InoutElement`] this pushes
    /// around the body. Copy-back at the iteration's end would be one
    /// [`InstKind::ArraySet`] instead of one per write, but it would have to
    /// be emitted at every edge that ends an iteration and get the `return`
    /// and the throwing ones right too — where PHP, whose `&$v` is a true
    /// alias, has already written. Write-through is that alias, one store
    /// later.
    ///
    /// The array's binding is re-pointed by every such write, because
    /// [`InstKind::ArraySet`] consumes one reference and produces the one the
    /// holder now owns (see [`Self::write_back_array`]). That is why the
    /// subject's name gets a header phi like any reassigned local, and why
    /// [`InoutElement`] carries `Env` names rather than values: a write
    /// inside the body is what the *next* iteration walks.
    ///
    /// # Panics
    ///
    /// Two [`guarded_by!`] sites, each naming the diagnostic that refuses its
    /// shape where it is written: a subject that is not an `array<T>` once the
    /// cursor lowering above has taken `Iterable`/`Iterator` (`E0443`, from
    /// `nvs_types::expr::iteration::report_not_iterable`, which is what an
    /// un-narrowed `?array<T>` meets), and a key binding declared as anything
    /// but `string` (`E0723`, `rule:types/arrays` making every stored key a
    /// `string`). Reaching either means this body was checked against a
    /// different table. A binding with no declared type at all panics in
    /// [`binding_ty`], which `nvs_types` has already diagnosed.
    #[expect(
        clippy::too_many_arguments,
        reason = "the arguments are one `StmtKind::Foreach`'s own fields plus \
                  the `(cur, env)` pair every lowering method threads"
    )]
    pub(crate) fn lower_foreach(
        &mut self,
        subject: &Expr,
        key: Option<&ForeachBinding>,
        value: &ForeachBinding,
        value_inout: bool,
        body: &'a Stmt,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        // `rule:iteration/foreach-subjects`'s three shapes are three loops, and which one this is
        // was decided by the checker — `nvs-ir` cannot re-derive it, because
        // reaching `Iterable` through a base class needs the `ClassGraph`
        // this crate deliberately does not depend on. See
        // `nvs_types::ExprTypeTable::foreach_drive`.
        let drive = self.exprs.foreach_drive(subject.span).unwrap_or_else(|| {
            panic!(
                "nvs-ir: the `foreach` subject at {:?} has no recorded `ForeachDrive` — either \
                 it erased to `mixed` (unsupported, see the crate docs' known gaps) or this \
                 program did not pass nvs_types::check_program with the same table",
                subject.span
            )
        });
        if drive != ForeachDrive::Array {
            assert!(
                !value_inout,
                "nvs-ir: a `foreach (… as inout $v)` over an `Iterable`/`Iterator` subject reached \
                 lowering — a cursor has no element storage to write back to, and nvs_types \
                 reports E0490 for one"
            );
            assert!(
                key.is_none(),
                "nvs-ir: a `foreach` key binding over an `Iterable`/`Iterator` subject reached \
                 lowering — `rule:iteration/two-interfaces` gives a cursor no key at all, and nvs_types reports \
                 E0444 for one"
            );
            self.lower_foreach_cursor(
                subject,
                value,
                drive == ForeachDrive::Iterable,
                body,
                cur,
                env,
            );
            return;
        }
        let value_ty = binding_ty(value, "value", self.exprs, self.checked_types);
        let key_binding = key.map(|k| {
            let ty = binding_ty(k, "key", self.exprs, self.checked_types);
            if ty != Ty::Str {
                guarded_by!(
                    code::E_FOREACH_KEY_TY,
                    "nvs-ir reached a `foreach` key binding at {ty:?}. `rule:types/arrays` gives an \
                     `array<T>` one stored key type, so `nvs_types::expr::iteration` refuses every \
                     other declared key type where the binding is written rather than converting \
                     it, and this body was not checked with the same table"
                );
            }
            strip_sigil(span_text(self.src, k.name)).to_owned()
        });
        let value_name = strip_sigil(span_text(self.src, value.name)).to_owned();

        let (array_v, array_ty) = self.lower_expr(subject, None, env, cur);
        if array_ty != Ty::Array {
            guarded_by!(
                code::E_FOREACH_SUBJECT_NOT_ITERABLE,
                "nvs-ir reached the array `foreach` lowering over {array_ty:?}. \
                 `rule:iteration/foreach-subjects` drives three subjects and refuses a fourth at \
                 the subject — `nvs_types::expr::iteration::report_not_iterable` is that refusal's \
                 one home — so an un-narrowed `?array<T>` is refused where it is written, and the \
                 `Iterable`/`Iterator` pair took the cursor lowering above"
            );
        }
        if !value_inout && self.aliasing_read(subject) {
            self.emit_retain(*cur, array_v);
        }

        let seq = self.foreach_seq;
        self.foreach_seq += 1;
        // A by-reference loop walks the subject variable's own binding — see
        // this method's doc comment on why it must be that one slot and not a
        // second reference to the same array.
        let array_name = if value_inout {
            let ExprKind::Variable(name_span) = &subject.kind else {
                panic!(
                    "nvs-ir: a `foreach (… as inout $v)` subject at {:?} is not a plain variable — \
                     nvs_types reports E0490 for one before this runs",
                    subject.span
                );
            };
            strip_sigil(span_text(self.src, *name_span)).to_owned()
        } else {
            let name = format!("foreach#{seq}");
            env.insert(name.clone(), (array_v, Ty::Array));
            name
        };
        let cursor_name = format!("foreach#{seq}$cursor");
        let slot_name = format!("foreach#{seq}$slot");
        let (zero_v, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
        env.insert(cursor_name.clone(), (zero_v, Ty::Int));

        // From here on this is `Self::lower_while`'s shape, with the cursor
        // prepended to the loop-carried names so it gets the same header phi
        // every reassigned local does.
        let mut seen = FxHashSet::default();
        let mut reassigned = vec![cursor_name.clone()];
        seen.insert(cursor_name.clone());
        // Every write through `inout $v` re-points the subject's binding, and the
        // syntactic scan below cannot see that — the write is spelled `$v`,
        // not `$a`. Seeding it is what gives the next iteration the array the
        // last one wrote into.
        if value_inout && seen.insert(array_name.clone()) {
            reassigned.push(array_name.clone());
        }
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);
        self.seed_generator_loop_carried(env, &mut seen, &mut reassigned);

        let pre_block = *cur;
        let header_block = self.new_block();
        self.seal(pre_block, Terminator::Jump(header_block));

        let mut header_env = env.clone();
        let mut phi_slots: Vec<(String, usize)> = Vec::new();
        for name in &reassigned {
            let Some(&(pre_v, ty)) = env.get(name) else {
                continue;
            };
            // An `inout $x` parameter's binding is an address that never changes:
            // writing to it stores *through* it rather than rebinding it
            // (`Ty::Ref`), so a header phi for one would carry the same value
            // on both edges and describe nothing.
            if ty == Ty::Ref {
                continue;
            }
            let phi_v = self.ids.next_value();
            let inst_index = self.block_insts[header_block.index() as usize].len();
            self.block_insts[header_block.index() as usize].push(Inst {
                result: Some(phi_v),
                ty: Some(ty),
                kind: InstKind::Phi {
                    incoming: vec![(pre_block, pre_v)],
                },
                on_error: None,
                raise_site: None,
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

        // Both reserved names are read back out of `header_env` rather than
        // reused from before the loop: inside a generator every live local
        // gets a header phi (`Self::seed_generator_loop_carried`), because the
        // resume edge re-enters this header with the values reloaded from the
        // generator frame. The pre-loop definition does not dominate that
        // edge, so using it directly builds code Cranelift's verifier rejects.
        // Outside a generator the phi carries the same value and this is a
        // no-op.
        let cursor_v = header_env[&cursor_name].0;
        let array_v = self.loop_array(header_block, &header_env, &array_name);
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
            continue_target: Some(header_block),
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned,
            carried: header_env.keys().cloned().collect(),
            // A by-reference loop's array *is* the subject's own binding, so
            // it is not private to the loop and outlives it — only the cursor
            // and the slot are reserved names to hide.
            loop_private: if value_inout {
                vec![cursor_name.clone(), slot_name.clone()]
            } else {
                vec![array_name.clone(), cursor_name.clone()]
            },
            try_depth: self.try_stack.len(),
        });

        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        let (one_v, _) = self.emit(body_cur, Ty::Int, InstKind::ConstInt(1));
        // A slot index cannot reach `i64::MAX`, so this add never throws —
        // but `rule:types/arithmetic`'s checked `int` row is what `nvs-codegen` emits for
        // it either way, and every instruction that returns a status carries a
        // landing block (`Inst::on_error`). The block is on an edge no run
        // takes.
        let (next_v, _) = self.emit_fallible(
            body_cur,
            Ty::Int,
            InstKind::BinOp {
                op: BinOp::Add,
                lhs: slot_v,
                rhs: one_v,
            },
            &body_env,
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
        // A by-reference binding writes its value back into the element, so
        // it keeps the element's own representation.
        let v_v = if value_inout {
            v_v
        } else {
            self.widen_marked_binding(value.span, v_v, value_ty, &mut body_env, body_cur)
                .0
        };
        if value_ty.is_refcounted() {
            self.emit_retain(body_cur, v_v);
        }
        body_env.insert(value_name.clone(), (v_v, value_ty));

        if value_inout {
            // The slot travels in the `Env` rather than as a `ValueId` for
            // `Self::seed_generator_loop_carried`'s reason: a write to `$v`
            // may sit after a `yield`, where the header's own definition has
            // been spilled and reloaded. It is a `Ty::Int`, so it owns
            // nothing and every sweep ignores it.
            body_env.insert(slot_name.clone(), (slot_v, Ty::Int));
            self.inout_elements.push(InoutElement {
                binding: value_name,
                array: array_name.clone(),
                slot: slot_name.clone(),
            });
        }
        self.lower_stmt(body, &mut body_cur, &mut body_env);
        if value_inout {
            self.inout_elements
                .pop()
                .expect("just pushed this loop's own by-reference binding above");
        }
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
                        "nvs-ir: `{name}` was reassigned in a foreach body per the syntactic \
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
        exit_env.remove(&cursor_name);
        exit_env.remove(&slot_name);
        if !value_inout {
            exit_env.remove(&array_name);
        }
        let mut after_incoming: Vec<(BlockId, Env)> = vec![(header_block, exit_env.clone())];
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &exit_env);
        // The by-reference loop never took a reference of its own, so there
        // is none to give back: what the subject's binding holds after the
        // loop is the array every write-through re-pointed it at, and its own
        // exit sweep releases that once.
        if !value_inout {
            self.emit_release(after_block, array_v);
        }
        *cur = after_block;
    }
    /// The array a `foreach` is walking, read out of the [`Env`] name it is
    /// bound under.
    ///
    /// One [`InstKind::RefLoad`] when that name is an `inout $x` parameter's cell
    /// (see [`Ty::Ref`]), the binding's own value otherwise. A by-reference
    /// loop walks the subject's binding directly ([`Self::lower_foreach`]),
    /// and a `&array<T>` parameter is as much a plain variable at the source
    /// level as any local — so both shapes reach here, and the cell is the
    /// one the write-through re-points.
    fn loop_array(&mut self, cur: BlockId, env: &Env, name: &str) -> ValueId {
        let &(v, ty) = env.get(name).unwrap_or_else(|| {
            panic!("nvs-ir: `foreach` lost the `Env` binding `{name}` it walks")
        });
        if ty == Ty::Ref {
            let pointee = self.pointee_of(name);
            let (loaded, _) = self.emit(cur, pointee, InstKind::RefLoad { slot: v });
            loaded
        } else {
            v
        }
    }
    /// Re-points the same binding at the array an [`InstKind::ArraySet`] just
    /// yielded — [`Self::loop_array`]'s other half, and
    /// [`Self::write_back_array`]'s policy against a name rather than an
    /// expression: no retain and no release, the reference consumed and the
    /// one produced being the holder's same one.
    fn store_loop_array(&mut self, cur: BlockId, env: &mut Env, name: &str, written: ValueId) {
        let &(v, ty) = env.get(name).unwrap_or_else(|| {
            panic!("nvs-ir: `foreach` lost the `Env` binding `{name}` it walks")
        });
        if ty == Ty::Ref {
            self.emit_ref_store(cur, v, written);
        } else {
            // The array a loop walks may itself be an enclosing loop's `inout $v`
            // binding (`foreach ($grid as inout array<int> $row) { foreach
            // ($row as inout int $cell) …`), and the separation the inner write just
            // caused is exactly what the outer entry has to be told about.
            // The recursion is one level per nesting level and terminates at
            // the outermost subject, which is a name no binding owns.
            self.write_through_element(cur, env, name, written, Ty::Array);
            env.insert(name.to_owned(), (written, Ty::Array));
        }
    }
    /// Writes `v` into the entry a `foreach (… as inout $v)` binding came from,
    /// when `name` is such a binding — the write-through
    /// [`Self::lower_foreach`]'s doc comment describes, and nothing at all
    /// for every other name, which is every name in a frame with no
    /// by-reference loop open.
    ///
    /// Called from the three places that re-point a local's slot:
    /// [`Self::bind_local_value`] (`$v = e` and every compound form),
    /// [`Self::write_back_holder`] (an `inout $v` argument's copy-back) and
    /// [`Self::write_back_array`] (`$v[0] = e`). The binding keeps its own
    /// reference and the array takes one of its own, so a refcounted value is
    /// retained here and released with the binding at the end of the
    /// iteration ([`LoopFrame::iteration_owned`]) — [`InstKind::ArrayKeyAt`]'s
    /// fresh key is consumed by the [`InstKind::ArraySet`] and owes nothing
    /// further.
    pub(crate) fn write_through_element(
        &mut self,
        cur: BlockId,
        env: &mut Env,
        name: &str,
        v: ValueId,
        ty: Ty,
    ) {
        if self.inout_elements.is_empty() {
            return;
        }
        let Some(element) = self.inout_elements.iter().rev().find(|e| e.binding == name) else {
            return;
        };
        let (array_name, slot_name) = (element.array.clone(), element.slot.clone());
        let &(slot_v, Ty::Int) = env.get(&slot_name).unwrap_or_else(|| {
            panic!("nvs-ir: a `foreach (… as inout $v)` body lost the cursor slot `{slot_name}`")
        }) else {
            panic!("nvs-ir: a `foreach` cursor slot is bound at `Ty::Int` and nothing rebinds it")
        };
        let array_v = self.loop_array(cur, env, &array_name);
        let (key_v, _) = self.emit(
            cur,
            Ty::Str,
            InstKind::ArrayKeyAt {
                array: array_v,
                slot: slot_v,
            },
        );
        if ty.is_refcounted() {
            self.emit_retain(cur, v);
        }
        let written = self.emit_array_set(cur, array_v, key_v, v);
        self.store_loop_array(cur, env, &array_name, written);
    }
    /// `foreach ($subject as $v) body` over `rule:iteration/foreach-subjects`'s other two shapes —
    /// an `Iterable<T>`, whose `iterate()` is called once for a fresh cursor,
    /// and an `Iterator<T>`, which *is* the cursor.
    ///
    /// The two differ by exactly that one call, so they share everything
    /// below it: `via_iterable` decides whether the value the loop drives is
    /// the subject itself or `iterate()`'s result.
    ///
    /// Structurally [`Self::lower_while`] again, with `advance()` as the
    /// condition — so the phi/`break`/`continue` machinery is unchanged from
    /// [`Self::lower_foreach`], and only what the header and the body's first
    /// instructions do is different:
    ///
    /// * **Both members are [`InstKind::CallVirtual`]**, never a static
    ///   [`InstKind::Call`]. `rule:iteration/two-interfaces`'s interfaces declare `advance`,
    ///   `current` and `iterate` without bodies, and `nvs_types::iter_lib`'s
    ///   own docs own why that is the mechanism rather than an accident: a
    ///   call resolving to a bodiless declaration names no compiled function,
    ///   so it dispatches on the receiver's runtime class — which is exactly
    ///   what driving a cursor whose concrete class the loop never knows
    ///   needs. This method therefore emits the instructions itself instead
    ///   of routing a synthesized AST node through [`Self::lower_expr`];
    ///   there is no `$cursor->advance()` in the source to look up.
    /// * **The loop owns one reference to the cursor**, released in the
    ///   after-block, and *retains it again before each member call* — a
    ///   receiver is parameter 0 and Novis transfers an argument's reference to
    ///   the callee (see [`ArgOwnership::Transferred`]), so a call that did
    ///   not retain first would consume the loop's own. It lives in the
    ///   [`Env`] under a reserved `foreach#N$iter` name for
    ///   [`Self::lower_foreach`]'s reason: that is what gets it swept by
    ///   [`Self::release_all_locals`] on a throw out of the body, and hidden
    ///   from everything after the loop by [`LoopFrame::loop_private`].
    /// * **`iterate()` consumes the subject reference this frame is holding**
    ///   rather than retaining a second one — the loop has no further use for
    ///   the subject once it has a cursor. For an aliasing subject that is
    ///   the retain taken just above; for a fresh one (`new Bag(4)`) it is
    ///   the single reference `Self::lower_expr` produced, which is why
    ///   that value is never entered into the `Env` and the after-block
    ///   releases the *cursor* instead.
    /// * **There is no cursor phi and no key.** The driven value never
    ///   changes — the position it walks lives inside the cursor object, not
    ///   in this frame — and `rule:iteration/two-interfaces` gives `Iterator<T>` no key member
    ///   for a binding to read.
    ///
    /// # Panics
    ///
    /// Panics if the subject did not lower to a [`Ty::Object`], which would
    /// mean `nvs_types` classified something as a cursor that has no runtime
    /// class to dispatch on.
    pub(crate) fn lower_foreach_cursor(
        &mut self,
        subject: &Expr,
        value: &ForeachBinding,
        via_iterable: bool,
        body: &'a Stmt,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let value_ty = binding_ty(value, "value", self.exprs, self.checked_types);
        let value_name = strip_sigil(span_text(self.src, value.name)).to_owned();

        let (subject_v, subject_ty) = self.lower_expr(subject, None, env, cur);
        assert!(
            subject_ty == Ty::Object,
            "nvs-ir: a `foreach` subject nvs_types classified as a cursor lowered to \
             {subject_ty:?} rather than an object — `rule:iteration/foreach-subjects`'s `Iterable`/`Iterator` shapes \
             are both class types"
        );
        if self.aliasing_read(subject) {
            self.emit_retain(*cur, subject_v);
        }

        let cursor_v = if via_iterable {
            self.emit_iface_call(*cur, subject_v, false, "iterate", Ty::Object, env)
        } else {
            subject_v
        };

        let seq = self.foreach_seq;
        self.foreach_seq += 1;
        let cursor_name = format!("foreach#{seq}$iter");
        env.insert(cursor_name.clone(), (cursor_v, Ty::Object));

        let mut seen = FxHashSet::default();
        let mut reassigned = Vec::new();
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);
        self.seed_generator_loop_carried(env, &mut seen, &mut reassigned);

        let pre_block = *cur;
        let header_block = self.new_block();
        self.seal(pre_block, Terminator::Jump(header_block));

        let mut header_env = env.clone();
        let mut phi_slots: Vec<(String, usize)> = Vec::new();
        for name in &reassigned {
            let Some(&(pre_v, ty)) = env.get(name) else {
                continue;
            };
            // See `Self::lower_foreach`: an `inout $x` parameter's binding is an
            // address that never changes, so a header phi for one would carry
            // the same value on both edges and describe nothing.
            if ty == Ty::Ref {
                continue;
            }
            let phi_v = self.ids.next_value();
            let inst_index = self.block_insts[header_block.index() as usize].len();
            self.block_insts[header_block.index() as usize].push(Inst {
                result: Some(phi_v),
                ty: Some(ty),
                kind: InstKind::Phi {
                    incoming: vec![(pre_block, pre_v)],
                },
                on_error: None,
                raise_site: None,
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

        // Read back out of `header_env` for the reason
        // [`Self::lower_foreach`] states at the same point: a generator's
        // resume edge re-enters this header with the cursor reloaded from its
        // frame, so only the header phi dominates every use below.
        let cursor_v = header_env[&cursor_name].0;
        let more_v = self.emit_iface_call(
            header_block,
            cursor_v,
            true,
            "advance",
            Ty::Bool,
            &header_env,
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

        self.loop_stack.push(LoopFrame {
            continue_target: Some(header_block),
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned: vec![value_name.clone()],
            carried: header_env.keys().cloned().collect(),
            loop_private: vec![cursor_name.clone()],
            try_depth: self.try_stack.len(),
        });

        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        let v_v = self.emit_iface_call(body_cur, cursor_v, true, "current", value_ty, &body_env);
        let (v_v, _) =
            self.widen_marked_binding(value.span, v_v, value_ty, &mut body_env, body_cur);
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
                        "nvs-ir: `{name}` was reassigned in a foreach body per the syntactic \
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
        exit_env.remove(&cursor_name);
        let mut after_incoming: Vec<(BlockId, Env)> = vec![(header_block, exit_env.clone())];
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &exit_env);
        self.emit_release(after_block, cursor_v);
        *cur = after_block;
    }
    /// One `rule:iteration/two-interfaces` member call on `receiver`, emitted directly rather
    /// than lowered from source — see [`Self::lower_foreach_cursor`] for why
    /// there is no AST node to route through.
    ///
    /// `keep_receiver` says whether the caller still owns its reference
    /// afterwards. A receiver is parameter 0 and Novis transfers an argument's
    /// reference to the callee ([`ArgOwnership::Transferred`]), on the
    /// callee's throw path as much as its return path, so keeping one means
    /// retaining a second — which is what the loop's per-iteration
    /// `advance()`/`current()` pair does, and what the once-only `iterate()`
    /// deliberately does not.
    pub(crate) fn emit_iface_call(
        &mut self,
        b: BlockId,
        receiver: ValueId,
        keep_receiver: bool,
        method: &str,
        ret: Ty,
        env: &Env,
    ) -> ValueId {
        if keep_receiver {
            self.emit_retain(b, receiver);
        }
        let (lsb, _) = self.emit(b, Ty::ClassDesc, InstKind::ClassDescOf { object: receiver });
        let (v, _) = self.emit_fallible(
            b,
            ret,
            InstKind::CallVirtual {
                lsb,
                method: method.to_owned(),
                fallback: None,
                receiver: Some(receiver),
                args: Vec::new(),
            },
            env,
        );
        v
    }
    /// `break;`/`break N;` — jumps straight to the exit block of the `N`-th
    /// enclosing `break` target, recording the current block and environment
    /// as one more incoming edge [`Self::lower_while`] folds into its own
    /// after-block merge once the body it's nested in finishes lowering.
    ///
    /// `N` counts **every** enclosing target, a `switch` included, which is
    /// PHP's own rule and the one thing `break` and `continue` disagree about
    /// here — see [`Self::lower_continue`], which counts loops only because a
    /// `switch` owns no back edge to jump to.
    ///
    /// Leaving more than one at once changes nothing about the shape of the
    /// jump and everything about what it owes: the *outermost* frame being
    /// left is the one whose [`LoopFrame::carried`] set decides which names
    /// are body-local, so one [`Self::end_iteration_at`] against that frame
    /// releases every intervening loop's iteration bindings and bookkeeping
    /// references as well as its own. See [`Self::loop_exit_level`] for what
    /// `level` is allowed to be.
    ///
    /// # Panics
    ///
    /// Panics if the level names more targets than enclose it —
    /// `nvs_types::locals` rejects that with `E0475` before lowering runs, so
    /// reaching it here is an internal error rather than a program's.
    pub(crate) fn lower_break(&mut self, level: &Option<Expr>, cur: &mut BlockId, env: &mut Env) {
        let level = self.loop_exit_level(level, "break");
        let at = self.loop_stack.len().checked_sub(level).unwrap_or_else(|| {
            panic!(
                "nvs-ir: `break {level}` reached lowering inside {} enclosing break \
                     target(s) — nvs_types rejects that with E0475 before this runs",
                self.loop_stack.len()
            )
        });
        let frame = &self.loop_stack[at];
        let after_block = frame.after_block;
        let try_depth = frame.try_depth;
        // A `break` leaves every protected region between it and its loop, so
        // each one's `finally` runs first — before the iteration's releases,
        // since the body's locals are still readable from that body.
        let mut exit_env = env.clone();
        self.run_finallys_above(try_depth, cur, &mut exit_env);
        if self.is_terminated(*cur) {
            return;
        }
        // A `break` ends the iteration it is in, so it owes exactly what a
        // back edge owes — see `LoopFrame::iteration_owned`. It additionally
        // hides the loop's own bookkeeping names from everything after the
        // loop, which the condition's own false edge never carries either.
        //
        // Only the target frame's own bookkeeping is *hidden* rather than
        // released: its after-block is where those references are dropped. An
        // intervening loop's are not in the target's `carried` set, so the
        // sweep above has already released them — which is exactly right,
        // since this jump skips the after-block that would have.
        self.end_iteration_at(*cur, &mut exit_env, at);
        for name in &self.loop_stack[at].loop_private {
            exit_env.remove(name);
        }
        self.loop_stack[at].break_edges.push((*cur, exit_env));
        self.seal(*cur, Terminator::Jump(after_block));
    }
    /// Releases everything the innermost loop's *current iteration* owns and
    /// drops it from `env` — the one thing every point an iteration ends has
    /// in common (the body's fall-through back edge, a `continue`, a `break`).
    ///
    /// The sets below, disjoint by construction:
    ///
    /// * [`LoopFrame::iteration_owned`] — a `foreach` header's key and value
    ///   bindings, named up front because the header rebinds them itself.
    /// * Every remaining binding whose name the loop header does not carry
    ///   ([`LoopFrame::carried`]) — a local the *body* declared. Nothing
    ///   after this point can reach it: the next iteration restarts from the
    ///   header environment and the loop's exit environment is built from the
    ///   header's, so without this release its reference would simply be
    ///   dropped on the floor, once per iteration.
    ///
    /// Sorted rather than left in `FxHashMap` order: which releases a block
    /// ends up holding must depend only on the source, never on hash-table
    /// internals — the same rule the crate's `ids` module states for value
    /// ids.
    pub(crate) fn end_iteration(&mut self, cur: BlockId, env: &mut Env) {
        let Some(at) = self.loop_stack.len().checked_sub(1) else {
            return;
        };
        self.end_iteration_at(cur, env, at);
    }
    /// [`Self::end_iteration`] against a named [`Self::loop_stack`] frame
    /// rather than the innermost one.
    ///
    /// The two differ for exactly one caller: a `continue` written inside a
    /// `switch` inside a loop, which ends an iteration of the **loop** while
    /// the innermost frame is the `switch`'s ([`Self::lower_switch`]). Reading
    /// the loop's own [`LoopFrame::carried`] there is what makes both the case
    /// body's locals and the loop body's locals get released on that edge —
    /// the switch's carried set names the loop-body locals, so using it would
    /// silently keep them alive across the back edge.
    pub(crate) fn end_iteration_at(&mut self, cur: BlockId, env: &mut Env, at: usize) {
        let Some(frame) = self.loop_stack.get(at) else {
            return;
        };
        let owned = frame.iteration_owned.clone();
        let carried = frame.carried.clone();
        let mut body_local: Vec<String> = env
            .iter()
            .filter(|(name, (_, ty))| ty.is_refcounted() && !carried.contains(*name))
            .map(|(name, _)| name.clone())
            .collect();
        body_local.sort_unstable();
        for name in owned.into_iter().chain(body_local) {
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
    pub(crate) fn lower_continue(
        &mut self,
        level: &Option<Expr>,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let level = self.loop_exit_level(level, "continue");
        // Counted PHP's way — every enclosing frame, a `switch` included — and
        // then walked *outward* to the nearest frame that is a loop, not
        // simply the `level`-th loop. The two steps are what make `continue N`
        // mean in Novis exactly what it means in PHP:
        //
        // * `continue 2` inside a `switch` inside one loop is PHP's own
        //   idiomatic spelling for "continue the loop", and counting loops
        //   alone would make it name a second loop that is not there.
        // * `continue 2` inside a `switch` inside two nested loops is PHP's
        //   "continue the *inner* loop"; counting loops alone would silently
        //   continue the outer one, which is the worst of the three
        //   possibilities.
        //
        // The walk is where the one divergence lives, and it is not a new
        // one: a level landing on a `switch` frame is PHP's "break the
        // switch", and Novis already reads a bare `continue` there as
        // continuing the enclosing loop instead — see
        // `LoopFrame::continue_target` and `Self::lower_switch`. Level 1
        // reduces to exactly that rule.
        let counted = self.loop_stack.len().checked_sub(level).unwrap_or_else(|| {
            panic!(
                "nvs-ir: `continue {level}` reached lowering inside {} enclosing frame(s) — \
                 nvs_types rejects that with E0475 before this runs",
                self.loop_stack.len()
            )
        });
        let at = self.loop_stack[..=counted]
            .iter()
            .rposition(|frame| frame.continue_target.is_some())
            .unwrap_or_else(|| {
                panic!(
                    "nvs-ir: `continue {level}` reached lowering with no enclosing loop at or \
                     outside its level — nvs_types rejects that with E0475 before this runs"
                )
            });
        let frame = &self.loop_stack[at];
        let header_block = frame
            .continue_target
            .expect("rposition only matches a frame with a continue target");
        let try_depth = frame.try_depth;
        // Same obligation a `break` has: every protected region this jump
        // leaves runs its `finally` first — see `Self::run_finallys_above`.
        let mut back_env = env.clone();
        self.run_finallys_above(try_depth, cur, &mut back_env);
        if self.is_terminated(*cur) {
            return;
        }
        // The next iteration rebinds a `foreach` header's key/value from
        // scratch, so this back edge ends the current one — see
        // `LoopFrame::iteration_owned`.
        self.end_iteration_at(*cur, &mut back_env, at);
        self.emit_safepoint(*cur);
        self.loop_stack[at].continue_edges.push((*cur, back_env));
        self.seal(*cur, Terminator::Jump(header_block));
    }
    /// Reads a `break`/`continue` statement's optional level operand — PHP's
    /// `break N;`/`continue N;`, which names the `N`-th enclosing target
    /// rather than the innermost one. `None` is level 1.
    ///
    /// The operand is an integer literal or nothing: a level computed at run
    /// time would have no target to resolve against at compile time, which is
    /// why PHP stopped accepting one in 5.4 and why `nvs_types::locals`
    /// refuses it with `E0475`. That checker also refuses a `0` and a level
    /// naming more targets than enclose it, so everything reaching here is a
    /// level the loop stack can satisfy.
    ///
    /// # Panics
    ///
    /// Panics on anything `nvs_types::locals` should already have refused —
    /// an internal error, not a program's.
    pub(crate) fn loop_exit_level(&self, level: &Option<Expr>, keyword: &str) -> usize {
        let Some(level_expr) = level else {
            return 1;
        };
        let ExprKind::Int(span) = &level_expr.kind else {
            panic!(
                "nvs-ir: a `{keyword}` level reached lowering as a non-literal — nvs_types \
                 rejects that with E0475 before this runs"
            );
        };
        let (radix, digits) = int_literal_digits(self.src, *span);
        let n = u64::from_str_radix(&digits, radix).unwrap_or_else(|_| {
            panic!("nvs-ir: `{keyword}` level literal `{digits}` doesn't fit a u64")
        });
        let n = usize::try_from(n).unwrap_or(usize::MAX);
        assert!(
            n >= 1,
            "nvs-ir: `{keyword} 0` reached lowering — nvs_types rejects that with E0475 before \
             this runs"
        );
        n
    }
    /// Merges the environments reaching a join block into one, inserting a
    /// [`InstKind::Phi`] for any local whose value differs across incoming
    /// edges. See the module docs for why a name missing from some incoming
    /// environment can be silently dropped rather than treated as an error.
    pub(crate) fn merge_envs(
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
                // The *union* of every edge's names, not the first edge's:
                // the two directions are not symmetric to look at but are to
                // account for. A name bound on the first edge and missing
                // elsewhere is dropped and released below; a name bound only
                // on a *later* edge owes exactly the same release, and
                // reading the first edge's keys alone would never see it — a
                // local declared inside a `try` body leaks precisely that
                // way, since the landing edge taken before the declaration
                // does not bind it.
                //
                // Sorted rather than left in `FxHashMap`'s bucket order: a
                // phi's id must depend only on source order (see the crate's
                // `ids` module docs on why), never on hash-table internals.
                let mut names: Vec<String> = incoming
                    .iter()
                    .flat_map(|(_, e)| e.keys().cloned())
                    .collect();
                names.sort_unstable();
                names.dedup();
                for name in names {
                    if !incoming.iter().all(|(_, e)| e.contains_key(&name)) {
                        // Not bound on every incoming edge — per the module
                        // docs, checked input never uses such a name past
                        // this point, so it needs no entry in the merged env.
                        // It still owes a release on each edge that *does*
                        // bind it: nothing downstream can reach the value, so
                        // this is the last point that could free it. See
                        // `Self::release_merged_away`.
                        self.release_merged_away(incoming, &name);
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
    /// Releases `name` on every incoming edge that binds it to a refcounted
    /// value, for a name [`Self::merge_envs`] is about to drop.
    ///
    /// A binding that survives on only some incoming edges is one a block
    /// *declared* — an `if` branch's own local, a loop body's own local. The
    /// merged environment cannot carry it (there is no value for the edges
    /// that never bound it, and Novis has no `null` in the IR to phi in), and
    /// the checker's definite-assignment rule already refuses any read of it
    /// past this point. So the merge is the last place the reference is
    /// reachable at all, and the edge that owns it is the one that must free
    /// it — without this, every conditionally-declared `string`/`array`/
    /// object local would leak exactly one reference per execution.
    ///
    /// Emitted into the incoming block itself, which by this point is usually
    /// already sealed. That is fine and deliberate: a block's instructions and
    /// its terminator are stored separately, so appending here still lands the
    /// release before the jump.
    pub(crate) fn release_merged_away(&mut self, incoming: &[(BlockId, Env)], name: &str) {
        let owed: Vec<(BlockId, ValueId)> = incoming
            .iter()
            .filter_map(|(block, env)| match env.get(name) {
                Some(&(v, ty)) if ty.is_refcounted() => Some((*block, v)),
                _ => None,
            })
            .collect();
        for (block, v) in owed {
            self.emit_release(block, v);
        }
    }
    /// Adds every remaining [`Env`] binding to a loop's carried set, but only
    /// inside a generator's `advance()`.
    ///
    /// Outside one, a value bound *before* a loop dominates the whole loop,
    /// so only a reassigned local needs a header phi — which is exactly what
    /// [`Self::collect_reassigned_locals`] finds. A generator breaks that
    /// assumption and is the only thing that does: its entry switch enters a
    /// resume block that may sit *inside* the loop body, so the header gains
    /// a predecessor whose path never passed through the block the pre-loop
    /// value was defined in. The resume block rebinds every name from a field
    /// (see [`lower_generator`]), so the values are all there — giving every
    /// binding a header phi is what lets them reach the header in SSA form.
    ///
    /// [`GEN_SELF`] is excluded: it is parameter 0, defined in the entry
    /// block, which dominates every block in the function including every
    /// resume block, so a phi for it would carry one value on both edges and
    /// describe nothing.
    pub(crate) fn seed_generator_loop_carried(
        &self,
        env: &mut Env,
        seen: &mut FxHashSet<String>,
        out: &mut Vec<String>,
    ) {
        if self.generator.is_none() {
            return;
        }
        // Sorted for `Self::release_all_locals`' reason: which phi gets which
        // id must depend only on source order, never on hash-bucket layout.
        let mut names: Vec<&String> = env.keys().filter(|n| n.as_str() != GEN_SELF).collect();
        names.sort();
        for name in names {
            if seen.insert(name.clone()) {
                out.push(name.clone());
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
    pub(crate) fn collect_reassigned_locals(
        &self,
        stmt: &Stmt,
        seen: &mut FxHashSet<String>,
        out: &mut Vec<String>,
    ) {
        match &stmt.kind {
            // Every assignment operator counts, not just `=`: a compound
            // `$n += 1` re-points its local exactly the way the `$n = $n + 1`
            // it lowers to does (`Self::lower_compound_assignment`), and a
            // loop header that misses its phi reads the pre-loop value on
            // every iteration — an infinite `while ($n < 4) { $n += 1; }`
            // rather than a diagnostic.
            StmtKind::Expr(e) => self.collect_reassigned_in_expr(e, seen, out),
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
                    // `unset($a[$i++]);` — the subscript is an ordinary
                    // expression and may itself re-point a local.
                    self.collect_reassigned_in_expr(target, seen, out);
                }
            }
            StmtKind::Block(b) => {
                for s in &b.stmts {
                    self.collect_reassigned_locals(s, seen, out);
                }
            }
            // Every clause below is scanned, not only the nested bodies: with
            // an increment lowering in value position, `if ($n++ > 2) { … }`
            // and `echo $n++;` re-point a local exactly as an expression
            // statement does, and the enclosing loop owes each of them the
            // same header phi.
            StmtKind::If { arms, else_ } => {
                for arm in arms {
                    self.collect_reassigned_in_expr(&arm.cond, seen, out);
                    self.collect_reassigned_locals(&arm.then, seen, out);
                }
                if let Some(e) = else_ {
                    self.collect_reassigned_locals(e, seen, out);
                }
            }
            StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
                self.collect_reassigned_in_expr(cond, seen, out);
                self.collect_reassigned_locals(body, seen, out);
            }
            StmtKind::Foreach { subject, body, .. } => {
                self.collect_reassigned_in_expr(subject, seen, out);
                self.collect_reassigned_locals(body, seen, out);
            }
            StmtKind::Return(Some(e))
            | StmtKind::Break(Some(e))
            | StmtKind::Continue(Some(e))
            | StmtKind::LocalDecl { value: Some(e), .. }
            | StmtKind::Destructure { value: e, .. } => {
                self.collect_reassigned_in_expr(e, seen, out);
            }
            StmtKind::Echo(operands) => {
                for e in operands {
                    self.collect_reassigned_in_expr(e, seen, out);
                }
            }
            // A `switch` nested in a loop body is one more place a local is
            // written, and a header phi that misses it reads the pre-loop
            // value forever — the same silent failure the `Try` arm below
            // exists for.
            StmtKind::Switch { subject, cases } => {
                self.collect_reassigned_in_expr(subject, seen, out);
                for case in cases {
                    if let Some(cond) = &case.cond {
                        self.collect_reassigned_in_expr(cond, seen, out);
                    }
                    for s in &case.body {
                        self.collect_reassigned_locals(s, seen, out);
                    }
                }
            }
            // A `for`'s step clause writes the loop variable of the loop it
            // belongs to, and an enclosing loop needs a header phi for it just
            // as much as the inner one does — see `Self::lower_for`, which
            // asks the same two questions of its own body and step.
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => {
                self.collect_reassigned_locals(body, seen, out);
                // `rule:iteration/for-init-clause`'s declaration form goes through the `LocalDecl`
                // arm above, which scans the initializer without registering
                // the counter — the counter is declared here, not re-pointed,
                // so an enclosing loop owes it no phi, but `int $i = $n++`
                // re-points `$n` and does owe one.
                if let Some(decl) = init.decl() {
                    self.collect_reassigned_locals(decl, seen, out);
                }
                for e in init.exprs().iter().chain(cond).chain(step) {
                    self.collect_reassigned_in_expr(e, seen, out);
                }
            }
            // A protected region is three more places a local is written, and
            // a loop-header phi that misses one carries the pre-loop value
            // forever — silently, since nothing downstream can tell a missing
            // phi from a local the body never touched.
            StmtKind::Try {
                body,
                catches,
                finally,
            } => {
                for s in &body.stmts {
                    self.collect_reassigned_locals(s, seen, out);
                }
                for clause in catches {
                    for s in &clause.body.stmts {
                        self.collect_reassigned_locals(s, seen, out);
                    }
                }
                if let Some(block) = finally {
                    for s in &block.stmts {
                        self.collect_reassigned_locals(s, seen, out);
                    }
                }
            }
            _ => {}
        }
    }
    /// One expression evaluated for its effect — an expression statement's own
    /// expression, or one clause of a `for` header — scanned for the locals it
    /// re-points, exactly as [`Self::collect_reassigned_locals`] scans a
    /// statement. Split out because a `for`'s step clause is a bare [`Expr`]
    /// with no [`Stmt`] wrapping it, and it owes the identical phi.
    pub(crate) fn collect_reassigned_in_expr(
        &self,
        e: &Expr,
        seen: &mut FxHashSet<String>,
        out: &mut Vec<String>,
    ) {
        // `$i++` re-points its local exactly the way the `$i = $i + 1` it
        // lowers to does (`Self::lower_incdec_stmt`), and it is the *usual*
        // spelling of a `for` step — a missed phi there reads the pre-loop
        // value on every iteration, which is an infinite loop rather than a
        // diagnostic.
        let target = match &e.kind {
            ExprKind::Assign {
                target,
                by_ref: false,
                ..
            }
            | ExprKind::PreIncDec { expr: target, .. }
            | ExprKind::PostIncDec { expr: target, .. } => Some(target),
            _ => None,
        };
        if let Some(target) = target
            && let Some(name) = self.rebound_local(target)
            && seen.insert(name.clone())
        {
            out.push(name);
        }
        // An `inout $x` argument re-points its holder just as an assignment does —
        // `Self::write_back_holder` is literally where — but nothing in the
        // statement's *syntax* says so, since the `&` is on the callee's
        // declaration. See below.
        self.collect_inout_holders(e, seen, out);
        self.collect_reassigned_in_children(e, seen, out);
    }
    /// [`Self::collect_reassigned_in_expr`] applied to every sub-expression of
    /// `e`, so that a rebinding written *inside* another expression is found.
    ///
    /// This walk is what an increment in **value** position costs: a shape
    /// that re-points a local need not be the whole expression, so looking at
    /// `e`'s own kind is not enough. `int $c = $b++ + $b++;` puts one
    /// arbitrarily deep, and a loop header that misses its phi reads the
    /// pre-loop value on every iteration — silently, since nothing downstream
    /// can tell a missing phi from a local the body never touched.
    ///
    /// **A closure body is deliberately not walked.** `rule:types/implicit-capture` captures by
    /// value, so `fn () => $x++` re-points the environment object's own copy
    /// and the enclosing local is untouched; a header phi for it would
    /// describe a write that never happens. The same goes for an anonymous
    /// class's members, which are a declaration and not this body's code.
    ///
    /// Every [`ExprKind`] on disk is named below. The trailing `_` arm is the
    /// cross-crate `#[non_exhaustive]` tax and nothing else — a variant added
    /// to `nvs-syntax` has to be listed here too, or an increment written
    /// inside it goes unseen.
    fn collect_reassigned_in_children(
        &self,
        e: &Expr,
        seen: &mut FxHashSet<String>,
        out: &mut Vec<String>,
    ) {
        match &e.kind {
            ExprKind::Null
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Duration(_)
            | ExprKind::Str(_)
            | ExprKind::Variable(_)
            | ExprKind::ConstFetch(_)
            | ExprKind::SelfExpr
            | ExprKind::StaticExpr
            | ExprKind::ParentExpr
            | ExprKind::Fn(_)
            | ExprKind::Error(_) => {}
            // A markup literal's holes are walked beside an interpolation's
            // for the same reason: `html`{$i++}`` re-points `$i` exactly as
            // `"{$i++}"` does, whatever the two literals then denote.
            ExprKind::Interpolated(parts) | ExprKind::Markup(parts) => {
                for part in parts {
                    if let StringPart::Expr(x) = part {
                        self.collect_reassigned_in_expr(x, seen, out);
                    }
                }
            }
            ExprKind::ArrayLiteral(items) => {
                for item in items {
                    if let Some(key) = &item.key {
                        self.collect_reassigned_in_expr(key, seen, out);
                    }
                    self.collect_reassigned_in_expr(&item.value, seen, out);
                }
            }
            ExprKind::ObjectLiteral(fields) => {
                for field in fields {
                    self.collect_reassigned_in_expr(&field.value, seen, out);
                }
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::PreIncDec { expr: inner, .. }
            | ExprKind::PostIncDec { expr: inner, .. }
            | ExprKind::Conversion { expr: inner, .. }
            | ExprKind::Clone(inner)
            | ExprKind::YieldFrom(inner)
            | ExprKind::Print(inner)
            | ExprKind::Throw(inner)
            | ExprKind::Empty(inner)
            | ExprKind::Paren(inner)
            | ExprKind::Require { path: inner } => {
                self.collect_reassigned_in_expr(inner, seen, out)
            }
            ExprKind::TypeTest {
                expr: inner,
                against,
            } => {
                self.collect_reassigned_in_expr(inner, seen, out);
                if let TestOperand::Value(operand) = against {
                    self.collect_reassigned_in_expr(operand, seen, out);
                }
            }
            ExprKind::Binary { lhs, rhs, .. } => {
                self.collect_reassigned_in_expr(lhs, seen, out);
                self.collect_reassigned_in_expr(rhs, seen, out);
            }
            ExprKind::Assign { target, value, .. } => {
                self.collect_reassigned_in_expr(target, seen, out);
                self.collect_reassigned_in_expr(value, seen, out);
            }
            ExprKind::Ternary { cond, then, else_ } => {
                self.collect_reassigned_in_expr(cond, seen, out);
                if let Some(then) = then {
                    self.collect_reassigned_in_expr(then, seen, out);
                }
                self.collect_reassigned_in_expr(else_, seen, out);
            }
            ExprKind::Call { callee, args } => {
                self.collect_reassigned_in_expr(callee, seen, out);
                if let CallArgs::List(list) = args {
                    for arg in list {
                        self.collect_reassigned_in_expr(&arg.value, seen, out);
                    }
                }
            }
            ExprKind::MethodCall { object, args, .. } => {
                self.collect_reassigned_in_expr(object, seen, out);
                if let CallArgs::List(list) = args {
                    for arg in list {
                        self.collect_reassigned_in_expr(&arg.value, seen, out);
                    }
                }
            }
            ExprKind::StaticCall { class, args, .. } => {
                self.collect_reassigned_in_expr(class, seen, out);
                if let CallArgs::List(list) = args {
                    for arg in list {
                        self.collect_reassigned_in_expr(&arg.value, seen, out);
                    }
                }
            }
            ExprKind::PropertyAccess { object, .. } => {
                self.collect_reassigned_in_expr(object, seen, out)
            }
            ExprKind::StaticPropertyAccess { class, .. }
            | ExprKind::ClassConstAccess { class, .. }
            | ExprKind::ClassNameConst { class } => {
                self.collect_reassigned_in_expr(class, seen, out)
            }
            ExprKind::Index { base, index } => {
                self.collect_reassigned_in_expr(base, seen, out);
                if let Some(index) = index {
                    self.collect_reassigned_in_expr(index, seen, out);
                }
            }
            ExprKind::New { target, args, .. } => {
                if let NewTarget::Expr(class) = target {
                    self.collect_reassigned_in_expr(class, seen, out);
                }
                if let CallArgs::List(list) = args {
                    for arg in list {
                        self.collect_reassigned_in_expr(&arg.value, seen, out);
                    }
                }
            }
            ExprKind::Catch { guarded, arms } => {
                self.collect_reassigned_in_expr(guarded, seen, out);
                for arm in arms {
                    self.collect_reassigned_in_expr(&arm.body, seen, out);
                }
            }
            ExprKind::Match { subject, arms } => {
                self.collect_reassigned_in_expr(subject, seen, out);
                for arm in arms {
                    for cond in arm.conditions.iter().flatten() {
                        self.collect_reassigned_in_expr(cond, seen, out);
                    }
                    self.collect_reassigned_in_expr(&arm.body, seen, out);
                }
            }
            ExprKind::Yield { key, value } => {
                if let Some(key) = key {
                    self.collect_reassigned_in_expr(key, seen, out);
                }
                if let Some(value) = value {
                    self.collect_reassigned_in_expr(value, seen, out);
                }
            }
            ExprKind::Isset(targets) => {
                for target in targets {
                    self.collect_reassigned_in_expr(target, seen, out);
                }
            }
            ExprKind::Exit(Some(code)) => self.collect_reassigned_in_expr(code, seen, out),
            ExprKind::Exit(None) => {}
            ExprKind::SpawnScript { path, options } => {
                self.collect_reassigned_in_expr(path, seen, out);
                for option in options {
                    self.collect_reassigned_in_expr(&option.value, seen, out);
                }
            }
            ExprKind::Await(inner) => self.collect_reassigned_in_expr(inner, seen, out),
            _ => {}
        }
    }
    /// [`Self::collect_reassigned_locals`]'s by-reference half: every local a
    /// call somewhere inside `e` re-points by handing it to an `inout $x`
    /// parameter.
    ///
    /// Separate from the assignment scan because the two read different
    /// things. An assignment says so in its own syntax; a by-reference
    /// argument does not — the `&` lives on the *callee's* declaration, so
    /// `Adder::bump($n)` is indistinguishable from a by-value call until the
    /// resolved signature is consulted. That is what
    /// `nvs_types::expr_table::ResolvedCall::inout` is recorded for, and
    /// missing this scan leaves a loop body writing back into the value the
    /// loop was *entered* with on every iteration — the exact failure
    /// [`Self::rebound_local`]'s own doc comment describes for an array
    /// element.
    ///
    /// Node-local: it asks only about `e`'s *own* argument list, because
    /// [`Self::collect_reassigned_in_children`] is what descends. A nested
    /// call (`Foo::a(Bar::b($n))`) is therefore still found — each call writes
    /// its own staging back at its own site ([`Self::flush_ref_writebacks`]),
    /// so both owe a header phi — but it is found once rather than once per
    /// level.
    pub(crate) fn collect_inout_holders(
        &self,
        e: &Expr,
        seen: &mut FxHashSet<String>,
        out: &mut Vec<String>,
    ) {
        let args = match &e.kind {
            ExprKind::MethodCall { args, .. }
            | ExprKind::StaticCall { args, .. }
            | ExprKind::New { args, .. } => args,
            _ => return,
        };
        let inout: &[bool] = match self.exprs.lookup(e.span) {
            Some(ExprInfo::Call(call)) => &call.inout,
            Some(ExprInfo::New {
                ctor: Some(call), ..
            }) => &call.inout,
            _ => &[],
        };
        let CallArgs::List(list) = args else {
            return;
        };
        for (index, arg) in list.iter().enumerate() {
            if inout.get(index).copied().unwrap_or(false)
                && let Some(name) = self.rebound_local(&arg.value)
                && seen.insert(name.clone())
            {
                out.push(name);
            }
        }
    }
    /// Which local, if any, an assignment or `unset` target re-points — the
    /// name [`Self::collect_reassigned_locals`] owes a loop-header phi.
    ///
    /// A bare `$x` is the obvious one. An array element (`$a[$k]`, `$a[]`) is
    /// the less obvious one and matters just as much: `rule:types/arrays`'s
    /// copy-on-write separation produces a *different* allocation, which
    /// [`Self::write_back_array`] stores back into the base's own local slot.
    /// Missing that phi would leave a loop body writing into the value the
    /// loop was entered with on every iteration instead of the one the last
    /// iteration produced — which is only invisible while the array is solely
    /// owned and therefore never actually separates.
    ///
    /// A property target (`$obj->p`, `$obj->items[$k]`) rebinds no local at
    /// all: the write goes to the object's own slot, which no phi describes.
    pub(crate) fn rebound_local(&self, target: &Expr) -> Option<String> {
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
}
