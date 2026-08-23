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

/// Lowers every pending closure, and every closure *those* bodies contain, to
/// exhaustion.
pub(super) fn drain_closures(
    mut pending: Vec<PendingClosure>,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> (Vec<Function>, Vec<crate::ir::Class>) {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    while let Some(next) = pending.pop() {
        let (function, class, more) = lower_closure(&next, src, exprs, checked_types);
        functions.push(function);
        classes.push(class);
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
pub(super) fn lower_closure(
    pending: &PendingClosure,
    src: &SourceFile,
    exprs: &ExprTypeTable,
    checked_types: &TypeInterner,
) -> (Function, crate::ir::Class, Vec<PendingClosure>) {
    let PendingClosure {
        class,
        fn_expr,
        captures,
        ret,
    } = pending;
    let label = format!("{class}::{FN_INVOKE}");
    let mut low = Lowering::new(&label, src, *ret, exprs, checked_types);
    let entry = low.new_block();
    let mut cur = entry;
    low.emit_safepoint(entry);

    let (self_v, _) = low.emit(entry, Ty::Object, InstKind::Param(0));
    let mut env = Env::default();
    env.insert(FN_SELF.to_owned(), (self_v, Ty::Object));
    let mut param_tys = vec![Ty::Object];

    // The captures first, so a parameter of the same name — which shadows one,
    // per `mwl_types::expr::check_fn_literal` — overwrites it rather than the
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

    for (i, p) in fn_expr.params.iter().enumerate() {
        assert!(
            !p.by_ref,
            "mwl-ir does not lower a closure with a `&$x` parameter: nothing calls a closure \
             through a signature yet, so there is no call site to stage the cell at; see the \
             crate docs' known gaps"
        );
        let decl_ty =
            p.ty.as_ref()
                .unwrap_or_else(|| panic!("ADR 0007 § 1: every parameter has a declared type"));
        let ty = lower_decl_type(decl_ty, exprs, checked_types);
        let index = u32::try_from(i + 1).expect("far more parameters than a call could ever take");
        let pname = strip_sigil(span_text(src, p.name)).to_owned();
        let (v, _) = low.emit(entry, ty, InstKind::Param(index));
        env.insert(pname, (v, ty));
        param_tys.push(ty);
    }

    match &fn_expr.body {
        // An expression body is an implicit `return` (ADR 0031 § 1), lowered
        // through the same path `StmtKind::Return` uses: retain if the value
        // is a borrowed read, release the frame's locals, return.
        FnBody::Expr(body) => {
            let (v, ty) = low.lower_expr(body, Some(*ret), &env, cur);
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
        crate::ir::Class {
            label: class.clone(),
            // `FN_ARITY` first, always — a native caller reads it by index,
            // not by name. See that constant.
            fields: std::iter::once(FN_ARITY.to_owned())
                .chain(captures.iter().map(|(n, _)| n.clone()))
                .collect(),
            conforms: Vec::new(),
            methods: vec![(FN_INVOKE.to_owned(), class.clone())],
        },
        more,
    )
}
