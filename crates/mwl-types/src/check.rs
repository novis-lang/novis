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
//! Mirrors [`mwl_hir::members`]'s own walk shape: [`check_stmts`] tracks
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

use mwl_diagnostics::{Diagnostics, SourceFile};
use mwl_hir::{Module, QName};
use mwl_syntax::ast::{
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

/// Type-checks every method body reachable from `stmts`, using the already
/// name-resolved `module` for symbol/alias lookups. `interner` accumulates
/// every type this run interns — pass the same one across every file of a
/// program sharing `module`, the same way `module` itself is built once and
/// shared. `exprs` accumulates every call's/`new`'s resolved target this run
/// records — see [`crate::expr_table`]'s own module docs; a caller with no use
/// for it yet (today, only `mwl-ir` reads it back) still passes one and may
/// simply drop it afterward.
pub fn check_program(
    stmts: &[Stmt],
    src: &SourceFile,
    module: &Module,
    interner: &mut TypeInterner,
    exprs: &mut ExprTypeTable,
    diags: &mut Diagnostics,
) {
    // ADR 0010 § 2's backing types first: interning an enum-typed annotation
    // needs one, and `build_signatures` interns every declared annotation in
    // the file. See `crate::enums`.
    let enums = crate::enums::build_enum_table(stmts, src, diags);
    let signatures = build_signatures(stmts, module, &enums, src, interner, diags);
    let mut env = Env {
        symbols: &module.symbols,
        aliases: &module.aliases,
        graph: &module.graph,
        signatures: &signatures,
        enums: &enums,
        src,
        interner,
        exprs,
        diags,
    };
    let mut frame = ScriptFrame {
        scope: LocalScope::new(),
        live: FxHashSet::default(),
        // ADR 0021 § 3: `require`'s value is what a `return`-ing target file
        // hands back, typed `mixed` at the boundary — so the synthesized
        // frame's return type is `mixed`, not `void`.
        return_ty: env.interner.mixed(),
    };
    check_stmts(stmts, &[], &FxHashMap::default(), &mut frame, &mut env);
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
                };
                check_members(&decl.members, &ctx, env);
                check_class_init(decl, &qname, env);
                check_class_lateinit_reads(decl, &qname, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                };
                check_members(&decl.members, &ctx, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                    current_hook: None,
                };
                check_members(&decl.members, &ctx, env);
            }
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
/// [`crate::expr::check_assign`] already performs for the desugared store.
///
/// [`Ctx::current_hook`] is what stops `$this->p` inside `$p`'s own hooks
/// from resolving to a re-entrant call to the very accessor being checked.
fn check_property_hooks(p: &mwl_syntax::ast::PropertyMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
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
        let is_set = hook.kind == mwl_syntax::ast::PropertyHookKind::Set;
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
            mwl_syntax::ast::PropertyHookBody::Expr(e) => {
                let expected = if is_set { prop_ty } else { return_ty };
                crate::expr::check_expr(e, Some(expected), &mut live, &scope, &inner, env);
            }
            mwl_syntax::ast::PropertyHookBody::Block(block) => {
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
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        scope.declare_param(name.clone(), ty, param.name);
        live.insert(name);
    }
    let return_ty = lower_optional_type(m.return_type.as_ref(), ctx, env);

    check_block(&body.stmts, &mut live, &mut scope, return_ty, ctx, env);
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::{SourceMap, code};
    use mwl_hir::resolve_file;
    use mwl_syntax::parse_file;

    use super::*;

    /// Wraps `body` inside `class T { function m(): void { ... } }` and
    /// checks it — the common shape for a definite-assignment/expression
    /// fixture that doesn't need its own class.
    fn check_in_method(body: &str) -> Diagnostics {
        check_src(&format!(
            "<?mwl\nclass T {{\n  function m(): void {{\n{body}\n  }}\n}}\n"
        ))
    }

    fn check_src(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        check_program(
            &stmts,
            map.file(file),
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        diags
    }

    /// A `Core` member resolves through the signature table
    /// `crate::core_lib` seeded, so its return type reaches the binding it is
    /// assigned to -- `array<T> -> uint` with `T` bound from the argument.
    #[test]
    fn a_core_member_call_type_checks_against_its_registered_signature() {
        let diags =
            check_in_method("array<int> $a = [1, 2];\nuint $n = Core\\Arr::count($a);\necho $n;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The point of registering a signature at all: a `Core` call is now
    /// arity-checked exactly like a user-declared one, where before every
    /// `Core\...` reference was trusted unchecked.
    #[test]
    fn a_core_member_call_with_the_wrong_arity_is_diagnosed() {
        let diags = check_in_method("array<int> $a = [1];\nuint $n = Core\\Arr::count($a, 2);\n");
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
            "{diags:?}"
        );
    }

    /// An argument that cannot bind the variable at all: `T` stays unbound and
    /// substitutes to `mixed`, so the parameter reads `array<mixed>` and an
    /// `int` still fails it. See `crate::generics` for that rule.
    #[test]
    fn a_core_member_call_with_a_wrongly_typed_argument_is_diagnosed() {
        let diags = check_in_method("uint $n = Core\\Arr::count(7);\n");
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{diags:?}"
        );
    }

    /// A type variable never survives the call site, so the assignment below
    /// is checked against the concrete `uint` the signature declares.
    #[test]
    fn a_core_member_call_assigned_to_the_wrong_type_is_diagnosed() {
        let diags = check_in_method("array<int> $a = [1];\nstring $n = Core\\Arr::count($a);\n");
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{diags:?}"
        );
    }

    /// A `Core` class the registry does not name stays trusted rather than
    /// becoming an error -- `crate::core_lib`'s own docs own why, and name
    /// removing that trust as what completing the registry buys.
    #[test]
    fn an_unregistered_core_reference_is_still_trusted() {
        let diags = check_in_method("Core\\Str::upper(\"x\");\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_declared_and_assigned_local_reads_fine() {
        let diags = check_in_method("int $n = 1;\n$n = $n + 1;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn reading_an_undeclared_local_is_diagnosed() {
        let diags = check_in_method("echo $missing;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE))
        );
    }

    #[test]
    fn redeclaring_a_local_is_diagnosed() {
        let diags = check_in_method("int $n = 1;\nint $n = 2;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_REDECLARED_LOCAL))
        );
    }

    /// ADR 0037: `var $n = 1;` fixes `$n`'s type to `int`, exactly as if it
    /// had been written out — so a later assignment of a different type is
    /// the ordinary `E_TYPE_MISMATCH` a typed local would also get.
    #[test]
    fn var_infers_the_initializers_type_and_fixes_it() {
        let diags = check_in_method("var $n = 1;\n$n = \"x\";\n");
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn var_infers_a_class_type_from_new() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    var $x = new Foo();\n    $x->missing;\n  }\n}\n",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
            "$x should be inferred as `Foo`, so `->missing` is unknown: {diags:?}"
        );
    }

    #[test]
    fn redeclaring_a_var_local_is_diagnosed_like_any_other() {
        let diags = check_in_method("var $n = 1;\nvar $n = 2;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_REDECLARED_LOCAL))
        );
    }

    /// ADR 0037 § 2: a bare array literal has no target type to synthesize
    /// against, so `var` cannot infer one — this is the one initializer
    /// shape it refuses rather than silently falling back to `array<mixed>`.
    #[test]
    fn var_rejects_a_bare_array_literal_initializer() {
        let diags = check_in_method("var $rows = [1, 2, 3];\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE)),
            "{diags:?}"
        );
    }

    #[test]
    fn reading_a_variable_assigned_on_only_one_if_branch_is_diagnosed() {
        let diags =
            check_in_method("bool $flag = true;\nint $n;\nif ($flag) { $n = 1; }\necho $n;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
            "{diags:?}"
        );
    }

    #[test]
    fn reading_a_variable_assigned_on_both_branches_is_fine() {
        let diags = check_in_method(
            "bool $flag = true;\nint $n;\nif ($flag) { $n = 1; } else { $n = 2; }\necho $n;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_variable_assigned_before_a_loop_reads_fine_after_it() {
        let diags = check_in_method("int $n = 0;\nwhile (false) { $n = 1; }\necho $n;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `locals::check_stmt`'s `Switch` arm: every case ends in a `break`, and
    /// a `default` covers "no case matched" — so `$n` reads fine after it.
    #[test]
    fn a_switch_with_default_and_a_break_in_every_case_assigns_definitely() {
        let diags = check_in_method(
            "int $n;\nswitch (1) {\ncase 1:\n  $n = 1;\n  break;\ndefault:\n  $n = 2;\n  break;\n}\necho $n;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// Same shape but with no `default` arm: "no case matched" is a real
    /// path that leaves `$n` unassigned, so the read is still diagnosed.
    #[test]
    fn a_switch_with_no_default_never_assigns_definitely() {
        let diags =
            check_in_method("int $n;\nswitch (1) {\ncase 1:\n  $n = 1;\n  break;\n}\necho $n;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
            "{diags:?}"
        );
    }

    /// A case that falls through with no `break` contributes nothing on its
    /// own, but the case it falls into still counts — so this still assigns
    /// definitely on every path (direct jump to either `case`, or fallthrough
    /// from `case 1` into `case 2`).
    #[test]
    fn a_switch_case_falling_through_into_an_assigning_case_still_assigns_definitely() {
        let diags = check_in_method(
            "int $n;\nswitch (1) {\ncase 1:\n  $n = 1;\ncase 2:\n  $n = 2;\n  break;\ndefault:\n  $n = 3;\n  break;\n}\necho $n;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `locals::check_stmt`'s `Try` arm: `body` completing and `catch`
    /// completing both assign `$n`, so it reads fine afterward.
    #[test]
    fn a_try_and_its_catch_both_assigning_reads_fine_after() {
        let diags = check_in_method(
            "int $n;\ntry {\n  $n = 1;\n} catch (LogicError $e) {\n  $n = 2;\n}\necho $n;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The `catch` doesn't assign `$n`, so it isn't definite on every path —
    /// unlike the fully-conservative old behavior, this now depends on
    /// what's actually inside `catch`, not just that a `try` was involved.
    #[test]
    fn a_try_whose_catch_does_not_assign_is_still_diagnosed() {
        let diags =
            check_in_method("int $n;\ntry {\n  $n = 1;\n} catch (LogicError $e) {\n}\necho $n;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
            "{diags:?}"
        );
    }

    /// `finally` always runs, so its assignment carries forward even though
    /// neither `body` nor any `catch` touches `$n` at all.
    #[test]
    fn a_trys_finally_assignment_reads_fine_after_it() {
        let diags = check_in_method(
            "int $n;\ntry {\n  echo \"ok\";\n} finally {\n  $n = 1;\n}\necho $n;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// ADR 0035: a condition — `if`/`while`/`for`'s middle clause/`?:`/`&&`/
    /// `||`/`!` — accepts any type at all, judged by PHP's full truthy table
    /// at runtime, never `E_TYPE_MISMATCH` for not already being `bool`.
    /// `bool`-typed positions elsewhere (a parameter, a property, `== `/`===`)
    /// are unaffected and still need an explicit `as bool` or comparison.
    #[test]
    fn a_non_bool_condition_is_never_a_type_mismatch() {
        let diags = check_in_method(
            "string $s = \"\";\nint $n = 0;\narray<int> $rows = [];\n\
             if ($s) {}\nwhile ($n) {}\nfor (; $rows; ) {}\n\
             bool $ok = $s && $n || !$rows;\necho $ok ? 1 : 2;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn int_plus_uint_is_diagnosed() {
        let diags = check_in_method("int $a = 1;\nuint $b = 1;\nint $c = $a + $b;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INT_UINT_ARITHMETIC))
        );
    }

    #[test]
    fn an_integer_literal_assigned_into_a_uint_local_is_fine() {
        // ADR 0007 § 4: a plain integer literal means `uint` exactly where
        // that's the expected type — this must not be diagnosed as `int`
        // vs. `uint` mismatch.
        let diags = check_in_method("uint $n = 1;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// ADR 0007 § 4: a literal larger than `i64::MAX` (`9223372036854775807`)
    /// but still within `uint`'s `u64` range is legal exactly where `uint` is
    /// expected.
    #[test]
    fn a_literal_too_large_for_int_assigned_into_a_uint_local_is_fine() {
        let diags = check_in_method("uint $n = 9223372036854775808;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The same literal has no legal home as a plain `int` — ADR 0007 § 4's
    /// "otherwise a diagnostic saying exactly that."
    #[test]
    fn a_literal_too_large_for_int_assigned_into_an_int_local_is_diagnosed() {
        let diags = check_in_method("int $n = 9223372036854775808;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
            "{diags:?}"
        );
    }

    /// Even a bare, un-targeted literal with no `uint` in sight still gets
    /// range-checked against `int`'s own half of the range.
    #[test]
    fn a_literal_too_large_for_int_with_no_expected_type_is_diagnosed() {
        let diags = check_in_method("echo 9223372036854775808;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
            "{diags:?}"
        );
    }

    /// A literal past even `uint`'s full `u64` range is always out of range,
    /// regardless of what's expected.
    #[test]
    fn a_literal_too_large_for_uint_is_diagnosed_even_where_uint_is_expected() {
        let diags = check_in_method("uint $n = 99999999999999999999999999;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
            "{diags:?}"
        );
    }

    /// The magnitude check applies to every base `mwl-syntax`'s lexer cooks,
    /// not just decimal — a hex literal one bit past `uint`'s 64-bit range.
    #[test]
    fn a_hex_literal_too_large_for_uint_is_diagnosed() {
        let diags = check_in_method("uint $n = 0x1_0000_0000_0000_0000;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
            "{diags:?}"
        );
    }

    /// A negative literal (`-5`) needs no magnitude-specific diagnostic at
    /// all: the inner literal always types as plain `int` (the wrapping
    /// `Unary::Neg` node checks it with no expected type), so assigning that
    /// `int` into a `uint` target is the ordinary `int`-vs-`uint`
    /// `E_TYPE_MISMATCH`, not `E_INT_LITERAL_OUT_OF_RANGE`.
    #[test]
    fn a_negative_literal_into_a_uint_local_is_an_ordinary_type_mismatch() {
        let diags = check_in_method("uint $n = -5;\n");
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_INT_LITERAL_OUT_OF_RANGE)),
            "{diags:?}"
        );
    }

    /// A `\u{...}` escape naming a codepoint past Unicode's `0x10FFFF` scalar
    /// ceiling has no UTF-8 encoding — `crate::string_lit`'s own
    /// `E_INVALID_UNICODE_ESCAPE`.
    #[test]
    fn a_unicode_escape_out_of_range_is_diagnosed() {
        let diags = check_in_method(r#"string $s = "\u{110000}";"#);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INVALID_UNICODE_ESCAPE)),
            "{diags:?}"
        );
    }

    /// A `\u{...}` escape naming a UTF-16 surrogate codepoint is the other
    /// shape with no UTF-8 encoding.
    #[test]
    fn a_unicode_escape_naming_a_surrogate_is_diagnosed() {
        let diags = check_in_method(r#"string $s = "\u{D800}";"#);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INVALID_UNICODE_ESCAPE)),
            "{diags:?}"
        );
    }

    /// A lone `\xFF` byte escape can never be a valid UTF-8 sequence on its
    /// own — `string` is guaranteed-valid UTF-8 (ADR 0009), so this is
    /// `E_STRING_LITERAL_INVALID_UTF8`, not silently accepted.
    #[test]
    fn a_byte_escape_producing_invalid_utf8_is_diagnosed() {
        let diags = check_in_method(r#"string $s = "\xFF";"#);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_STRING_LITERAL_INVALID_UTF8)),
            "{diags:?}"
        );
    }

    /// Two byte escapes that combine into one valid UTF-8 character
    /// (`\xC3\xA9` is `é`) are fine — the check is on the assembled result,
    /// not each escape byte in isolation.
    #[test]
    fn byte_escapes_combining_into_valid_utf8_are_not_diagnosed() {
        let diags = check_in_method(r#"string $s = "\xC3\xA9";"#);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// A single-quoted literal's own two escapes (`\\`/`\'`) can never
    /// produce invalid UTF-8, so it gets no cooking-diagnostic pass at all —
    /// confirming the quote-kind guard actually gates on the literal's own
    /// spelling rather than always running.
    #[test]
    fn a_single_quoted_literal_is_never_escape_diagnosed() {
        let diags = check_in_method(r"string $s = '\xFF';");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The same invalid-UTF-8 byte-escape check applies inside an
    /// interpolated string's own literal text, not just a plain `Str` —
    /// `ExprKind::Interpolated`'s `StringPart::Text` arm.
    #[test]
    fn a_byte_escape_inside_an_interpolated_string_is_diagnosed() {
        let diags = check_in_method("string $y = \"z\";\nstring $s = \"\\xFF$y\";\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_STRING_LITERAL_INVALID_UTF8)),
            "{diags:?}"
        );
    }

    // --- heredoc/nowdoc flexible-indentation stripping ---------------------

    /// A heredoc whose closing marker is indented, and whose body lines
    /// carry exactly that much indentation, is fine — the common,
    /// well-formed shape PHP 7.3's "flexible heredoc" rule exists for.
    #[test]
    fn a_consistently_indented_heredoc_is_not_diagnosed() {
        let diags = check_in_method("string $s = <<<EOT\n    hello\n    world\n    EOT;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// The closing marker's own indentation mixing spaces and tabs is
    /// `E_HEREDOC_MIXED_INDENT` — PHP requires one or the other so a body
    /// line's leading whitespace can be compared byte-for-byte.
    #[test]
    fn a_heredoc_closing_marker_mixing_tabs_and_spaces_is_diagnosed() {
        let diags = check_in_method("string $s = <<<EOT\n\thello\n\t EOT;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_HEREDOC_MIXED_INDENT)),
            "{diags:?}"
        );
    }

    /// A body line with less leading whitespace than the closing marker is
    /// `E_HEREDOC_INSUFFICIENT_INDENT`.
    #[test]
    fn a_heredoc_body_line_with_insufficient_indentation_is_diagnosed() {
        let diags = check_in_method("string $s = <<<EOT\n    hello\n  world\n    EOT;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_HEREDOC_INSUFFICIENT_INDENT)),
            "{diags:?}"
        );
    }

    /// A truly empty body line is exempt from the indentation check, even
    /// between two consistently indented lines.
    #[test]
    fn a_blank_heredoc_body_line_is_not_diagnosed() {
        let diags = check_in_method("string $s = <<<EOT\n    hello\n\n    world\n    EOT;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// A nowdoc applies no escape grammar at all (unlike a heredoc, which
    /// still cooks `\n`/`\t`/etc.), but still gets indentation-checked —
    /// `\n` here must stay two literal characters, and the check above it
    /// runs regardless of whether escapes do.
    #[test]
    fn a_nowdoc_skips_escape_cooking_but_still_checks_indentation() {
        let diags = check_in_method("string $s = <<<'EOT'\n    raw \\n text\n    EOT;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// An interpolated heredoc's own indentation check applies per body run,
    /// not just to a plain `Str`-collapsed one — this line becomes its own
    /// `StringPart::Text` run picking up right after `$y`'s interpolation
    /// site, so it has to be recognized as a fresh line on its own.
    #[test]
    fn an_interpolated_heredoc_body_line_after_interpolation_is_still_checked() {
        let diags = check_in_method(
            "string $y = \"z\";\nstring $s = <<<EOT\n    pre $y\n  post\n    EOT;\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_HEREDOC_INSUFFICIENT_INDENT)),
            "{diags:?}"
        );
    }

    /// The same interpolated-heredoc shape, consistently indented, is fine —
    /// confirms the indentation check doesn't false-positive on a
    /// well-formed interpolation site.
    #[test]
    fn a_consistently_indented_interpolated_heredoc_is_not_diagnosed() {
        let diags = check_in_method(
            "string $y = \"z\";\nstring $s = <<<EOT\n    pre $y\n    post\n    EOT;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn integer_division_into_a_plain_int_is_diagnosed() {
        let diags = check_in_method("int $n = 7 / 2;\n");
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn integer_division_into_a_union_target_is_fine() {
        let diags = check_in_method("int|float $n = 7 / 2;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn assigning_mixed_into_a_typed_local_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function m(mixed $m): void {\n    int $n = $m;\n  }\n}\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn an_array_literal_element_mismatch_at_depth_one_is_diagnosed() {
        let diags = check_in_method(r#"array<int> $a = [1, "x"];"#);
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn an_array_literal_element_mismatch_at_depth_two_is_diagnosed() {
        let diags = check_in_method(r#"array<array<int>> $a = [[1, 2], [1, "x"]];"#);
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn an_array_literal_element_mismatch_at_depth_three_is_diagnosed() {
        let diags = check_in_method(r#"array<array<array<int>>> $a = [[[1], [1, "x"]]];"#);
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_correctly_typed_nested_array_literal_is_fine() {
        let diags = check_in_method("array<array<int>> $a = [[1, 2], [3]];\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ADR 0007 § 5: an array key is `int`, `uint`, or `string`; a `float`,
    // `bool`, or `null` key is rejected outright.

    #[test]
    fn a_string_key_array_literal_is_fine() {
        let diags = check_in_method(r#"array<int> $a = ["x" => 1, "y" => 2];"#);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_int_key_array_literal_is_fine() {
        let diags = check_in_method(r#"array<int> $a = [5 => 1, 6 => 2];"#);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_dynamic_string_key_array_literal_is_fine() {
        let diags = check_in_method("string $k = \"x\";\narray<int> $a = [$k => 1];\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_float_key_array_literal_is_diagnosed() {
        let diags = check_in_method(r#"array<int> $a = [1.5 => 1];"#);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_bool_key_array_literal_is_diagnosed() {
        let diags = check_in_method(r#"array<int> $a = [true => 1];"#);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_null_key_array_literal_is_diagnosed() {
        let diags = check_in_method(r#"array<int> $a = [null => 1];"#);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_bool_subscript_key_is_diagnosed() {
        let diags = check_in_method("array<int> $a = [1];\nbool $b = true;\n$a[$b] = 1;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ARRAY_KEY_INVALID_TYPE)),
            "{diags:?}"
        );
    }

    #[test]
    fn an_int_subscript_key_is_fine() {
        let diags = check_in_method("array<int> $a = [1, 2, 3];\nint $x = $a[1];\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_type_alias_is_substituted_into_a_local_declaration() {
        let diags = check_src(
            "<?mwl\ntype Id = uint;\nclass T {\n  function m(): void {\n    Id $x = 1;\n    int $y = $x;\n  }\n}\n",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "`Id` expands to `uint`, so assigning it into a plain `int` should mismatch: {diags:?}"
        );
    }

    #[test]
    fn self_resolves_inside_a_method_body() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function m(): void {\n    self $x = new self();\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_return_type_mismatch_is_diagnosed() {
        let diags =
            check_src("<?mwl\nclass T {\n  function m(): int {\n    return \"x\";\n  }\n}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BAD_RETURN_TYPE))
        );
    }

    #[test]
    fn a_matching_return_type_is_fine() {
        let diags = check_src("<?mwl\nclass T {\n  function m(): int {\n    return 1;\n  }\n}\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_this_property_access_has_its_declared_type() {
        // The property has an inline default, so ADR 0022's own check
        // (`crate::ctor_init`) has nothing to say about a missing
        // constructor here — this fixture is only exercising property-type
        // recovery.
        let diags = check_src(
            "<?mwl\nclass T {\n  public int $count = 0;\n  function m(): void {\n    int $n = $this->count;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_this_property_type_mismatch_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass T {\n  public int $count;\n  function m(): void {\n    string $n = $this->count;\n  }\n}\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_this_method_call_returns_its_declared_type() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function a(): int { return 1; }\n  function b(): void {\n    int $n = $this->a();\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_undeclared_this_method_call_is_diagnosed() {
        let diags =
            check_src("<?mwl\nclass T {\n  function m(): void {\n    $this->missing();\n  }\n}\n");
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_property_access_on_a_new_expression_resolves() {
        // Inline default again, for the same reason as the fixture above.
        let diags = check_src(
            "<?mwl\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(): void {\n    int $n = (new Foo())->count;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_undeclared_property_on_a_typed_local_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $x = new Foo();\n    $x->missing;\n  }\n}\n",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
            "{diags:?}"
        );
    }

    #[test]
    fn an_arity_mismatch_on_a_method_call_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function a(int $x): void {}\n  function b(): void {\n    $this->a();\n  }\n}\n",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
            "{diags:?}"
        );
    }

    #[test]
    fn an_argument_type_mismatch_on_a_method_call_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function a(int $x): void {}\n  function b(): void {\n    $this->a(\"s\");\n  }\n}\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_constructor_argument_is_type_checked() {
        let diags = check_src(
            "<?mwl\nclass Foo {\n  function constructor(int $x) {}\n}\nclass T {\n  function m(): void {\n    new Foo(\"s\");\n  }\n}\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_static_call_return_type_is_recovered() {
        let diags = check_src(
            "<?mwl\nclass T {\n  static function make(): int { return 1; }\n  function m(): void {\n    int $n = self::make();\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn new_parent_resolves_to_the_parent_class() {
        let diags = check_src(
            "<?mwl\nclass Base {}\nclass Sub extends Base {\n  function m(): void {\n    Base $x = new parent();\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `parent` as a *type* atom (a parameter here) — distinct from `new
    /// parent(...)`, which the previous test already covers — resolves
    /// against the same `extends` link.
    #[test]
    fn a_parent_typed_parameter_resolves_to_the_parent_class() {
        let diags = check_src(
            "<?mwl\nclass Base {}\nclass Sub extends Base {\n  function m(parent $x): void {\n    Base $y = $x;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// A class with no `extends` has no parent for the `parent` type atom to
    /// name — diagnosed rather than silently `mixed`, unlike `new
    /// parent(...)`'s deliberate silent fallback for the same shape.
    #[test]
    fn a_parent_type_atom_with_no_extends_is_diagnosed() {
        let diags = check_src("<?mwl\nclass Base {\n  function m(parent $x): void {\n  }\n}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_NO_PARENT_CLASS)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_match_expressions_type_is_the_union_of_its_arms() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function m(): void {\n    string $n = match (1) { 1 => 2, default => 3 };\n  }\n}\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_ternary_expressions_type_is_the_union_of_its_branches() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function m(): void {\n    int|string $n = true ? 1 : \"s\";\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_ternary_expressions_type_mismatch_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function m(): void {\n    int $n = true ? 1 : \"s\";\n  }\n}\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    // ADR 0013: `<`/`<=`/`>`/`>=`/`<=>` between two objects.

    #[test]
    fn comparable_objects_of_the_same_class_type_check_as_bool_or_int() {
        let diags = check_src(
            "<?mwl\n\
             class Money implements Comparable {\n\
             \x20 private int $cents;\n\
             \x20 function constructor(int $cents) { $this->cents = $cents; }\n\
             \x20 function compareTo(self $other): int { return $this->cents <=> $other->cents; }\n\
             }\n\
             class T {\n\
             \x20 function m(): void {\n\
             \x20\x20 Money $a = new Money(1);\n\
             \x20\x20 Money $b = new Money(2);\n\
             \x20\x20 bool $lt = $a < $b;\n\
             \x20\x20 bool $le = $a <= $b;\n\
             \x20\x20 bool $gt = $a > $b;\n\
             \x20\x20 bool $ge = $a >= $b;\n\
             \x20\x20 int $cmp = $a <=> $b;\n\
             \x20 }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn comparing_two_objects_of_a_non_comparable_class_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    Foo $b = new Foo();\n    $a < $b;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_COMPARISON_REQUIRES_COMPARABLE)),
            "{diags:?}"
        );
    }

    #[test]
    fn comparing_two_different_comparable_classes_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass A implements Comparable {}\nclass B implements Comparable {}\nclass T {\n  function m(): void {\n    A $a = new A();\n    B $b = new B();\n    $a < $b;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_COMPARISON_REQUIRES_COMPARABLE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_subclass_of_a_comparable_class_is_comparable_to_itself() {
        let diags = check_src(
            "<?mwl\nclass Money implements Comparable {}\nclass Cents extends Money {}\nclass T {\n  function m(): void {\n    Cents $a = new Cents();\n    Cents $b = new Cents();\n    $a < $b;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn concatenating_a_non_stringable_object_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    string $s = \"\" . $a;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
            "{diags:?}"
        );
    }

    #[test]
    fn interpolating_a_non_stringable_object_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    string $s = \"value: $a\";\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
            "{diags:?}"
        );
    }

    #[test]
    fn echoing_a_non_stringable_object_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    echo $a;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
            "{diags:?}"
        );
    }

    #[test]
    fn printing_a_non_stringable_object_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    print $a;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
            "{diags:?}"
        );
    }

    #[test]
    fn as_string_on_a_non_stringable_object_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    $a as string;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_class_implementing_stringable_converts_at_every_site_with_no_diagnostic() {
        let diags = check_src(
            "<?mwl\n\
             class Name implements Stringable {\n\
             \x20 function toString(): string { return \"x\"; }\n\
             }\n\
             class T {\n\
             \x20 function m(): void {\n\
             \x20\x20 Name $a = new Name();\n\
             \x20\x20 string $s1 = \"\" . $a;\n\
             \x20\x20 string $s2 = \"value: $a\";\n\
             \x20\x20 string $s3 = $a as string;\n\
             \x20\x20 echo $a;\n\
             \x20\x20 print $a;\n\
             \x20 }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn unset_on_a_declared_property_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo { public int $x; function constructor() { $this->x = 1; } }\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    unset($a->x);\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNSET_ON_PROPERTY)),
            "{diags:?}"
        );
    }

    #[test]
    fn unset_on_a_local_variable_is_unaffected() {
        let diags = check_in_method("mixed $x = 1;\nunset($x);");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ADR 0036 § 1: `object` is the real supertype of every class type.

    #[test]
    fn a_class_instance_is_assignable_to_object() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    object $o = new Foo();\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_object_literal_is_assignable_to_object() {
        let diags = check_in_method("object $o = {x: 1};");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_scalar_is_not_assignable_to_object() {
        let diags = check_in_method("object $o = 1;");
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    // ADR 0036 § 3: a shape type is checked structurally, by width subtyping
    // plus ordinary field-type assignability.

    #[test]
    fn an_object_literal_with_exactly_the_shapes_fields_is_fine() {
        let diags = check_in_method("({x: int, y: int}) $p = {x: 1, y: 2};");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_object_literal_with_extra_fields_still_satisfies_a_narrower_shape() {
        // Width subtyping: a source with extra fields beyond the shape still
        // satisfies it.
        let diags = check_in_method("({x: int}) $p = {x: 1, y: 2};");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_object_literal_missing_a_shapes_field_is_diagnosed() {
        let diags = check_in_method("({x: int, y: int}) $p = {x: 1};");
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn an_object_literal_with_a_mismatched_field_type_is_diagnosed() {
        let diags = check_in_method(r#"({x: int}) $p = {x: "s"};"#);
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_class_with_a_matching_property_satisfies_a_shape_type() {
        let diags = check_src(
            "<?mwl\nclass Foo {\n  public int $x = 0;\n}\nclass T {\n  function m(): void {\n    ({x: int}) $p = new Foo();\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_class_missing_a_shapes_field_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    ({x: int}) $p = new Foo();\n  }\n}\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_shape_type_alias_resolves_like_any_other_alias() {
        let diags = check_src(
            "<?mwl\ntype Point = {x: int, y: int};\nclass T {\n  function m(): void {\n    Point $p = {x: 1, y: 2};\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ADR 0036 § 4: property access through an erased view.

    #[test]
    fn reading_a_field_a_shape_names_recovers_its_type_with_no_diagnostic() {
        let diags = check_in_method("({x: int}) $p = {x: 1};\nint $n = $p->x;");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn reading_a_field_a_shape_does_not_name_is_erased_with_no_diagnostic() {
        // Deferred to ADR 0014 § 5's runtime-checked fallback (M4) — this
        // compile-time checker cannot know either way, so it reports
        // nothing rather than guessing.
        let diags = check_in_method("({x: int}) $p = {x: 1};\nmixed $n = $p->y;");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn reading_a_field_through_plain_object_is_erased_with_no_diagnostic() {
        let diags = check_in_method("object $o = {x: 1};\nmixed $n = $o->x;");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ADR 0010 § 4-5: an enum's name is a real, distinct type — a case
    // access recovers `Ty::Enum`, not `mixed`, so it type-checks like any
    // other declared type rather than accepting anything at all.

    #[test]
    fn an_enum_case_access_types_as_its_enum() {
        let diags = check_src(
            "<?mwl\nenum Status { Active, Banned }\nclass T {\n  function m(): void {\n    Status $s = Status::Active;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn an_enum_case_assigned_into_a_different_enums_local_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nenum Status { Active, Banned }\nenum Color { Red, Blue }\nclass T {\n  function m(): void {\n    Color $c = Status::Active;\n  }\n}\n",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "`Status::Active` should type as `Status`, not `mixed`, so this must mismatch: {diags:?}"
        );
    }

    #[test]
    fn arithmetic_directly_on_an_enum_case_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nenum Permission: uint { Read = 1, Write = 2 }\nclass T {\n  function m(): void {\n    Permission::Read + Permission::Write;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ENUM_ARITHMETIC_UNSUPPORTED)),
            "{diags:?}"
        );
    }

    #[test]
    fn bitwise_or_directly_on_an_enum_case_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nenum Permission: uint { Read = 1, Write = 2 }\nclass T {\n  function m(): void {\n    Permission::Read | Permission::Write;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ENUM_ARITHMETIC_UNSUPPORTED)),
            "{diags:?}"
        );
    }

    #[test]
    fn converting_an_enum_case_to_its_underlying_type_is_fine() {
        let diags = check_src(
            "<?mwl\nenum Permission: uint { Read = 1, Write = 2 }\nclass T {\n  function m(): void {\n    uint $bits = Permission::Write as uint;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn converting_one_enum_to_a_different_enum_via_as_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nenum Status { Active, Banned }\nenum Color { Red, Blue }\nclass T {\n  function m(): void {\n    Status::Active as Color;\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ENUM_CONVERSION_UNSUPPORTED)),
            "{diags:?}"
        );
    }

    #[test]
    fn converting_an_enum_to_itself_via_as_is_fine() {
        let diags = check_src(
            "<?mwl\nenum Status { Active, Banned }\nclass T {\n  function m(): void {\n    Status::Active as Status;\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ADR 0024 §§ 2-3: `tainted` propagation and laundering.

    #[test]
    fn a_plain_string_is_assignable_into_a_tainted_typed_target() {
        // ADR 0024 § 2: a trusted value is always a safe over-approximation
        // of "may be tainted" — the one-directional widening this ADR adds,
        // mirrored from `mixed`'s own one-directional rule.
        let diags = check_in_method(r#"tainted string $t = "literal";"#);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_tainted_value_is_not_assignable_into_a_plain_typed_target() {
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             string $s = $t;\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn concatenating_a_tainted_operand_poisons_the_result() {
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             string $s = $t . \"x\";\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn interpolating_a_tainted_operand_poisons_the_result() {
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             string $s = \"value: $t\";\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn concatenating_two_untainted_operands_stays_untainted() {
        let diags = check_in_method(r#"string $s = "a" . "b";"#);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_checked_conversion_launders_a_tainted_source() {
        // ADR 0024 § 2's own example: `as uint` already throws on a
        // malformed shape, so a value that survives it is proven safe.
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             uint $n = $t as uint;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn as_string_does_not_launder_a_tainted_source() {
        // The identity-shaped conversion this ADR must not treat as
        // laundering — otherwise `$tainted as string` would be a silent
        // bypass of the whole mechanism.
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             string $s = $t as string;\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn converting_a_tainted_string_to_bytes_preserves_the_qualifier() {
        // ADR 0009 § 3, amended by ADR 0024 § 2: `bytes`/`string` conversion
        // preserves `tainted` across either direction.
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             tainted bytes $b = $t as bytes;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn converting_a_tainted_string_to_bytes_does_not_launder_it() {
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             bytes $b = $t as bytes;\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn converting_a_literal_string_to_markup_is_fine() {
        let diags = check_in_method("Core\\Html\\Markup $m = \"literal\" as Core\\Html\\Markup;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn converting_a_tainted_string_to_markup_is_diagnosed() {
        let diags = check_in_method(
            "tainted string $t = \"literal\" as tainted string;\n\
             Core\\Html\\Markup $m = $t as Core\\Html\\Markup;\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_MARKUP_REQUIRES_LITERAL)),
            "{diags:?}"
        );
    }

    #[test]
    fn converting_a_runtime_computed_untainted_string_to_markup_is_still_diagnosed() {
        // ADR 0024 § 5: only a literal token qualifies — even an untainted
        // runtime value is refused.
        let diags = check_in_method(
            "string $s = \"literal\";\n\
             Core\\Html\\Markup $m = $s as Core\\Html\\Markup;\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_MARKUP_REQUIRES_LITERAL)),
            "{diags:?}"
        );
    }

    // ADR 0033 §§ 2-4: `secret`, the same shape as `tainted` on an
    // independent axis.

    #[test]
    fn a_plain_string_is_assignable_into_a_secret_typed_target() {
        let diags = check_in_method(r#"secret string $s = "literal";"#);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_secret_value_is_not_assignable_into_a_plain_typed_target() {
        let diags = check_in_method(
            "secret string $s = \"literal\";\n\
             string $out = $s;\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn concatenating_a_secret_operand_poisons_the_result_independently_of_tainted() {
        // `secret` poisons through concatenation exactly like `tainted`,
        // with no `tainted` qualifier anywhere in sight — the two axes are
        // independent.
        let diags = check_in_method(
            "secret string $s = \"literal\";\n\
             string $out = $s . \"x\";\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn interpolating_a_secret_operand_poisons_the_result() {
        let diags = check_in_method(
            "secret string $s = \"literal\";\n\
             string $out = \"value: $s\";\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn a_secret_tainted_operand_poisons_both_axes_through_concatenation() {
        let diags = check_in_method(
            "secret tainted string $s = \"literal\" as secret tainted string;\n\
             tainted string $out = $s . \"x\";\n",
        );
        // `secret tainted` concatenated with a plain string stays poisoned on
        // both axes, so it satisfies neither a plain-`tainted` target...
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
        let diags = check_in_method(
            "secret tainted string $s = \"literal\" as secret tainted string;\n\
             secret tainted string $out = $s . \"x\";\n",
        );
        // ...but does satisfy the same, fully-qualified target.
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_checked_conversion_strips_secret_the_same_way_it_strips_tainted() {
        // ADR 0033 § 2's known, accepted gap: "shape-proof implies safe"
        // never actually justified stripping `secret`, but the rule is kept
        // for consistency with `tainted` anyway.
        let diags = check_in_method(
            "secret tainted string $s = \"literal\" as secret tainted string;\n\
             uint $n = $s as uint;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn as_string_does_not_launder_a_secret_source() {
        let diags = check_in_method(
            "secret string $s = \"literal\";\n\
             string $out = $s as string;\n",
        );
        assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
    }

    #[test]
    fn converting_a_secret_string_to_bytes_preserves_the_qualifier() {
        let diags = check_in_method(
            "secret string $s = \"literal\";\n\
             secret bytes $b = $s as bytes;\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_secret_value_converted_to_markup_is_diagnosed_naming_secret() {
        let diags = check_in_method(
            "secret string $s = \"literal\";\n\
             Core\\Html\\Markup $m = $s as Core\\Html\\Markup;\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_SECRET_MARKUP_UNSUPPORTED)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_secret_value_passed_directly_to_a_throwable_is_diagnosed() {
        let diags = check_in_method(
            "secret string $s = \"literal\";\n\
             throw new LogicError($s);\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_SECRET_THROWABLE_MESSAGE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_secret_value_passed_to_a_subclass_of_throwable_is_diagnosed() {
        let diags = check_src(
            "<?mwl\n\
             class MyError extends Throwable {}\n\
             class T {\n  function m(secret string $s): void {\n\
             throw new MyError($s);\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_SECRET_THROWABLE_MESSAGE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_plain_value_passed_to_a_throwable_is_fine() {
        let diags = check_in_method(r#"throw new LogicError("plain message");"#);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ADR 0027: `callable` is satisfied by exactly one shape of value.

    #[test]
    fn a_bare_string_where_callable_is_expected_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function run(callable $fn): void {}\n  function m(): void {\n    $this->run(\"strlen\");\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_CALLABLE_STRING_UNSUPPORTED)),
            "{diags:?}"
        );
    }

    #[test]
    fn an_array_callable_spelling_where_callable_is_expected_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function run(callable $fn): void {}\n  function m(): void {\n    $this->run([$this, \"m\"]);\n  }\n}\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_CALLABLE_ARRAY_UNSUPPORTED)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_first_class_callable_reference_satisfies_a_callable_parameter() {
        let diags = check_src(
            "<?mwl\nclass T {\n  function run(callable $fn): void {}\n  function target(): void {}\n  function m(): void {\n    $this->run($this->target(...));\n  }\n}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn calling_a_non_callable_object_is_diagnosed() {
        let diags = check_src(
            "<?mwl\nclass Adder {\n  function add(int $a, int $b): int { return $a + $b; }\n}\nclass T {\n  function m(): void {\n    Adder $adder = new Adder();\n    $adder(1, 2);\n  }\n}\n",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_NOT_CALLABLE)),
            "{diags:?}"
        );
    }

    #[test]
    fn calling_a_closure_value_is_unaffected() {
        let diags = check_in_method("callable $fn = fn(): int => 1;\n$fn();\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // ------------------------------------------------------------------
    // ADR 0043 §§ 2-3 -- interface default/private methods (M2 follow-up).
    // ------------------------------------------------------------------

    /// § 2's own worked example: `Person` never declares `greet()` itself,
    /// but inherits it from `Greets` exactly like an ordinary inherited
    /// method — the same `resolve_method` ancestor walk `extends` already
    /// used, now also walking `implements`.
    #[test]
    fn a_default_interface_method_is_inherited_and_callable() {
        let diags = check_src(
            "<?mwl\n\
             interface Greets {\n\
             \x20 public function name(): string;\n\
             \x20 public function greet(): string { return \"Hello, \" . $this->name() . \"!\"; }\n\
             }\n\
             class Person implements Greets {\n\
             \x20 private string $personName;\n\
             \x20 function constructor(string $personName) { $this->personName = $personName; }\n\
             \x20 public function name(): string { return $this->personName; }\n\
             }\n\
             class T {\n\
             \x20 function m(): void {\n\
             \x20\x20 Person $p = new Person(\"Ada\");\n\
             \x20\x20 string $g = $p->greet();\n\
             \x20 }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// A class overriding a default method wins over the interface's own
    /// body — an ordinary override, resolved because `resolve_method` checks
    /// a class's own signature table before ever walking to `implements`.
    /// Proven here by a signature-shape change (`string` vs `int`) that would
    /// mismatch if the stale interface signature were used instead of the
    /// override's own.
    #[test]
    fn a_class_can_override_a_default_interface_method() {
        let diags = check_src(
            "<?mwl\n\
             interface Greets { public function greet(string $x): void {} }\n\
             class Person implements Greets {\n\
             \x20 public function greet(int $x): void {}\n\
             }\n\
             class T {\n\
             \x20 function m(): void {\n\
             \x20\x20 Person $p = new Person();\n\
             \x20\x20 $p->greet(1);\n\
             \x20 }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// § 2: `$this` inside an interface's own default method body is typed as
    /// that interface, not whatever concrete class ends up implementing it —
    /// so a member only the *implementing* class declares is not reachable
    /// through `$this` from inside the interface's own body, even though the
    /// same call would resolve fine from inside the implementing class.
    #[test]
    fn this_inside_a_default_method_body_does_not_see_the_implementing_class() {
        let diags = check_src(
            "<?mwl\n\
             interface Greets {\n\
             \x20 public function greet(): string { return $this->onlyOnPerson(); }\n\
             }\n\
             class Person implements Greets {\n\
             \x20 public function onlyOnPerson(): string { return \"x\"; }\n\
             }\n",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
            "{diags:?}"
        );
    }

    /// § 3: a `private` interface method is callable from another method of
    /// the *same* interface (here, a second default method), and the default
    /// method that calls it is itself inherited and externally callable —
    /// privacy only closes the helper itself, not the contract around it.
    #[test]
    fn a_private_interface_method_is_visible_from_its_own_interfaces_default_method() {
        let diags = check_src(
            "<?mwl\n\
             interface Csv {\n\
             \x20 private function escapeField(string $field): string { return $field; }\n\
             \x20 public function toCsvRow(): string { return $this->escapeField(\"x\"); }\n\
             }\n\
             class Report implements Csv {}\n\
             class T {\n\
             \x20 function m(): void {\n\
             \x20\x20 Report $r = new Report();\n\
             \x20\x20 string $s = $r->toCsvRow();\n\
             \x20 }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// § 3's own diagnostic: an implementing class reaching for the private
    /// helper directly via `$this->` is `E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE`
    /// — it was never part of the interface's contract.
    #[test]
    fn a_private_interface_method_is_not_visible_through_this_from_an_implementing_class() {
        let diags = check_src(
            "<?mwl\n\
             interface Csv {\n\
             \x20 private function escapeField(string $field): string { return $field; }\n\
             \x20 public function toCsvRow(): string { return $this->escapeField(\"x\"); }\n\
             }\n\
             class Report implements Csv {\n\
             \x20 public function toCsvRow(): string { return $this->escapeField(\"y\"); }\n\
             }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE)),
            "{diags:?}"
        );
    }

    /// The same refusal applies through an explicit qualified call
    /// (`InterfaceName::method()`, § 5's grammar for reaching a specific
    /// default explicitly) — a private helper stays invisible outside its own
    /// interface regardless of which call form reaches for it.
    #[test]
    fn a_private_interface_method_is_not_visible_via_a_qualified_call_from_outside() {
        let diags = check_src(
            "<?mwl\n\
             interface Csv {\n\
             \x20 private function escapeField(string $field): string { return $field; }\n\
             }\n\
             class Report implements Csv {\n\
             \x20 public function useIt(): string { return Csv::escapeField(\"z\"); }\n\
             }\n",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE)),
            "{diags:?}"
        );
    }

    // ADR 0008 § 2: a file's top-level statements are one synthesized frame
    // whose variables are locals. Each of these has an exact counterpart in
    // the `check_in_method` fixtures above — the point is that the two
    // report identically.

    #[test]
    fn reading_an_undeclared_local_at_file_scope_is_diagnosed() {
        let diags = check_src("<?mwl\necho $missing;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_declared_and_assigned_file_scope_local_reads_fine() {
        let diags = check_src("<?mwl\nint $n = 1;\n$n = $n + 1;\necho $n;\n");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn redeclaring_a_file_scope_local_is_diagnosed() {
        let diags = check_src("<?mwl\nint $n = 1;\nint $n = 2;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_REDECLARED_LOCAL)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_file_scope_type_mismatch_is_diagnosed() {
        let diags = check_src("<?mwl\nint $n = 1;\n$n = \"x\";\n");
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{diags:?}"
        );
    }

    /// `$this` has no meaning at file scope — there is no enclosing class,
    /// so it is an ordinary undeclared name rather than a special case.
    #[test]
    fn this_at_file_scope_is_an_undeclared_local() {
        let diags = check_src("<?mwl\necho $this;\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
            "{diags:?}"
        );
    }

    /// A `namespace { ... }` block scopes *names*, not storage: its top-level
    /// statements land in the same synthesized frame as every other one in
    /// the file, so a redeclaration across two blocks still conflicts.
    #[test]
    fn a_namespace_block_shares_the_one_script_frame() {
        let diags = check_src("<?mwl\nnamespace A { int $n = 1; }\nnamespace B { int $n = 2; }\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_REDECLARED_LOCAL)),
            "{diags:?}"
        );
    }

    /// A class body is still its own frame — a file-scope local is not
    /// visible from inside a method (ADR 0008's storage-class table: "the
    /// script's own frame, unreachable from a function").
    #[test]
    fn a_file_scope_local_is_not_visible_inside_a_method() {
        let diags =
            check_src("<?mwl\nint $n = 1;\nclass T {\n  function m(): void { echo $n; }\n}\n");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
            "{diags:?}"
        );
    }

    // ------------------------------------------------------------------
    // By-reference parameters -- the two obligations `check_by_ref_arg`
    // adds on top of ordinary assignability. See `mwl_ir::Ty::Ref` for the
    // representation those obligations exist to keep sound.
    // ------------------------------------------------------------------

    #[test]
    fn a_by_reference_argument_that_is_a_local_is_accepted() {
        let diags = check_src(
            "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
               function m(): void {
    int $n = 1;
    T::bump($n);
  }
}
",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_by_reference_argument_that_is_a_property_is_accepted() {
        let diags = check_src(
            "<?mwl
class T {
  public int $hits;
               function constructor(int $hits) { $this->hits = $hits; }
               static function bump(int &$s): void { $s = $s + 1; }
               function m(): void {
    T::bump($this->hits);
  }
}
",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_literal_passed_by_reference_is_diagnosed() {
        let diags = check_src(
            "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
               function m(): void {
    T::bump(1);
  }
}
",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_calls_own_result_passed_by_reference_is_diagnosed() {
        let diags = check_src(
            "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
               static function one(): int { return 1; }
               function m(): void {
    T::bump(T::one());
  }
}
",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
            "{diags:?}"
        );
    }

    /// ADR 0007 § 5's copy-on-write leaves an element no stable address, and
    /// the staged-slot model has no write-back path for one either -- so this
    /// is refused with a message that says which of the two it is.
    #[test]
    fn an_array_element_passed_by_reference_is_diagnosed() {
        let diags = check_src(
            "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
               function m(): void {
    array<int> $a = [1];
    T::bump($a[0]);
  }
}
",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
            "{diags:?}"
        );
    }

    /// ADR 0014 section 1 makes a hooked read a call and a hooked write a
    /// second one, so there is no slot to hand a callee.
    #[test]
    fn a_hooked_property_passed_by_reference_is_diagnosed() {
        let diags = check_src(
            "<?mwl
class T {
  public int $n;
               public int $doubled { get => $this->n * 2; set(int $v) { $this->n = $v; } }
               function constructor(int $n) { $this->n = $n; }
               static function bump(int &$s): void { $s = $s + 1; }
               function m(): void {
    T::bump($this->doubled);
  }
}
",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
            "{diags:?}"
        );
    }

    /// A plain `string` satisfies a `tainted string` parameter by value --
    /// ADR 0024 section 2's "a plain value is always a safe
    /// over-approximation of may-be-tainted" -- but never by reference: the
    /// callee writes a `tainted string` back, and the caller's holder is
    /// declared plain. Accepting it would launder taint through an argument
    /// list, which is exactly the hole ADR 0024 exists to close, and it is
    /// why assignability alone is not enough at a `&` position.
    #[test]
    fn a_plain_string_passed_to_a_tainted_reference_parameter_is_diagnosed() {
        let diags = check_src(
            "<?mwl
class T {
               static function fill(tainted string &$s): void { $s = $s; }
               function m(): void {
    string $t = \"x\";
    T::fill($t);
  }
}
",
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_BY_REF_ARG_TYPE_NOT_EXACT)),
            "{diags:?}"
        );
    }

    /// The exactness check is reported only for an argument that would
    /// otherwise have been accepted -- an outright mismatch already has
    /// `E_TYPE_MISMATCH` at the same span, and two diagnostics for one
    /// mistake helps nobody.
    #[test]
    fn an_unassignable_by_reference_argument_is_diagnosed_only_once() {
        let diags = check_src(
            "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
               function m(): void {
    string $t = \"x\";
    T::bump($t);
  }
}
",
        );
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "{diags:?}"
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_BY_REF_ARG_TYPE_NOT_EXACT)),
            "{diags:?}"
        );
    }
}
