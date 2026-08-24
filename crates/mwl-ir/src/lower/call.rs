//! Call lowering: argument ownership, ADR 0063 R2's options bag flattened at the site, and a `&$x` argument staged and written back.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

impl<'a> Lowering<'a> {
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
    /// A `&$x` parameter's argument is **staged** instead (see [`Ty::Ref`]):
    /// the holder's current value is read, retained, copied into a fresh
    /// one-cell slot, and that slot's address is what the callee receives.
    /// The matching copy-back is parked in [`Self::pending_refs`] for
    /// [`Self::flush_ref_writebacks`] to emit once the call has returned.
    /// `ownership` does not apply to one: a staged argument is neither
    /// borrowed nor transferred, it is copied, and the retain that pays for
    /// the copy-back's release is emitted unconditionally rather than only for
    /// an aliasing read.
    ///
    /// # Panics
    ///
    /// Panics naming the unsupported shape for anything outside this slice's
    /// scope: a variadic signature, a named or spread argument (`mwl_types`
    /// itself doesn't fully positionally type-check these against a signature
    /// yet — see its own known gaps), more arguments than `sig` has parameters
    /// or a missing one with no default (this crate trusts
    /// `mwl_types::check_program` already enforced arity for a non-variadic
    /// signature), or a by-reference argument that is neither a bare local nor
    /// a compile-time-known property.
    ///
    /// # Omitted arguments
    ///
    /// A call may stop short of `sig`'s parameter list: every parameter past
    /// the last written argument is materialized from its own default by
    /// [`Self::emit_const_arg`], in declaration order, so the callee still
    /// receives exactly one value per parameter.
    pub(super) fn lower_call_args(
        &mut self,
        args: &CallArgs,
        sig: &ArgSig,
        checked_types: &TypeInterner,
        ownership: ArgOwnership,
        env: &Env,
        cur: &mut BlockId,
    ) -> LoweredArgs {
        assert!(
            !sig.variadic,
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
        assert!(
            list.len() <= sig.param_tys.len(),
            "mwl-ir: a resolved call passes more arguments than its signature has parameters — \
             this crate trusts mwl_types::check_program already enforced arity"
        );
        let mut out = LoweredArgs::default();
        for (index, (arg, &pty)) in list.iter().zip(&sig.param_tys).enumerate() {
            // ADR 0063 R2's options bag: not one argument but one *per
            // declared option*, so it never reaches `lower_checked_ty` — it
            // has no IR type at all. See [`Self::lower_options_arg`].
            if let CheckedTy::Options(options) = checked_types.get(pty) {
                let options = options.clone();
                let defaults = options_defaults(&sig.defaults, index);
                self.lower_options_arg(
                    Some(&arg.value),
                    &options,
                    defaults,
                    checked_types,
                    ownership,
                    env,
                    cur,
                    &mut out,
                );
                continue;
            }
            // A `Core` parameter declared as a union has no single IR
            // representation to expect, and needs none: the helper's slot is a
            // tagged `Value` that `mwl-codegen` writes from the *argument's*
            // own representation. See `ArgSig::helper`, which owns why the
            // same declaration on a compiled MWL function is not lowerable.
            let expected = match sig.expectation(index, checked_types) {
                Some(expected) => expected,
                None => {
                    let (v, ty) = self.lower_expr(&arg.value, None, env, cur);
                    let aliasing = self.aliasing_read(&arg.value);
                    self.account_for_arg(v, ty, ownership, aliasing, &mut out, *cur);
                    out.values.push(v);
                    continue;
                }
            };
            if sig.is_by_ref(index) {
                out.values
                    .push(self.stage_ref_arg(&arg.value, expected, env, cur));
                continue;
            }
            let (v, ty) = self.lower_expr(&arg.value, Some(expected), env, cur);
            let aliasing = self.aliasing_read(&arg.value);
            self.account_for_arg(v, ty, ownership, aliasing, &mut out, *cur);
            // A parameter declared wider than the argument -- `?T` or another
            // union -- is `Ty::Tagged`, so the argument is widened into the
            // slot's representation here. `Self::coerce` transfers whatever
            // ownership `account_for_arg` just settled, so the order of the
            // two does not matter.
            let v = self.coerce(*cur, v, ty, expected);
            out.values.push(v);
        }
        for (index, default) in sig.defaults.iter().enumerate().skip(list.len()) {
            let default = default.as_ref().unwrap_or_else(|| {
                panic!(
                    "mwl-ir: parameter {index} was omitted at a call site and has no default — \
                     this crate trusts mwl_types::check_program already enforced arity"
                )
            });
            // A bag omitted whole is every one of its options taking its own
            // default, in the same declared order a written one flattens in.
            if let mwl_types::ConstArg::Options(options) = default {
                for (_, value) in options {
                    let (v, ty) = self.emit_const_arg(value, *cur);
                    self.account_for_arg(v, ty, ownership, false, &mut out, *cur);
                    out.values.push(v);
                }
                continue;
            }
            let (v, ty) = self.emit_const_arg(default, *cur);
            // A materialized default is always freshly built, never a read of
            // storage someone else owns — so `aliasing` is `false` here by
            // construction.
            self.account_for_arg(v, ty, ownership, false, &mut out, *cur);
            out.values.push(v);
        }
        out
    }
    /// Which frame owes a release for one lowered argument, and whether it has
    /// to take a reference first — the caller-side half of the refcount
    /// protocol, in one table:
    ///
    /// * A **borrowed** argument some other binding already owns (`aliasing`)
    ///   is that binding's to release, not this call site's.
    /// * A **borrowed** argument this expression built — a `fn` literal, a
    ///   nested call's result, a concatenation, a materialized default — has
    ///   exactly one owner, and it is this frame.
    /// * A **transferred** argument is released by the callee's own exit sweep
    ///   either way; a copy of storage someone else owns needs a second
    ///   reference first, a freshly built value does not.
    ///
    /// Shared by the written arguments, the materialized defaults and each
    /// flattened option, so a bag's options are accounted exactly as the
    /// arguments beside them are.
    pub(super) fn account_for_arg(
        &mut self,
        v: ValueId,
        ty: Ty,
        ownership: ArgOwnership,
        aliasing: bool,
        out: &mut LoweredArgs,
        cur: BlockId,
    ) {
        if !ty.is_refcounted() {
            return;
        }
        match (ownership, aliasing) {
            (ArgOwnership::Borrowed, true) | (ArgOwnership::Transferred, false) => {}
            (ArgOwnership::Borrowed, false) => out.temporaries.push(v),
            (ArgOwnership::Transferred, true) => self.emit_retain(cur, v),
        }
    }
    /// Flattens one ADR 0063 R2 options bag into `out`: one value per option
    /// `options` declares, in that declared order — the written field's value
    /// where the call site gave one, the option's own default where it did
    /// not.
    ///
    /// `written` is the object literal the call site passed, or `None` for a
    /// bag omitted entirely. This is why an options argument has to be a
    /// literal at the call site (`mwl_types` reports `E_OPTIONS_NOT_A_LITERAL`
    /// for anything else): the flattening is per-option and static, so there
    /// is nothing to read a variable's fields out of. Nothing below this line
    /// — not `mwl-codegen`, not the helper convention a `Core` member is
    /// reached through — learns that bags exist, exactly as nothing learns
    /// that defaults do ([`Self::emit_const_arg`]).
    ///
    /// # Panics
    ///
    /// Panics if `written` is not an object literal, or if an option has
    /// neither a written field nor a default: both are shapes
    /// `mwl_types::check_program` and `mwl_types::core_lib` are trusted to
    /// have made impossible.
    #[expect(
        clippy::too_many_arguments,
        reason = "the call-lowering context `Self::lower_call_args` already \
                  threads — expectation, ownership, environment, block — plus \
                  the bag's own two halves; bundling them into a struct would \
                  be one type used at one call site"
    )]
    pub(super) fn lower_options_arg(
        &mut self,
        written: Option<&Expr>,
        options: &[(String, TypeId)],
        defaults: &[(String, mwl_types::ConstArg)],
        checked_types: &TypeInterner,
        ownership: ArgOwnership,
        env: &Env,
        cur: &mut BlockId,
        out: &mut LoweredArgs,
    ) {
        let fields: Vec<(String, &Expr)> = match written {
            Some(expr) => match &expr.kind {
                ExprKind::ObjectLiteral(fields) => fields
                    .iter()
                    .map(|field| (span_text(self.src, field.name).to_owned(), &field.value))
                    .collect(),
                other => panic!(
                    "mwl-ir: an options argument lowered from {other:?} rather than an object \
                     literal — mwl_types::check_program is trusted to have reported \
                     E_OPTIONS_NOT_A_LITERAL for anything else"
                ),
            },
            None => Vec::new(),
        };
        for (name, option_ty) in options {
            if let Some((_, value)) = fields.iter().find(|(field, _)| field == name) {
                let expected = lower_checked_ty(*option_ty, checked_types);
                let (v, ty) = self.lower_expr(value, Some(expected), env, cur);
                let aliasing = self.aliasing_read(value);
                self.account_for_arg(v, ty, ownership, aliasing, out, *cur);
                out.values.push(v);
                continue;
            }
            let default = defaults
                .iter()
                .find(|(option, _)| option == name)
                .map(|(_, value)| value)
                .unwrap_or_else(|| {
                    panic!(
                        "mwl-ir: the option `{name}` was omitted at a call site and has no \
                         default — mwl_types::core_lib is trusted to record one per declared \
                         option"
                    )
                });
            let (v, ty) = self.emit_const_arg(default, *cur);
            self.account_for_arg(v, ty, ownership, false, out, *cur);
            out.values.push(v);
        }
    }
    /// Materializes one omitted parameter's default as an ordinary constant in
    /// `cur`.
    ///
    /// This is the whole of MWL's default-argument mechanism at the IR level,
    /// which is the point of evaluating a default at signature collection
    /// rather than in the callee (`mwl_types::defaults` owns why): every
    /// compiled function keeps exactly one arity, so nothing below this line —
    /// not the ADR 0002 call ABI, not `mwl-codegen`, not the helper
    /// convention a `Core` member is reached through — learns that defaults
    /// exist at all.
    ///
    /// A `ConstArg::Str` allocates a fresh string per evaluation, exactly as a
    /// written string literal does today (`mwl-codegen`'s known gap 4); it is
    /// the same `InstKind::ConstStr` and closing that gap closes both.
    pub(super) fn emit_const_arg(
        &mut self,
        default: &mwl_types::ConstArg,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        let (ty, kind) = match default {
            mwl_types::ConstArg::Null => (Ty::Null, InstKind::ConstNull),
            mwl_types::ConstArg::Bool(b) => (Ty::Bool, InstKind::ConstBool(*b)),
            mwl_types::ConstArg::Int(v) => (Ty::Int, InstKind::ConstInt(*v)),
            mwl_types::ConstArg::Uint(v) => (Ty::Uint, InstKind::ConstUint(*v)),
            mwl_types::ConstArg::Float(v) => (Ty::Float, InstKind::ConstFloat(*v)),
            mwl_types::ConstArg::Str(s) => (Ty::Str, InstKind::ConstStr(s.clone())),
            // A bag has no single constant to emit — it is one per option, so
            // its own two call sites expand it before reaching here.
            mwl_types::ConstArg::Options(_) => panic!(
                "mwl-ir: an options bag has no IR constant of its own; \
                 `Lowering::lower_options_arg` expands it per option"
            ),
        };
        self.emit(cur, ty, kind)
    }
    /// Releases what [`LoweredArgs::temporaries`] collected, after the call
    /// that borrowed them has been emitted into `cur`.
    ///
    /// **Known gap, and the same one [`Self::landing_block`] already has:**
    /// these sit on the normal edge only, so a helper that fails leaves each
    /// of them unreleased. Closing it means the owned-temporaries stack
    /// threaded through [`Self::lower_expr`] that `docs/agent/loop-goal.md`
    /// already names — this is one more caller for it, not a second design.
    pub(super) fn release_call_temporaries(&mut self, temporaries: Vec<ValueId>, cur: BlockId) {
        for v in temporaries {
            self.emit_release(cur, v);
        }
    }
    /// Stages one by-reference argument, returning the [`Ty::Ref`] the callee
    /// is handed — see [`Ty::Ref`], which owns the representation, and
    /// [`Self::lower_call_args`], which owns why `ownership` does not reach
    /// here.
    ///
    /// The holder's receiver (for a property) is lowered exactly once, here,
    /// and remembered in the [`RefHolder`] so the copy-back re-uses it rather
    /// than evaluating it a second time.
    pub(super) fn stage_ref_arg(
        &mut self,
        arg: &Expr,
        ty: Ty,
        env: &Env,
        cur: &mut BlockId,
    ) -> ValueId {
        let (holder, init) = match &arg.kind {
            ExprKind::Variable(name_span) => {
                let name = strip_sigil(span_text(self.src, *name_span)).to_owned();
                let (v, _) = self.lower_expr(arg, Some(ty), env, cur);
                (RefHolder::Local(name), v)
            }
            ExprKind::PropertyAccess { object, .. } => {
                let Some(ExprInfo::Property { class, name, .. }) = self.exprs.lookup(arg.span)
                else {
                    panic!(
                        "mwl-ir: the by-reference argument at {:?} is a property with no \
                         resolved declaring class recorded in the typed-expression table — \
                         either it wasn't checked with the same table, or its receiver erased \
                         to a shape/plain `object` (ADR 0036 § 4); mwl_types' \
                         `check_by_ref_arg` is expected to have refused both",
                        arg.span
                    );
                };
                let class = class.to_string();
                let field = name.clone();
                let (object_v, _) = self.lower_expr(object, None, env, cur);
                let (v, _) = self.emit(
                    *cur,
                    ty,
                    InstKind::FieldGet {
                        object: object_v,
                        class: class.clone(),
                        field: field.clone(),
                    },
                );
                (
                    RefHolder::Field {
                        object: object_v,
                        class,
                        field,
                    },
                    v,
                )
            }
            other => panic!(
                "mwl-ir stages a by-reference argument only from a bare local or a \
                 compile-time-known property — not from {other:?}; mwl_types' \
                 `check_by_ref_arg` is expected to have refused it at the call site"
            ),
        };
        // The staging retain: from here the slot owns one reference of its
        // own, which `Self::write_back_holder`'s release pays back. See
        // `Ty::Ref`'s refcounting section.
        if ty.is_refcounted() {
            self.emit_retain(*cur, init);
        }
        let (slot, _) = self.emit(*cur, Ty::Ref, InstKind::RefSlot { init });
        self.pending_refs.push(StagedRef { holder, slot, ty });
        slot
    }
}
