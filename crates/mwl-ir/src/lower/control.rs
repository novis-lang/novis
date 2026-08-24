//! Control flow — `if`, `while`, both `foreach` shapes, `break`/`continue`, and the env merge every join needs.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

impl<'a> Lowering<'a> {
    /// `if (cond) then (else else_)?` — the module docs describe the
    /// merge-point construction this drives.
    ///
    /// # Panics
    ///
    /// Panics naming the case for a `cond` whose static type
    /// [`Self::lower_truthy_cond`] doesn't yet convert — see that method's
    /// own doc comment for exactly what's covered and what still isn't
    /// (`null`/`mixed`/a union).
    pub(super) fn lower_if(
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
    pub(super) fn lower_while(
        &mut self,
        cond: &Expr,
        body: &'a Stmt,
        cur: &mut BlockId,
        env: &mut Env,
    ) {
        let mut seen = FxHashSet::default();
        let mut reassigned = Vec::new();
        self.collect_reassigned_locals(body, &mut seen, &mut reassigned);
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
            // A `&$x` parameter's binding is an address that never changes:
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
            carried: header_env.keys().cloned().collect(),
            loop_private: Vec::new(),
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
    pub(super) fn lower_foreach(
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
        // ADR 0053 § 3's three shapes are three loops, and which one this is
        // was decided by the checker — `mwl-ir` cannot re-derive it, because
        // reaching `Iterable` through a base class needs the `ClassGraph`
        // this crate deliberately does not depend on. See
        // `mwl_types::ExprTypeTable::foreach_drive`.
        let drive = self.exprs.foreach_drive(subject.span).unwrap_or_else(|| {
            panic!(
                "mwl-ir: the `foreach` subject at {:?} has no recorded `ForeachDrive` — either \
                 it erased to `mixed` (unsupported, see the crate docs' known gaps) or this \
                 program did not pass mwl_types::check_program with the same table",
                subject.span
            )
        });
        if drive != ForeachDrive::Array {
            assert!(
                key.is_none(),
                "mwl-ir: a `foreach` key binding over an `Iterable`/`Iterator` subject reached \
                 lowering — ADR 0053 § 1 gives a cursor no key at all, and mwl_types reports \
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
        if self.aliasing_read(subject) {
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
            // A `&$x` parameter's binding is an address that never changes:
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
            carried: header_env.keys().cloned().collect(),
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
    /// `foreach ($subject as $v) body` over ADR 0053 § 3's other two shapes —
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
    ///   [`InstKind::Call`]. ADR 0053 § 1's interfaces declare `advance`,
    ///   `current` and `iterate` without bodies, and `mwl_types::iter_lib`'s
    ///   own docs own why that is the mechanism rather than an accident: a
    ///   call resolving to a bodiless declaration names no compiled function,
    ///   so it dispatches on the receiver's runtime class — which is exactly
    ///   what driving a cursor whose concrete class the loop never knows
    ///   needs. This method therefore emits the instructions itself instead
    ///   of routing a synthesized AST node through [`Self::lower_expr`];
    ///   there is no `$cursor->advance()` in the source to look up.
    /// * **The loop owns one reference to the cursor**, released in the
    ///   after-block, and *retains it again before each member call* — a
    ///   receiver is parameter 0 and MWL transfers an argument's reference to
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
    ///   the single reference `Self::lower_expr_top` produced, which is why
    ///   that value is never entered into the `Env` and the after-block
    ///   releases the *cursor* instead.
    /// * **There is no cursor phi and no key.** The driven value never
    ///   changes — the position it walks lives inside the cursor object, not
    ///   in this frame — and ADR 0053 § 1 gives `Iterator<T>` no key member
    ///   for a binding to read.
    ///
    /// # Panics
    ///
    /// Panics if the subject did not lower to a [`Ty::Object`], which would
    /// mean `mwl_types` classified something as a cursor that has no runtime
    /// class to dispatch on.
    pub(super) fn lower_foreach_cursor(
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

        let (subject_v, subject_ty) = self.lower_expr_top(subject, None, env, cur);
        assert!(
            subject_ty == Ty::Object,
            "mwl-ir: a `foreach` subject mwl_types classified as a cursor lowered to \
             {subject_ty:?} rather than an object — ADR 0053 § 3's `Iterable`/`Iterator` shapes \
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
            // See `Self::lower_foreach`: a `&$x` parameter's binding is an
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
            });
            header_env.insert(name.clone(), (phi_v, ty));
            phi_slots.push((name.clone(), inst_index));
        }

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
            header_block,
            after_block,
            continue_edges: Vec::new(),
            break_edges: Vec::new(),
            iteration_owned: vec![value_name.clone()],
            carried: header_env.keys().cloned().collect(),
            loop_private: vec![cursor_name.clone()],
        });

        let mut body_env = header_env.clone();
        let mut body_cur = body_block;
        let v_v = self.emit_iface_call(body_cur, cursor_v, true, "current", value_ty, &body_env);
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
        exit_env.remove(&cursor_name);
        let mut after_incoming: Vec<(BlockId, Env)> = vec![(header_block, exit_env.clone())];
        after_incoming.extend(frame.break_edges);
        *env = self.merge_envs(after_block, &after_incoming, &exit_env);
        self.emit_release(after_block, cursor_v);
        *cur = after_block;
    }
    /// One ADR 0053 § 1 member call on `receiver`, emitted directly rather
    /// than lowered from source — see [`Self::lower_foreach_cursor`] for why
    /// there is no AST node to route through.
    ///
    /// `keep_receiver` says whether the caller still owns its reference
    /// afterwards. A receiver is parameter 0 and MWL transfers an argument's
    /// reference to the callee ([`ArgOwnership::Transferred`]), on the
    /// callee's throw path as much as its return path, so keeping one means
    /// retaining a second — which is what the loop's per-iteration
    /// `advance()`/`current()` pair does, and what the once-only `iterate()`
    /// deliberately does not.
    pub(super) fn emit_iface_call(
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
    pub(super) fn lower_break(&mut self, level: &Option<Expr>, cur: &mut BlockId, env: &Env) {
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
    /// Releases everything the innermost loop's *current iteration* owns and
    /// drops it from `env` — the one thing every point an iteration ends has
    /// in common (the body's fall-through back edge, a `continue`, a `break`).
    ///
    /// Two sets, and they are disjoint by construction:
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
    pub(super) fn end_iteration(&mut self, cur: BlockId, env: &mut Env) {
        let Some(frame) = self.loop_stack.last() else {
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
    pub(super) fn lower_continue(&mut self, level: &Option<Expr>, cur: &mut BlockId, env: &Env) {
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
    pub(super) fn loop_exit_level(&self, level: &Option<Expr>, keyword: &str) {
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
    pub(super) fn merge_envs(
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
    /// that never bound it, and MWL has no `null` in the IR to phi in), and
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
    pub(super) fn release_merged_away(&mut self, incoming: &[(BlockId, Env)], name: &str) {
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
    pub(super) fn seed_generator_loop_carried(
        &self,
        env: &Env,
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
    pub(super) fn collect_reassigned_locals(
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
                // A `&$x` argument re-points its holder just as an assignment
                // does — `Self::write_back_holder` is literally where — but
                // nothing in the statement's *syntax* says so, since the `&`
                // is on the callee's declaration. See below.
                self.collect_by_ref_holders(e, seen, out);
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
    /// [`Self::collect_reassigned_locals`]'s by-reference half: every local a
    /// call somewhere inside `e` re-points by handing it to a `&$x`
    /// parameter.
    ///
    /// Separate from the assignment scan because the two read different
    /// things. An assignment says so in its own syntax; a by-reference
    /// argument does not — the `&` lives on the *callee's* declaration, so
    /// `Adder::bump($n)` is indistinguishable from a by-value call until the
    /// resolved signature is consulted. That is what
    /// `mwl_types::expr_table::ResolvedCall::by_ref` is recorded for, and
    /// missing this scan leaves a loop body writing back into the value the
    /// loop was *entered* with on every iteration — the exact failure
    /// [`Self::rebound_local`]'s own doc comment describes for an array
    /// element.
    ///
    /// Walks nested calls too (`Foo::a(Bar::b($n))`): both stagings are
    /// flushed by the same [`Self::flush_ref_writebacks`] call, so both owe a
    /// header phi.
    pub(super) fn collect_by_ref_holders(
        &self,
        e: &Expr,
        seen: &mut FxHashSet<String>,
        out: &mut Vec<String>,
    ) {
        let args = match &e.kind {
            ExprKind::Assign { value, .. } => {
                self.collect_by_ref_holders(value, seen, out);
                return;
            }
            ExprKind::Paren(inner) => {
                self.collect_by_ref_holders(inner, seen, out);
                return;
            }
            ExprKind::MethodCall { args, .. }
            | ExprKind::StaticCall { args, .. }
            | ExprKind::New { args, .. } => args,
            _ => return,
        };
        let by_ref: &[bool] = match self.exprs.lookup(e.span) {
            Some(ExprInfo::Call(call)) => &call.by_ref,
            Some(ExprInfo::New {
                ctor: Some(call), ..
            }) => &call.by_ref,
            _ => &[],
        };
        let CallArgs::List(list) = args else {
            return;
        };
        for (index, arg) in list.iter().enumerate() {
            self.collect_by_ref_holders(&arg.value, seen, out);
            if by_ref.get(index).copied().unwrap_or(false)
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
    pub(super) fn rebound_local(&self, target: &Expr) -> Option<String> {
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
