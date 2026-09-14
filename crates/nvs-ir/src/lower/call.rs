//! Call lowering: argument ownership, `rule:core-api/shape-rules` R2's options bag flattened at the site, an `inout $x` argument staged and written back, `$fn(...)` through the one helper a `Core` member's callback already takes, and `rule:classes/delegation-by-field`'s `by $field` forward, which is a whole synthesized function rather than a lowered call.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. The methods
//! are `pub(crate)` so they reach across these modules and no further, which
//! is the reach a single-file `lower` would give them.

use super::*;
use crate::lower::expr::ReceiverProof;

impl<'a> Lowering<'a> {
    /// Lowers a resolved call's/`new`'s argument list against `param_tys` —
    /// the already-resolved parameter types from
    /// `nvs_types::expr_table::ResolvedCall` — placing each written argument at
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
    /// An `inout $x` parameter's argument is **staged** instead (see [`Ty::Ref`]):
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
    /// `nvs_types::check_program` to have refused before it ever got here: an
    /// argument that reached no parameter at all, more arguments than `sig`
    /// has parameters, a parameter no argument filled and that has no default,
    /// or a by-reference argument that is neither a bare local nor a
    /// compile-time-known property — that last one a [`guarded_by!`] naming
    /// `E0439`, which `nvs_types::expr::args::check_inout_arg` raises at the
    /// call site.
    ///
    /// The first-class callable sentinel panics too, and its roster is closed:
    /// `rule:types/callable-is-a-closure`'s member spellings record
    /// `nvs_types::expr_table::ExprInfo::CallableRef` rather than `Call`, so
    /// [`super::expr`]'s call arms never dispatch here for one, and the shapes
    /// that name no member are diagnostics where they are written
    /// (`E0732` for a `mixed` receiver, `E0740` for `new C(...)`).
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
    pub(crate) fn lower_call_args(
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
                "nvs-ir: a resolved call/`new` reached argument lowering with \
                 {args:?} where a written argument list belongs — this crate trusts \
                 nvs_types::check_program to have settled every other shape. The \
                 roster is closed: `rule:types/callable-is-a-closure`'s `Class::method(...)` and \
                 `$obj->method(...)` record `ExprInfo::CallableRef` and never reach \
                 this function, `$m->method(...)` on a `mixed` receiver is `E0732`, \
                 and `new C(...)` is `E0740`"
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
            "nvs-ir: a resolved call records one argument slot per written argument — \
             nvs_types is trusted to have mapped every one of them"
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
                    "nvs-ir: a written argument reached no parameter — this crate trusts \
                     nvs_types::check_program already reported it"
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
            "nvs-ir: a resolved call passes more arguments than its signature has parameters — \
             this crate trusts nvs_types::check_program already enforced arity"
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
        arg: &nvs_syntax::ast::Arg,
        index: usize,
        sig: &ArgSig,
        checked_types: &TypeInterner,
        ownership: ArgOwnership,
        env: &mut Env,
        cur: &mut BlockId,
        out: &mut LoweredArgs,
    ) {
        // `rule:core-api/shape-rules` R2's options bag, and `rule:core-api/shape-flattens-at-the-abi`'s shape parameter with
        // it: not one argument but one *per slot of the merged list*, so
        // neither ever reaches `erase_checked_ty` — a shape has no IR type at
        // all. See [`Self::lower_options_arg`].
        // The merged list and never the arms: which arm the literal selected is
        // a checking question `nvs-types` has already settled, and § 3's ABI is
        // one argument per merged slot whichever arm that was.
        if let CheckedTy::CoreShape(shape) = checked_types.get(sig.param_tys[index]) {
            let options = shape.fields.clone();
            let defaults = shape_fills(&sig.defaults, index);
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
        // tagged `Value` that `nvs-codegen` writes from the *argument's* own
        // representation. See `ArgSig::helper`, which owns why the same
        // declaration on a compiled Novis function is not lowerable.
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
        if sig.is_inout(index) {
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
                "nvs-ir: parameter {index} was filled by no argument at a call site and has no \
                 default — this crate trusts nvs_types::check_program already enforced arity"
            )
        });
        // A bag omitted whole is every one of its options taking its own
        // default, in the same declared order a written one flattens in —
        // which is exactly what `lower_options_arg` does with no written
        // literal, so it is reached rather than repeated here. Going through
        // it is also what gives an omitted option the same widening into its
        // declared slot that a written one gets.
        if let nvs_types::ConstArg::Options(options) = default {
            let CheckedTy::CoreShape(shape) = checked_types.get(sig.param_tys[index]) else {
                panic!(
                    "nvs-ir: parameter {index} carries an options-bag default but its declared \
                     type is not an options bag — nvs_types is trusted to record the two together"
                );
            };
            self.lower_options_arg(
                None,
                &shape.fields,
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

    /// `rule:core-api/shape-rules`'s variadic tail as the single ABI argument it becomes: every
    /// argument from parameter `fixed` onward collected into one fresh
    /// `array<T>`, keyed `"0"`, `"1"`, … in written order.
    ///
    /// The array is what `nvs_stdlib::registry::CoreTy::Variadic` promises the
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
    /// there is no lowering-time key to give them. `nvs_runtime::nvs_array_spread`
    /// owns which of the subject's keys survive (`rule:types/arrays`) and it is PHP's
    /// unpacking rule as well as PHP's array-literal one — an integer-looking
    /// key is renumbered under the tail's own append counter, so
    /// `f(...$xs, ...$ys)` concatenates, and a string key is preserved, which
    /// is what a `string`-keyed unpack lands in PHP's variadic parameter too.
    ///
    /// The written-out entries keep their single `ArrayNew` because they are
    /// always a *prefix*: a positional argument cannot follow a `...`, and a
    /// `name:` never reaches the variadic parameter at all
    /// (`nvs_types::expr::args::map_arguments` rules 1 and 3). So a call with
    /// no spread emits that one instruction and nothing beside it.
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
        rest: &[&nvs_syntax::ast::Arg],
        fixed: usize,
        sig: &ArgSig,
        ownership: ArgOwnership,
        env: &mut Env,
        cur: &mut BlockId,
        out: &mut LoweredArgs,
    ) {
        let expected = sig.expectation(fixed, self.checked_types);
        let array = self.lower_args_as_array(rest, expected, env, cur);
        self.account_for_arg(array, Ty::Array, ownership, false, *cur);
        out.values.push(array);
    }

    /// A run of written arguments, built into one `array<T>` keyed `"0"`,
    /// `"1"`, … in written order, with each `...` argument's own entries
    /// flattened in at the position it was written.
    ///
    /// The shape both variadic call sites share: a resolved call's variadic
    /// tail ([`Self::lower_variadic_tail`], which owns what the array *means*
    /// there) and a call through a `callable` that wrote a `...`
    /// ([`Self::lower_closure_call`], where it is the whole argument list).
    /// `expected` is the element type to widen each written-out entry into, and
    /// is `None` at the second site: `rule:types/closure-literal` gives `callable` no parameter
    /// list, so there is nothing to widen towards.
    ///
    /// The returned array is a **fresh producer** and is left accounted to
    /// nobody: each caller decides whether it is an argument
    /// ([`Self::account_for_arg`]) or its own temporary, and one of the two has
    /// to happen or the allocation leaks.
    fn lower_args_as_array(
        &mut self,
        rest: &[&nvs_syntax::ast::Arg],
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        // Every argument written out one by one is a prefix of the tail: a
        // positional argument cannot follow a `...` (`nvs_types`' E0488) and a
        // `name:` never reaches the variadic parameter at all, so the first
        // spread is where the lowering-time keys stop.
        let spread_from = rest.iter().position(|arg| arg.spread).unwrap_or(rest.len());
        let mut entries = Vec::with_capacity(spread_from);
        for (index, arg) in rest[..spread_from].iter().enumerate() {
            let (v, ty) = self.lower_expr(&arg.value, expected, env, cur);
            if ty.is_refcounted() && self.aliasing_read(&arg.value) {
                self.emit_retain(*cur, v);
            }
            // The element is stored as a whole `nvs_runtime::Value`, so it is
            // widened into the parameter's own representation here exactly as
            // a fixed argument is — `Self::coerce` is ownership-transparent,
            // so the retain above still pays for what lands in the array.
            let v = match expected {
                Some(expected) => self.coerce(*cur, v, ty, expected, env),
                None => v,
            };
            entries.push((index.to_string(), v));
        }
        let (mut array, _) = self.emit(*cur, Ty::Array, InstKind::ArrayNew { entries });
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
                    "nvs-ir: a positional argument follows a `...` in a variadic tail — \
                     this crate trusts nvs_types::check_program already reported it as E0488"
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
            // The finished array is the caller's from here, exactly as the
            // no-spread one beside it is.
            self.forget_temporary(slot);
        }
        array
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
    ///   either way, from the moment the call is emitted; a copy of storage
    ///   someone else owns needs a second reference first, a freshly built
    ///   value does not. Until that instruction exists it is still this
    ///   frame's, so it is staged too, under
    ///   [`TemporaryKind::Transferred`].
    ///
    /// Shared by the written arguments, the materialized defaults and each
    /// flattened option, so a bag's options are accounted exactly as the
    /// arguments beside them are.
    ///
    /// Everything it stages goes on [`Self::owned_temporaries`] rather than
    /// into a list the caller gets back: a later argument's own call can throw
    /// before this call is ever emitted, and [`Self::landing_block`] has to be
    /// able to find it.
    pub(crate) fn account_for_arg(
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
            (ArgOwnership::Borrowed, true) => {}
            (ArgOwnership::Borrowed, false) => self.own_temporary(v),
            (ArgOwnership::Transferred, aliasing) => {
                if aliasing {
                    self.emit_retain(cur, v);
                }
                self.own_transferred_temporary(v);
            }
        }
    }
    /// Flattens one `rule:core-api/shape-rules` R2 options bag into `out`: one value per option
    /// `options` declares, in that declared order — the written field's value
    /// where the call site gave one, the option's own default where it did
    /// not.
    ///
    /// `written` is the object literal the call site passed, or `None` for a
    /// bag omitted entirely. This is why an options argument has to be a
    /// literal at the call site (`nvs_types` reports `E_OPTIONS_NOT_A_LITERAL`
    /// for anything else): the flattening is per-option and static, so there
    /// is nothing to read a variable's fields out of. Nothing below this line
    /// — not `nvs-codegen`, not the helper convention a `Core` member is
    /// reached through — learns that bags exist, exactly as nothing learns
    /// that defaults do ([`Self::emit_const_arg`]).
    ///
    /// # Panics
    ///
    /// Panics if `written` is not an object literal, or if an option has
    /// neither a written field nor a default: both are shapes
    /// `nvs_types::check_program` and `nvs_types::core_lib` are trusted to
    /// have made impossible.
    #[expect(
        clippy::too_many_arguments,
        reason = "the call-lowering context `Self::lower_call_args` already \
                  threads — expectation, ownership, environment, block — plus \
                  the bag's own two halves; bundling them into a struct would \
                  be one type used at one call site"
    )]
    pub(crate) fn lower_options_arg(
        &mut self,
        written: Option<&Expr>,
        options: &[nvs_types::CoreShapeField],
        defaults: &[(String, nvs_types::ConstArg)],
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
                    "nvs-ir: an options argument lowered from {other:?} rather than an object \
                     literal — nvs_types::check_program is trusted to have reported \
                     E_OPTIONS_NOT_A_LITERAL for anything else"
                ),
            },
            None => Vec::new(),
        };
        for option in options {
            let name = &option.name;
            // Each flattened option is widened into the slot its *declared*
            // type erases to, exactly as a positional argument is. A helper's
            // slot is a whole `Value` and would take either representation
            // (`ArgSig::helper`), so for a `Core` member this is the identity;
            // a compiled Novis function's slot is typed, and `Throwable|null`
            // being `Ty::Tagged` is what makes the exception constructor's
            // `{previous}` bag reach it at all.
            let expected = erase_checked_ty(option.ty, checked_types);
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
                        "nvs-ir: the option `{name}` was omitted at a call site and has no \
                         default — nvs_types::core_lib is trusted to record one per declared \
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
    /// This is the whole of Novis's default-argument mechanism at the IR level,
    /// which is the point of evaluating a default at signature collection
    /// rather than in the callee (`nvs_types::defaults` owns why): every
    /// compiled function keeps exactly one arity, so nothing below this line —
    /// not the `rule:errors/propagation` call ABI, not `nvs-codegen`, not the helper
    /// convention a `Core` member is reached through — learns that defaults
    /// exist at all.
    ///
    /// A `ConstArg::Str` allocates a fresh string per evaluation, exactly as a
    /// written string literal does today (`nvs-codegen`'s known gap 4); it is
    /// the same `InstKind::ConstStr` and closing that gap closes both.
    /// `ConstArg::Bytes` is that entry under `Ty::Bytes` — one allocation,
    /// one tag apart — and it is the only way a `bytes` constant enters a
    /// program at all, since the language has no `bytes` literal.
    ///
    /// `ConstArg::Built` is the one entry that emits a **call** rather than a
    /// constant — an instance has no constant form, so what an `rule:classes/no-free-functions-or-constants` class
    /// constant of instance type inlines is the `Core` member that produces
    /// one. It is therefore the one entry that can fail, and it carries ADR
    /// 0002's error edge like any other call.
    pub(crate) fn emit_const_arg(
        &mut self,
        default: &nvs_types::ConstArg,
        env: &mut Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        if let nvs_types::ConstArg::Built { symbol, args } = default {
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
            nvs_types::ConstArg::Null => (Ty::Null, InstKind::ConstNull),
            // Already tagged, and that is the point: an omitted *nullable*
            // option's slot has to hold something a written `null` cannot,
            // and `Tag::Unset` is it (`InstKind::ConstUnset`). Emitting it as
            // `Ty::Tagged` also makes the widening below the identity, since
            // a nullable option's declared slot erases to `Ty::Tagged` too.
            nvs_types::ConstArg::NeverWritten => (Ty::Tagged, InstKind::ConstUnset),
            nvs_types::ConstArg::Bool(b) => (Ty::Bool, InstKind::ConstBool(*b)),
            nvs_types::ConstArg::Int(v) => (Ty::Int, InstKind::ConstInt(*v)),
            nvs_types::ConstArg::Uint(v) => (Ty::Uint, InstKind::ConstUint(*v)),
            nvs_types::ConstArg::Float(v) => (Ty::Float, InstKind::ConstFloat(*v)),
            nvs_types::ConstArg::Str(s) => (Ty::Str, InstKind::ConstStr(s.clone())),
            nvs_types::ConstArg::Bytes(b) => (Ty::Bytes, InstKind::ConstBytes(b.clone())),
            // The same instruction a written `[]` lowers to — an empty
            // `ArrayNew` is already the fixed-shape literal's own zero case
            // (`InstKind::ArrayNew`'s doc comment), so an omitted `array<T>`
            // argument and a written one produce the identical value with the
            // identical single natural owner.
            nvs_types::ConstArg::EmptyArray => (
                Ty::Array,
                InstKind::ArrayNew {
                    entries: Vec::new(),
                },
            ),
            // A bag has no single constant to emit — it is one per option, so
            // its own two call sites expand it before reaching here.
            nvs_types::ConstArg::Options(_) => panic!(
                "nvs-ir: an options bag has no IR constant of its own; \
                 `Lowering::lower_options_arg` expands it per option"
            ),
            // The same, one level further: a shape parameter is written at
            // every call site (`ConstArg::RequiredShape`), so reaching here
            // means an argument the arity check should already have demanded
            // was treated as omitted.
            nvs_types::ConstArg::RequiredShape(_) => panic!(
                "nvs-ir: a shape parameter has no IR constant of its own and is never omitted; \
                 `Lowering::lower_options_arg` expands its written literal per slot"
            ),
            // Handled above, before the constant table: it is a call.
            nvs_types::ConstArg::Built { .. } => unreachable!(),
            // `rule:attributes/retrieval-folds-while-checking`'s folded retrieval. Both build a value out of
            // several, so neither is one instruction and both return early.
            nvs_types::ConstArg::Shape(fields) => {
                return self.emit_const_shape(fields, env, cur);
            }
            nvs_types::ConstArg::Array(entries) => {
                let mut values = Vec::with_capacity(entries.len());
                for (key, entry) in entries {
                    let (value, _) = self.emit_const_arg(entry, env, cur);
                    values.push((key.clone(), value));
                }
                (Ty::Array, InstKind::ArrayNew { entries: values })
            }
        };
        self.emit(cur, ty, kind)
    }

    /// An `rule:types/object-literal`
    /// shape value, materialized from a constant rather than from a written
    /// literal — `rule:attributes/retrieval-folds-while-checking`'s fold is the one producer.
    ///
    /// Deliberately the same synthesized class a written literal of the same
    /// field set gets ([`super::shape_class_label`], keyed on the sorted
    /// names), so a retrieved payload and a hand-written `{path: "/x"}` are
    /// one class with one layout: `nvs_types` matched the payload against the
    /// caller's shape structurally, and two classes for one shape would make
    /// the field offsets they agree on a coincidence.
    ///
    /// No retain on any field: every value here is a constant this call just
    /// emitted, so each is a fresh producer whose one reference the slot takes
    /// — the written literal's [`Self::aliasing_read`] question has no
    /// expression to ask about and no borrowed operand to ask it of.
    fn emit_const_shape(
        &mut self,
        fields: &[(String, nvs_types::ConstArg)],
        env: &mut Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        let mut sorted: Vec<String> = fields.iter().map(|(name, _)| name.clone()).collect();
        sorted.sort();
        sorted.dedup();
        let class = super::shape_class_label(&sorted);
        let (obj, _) = self.emit_fallible(
            cur,
            Ty::Object,
            InstKind::New {
                class: class.clone(),
                ctor: None,
                args: Vec::new(),
            },
            env,
        );
        let mut reprs: FxHashMap<&str, Ty> = FxHashMap::default();
        for (name, value) in fields {
            let (value, ty) = self.emit_const_arg(value, env, cur);
            reprs.insert(name.as_str(), ty);
            self.emit_field_set(cur, obj, class.clone(), name.clone(), value);
        }
        let reprs = sorted
            .iter()
            .map(|name| reprs[name.as_str()])
            .collect::<Vec<_>>();
        self.record_shape_class(class, sorted, reprs);
        (obj, Ty::Object)
    }
    /// `$fn(...)` —
    /// `rule:types/closure-literal`'s
    /// closure, called through the variable holding it.
    ///
    /// One [`Helper::CallClosure`], with the closure at `args[0]` and its
    /// arguments after it in written order — which is
    /// `nvs_runtime::call_closure`, the same entry point every `Core` member
    /// taking a `callable` already reaches, so a closure invoked from Novis
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
    /// # A `...` argument
    ///
    /// A spread makes the argument *count* the subject's own run-time length,
    /// and [`Helper::CallClosure`]'s count is a literal in the emitted call —
    /// `nvs-codegen` writes it beside the argument slot. So a call site that
    /// wrote one goes through [`Helper::CallClosureArray`] instead, with the
    /// whole list built into one array by [`Self::lower_args_as_array`], which
    /// is the same array a resolved call's variadic tail already is. That
    /// array is one more borrowed argument, so this frame releases it on both
    /// edges exactly as it releases everything else it built here.
    ///
    /// # `$f(...)`
    ///
    /// The first-class-callable sentinel makes no call at all.
    /// `rule:types/callable-is-a-closure` gives `callable` exactly one
    /// inhabitant, a closure, so `$f(...)` already names the value a reference
    /// to `$f` would have to produce and the answer is that closure itself —
    /// which is also PHP's, pinned by
    /// `tests/differential/lang/a-first-class-callable-of-a-closure-matches-phps.nvst`.
    /// It is handed on as a fresh owner: one retain where the callee borrowed a
    /// slot this frame does not own, and none where the callee already produced
    /// one, which is [`Self::aliasing_read`]'s judgment everywhere else in this
    /// file.
    ///
    /// # Panics
    ///
    /// Panics for a `name:` argument — `rule:types/closure-literal` gives
    /// `callable` no parameter list, so there is no parameter for a name to fill
    /// and `nvs_types` refuses one where it is written (`E0712`).
    pub(crate) fn lower_closure_call(
        &mut self,
        call: &Expr,
        callee: &Expr,
        args: &CallArgs,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let list = match args {
            CallArgs::List(list) => list,
            CallArgs::FirstClassCallable => {
                let (closure, closure_ty) = self.lower_expr(callee, None, env, cur);
                if closure_ty.is_refcounted() && self.aliasing_read(callee) {
                    self.emit_retain(*cur, closure);
                }
                return (closure, closure_ty);
            }
        };
        // `rule:types/callable-signature`: the checker records this on the
        // call's own span, and only where the callee's type named its
        // parameters — so its *absence* is what keeps every other site on the
        // dynamic path. Read out whole here, because the borrow of the check
        // run's table ends where this statement does and the frame is mutated
        // below.
        let proven = match self.exprs.lookup(call.span) {
            Some(ExprInfo::CallThroughSignature { params, ret }) => Some((params.clone(), *ret)),
            _ => None,
        };
        // Before the closure, not before the argument list: a freshly built
        // one — `(fn (): int => 7)()` — is this frame's temporary too, and an
        // argument that throws while it is in flight has to drop it.
        let mark = self.temporaries_mark();
        let (closure, closure_ty) = self.lower_expr(callee, None, env, cur);
        let aliasing = self.aliasing_read(callee);
        self.account_for_arg(closure, closure_ty, ArgOwnership::Borrowed, aliasing, *cur);
        let mut values = vec![closure];
        assert!(
            list.iter().all(|arg| arg.name.is_none()),
            "nvs-ir: a `name:` argument reached a call through a `callable` — this crate trusts \
             nvs_types::check_program already reported it as E0712"
        );
        // A `...` makes the argument *count* a run-time fact, which the one
        // helper whose count is a literal in the emitted call cannot carry. So
        // the whole list becomes one array instead, and the other helper reads
        // its length — see `Helper::CallClosureArray`.
        let helper = match list.iter().any(|arg| arg.spread) {
            true => {
                let rest: Vec<&nvs_syntax::ast::Arg> = list.iter().collect();
                let array = self.lower_args_as_array(&rest, None, env, cur);
                self.account_for_arg(array, Ty::Array, ArgOwnership::Borrowed, false, *cur);
                values.push(array);
                Helper::CallClosureArray
            }
            false => match &proven {
                // The proof spent on the arguments. Each one reaches the callee
                // in the representation its *declared* parameter names, because
                // `nvs_runtime::closure`'s `check_param_tags` — the one thing
                // this helper does not run — is also what widened an `int` into
                // a `float` parameter, and nothing else would.
                Some((params, _)) => {
                    assert_eq!(
                        list.len(),
                        params.len(),
                        "nvs-ir: a proven call through a `callable` signature was recorded for a \
                         list of a different length — nvs_types::expr::calls records \
                         `ExprInfo::CallThroughSignature` only where the counts agree"
                    );
                    for (arg, param) in list.iter().zip(params) {
                        let (v, ty) = self.lower_expr(&arg.value, None, env, cur);
                        let want = erase_checked_ty(*param, self.checked_types);
                        let v = self.coerce(*cur, v, ty, want, env);
                        let aliasing = self.aliasing_read(&arg.value);
                        self.account_for_arg(v, want, ArgOwnership::Borrowed, aliasing, *cur);
                        values.push(v);
                    }
                    Helper::CallClosureProven
                }
                None => {
                    for arg in list {
                        let (v, ty) = self.lower_expr(&arg.value, None, env, cur);
                        let aliasing = self.aliasing_read(&arg.value);
                        self.account_for_arg(v, ty, ArgOwnership::Borrowed, aliasing, *cur);
                        values.push(v);
                    }
                    Helper::CallClosure
                }
            },
        };
        // `Ty::Tagged` is what the callee writes into the out slot whatever it
        // declares, and `mixed` is the only answer the checker has for a call
        // whose target it cannot name — `nvs_types::expr`'s own `ExprKind::Call`
        // arm.
        let (value, ty) = self.emit_fallible(
            *cur,
            Ty::Tagged,
            InstKind::HelperCall {
                helper,
                args: values,
            },
            env,
        );
        // The proof spent on the result: a proven site's callee declared what
        // it answers, so the slot is read at that representation here rather
        // than left for every consumer to narrow one `mixed` at a time.
        // `Ty::Void` names no register at all and `Ty::Tagged` is already what
        // the slot holds, so neither of those moves.
        let narrowed = proven
            .map(|(_, ret)| erase_checked_ty(ret, self.checked_types))
            .filter(|repr| !matches!(repr, Ty::Tagged | Ty::Void));
        let called = match narrowed {
            Some(repr) => (self.coerce(*cur, value, ty, repr, env), repr),
            None => (value, ty),
        };
        self.release_temporaries_since(mark, *cur);
        called
    }
    /// `$m->method(...)` on a **`mixed`** receiver —
    /// `rule:types/erased-member-access`'s
    /// deferral applied to a call, which
    /// [`crate::ir::Helper::CallErasedMethod`] owns the convention for.
    ///
    /// One helper, with the receiver at `args[0]` still **tagged** — nothing
    /// proved it holds an object, so the tag test is the runtime's — the
    /// member name at `args[1]` as an immortal `string` constant, and every
    /// argument in written order packed into one array at `args[2]`. It is
    /// deliberately not an [`InstKind::Call`] nor an [`InstKind::CallVirtual`]:
    /// there is no resolved target to name, no signature to lower each
    /// argument against and no arity to check against, and the receiver's own
    /// class descriptor answers all three when the call runs.
    ///
    /// The arguments are packed rather than passed one per slot for
    /// [`Self::lower_closure_call`]'s `...` reason, and here it holds for
    /// *every* site: a helper's argument count is a literal `nvs-codegen`
    /// writes beside the slot, while what this call site wrote is judged
    /// against a callee chosen when it runs. So one shape carries both the
    /// spread and the plain list, and the array is the same one a variadic
    /// tail already is.
    ///
    /// Ownership is [`Self::lower_closure_call`]'s throughout — the borrowed
    /// column for the receiver and for the array, released on both edges by
    /// this frame — and the name needs no accounting at all, an
    /// `InstKind::ConstStr` being an immortal address in the unit's data
    /// section rather than an allocation.
    ///
    /// # Panics
    ///
    /// Panics naming any shape `nvs_types` is trusted to have settled first: a
    /// `name:` argument (`E0712`) and an `inout` one (`E0714`), neither of
    /// which the deferral can express, and the first-class-callable sentinel,
    /// which is `rule:types/callable-is-a-closure`'s `$m->method(...)` and names a closure *value*
    /// rather than making a call — refused where it is written (`E0732`),
    /// because a closure carries its callee and the deferral has none to
    /// carry. No program constructs any of the three, so each is an engine
    /// invariant rather than a shape the language still refuses.
    pub(crate) fn lower_erased_method_call(
        &mut self,
        object: &Expr,
        nullsafe: bool,
        name: &str,
        args: &CallArgs,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let CallArgs::List(list) = args else {
            panic!(
                "nvs-ir: a call through a `mixed` receiver reached lowering with \
                 {args:?} where a written argument list belongs — `ExprInfo::ErasedCall` \
                 is recorded only for the non-sentinel branch, `rule:types/callable-is-a-closure`'s \
                 `$m->method(...)` being refused where it is written (`E0732`), so no \
                 program constructs this"
            );
        };
        assert!(
            list.iter().all(|arg| arg.name.is_none() && !arg.inout),
            "nvs-ir: a `name:` or `inout` argument reached a call through a `mixed` receiver — \
             this crate trusts nvs_types::check_program already reported it as E0712/E0714"
        );
        // Before the receiver, not before the argument list: a freshly built
        // receiver is this frame's temporary too, and an argument that throws
        // while it is in flight has to drop it.
        let mark = self.temporaries_mark();
        // `ReceiverProof::Erased`: no `Untag` is emitted, so the whole tagged
        // value travels to the helper and the tag test is made there.
        let (object_v, receiver_ty, guard) =
            self.open_nullsafe(object, nullsafe, ReceiverProof::Erased, env, cur);
        let aliasing = self.aliasing_read(object);
        self.account_for_arg(
            object_v,
            receiver_ty,
            ArgOwnership::Borrowed,
            aliasing,
            *cur,
        );
        let (member, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(name.to_owned()));
        let rest: Vec<&nvs_syntax::ast::Arg> = list.iter().collect();
        let array = self.lower_args_as_array(&rest, None, env, cur);
        self.account_for_arg(array, Ty::Array, ArgOwnership::Borrowed, false, *cur);
        // `Ty::Tagged` because `mixed` is the only answer the checker has for
        // a call whose target it cannot name — `nvs_types::expr::calls`'
        // `ExprInfo::ErasedCall` arm.
        let (v, ty) = self.emit_fallible(
            *cur,
            Ty::Tagged,
            InstKind::HelperCall {
                helper: Helper::CallErasedMethod,
                args: vec![object_v, member, array],
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        self.close_nullsafe(guard, v, ty, env, cur)
    }
    /// Records `v` as a reference this frame owns and nothing else can find —
    /// see [`Self::owned_temporaries`], which owns the whole protocol.
    pub(crate) fn own_temporary(&mut self, v: ValueId) {
        self.owned_temporaries.push((v, TemporaryKind::Owned));
    }
    /// Records `v` as a reference this frame holds only until the call it was
    /// staged for is emitted — see [`TemporaryKind::Transferred`], and
    /// [`Self::forget_transferred_since`], which is its normal edge.
    pub(crate) fn own_transferred_temporary(&mut self, v: ValueId) {
        self.owned_temporaries.push((v, TemporaryKind::Transferred));
    }
    /// The height of [`Self::owned_temporaries`] before a call's arguments are
    /// lowered — what [`Self::release_temporaries_since`] releases back down
    /// to once the call has been emitted.
    ///
    /// Taken **before the receiver**, not before the argument list: a
    /// freshly-built receiver is this frame's temporary too, and an argument
    /// that throws while it is in flight has to drop it.
    pub(crate) fn temporaries_mark(&self) -> usize {
        self.owned_temporaries.len()
    }
    /// Releases every temporary staged since `mark` into `cur`, in the order
    /// they were staged — the normal edge of the call that borrowed them.
    ///
    /// The error edge is [`Self::landing_block`]'s, and it releases the same
    /// values off the same stack: one set of temporaries, two exits, which is
    /// why nothing here is handed a list to keep in step with.
    pub(crate) fn release_temporaries_since(&mut self, mark: usize, cur: BlockId) {
        let temporaries: Vec<(ValueId, TemporaryKind)> =
            self.owned_temporaries.drain(mark..).collect();
        for (v, kind) in temporaries {
            // A transferred entry reaching here at all means its own call site
            // did not forget it, and this edge is a normal one — so the call
            // was emitted and its callee already owns the reference. Dropping
            // it is the backstop; releasing it would be the double drop.
            if kind == TemporaryKind::Owned {
                self.emit_release(cur, v);
            }
        }
    }
    /// Drops every [`TemporaryKind::Transferred`] entry staged since `mark`
    /// without releasing it, the owned ones staged beside them keeping their
    /// place — the normal edge of a call whose arguments the callee now owns.
    ///
    /// Run immediately *before* the call is emitted, not after it and not at
    /// the end of the enclosing statement. The callee's exit sweep releases a
    /// transferred parameter on the callee's *throwing* edge as much as on its
    /// normal one, so the call's own fault edge — built by
    /// [`Self::emit_fallible`], and therefore from whatever is on the stack at
    /// that moment — must already be past them. Everything fallible that
    /// *evaluated* them is behind that point, which is what leaves the leaking
    /// window covered and this one empty.
    pub(crate) fn forget_transferred_since(&mut self, mark: usize) {
        let tail = self.owned_temporaries.split_off(mark);
        self.owned_temporaries.extend(
            tail.into_iter()
                .filter(|&(_, kind)| kind != TemporaryKind::Transferred),
        );
    }
    /// Drops every entry staged since `mark` **without** releasing it — the
    /// one expression whose in-flight temporary is its own answer.
    ///
    /// [`Self::lower_interpolation`] accumulates through the stack, so the
    /// last `Concat`'s result is still on it when the expression finishes;
    /// from there it is the caller's value, released wherever that caller
    /// puts it. Everything else releases.
    pub(crate) fn forget_temporaries_since(&mut self, mark: usize) {
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
    pub(crate) fn retarget_temporary(&mut self, slot: usize, v: ValueId) {
        self.owned_temporaries[slot].0 = v;
    }
    /// Removes the temporary staged at `slot` **without** releasing it, the
    /// entries above it keeping their order — [`Self::forget_temporaries_since`]
    /// for one entry that is no longer the top of the stack, which is what a
    /// finished array literal's own result is.
    pub(crate) fn forget_temporary(&mut self, slot: usize) {
        self.owned_temporaries.remove(slot);
    }
    /// The value staged for `span`, if any — [`Self::staged_targets`] searched
    /// innermost first, which is the whole read side of that table.
    pub(crate) fn staged(&self, span: Span) -> Option<(ValueId, Ty)> {
        self.staged_targets
            .iter()
            .rev()
            .find(|(s, ..)| *s == span)
            .map(|&(_, v, ty)| (v, ty))
    }
    /// Records `(v, ty)` as the already-lowered value of the expression at
    /// `span` — see [`Self::staged_targets`], which owns the protocol.
    pub(crate) fn stage(&mut self, span: Span, v: ValueId, ty: Ty) {
        self.staged_targets.push((span, v, ty));
    }
    /// The height of [`Self::staged_targets`] — the mark
    /// [`Self::unstage_to`] winds back to.
    pub(crate) fn staged_mark(&self) -> usize {
        self.staged_targets.len()
    }
    /// Drops every staging recorded since `mark`. Never releases anything: a
    /// staged entry is a borrow, and whatever owns the value it names —
    /// [`Self::owned_temporaries`], or a durable slot — is what releases it.
    pub(crate) fn unstage_to(&mut self, mark: usize) {
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
    pub(crate) fn synthetic_span(&self) -> Span {
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
    pub(crate) fn stage_ref_arg(
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
                        "nvs-ir: the by-reference argument at {:?} is a property with no \
                         resolved declaring class recorded in the typed-expression table — \
                         either it wasn't checked with the same table, or its receiver erased \
                         to a shape/plain `object` (`rule:types/erased-member-access`); nvs_types' \
                         `check_inout_arg` is expected to have refused both",
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
            other => guarded_by!(
                code::E_INOUT_ARG_NOT_A_PLACE,
                "nvs-ir reached by-reference argument staging from {other:?}. The callee writes \
                 back through the reference, so the argument has to name storage that outlives \
                 the call, and `nvs_types::expr::args::check_inout_arg` refuses every other \
                 operand at the call site — an array element among them, which \
                 `rule:types/arrays`'s copy-on-write separation leaves no stable address"
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

/// One `rule:classes/delegation-by-field` `implements I by $field;` forward, built as a whole
/// [`Function`] — the shape `nvs_types::expr_table::Delegation` decided.
///
/// It is a synthesized **method** rather than a rewrite of the call site, and
/// that is the decision in it: a receiver typed as the interface dispatches on
/// its runtime class, so a rewrite would forward `$post->touch()` and leave
/// `$timestamped->touch()` reaching nothing. `lower_program` adds the same
/// name to the class's method table, which is what that dispatch reads.
///
/// The body is § 4's own one-liner, `return $this->field->method(...);`: read
/// the field, retain it (a field read borrows, and
/// [`InstKind::CallVirtual`] transfers its receiver), then the call, then the
/// return. The call is late-bound with **no fallback** — the field's declared
/// type is the interface, whose member has no body to name — so the target is
/// whatever the field's runtime class answers, which is the whole point of
/// delegating to it.
///
/// Ownership is the ordinary compiled-method convention: every parameter is
/// transferred in, each argument is transferred straight on to the callee, and
/// `$this` is the one reference this frame owes a release for — on the normal
/// exit and again in the landing block, since a throwing callee already owns
/// what it was handed.
///
/// **`never_written` splits the entry block in two**, and it is
/// `rule:classes/an-unwritten-property-read-throws` reaching the one read this function makes. A `lateinit` delegate field
/// (`rule:classes/lateinit`) is the only one `E0720` admits that no constructor is obliged to
/// fill, and dispatching on what the slot then holds is not a null-receiver
/// bug one call down but an unbounded recursion: [`InstKind::ClassDescOf`] on
/// a slot holding nothing reads the *forwarding* class back and the forward
/// calls itself until the stack is gone. So the read is guarded exactly as a
/// written `$w->logger` is — `Lowering::emit_never_written_guard` owns the
/// argument for why the test is the payload — and the throw is the same
/// `LogicError`, worded the same way, so that `rule:classes/delegation-by-field`'s forward reports
/// the failure `rule:classes/an-unwritten-property-read-throws` defines rather than one of its own.
///
/// The guarded edge is the one place this function's ownership is not the
/// callee's: no call runs, so every argument it was transferred is released
/// where the callee would have released it, and `$this` is left to the
/// landing block that already owes it.
///
/// `None` only when the parameter list is longer than an ABI slot index can
/// count. Every type it declares erases — including a compiler-owned
/// interface's type variable, which reaches [`super::erase_checked_ty`] here
/// and nowhere else, since a forward is written against the interface's own
/// signature rather than against a call site that substituted it.
pub(crate) fn delegation_forward(
    delegation: &nvs_types::Delegation,
    checked_types: &TypeInterner,
    never_written: bool,
) -> Option<Function> {
    let mut ids = IdGen::default();
    let entry = ids.next_block();
    let landing = ids.next_block();
    let this = ids.next_value();
    let inner = ids.next_value();
    let desc = ids.next_value();

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

    let mut entry_block = entry;
    let mut params = vec![Ty::Object];
    let mut args = Vec::new();
    let mut insts = vec![
        plain(InstKind::Safepoint),
        defines(this, Ty::Object, InstKind::Param(0)),
    ];
    for (at, param) in delegation.params.iter().enumerate() {
        let ty = super::erase_checked_ty(*param, checked_types);
        let value = ids.next_value();
        // `at + 1`: slot zero is the receiver, exactly as an ordinary
        // instance method's is.
        insts.push(defines(
            value,
            ty,
            InstKind::Param(u32::try_from(at + 1).ok()?),
        ));
        params.push(ty);
        args.push(value);
    }
    insts.push(defines(
        inner,
        Ty::Object,
        InstKind::FieldGet {
            object: this,
            class: delegation.class.clone(),
            field: delegation.field.clone(),
        },
    ));
    // `rule:classes/an-unwritten-property-read-throws`'s guard, which splits the entry block: an unguarded
    // forward keeps the single entry block the rest of this function builds.
    let mut guard_blocks = Vec::new();
    if never_written {
        let is_unset = ids.next_value();
        let unset = ids.next_block();
        let body = ids.next_block();
        insts.push(defines(
            is_unset,
            Ty::Bool,
            InstKind::IsNull { operand: inner },
        ));
        let entry_insts = std::mem::take(&mut insts);
        let mut refused = Vec::new();
        // No call runs on this edge, so each argument's transferred
        // reference is this frame's to pay back; `$this` is the landing
        // block's, which every other exit already leaves to it.
        for (&arg, param) in args.iter().zip(params.iter().skip(1)) {
            if param.is_refcounted() {
                refused.push(plain(InstKind::Release { operand: arg }));
            }
        }
        let message = ids.next_value();
        let absent = ids.next_value();
        let tagged = ids.next_value();
        let exception = ids.next_value();
        refused.push(defines(
            message,
            Ty::Str,
            InstKind::ConstStr(format!(
                "`{}`'s property `${}` is read before it is written",
                delegation.class, delegation.field
            )),
        ));
        // Argument 2 is the `{previous}` bag flattened to its own `null`
        // default, widened into the `Ty::Tagged` slot spec § 10's
        // `Throwable|null` erases to — the list every hand-built throw in
        // this crate assembles, `Lowering::lower_match`'s included.
        refused.push(defines(absent, Ty::Null, InstKind::ConstNull));
        refused.push(defines(
            tagged,
            Ty::Tagged,
            InstKind::Tag { operand: absent },
        ));
        refused.push(Inst {
            result: Some(exception),
            ty: Some(Ty::Object),
            kind: InstKind::New {
                class: "LogicError".to_owned(),
                ctor: Some(super::exception::THROWABLE_CTOR.to_owned()),
                args: vec![message, tagged],
            },
            on_error: Some(landing),
        });
        let unset_edge = ids.next_edge(delegation.span);
        let body_edge = ids.next_edge(delegation.span);
        guard_blocks.push(BasicBlock {
            id: entry,
            insts: entry_insts,
            term: Terminator::Branch {
                cond: is_unset,
                then_block: unset,
                then_edge: unset_edge,
                else_block: body,
                else_edge: body_edge,
            },
        });
        guard_blocks.push(BasicBlock {
            id: unset,
            insts: refused,
            // No source: this block is synthesized around a delegation the
            // program never wrote a `throw` in, and the machinery that resolves
            // a span to a line is `Lowering`'s rather than this builder's.
            term: Terminator::Throw {
                value: exception,
                source: None,
                landing,
            },
        });
        entry_block = body;
    }
    insts.push(plain(InstKind::Retain { operand: inner }));
    insts.push(defines(
        desc,
        Ty::ClassDesc,
        InstKind::ClassDescOf { object: inner },
    ));
    let ret = super::erase_checked_ty(delegation.return_ty, checked_types);
    // A `void` member has no value to define, so the call defines none and the
    // exit is `return;` — the same split every other `void` call already makes.
    let answer = (ret != Ty::Void).then(|| ids.next_value());
    insts.push(Inst {
        result: answer,
        ty: answer.map(|_| ret),
        kind: InstKind::CallVirtual {
            lsb: desc,
            method: delegation.method.clone(),
            fallback: None,
            receiver: Some(inner),
            args,
        },
        on_error: Some(landing),
    });
    insts.push(plain(InstKind::Release { operand: this }));

    let (stmt_spans, edge_spans) = ids.into_spans();
    let mut blocks = guard_blocks;
    blocks.extend([
        BasicBlock {
            id: entry_block,
            insts,
            term: Terminator::Return(answer),
        },
        BasicBlock {
            id: landing,
            insts: vec![plain(InstKind::Release { operand: this })],
            term: Terminator::Propagate {
                frame: delegation.frame.clone(),
            },
        },
    ]);
    Some(Function {
        name: format!("{}::{}", delegation.class, delegation.method),
        params,
        ret,
        blocks,
        entry,
        stmt_spans,
        edge_spans,
    })
}
