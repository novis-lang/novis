//! ADR 0053 § 4's state-machine transform: the frame, the spill/reload of a local live across a `yield`, and the three synthesized methods.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

impl<'a> Lowering<'a> {
    /// Reads `name`'s parked value back out of the state object and takes a
    /// reference of its own — the reload half of a generator suspension. See
    /// [`lower_generator`], which owns the whole protocol.
    pub(super) fn reload_field(&mut self, b: BlockId, name: &str, ty: Ty) -> ValueId {
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
    /// `mwl_runtime` release primitive answers a null payload with a no-op,
    /// which is what makes the first spill need no special case.
    pub(super) fn spill_field(&mut self, b: BlockId, name: &str, v: ValueId, ty: Ty) {
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
    pub(super) fn gen_target(&self) -> (String, ValueId) {
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
    pub(super) fn finish_generator(&mut self, cur: BlockId, env: &Env) {
        let (class, gen_v) = self.gen_target();
        let (done, _) = self.emit(cur, Ty::Int, InstKind::ConstInt(GEN_DONE));
        self.emit_field_set(cur, gen_v, class, GEN_STATE.to_owned(), done);
        self.release_all_locals(cur, env, None);
        let (fal, _) = self.emit(cur, Ty::Bool, InstKind::ConstBool(false));
        self.seal(cur, Terminator::Return(Some(fal)));
    }
    /// `yield expr;` — ADR 0053 § 4's suspension point, lowered as an
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
    /// Panics outside a generator body (`mwl_types` reports E0445).
    ///
    /// The assert on a [`Ty::Ref`] binding live at the suspension is an
    /// internal-consistency check, not a gap: the only thing that ever binds
    /// one is [`super::lower_method`]'s parameter loop, and a generator
    /// declaring a `&$x` parameter is `E0492` — see [`lower_generator`].
    pub(super) fn lower_yield(&mut self, value: &Expr, env: &mut Env, cur: &mut BlockId) {
        let elem = self
            .generator
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "mwl-ir: a `yield` reached lowering outside a generator body — mwl_types \
                     reports E0445 for one, so this program should not have got here"
                )
            })
            .elem;
        let (class, gen_v) = self.gen_target();

        let (v, vty) = self.lower_expr(value, Some(elem), env, cur);
        assert!(
            vty == elem,
            "mwl-ir: a `yield` operand lowered to {vty:?} where the declared `Iterator<T>` \
             gives {elem:?} — mwl_types checks the operand against `T`, so this is a lowering \
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
                "mwl-ir: the `&$x` binding `{name}` is live across a `yield` — the cell it \
                 addresses is the caller's, and the caller is gone by the time the generator \
                 resumes. Only `lower_method`'s parameter loop ever binds a `Ty::Ref`, and \
                 `mwl_types::check` refuses a generator that declares one as `E0492`, so no \
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
        self.emit_field_set(*cur, gen_v, class, GEN_STATE.to_owned(), state_v);
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
        *env = next;
        *cur = resume;
    }
}

// ---------------------------------------------------------------------------
// ADR 0053 § 4: generators
// ---------------------------------------------------------------------------

/// The state field's name in a generator's synthesized state class — which
/// resumption point [`GEN_ADVANCE`]'s entry switch enters.
///
/// A `#` can never appear in an MWL identifier (ADR 0029/0030 fix the whole
/// character set), so neither this nor [`GEN_CURRENT`] can collide with a
/// local the body spilled under its own name — the same guarantee
/// [`Lowering::lower_foreach`]'s `foreach#N` bookkeeping names rest on.
pub(super) const GEN_STATE: &str = "gen#state";

/// The most recently yielded element, which [`GEN_CURRENT_METHOD`] reads.
pub(super) const GEN_CURRENT: &str = "gen#current";

pub(super) const GEN_SELF: &str = "gen#self";

/// The state value meaning "this generator has finished" — any value no
/// resumption arm names, so the entry switch's default arm takes it.
pub(super) const GEN_DONE: i64 = -1;

/// `Iterator<T>::advance`'s name, as the method table spells it.
pub(super) const GEN_ADVANCE: &str = "advance";

/// `Iterator<T>::current`'s name.
pub(super) const GEN_CURRENT_METHOD: &str = "current";

/// One generator's synthesized state class, accumulated while its
/// `advance()` body is lowered — see [`lower_generator`].
pub(super) struct GenFrame {
    /// The state class's label.
    pub(super) class: String,
    /// `T`, from the declared `Iterator<T>` return type.
    pub(super) elem: Ty,
    /// This frame's own receiver, the state object.
    pub(super) gen_v: ValueId,
    /// Every field the class needs, in first-registered order: the two
    /// reserved ones, then each parameter, then each local some `yield`
    /// spilled. Deduplicated by name.
    pub(super) fields: Vec<(String, Ty)>,
    /// One resume block per `yield` lowered so far, in source order — the
    /// entry switch's arms, whose case value is the index plus one (state `0`
    /// is the body's own start).
    pub(super) resumes: Vec<BlockId>,
}

impl GenFrame {
    /// Registers `name` as a field at `ty`, or checks that an already-known
    /// one agrees.
    pub(super) fn field(&mut self, name: &str, ty: Ty) {
        match self.fields.iter().find(|(n, _)| n == name) {
            Some((_, known)) => assert!(
                *known == ty,
                "mwl-ir: the generator local `{name}` was spilled at {ty:?} and at {known:?} — \
                 a local's representation is fixed at its binding, so this is a lowering bug"
            ),
            None => self.fields.push((name.to_owned(), ty)),
        }
    }
}

/// Lowers a generator declaration — ADR 0053 § 4's state-machine transform.
///
/// One source method becomes **three functions and one class**:
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
/// * `{name}$gen` is the state class those two are methods of. `$` cannot
///   appear in an MWL identifier, so the label can never collide with a
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
/// # Ownership, and why it never dangles
///
/// While the generator is suspended, its fields own every reference; while
/// `advance()` is running, the locals own a second one each. A generator
/// dropped mid-sequence is dismantled like any other object, so
/// `mwl_runtime::object::dismantle` releases exactly what the last spill
/// stored — there is no state in which a slot holds a reference nobody
/// releases, and none in which two things release the same one.
///
/// # Panics
///
/// Panics naming the shape for a generator whose declared return type is not
/// an `Iterator<T>` the checker resolved (E0446 has already reported one).
///
/// The assert on a `&$x` parameter is an internal-consistency check rather
/// than a gap: a by-reference binding is the address of a caller-staged cell
/// (see [`Ty::Ref`]), which stops existing the moment the factory returns, so
/// `mwl_types::check::check_generator_by_ref_params` refuses the shape where
/// it is written, as `E0492`, and nothing that reaches here declares one.
pub(super) fn lower_generator(
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
    let (mut advance, fields) = advance;
    let current = lower_generator_current(&class, elem, src);

    let mut functions = vec![factory, advance.function, current];
    functions.append(&mut advance.closures);
    let mut classes = advance.classes;
    classes.push(crate::ir::Class {
        label: class.clone(),
        fields: fields.into_iter().map(|(n, _)| n).collect(),
        // A generator's frame is never a `SlotSet` receiver — see `ir::Class`.
        field_reprs: Vec::new(),
        // `Iterable`/`Iterator` are compiler-declared and have no layout
        // entry of their own, so `mwl_codegen::Classes::define` drops an
        // unresolvable label here the same way it does for any other —
        // which costs nothing today, since a `foreach` over a cursor
        // dispatches through the method table rather than through an
        // `instanceof`. Stated rather than left implicit: an
        // `$gen instanceof Iterator` would answer `false`.
        conforms: vec![mwl_hir_iterator_label()],
        methods: vec![
            (GEN_ADVANCE.to_owned(), class.clone()),
            (GEN_CURRENT_METHOD.to_owned(), class),
        ],
        // A generator state class is synthesized, so nothing wrote an
        // attribute on it, and no source property to carry a default.
        codec: Vec::new(),
        ctor_arity: 0,
        defaults: Vec::new(),
    });
    (functions, classes)
}

/// `Iterator`'s bare label, restated here for the reason
/// [`THROWABLE_ROOT`] is: this crate depends on neither `mwl-hir` nor
/// `mwl-types`' name resolution.
pub(super) fn mwl_hir_iterator_label() -> String {
    "Iterator".to_owned()
}

/// `T`, read back off the declared `Iterator<T>` return type the checker
/// already resolved and recorded.
pub(super) fn generator_element(
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
                "mwl-ir: the generator `{name}` has no resolved return type recorded — \
                 mwl_types reports E0446 for one that is not an `Iterator<T>`, so lowering \
                 should never have been reached"
            )
        });
    match checked_types.get(declared) {
        CheckedTy::Class(_, args) if !args.is_empty() => lower_checked_ty(args[0], checked_types),
        other => panic!(
            "mwl-ir: the generator `{name}` declares {other:?} rather than an `Iterator<T>` — \
             mwl_types reports E0446 for that"
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
pub(super) fn lower_generator_factory(
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
    let mut low = Lowering::new(name, src, Ty::Object, exprs, checked_types, enums);
    let entry = low.new_block();
    low.emit_safepoint(entry);

    // Parameter 0 is the receiver for an instance method and the called class
    // for a static one, exactly as `lower_method` seeds it.
    let recv_ty = if is_static { Ty::ClassDesc } else { Ty::Object };
    let (recv_v, _) = low.emit(entry, recv_ty, InstKind::Param(0));
    let mut param_tys = vec![recv_ty];

    let (gen_v, _) = low.emit(
        entry,
        Ty::Object,
        InstKind::New {
            class: class.to_owned(),
            ctor: None,
            args: Vec::new(),
        },
    );
    let (zero, _) = low.emit(entry, Ty::Int, InstKind::ConstInt(0));
    low.emit_field_set(entry, gen_v, class.to_owned(), GEN_STATE.to_owned(), zero);

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
            "a generator with a `&$x` parameter reached lowering: the slot it binds is a \
             caller-staged cell that stops existing when the factory returns, so there is \
             nothing sound to park in the state object — `mwl_types::check` refuses this \
             where it is written, as `E0492`"
        );
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
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
pub(super) fn lower_generator_advance(
    class: &str,
    m: &MethodMember,
    is_static: bool,
    elem: Ty,
    fields: Vec<(String, Ty)>,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (Lowered, Vec<(String, Ty)>) {
    let label = format!("{class}::{GEN_ADVANCE}");
    let mut low = Lowering::new(&label, src, Ty::Bool, exprs, checked_types, enums);
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

    // Everything the factory parked, minus the two reserved slots, is what
    // state 0 reloads — the same shape a resume block reloads, so the body
    // sees one kind of binding rather than two.
    let seeded: Vec<(String, Ty)> = fields
        .iter()
        .filter(|(n, _)| n != GEN_STATE && n != GEN_CURRENT)
        .cloned()
        .collect();
    low.generator = Some(GenFrame {
        class: class.to_owned(),
        elem,
        gen_v,
        fields,
        resumes: Vec::new(),
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
    let (blocks, stmt_spans, edge_spans) = low.finish();
    let (closures, classes) = drain_closures(pending, src, exprs, checked_types, enums);
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
    )
}

/// The one-line accessor half: hand back the element the last `yield`
/// parked, retained, since the field keeps owning its own reference.
///
/// ADR 0053 § 1 says `current()` called before the first `advance()` or after
/// one returned `false` throws. **It does not yet**: the slot is `null` at
/// both points and this reads it as a `T`, which is a known gap rather than a
/// decision — a `foreach`, the only thing that drives a cursor today, never
/// calls `current()` at either point.
pub(super) fn lower_generator_current(class: &str, elem: Ty, src: &SourceFile) -> Function {
    let mut ids = IdGen::default();
    let block = ids.next_block();
    let gen_v = ids.next_value();
    let value = ids.next_value();

    let plain = |kind: InstKind| Inst {
        result: None,
        ty: None,
        kind,
        on_error: None,
    };
    let mut insts = vec![
        plain(InstKind::Safepoint),
        Inst {
            result: Some(gen_v),
            ty: Some(Ty::Object),
            kind: InstKind::Param(0),
            on_error: None,
        },
        Inst {
            result: Some(value),
            ty: Some(elem),
            kind: InstKind::FieldGet {
                object: gen_v,
                class: class.to_owned(),
                field: GEN_CURRENT.to_owned(),
            },
            on_error: None,
        },
    ];
    if elem.is_refcounted() {
        insts.push(plain(InstKind::Retain { operand: value }));
    }
    insts.push(plain(InstKind::Release { operand: gen_v }));

    let _ = src;
    let (stmt_spans, edge_spans) = ids.into_spans();
    Function {
        name: format!("{class}::{GEN_CURRENT_METHOD}"),
        params: vec![Ty::Object],
        ret: elem,
        blocks: vec![BasicBlock {
            id: block,
            insts,
            term: Terminator::Return(Some(value)),
        }],
        entry: block,
        stmt_spans,
        edge_spans,
    }
}
