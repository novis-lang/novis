//! ADR 0031's closure literals, lowered to an object of a synthesized class with one field per capture.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

/// The [`Env`] name a generator's `advance()`/`current()` frame holds its own
/// receiver — the state object — under. Present so the ordinary exit sweep
/// ([`Lowering::release_all_locals`]) and every landing block release it
/// without a special case; excluded from spilling, since a field of the state
/// object pointing at the state object is a cycle with nothing to say.
/// The reserved `Env` name a closure's `invoke` binds its own captured-
/// environment object under — the receiver, so that
/// [`Lowering::release_all_locals`] releases it at every exit with no
/// closure-specific cleanup path. `#` cannot appear in an MWL identifier, so
/// it can never collide with a capture or a parameter.
pub(super) const FN_SELF: &str = "fn#self";

/// One `fn` literal met while lowering a body, waiting for its own function
/// to be built — see [`lower_closure`].
///
/// Owns its [`FnExpr`] rather than borrowing it. A borrow would have to live
/// as long as [`Lowering`]'s own lifetime parameter, which is the *source
/// file's*; threading a second one through every `lower_expr` call site to
/// buy back one clone of a small AST subtree, once per closure literal, at
/// compile time only, is the wrong trade under this repository's priority
/// ordering.
pub(super) struct PendingClosure {
    /// The environment class's label.
    pub(super) class: String,
    /// The literal itself.
    pub(super) fn_expr: FnExpr,
    /// Every captured binding, in the order `mwl_types` recorded it — which
    /// is the field order of the class above, so the two sides agree by
    /// construction rather than by both sorting the same way.
    pub(super) captures: Vec<(String, Ty)>,
    /// What the body produces.
    pub(super) ret: Ty,
}

/// The [`FN_PARAM_TAGS`] word for `fn_expr` — one nibble per declared
/// parameter, in declaration order, least significant first.
///
/// Read at the *literal*, where the declared types are still in hand, and
/// stored in the object the literal builds; that constant owns the encoding
/// and why the object carries it at all. Parameters past
/// [`FN_PARAM_TAGS_CAPACITY`] contribute no nibble, which is what makes
/// `mwl_runtime::call_closure` refuse the call rather than pass an argument it
/// cannot judge.
///
/// # Panics
///
/// Naming ADR 0007 § 1 for a parameter with no declared type, exactly as
/// [`lower_closure`] does for the same parameter list.
pub(super) fn param_tags_word(
    fn_expr: &FnExpr,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> i64 {
    let mut word: u64 = 0;
    for (i, p) in fn_expr
        .params
        .iter()
        .take(FN_PARAM_TAGS_CAPACITY)
        .enumerate()
    {
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
        let nibble = param_tag_nibble(lower_decl_type(decl_ty, exprs, checked_types));
        word |= u64::from(nibble) << (i * 4);
    }
    // A sixteenth parameter puts a nibble in the sign bit. The slot holds the
    // same 64 bits whichever way they are read, and the reader takes them
    // apart nibble by nibble.
    i64::from_ne_bytes(word.to_ne_bytes())
}

/// Lowers every pending closure, and every closure *those* bodies contain, to
/// exhaustion.
pub(super) fn drain_closures(
    mut pending: Vec<PendingClosure>,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (Vec<Function>, Vec<crate::ir::Class>) {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    while let Some(next) = pending.pop() {
        let (function, synthesized, more) = lower_closure(&next, src, exprs, checked_types, enums);
        functions.push(function);
        classes.extend(synthesized);
        pending.extend(more);
    }
    (functions, classes)
}

/// Lowers one `fn` literal's body to the `invoke` method of its own
/// captured-environment class — [ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md)
/// § 1/§ 2.
///
/// # The representation
///
/// A closure **is an object**, of a class with no source declaration: one
/// field per captured binding, one method, no supertypes. That is the whole
/// design, and it is a reuse decision rather than a new mechanism —
/// refcounting, field slots, class descriptors and the indirect call through
/// [`mwl_runtime::mwl_class_method`] all already exist for ordinary objects,
/// and a closure needs exactly those four things and nothing else. The
/// alternative, a dedicated code-pointer-plus-environment header, would be a
/// second refcounted heap shape for the runtime to know about, a second thing
/// `mwl_runtime::object::dismantle` has to sweep, and a second call path in
/// `mwl-codegen` — for no capability the object shape does not already have.
///
/// The cost is stated rather than hidden: one heap allocation per evaluation
/// of a `fn` literal, plus one 16-byte slot per captured binding, plus a
/// method-table lookup per call through it. A closure that captures nothing
/// still allocates; folding that case to a shared singleton is a real
/// optimisation, and deliberately not taken here, because the allocation is
/// what makes every closure value uniform for the caller.
///
/// The receiver is parameter 0, exactly as it is for a declared method, so
/// the closure's own environment reaches its body through the same
/// [`InstKind::Param`] any method's `$this` does — and calling one is an
/// ordinary MWL method call at the ABI level, which is what lets a native
/// `Core` member invoke a closure with no closure-specific entry point.
///
/// # Ownership
///
/// The literal site retains every capture it stores, so the environment
/// object owns one reference per field for as long as it lives; `invoke`
/// retains again when it reads one back into a local, and
/// [`Lowering::release_all_locals`] pays that back at every exit. The
/// receiver is bound in `Env` under [`FN_SELF`] for exactly that reason: a
/// callee owns its parameters, and putting it in `Env` is what makes the
/// existing sweep release it rather than needing a closure-specific one.
///
/// # Panics
///
/// Panics naming the shape for a `fn` literal the checker recorded no
/// [`ExprInfo::Closure`] for, and for a parameter with no declared type.
///
/// The assert on an `inout $x` parameter is an internal-consistency check rather
/// than a gap: `callable` carries no parameter list for a call site to read
/// (ADR 0031 § 4), so `mwl_types::expr::calls` refuses one as `E0493` and
/// nothing that reaches here declares one.
///
/// # Returns
///
/// The environment class first, then one per ADR 0036 § 2 shape literal the
/// body wrote — [`Lowering::shapes`], which has nowhere else to travel.
pub(super) fn lower_closure(
    pending: &PendingClosure,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (Function, Vec<crate::ir::Class>, Vec<PendingClosure>) {
    let PendingClosure {
        class,
        fn_expr,
        captures,
        ret,
    } = pending;
    let label = format!("{class}::{FN_INVOKE}");
    let mut low = Lowering::new(&label, src, *ret, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    low.emit_safepoint(entry);

    let (self_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    let mut env = Env::default();
    env.insert(FN_SELF.to_owned(), (self_v, Ty::Object));
    let mut param_tys = vec![Ty::Object];

    // The captures first, so a parameter of the same name — which shadows one,
    // per `mwl_types::expr::calls::check_fn_literal` — overwrites it rather than the
    // other way round.
    for (name, ty) in captures {
        let (v, _) = low.emit(
            entry,
            *ty,
            InstKind::FieldGet {
                object: self_v,
                class: class.clone(),
                field: name.clone(),
            },
        );
        if ty.is_refcounted() {
            low.emit_retain(entry, v);
        }
        env.insert(name.clone(), (v, *ty));
    }

    // Every parameter is bound in `entry` — `InstKind::Param` is an
    // entry-block instruction — and the class checks run afterwards, from
    // `cur`, so that each one's throw path sees the *whole* parameter list in
    // `env` and releases the frame rather than the prefix bound so far.
    let mut class_checks: Vec<(usize, ValueId, String, Span)> = Vec::new();
    for (i, p) in fn_expr.params.iter().enumerate() {
        assert!(
            !p.inout,
            "a closure with an `inout $x` parameter reached lowering: a closure's type is \
             `callable` and carries no parameter list, so there is no call site that could \
             know to stage the cell — `mwl_types::expr::calls` refuses this where it is \
             written, as `E0493`"
        );
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
        let ty = lower_decl_type(decl_ty, exprs, checked_types);
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        let (v, _) = low.emit(entry, ty, InstKind::Param(index));
        if let Some(class) = declared_class(decl_ty, exprs, checked_types) {
            class_checks.push((i, v, class, decl_ty.span));
        }
        env.insert(pname, (v, ty));
        param_tys.push(ty);
    }
    for (i, value, class, span) in class_checks {
        cur = check_param_class(&mut low, cur, i, value, &class, span, &mut env);
    }

    match &fn_expr.body {
        // An expression body is an implicit `return` (ADR 0031 § 1), lowered
        // through the same path `StmtKind::Return` uses: retain if the value
        // is a borrowed read, release the frame's locals, return.
        FnBody::Expr(body) => {
            let (v, ty) = low.lower_expr(body, Some(*ret), &mut env, &mut cur);
            if ty.is_refcounted() && low.aliasing_read(body) {
                low.emit_retain(cur, v);
            }
            low.release_all_locals(cur, &env, None);
            low.seal(cur, Terminator::Return(Some(v)));
        }
        FnBody::Block(block) => {
            low.lower_stmts(&block.stmts, &mut cur, &mut env);
            if !low.is_terminated(cur) {
                low.release_all_locals(cur, &env, None);
                low.seal(cur, Terminator::Return(None));
            }
        }
    }

    let more = std::mem::take(&mut low.closures);
    // A shape literal written *inside* a closure body synthesizes its class
    // here rather than in the enclosing function, so it rides out beside the
    // environment class — see `Lowering::shapes`.
    let shapes = std::mem::take(&mut low.shapes);
    let (blocks, stmt_spans, edge_spans) = low.finish();
    (
        Function {
            name: label,
            params: param_tys,
            ret: *ret,
            blocks,
            entry,
            stmt_spans,
            edge_spans,
        },
        std::iter::once(crate::ir::Class {
            label: class.clone(),
            // `FN_ARITY` first and `FN_PARAM_TAGS` second, always — a native
            // caller reads both by index, not by name, so a capture's slot is
            // its position in this list plus two. See those constants.
            fields: [FN_ARITY.to_owned(), FN_PARAM_TAGS.to_owned()]
                .into_iter()
                .chain(captures.iter().map(|(n, _)| n.clone()))
                .collect(),
            // A closure's environment is never a `SlotSet` receiver: it has no
            // shape type and no erased view reaches it. See `ir::Class`.
            field_reprs: Vec::new(),
            conforms: Vec::new(),
            methods: vec![(FN_INVOKE.to_owned(), class.clone())],
            // A closure is not a declaration and carries no attribute, and
            // every one of its slots is written by the factory that builds it.
            codec: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
        })
        .chain(shapes)
        .collect(),
        more,
    )
}

/// The class an annotation names and a run-time test can compare against, or
/// `None` where nothing can check it. Two callers ask the same question: a
/// closure parameter's entry check below, and
/// [`Lowering::lower_checked_downcast`](super::Lowering::lower_checked_downcast),
/// ADR 0007 § 6's checked way out of `mixed`. `mwl_types` refuses the `None`
/// case at the conversion (`E0711`), so the second caller's `None` is a
/// conversion this crate has no lowering for rather than a shape it declines.
///
/// The declaration has to be a class *name* and nothing wider: `object`, a
/// shape and `?C` alike erase to a representation that names no class — a `?C`
/// through [`Ty::Tagged`], which is why the nullable spelling still accepts
/// anything an object-or-null slot can hold. See
/// [`super::param_tag_nibble`] for the four-bit half of the same question.
///
/// A `Core` class is the one named class that answers `None`: it has no
/// descriptor in the unit — `mwl_codegen`'s class table is built from
/// `mwl_types::layout`, which holds the declared tree — so an `instanceof`
/// against one has nothing to compare and does not exist as a spelling either
/// (`E0496` at the checker). That leaves a `Core\Cli\Text $c` parameter
/// checked for objecthood alone, which `docs/adr/README.md` § *Decisions taken
/// at project start* records as the remainder rather than the rule.
pub(super) fn declared_class(
    ty: &Type,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> Option<String> {
    let id = exprs.declared_ty(ty.span)?;
    match checked_types.get(id) {
        // `QName` is destructured here rather than passed on: `mwl-hir` is a
        // dev-dependency of this crate, so a helper naming the type in its
        // signature would not compile.
        CheckedTy::Class(qname, _) if !qname.is_core() => Some(qname.to_string()),
        _ => None,
    }
}

/// The class check one class-declared closure parameter runs at the body's
/// first block, returning the block the body continues in.
///
/// `mwl_runtime::closure::check_param_tags` compares representations, and a
/// four-bit nibble has no room for a class label, so every class name arrives
/// as the same "an object" — a lie a named-class binding then reads and writes
/// at a *fixed offset*, which is a type confusion rather than a wrong answer.
/// `docs/adr/README.md` § *Decisions taken at project start* owns why the
/// closure's entry is the boundary that pays: one
/// `mwl_runtime::object::MwlObj::is_instance_of` per class-declared parameter
/// per call, in the one position nothing else looked, rather than a name-keyed
/// fetch at every named-class property access in every program.
///
/// The refusal is the sentence `check_param_tags` already writes, with the
/// declared class where a representation would be. What arrived is *not*
/// named: no IR instruction reads an object's class name — the runtime's
/// `mwl_object_class_name` has no [`InstKind`] wrapping it — and adding one to
/// widen a message is a new value shape for a diagnostic's sake. The ADR
/// paragraph records that as the message's known limit.
fn check_param_class(
    low: &mut Lowering<'_>,
    cur: BlockId,
    index: usize,
    value: ValueId,
    class: &str,
    span: Span,
    env: &mut Env,
) -> BlockId {
    // A closure literal's body has no enclosing statement of its own, so the
    // parameter's annotation is what `write_throw_location` and
    // `Lowering::frame_label` render — the closure's own line, rather than the
    // file's first.
    low.cur_stmt_span = span;
    let (is_instance, _) = low.emit(
        cur,
        Ty::Bool,
        InstKind::InstanceOf {
            value,
            class: class.to_owned(),
        },
    );
    let body = low.new_block();
    let refused = low.new_block();
    let body_edge = low.ids.next_edge(span);
    let refused_edge = low.ids.next_edge(span);
    low.seal(
        cur,
        Terminator::Branch {
            cond: is_instance,
            then_block: body,
            then_edge: body_edge,
            else_block: refused,
            else_edge: refused_edge,
        },
    );
    let (message, _) = low.emit(
        refused,
        Ty::Str,
        InstKind::ConstStr(format!(
            "argument {} to a `callable` must be of type {class}, another class given",
            index + 1
        )),
    );
    // Argument 2 is the `{previous}` bag flattened to its own `null` default,
    // widened into the `Ty::Tagged` slot spec § 10's `Throwable|null` erases
    // to — the same list `Lowering::lower_match`'s unmatched throw builds by
    // hand, and for the same reason.
    let (absent, _) = low.emit(refused, Ty::Null, InstKind::ConstNull);
    let absent = low.coerce(refused, absent, Ty::Null, Ty::Tagged, env);
    let (exception, _) = low.emit_fallible(
        refused,
        Ty::Object,
        InstKind::New {
            class: "LogicError".to_owned(),
            ctor: Some(THROWABLE_CTOR.to_owned()),
            args: vec![message, absent],
        },
        env,
    );
    low.write_throw_location(refused, exception);
    let landing = low.landing_block(env);
    low.seal(
        refused,
        Terminator::Throw {
            value: exception,
            landing,
        },
    );
    body
}
