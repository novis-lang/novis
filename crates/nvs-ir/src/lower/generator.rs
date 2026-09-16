//! `rule:iteration/generators`'s state-machine transform: the frame, the spill/reload of a local live across a `yield`, and the synthesized methods.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. The methods
//! are `pub(crate)` so they reach across these modules and no further.
//!
//! **`current()` outside the protocol throws spec § 10's `LogicError`** — ADR
//! 0053 § 1 says it throws and does not say what, so this is where that is
//! decided. Driving a cursor is the calling code's own control flow, so
//! reading an element it never advanced to is a bug in that code rather than a
//! condition the run produced, which is the whole of the `LogicError` /
//! `RuntimeError` split. [`lower_generator_current`] is the guard and owns why
//! one comparison on [`GEN_STATE`] is exactly the two points § 1 names.

use super::*;

impl<'a> Lowering<'a> {
    /// Reads `name`'s parked value back out of the state object and takes a
    /// reference of its own — the reload half of a generator suspension. See
    /// [`lower_generator`], which owns the whole protocol.
    pub(crate) fn reload_field(&mut self, b: BlockId, name: &str, ty: Ty) -> ValueId {
        let (class, gen_v) = self.gen_target();
        let (v, _) = self.emit(
            b,
            ty,
            InstKind::FieldGet {
                object: gen_v,
                class,
                field: name.to_owned(),
            },
        );
        if ty.is_refcounted() {
            self.emit_retain(b, v);
        }
        v
    }
    /// Parks `v` in `name`'s field — the spill half. The field takes its own
    /// reference and releases whatever it held before, which at the first
    /// suspension is the `null` [`InstKind::New`] left there; every
    /// `nvs_runtime` release primitive answers a null payload with a no-op,
    /// which is what makes the first spill need no special case.
    pub(crate) fn spill_field(&mut self, b: BlockId, name: &str, v: ValueId, ty: Ty) {
        self.generator
            .as_mut()
            .expect("spill_field is only reached inside a generator frame")
            .field(name, ty);
        let (class, gen_v) = self.gen_target();
        if ty.is_refcounted() {
            let (old, _) = self.emit(
                b,
                ty,
                InstKind::FieldGet {
                    object: gen_v,
                    class: class.clone(),
                    field: name.to_owned(),
                },
            );
            self.emit_retain(b, v);
            self.emit_field_set(b, gen_v, class, name.to_owned(), v);
            self.emit_release(b, old);
        } else {
            self.emit_field_set(b, gen_v, class, name.to_owned(), v);
        }
    }
    /// This generator frame's state-class label and receiver.
    ///
    /// # Panics
    ///
    /// Panics outside a generator's `advance()` — every caller is reached
    /// only from one.
    pub(crate) fn gen_target(&self) -> (String, ValueId) {
        let frame = self
            .generator
            .as_ref()
            .expect("a generator field access outside a generator frame");
        (frame.class.clone(), frame.gen_v)
    }
    /// Marks this generator finished and leaves `advance()` with `false` —
    /// what a bare `return;` in the body and running off its end both do.
    ///
    /// The state moves to [`GEN_DONE`], which no resumption arm names, so a
    /// further `advance()` takes the entry switch's default arm and answers
    /// `false` again rather than re-running anything.
    pub(crate) fn finish_generator(&mut self, cur: BlockId, env: &Env) {
        let (class, gen_v) = self.gen_target();
        let (done, _) = self.emit(cur, Ty::Int, InstKind::ConstInt(GEN_DONE));
        self.emit_field_set(cur, gen_v, class, GEN_STATE.to_owned(), done);
        self.release_all_locals(cur, env, None);
        let (fal, _) = self.emit(cur, Ty::Bool, InstKind::ConstBool(false));
        self.seal(cur, Terminator::Return(Some(fal)));
    }
    /// `yield expr;` — `rule:iteration/generators`'s suspension point, lowered as an
    /// ordinary `return true` bracketed by a spill and a reload.
    ///
    /// [`lower_generator`] owns the protocol and the reason it is shaped this
    /// way; what happens here is exactly its two halves in order: park the
    /// element, park every binding, record which resumption point this is,
    /// leave the frame the way any `return` would, and open the resume block
    /// the enclosing statement carries on in.
    ///
    /// # Panics
    ///
    /// Panics outside a generator body (`nvs_types` reports E0445).
    ///
    /// The assert on a [`Ty::Ref`] binding live at the suspension is an
    /// internal-consistency check, not a gap: the only thing that ever binds
    /// one is [`super::lower_method`]'s parameter loop, and a generator
    /// declaring an `inout $x` parameter is `E0492` — see [`lower_generator`].
    pub(crate) fn lower_yield(&mut self, value: &Expr, env: &mut Env, cur: &mut BlockId) {
        let elem = self
            .generator
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "nvs-ir: a `yield` reached lowering outside a generator body — nvs_types \
                     reports E0445 for one, so this program should not have got here"
                )
            })
            .elem;
        let (class, gen_v) = self.gen_target();

        let (v, vty) = self.lower_expr(value, Some(elem), env, cur);
        assert!(
            vty == elem,
            "nvs-ir: a `yield` operand lowered to {vty:?} where the declared `Iterator<T>` \
             gives {elem:?} — nvs_types checks the operand against `T`, so this is a lowering \
             bug"
        );
        if vty.is_refcounted() && self.aliasing_read(value) {
            self.emit_retain(*cur, v);
        }
        // The element field owns its reference between suspensions, which is
        // what lets `current()` hand out a retained copy without the loop
        // driving it having to know anything about ownership.
        if elem.is_refcounted() {
            let (old, _) = self.emit(
                *cur,
                elem,
                InstKind::FieldGet {
                    object: gen_v,
                    class: class.clone(),
                    field: GEN_CURRENT.to_owned(),
                },
            );
            self.emit_field_set(*cur, gen_v, class.clone(), GEN_CURRENT.to_owned(), v);
            self.emit_release(*cur, old);
        } else {
            self.emit_field_set(*cur, gen_v, class.clone(), GEN_CURRENT.to_owned(), v);
        }

        // Sorted rather than left in `FxHashMap`'s bucket order, for
        // `Self::release_all_locals`' reason: an emitted instruction's id must
        // depend only on source order.
        let mut names: Vec<String> = env
            .keys()
            .filter(|n| n.as_str() != GEN_SELF)
            .cloned()
            .collect();
        names.sort();
        let mut spilled: Vec<(String, Ty)> = Vec::with_capacity(names.len());
        for name in names {
            let &(lv, lty) = &env[&name];
            assert!(
                lty != Ty::Ref,
                "nvs-ir: the `inout $x` binding `{name}` is live across a `yield` — the cell it \
                 addresses is the caller's, and the caller is gone by the time the generator \
                 resumes. Only `lower_method`'s parameter loop ever binds a `Ty::Ref`, and \
                 `nvs_types::check` refuses a generator that declares one as `E0492`, so no \
                 program reaches this"
            );
            self.spill_field(*cur, &name, lv, lty);
            spilled.push((name, lty));
        }

        let index = self
            .generator
            .as_ref()
            .expect("checked above")
            .resumes
            .len();
        let state = i64::try_from(index + 1).expect("far fewer than i64::MAX yields in one body");
        let (state_v, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(state));
        self.emit_field_set(*cur, gen_v, class.clone(), GEN_STATE.to_owned(), state_v);
        self.release_all_locals(*cur, env, None);
        let (t, _) = self.emit(*cur, Ty::Bool, InstKind::ConstBool(true));
        self.seal(*cur, Terminator::Return(Some(t)));

        let resume = self.new_block();
        self.generator
            .as_mut()
            .expect("checked above")
            .resumes
            .push(resume);
        let mut next = Env::default();
        next.insert(GEN_SELF.to_owned(), (gen_v, Ty::Object));
        for (name, lty) in spilled {
            let rv = self.reload_field(resume, &name, lty);
            next.insert(name, (rv, lty));
        }

        // The resume-to-unwind test, and the one place [`GEN_UNWIND`] is read.
        // Emitted only where this suspension point sits inside a protected
        // region that owns a `finally`: everywhere else an abandoned generator
        // owes nothing, so there is nothing for the entry point to resume
        // *into* and the branch would be a test no run can take. See
        // [`lower_generator`] § *An abandoned generator runs its `finally`*.
        let mut carry_on = resume;
        if self.try_stack.iter().any(|frame| frame.finally.is_some()) {
            self.generator
                .as_mut()
                .expect("checked above")
                .unwind_states
                .push(state);
            let owed = self.new_block();
            let resumed = self.new_block();
            let (flag, _) = self.emit(
                resume,
                Ty::Int,
                InstKind::FieldGet {
                    object: gen_v,
                    class,
                    field: GEN_UNWIND.to_owned(),
                },
            );
            let (zero, _) = self.emit(resume, Ty::Int, InstKind::ConstInt(0));
            let (abandoned, _) = self.emit(
                resume,
                Ty::Bool,
                InstKind::BinOp {
                    op: BinOp::NotEq,
                    lhs: flag,
                    rhs: zero,
                },
            );
            let owed_edge = self.ids.next_edge(value.span);
            let resumed_edge = self.ids.next_edge(value.span);
            self.seal(
                resume,
                Terminator::Branch {
                    cond: abandoned,
                    then_block: owed,
                    then_edge: owed_edge,
                    else_block: resumed,
                    else_edge: resumed_edge,
                },
            );
            // Exactly what `return;` lowers to at this same point (see
            // `Lowering::lower_stmt`'s generator arm): every enclosing
            // `finally` in turn, innermost first, then the frame left the way
            // running off the end leaves it. The env is cloned because those
            // bodies bind and rebind like any other statements, and the
            // resumption path below must see the reloaded bindings unchanged.
            let mut unwind_env = next.clone();
            let mut unwind_cur = owed;
            self.run_pending_finallys(&mut unwind_cur, &mut unwind_env);
            if !self.is_terminated(unwind_cur) {
                self.finish_generator(unwind_cur, &unwind_env);
            }
            carry_on = resumed;
        }

        *env = next;
        *cur = carry_on;
    }
}

// ---------------------------------------------------------------------------
// `rule:iteration/generators`: generators
// ---------------------------------------------------------------------------

/// The state field's name in a generator's synthesized state class — which
/// resumption point [`GEN_ADVANCE`]'s entry switch enters.
///
/// A `#` can never appear in an Novis identifier (`rule:core-api/identifier-casing`/0030 fix the whole
/// character set), so neither this nor [`GEN_CURRENT`] can collide with a
/// local the body spilled under its own name — the same guarantee
/// [`Lowering::lower_foreach`]'s `foreach#N` bookkeeping names rest on.
pub(crate) const GEN_STATE: &str = "gen#state";

/// The most recently yielded element, which [`GEN_CURRENT_METHOD`] reads.
pub(crate) const GEN_CURRENT: &str = "gen#current";

/// Set to `1` by [`GEN_UNWIND_METHOD`] and read by every suspension point
/// inside a `finally`-owning region: `0` means an ordinary resumption, and
/// anything else means "you are being abandoned — run what you owe and
/// finish". See [`lower_generator`] § *An abandoned generator runs its
/// `finally`*.
///
/// **Cost:** one field slot on every generator's state object, whether or not
/// its body has a `finally` at all — the factory is lowered before the body,
/// so what the body turns out to owe is not yet known when the field list is
/// fixed. Eight bytes per *suspended* generator, which is O(in-flight) and
/// last in AGENTS.md's priority ordering.
pub(crate) const GEN_UNWIND: &str = "gen#unwind";

pub(crate) const GEN_SELF: &str = "gen#self";

/// The state value meaning "this generator has finished" — any value no
/// resumption arm names, so the entry switch's default arm takes it.
pub(crate) const GEN_DONE: i64 = -1;

/// `Iterator<T>::advance`'s name, as the method table spells it.
pub(crate) const GEN_ADVANCE: &str = "advance";

/// `Iterator<T>::current`'s name.
pub(crate) const GEN_CURRENT_METHOD: &str = "current";

/// The resume-to-unwind entry point's name in the state class's method table,
/// which is how the release path reaches it — see
/// [`lower_generator_unwind`]. It is not a member of `Iterator<T>`, which
/// declares only `advance` and `current`.
///
/// Spelled with the `#` every parked field carries, and for a stronger reason
/// than theirs: `nvs_runtime::object::dismantle` probes **every** dying
/// object's class for this name, so a name a source program could declare
/// would turn a user method into the destructor
/// `rule:classes/no-destructors` says Novis does not have. `#` is not in an identifier, so no class but
/// one this transform synthesized can answer. `nvs_runtime` restates the
/// string as `nvs_runtime::object::GENERATOR_UNWIND_METHOD`, for the reason
/// [`THROWABLE_ROOT`] is restated here.
pub(crate) const GEN_UNWIND_METHOD: &str = "gen#unwind";

/// One generator's synthesized state class, accumulated while its
/// `advance()` body is lowered — see [`lower_generator`].
pub(crate) struct GenFrame {
    /// The state class's label.
    pub(crate) class: String,
    /// `T`, from the declared `Iterator<T>` return type.
    pub(crate) elem: Ty,
    /// This frame's own receiver, the state object.
    pub(crate) gen_v: ValueId,
    /// Every field the class needs, in first-registered order: the two
    /// reserved ones, then each parameter, then each local some `yield`
    /// spilled. Deduplicated by name.
    pub(crate) fields: Vec<(String, Ty)>,
    /// One resume block per `yield` lowered so far, in source order — the
    /// entry switch's arms, whose case value is the index plus one (state `0`
    /// is the body's own start).
    pub(crate) resumes: Vec<BlockId>,
    /// The state values whose resume block grew an unwind arm — the
    /// suspension points that sit inside a `finally`-owning region, and so the
    /// only ones [`lower_generator_unwind`] may resume into.
    ///
    /// Collected rather than recomputed because it is what makes the entry
    /// point *safe*: resuming a suspension that owes nothing would run the
    /// rest of the body, which is the opposite of abandoning it.
    pub(crate) unwind_states: Vec<i64>,
}

impl GenFrame {
    /// Registers `name` as a field at `ty`, or checks that an already-known
    /// one agrees.
    pub(crate) fn field(&mut self, name: &str, ty: Ty) {
        match self.fields.iter().find(|(n, _)| n == name) {
            Some((_, known)) => assert!(
                *known == ty,
                "nvs-ir: the generator local `{name}` was spilled at {ty:?} and at {known:?} — \
                 a local's representation is fixed at its binding, so this is a lowering bug"
            ),
            None => self.fields.push((name.to_owned(), ty)),
        }
    }
}

/// Lowers a generator declaration — `rule:iteration/generators`'s state-machine transform.
///
/// One source method becomes **one class and the functions that drive it**:
///
/// * `name` itself keeps the label every call site already resolves to, but
///   runs no user code at all: it allocates the state object, stores its
///   receiver and every argument into that object's fields, and returns it.
///   That is § 4's "calling it runs no user code", and it is what makes a
///   generator's result an ordinary `Iterator<T>` value rather than a
///   suspended frame.
/// * `{name}$gen::advance` holds the original body, cut into resumption
///   segments at each `yield`.
/// * `{name}$gen::current` returns the last yielded element.
/// * `{name}$gen::unwind` is the resume-to-unwind entry point — see below.
/// * `{name}$gen` is the state class those methods belong to. `$` cannot
///   appear in an Novis identifier, so the label can never collide with a
///   user class.
///
/// # How the body survives being cut in half
///
/// The body is lowered by the ordinary [`Lowering`] machinery, unchanged —
/// same `Env`, same phis, same loops, same landing blocks. Only the two ends
/// of a `yield` are new, and they are exact inverses:
///
/// * **Spill.** Every `Env` binding is stored into a field of the state
///   object, then the frame exits with `true` exactly as an ordinary
///   `return` would, releasing what it owes. The field takes its own
///   reference first, so the two do not cancel.
/// * **Reload.** The resume block reads every one of those fields back and
///   retains it, rebuilding an `Env` with the same names at fresh SSA values.
///
/// So a value never has to live *across* a suspension in SSA form, which is
/// the thing a state machine cannot express — and the resume block is an
/// ordinary block the enclosing `while`/`if`/`try` lowering then continues
/// from, so a `yield` inside a loop body needs nothing from this function at
/// all: the loop's own back edge picks up the reloaded values as one more
/// incoming edge to its header phi.
///
/// Spilling *everything* rather than only what is live across the `yield` is
/// deliberate: liveness would be an analysis this crate does not have, and
/// what it would buy is fewer stores in a routine that is already returning.
///
/// # An abandoned generator runs its `finally`
///
/// A generator suspended inside `try { … } finally { … }` and then dropped
/// still owes that `finally` body. PHP resumes such a generator in a
/// return-like mode and prints it, and priority 2 (PHP-compatible observable
/// behaviour) outranks priority 4 (simplicity), so Novis does the same.
///
/// The mechanism is one field and one entry point, both of them ordinary:
///
/// * [`GEN_UNWIND`] is a flag on the state object, `0` until something sets
///   it.
/// * [`lower_generator_unwind`] builds `{name}$gen::gen#unwind`, which the
///   release path calls as it dismantles the state object
///   (`nvs_runtime::object::dismantle` § *An abandoned generator runs its
///   `finally`* owns that end, including the resurrection it needs). It sets
///   that flag and calls `advance()` — but only when the parked state says the
///   generator is actually *suspended*. A state of `0` means the body has
///   never been entered, so no `try` has been entered either and there is
///   nothing to run; [`GEN_DONE`] means it has already finished. Both are
///   PHP's answers too.
/// * Each suspension point inside a `finally`-owning region reloads its
///   bindings as it always did, then branches on that flag. The unwind arm is
///   lowered as exactly what `return;` lowers to at that same point —
///   [`Lowering::run_pending_finallys`] over every enclosing region, innermost
///   first, then [`Lowering::finish_generator`] — so the ladder, the release
///   of each binding and the exit are the ones the body already had, not a
///   second copy of the rules.
///
/// **This is not a destructor**, and it re-opens nothing in
/// `rule:classes/no-destructors`: no user code runs that the program did not already suspend inside, no
/// `__destruct` is recognized on any class, and a generator's state class gains
/// no lifecycle hook a user class could ever declare. `unwind` resumes a
/// suspended frame; it does not tear an object down.
///
/// **`unwind` borrows its receiver**, alone among compiled methods, so that
/// the release path can call it at the moment a count has already reached
/// zero without that count crossing zero a second time.
/// [`lower_generator_unwind`] states the argument in full.
///
/// # Ownership, and why it never dangles
///
/// While the generator is suspended, its fields own every reference; while
/// `advance()` is running, the locals own a second one each. A generator
/// dropped mid-sequence is dismantled like any other object, so
/// `nvs_runtime::object::dismantle` releases exactly what the last spill
/// stored — there is no state in which a slot holds a reference nobody
/// releases, and none in which two things release the same one.
///
/// # Panics
///
/// Panics naming the shape for a generator whose declared return type is not
/// an `Iterator<T>` the checker resolved (E0446 has already reported one).
///
/// The assert on an `inout $x` parameter is an internal-consistency check rather
/// than a gap: a by-reference binding is the address of a caller-staged cell
/// (see [`Ty::Ref`]), which stops existing the moment the factory returns, so
/// `nvs_types::check::check_generator_inout_params` refuses the shape where
/// it is written, as `E0492`, and nothing that reaches here declares one.
pub(crate) fn lower_generator(
    name: &str,
    m: &MethodMember,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (Vec<Function>, Vec<crate::ir::Class>) {
    let class = format!("{name}$gen");
    let elem = generator_element(name, m, exprs, checked_types);
    let is_static = m.modifiers.contains(&Modifier::Static);

    let mut fields = vec![
        (GEN_STATE.to_owned(), Ty::Int),
        (GEN_CURRENT.to_owned(), elem),
        (GEN_UNWIND.to_owned(), Ty::Int),
    ];
    let factory = lower_generator_factory(
        name,
        &class,
        m,
        is_static,
        &mut fields,
        src,
        exprs,
        checked_types,
        enums,
    );
    let advance = lower_generator_advance(
        &class,
        m,
        is_static,
        elem,
        fields.clone(),
        src,
        exprs,
        checked_types,
        enums,
    );
    let (mut advance, fields, owed) = advance;
    let current = lower_generator_current(&class, m, elem, src, exprs, checked_types, enums);

    let mut functions = vec![factory, advance.function, current];
    // A generator no suspension of which sits inside a `finally`-owning region
    // owes nothing when it is abandoned, so it carries no entry point at all
    // and `nvs_runtime::object::dismantle` finds a null field rather than a
    // method to call.
    if !owed.is_empty() {
        functions.push(lower_generator_unwind(
            &class,
            m,
            &owed,
            src,
            exprs,
            checked_types,
            enums,
        ));
    }
    functions.append(&mut advance.closures);
    let mut classes = advance.classes;
    classes.push(crate::ir::Class {
        label: class.clone(),
        fields: fields.into_iter().map(|(n, _)| n).collect(),
        // A generator's frame is never a `SlotSet` receiver — see `ir::Class`.
        field_reprs: Vec::new(),
        secret_fields: Vec::new(),
        public_fields: Vec::new(),
        field_types: Vec::new(),
        // `Iterable`/`Iterator` are compiler-declared and have no layout
        // entry of their own, so `nvs_codegen::Classes::define` drops an
        // unresolvable label here the same way it does for any other —
        // which costs nothing today, since a `foreach` over a cursor
        // dispatches through the method table rather than through an
        // `instanceof`. Stated rather than left implicit: an
        // `$gen instanceof Iterator` would answer `false`.
        conforms: vec![nvs_hir_iterator_label()],
        // Public, all three: a state class is unspellable and its members are
        // named by `foreach`'s own lowering and by `dismantle`, neither of
        // which is inside any class.
        methods: {
            let mut methods = vec![
                (GEN_ADVANCE.to_owned(), class.clone(), true),
                (GEN_CURRENT_METHOD.to_owned(), class.clone(), true),
            ];
            if !owed.is_empty() {
                methods.push((GEN_UNWIND_METHOD.to_owned(), class, true));
            }
            methods
        },
        // Its slots are spilled locals, and a local declares no accessor.
        hooks: Vec::new(),
        // A generator state class is synthesized, so nothing wrote an
        // attribute on it, and no source property to carry a default.
        codec: Vec::new(),
        db_codec: Vec::new(),
        ctor_arity: 0,
        defaults: Vec::new(),
    });
    (functions, classes)
}

/// `Iterator`'s bare label, restated here for the reason
/// [`THROWABLE_ROOT`] is: this crate depends on neither `nvs-hir` nor
/// `nvs-types`' name resolution.
pub(crate) fn nvs_hir_iterator_label() -> String {
    "Iterator".to_owned()
}

/// `T`, read back off the declared `Iterator<T>` return type the checker
/// already resolved and recorded.
pub(crate) fn generator_element(
    name: &str,
    m: &MethodMember,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> Ty {
    let declared = m
        .return_type
        .as_ref()
        .and_then(|t| exprs.declared_ty(t.span))
        .unwrap_or_else(|| {
            panic!(
                "nvs-ir: the generator `{name}` has no resolved return type recorded — \
                 nvs_types reports E0446 for one that is not an `Iterator<T>`, so lowering \
                 should never have been reached"
            )
        });
    match checked_types.get(declared) {
        CheckedTy::Class(_, args) if !args.is_empty() => erase_checked_ty(args[0], checked_types),
        other => panic!(
            "nvs-ir: the generator `{name}` declares {other:?} rather than an `Iterator<T>` — \
             nvs_types reports E0446 for that"
        ),
    }
}

/// The factory half: allocate the state object, park the receiver and every
/// argument in it, return it. See [`lower_generator`].
#[expect(
    clippy::too_many_arguments,
    reason = "the arguments are one declaration's own parts plus the four \
              tables every lowering entry point takes"
)]
pub(crate) fn lower_generator_factory(
    name: &str,
    class: &str,
    m: &MethodMember,
    is_static: bool,
    fields: &mut Vec<(String, Ty)>,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> Function {
    let mut low = Lowering::new(
        name,
        Some(name),
        src,
        Ty::Object,
        exprs,
        checked_types,
        enums,
    );
    let entry = low.new_block();
    low.emit_safepoint(entry);

    // Parameter 0 is the receiver for an instance method and the called class
    // for a static one, exactly as `lower_method` seeds it.
    let recv_ty = if is_static { Ty::ClassDesc } else { Ty::Object };
    let (recv_v, _) = low.emit(entry, recv_ty, InstKind::Param(0));
    let mut param_tys = vec![recv_ty];

    // The landing block is over an empty `Env` because that is what this
    // frame holds here: the state object is the first thing it allocates, so
    // a failing status has nothing of its own to release and only propagates.
    let (gen_v, _) = low.emit_fallible(
        entry,
        Ty::Object,
        InstKind::New {
            class: class.to_owned(),
            ctor: None,
            args: Vec::new(),
        },
        &Env::default(),
    );
    let (zero, _) = low.emit(entry, Ty::Int, InstKind::ConstInt(0));
    low.emit_field_set(entry, gen_v, class.to_owned(), GEN_STATE.to_owned(), zero);
    // Written rather than left at the `New`'s null payload, because the flag
    // is read as a plain `Ty::Int` at every suspension point that tests it.
    low.emit_field_set(entry, gen_v, class.to_owned(), GEN_UNWIND.to_owned(), zero);

    // Every stored parameter *transfers* the reference the caller handed this
    // frame — the field owns it from here, and there is no release to pair,
    // which is why the factory never sweeps its own locals. A `static`
    // method's parameter 0 is a `Ty::ClassDesc` and is simply dropped: it is
    // not refcounted, and nothing in a generator body can ask for it (see
    // `Lowering::lsb`'s panic).
    if !is_static {
        fields.push(("this".to_owned(), Ty::Object));
        low.emit_field_set(entry, gen_v, class.to_owned(), "this".to_owned(), recv_v);
    }
    for (i, p) in m.params.iter().enumerate() {
        assert!(
            !p.inout,
            "a generator with an `inout $x` parameter reached lowering: the slot it binds is a \
             caller-staged cell that stops existing when the factory returns, so there is \
             nothing sound to park in the state object — `nvs_types::check` refuses this \
             where it is written, as `E0492`"
        );
        let decl_ty = p.ty.as_ref().unwrap_or_else(|| {
            panic!("`rule:types/declaration`: every parameter has a declared type")
        });
        let ty = lower_decl_type(decl_ty, exprs, checked_types);
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        let (v, _) = low.emit(entry, ty, InstKind::Param(index));
        param_tys.push(ty);
        fields.push((pname.clone(), ty));
        low.emit_field_set(entry, gen_v, class.to_owned(), pname, v);
    }
    low.seal(entry, Terminator::Return(Some(gen_v)));

    let (blocks, stmt_spans, edge_spans) = low.finish();
    Function {
        name: name.to_owned(),
        params: param_tys,
        ret: Ty::Object,
        blocks,
        entry,
        stmt_spans,
        edge_spans,
    }
}

/// The body half: the original statements, cut into resumption segments,
/// behind an entry switch on the parked state. See [`lower_generator`].
#[expect(
    clippy::too_many_arguments,
    reason = "the arguments are one declaration's own parts plus the four \
              tables every lowering entry point takes"
)]
pub(crate) fn lower_generator_advance(
    class: &str,
    m: &MethodMember,
    is_static: bool,
    elem: Ty,
    fields: Vec<(String, Ty)>,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (Lowered, Vec<(String, Ty)>, Vec<i64>) {
    let label = format!("{class}::{GEN_ADVANCE}");
    let mut low = Lowering::new(
        &label,
        Some(&label),
        src,
        Ty::Bool,
        exprs,
        checked_types,
        enums,
    );
    let entry = low.new_block();
    low.emit_safepoint(entry);
    let (gen_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    let (state_v, _) = low.emit(
        entry,
        Ty::Int,
        InstKind::FieldGet {
            object: gen_v,
            class: class.to_owned(),
            field: GEN_STATE.to_owned(),
        },
    );

    // The two fixed arms. `start` is state 0 — the first `advance()`, which
    // reloads what the factory parked and runs the body from the top;
    // `exhausted` is the default, reached both by a generator that has
    // already finished and by one whose body ran off the end.
    let start = low.new_block();
    let exhausted = low.new_block();

    // Everything the factory parked, minus the reserved slots, is what
    // state 0 reloads — the same shape a resume block reloads, so the body
    // sees one kind of binding rather than two.
    let seeded: Vec<(String, Ty)> = fields
        .iter()
        .filter(|(n, _)| n != GEN_STATE && n != GEN_CURRENT && n != GEN_UNWIND)
        .cloned()
        .collect();
    low.generator = Some(GenFrame {
        class: class.to_owned(),
        elem,
        gen_v,
        fields,
        resumes: Vec::new(),
        unwind_states: Vec::new(),
    });

    let mut env = Env::default();
    env.insert(GEN_SELF.to_owned(), (gen_v, Ty::Object));
    let mut cur = start;
    for (name, ty) in &seeded {
        let v = low.reload_field(cur, name, *ty);
        env.insert(name.clone(), (v, *ty));
    }
    // `$this` inside a generator body is an ordinary reloaded local, so
    // `Lowering::this` stays `None` and `static::`/`new static()` panic
    // naming the gap rather than reading a value from a block that does not
    // dominate every resume point.
    let _ = is_static;

    let body = m
        .body
        .as_ref()
        .expect("lower_generator is only reached for a declaration with a body");
    low.lower_stmts(&body.stmts, &mut cur, &mut env);
    if !low.is_terminated(cur) {
        low.finish_generator(cur, &env);
    }

    let (fal, _) = low.emit(exhausted, Ty::Bool, InstKind::ConstBool(false));
    low.emit_release(exhausted, gen_v);
    low.seal(exhausted, Terminator::Return(Some(fal)));

    // Sealed last, because the arms are exactly the `yield`s the body turned
    // out to contain — a block's terminator is a separate field from its
    // instruction list, so appending the switch here still lands after the
    // loads above.
    let frame = low
        .generator
        .take()
        .expect("just installed this frame's own");
    let arms: Vec<(i64, BlockId, EdgeId)> = std::iter::once(start)
        .chain(frame.resumes.iter().copied())
        .enumerate()
        .map(|(i, block)| {
            let case = i64::try_from(i).expect("far fewer than i64::MAX yields in one body");
            (case, block, low.ids.next_edge(m.name))
        })
        .collect();
    let default_edge = low.ids.next_edge(m.name);
    low.seal(
        entry,
        Terminator::Switch {
            value: state_v,
            arms,
            default: exhausted,
            default_edge,
        },
    );

    let pending = std::mem::take(&mut low.closures);
    // Beside the closures, and out the same channel: see `Lowering::callables`.
    let callables = std::mem::take(&mut low.callables);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, classes) = drain_closures(pending, callables, src, exprs, checked_types, enums);
    (
        Lowered {
            function: Function {
                name: label,
                params: vec![Ty::Object],
                ret: Ty::Bool,
                blocks,
                entry,
                stmt_spans,
                edge_spans,
            },
            closures,
            classes,
        },
        frame.fields,
        frame.unwind_states,
    )
}

/// The accessor half: `rule:iteration/two-interfaces`'s protocol guard, and behind it the element
/// the last `yield` parked, retained, since the field keeps owning its own
/// reference.
///
/// **The guard is `gen#state >= 1`, and that one comparison is exactly § 1's
/// two out-of-protocol points.** [`GEN_STATE`] is `0` from
/// [`generator_factory`] until the first [`GEN_ADVANCE`] returns; every
/// suspension parks its own resume index, which is `index + 1` and therefore
/// never `0`; and [`Lowering::finish_generator`] parks [`GEN_DONE`] — `-1` —
/// when the body runs off the end. So "before the first `advance()`" and
/// "after one returned `false`" are the only two states below `1`, and the
/// guard needs no flag of its own: it reads the field the entry switch
/// already reads, and the state object grows by nothing.
///
/// **It throws `LogicError`**, spec § 10's class for a caller that broke a
/// contract it could have checked, rather than a `RuntimeError` under it.
/// Which element a cursor is on is a fact of the calling code's own control
/// flow rather than of the run — the caller either drove `advance()` or did
/// not — so this is the same kind of failure a null receiver is, and never a
/// condition the program has to be prepared for. Both points share one
/// message: telling them apart would cost a second branch on the one path
/// that is about to unwind anyway, and the fix a reader needs is the same.
///
/// **Cost:** one integer compare and one branch per `current()` call, on the
/// arm that is always taken in a well-formed loop. `foreach` drives
/// `advance()`/`current()` in lockstep and so never reaches the throw, which
/// makes this the same price `rule:errors/propagation`'s status check pays after every call —
/// AGENTS.md's priority 3, bought for its priority 2.
pub(crate) fn lower_generator_current(
    class: &str,
    m: &MethodMember,
    elem: Ty,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> Function {
    let label = format!("{class}::{GEN_CURRENT_METHOD}");
    let mut low = Lowering::new(&label, Some(&label), src, elem, exprs, checked_types, enums);
    // The generator's declaration is the nearest real source a body nobody
    // wrote has, so it is what `$e->location` and the backtrace frame name.
    low.cur_stmt_span = m.name;
    let entry = low.new_block();
    low.emit_safepoint(entry);
    let (gen_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    let (state_v, _) = low.emit(
        entry,
        Ty::Int,
        InstKind::FieldGet {
            object: gen_v,
            class: class.to_owned(),
            field: GEN_STATE.to_owned(),
        },
    );
    let (first, _) = low.emit(entry, Ty::Int, InstKind::ConstInt(1));
    let (inside, _) = low.emit(
        entry,
        Ty::Bool,
        InstKind::BinOp {
            op: BinOp::GtEq,
            lhs: state_v,
            rhs: first,
        },
    );
    let read = low.new_block();
    let outside = low.new_block();
    let read_edge = low.ids.next_edge(m.name);
    let outside_edge = low.ids.next_edge(m.name);
    low.seal(
        entry,
        Terminator::Branch {
            cond: inside,
            then_block: read,
            then_edge: read_edge,
            else_block: outside,
            else_edge: outside_edge,
        },
    );

    let (value, _) = low.emit(
        read,
        elem,
        InstKind::FieldGet {
            object: gen_v,
            class: class.to_owned(),
            field: GEN_CURRENT.to_owned(),
        },
    );
    if elem.is_refcounted() {
        low.emit_retain(read, value);
    }
    low.emit_release(read, gen_v);
    low.seal(read, Terminator::Return(Some(value)));

    // The receiver goes back *before* the exception is built, not after: this
    // arm has no further use for it, and the `New` below carries an error edge
    // of its own that would otherwise need a release of its own too.
    low.emit_release(outside, gen_v);
    let (message, _) = low.emit(
        outside,
        Ty::Str,
        InstKind::ConstStr(OUTSIDE_THE_PROTOCOL.to_owned()),
    );
    // Tagged rather than bare, because spec § 10 types the `previous` option
    // `Throwable|null` and the constructor reads parameter 2 as one `Value`.
    // Nothing to retain: a `Ty::Null` holds no reference to take.
    let (absent, _) = low.emit(outside, Ty::Null, InstKind::ConstNull);
    let (previous, _) = low.emit(outside, Ty::Tagged, InstKind::Tag { operand: absent });
    // Over an empty `Env` for `generator_factory`'s reason: this body binds no
    // local, so a failing status has nothing left to release and only
    // propagates.
    let (thrown, _) = low.emit_fallible(
        outside,
        Ty::Object,
        InstKind::New {
            class: LOGIC_ERROR.to_owned(),
            ctor: Some(THROWABLE_CTOR.to_owned()),
            args: vec![message, previous],
        },
        &Env::default(),
    );
    let source = low.throw_source(outside);
    let landing = low.landing_block(&Env::default());
    low.seal(
        outside,
        Terminator::Throw {
            value: thrown,
            source,
            landing,
        },
    );

    let (blocks, stmt_spans, edge_spans) = low.finish();
    Function {
        name: label,
        params: vec![Ty::Object],
        ret: elem,
        blocks,
        entry,
        stmt_spans,
        edge_spans,
    }
}

/// The message [`lower_generator_current`]'s guard raises, at both of `rule:iteration/two-interfaces`'s two points.
const OUTSIDE_THE_PROTOCOL: &str =
    "current() outside the iteration protocol: it answers only after advance() returned true";

/// The resume-to-unwind entry point: `{class}::unwind`, which the release
/// path calls on a generator that is being dropped.
///
/// No user code of its own: it reads the parked state and, where that state is
/// one of `owed` — the suspension points that sit inside a `finally`-owning
/// region, [`GenFrame::unwind_states`] — it raises the [`GEN_UNWIND`] flag and
/// re-enters [`GEN_ADVANCE`], whose entry switch lands on that suspension's own
/// resume block. The branch there takes the unwind arm, runs every enclosing
/// `finally` and finishes the generator, so this function never has to know
/// anything about the body it is unwinding.
///
/// **The test is membership, not `state > 0`**, and the difference is the whole
/// correctness of it: a resume block that owes nothing carries no unwind arm,
/// so entering it would carry on running the body rather than abandon it. A
/// generator with no owed state at all gets no entry point emitted and no
/// method table row — [`lower_generator`] is where that is decided.
///
/// Why it is a *method* rather than something the release path does
/// itself: the flag's slot index and the entry switch's encoding are both
/// facts of this transform, and a runtime that had to know either would be
/// holding a copy of [`lower_generator`]'s protocol. A method table entry is
/// a name, which is the only thing the release path should need.
///
/// **Argument 0 is borrowed, and this is the one compiled method of which that
/// is true** — every other owns its parameters, which is why
/// [`Lowering::emit_iface_call`] retains before calling `advance` or
/// `current`. It is inverted here because of who calls it: the release path
/// reaches this at the moment a count has already hit zero, so a convention
/// that made this function consume a reference would ask that caller to hand
/// over one it no longer has, and the release `advance()` performs on its way
/// out would then cross zero a second time and re-enter the release path on
/// the very allocation it is already dismantling. Borrowing removes the
/// crossing rather than guarding it: the retain here pairs with `advance`'s
/// own release, so the count this function is handed is the count it leaves
/// behind, whatever that count is.
///
/// An exception thrown by a `finally` body on the way out propagates to the
/// caller: the call names no landing block because this frame owns nothing
/// left to release at that point.
pub(crate) fn lower_generator_unwind(
    class: &str,
    m: &MethodMember,
    owed: &[i64],
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> Function {
    let label = format!("{class}::{GEN_UNWIND_METHOD}");
    let mut low = Lowering::new(
        &label,
        Some(&label),
        src,
        Ty::Void,
        exprs,
        checked_types,
        enums,
    );
    let entry = low.new_block();
    low.emit_safepoint(entry);
    let (gen_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    let (state_v, _) = low.emit(
        entry,
        Ty::Int,
        InstKind::FieldGet {
            object: gen_v,
            class: class.to_owned(),
            field: GEN_STATE.to_owned(),
        },
    );
    // One test per suspension point that owes a `finally`, and deliberately
    // not `state > 0`: a resume block that owes nothing carries no unwind arm,
    // so resuming into it would run the rest of the body to completion, which
    // is the opposite of abandoning it.
    let resume = low.new_block();
    let nothing_owed = low.new_block();
    let mut test = entry;
    for state in owed {
        let (want, _) = low.emit(test, Ty::Int, InstKind::ConstInt(*state));
        let (hit, _) = low.emit(
            test,
            Ty::Bool,
            InstKind::BinOp {
                op: BinOp::Eq,
                lhs: state_v,
                rhs: want,
            },
        );
        let next = low.new_block();
        let resume_edge = low.ids.next_edge(m.name);
        let next_edge = low.ids.next_edge(m.name);
        low.seal(
            test,
            Terminator::Branch {
                cond: hit,
                then_block: resume,
                then_edge: resume_edge,
                else_block: next,
                else_edge: next_edge,
            },
        );
        test = next;
    }
    low.seal(test, Terminator::Jump(nothing_owed));

    let (one, _) = low.emit(resume, Ty::Int, InstKind::ConstInt(1));
    low.emit_field_set(resume, gen_v, class.to_owned(), GEN_UNWIND.to_owned(), one);
    // The retain is what makes argument 0 *borrowed* — see this function's own
    // doc comment for why this one method inverts the convention.
    low.emit_retain(resume, gen_v);
    // Over an empty `Env` for `generator_factory`'s reason: this entry point
    // binds no local of its own, and the reference it holds on `gen_v` is the
    // one `advance` consumes on either edge. A status coming back out of it
    // is the caller's — `nvs_runtime::object::dismantle` reached this from a
    // release path, and `Ctx::with_pending_set_aside` is what decides what
    // becomes of a throw raised there.
    low.emit_fallible(
        resume,
        Ty::Bool,
        InstKind::Call {
            target: format!("{class}::{GEN_ADVANCE}"),
            receiver: Some(gen_v),
            args: Vec::new(),
        },
        &Env::default(),
    );
    low.seal(resume, Terminator::Return(None));

    low.seal(nothing_owed, Terminator::Return(None));

    let (blocks, stmt_spans, edge_spans) = low.finish();
    Function {
        name: label,
        params: vec![Ty::Object],
        ret: Ty::Void,
        blocks,
        entry,
        stmt_spans,
        edge_spans,
    }
}
