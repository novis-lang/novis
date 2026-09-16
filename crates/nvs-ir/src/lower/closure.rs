//! `rule:types/closure-literal`'s closure literals, lowered to an object of a synthesized class with one field per capture.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. The methods
//! are `pub(crate)` so they reach across these modules and no further, which
//! is the reach a single-file `lower` would give them.

use super::*;

/// The [`Env`] name a generator's `advance()`/`current()` frame holds its own
/// receiver — the state object — under. Present so the ordinary exit sweep
/// ([`Lowering::release_all_locals`]) and every landing block release it
/// without a special case; excluded from spilling, since a field of the state
/// object pointing at the state object is a cycle with nothing to say.
/// The reserved `Env` name a closure's `invoke` binds its own captured-
/// environment object under — the receiver, so that
/// [`Lowering::release_all_locals`] releases it at every exit with no
/// closure-specific cleanup path. `#` cannot appear in an Novis identifier, so
/// it can never collide with a capture or a parameter.
pub(crate) const FN_SELF: &str = "fn#self";

/// One `fn` literal met while lowering a body, waiting for its own function
/// to be built — see [`lower_closure`].
///
/// Owns its [`FnExpr`] rather than borrowing it. A borrow would have to live
/// as long as [`Lowering`]'s own lifetime parameter, which is the *source
/// file's*; threading a second one through every `lower_expr` call site to
/// buy back one clone of a small AST subtree, once per closure literal, at
/// compile time only, is the wrong trade under this repository's priority
/// ordering.
pub(crate) struct PendingClosure {
    /// The environment class's label.
    pub(crate) class: String,
    /// The literal itself.
    pub(crate) fn_expr: FnExpr,
    /// Every captured binding, in the order `nvs_types` recorded it — which
    /// is the field order of the class above, so the two sides agree by
    /// construction rather than by both sorting the same way.
    pub(crate) captures: Vec<(String, Ty)>,
    /// What the body produces.
    pub(crate) ret: Ty,
    /// Where the literal was written. The key `nvs_types::ExprTypeTable`
    /// recorded this closure's `callable(...)` conformance under, and the only
    /// one both sides hold: the class label above is this crate's own name for
    /// the site and never reaches the checker.
    pub(crate) span: Span,
}

/// The [`FN_PARAM_TAGS`] word for `fn_expr` — one nibble per declared
/// parameter, in declaration order, least significant first.
///
/// Read at the *literal*, where the declared types are still in hand, and
/// stored in the object the literal builds; that constant owns the encoding
/// and why the object carries it at all. This half is the *lowering* of each
/// declared type — the packing itself is [`super::pack_param_tags`], shared
/// with the method row `nvs-codegen` writes for the same reader.
///
/// # Panics
///
/// Reading each parameter through [`closure_param_ty`], exactly as
/// [`lower_closure`] reads the same list.
pub(crate) fn param_tags_word(
    fn_expr: &FnExpr,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> i64 {
    let word = super::pack_param_tags(
        fn_expr
            .params
            .iter()
            .map(|p| closure_param_ty(p, exprs, checked_types).0),
    );
    // A sixteenth parameter puts a nibble in the sign bit. The slot holds the
    // same 64 bits whichever way they are read, and the reader takes them
    // apart nibble by nibble.
    i64::from_ne_bytes(word.to_ne_bytes())
}

/// Lowers every pending closure, and every closure *those* bodies contain, to
/// exhaustion — then every [`PendingCallable`] met along the way.
///
/// The two travel together because a closure body may write a first-class
/// callable and a thunk body may not write anything at all: draining the
/// closures first is what makes `callables` complete by the time the second
/// loop starts, so neither list needs a second pass.
pub(crate) fn drain_closures(
    mut pending: Vec<PendingClosure>,
    mut callables: Vec<PendingCallable>,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (Vec<Function>, Vec<crate::ir::Class>) {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    while let Some(next) = pending.pop() {
        let (function, synthesized, more, more_callables) =
            lower_closure(&next, src, exprs, checked_types, enums);
        functions.push(function);
        classes.extend(synthesized);
        pending.extend(more);
        callables.extend(more_callables);
    }
    for next in &callables {
        let (function, synthesized) = lower_callable(next, src, exprs, checked_types, enums);
        functions.push(function);
        classes.push(synthesized);
    }
    (functions, classes)
}

/// Lowers one `fn` literal's body to the `invoke` method of its own
/// captured-environment class — `rule:types/closure-literal`/§ 2.
///
/// # The representation
///
/// A closure **is an object**, of a class with no source declaration: one
/// field per captured binding, one method, no supertypes. That is the whole
/// design, and it is a reuse decision rather than a new mechanism —
/// refcounting, field slots, class descriptors and the indirect call through
/// [`nvs_runtime::nvs_class_method`] all already exist for ordinary objects,
/// and a closure needs exactly those and nothing else. The alternative, a
/// dedicated code-pointer-plus-environment header, would be a second
/// refcounted heap shape for the runtime to know about, a second thing
/// `nvs_runtime::object::dismantle` has to sweep, and a second call path in
/// `nvs-codegen` — for no capability the object shape does not already have.
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
/// ordinary Novis method call at the ABI level, which is what lets a native
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
/// (`rule:types/callable-absorbs-closure`), so `nvs_types::expr::calls` refuses one as `E0493` and
/// nothing that reaches here declares one.
///
/// # Returns
///
/// The environment class first, then one per `rule:types/object-literal` shape literal the
/// body wrote — [`Lowering::shapes`], which has nowhere else to travel.
pub(crate) fn lower_closure(
    pending: &PendingClosure,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (
    Function,
    Vec<crate::ir::Class>,
    Vec<PendingClosure>,
    Vec<PendingCallable>,
) {
    let PendingClosure {
        class,
        fn_expr,
        captures,
        ret,
        span,
    } = pending;
    let label = format!("{class}::{FN_INVOKE}");
    let mut low = Lowering::new(&label, Some(&label), src, *ret, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    low.emit_safepoint(entry);

    let (self_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    let mut env = Env::default();
    env.insert(FN_SELF.to_owned(), (self_v, Ty::Object));
    let mut param_tys = vec![Ty::Object];

    // The captures first, so a parameter of the same name — which shadows one,
    // per `nvs_types::expr::calls::check_fn_literal` — overwrites it rather than the
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
             know to stage the cell — `nvs_types::expr::calls` refuses this where it is \
             written, as `E0493`"
        );
        let (ty, class, ty_span) = closure_param_ty(p, exprs, checked_types);
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        let (v, _) = low.emit(entry, ty, InstKind::Param(index));
        if let Some(class) = class {
            class_checks.push((i, v, class, ty_span));
        }
        env.insert(pname, (v, ty));
        param_tys.push(ty);
    }
    for (i, value, class, span) in class_checks {
        cur = check_param_class(&mut low, cur, i, value, &class, span, &mut env);
    }

    match &fn_expr.body {
        // An expression body is an implicit `return` (`rule:types/closure-literal`), lowered
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
    // `rule:types/callable-is-a-closure`'s `(...)` written inside a closure body has the same nowhere
    // else to go — see `Lowering::callables`.
    let more_callables = std::mem::take(&mut low.callables);
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
            secret_fields: Vec::new(),
            // Left empty on `secret_fields`' terms: a capture is not a declared
            // property, so no keyword decided either bit — and an unreachable
            // slot reading as unreadable is the direction `field_is_public`
            // wants.
            public_fields: Vec::new(),
            // And for the same reason again: a capture's type was written on
            // the variable it closes over, not on a property declaration.
            field_types: Vec::new(),
            // The marker `rule:types/callable-is-a-closure`'s `$x is callable`
            // walks for — see `super::CLOSURE_MARKER` — and one more per
            // written signature this literal satisfies, which is the same walk
            // one step more specific. The checker decided the second set,
            // keyed by this literal's span; `nvs_types::callables` is why.
            conforms: std::iter::once(super::CLOSURE_MARKER.to_owned())
                .chain(exprs.callable_markers_at(*span).iter().cloned())
                .collect(),
            // Public: a closure's environment class is unspellable, so nothing
            // can name this member at all except the runtime's own call path.
            methods: vec![(FN_INVOKE.to_owned(), class.clone(), true)],
            // Nor a property to hook: every slot is a capture.
            hooks: Vec::new(),
            // A closure is not a declaration and carries no attribute, and
            // every one of its slots is written by the factory that builds it.
            codec: Vec::new(),
            db_codec: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
        })
        .chain(shapes)
        .collect(),
        more,
        more_callables,
    )
}

/// The class an annotation names and a run-time test can compare against, or
/// `None` where nothing can check it. The same question is asked by a closure
/// parameter's entry check below and by
/// [`Lowering::lower_checked_downcast`](super::Lowering::lower_checked_downcast),
/// `rule:types/unions-and-mixed`'s checked way out of `mixed`. `nvs_types` refuses the `None`
/// case at the conversion (`E0711`), so the downcast's `None` is a
/// conversion this crate has no lowering for rather than a shape it declines.
///
/// The declaration has to be a class *name* and nothing wider: `object`, a
/// shape and `?C` alike erase to a representation that names no class — a `?C`
/// through [`Ty::Tagged`], which is why the nullable spelling still accepts
/// anything an object-or-null slot can hold. See
/// [`super::param_tag_nibble`] for the four-bit half of the same question.
///
/// A `Core` class is the one named class that answers `None`: it has no
/// descriptor in the unit — `nvs_codegen`'s class table is built from
/// `nvs_types::layout`, which holds the declared tree — so an `instanceof`
/// against one has nothing to compare and does not exist as a spelling either
/// (`E0496` at the checker). That leaves a `Core\Cli\Text $c` parameter
/// checked for objecthood alone, which `docs/adr/README.md` § *Decisions taken
/// at project start* records as the remainder rather than the rule.
pub(crate) fn declared_class(
    ty: &Type,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> Option<String> {
    let id = exprs.declared_ty(ty.span)?;
    match checked_types.get(id) {
        // `QName` is destructured here rather than passed on: `nvs-hir` is a
        // dev-dependency of this crate, so a helper naming the type in its
        // signature would not compile.
        CheckedTy::Class(qname, _) if !qname.is_core() => Some(qname.to_string()),
        _ => None,
    }
}

/// The class check one class-declared closure parameter runs at the body's
/// first block, returning the block the body continues in.
///
/// `nvs_runtime::closure::check_param_tags` compares representations, and a
/// four-bit nibble has no room for a class label, so every class name arrives
/// as the same "an object" — a lie a named-class binding then reads and writes
/// at a *fixed offset*, which is a type confusion rather than a wrong answer.
/// `docs/adr/README.md` § *Decisions taken at project start* owns why the
/// closure's entry is the boundary that pays: one
/// `nvs_runtime::object::NvsObj::is_instance_of` per class-declared parameter
/// per call, in the one position nothing else looked, rather than a name-keyed
/// fetch at every named-class property access in every program.
///
/// The refusal is the sentence `check_param_tags` already writes, with the
/// declared class where a representation would be. What arrived is *not*
/// named: no IR instruction reads an object's class name — the runtime's
/// `nvs_object_class_name` has no [`InstKind`] wrapping it — and adding one to
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
    // parameter's annotation is what `Lowering::source` reads — the closure's
    // own line, rather than the file's first — for the carrier a throw here
    // hands the raise and for `Lowering::frame_label` alike.
    low.cur_stmt_span = span;
    let (is_instance, _) = low.emit(
        cur,
        Ty::Bool,
        InstKind::InstanceOf {
            value,
            class: TestedClass::Named(class.to_owned()),
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
    let source = low.throw_source(refused);
    let landing = low.landing_block(env);
    low.seal(
        refused,
        Terminator::Throw {
            value: exception,
            source,
            landing,
        },
    );
    body
}

/// The reserved field an **instance** first-class callable's object holds its
/// target's receiver under — `$obj->method(...)` and the `self::method(...)`
/// spelling of a non-`static` member alike
/// (`rule:types/callable-is-a-closure`).
///
/// Absent from a static target's class, which has nothing to remember: its
/// called class is a compile-time constant the thunk materializes for itself.
/// Named like [`FN_ARITY`] so no declaration can collide with it, and placed
/// **last** so a native reader's slot arithmetic over the reserved fields
/// ahead of it — [`FN_ARITY`], [`FN_PARAM_TAGS`] and
/// [`FN_PARAM_NAMES`](crate::lower::FN_PARAM_NAMES) — is the one those
/// constants already state. Nothing reads this field by index: the thunk
/// reads it by name, so a reserved field placed in front of it costs a
/// sentence here and no code anywhere.
pub(crate) const FCC_RECV: &str = "fcc#recv";

/// One `Class::method(...)`/`$obj->method(...)` met while lowering a body,
/// waiting for the thunk that forwards it — see [`lower_callable`].
///
/// Owns its [`ResolvedCall`] for [`PendingClosure`]'s reason: the borrow would
/// have to live as long as the source file's lifetime, and a call's resolved
/// facts are a small clone taken once per written `(...)`.
pub(crate) struct PendingCallable {
    /// The synthesized class's label, unique within the compiled unit — the
    /// enclosing frame's own label plus this site's index, so two files
    /// naming the same member still get two labels and no `Function` in
    /// [`crate::ir::Program`] is written twice.
    pub(crate) class: String,
    /// The member the `(...)` named.
    pub(crate) call: ResolvedCall,
    /// Whether the target takes a receiver — `!ResolvedCall::is_static`,
    /// decided at the site and recorded so the thunk and the object's field
    /// list cannot disagree about whether [`FCC_RECV`] exists.
    pub(crate) takes_receiver: bool,
    /// Each parameter's representation, positional and already lowered —
    /// the thunk's own parameter list past its receiver, and the word
    /// [`FN_PARAM_TAGS`] packs at the site.
    pub(crate) params: Vec<Ty>,
    /// Where the `(...)` was written, for the frame label and for the class
    /// check of a class-declared parameter. A thunk has no statement of its
    /// own, exactly as a closure literal's body has none.
    pub(crate) span: Span,
}

/// Lowers one first-class callable to the `invoke` method of a class
/// synthesized for that one site — `rule:types/callable-is-a-closure`, on top of
/// [`lower_closure`]'s representation and adding nothing to it.
///
/// # Why a thunk rather than another call shape
///
/// `rule:types/closure-literal` makes `callable` the only closure type, so the *value* a
/// `(...)` produces has to be the same object every `fn` literal produces:
/// [`FN_ARITY`], [`FN_PARAM_TAGS`], and one `invoke` the runtime reaches
/// through the method table. Given that, the cheapest correct body for that
/// `invoke` is the forwarding call this builds — every argument passed
/// straight through, the receiver read back out of [`FCC_RECV`] — and the
/// alternative, teaching `nvs_runtime::call_closure` to dispatch on a second
/// closure shape carrying a method row instead of a code pointer, is a second
/// callable representation for every native caller to know about. The cost is
/// stated rather than hidden: one extra compiled function per written
/// `(...)`, and one extra call frame per invocation through one.
///
/// # What the thunk captures, and what it does not
///
/// An instance target's receiver is stored **by value at the point the
/// `(...)` is evaluated**, which is `rule:types/implicit-capture`'s rule for a capture and the
/// answer PHP's own first-class callable syntax gives. A static target's
/// called class is baked in as an [`InstKind::ClassDescConst`] instead.
///
/// **`static::method(...)` binds the declaring class, not the frame's called
/// class.** The checker records `ResolvedCall::static_class` only for a
/// written class name, and a class descriptor is not a value a field slot can
/// hold — so the one spelling whose late static binding would have to survive
/// past the site is bound early. `self::`/`parent::` are unaffected, since
/// both mean the declaring class already. This is the whole of what this
/// lowering does not answer; it is a divergence worth a redesign rather than
/// a bug in the shape.
///
/// # Ownership
///
/// The thunk owns each of its own parameters, as any callee does, and hands
/// that ownership straight on: a compiled target is passed transferred
/// arguments and releases them itself, and a `Core` helper borrows, so the
/// thunk keeps them as owned temporaries and releases them after the call.
/// The receiver is a borrowed read out of a field, so it is retained before
/// it is passed. The closure object itself is bound under [`FN_SELF`], which
/// is what makes [`Lowering::release_all_locals`] release it at every exit.
///
/// # What a `void` target hands back
///
/// Nothing, by the same seal every other `void` frame uses — a
/// `Terminator::Return(None)`, as `lower_method` writes when a body runs out
/// of statements. A `void` call *defines* no value, [`Ty::Void`] having no
/// register representation at all (`nvs_codegen::ty::clif_ty`), so returning
/// the call's result names an operand nothing defines; the split is the one
/// [`super::call`]'s delegation thunk already makes.
///
/// The caller still receives a value, and it is `null`:
/// `nvs_runtime::abi::call` pre-sets the `out` slot it hands a compiled
/// function, and a frame that returns nothing leaves it as it found it. So
/// `$f()` over a `void` target answers `null` — what calling that method
/// directly in a value position would answer — rather than a second
/// representation for a caller to test for. Returning an explicit
/// `InstKind::ConstNull` instead would produce the same value through an
/// instruction, and would make this the one thunk whose shape differs from
/// the method it names.
pub(crate) fn lower_callable(
    pending: &PendingCallable,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> (Function, crate::ir::Class) {
    let PendingCallable {
        class,
        call,
        takes_receiver,
        params,
        span,
    } = pending;
    let ret = erase_checked_ty(call.return_ty, checked_types);
    let label = format!("{class}::{FN_INVOKE}");
    let mut low = Lowering::new(&label, Some(&label), src, ret, exprs, checked_types, enums);
    let entry = low.new_block();
    let mut cur = entry;
    low.emit_safepoint(entry);
    // A thunk has no statement of its own, so the `(...)` is what
    // `Lowering::frame_label` and a throw location render — the site the
    // reader wrote, rather than the file's first line.
    low.cur_stmt_span = *span;

    let (self_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    let mut env = Env::default();
    env.insert(FN_SELF.to_owned(), (self_v, Ty::Object));
    let mut param_tys = vec![Ty::Object];

    // Everything the target is handed is staged as an owned temporary first,
    // so a class check's refusal below releases the whole argument list
    // rather than the prefix bound so far. Which way it is *un*staged is what
    // the calling conventions differ in, at the bottom.
    let mark = low.temporaries_mark();
    let receiver = takes_receiver.then(|| {
        let (v, _) = low.emit(
            entry,
            Ty::Object,
            InstKind::FieldGet {
                object: self_v,
                class: class.clone(),
                field: FCC_RECV.to_owned(),
            },
        );
        // A field read borrows: the object keeps its own reference, so the
        // one the callee will release has to be a new one.
        low.emit_retain(entry, v);
        low.own_temporary(v);
        v
    });

    let mut args = Vec::with_capacity(params.len());
    let mut class_checks: Vec<(usize, ValueId, String)> = Vec::new();
    for (i, ty) in params.iter().enumerate() {
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let (v, _) = low.emit(entry, *ty, InstKind::Param(index));
        if let Some(class) = checked_class(call.param_tys[i], checked_types) {
            class_checks.push((i, v, class));
        }
        if ty.is_refcounted() {
            low.own_temporary(v);
        }
        args.push(v);
        param_tys.push(*ty);
    }
    // The same guarantee a closure literal's own parameters get, and for the
    // same reason: `FN_PARAM_TAGS` has four bits per parameter and no room
    // for a class label, so a named-class parameter is checked here or not at
    // all. See `check_param_class`.
    for (i, value, class) in class_checks {
        cur = check_param_class(&mut low, cur, i, value, &class, *span, &mut env);
    }

    let value = if let Some(symbol) = nvs_types::core_symbol_of(&call.class, &call.method) {
        // A Tier 0 `Core` member borrows every argument, receiver included,
        // so what this frame staged it also releases — see `InstKind::CoreCall`.
        let written = nvs_types::core_takes_written_class(&call.class.to_string(), &call.method)
            .then(|| low.written_type_constants(cur, call));
        // A producer reached through a callable reference is handed the zero
        // word rather than this thunk's own position: the record is produced
        // wherever the callable is later invoked, and the line that wrote the
        // reference is not that place. `rule:errors/a-record-names-where-it-was-produced`
        // omits the field rather than naming somewhere the program did not
        // produce a record.
        let source =
            nvs_types::core_takes_source(&call.class.to_string(), &call.method).then(|| {
                low.emit(cur, Ty::ClassDesc, InstKind::SourceConst { source: None })
                    .0
            });
        // A member on `nvs_stdlib::registry::CALL_SITE_MEMBERS` reached through
        // a callable reference is inside no class, for the reason the producer
        // above is handed the zero word: the act happens wherever the callable
        // is later invoked, and the line that wrote the reference is not that
        // place. Outside is the answer that fails closed, which is what makes
        // the zero word right here rather than merely convenient.
        let site =
            nvs_types::core_takes_call_site(&call.class.to_string(), &call.method).then(|| {
                low.emit(cur, Ty::ClassDesc, InstKind::SourceConst { source: None })
                    .0
            });
        let args = source
            .into_iter()
            .chain(written.into_iter().flatten())
            .chain(receiver)
            .chain(args)
            .chain(site)
            .collect::<Vec<_>>();
        let (v, _) = low.emit_fallible(cur, ret, InstKind::CoreCall { symbol, args }, &env);
        low.release_temporaries_since(mark, cur);
        v
    } else {
        let target = format!("{}::{}", call.class, call.method);
        let kind = match receiver {
            // An instance target dispatches on the receiver's own class
            // wherever the label would name the wrong function — the rule
            // `Lowering::lower_method_call` states in full.
            Some(receiver) if !call.has_body || call.overridden => {
                let (lsb, _) = low.emit(
                    cur,
                    Ty::ClassDesc,
                    InstKind::ClassDescOf { object: receiver },
                );
                InstKind::CallVirtual {
                    lsb,
                    method: call.method.clone(),
                    fallback: call.has_body.then_some(target),
                    receiver: Some(receiver),
                    args,
                }
            }
            Some(receiver) => InstKind::Call {
                target,
                receiver: Some(receiver),
                args,
            },
            // A static target's slot 0 carries the called class, as it does
            // at an ordinary call site. `static_class` is the class the site
            // *wrote*; the declaring class stands in for the forwarding
            // spellings, which is this function's stated divergence.
            None => {
                let called = call
                    .static_class
                    .as_ref()
                    .map_or_else(|| call.class.to_string(), ToString::to_string);
                let (desc, _) = low.emit(
                    cur,
                    Ty::ClassDesc,
                    InstKind::ClassDescConst { class: called },
                );
                if call.has_body {
                    InstKind::Call {
                        target,
                        receiver: Some(desc),
                        args,
                    }
                } else {
                    InstKind::CallVirtual {
                        lsb: desc,
                        method: call.method.clone(),
                        fallback: None,
                        receiver: None,
                        args,
                    }
                }
            }
        };
        // From here the callee owns every transferred argument and releases
        // them on its own throwing edge, so they leave this frame's stack
        // before the call's fault edge is built.
        low.forget_transferred_since(mark);
        let (v, _) = low.emit_fallible(cur, ret, kind, &env);
        v
    };

    low.release_all_locals(cur, &env, None);
    // A `void` target defines no value to return — see this function's own
    // doc comment for why the caller still sees `null`.
    low.seal(cur, Terminator::Return((ret != Ty::Void).then_some(value)));

    let (blocks, stmt_spans, edge_spans) = low.finish();
    (
        Function {
            name: label,
            params: param_tys,
            ret,
            blocks,
            entry,
            stmt_spans,
            edge_spans,
        },
        crate::ir::Class {
            label: class.clone(),
            // `FN_ARITY` first and `FN_PARAM_TAGS` second, as for every
            // closure — a native caller reads both by index — then
            // `FN_PARAM_NAMES`, which only this kind of closure has, and
            // `FCC_RECV` last. Nothing reads the receiver by index, so where
            // it lands is a comment's problem and not a reader's; see those
            // constants.
            fields: [
                FN_ARITY.to_owned(),
                FN_PARAM_TAGS.to_owned(),
                FN_PARAM_NAMES.to_owned(),
            ]
            .into_iter()
            .chain(takes_receiver.then(|| FCC_RECV.to_owned()))
            .collect(),
            field_reprs: Vec::new(),
            secret_fields: Vec::new(),
            public_fields: Vec::new(),
            field_types: Vec::new(),
            // A first-class callable is a closure, so it carries the same
            // edges an `fn` literal's class does — the one every closure has
            // and one per written signature it satisfies, read back at the
            // span the `(...)` was written at.
            conforms: std::iter::once(super::CLOSURE_MARKER.to_owned())
                .chain(exprs.callable_markers_at(*span).iter().cloned())
                .collect(),
            methods: vec![(FN_INVOKE.to_owned(), class.clone(), true)],
            // Nor a property to hook: every slot is a capture.
            hooks: Vec::new(),
            codec: Vec::new(),
            db_codec: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
        },
    )
}

/// One closure parameter's lowered type, the class it must be checked against
/// where it names one, and the span a failure points at.
///
/// A parameter that wrote its type is read off that annotation, exactly as a
/// declaration's is. One that left it out took its type from the position the
/// literal was written in (`rule:types/callable-literal-inference`), and
/// `nvs_types::expr::calls::check_fn_literal` recorded the answer under the
/// parameter's own name — the only span an unannotated parameter has. The
/// checker refuses the literal outright where it could not answer, so an
/// unrecorded one here is a bug in that pass rather than a program.
fn closure_param_ty(
    p: &nvs_syntax::ast::Param,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> (Ty, Option<String>, Span) {
    match &p.ty {
        Some(decl_ty) => (
            lower_decl_type(decl_ty, exprs, checked_types),
            declared_class(decl_ty, exprs, checked_types),
            decl_ty.span,
        ),
        None => {
            let id = exprs.declared_ty(p.name).unwrap_or_else(|| {
                panic!(
                    "`rule:types/callable-literal-inference`: an unannotated closure parameter \
                     carries the type the checker inferred for it, recorded under its name"
                )
            });
            (
                erase_checked_ty(id, checked_types),
                checked_class(id, checked_types),
                p.name,
            )
        }
    }
}

/// [`declared_class`]'s question asked of an *already-checked* type rather
/// than of a `Type` AST node — a resolved call's parameter type, which is a
/// [`TypeId`] out of `nvs_types`' own interner and has no local declaration
/// this crate could read instead. Same answer, same `Core` exclusion, same
/// reason.
fn checked_class(id: TypeId, checked_types: &TypeInterner) -> Option<String> {
    match checked_types.get(id) {
        CheckedTy::Class(qname, _) if !qname.is_core() => Some(qname.to_string()),
        _ => None,
    }
}
