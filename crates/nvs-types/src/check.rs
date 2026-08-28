//! Entry point: walks a resolved [`Module`]'s classes and methods, type-
//! checking each method body against ADR 0007 §§ 1-4 (see the crate docs for
//! the exact scope of this slice), plus the file's own top-level statements
//! as one synthesized frame ([`ScriptFrame`]) — ADR 0008 § 2's "the script
//! body is a function, so its variables are locals". That frame is threaded
//! across `namespace { ... }` blocks, since a namespace scopes names rather
//! than storage, and it is entirely separate from every method's own frame:
//! a file-scope local is unreachable from a function, exactly as that ADR's
//! storage-class table says.
//!
//! Mirrors [`nvs_hir::members`]'s own walk shape: [`check_stmts`] tracks
//! namespace/`use` scope the same way (there is no enclosing-class scope to
//! track at this level — a fresh [`Ctx`] naming the class is built right at
//! each declaration site instead), recursing into each class/interface's
//! methods via [`check_members`]. A method with no body (abstract, or an
//! interface signature) has nothing to check. [`check_method`] seeds a fresh
//! [`crate::locals::LocalScope`] from the method's own lowered parameters
//! (already definitely assigned), lowers its return type once, and hands the
//! body to [`crate::locals::check_block`]. Right after a `ClassDecl`'s
//! members are checked this way, [`crate::ctor_init::check_class_init`] runs
//! its own, separate constructor-only pass over the same declaration for
//! ADR 0022 § 2, and [`crate::lateinit::check_class_lateinit_reads`] runs
//! ADR 0038 § 3's sibling pass over every one of that declaration's *other*
//! methods too — interfaces/enums never get either call, since only a class
//! is ever instantiated through a constructor.
//!
//! **Known gap:** a class/interface/enum declared *inside* a method body is
//! not descended into here at all — only top-level declarations (and ones
//! nested in a `namespace { ... }` block) are found by [`check_stmts`].

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, code};
use nvs_hir::{Module, QName};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, MethodMember, Name, NamespaceDecl, Stmt, StmtKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ctor_init::check_class_init;
use crate::expr::class_of_ctx;
use crate::expr_table::ExprTypeTable;
use crate::lateinit::check_class_lateinit_reads;
use crate::locals::{LocalScope, check_block, check_stmt};
use crate::lower::lower_optional_type;
use crate::signatures::build_signatures;
use crate::ty::TypeInterner;
use crate::{Ctx, Env, span_text, strip_sigil};

fn qname_segments(src: &SourceFile, name: &Name) -> Vec<String> {
    QName::parse(span_text(src, name.span)).segments().to_vec()
}

/// Type-checks every method body reachable from `files`, using the already
/// name-resolved `module` for symbol/alias lookups. `interner` accumulates
/// every type this run interns; `exprs` accumulates every call's/`new`'s
/// resolved target this run records — see [`crate::expr_table`]'s own module
/// docs; a caller with no use for it yet (today, only `nvs-ir` reads it back)
/// still passes one and may simply drop it afterward.
///
/// `files` is the whole `require`/`autoload` graph — `nvs_hir::resolve_program`'s
/// second return value, in entry-first load order, mapped to
/// [`crate::ProgramFile`] — and `module` must be the one that walk resolved
/// over the same set. Every table below is built across all of it before any
/// body is checked, because a class declared in one file is referenced from
/// another; only the per-file phases below are actually per file.
///
/// **Each file gets its own [`ScriptFrame`].** ADR 0008 § 2 makes a file's
/// top-level statements a function body, and a file is where that body ends:
/// `$x` at the top of the entry file and `$x` at the top of a `require`d one
/// are two locals of two frames, so neither the declare-once rule nor
/// definite assignment reaches across the boundary.
///
/// Hands the [`crate::EnumTable`] it built back rather than dropping it, for
/// the same reason `nvs_hir::resolve_program` hands its autoload map back:
/// `nvs-ir` needs each case's constant value to lower ADR 0047 § 3's
/// enum-case membership test, and [`crate::enums::build_enum_table`] reports
/// ADR 0010 § 1/§ 2's declaration errors, so a caller that rebuilt the table
/// for itself would report every one of them twice. A caller with no use for
/// it drops it, exactly as it may drop `exprs`.
pub fn check_program(
    files: &[crate::ProgramFile<'_>],
    module: &Module,
    interner: &mut TypeInterner,
    exprs: &mut ExprTypeTable,
    diags: &mut Diagnostics,
) -> crate::EnumTable {
    // ADR 0010 § 2's backing types first: interning an enum-typed annotation
    // needs one, and `build_signatures` interns every declared annotation in
    // the program. See `crate::enums`.
    let enums = crate::enums::build_enum_table(files, diags);
    // ADR 0047 § 2's fold, on the same terms and for the same reason as the
    // enum table one line above: `build_signatures` interns every declared
    // annotation, and one of them may be a `Foo::CONST` type.
    let consts = crate::consts::build_const_table(files);
    let signatures = build_signatures(files, module, &enums, &consts, interner, diags);
    // ADR 0046 § 4's retrieval reads this. Built whole, ahead of the walk, for
    // `build_const_table`'s reason: a retrieval may be written above the
    // declaration it asks about, so a table filled as the walk descends would
    // answer differently depending on source order.
    let attributes = crate::retrieval::build_attribute_table(files);
    // Threaded across the files rather than restarted at each: an ADR 0031
    // closure literal at file scope is labelled `Script$fn<n>`, with no
    // declaring class to disambiguate it, so a counter that restarted per
    // file would give two files' first closures the same synthesized class.
    let mut closure_seq = 0;
    for file in files {
        let mut env = Env {
            symbols: &module.symbols,
            aliases: &module.aliases,
            graph: &module.graph,
            signatures: &signatures,
            enums: &enums,
            consts: &consts,
            attributes: &attributes,
            src: file.src,
            interner: &mut *interner,
            exprs: &mut *exprs,
            diags: &mut *diags,
            closure_seq,
            exit_targets: Vec::new(),
            write_target_levels: FxHashMap::default(),
            coalesce_guarded: FxHashSet::default(),
        };
        let mut frame = ScriptFrame {
            scope: LocalScope::new(),
            live: FxHashSet::default(),
            // ADR 0021 § 3: `require`'s value is what a `return`-ing target
            // file hands back, typed `mixed` at the boundary — so the
            // synthesized frame's return type is `mixed`, not `void`.
            return_ty: env.interner.mixed(),
        };
        check_stmts(file.stmts, &[], &FxHashMap::default(), &mut frame, &mut env);
        closure_seq = env.closure_seq;
    }
    record_property_types(&signatures, exprs);
    enums
}

/// Copies every class's own declared property types into the expression
/// table, so `nvs_ir::lower` can join them against the flattened slot order
/// and hand each class's per-slot types to codegen — ADR 0036 § 4's erased
/// **write** check, which is the one write site that has no statically known
/// field type of its own and so must ask the receiver's concrete class at run
/// time.
///
/// Driven off the signature table rather than off the checked files, unlike
/// [`record_property_defaults`]: the classes with no source declaration —
/// `nvs_hir::errors`' exception tree, and every installed `Core` class — hold
/// state an erased write can reach just as well as a user class's, and their
/// declarations are only ever in here.
fn record_property_types(signatures: &crate::SignatureTable, exprs: &mut ExprTypeTable) {
    for (qname, sig) in signatures.iter() {
        if sig.properties.is_empty() {
            continue;
        }
        let mut types: Vec<(String, crate::ty::TypeId)> = sig
            .properties
            .iter()
            .map(|(name, ty)| (name.clone(), *ty))
            .collect();
        types.sort_by(|a, b| a.0.cmp(&b.0));
        exprs.record_property_types(qname.to_string(), types);
    }
}

/// The one synthesized frame a file's top-level statements share
/// (ADR 0008 § 2: "the script body is a function, so its variables are
/// locals"). Threaded through [`check_stmts`] so that a `namespace { ... }`
/// block's own top-level statements land in the *same* frame as the ones
/// outside it — a namespace scopes names, not storage.
struct ScriptFrame {
    scope: LocalScope,
    live: FxHashSet<String>,
    return_ty: crate::ty::TypeId,
}

fn check_stmts(
    stmts: &[Stmt],
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    frame: &mut ScriptFrame,
    env: &mut Env<'_>,
) {
    let mut current_ns: Vec<String> = namespace.to_vec();
    let mut current_imports: FxHashMap<String, QName> = imports.clone();

    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name
                    .as_ref()
                    .map_or_else(Vec::new, |n| qname_segments(env.src, n));
                match body {
                    Some(block) => {
                        check_stmts(&block.stmts, &new_ns, &FxHashMap::default(), frame, env);
                    }
                    None => {
                        current_ns = new_ns;
                        current_imports.clear();
                    }
                }
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(span_text(env.src, use_decl.path.span));
                current_imports.insert(target.short_name().to_owned(), target);
            }
            StmtKind::ClassDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                    generator_elem: None,
                };
                check_members(&decl.members, &ctx, env);
                crate::attributes::check_declaration(
                    &decl.attributes,
                    &decl.members,
                    &[],
                    &ctx,
                    env,
                );
                check_class_init(decl, &qname, env);
                check_class_lateinit_reads(decl, &qname, env);
                crate::conformance::check_class_conformance(decl, &qname, env);
                crate::derive::check_class_derive(decl, &qname, &ctx, env);
                record_property_defaults(&qname, env);
                record_lateinit_properties(&qname, env);
                record_static_properties(&qname, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                    generator_elem: None,
                };
                check_members(&decl.members, &ctx, env);
                crate::attributes::check_declaration(
                    &decl.attributes,
                    &decl.members,
                    &[],
                    &ctx,
                    env,
                );
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                    generator_elem: None,
                };
                check_members(&decl.members, &ctx, env);
                crate::attributes::check_declaration(
                    &decl.attributes,
                    &decl.members,
                    &decl.cases,
                    &ctx,
                    env,
                );
            }
            // Two more declarations this walk has nothing to check, matched
            // here rather than left to fall through, because what
            // `crate::locals::check_stmt` does with one now is refuse it as
            // `E0233` — and there the fact that it arrived at all *is* the
            // proof it was nested. `nvs_hir` is what reads both: a `type`
            // alias into the type table, an `autoload` into ADR 0061's map.
            StmtKind::TypeAliasDecl(_) | StmtKind::AutoloadDecl(_) => {}
            // Everything else is a *statement* of the script body, not a
            // declaration: one synthesized frame for the whole file, whose
            // variables are ordinary locals (ADR 0008 § 2). Reuses
            // `check_stmt` verbatim rather than adding a second walk, so a
            // top-level `echo $missing;` reports exactly what the same line
            // inside a method reports. `current_class` is `None` — there is
            // no `$this` at file scope.
            _ => {
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: None,
                    current_hook: None,
                    generator_elem: None,
                };
                check_stmt(
                    stmt,
                    &mut frame.live,
                    &mut frame.scope,
                    frame.return_ty,
                    &ctx,
                    env,
                );
            }
        }
    }
}

/// Copies `qname`'s own evaluated property defaults from the signature table
/// into the expression table, under the same class label `nvs_types::layout`
/// keys a layout by.
///
/// A move of already-computed data rather than a check: `crate::signatures`
/// evaluated and diagnosed each one when it collected the declaration, and
/// `nvs-ir` is handed the expression table alone — see
/// [`crate::expr_table::ExprTypeTable::record_property_defaults`].
fn record_property_defaults(qname: &QName, env: &mut Env<'_>) {
    let Some(sig) = env.signatures.get(qname) else {
        return;
    };
    if sig.property_defaults.is_empty() {
        return;
    }
    let defaults = sig.property_defaults.clone();
    env.exprs
        .record_property_defaults(qname.to_string(), defaults);
}

/// The same move for the class's `lateinit` properties (ADR 0038 § 1), which
/// `nvs-ir` needs for ADR 0022 § 3's never-written storage state — see
/// [`crate::expr_table::ExprTypeTable::record_lateinit_properties`].
///
/// **Flattened, unlike [`record_property_defaults`]'s own-only entry**, and
/// that is the one thing to get right here: a property access records the
/// class the *receiver* was typed as
/// ([`crate::expr::members::check_property_member`]), not the one that
/// declared the property, so a subclass label has to answer for what it
/// inherited or an inherited `lateinit` read would be guarded on the parent
/// and unguarded on every child. That is the same direction `nvs_runtime`'s
/// own layout decision takes — a slot index computed against a base class is
/// valid for every subclass — rather than a second lookup rule.
fn record_lateinit_properties(qname: &QName, env: &mut Env<'_>) {
    let mut names: Vec<String> = Vec::new();
    let mut pending = vec![qname.clone()];
    let mut seen: Vec<QName> = Vec::new();
    while let Some(current) = pending.pop() {
        if seen.contains(&current) {
            continue;
        }
        if let Some(sig) = env.signatures.get(&current) {
            names.extend(sig.lateinit_properties.iter().cloned());
        }
        if let Some(links) = env.graph.get(&current) {
            pending.extend(links.extends.iter().cloned());
            pending.extend(links.implements.iter().cloned());
        }
        seen.push(current);
    }
    if names.is_empty() {
        return;
    }
    names.dedup();
    env.exprs
        .record_lateinit_properties(qname.to_string(), names);
}

/// The same move for the class's own `static` properties — see
/// [`crate::expr_table::ExprTypeTable::record_static_properties`], which is
/// what `nvs-ir` enumerates the program's static slots out of.
fn record_static_properties(qname: &QName, env: &mut Env<'_>) {
    let Some(sig) = env.signatures.get(qname) else {
        return;
    };
    if sig.static_properties.is_empty() {
        return;
    }
    let statics = sig.static_properties.clone();
    env.exprs
        .record_static_properties(qname.to_string(), statics);
}

fn check_members(members: &[ClassMember], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => check_method(m, ctx, env),
            ClassMemberKind::Property(p) => check_property_hooks(p, ctx, env),
            _ => {}
        }
    }
}

/// Type-checks each of `p`'s ADR 0014 § 1 hook bodies as its own frame, and
/// records the label the compiled accessor is emitted under.
///
/// A hook is an ordinary function in every respect that matters here: it has
/// an implicit `$this`, its own locals, and a return type — the property's
/// own type for `get`, `void` for `set`. `set`'s parameter is the one thing
/// with no method equivalent: PHP 8.4 lets it be written explicitly
/// (`set(string $v)`) or left implicit, in which case it is named `$value`
/// and typed as the property. The short `=> expr;` body form differs by
/// accessor the same way PHP's does — for `get` the expression is the value
/// returned, for `set` it is the value stored — so only `get` checks it
/// against the property's type here; `set`'s is checked at the assignment
/// [`crate::expr::assign::check_assign`] already performs for the desugared store.
///
/// [`Ctx::current_hook`] is what stops `$this->p` inside `$p`'s own hooks
/// from resolving to a re-entrant call to the very accessor being checked.
fn check_property_hooks(p: &nvs_syntax::ast::PropertyMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let Some(hooks) = &p.hooks else { return };
    let Some(class) = ctx.current_class else {
        return;
    };
    let name = strip_sigil(span_text(env.src, p.name)).to_owned();
    let prop_ty = crate::lower::lower_type(&p.ty, ctx, env);
    let inner = Ctx {
        namespace: ctx.namespace,
        imports: ctx.imports,
        current_class: ctx.current_class,
        current_hook: Some(&name),
        generator_elem: None,
    };
    for hook in hooks {
        env.exprs.record_method(
            hook.span,
            crate::signatures::hook_label(class, &name, hook.kind),
        );
        let Some(body) = &hook.body else {
            continue; // abstract hook — a requirement, not code
        };
        let mut scope = LocalScope::new();
        let mut live: FxHashSet<String> = FxHashSet::default();
        let this_ty = class_of_ctx(&inner, env);
        scope.declare_param("this".to_owned(), this_ty, p.name);
        live.insert("this".to_owned());
        let is_set = hook.kind == nvs_syntax::ast::PropertyHookKind::Set;
        if is_set {
            let (pname, pspan, pty) = match &hook.param {
                Some(param) => (
                    strip_sigil(span_text(env.src, param.name)).to_owned(),
                    param.name,
                    lower_optional_type(param.ty.as_ref(), &inner, env),
                ),
                None => (HOOK_VALUE_PARAM.to_owned(), p.name, prop_ty),
            };
            scope.declare_param(pname.clone(), pty, pspan);
            live.insert(pname);
        }
        let return_ty = if is_set { env.interner.void() } else { prop_ty };
        match body {
            nvs_syntax::ast::PropertyHookBody::Expr(e) => {
                let expected = if is_set { prop_ty } else { return_ty };
                crate::expr::check_expr(e, Some(expected), &mut live, &scope, &inner, env);
            }
            nvs_syntax::ast::PropertyHookBody::Block(block) => {
                check_block(&block.stmts, &mut live, &mut scope, return_ty, &inner, env);
            }
        }
    }
}

/// The name a `set` hook's parameter binds under when the declaration leaves
/// it implicit (`set => $this->x = $value;`) — PHP 8.4's own spelling.
pub const HOOK_VALUE_PARAM: &str = "value";

fn check_method(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    // Recorded before the abstract/interface early return: a declaration with
    // no body still has a label, and nothing here needs a body to spell one.
    // See `ExprTypeTable::method_label` for why the definition side of the
    // label is recorded at all.
    if let Some(class) = ctx.current_class {
        env.exprs
            .record_method(m.name, format!("{class}::{}", span_text(env.src, m.name)));
    }

    let Some(body) = &m.body else {
        return; // abstract method or interface signature — nothing to check
    };

    let mut scope = LocalScope::new();
    let mut live: FxHashSet<String> = FxHashSet::default();
    if ctx.current_class.is_some() {
        // Seeded here rather than as an ordinary parameter: `$this` has no
        // `Param` node of its own to read a span from, and property/method
        // access on it (`crate::expr`) needs its type to be `self`'s class
        // the same way an explicit `new Foo()` result is. Not gated on a
        // `static` modifier — a static method's own body referencing `$this`
        // is a distinct, unrelated diagnostic this slice doesn't add.
        let this_ty = class_of_ctx(ctx, env);
        scope.declare_param("this".to_owned(), this_ty, m.name);
        live.insert("this".to_owned());
    }
    for param in &m.params {
        let ty = lower_optional_type(param.ty.as_ref(), ctx, env);
        // `...$rest` declares the type of *each* trailing argument — which is
        // what `MethodSig::param_at` matches an argument against — but the
        // body is handed the one array the call site collected them into, so
        // the binding is `array<` that `>`. Getting this wrong is not a
        // checker-only mistake: `nvs_ir::lower::lower_method` binds the same
        // slot, and the caller has always passed an array there.
        let ty = match param.variadic {
            true => env.interner.array(ty),
            false => ty,
        };
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        scope.declare_param(name.clone(), ty, param.name);
        live.insert(name);
    }
    let return_ty = lower_optional_type(m.return_type.as_ref(), ctx, env);

    // ADR 0053 § 4: a body containing `yield` is a generator, and everything
    // that follows from that is decided here rather than at each `yield` —
    // the declared return type must be `Iterator<T>`, and `T` is what every
    // `yield` operand in the body is checked against.
    let Some(elem) = generator_element(m, body, return_ty, ctx, env) else {
        check_block(&body.stmts, &mut live, &mut scope, return_ty, ctx, env);
        return;
    };
    check_generator_inout_params(m, env);
    let inner = Ctx {
        namespace: ctx.namespace,
        imports: ctx.imports,
        current_class: ctx.current_class,
        current_hook: ctx.current_hook,
        generator_elem: Some(elem),
    };
    // A generator's body returns nothing: calling it produced the cursor, and
    // ADR 0053 § 5 leaves no return value to retrieve. So the body is checked
    // against `void` — which is what makes `return $x;` inside one report
    // `E0447` from `crate::expr::check_return` rather than a mismatch against
    // the `Iterator<T>` the *declaration* names.
    let void = env.interner.void();
    check_block(&body.stmts, &mut live, &mut scope, void, &inner, env);
}

/// ADR 0053 § 4's frame lifetime, as a refusal: a generator declares no `inout $x`
/// parameter.
///
/// A by-reference parameter addresses a cell the **call site** stages, writes
/// back from and then drops — `nvs_ir::lower::call` owns that staging, and
/// what makes it sound is that the callee's frame dies first. A generator
/// inverts exactly that: calling one runs none of the body, it allocates the
/// state object and returns, so the staged cell is gone before the first
/// `advance()` and the parked frame would be addressing a slot of a call that
/// has already returned. There is no representation for it to keep instead —
/// copying the value in would silently stop being a reference, and the whole
/// observable point of `inout $x` is that the caller sees the writes.
///
/// So it is a shape the language does not have, refused where it is written.
/// Reported once per by-reference parameter, and only for a body that already
/// established itself as a generator, so an ordinary method's `inout $x` — which
/// lowers — is untouched.
fn check_generator_inout_params(m: &MethodMember, env: &mut Env<'_>) {
    for param in m.params.iter().filter(|p| p.inout) {
        let name = span_text(env.src, param.name).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_GENERATOR_INOUT_PARAM,
                format!("a generator cannot take `{name}` as `inout`"),
            )
            .with_primary(param.name, "declared `inout` here")
            .with_help(
                "ADR 0053 § 4: calling a generator returns the state object without running \
                 the body, so the caller's cell is gone before the first `advance()` — take \
                 the value by copy and `yield` what the body computes from it",
            ),
        );
    }
}

/// ADR 0053 § 4's `T`, for a method whose body makes it a generator —
/// `None` for an ordinary method, and `None` (after a diagnostic) for a
/// generator whose declared return type is not an `Iterator<T>`.
fn generator_element(
    m: &MethodMember,
    body: &nvs_syntax::ast::Block,
    return_ty: crate::ty::TypeId,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<crate::ty::TypeId> {
    if !nvs_syntax::ast::is_generator_body(body) {
        return None;
    }
    if let crate::ty::Ty::Class(qname, args) = env.interner.get(return_ty)
        && qname.short_name() == nvs_hir::interfaces::ITERATOR
        && qname.is_reserved_global_interface()
        && let Some(&elem) = args.first()
    {
        return Some(elem);
    }
    let got = env.interner.describe(return_ty);
    let span = m.return_type.as_ref().map_or(m.name, |t| t.span);
    let name = span_text(env.src, m.name);
    env.diags.report(
        Diagnostic::error(
            code::E_GENERATOR_RETURN_TYPE,
            format!("`{name}` contains `yield`, so it must return an `Iterator<T>`"),
        )
        .with_primary(span, format!("this declares `{got}`"))
        .with_help(
            "ADR 0053 § 4: calling a generator runs no user code — it allocates and returns \
             the state object, which implements `Iterator<T>`",
        ),
    );
    let _ = ctx;
    // Still a generator, with an unchecked element type: `mixed` accepts
    // every `yield` operand, so one wrong return type reports once instead of
    // once plus a stray-`yield` diagnostic per `yield` in the body.
    Some(env.interner.mixed())
}
