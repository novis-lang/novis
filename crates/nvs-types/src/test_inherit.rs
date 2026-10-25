//! `rule:testing/hook-inheritance`: a subclass inherits each hook, each
//! `#[Fixture]`, each `#[Test]` and each `#[Property]` of its ancestors, the way
//! it inherits any method.
//!
//! # At check time
//!
//! [`check_class`] reads the ancestors' marked methods off the signature table
//! and the attribute table. Both are built for the whole program before any
//! class is checked, so a base class declared in a later file is read the same
//! way as one declared above. It makes the rule's two refusals: an override
//! that does not repeat the attribute of the method it overrides, and a second
//! hook of one kind beside an inherited one. What it returns is the inherited
//! roster, which [`crate::testing::check_class_tests`] resolves the class's own
//! parameters against, so a subclass's own test receives an inherited fixture
//! by its type.
//!
//! # After every file is checked
//!
//! [`merge`] builds each subclass's four tables out of its ancestors'
//! recorded ones and its own, once every class has recorded its own. The
//! nearest declaration of a name wins, and keeps the place the first
//! declaration of that name had, so an inherited test is reported where its
//! base class declared it. An abstract class's tests then leave the table: the
//! runner never constructs one, and each concrete subclass runs them under its
//! own name.
//!
//! An inherited fixture or class hook is `static`, and its compiled function
//! is the declaring class's, so each row names that class
//! ([`crate::testing::Fixture::declared_in`],
//! [`crate::test_hooks::HookMethod::declared_in`]) and the runner calls it with
//! the subclass as the called class.
//!
//! **What it spends:** each subclass of a test class holds its own copy of
//! every row it inherits, for as long as the compiled program's tables live. A
//! class with no marked ancestor records nothing.

use nvs_diagnostics::{Code, Diagnostic, code};
use nvs_hir::{ClassGraph, QName};
use nvs_syntax::ast::{ClassDecl, ClassMemberKind};
use rustc_hash::FxHashSet;

use crate::expr_table::ExprTypeTable;
use crate::property::PropertyCase;
use crate::test_hooks::TestHooks;
use crate::testing::{Fixture, TestCase, attribute_named};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// The seven test attributes, each with the way a diagnostic writes it.
const MARKERS: [(&str, &str); 7] = [
    (crate::derive::TEST, "`#[Test]`"),
    (crate::derive::BENCH, "`#[Bench]`"),
    (crate::derive::PROPERTY, "`#[Property]`"),
    (crate::derive::FIXTURE, "`#[Fixture]`"),
    (crate::derive::BEFORE_CLASS, "`#[BeforeClass]`"),
    (crate::derive::AFTER_CLASS, "`#[AfterClass]`"),
    (crate::derive::AFTER_EACH_TEST, "`#[AfterEachTest]`"),
];

/// One marked method a class inherits.
pub(crate) struct Inherited {
    /// The method's name.
    pub method: String,
    /// The nearest ancestor that declares it.
    pub owner: QName,
    /// Its attribute's qualified name, one of [`MARKERS`].
    pub marker: &'static str,
}

/// `class`'s ancestors through `extends`, nearest first. A cycle is
/// `nvs_hir`'s refusal, so the walk stops at the first class it meets twice.
fn ancestors(class: &QName, graph: &ClassGraph) -> Vec<QName> {
    let mut out: Vec<QName> = Vec::new();
    let mut next = graph.get(class).and_then(|links| links.extends.first());
    while let Some(parent) = next {
        if parent == class || out.contains(parent) {
            break;
        }
        out.push(parent.clone());
        next = graph.get(parent).and_then(|links| links.extends.first());
    }
    out
}

/// The test attribute `class`'s own method `method` carries, or `None`.
fn marker_of(class: &QName, method: &str, env: &Env<'_>) -> Option<&'static str> {
    MARKERS
        .iter()
        .map(|(name, _)| *name)
        .find(|name| env.attributes.method_carries(class, method, name))
}

/// How a diagnostic writes `marker`, and the code an override of it that
/// drops the attribute is refused under: each kind's own shape code.
fn described(marker: &str) -> (&'static str, Code) {
    let written = MARKERS
        .iter()
        .find(|(name, _)| *name == marker)
        .map_or("", |(_, written)| *written);
    let code = if marker == crate::derive::TEST
        || marker == crate::derive::BENCH
        || marker == crate::derive::PROPERTY
    {
        code::E_TEST_METHOD_SHAPE
    } else if marker == crate::derive::FIXTURE {
        code::E_FIXTURE_METHOD_SHAPE
    } else {
        code::E_TEST_HOOK_SHAPE
    };
    (written, code)
}

/// Whether `marker` is one of the three hook attributes.
fn is_hook(marker: &str) -> bool {
    marker != crate::derive::TEST
        && marker != crate::derive::BENCH
        && marker != crate::derive::PROPERTY
        && marker != crate::derive::FIXTURE
}

/// Every marked method `class` inherits that no nearer ancestor redeclares,
/// nearest ancestor first and by name within one.
fn inherited(class: &QName, env: &Env<'_>) -> Vec<Inherited> {
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    let mut out = Vec::new();
    for owner in ancestors(class, env.graph) {
        let Some(sig) = env.signatures.get(&owner) else {
            continue;
        };
        let mut names: Vec<&str> = sig.methods.keys().map(String::as_str).collect();
        names.sort_unstable();
        for name in names {
            if !seen.insert(name) {
                continue;
            }
            if let Some(marker) = marker_of(&owner, name, env) {
                out.push(Inherited {
                    method: name.to_owned(),
                    owner: owner.clone(),
                    marker,
                });
            }
        }
    }
    out
}

/// The fixtures `decl` inherits and does not declare itself, as a roster row
/// each: the method's name, the type it supplies, and the ancestor that
/// declares it. `rule:testing/hook-inheritance`'s two refusals are made here
/// as well, and a class that inherits a marked method is recorded for
/// [`merge`].
pub(crate) fn check_class(
    decl: &ClassDecl,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Vec<(String, TypeId, QName)> {
    let inherited = inherited(class, env);
    if inherited.is_empty() {
        return Vec::new();
    }
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let method = span_text(env.src, m.name);
        let own = MARKERS
            .iter()
            .map(|(name, _)| *name)
            .find(|name| attribute_named(&m.attributes, name, ctx, env).is_some());
        if let Some(base) = inherited.iter().find(|row| row.method == method) {
            if own == Some(base.marker) {
                continue;
            }
            let (written, code) = described(base.marker);
            let did = own.map_or_else(
                || "does not repeat its attribute".to_owned(),
                |other| format!("is {} instead", described(other).0),
            );
            env.diags.report(
                Diagnostic::error(
                    code,
                    format!(
                        "`{class}::{method}` overrides the {written} method `{}::{method}` and {did}",
                        base.owner
                    ),
                )
                .with_primary(m.name, format!("write {written} above this method"))
                .with_help(
                    "an override repeats the attribute, so a reader of this class alone sees what \
                     the test runner does with the method",
                ),
            );
            continue;
        }
        let Some(own) = own.filter(|own| is_hook(own)) else {
            continue;
        };
        if let Some(base) = inherited.iter().find(|row| row.marker == own) {
            let (written, _) = described(own);
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!(
                        "`{class}` has two {written} methods, `{method}` and `{}::{}`, which it \
                         inherits",
                        base.owner, base.method
                    ),
                )
                .with_primary(
                    m.name,
                    format!("`{}` is already this class's hook", base.method),
                )
                .with_help(format!(
                    "a class has one hook of each kind, so override `{0}` instead, and call \
                     `parent::{0}()` to keep its steps",
                    base.method
                )),
            );
        }
    }
    let lineage = ancestors(class, env.graph)
        .iter()
        .map(ToString::to_string)
        .collect();
    env.exprs.record_test_lineage(class.to_string(), lineage);
    let own = env.signatures.get(class);
    inherited
        .into_iter()
        .filter(|row| row.marker == crate::derive::FIXTURE)
        .filter(|row| own.is_none_or(|sig| !sig.methods.contains_key(&row.method)))
        .filter_map(|row| {
            let ty = env
                .signatures
                .get(&row.owner)?
                .methods
                .get(&row.method)?
                .return_ty;
            (!matches!(env.interner.get(ty), Ty::Void)).then_some((row.method, ty, row.owner))
        })
        .collect()
}

/// Builds every recorded subclass's tables out of its ancestors' and its own,
/// then removes each abstract class's tests. See the module docs.
pub(crate) fn merge(graph: &ClassGraph, exprs: &mut ExprTypeTable) {
    let mut built = Vec::new();
    for (class, ancestors) in exprs.take_test_lineages() {
        let mut tests: Vec<TestCase> = Vec::new();
        let mut properties: Vec<PropertyCase> = Vec::new();
        let mut fixtures: Vec<Fixture> = Vec::new();
        let mut hooks = TestHooks::default();
        for label in ancestors.iter().rev().chain(std::iter::once(&class)) {
            for case in exprs.tests(label).unwrap_or_default() {
                place(&mut tests, case.clone(), |a, b| a.method == b.method);
            }
            for case in exprs.properties(label).unwrap_or_default() {
                place(&mut properties, case.clone(), |a, b| a.method == b.method);
            }
            for fixture in exprs.fixtures(label).unwrap_or_default() {
                place(&mut fixtures, fixture.clone(), |a, b| a.method == b.method);
            }
            if let Some(own) = exprs.test_hooks(label) {
                let own = own.clone();
                hooks.before_class = own.before_class.or(hooks.before_class);
                hooks.after_class = own.after_class.or(hooks.after_class);
                hooks.after_each_test = own.after_each_test.or(hooks.after_each_test);
            }
        }
        built.push((class, tests, properties, fixtures, hooks));
    }
    for (class, tests, properties, fixtures, hooks) in built {
        exprs.replace_test_tables(&class, tests, properties, fixtures, hooks);
    }
    let abstract_classes: Vec<String> = exprs
        .test_classes()
        .into_iter()
        .filter(|label| {
            graph
                .get(&QName::parse(label))
                .is_some_and(|links| !links.concrete)
        })
        .map(str::to_owned)
        .collect();
    for label in abstract_classes {
        exprs.forget_tests(&label);
    }
}

/// Puts `row` in the place of the row `same` matches, or at the end.
fn place<T>(rows: &mut Vec<T>, row: T, same: impl Fn(&T, &T) -> bool) {
    match rows.iter().position(|already| same(already, &row)) {
        Some(index) => rows[index] = row,
        None => rows.push(row),
    }
}
