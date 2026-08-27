//! Call lowering: argument ownership, ADR 0063 R2's options bag flattened at the site, a `&$x` argument staged and written back, and `$fn(...)` through the one helper a `Core` member's callback already takes.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

impl<'a> Lowering<'a> {
    /// Lowers a resolved call's/`new`'s argument list against `param_tys` —
    /// the already-resolved parameter types from
    /// `mwl_types::expr_table::ResolvedCall` — placing each written argument at
    /// the ABI position of the parameter it fills rather than at its own place
    /// in the list. Which parameter that is comes from [`ArgSig::arg_slots`],
    /// which owns why this crate cannot work it out itself. An argument whose
    /// expected type [`Ty::is_refcounted`] and whose source expression
    /// [`is_aliasing_read`] (a bare variable or a compile-time-known property
    /// read) is retained
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
    /// Panics naming the shape for anything this crate trusts
    /// `mwl_types::check_program` to have refused before it ever got here: an
    /// argument that reached no parameter at all, more arguments than `sig`
    /// has parameters, a parameter no argument filled and that has no default,
    /// or a by-reference argument that is neither a bare local nor a
    /// compile-time-known property.
    ///
    /// # A `name:` argument
    ///
    /// Evaluation stays in **written** order — a named argument's own side
    /// effects happen where the call site wrote it — while the value it
    /// produces lands at its parameter's position, so the callee's typed slots
    /// are filled in declaration order however the call spelled them. The
    /// checker guarantees the shape this rests on: at most one argument per
    /// fixed parameter, and every positional argument before the first `name:`
    /// or `...` one, so nothing here has to refuse a mapping.
    ///
    /// # A variadic tail
    ///
    /// A variadic signature's last parameter takes every remaining argument
    /// at once, as one array — see [`Self::lower_variadic_tail`], which owns
    /// the shape and the ownership rule. Everything before it is lowered
    /// exactly as a fixed parameter, defaults included.
    ///
    /// # Omitted arguments
    ///
    /// A call may stop short of `sig`'s parameter list, and with a `name:` in
    /// play it may skip one in the middle: every fixed parameter no argument
    /// filled is materialized from its own default by
    /// [`Self::emit_const_arg`], in declaration order and after every written
    /// argument has been lowered, so the callee still receives exactly one
    /// value per parameter.
    pub(super) fn lower_call_args(
        &mut self,
        args: &CallArgs,
        sig: &ArgSig,
        checked_types: &TypeInterner,
        ownership: ArgOwnership,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> LoweredArgs {
        let CallArgs::List(list) = args else {
            panic!(
                "mwl-ir only lowers a plain positional argument list for a resolved call/`new` \
                 — got {args:?}; see the crate docs' known gaps"
            );
        };
        // A variadic signature's last parameter is not one ABI argument per
        // written argument: it is one array holding all of them, built below.
        // `fixed` is how many parameters still map one-to-one.
        let fixed = match sig.variadic {
            true => sig.param_tys.len() - 1,
            false => sig.param_tys.len(),
        };
        assert_eq!(
            list.len(),
            sig.arg_slots.len(),
            "mwl-ir: a resolved call records one argument slot per written argument — \
             mwl_types is trusted to have mapped every one of them"
        );
        // One entry per fixed parameter, at its own ABI position — `None` until
        // some argument fills it, and a `Vec` rather than a value because ADR
        // 0063 R2's options bag flattens into one value per declared option.
        let mut filled: Vec<Option<LoweredArgs>> = (0..fixed).map(|_| None).collect();
        let mut tail = Vec::new();
        for (arg, slot) in list.iter().zip(&sig.arg_slots) {
            let index = match *slot {
                ArgSlot::Param(index) | ArgSlot::Spread(index) => index,
                ArgSlot::Unresolved => panic!(
                    "mwl-ir: a written argument reached no parameter — this crate trusts \
                     mwl_types::check_program already reported it"
                ),
            };
            // Everything from the variadic parameter onward is one array,
            // built once below out of every argument written into it.
            if index >= fixed {
                tail.push(arg);
                continue;
            }
            let mut one = LoweredArgs::default();
            self.lower_fixed_arg(
                arg,
                index,
                sig,
                checked_types,
                ownership,
                env,
                cur,
                &mut one,
            );
            filled[index] = Some(one);
        }
        assert!(
            sig.variadic || tail.is_empty(),
            "mwl-ir: a resolved call passes more arguments than its signature has parameters — \
             this crate trusts mwl_types::check_program already enforced arity"
        );
        let mut out = LoweredArgs::default();
        for (index, one) in filled.into_iter().enumerate() {
            match one {
                Some(one) => out.values.extend(one.values),
                None => {
                    self.lower_default_arg(
                        index,
                        sig,
                        checked_types,
                        ownership,
                        env,
                        cur,
                        &mut out,
                    );
                }
            }
        }
        if sig.variadic {
            self.lower_variadic_tail(&tail, fixed, sig, ownership, env, cur, &mut out);
        }
        out
    }

    /// One written argument, lowered against the fixed parameter at `index` —
    /// which is the parameter its `ArgSlot` named, not its own place in the
    /// list. Everything it produces goes into `out`, which is that parameter's
    /// own slice of the ABI argument list and nothing else's.
    #[expect(
        clippy::too_many_arguments,
        reason = "the same context `lower_call_args` itself threads; splitting it into a struct \
                  would buy one call site nothing"
    )]
    fn lower_fixed_arg(
        &mut self,
        arg: &mwl_syntax::ast::Arg,
        index: usize,
        sig: &ArgSig,
        checked_types: &TypeInterner,
        ownership: ArgOwnership,
        env: &mut Env,
        cur: &mut BlockId,
        out: &mut LoweredArgs,
    ) {
        // ADR 0063 R2's options bag: not one argument but one *per declared
        // option*, so it never reaches `lower_checked_ty` — it has no IR type
        // at all. See [`Self::lower_options_arg`].
        if let CheckedTy::Options(options) = checked_types.get(sig.param_tys[index]) {
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
                out,
            );
            return;
        }
        // A `Core` parameter declared as a union has no single IR
        // representation to expect, and needs none: the helper's slot is a
        // tagged `Value` that `mwl-codegen` writes from the *argument's* own
        // representation. See `ArgSig::helper`, which owns why the same
        // declaration on a compiled MWL function is not lowerable.
        let expected = match sig.expectation(index, checked_types) {
            Some(expected) => expected,
            None => {
                let (v, ty) = self.lower_expr(&arg.value, None, env, cur);
                let aliasing = self.aliasing_read(&arg.value);
                self.account_for_arg(v, ty, ownership, aliasing, *cur);
                out.values.push(v);
                return;
            }
        };
        if sig.is_by_ref(index) {
            out.values
                .push(self.stage_ref_arg(&arg.value, expected, env, cur));
            return;
        }
        let (v, ty) = self.lower_expr(&arg.value, Some(expected), env, cur);
        let aliasing = self.aliasing_read(&arg.value);
        self.account_for_arg(v, ty, ownership, aliasing, *cur);
        // A parameter declared wider than the argument -- `?T` or another
        // union -- is `Ty::Tagged`, so the argument is widened into the slot's
        // representation here. `Self::coerce` transfers whatever ownership
        // `account_for_arg` just settled, so the order of the two does not
        // matter.
        let v = self.coerce(*cur, v, ty, expected, env);
        out.values.push(v);
    }

    /// The fixed parameter at `index`, which no written argument filled,
    /// materialized from its own recorded default.
    #[expect(
        clippy::too_many_arguments,
        reason = "the same context `lower_call_args` itself threads; splitting it into a struct \
                  would buy one call site nothing"
    )]
    fn lower_default_arg(
        &mut self,
        index: usize,
        sig: &ArgSig,
        checked_types: &TypeInterner,
        ownership: ArgOwnership,
        env: &mut Env,
        cur: &mut BlockId,
        out: &mut LoweredArgs,
    ) {
        let default = sig.defaults[index].as_ref().unwrap_or_else(|| {
            panic!(
                "mwl-ir: parameter {index} was filled by no argument at a call site and has no \
                 default — this crate trusts mwl_types::check_program already enforced arity"
            )
        });
        // A bag omitted whole is every one of its options taking its own
        // default, in the same declared order a written one flattens in —
        // which is exactly what `lower_options_arg` does with no written
        // literal, so it is reached rather than repeated here. Going through
        // it is also what gives an omitted option the same widening into its
        // declared slot that a written one gets.
        if let mwl_types::ConstArg::Options(options) = default {
            let CheckedTy::Options(declared) = checked_types.get(sig.param_tys[index]) else {
                panic!(
                    "mwl-ir: parameter {index} carries an options-bag default but its declared \
                     type is not an options bag — mwl_types is trusted to record the two together"
                );
            };
            self.lower_options_arg(
                None,
                declared,
                options,
                checked_types,
                ownership,
                env,
                cur,
                out,
            );
            return;
        }
        let (v, ty) = self.emit_const_arg(default, env, *cur);
        // A materialized default is always freshly built, never a read of
        // storage someone else owns — so `aliasing` is `false` here by
        // construction.
        self.account_for_arg(v, ty, ownership, false, *cur);
        out.values.push(v);
    }

    /// ADR 0063's variadic tail as the single ABI argument it becomes: every
    /// argument from parameter `fixed` onward collected into one fresh
    /// `array<T>`, keyed `"0"`, `"1"`, … in written order.
    ///
    /// The array is what `mwl_stdlib::registry::CoreTy::Variadic` promises the
    /// helper — a `Tag::Array` in a fixed `args: [N]` slot — so a variadic
    /// member costs one allocation per call and needs no second calling
    /// convention. A call that writes no trailing argument still passes an
    /// array, empty rather than absent, so the body has one shape to read.
    ///
    /// Ownership follows [`ir::InstKind::ArrayNew`]'s own rule rather than
    /// [`Self::account_for_arg`]'s: the array *durably owns* each element the
    /// way a callee's parameter slot does, so a refcounted element that is an
    /// [`Lowering::aliasing_read`] is retained before it is stored, and the
    /// array itself is the one value accounted at the call boundary — a fresh
    /// producer, hence this frame's temporary to release once the call has
    /// returned.
    ///
    /// # A `...` argument
    ///
    /// A spread hands over an array whose *entries* become arguments, so it is
    /// one [`ir::InstKind::ArraySpread`] into the tail array rather than one
    /// entry of it: how many arrived is the subject's own run-time length, and
    /// there is no lowering-time key to give them. `mwl_runtime::mwl_array_spread`
    /// owns which of the subject's keys survive (ADR 0007 § 5) and it is PHP's
    /// unpacking rule as well as PHP's array-literal one — an integer-looking
    /// key is renumbered under the tail's own append counter, so
    /// `f(...$xs, ...$ys)` concatenates, and a string key is preserved, which
    /// is what a `string`-keyed unpack lands in PHP's variadic parameter too.
    ///
    /// The written-out entries keep their single `ArrayNew` because they are
    /// always a *prefix*: a positional argument cannot follow a `...`, and a
    /// `name:` never reaches the variadic parameter at all
    /// (`mwl_types::expr::args::map_arguments` rules 1 and 3). So a call with
    /// no spread emits exactly the instruction it emitted before.
    ///
    /// The subject is **borrowed** by the copy and the half-built array is
    /// named by nothing, so both go on [`Self::owned_temporaries`] while the
    /// copies run — the array re-pointed after each one, and handed back to
    /// [`Self::account_for_arg`] at the end.
    #[expect(
        clippy::too_many_arguments,
        reason = "the same context `lower_call_args` itself threads; splitting it into a struct \
                  would buy one call site nothing"
    )]
    fn lower_variadic_tail(
        &mut self,
        rest: &[&mwl_syntax::ast::Arg],
        fixed: usize,
        sig: &ArgSig,
        ownership: ArgOwnership,
        env: &mut Env,
        cur: &mut BlockId,
        out: &mut LoweredArgs,
    ) {
        let expected = sig.expectation(fixed, self.checked_types);
        // Every argument written out one by one is a prefix of the tail: a
        // positional argument cannot follow a `...` (`mwl_types`' E0488) and a
        // `name:` never reaches the variadic parameter at all, so the first
        // spread is where the lowering-time keys stop.
        let spread_from = rest.iter().position(|arg| arg.spread).unwrap_or(rest.len());
        let mut entries = Vec::with_capacity(spread_from);
        for (index, arg) in rest[..spread_from].iter().enumerate() {
            let (v, ty) = self.lower_expr(&arg.value, expected, env, cur);
            if ty.is_refcounted() && self.aliasing_read(&arg.value) {
                self.emit_retain(*cur, v);
            }
            // The element is stored as a whole `mwl_runtime::Value`, so it is
            // widened into the parameter's own representation here exactly as
            // a fixed argument is — `Self::coerce` is ownership-transparent,
            // so the retain above still pays for what lands in the array.
            let v = match expected {
                Some(expected) => self.coerce(*cur, v, ty, expected, env),
                None => v,
            };
            entries.push((index.to_string(), v));
        }
        let (mut array, ty) = self.emit(*cur, Ty::Array, InstKind::ArrayNew { entries });
        if spread_from < rest.len() {
            // The tail array is named by no local while the copies run, and
            // `ArraySpread` can throw, so it is this frame's temporary and is
            // re-pointed after every write — exactly what an array literal
            // containing a spread does with the array it is building.
            let slot = self.temporaries_mark();
            self.own_temporary(array);
            for arg in &rest[spread_from..] {
                assert!(
                    arg.spread,
                    "mwl-ir: a positional argument follows a `...` in a variadic tail — \
                     this crate trusts mwl_types::check_program already reported it as E0488"
                );
                let mark = self.temporaries_mark();
                // The subject is *borrowed* by the copy, so a freshly-built one
                // is this frame's to release on whichever edge the copy takes.
                let (subject, subject_ty) = self.lower_expr(&arg.value, None, env, cur);
                if subject_ty.is_refcounted() && !self.aliasing_read(&arg.value) {
                    self.own_temporary(subject);
                }
                array = self
                    .emit_fallible(
                        *cur,
                        Ty::Array,
                        InstKind::ArraySpread { array, subject },
                        env,
                    )
                    .0;
                self.retarget_temporary(slot, array);
                self.release_temporaries_since(mark, *cur);
            }
            // The finished array is `account_for_arg`'s from here, exactly as
            // the no-spread one beside it is.
            self.forget_temporary(slot);
        }
        self.account_for_arg(array, ty, ownership, false, *cur);
        out.values.push(array);
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
    ///
    /// The second case is the only one that leaves anything behind, and it
    /// goes on [`Self::owned_temporaries`] rather than into a list the caller
    /// gets back: a later argument's own call can throw before this call is
    /// ever emitted, and [`Self::landing_block`] has to be able to find it.
    pub(super) fn account_for_arg(
        &mut self,
        v: ValueId,
        ty: Ty,
        ownership: ArgOwnership,
        aliasing: bool,
        cur: BlockId,
    ) {
        if !ty.is_refcounted() {
            return;
        }
        match (ownership, aliasing) {
            (ArgOwnership::Borrowed, true) | (ArgOwnership::Transferred, false) => {}
            (ArgOwnership::Borrowed, false) => self.own_temporary(v),
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
        env: &mut Env,
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
            // Each flattened option is widened into the slot its *declared*
            // type erases to, exactly as a positional argument is. A helper's
            // slot is a whole `Value` and would take either representation
            // (`ArgSig::helper`), so for a `Core` member this is the identity;
            // a compiled MWL function's slot is typed, and `Throwable|null`
            // being `Ty::Tagged` is what makes the exception constructor's
            // `{previous}` bag reach it at all.
            let expected = lower_checked_ty(*option_ty, checked_types);
            if let Some((_, value)) = fields.iter().find(|(field, _)| field == name) {
                let (v, ty) = self.lower_expr(value, Some(expected), env, cur);
                let aliasing = self.aliasing_read(value);
                self.account_for_arg(v, ty, ownership, aliasing, *cur);
                let v = self.coerce(*cur, v, ty, expected, env);
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
            let (v, ty) = self.emit_const_arg(default, env, *cur);
            self.account_for_arg(v, ty, ownership, false, *cur);
            let v = self.coerce(*cur, v, ty, expected, env);
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
    /// `ConstArg::Bytes` is that entry under `Ty::Bytes` — one allocation,
    /// one tag apart — and it is the only way a `bytes` constant enters a
    /// program at all, since the language has no `bytes` literal.
    ///
    /// `ConstArg::Built` is the one entry that emits a **call** rather than a
    /// constant — an instance has no constant form, so what an ADR 0011 class
    /// constant of instance type inlines is the `Core` member that produces
    /// one. It is therefore the one entry that can fail, and it carries ADR
    /// 0002's error edge like any other call.
    pub(super) fn emit_const_arg(
        &mut self,
        default: &mwl_types::ConstArg,
        env: &mut Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        if let mwl_types::ConstArg::Built { symbol, args } = default {
            let mark = self.temporaries_mark();
            // A `Core` member *borrows* its arguments (`InstKind::CoreCall`),
            // and each of these was freshly materialized here, so this frame
            // is the only owner — a `Const::Str` argument leaks without this,
            // once per use site of the constant. Staged the same way a written
            // argument is, so the builder's own failure edge drops them too.
            let values: Vec<ValueId> = args
                .iter()
                .map(|arg| {
                    let (v, ty) = self.emit_const_arg(arg, env, cur);
                    self.account_for_arg(v, ty, ArgOwnership::Borrowed, false, cur);
                    v
                })
                .collect();
            let built = self.emit_fallible(
                cur,
                Ty::Object,
                InstKind::CoreCall {
                    symbol,
                    args: values,
                },
                env,
            );
            self.release_temporaries_since(mark, cur);
            return built;
        }
        let (ty, kind) = match default {
            mwl_types::ConstArg::Null => (Ty::Null, InstKind::ConstNull),
            mwl_types::ConstArg::Bool(b) => (Ty::Bool, InstKind::ConstBool(*b)),
            mwl_types::ConstArg::Int(v) => (Ty::Int, InstKind::ConstInt(*v)),
            mwl_types::ConstArg::Uint(v) => (Ty::Uint, InstKind::ConstUint(*v)),
            mwl_types::ConstArg::Float(v) => (Ty::Float, InstKind::ConstFloat(*v)),
            mwl_types::ConstArg::Str(s) => (Ty::Str, InstKind::ConstStr(s.clone())),
            mwl_types::ConstArg::Bytes(b) => (Ty::Bytes, InstKind::ConstBytes(b.clone())),
            // The same instruction a written `[]` lowers to — an empty
            // `ArrayNew` is already the fixed-shape literal's own zero case
            // (`InstKind::ArrayNew`'s doc comment), so an omitted `array<T>`
            // argument and a written one produce the identical value with the
            // identical single natural owner.
            mwl_types::ConstArg::EmptyArray => (
                Ty::Array,
                InstKind::ArrayNew {
                    entries: Vec::new(),
                },
            ),
            // A bag has no single constant to emit — it is one per option, so
            // its own two call sites expand it before reaching here.
            mwl_types::ConstArg::Options(_) => panic!(
                "mwl-ir: an options bag has no IR constant of its own; \
                 `Lowering::lower_options_arg` expands it per option"
            ),
            // Handled above, before the constant table: it is a call.
            mwl_types::ConstArg::Built { .. } => unreachable!(),
        };
        self.emit(cur, ty, kind)
    }
    /// `$fn(...)` —
    /// [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)'s
    /// closure, called through the variable holding it.
    ///
    /// One [`Helper::CallClosure`], with the closure at `args[0]` and its
    /// arguments after it in written order — which is
    /// `mwl_runtime::call_closure`, the same entry point every `Core` member
    /// taking a `callable` already reaches, so a closure invoked from MWL
    /// takes no second path into a compiled body. It is deliberately **not**
    /// an [`InstKind::Call`]: § 1 gives `callable` no parameter list, so
    /// there is no resolved target to name, no per-argument expected type to
    /// lower against and no arity to check, and the closure object's own
    /// `invoke` answers all three at run time.
    ///
    /// Ownership is [`Self::account_for_arg`]'s borrowed column, the closure
    /// itself included: `call_closure` retains everything it passes and the
    /// callee's exit sweep releases that, so this frame keeps owning exactly
    /// what it lowered. Whatever the expression built is released after the
    /// call, and on the error edge by the landing block — the shape a `Core`
    /// member's arguments already have.
    ///
    /// # Panics
    ///
    /// Panics for a `name:`, `...` or first-class-callable argument list, the
    /// way [`Self::lower_call_args`] does for a resolved call: there is no
    /// signature to map one against here at all, so the checker's own gap
    /// (see the crate docs' known gaps) is not something this can lower
    /// around.
    pub(super) fn lower_closure_call(
        &mut self,
        callee: &Expr,
        args: &CallArgs,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let CallArgs::List(list) = args else {
            panic!(
                "mwl-ir only lowers a plain positional argument list for a call through a \
                 `callable` — got {args:?}; see the crate docs' known gaps"
            );
        };
        // Before the closure, not before the argument list: a freshly built
        // one — `(fn (): int => 7)()` — is this frame's temporary too, and an
        // argument that throws while it is in flight has to drop it.
        let mark = self.temporaries_mark();
        let (closure, closure_ty) = self.lower_expr(callee, None, env, cur);
        let aliasing = self.aliasing_read(callee);
        self.account_for_arg(closure, closure_ty, ArgOwnership::Borrowed, aliasing, *cur);
        let mut values = vec![closure];
        for arg in list {
            assert!(
                arg.name.is_none() && !arg.spread,
                "mwl-ir does not yet lower a named or spread argument to a call through a \
                 `callable`; see the crate docs' known gaps"
            );
            let (v, ty) = self.lower_expr(&arg.value, None, env, cur);
            let aliasing = self.aliasing_read(&arg.value);
            self.account_for_arg(v, ty, ArgOwnership::Borrowed, aliasing, *cur);
            values.push(v);
        }
        // `Ty::Tagged` because `mixed` is the only answer the checker has for
        // a call whose target it cannot name — `mwl_types::expr`'s own
        // `ExprKind::Call` arm.
        let called = self.emit_fallible(
            *cur,
            Ty::Tagged,
            InstKind::HelperCall {
                helper: Helper::CallClosure,
                args: values,
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        called
    }
    /// Records `v` as a reference this frame owns and nothing else can find —
    /// see [`Self::owned_temporaries`], which owns the whole protocol.
    pub(super) fn own_temporary(&mut self, v: ValueId) {
        self.owned_temporaries.push(v);
    }
    /// The height of [`Self::owned_temporaries`] before a call's arguments are
    /// lowered — what [`Self::release_temporaries_since`] releases back down
    /// to once the call has been emitted.
    ///
    /// Taken **before the receiver**, not before the argument list: a
    /// freshly-built receiver is this frame's temporary too, and an argument
    /// that throws while it is in flight has to drop it.
    pub(super) fn temporaries_mark(&self) -> usize {
        self.owned_temporaries.len()
    }
    /// Releases every temporary staged since `mark` into `cur`, in the order
    /// they were staged — the normal edge of the call that borrowed them.
    ///
    /// The error edge is [`Self::landing_block`]'s, and it releases the same
    /// values off the same stack: one set of temporaries, two exits, which is
    /// why nothing here is handed a list to keep in step with.
    pub(super) fn release_temporaries_since(&mut self, mark: usize, cur: BlockId) {
        let temporaries: Vec<ValueId> = self.owned_temporaries.drain(mark..).collect();
        for v in temporaries {
            self.emit_release(cur, v);
        }
    }
    /// Drops every entry staged since `mark` **without** releasing it — the
    /// one expression whose in-flight temporary is its own answer.
    ///
    /// [`Self::lower_interpolation`] accumulates through the stack, so the
    /// last `Concat`'s result is still on it when the expression finishes;
    /// from there it is the caller's value, released wherever that caller
    /// puts it. Everything else releases.
    pub(super) fn forget_temporaries_since(&mut self, mark: usize) {
        self.owned_temporaries.truncate(mark);
    }
    /// Re-points the temporary staged at `slot` — an array under
    /// construction, after a write that consumed its reference and yielded
    /// another name for the same one.
    ///
    /// The one temporary whose *value* changes while later temporaries sit
    /// above it on the stack, so it is addressed by index rather than through
    /// [`Self::forget_temporaries_since`]: truncating down to it would drop an
    /// element's own in-flight temporary without releasing it. Skipping the
    /// re-point would be worse than untidy — a write that separated a shared
    /// array released the reference the old name held, so a landing block
    /// still naming it would release it twice.
    pub(super) fn retarget_temporary(&mut self, slot: usize, v: ValueId) {
        self.owned_temporaries[slot] = v;
    }
    /// Removes the temporary staged at `slot` **without** releasing it, the
    /// entries above it keeping their order — [`Self::forget_temporaries_since`]
    /// for one entry that is no longer the top of the stack, which is what a
    /// finished array literal's own result is.
    pub(super) fn forget_temporary(&mut self, slot: usize) {
        self.owned_temporaries.remove(slot);
    }
    /// The value staged for `span`, if any — [`Self::staged_targets`] searched
    /// innermost first, which is the whole read side of that table.
    pub(super) fn staged(&self, span: Span) -> Option<(ValueId, Ty)> {
        self.staged_targets
            .iter()
            .rev()
            .find(|(s, ..)| *s == span)
            .map(|&(_, v, ty)| (v, ty))
    }
    /// Records `(v, ty)` as the already-lowered value of the expression at
    /// `span` — see [`Self::staged_targets`], which owns the protocol.
    pub(super) fn stage(&mut self, span: Span, v: ValueId, ty: Ty) {
        self.staged_targets.push((span, v, ty));
    }
    /// The height of [`Self::staged_targets`] — the mark
    /// [`Self::unstage_to`] winds back to.
    pub(super) fn staged_mark(&self) -> usize {
        self.staged_targets.len()
    }
    /// Drops every staging recorded since `mark`. Never releases anything: a
    /// staged entry is a borrow, and whatever owns the value it names —
    /// [`Self::owned_temporaries`], or a durable slot — is what releases it.
    pub(super) fn unstage_to(&mut self, mark: usize) {
        self.staged_targets.truncate(mark);
    }
    /// A span no expression in this file can carry: empty, one past the last
    /// byte a `u32` offset can name.
    ///
    /// The one thing staged under it is an increment's implicit `1`
    /// ([`Self::lower_read_modify_write`]), whose span is never *read* —
    /// [`Self::lower_expr`] answers from [`Self::staged_targets`] before it
    /// looks at the expression's kind, so the `ExprKind::Int` wrapped around
    /// it never cooks any digits.
    pub(super) fn synthetic_span(&self) -> Span {
        Span::at(self.src.id(), u32::MAX)
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
        env: &mut Env,
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
