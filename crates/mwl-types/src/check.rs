//! Entry point: walks a resolved [`Module`]'s classes and methods, type-
//! checking each method body against ADR 0007 §§ 1-4 (see the crate docs for
//! the exact scope of this slice).
//!
//! Mirrors [`mwl_hir::members`]'s own walk shape: [`check_stmts`] tracks
//! namespace/`use` scope the same way (there is no enclosing-class scope to
//! track at this level — a fresh [`Ctx`] naming the class is built right at
//! each declaration site instead), recursing into each class/interface/
//! trait/enum's methods via [`check_members`]. A method with no body
//! (abstract, or an interface signature) has nothing to check.
//! [`check_method`] seeds a fresh [`crate::locals::LocalScope`] from the
//! method's own lowered parameters (already definitely assigned), lowers its
//! return type once, and hands the body to [`crate::locals::check_block`].
//! Right after a `ClassDecl`'s members are checked this way,
//! [`crate::ctor_init::check_class_init`] runs its own, separate
//! constructor-only pass over the same declaration for ADR 0022 § 2 —
//! interfaces/traits/enums never get that call, since only a class is ever
//! instantiated through a constructor.
//!
//! **Known gap:** a class/interface/trait/enum declared *inside* a method
//! body is not descended into here at all — only top-level declarations (and
//! ones nested in a `namespace { ... }` block) are found by [`check_stmts`].

use mwl_diagnostics::{Diagnostics, SourceFile};
use mwl_hir::{Module, QName};
use mwl_syntax::ast::{
    ClassMember, ClassMemberKind, MethodMember, Name, NamespaceDecl, Stmt, StmtKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ctor_init::check_class_init;
use crate::expr::class_of_ctx;
use crate::locals::{LocalScope, check_block};
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
/// shared.
pub fn check_program(
    stmts: &[Stmt],
    src: &SourceFile,
    module: &Module,
    interner: &mut TypeInterner,
    diags: &mut Diagnostics,
) {
    let signatures = build_signatures(
        stmts,
        &module.symbols,
        &module.aliases,
        &module.graph,
        src,
        interner,
        diags,
    );
    let mut env = Env {
        symbols: &module.symbols,
        aliases: &module.aliases,
        graph: &module.graph,
        signatures: &signatures,
        src,
        interner,
        diags,
    };
    check_stmts(stmts, &[], &FxHashMap::default(), &mut env);
}

fn check_stmts(
    stmts: &[Stmt],
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
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
                        check_stmts(&block.stmts, &new_ns, &FxHashMap::default(), env);
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
                };
                check_members(&decl.members, &ctx, env);
                check_class_init(decl, &qname, env);
            }
            StmtKind::InterfaceDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                };
                check_members(&decl.members, &ctx, env);
            }
            StmtKind::TraitDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                };
                check_members(&decl.members, &ctx, env);
            }
            StmtKind::EnumDecl(decl) => {
                let qname = QName::join(&current_ns, span_text(env.src, decl.name.span));
                let ctx = Ctx {
                    namespace: &current_ns,
                    imports: &current_imports,
                    current_class: Some(&qname),
                };
                check_members(&decl.members, &ctx, env);
            }
            _ => {}
        }
    }
}

fn check_members(members: &[ClassMember], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for member in members {
        if let ClassMemberKind::Method(m) = &member.kind {
            check_method(m, ctx, env);
        }
    }
}

fn check_method(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
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
        check_program(&stmts, map.file(file), &module, &mut interner, &mut diags);
        diags
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
}
