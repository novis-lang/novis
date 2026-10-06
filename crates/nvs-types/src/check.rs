//! Entry point: walks a resolved [`Module`]'s classes and methods, type-
//! checking each method body against `rule:types/declaration`, `rule:types/conversion`, `rule:types/grammar` and `rule:types/arithmetic` (see the crate docs for
//! the exact scope of this slice), plus the file's own top-level statements
//! as one synthesized frame ([`ScriptFrame`]) — `rule:statements/storage-that-outlives-a-call`'s "the script
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
//! `rule:classes/definite-property-initialization`, and [`crate::lateinit::check_class_lateinit_reads`] runs
//! `rule:classes/lateinit-read-before-write`'s sibling pass over every one of that declaration's *other*
//! methods too — interfaces/enums never get either call, since only a class
//! is ever instantiated through a constructor.
//!
//! **A declaration inside a method body is refused rather than checked here.**
//! [`check_stmts`] finds a top-level `class`/`interface`/`enum` and one nested
//! in a `namespace { ... }` block, and nothing else. One written inside a body
//! reaches [`crate::locals`]'s walk instead, where arriving is proof of nesting
//! and its `nested_declaration` reports `E0233` on the spot — so there is
//! nothing under a body left for this pass to descend into.

use nvs_diagnostics::{BytePos, Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_hir::{Module, QName};
use nvs_syntax::ast::{
    Block, ClassMember, ClassMemberKind, Expr, ExprKind, MemberName, MethodMember, Modifier, Name,
    NamespaceDecl, NewTarget, Stmt, StmtKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ctor_init::check_class_init;
use crate::expr::class_of_ctx;
use crate::expr_table::{ExprTypeTable, LocalBinding};
use crate::lateinit::check_class_lateinit_reads;
use crate::locals::{Live, LocalScope, check_block, check_stmt};
use crate::lower::lower_optional_type;
use crate::returns::{block_always_exits, closing_brace};
use crate::signatures::build_signatures;
use crate::ty::{Ty, TypeInterner};
use crate::{Ctx, Env, TypeId, span_text, strip_sigil};

fn qname_segments(src: &SourceFile, name: &Name) -> Vec<String> {
    QName::parse(span_text(src, name.span)).segments().to_vec()
}

/// Type-checks every method body reachable from `files`, using the already
/// name-resolved `module` for symbol/alias lookups. `interner` accumulates
/// every type this run interns; `exprs` accumulates every call's/`new`'s
/// resolved target this run records — see [`crate::expr_table`]'s own module
/// docs; a caller with no use for it (only `nvs-ir` reads it back) still
/// passes one and may simply drop it afterward.
///
/// `files` is the whole `require`/`autoload` graph — `nvs_hir::resolve_program`'s
/// second return value, in entry-first load order, mapped to
/// [`crate::ProgramFile`] — and `module` must be the one that walk resolved
/// over the same set. Every table below is built across all of it before any
/// body is checked, because a class declared in one file is referenced from
/// another; only the per-file phases below are actually per file.
///
/// **Each file gets its own [`ScriptFrame`].** `rule:statements/storage-that-outlives-a-call` makes a file's
/// top-level statements a function body, and a file is where that body ends:
/// `$x` at the top of the entry file and `$x` at the top of a `require`d one
/// are two locals of two frames, so neither the declare-once rule nor
/// definite assignment reaches across the boundary.
///
/// Hands the [`crate::EnumTable`] it built back rather than dropping it, for
/// the same reason `nvs_hir::resolve_program` hands its autoload map back:
/// `nvs-ir` needs each case's constant value to lower `rule:types/enum-case-type`'s
/// enum-case membership test, and [`crate::enums::build_enum_table`] reports
/// `rule:enums/declaration`/§ 2's declaration errors, so a caller that rebuilt the table
/// for itself would report every one of them twice. A caller with no use for
/// it drops it, exactly as it may drop `exprs`.
pub fn check_program(
    files: &[crate::ProgramFile<'_>],
    module: &Module,
    interner: &mut TypeInterner,
    exprs: &mut ExprTypeTable,
    diags: &mut Diagnostics,
) -> crate::EnumTable {
    check_program_granted(files, module, None, interner, exprs, diags)
}

/// [`check_program`] with the deployment's `[capabilities]` block in front of
/// it — `rule:security/capability-question-is-grant-and-scope`
/// 's grants, as the machine that is compiling reads them.
///
/// **`None` is "no configuration was read", and it is not an empty grant
/// set.** A capability check made against an absent configuration would refuse
/// every program compiled outside a project root, which is the opposite of
/// what deny-by-default means here: the door is
/// `nvs_runtime::capability::require`, and this pass only ever moves one of
/// that door's refusals earlier (`rule:expressions/preparation-preserves-behaviour`). Every fixture in the tree checks with `None` for that reason, and
/// says nothing about capabilities at all.
///
/// A second entry point rather than another parameter on the first: hardly any
/// call site in the whole workspace has a `Capabilities` to pass, and threading
/// an argument every other harness would spell `None` prices the seam to the
/// callers that do not use it.
pub fn check_program_granted(
    files: &[crate::ProgramFile<'_>],
    module: &Module,
    grants: Option<&nvs_config::tree::Capabilities>,
    interner: &mut TypeInterner,
    exprs: &mut ExprTypeTable,
    diags: &mut Diagnostics,
) -> crate::EnumTable {
    check_program_loaded(files, module, grants, &[], interner, exprs, diags)
}

/// [`check_program_granted`] with the manifests of the loaded extension set,
/// whose classes [`crate::ext_lib`] seeds into the signature table
/// (`rule:packaging/extension-calls-are-statically-typed`).
///
/// `module` must have been resolved with the same set's
/// [`crate::ext_lib::hir_classes`] declared, so a reference to an extension
/// class resolves before this pass types it.
pub fn check_program_loaded(
    files: &[crate::ProgramFile<'_>],
    module: &Module,
    grants: Option<&nvs_config::tree::Capabilities>,
    extensions: &[nvs_ext::manifest::Manifest],
    interner: &mut TypeInterner,
    exprs: &mut ExprTypeTable,
    diags: &mut Diagnostics,
) -> crate::EnumTable {
    // `rule:enums/one-backing-type`'s backing types first: interning an enum-typed annotation
    // needs one, and `build_signatures` interns every declared annotation in
    // the program. See `crate::enums`.
    let enums = crate::enums::build_enum_table(files, extensions, diags);
    // `rule:types/constant-in-type-position`'s fold, on the same terms and for the same reason as the
    // enum table one line above: `build_signatures` interns every declared
    // annotation, and one of them may be a `Foo::CONST` type.
    let consts = crate::consts::build_const_table(files);
    let signatures = build_signatures(files, module, &enums, &consts, extensions, interner, diags);
    // `rule:attributes/structural-retrieval`'s retrieval reads this. Built whole, ahead of the walk, for
    // `build_const_table`'s reason: a retrieval may be written above the
    // declaration it asks about, so a table filled as the walk descends would
    // answer differently depending on source order.
    let attributes = crate::retrieval::build_attribute_table(files);
    let deprecations = crate::deprecated::build_table(files);
    // Threaded across the files rather than restarted at each: an `rule:types/anonymous-function`
    // anonymous function at file scope is labelled `Script$fn<n>`, with no
    // declaring class to disambiguate it, so a counter that restarted per
    // file would give two files' first anonymous functions the same synthesized class.
    let mut anon_fn_seq = 0;
    // `rule:routing/table-is-opt-in`'s rows accumulate across the files rather than per file:
    // `crate::routes::check_table` reports collisions between declarations,
    // and § 5's scan is what brings two files' routes into one program.
    let mut routes = crate::routes::RouteTable::default();
    // `rule:tooling/commands-are-compiled`'s rows, accumulated for exactly that reason: a duplicate
    // command name is a collision between two declarations, and § 6's scan is
    // the same one that brings two files' routes into one program.
    let mut commands = crate::commands::CommandTable::default();
    // § 4's links accumulate beside the rows and are resolved after the loop,
    // never inside it: a `Core\Router::url` in the entry file routinely names a
    // route § 5's scan finds in a later one. See `crate::links`.
    let mut links = Vec::new();
    // `rule:core-classes/derive-field-list`'s field-type question, accumulated for the same reason and
    // resolved in the same place — see `crate::derive::resolve_field_types`.
    let mut codec_sites = Vec::new();
    // `rule:core-classes/db-column-types`'s question about a `queryAs<T>`'s class, accumulated for the
    // same reason and resolved in the same place — see
    // `crate::derive::check_row_sites`.
    let mut row_sites = Vec::new();
    // `rule:core-classes/derive-field-list`'s skipped field asked of the class a
    // document decoder wrote, accumulated for the same reason — see
    // `crate::derive::check_json_sites`.
    let mut json_sites = Vec::new();
    // `rule:security/derived-codec-qualifiers`'s question about the class a
    // `jsonAs<T>` wrote, accumulated for the same reason — see
    // `crate::derive::check_decode_sites`.
    let mut decode_sites = Vec::new();
    for file in files {
        let mut env = Env {
            symbols: &module.symbols,
            aliases: &module.aliases,
            graph: &module.graph,
            signatures: &signatures,
            enums: &enums,
            consts: &consts,
            attributes: &attributes,
            deprecations: &deprecations,
            grants,
            src: file.src,
            stmts: file.stmts,
            files,
            interner: &mut *interner,
            exprs: &mut *exprs,
            routes: &mut routes,
            commands: &mut commands,
            links: &mut links,
            codec_sites: &mut codec_sites,
            row_sites: &mut row_sites,
            json_sites: &mut json_sites,
            decode_sites: &mut decode_sites,
            diags: &mut *diags,
            anon_fn_seq,
            refused_exprs: 0,
            fn_self: None,
            exit_targets: Vec::new(),
            write_target_levels: FxHashMap::default(),
            coalesce_guarded: FxHashSet::default(),
            method_ref_args: FxHashSet::default(),
            body_writers: crate::response::BodyWriters::default(),
            in_call_argument: false,
            deprecated_uses: None,
        };
        let mut frame = ScriptFrame {
            scope: LocalScope::new(),
            live: Live::default(),
            // `rule:statements/a-require-expression-is-mixed`: `require`'s value is what a `return`-ing target
            // file hands back, typed `mixed` at the boundary — so the
            // synthesized frame's return type is `mixed`, not `void`.
            return_ty: env.interner.mixed(),
        };
        check_stmts(file.stmts, &[], &FxHashMap::default(), &mut frame, &mut env);
        // The whole file, because the synthesized frame is the whole file's:
        // `rule:statements/storage-that-outlives-a-call` makes the script body
        // a function, and a `namespace { ... }` block's statements land in this
        // same frame rather than in one of their own.
        let script = Span::new(
            file.src.id(),
            0,
            BytePos::try_from(file.src.text().len()).unwrap_or(BytePos::MAX),
        );
        record_locals(script, &mut frame.scope, &mut env);
        anon_fn_seq = env.anon_fn_seq;
    }
    crate::routes::check_table(&routes, diags);
    // `rule:tooling/commands-are-compiled`'s duplicate command name, over the same enumeration and for
    // the same reason — see `crate::commands::check_table`.
    crate::commands::check_table(&commands, diags);
    // § 4's fold, which is the second pass over the same table and the reason
    // the table is collected across the files rather than per file.
    crate::links::resolve(&routes, &links, exprs, diags);
    // `rule:core-classes/derive-field-list`'s second pass over the same table, after every deriving
    // class has recorded its codec: a field naming one of them is reachable
    // whichever file declared it.
    crate::derive::resolve_field_types(&codec_sites, &signatures, interner, exprs, diags);
    // The call-site half of the same deferral: § 9's map asked of the class a
    // `queryAs<T>` wrote, after every deriving class has recorded its mapping.
    crate::derive::check_row_sites(&row_sites, &signatures, exprs, diags);
    // The same deferral at the document door: whether the written class
    // participates in the format at all, and a constructor the contract fills
    // less than all of, asked of it and of every deriving class its fields
    // reach.
    crate::derive::check_json_sites(&json_sites, &signatures, exprs, diags);
    // The same deferral for the other member that writes a class: the body a
    // `jsonAs<T>` decodes is a peer's, so `rule:security/derived-codec-qualifiers`
    // asks the fields receiving it to declare the qualifier they receive.
    crate::derive::check_decode_sites(&decode_sites, &codec_sites, interner, diags);
    // `rule:types/type-test`'s `is callable(int): string` asks which of the
    // program's anonymous functions satisfy a written signature, which is the same
    // deferral one more time: the `is` and the literal it answers about need
    // not be in the same file. See `crate::callables`.
    crate::callables::resolve(exprs, interner, &module.graph, &signatures);
    // § 5's table crosses to `nvs-ir` here rather than being dropped: `rule:routing/api-document-is-generated-from-the-route-table`
    // emits the OpenAPI document from it, and it is what `nvs-ir` reads a
    // handler's declared path back out of.
    exprs.record_routes(routes);
    // § 6's table crosses the same way and at the same point: `Core\Command`'s
    // dispatch, its usage text and its completions are all generated from it.
    exprs.record_commands(commands);
    record_property_types(&signatures, exprs);
    // Declaration-side property types, for the one consumer that asks at a
    // declaration rather than at an access — see
    // `ExprTypeTable::property_default_ty`.
    for (span, ty) in signatures.property_default_types() {
        exprs.record_property_default_ty(*span, *ty);
    }
    enums
}

/// Hands one finished body's locals to the expression table, keyed by the span
/// the body covers.
///
/// Called at each frame's end rather than at each declaration: the scope is a
/// single table for the whole body (`rule:types/declaration` is
/// function-scoped), so one call carries every binding and none of them is a
/// name the body does not have. The map is **moved** out of the scope, which is
/// about to be dropped — see [`LocalBinding`] for what that costs a compile
/// that never reads it back.
///
/// Sorted by name so the order does not come from a hash map's, which is what
/// a `.lspt` expectation would otherwise be frozen against.
pub(crate) fn record_locals(body: Span, scope: &mut LocalScope, env: &mut Env<'_>) {
    let mut locals: Vec<LocalBinding> = std::mem::take(&mut scope.by_name)
        .into_iter()
        .map(|(name, info)| LocalBinding {
            name,
            ty: info.ty,
            declared: info.declared_span,
        })
        .collect();
    locals.sort_by(|left, right| left.name.cmp(&right.name));
    env.exprs.record_locals(body, locals);
}

/// Copies every class's own declared property types into the expression
/// table, so `nvs_ir::lower` can join them against the flattened slot order
/// and hand each class's per-slot types to codegen — `rule:types/erased-member-access`'s erased
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
/// (`rule:statements/storage-that-outlives-a-call`: "the script body is a function, so its variables are
/// locals"). Threaded through [`check_stmts`] so that a `namespace { ... }`
/// block's own top-level statements land in the *same* frame as the ones
/// outside it — a namespace scopes names, not storage.
pub(crate) struct ScriptFrame {
    scope: LocalScope,
    live: Live,
    return_ty: crate::ty::TypeId,
}

pub(crate) fn check_stmts(
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
                    in_constructor: false,
                    in_anon_fn: false,
                };
                let parents = decl.extends.iter();
                let parents = parents.chain(decl.implements.iter().map(|clause| &clause.name));
                warn_deprecated_parents(&qname, parents, &ctx, env);
                check_members(&decl.members, &ctx, env);
                crate::attributes::check_declaration(
                    crate::deprecated::Decl::Class,
                    &decl.attributes,
                    &decl.members,
                    &[],
                    &ctx,
                    env,
                );
                check_class_init(decl, &qname, env);
                check_class_lateinit_reads(decl, &qname, env);
                crate::conformance::check_class_conformance(decl, &qname, env);
                crate::conformance::check_constant_redeclarations(&decl.members, &qname, env);
                crate::conformance::check_class_finality(decl, &qname, env);
                crate::conformance::check_abstract_members(decl, &qname, env);
                crate::derive::check_class_derive(decl, &qname, &ctx, env);
                crate::testing::check_class_tests(decl, &qname, &ctx, env);
                crate::commands::check_class_commands(decl, &qname, &ctx, env);
                crate::routes::check_class_routes(decl, &qname, &ctx, env);
                record_property_defaults(&qname, env);
                record_lateinit_properties(&qname, env);
                record_static_properties(&qname, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                crate::conformance::check_constant_redeclarations(&decl.members, &qname, env);
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                    generator_elem: None,
                    in_constructor: false,
                    in_anon_fn: false,
                };
                warn_deprecated_parents(&qname, decl.extends.iter(), &ctx, env);
                check_members(&decl.members, &ctx, env);
                crate::attributes::check_declaration(
                    crate::deprecated::Decl::Interface,
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
                    in_constructor: false,
                    in_anon_fn: false,
                };
                check_members(&decl.members, &ctx, env);
                crate::attributes::check_declaration(
                    crate::deprecated::Decl::Enum,
                    &decl.attributes,
                    &decl.members,
                    &decl.cases,
                    &ctx,
                    env,
                );
            }
            // Declarations this walk has nothing to check, matched here
            // rather than left to fall through, because what
            // `crate::locals::check_stmt` does with one is refuse it as
            // `E0233` — and there the fact that it arrived at all *is* the
            // proof it was nested. `nvs_hir` is what reads both: a `type`
            // alias into the type table, an `autoload` into `rule:programs/no-runtime-autoload`'s map.
            StmtKind::TypeAliasDecl(_) | StmtKind::AutoloadDecl(_) => {}
            // Everything else is a *statement* of the script body, not a
            // declaration: one synthesized frame for the whole file, whose
            // variables are ordinary locals (`rule:statements/storage-that-outlives-a-call`). Reuses
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
                    in_constructor: false,
                    in_anon_fn: false,
                };
                crate::deprecated::window(None, None, env, |env| {
                    check_stmt(
                        stmt,
                        &mut frame.live,
                        &mut frame.scope,
                        frame.return_ty,
                        &ctx,
                        env,
                    );
                });
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

/// The same move for the class's `lateinit` properties (`rule:classes/lateinit-restrictions`), which
/// `nvs-ir` needs for `rule:classes/an-unwritten-property-read-throws`'s never-written storage state — see
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

/// `W1003` at each class or interface a declaration's `extends` or
/// `implements` names, when that one is deprecated and `class` is not.
fn warn_deprecated_parents<'n>(
    class: &QName,
    parents: impl Iterator<Item = &'n nvs_syntax::ast::Name>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    crate::deprecated::window(Some(class), None, env, |env| {
        for name in parents {
            let text = span_text(env.src, name.span);
            let parent = nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports);
            crate::deprecated::warn(name.span, &parent, crate::deprecated::Member::Type, env);
        }
    });
}

/// Checks each method and property hook body, each in its own
/// [`crate::deprecated::window`].
fn check_members(members: &[ClassMember], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    use crate::deprecated::{Member, window};
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => {
                let name = span_text(env.src, m.name).to_owned();
                let member = Some(Member::Method(name.clone()));
                window(ctx.current_class, member, env, |env| {
                    if let Some(class) = ctx.current_class {
                        crate::deprecated::warn_override(class, &name, m.name, env);
                    }
                    check_method(m, ctx, env);
                });
            }
            ClassMemberKind::Property(p) => {
                let name = strip_sigil(span_text(env.src, p.name)).to_owned();
                window(
                    ctx.current_class,
                    Some(Member::Property(name)),
                    env,
                    |env| {
                        check_property_hooks(p, ctx, env);
                    },
                );
            }
            _ => {}
        }
    }
}

/// Type-checks each of `p`'s `rule:classes/property-hooks` hook bodies as its own frame, and
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
        // A hook body is not the constructor's, whichever accessor it is: ADR
        // 0014 § 1 makes it a member called on a built instance.
        in_constructor: false,
        in_anon_fn: false,
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
        let mut live = Live::default();
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
                record_locals(block.span, &mut scope, env);
                let subject = format!(
                    "`{}`",
                    crate::signatures::hook_label(class, &name, hook.kind)
                );
                check_body_exits(&subject, block, return_ty, p.ty.span, env);
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
    let label = ctx
        .current_class
        .map(|class| format!("{class}::{}", span_text(env.src, m.name)));
    if let Some(label) = &label {
        env.exprs.record_method(m.name, label.clone());
    }
    if let Some(class) = ctx.current_class {
        crate::deprecated::record_entry(class, span_text(env.src, m.name), m.name, env);
    }

    check_return_type_is_written(m, env);

    let Some(body) = &m.body else {
        return; // abstract method or interface signature — nothing to check
    };

    // `rule:classes/lateinit-restrictions`: `readonly` means "assigned exactly once, and that
    // assignment happens during construction", so the constructor's own body
    // is the one place `crate::expr::assign::check_write_target` lets a write
    // to such a property through. Decided once here, from the name this
    // declaration was written with, rather than re-derived at every write.
    let ctx = &Ctx {
        namespace: ctx.namespace,
        imports: ctx.imports,
        current_class: ctx.current_class,
        current_hook: ctx.current_hook,
        generator_elem: ctx.generator_elem,
        in_constructor: span_text(env.src, m.name) == "constructor",
        in_anon_fn: false,
    };

    let mut scope = LocalScope::new();
    let mut live = Live::default();
    if ctx.current_class.is_some() && !m.modifiers.contains(&Modifier::Static) {
        // Seeded here rather than as an ordinary parameter: `$this` has no
        // `Param` node of its own to read a span from, and property/method
        // access on it (`crate::expr`) needs its type to be `self`'s class
        // the same way an explicit `new Foo()` result is. Gated on the
        // `static` modifier because `rule:statements/static-is-a-member-modifier` keeps PHP's semantics for
        // it: a static method is entered with no receiver, so `$this` in one
        // is `E0779` from `expr::assign::check_read` rather than a binding —
        // and `nvs_ir::lower::expr` panics on the read if it is not refused
        // here.
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

    // `rule:security/response-body-is-one-typed-member` is a fact about one body, so what answers it is
    // installed here and put back at every exit below — `crate::response`'s
    // module doc owns which bodies it arms and why a method's is the reach.
    let entering = crate::response::entering_body(m, ctx, env);
    let outer_writers = std::mem::replace(&mut env.body_writers, entering);

    // `rule:testing/inline-snapshots`: every inline snapshot the walk below records belongs to
    // *this* method, and stamping the rows afterwards is how they learn it —
    // `ExprTypeTable::own_inline_snapshots` owns why the walk is not told
    // which declaration it is inside. Taken here rather than at the top so a
    // declaration with no body cannot claim a row.
    let snapshots = env.exprs.inline_snapshot_mark();

    // `rule:iteration/generators`: a body containing `yield` is a generator, and everything
    // that follows from that is decided here rather than at each `yield` —
    // the declared return type must be `Iterator<T>`, and `T` is what every
    // `yield` operand in the body is checked against.
    let Some(elem) = generator_element(m, body, return_ty, ctx, env) else {
        check_block(&body.stmts, &mut live, &mut scope, return_ty, ctx, env);
        record_locals(body.span, &mut scope, env);
        check_every_path_returns(m, body, return_ty, env);
        reject_static_return_of_another_class(m, body, ctx, env);
        env.body_writers = outer_writers;
        if let Some(label) = label {
            env.exprs.own_inline_snapshots(snapshots, label);
        }
        return;
    };
    check_generator_inout_params(m, env);
    let inner = Ctx {
        namespace: ctx.namespace,
        imports: ctx.imports,
        current_class: ctx.current_class,
        current_hook: ctx.current_hook,
        generator_elem: Some(elem),
        in_constructor: ctx.in_constructor,
        in_anon_fn: ctx.in_anon_fn,
    };
    // A generator's body returns nothing: calling it produced the cursor, and
    // `rule:iteration/one-way-only` leaves no return value to retrieve. So the body is checked
    // against `void` — which is what makes `return $x;` inside one report
    // `E0447` from `crate::expr::check_return` rather than a mismatch against
    // the `Iterator<T>` the *declaration* names.
    let void = env.interner.void();
    check_block(&body.stmts, &mut live, &mut scope, void, &inner, env);
    record_locals(body.span, &mut scope, env);
    env.body_writers = outer_writers;
    if let Some(label) = label {
        env.exprs.own_inline_snapshots(snapshots, label);
    }
}

/// `rule:types/declaration`, at the one exit a body takes without writing anything: a
/// method promising a value at every exit may not have a path that reaches its
/// closing brace (`E0739`).
///
/// [`crate::returns`] owns the analysis and the asymmetry that makes reporting
/// safe. The gates that come first are not that analysis: a declaration
/// writing **no** return type at all promises nothing — that is a constructor,
/// and [`lower_optional_type`] typed it `mixed` — and `void`/`never` promise
/// nothing to write. Everything else, a declared `mixed` included, owes a
/// value: the declaration is what the caller reads, and `nvs-ir`'s fall-through
/// `Terminator::Return(None)` writes no slot for it to read.
/// `rule:statements/static-is-a-member-modifier`'s late static binding, held at the declaration: a body
/// promising `static` must answer the *called* class, not the declaring one
/// (`E0741`).
///
/// [`crate::signatures::MethodSig::returns_static`] owns why this is a refusal
/// rather than a pinned hole, and the diagnostic's own doc owns why refusing
/// is the PHP-compatible answer even though PHP accepts the declaration. These
/// expression shapes keep the promise, and they are the whole list:
///
/// * `$this` — the receiver *is* the called class.
/// * `new static(...)` — `rule:statements/static-is-a-member-modifier`'s allocation of it.
/// * a call written on `static`/`self`/`parent`/`$this` whose target itself
///   returns `static`, since each of those forwards the caller's called class.
///
/// An explicitly named class (`Base::make()`) is deliberately not on that
/// list: naming a class *sets* the called class, so its answer is that class
/// and not this frame's.
fn reject_static_return_of_another_class(
    m: &MethodMember,
    body: &Block,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if !crate::signatures::writes_static_return(m.return_type.as_ref()) {
        return;
    }
    let mut offenders: Vec<nvs_diagnostics::Span> = Vec::new();
    {
        let read: &Env<'_> = env;
        crate::returns::for_each_return(&body.stmts, &mut |e: &Expr| {
            if !yields_the_called_class(e, ctx, read) {
                offenders.push(e.span);
            }
        });
    }
    let declared = m.return_type.as_ref().map_or(m.name, |t| t.span);
    for span in offenders {
        env.diags.report(
            Diagnostic::error(
                code::E_STATIC_RETURN_NOT_CALLED_CLASS,
                "a body declaring `static` returns a value that is not the called class",
            )
            .with_primary(span, "not the called class")
            .with_secondary(declared, "declared `static` here")
            .with_help(
                "`static` is the class the call named, which a subclass may be — return `$this` \
                 or `new static(...)`, or declare `self` if the declaring class is what is meant",
            ),
        );
    }
}

/// Whether `e`'s runtime class is provably the called class — the whitelist
/// [`reject_static_return_of_another_class`] documents.
fn yields_the_called_class(e: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> bool {
    match &e.kind {
        ExprKind::Variable(span) => span_text(env.src, *span) == "$this",
        ExprKind::New {
            target: NewTarget::StaticTy,
            ..
        } => true,
        ExprKind::StaticCall {
            class,
            method: MemberName::Ident(name),
            ..
        } => {
            matches!(
                class.kind,
                ExprKind::SelfExpr | ExprKind::StaticExpr | ExprKind::ParentExpr
            ) && forwards_the_called_class(span_text(env.src, *name), ctx, env)
        }
        ExprKind::MethodCall {
            object,
            method: MemberName::Ident(name),
            ..
        } => {
            crate::expr::is_this_receiver(object, env.src)
                && forwards_the_called_class(span_text(env.src, *name), ctx, env)
        }
        _ => false,
    }
}

/// Whether `name`, resolved from the enclosing class, itself returns `static`.
fn forwards_the_called_class(name: &str, ctx: &Ctx<'_>, env: &Env<'_>) -> bool {
    ctx.current_class
        .and_then(|c| crate::signatures::resolve_method(c, name, env.signatures, env.graph))
        .is_some_and(|(_, sig)| sig.returns_static)
}

/// `rule:types/declaration`'s return slot, which every declaration fills but
/// the one that has no value to promise (`E0823`).
///
/// Asked ahead of [`check_method`]'s body gate rather than beside the lowering
/// below, because a declaration with no body is still a declaration a caller
/// reads: an abstract method and an interface member owe the slot on exactly
/// the terms a concrete one does. The constructor is the exception the rule
/// states — it answers with the instance, which is why
/// `E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE` refuses a valued `return` in one —
/// and it is recognized here the same way [`check_method`] recognizes it for
/// `readonly`, from the name this declaration was written with.
///
/// **`__construct` is silent here too, and it is not a second constructor
/// name.** `nvs_syntax::casing` has already refused the spelling
/// (`E_LEGACY_CONSTRUCTOR_SPELLING`), and a PHP constructor arrives written
/// exactly one way — no return type, because PHP has no slot to write one in.
/// Asking its author for a `: void` on the same declaration they are about to
/// rename is an edit they would only undo, on the one path this diagnostic's
/// whole audience walks.
///
/// Reporting does not stop the body being checked. [`lower_optional_type`]
/// still answers `mixed` for the empty slot, so what follows is the check the
/// author would have got by writing `mixed` themselves, and one missing
/// annotation does not hide every other error in the body behind it.
fn check_return_type_is_written(m: &MethodMember, env: &mut Env<'_>) {
    let name = span_text(env.src, m.name);
    if m.return_type.is_some() || name == "constructor" || name == "__construct" {
        return;
    }
    let message = format!("`{name}` declares no return type");
    env.diags.report(
        Diagnostic::error(code::E_METHOD_RETURN_TYPE_REQUIRED, message)
            .with_primary(m.name, "no `: T` follows this declaration's parameters")
            .with_help(
                "`rule:types/declaration` makes the return slot mandatory — write `: void` \
                 where the body hands nothing back, and `: never` where it always throws. \
                 A constructor is the one declaration that omits it",
            ),
    );
}

fn check_every_path_returns(m: &MethodMember, body: &Block, return_ty: TypeId, env: &mut Env<'_>) {
    if m.return_type.is_none() {
        return;
    }
    let name = span_text(env.src, m.name).to_owned();
    let declared = m.return_type.as_ref().map_or(m.name, |t| t.span);
    check_body_exits(&format!("`{name}`"), body, return_ty, declared, env);
}

/// `rule:types/declaration`'s promise read at both exits of one block body:
/// `E0739` where a path reaches the closing brace, `E0822` where a written
/// `return;` leaves with nothing in hand.
///
/// The two are one function because they are one promise — a non-`void`
/// declaration hands back a value **every** way out — and because every door a
/// body arrives through owes both: a method, a block-bodied `get` hook and a
/// block-bodied `fn` alike. `subject` is how the declaration is named back to
/// its author, already quoted, and `declared` is the span of the written type
/// the message points at.
///
/// Only the `E0739` half is an analysis. Whether a written `return;` is legal
/// is the declared type alone. The falling-off path is asked of `never` too:
/// a `never` body that reaches its end would come back to a caller that
/// [`crate::returns`]'s walk already treats as left, since a statement typed
/// `never` counts as an exit there. So that refusal is what keeps the walk
/// sound, and the help it gives names the exits a `never` body has.
pub(crate) fn check_body_exits(
    subject: &str,
    body: &Block,
    return_ty: TypeId,
    declared: Span,
    env: &mut Env<'_>,
) {
    if matches!(env.interner.get(return_ty), Ty::Void) {
        return;
    }
    let mut valueless = Vec::new();
    crate::returns::for_each_valueless_return(&body.stmts, &mut |span| valueless.push(span));
    let described = env.interner.describe(return_ty);
    for span in valueless {
        env.diags.report(
            Diagnostic::error(
                code::E_VALUELESS_RETURN,
                format!("{subject} declares `{described}` but this `return` hands back nothing"),
            )
            .with_primary(span, "leaves with no value")
            .with_secondary(declared, "declared here")
            .with_help(
                "return a value here, or declare `void` — `rule:types/conversion` has no implicit \
                 `null` to stand in for the declared type",
            ),
        );
    }
    if block_always_exits(&body.stmts, env.exprs) {
        return;
    }
    let help = if matches!(env.interner.get(return_ty), Ty::Never) {
        "throw, call `exit`, or call a `never` function on that path — a `never` body never \
         comes back to its caller, so no path may reach its end"
    } else {
        "return a value on that path, throw, or declare `void` — a body that falls off its \
         end returns nothing at all, and `rule:types/conversion` has no implicit `null` to stand in for \
         the declared type"
    };
    env.diags.report(
        Diagnostic::error(
            code::E_MISSING_RETURN,
            format!("{subject} declares `{described}` but a path reaches the end of its body"),
        )
        .with_primary(closing_brace(body), "reached without returning")
        .with_secondary(declared, "declared here")
        .with_help(help),
    );
}

/// `rule:iteration/generators`'s frame lifetime, as a refusal: a generator declares no `inout $x`
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
                "`rule:iteration/generators`: calling a generator returns the state object without running \
                 the body, so the caller's cell is gone before the first `advance()` — take \
                 the value by copy and `yield` what the body computes from it",
            ),
        );
    }
}

/// `rule:iteration/generators`'s `T`, for a method whose body makes it a generator —
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
            "`rule:iteration/generators`: calling a generator runs no user code — it allocates and returns \
             the state object, which implements `Iterator<T>`",
        ),
    );
    let _ = ctx;
    // Still a generator, with an unchecked element type: `mixed` accepts
    // every `yield` operand, so one wrong return type reports once instead of
    // once plus a stray-`yield` diagnostic per `yield` in the body.
    Some(env.interner.mixed())
}
