//! `rule:testing/class-hooks` and `rule:testing/after-each-test`: the three
//! hook attributes, `#[BeforeClass]`, `#[AfterClass]` and `#[AfterEachTest]`,
//! and what the method under each one has to be.
//!
//! All three are on [`crate::derive::ATTRIBUTES`] and are matched nominally,
//! exactly as `#[Fixture]` is, and none of them carries a payload: a field
//! written on one is [`code::E_UNKNOWN_OPTION`], the same refusal a field on
//! `#[Fixture]` draws.
//!
//! # The shape
//!
//! [`check_shape`] reads the resolved signature, for the reason
//! [`crate::testing`]'s `check_method_shape` gives. The two class hooks run in
//! the parent before any test object exists, so they are `static`; the
//! per-test hook runs on the object its test ran on, so it is not. Each is
//! `public` because the runner calls it from outside its class, returns
//! `void` because nothing reads what it returns, and has a body because it is
//! called. Every refusal is [`code::E_TEST_HOOK_SHAPE`], worded with the hook
//! it is about.
//!
//! The one optional parameter is matched by the name its type is written
//! with, resolved through the file's namespace and imports, which is the same
//! nominal match the attribute itself is: `Core\Test\Summary` for
//! `#[AfterClass]`, `Core\Test\Outcome` for `#[AfterEachTest]`, and nothing for
//! `#[BeforeClass]`.
//!
//! # One hook of each kind
//!
//! `rule:testing/hook-inheritance` gives a class at most one hook of each
//! kind, so a second is [`code::E_DUPLICATE_DECLARATION`], naming both
//! methods: it is one declaration made twice, as two fixtures of one type are.
//! A method carrying two hook attributes, or a hook attribute beside `#[Test]`
//! or `#[Fixture]`, is a shape refusal instead, and
//! [`crate::testing::check_class_tests`] leaves such a method to this module,
//! so it takes one diagnostic.
//!
//! # No static write in a class hook
//!
//! `rule:testing/class-hooks` refuses a write to a static property in a class
//! hook's own body, because every test reads the declared initial value in its
//! own isolate. [`static_writes`] finds one in the syntax tree
//! ([`nvs_syntax::walk`]), so a new expression kind is walked as soon as the
//! walk knows it. A write is an assignment of any operator, `++` or `--`, or an
//! `unset`, whose target is a static property or an element of one. A write
//! through an object a static property holds is not refused: it writes the
//! object, and that object is fresh in each isolate as well.
//!
//! **What it spends:** one syntax tree of the hook's body, built while
//! compiling and dropped at once, and only for a method carrying a class hook.
//!
//! # What the runner reads
//!
//! [`TestHooks`] is one class's hooks, recorded into [`crate::ExprTypeTable`]
//! beside its tests and read back by `nvs_cli`'s runner. A class that declares
//! no hook records nothing.

use nvs_diagnostics::{Diagnostic, SourceFile, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Attribute, ClassDecl, ClassMemberKind, MethodMember, Visibility};
use nvs_syntax::walk::Node;

use crate::testing::attribute_named;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// One hook method: its name, and whether it declares the one parameter its
/// kind allows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookMethod {
    /// The method's own name, exactly as declared.
    pub method: String,
    /// The label of the class that declares the method, whose compiled
    /// function the runner calls. A subclass that inherits the hook names its
    /// ancestor here (`rule:testing/hook-inheritance`).
    pub declared_in: String,
    /// Whether the method declares a parameter, which the runner fills with a
    /// `Core\Test\Summary` or a `Core\Test\Outcome`.
    pub takes_value: bool,
}

/// The hooks one test class declares, at most one of each kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestHooks {
    /// The `#[BeforeClass]` method, which never takes a value.
    pub before_class: Option<HookMethod>,
    /// The `#[AfterClass]` method.
    pub after_class: Option<HookMethod>,
    /// The `#[AfterEachTest]` method.
    pub after_each_test: Option<HookMethod>,
}

/// One of the three hook kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hook {
    BeforeClass,
    AfterClass,
    AfterEachTest,
}

impl Hook {
    const ALL: [Self; 3] = [Self::BeforeClass, Self::AfterClass, Self::AfterEachTest];

    /// The attribute's qualified name on [`crate::derive::ATTRIBUTES`].
    fn attribute(self) -> &'static str {
        match self {
            Self::BeforeClass => crate::derive::BEFORE_CLASS,
            Self::AfterClass => crate::derive::AFTER_CLASS,
            Self::AfterEachTest => crate::derive::AFTER_EACH_TEST,
        }
    }

    /// The attribute as a diagnostic names it.
    fn written(self) -> &'static str {
        match self {
            Self::BeforeClass => "`#[BeforeClass]`",
            Self::AfterClass => "`#[AfterClass]`",
            Self::AfterEachTest => "`#[AfterEachTest]`",
        }
    }

    /// Whether the hook runs once around the class, in the parent, rather
    /// than once per test attempt.
    fn is_class_level(self) -> bool {
        !matches!(self, Self::AfterEachTest)
    }

    /// The one parameter type the hook may declare, fully qualified.
    fn parameter(self) -> Option<&'static str> {
        match self {
            Self::BeforeClass => None,
            Self::AfterClass => Some(r"Core\Test\Summary"),
            Self::AfterEachTest => Some(r"Core\Test\Outcome"),
        }
    }
}

/// Whether `m` carries any of the three hook attributes.
pub(crate) fn is_hook(m: &MethodMember, ctx: &Ctx<'_>, env: &Env<'_>) -> bool {
    Hook::ALL
        .iter()
        .any(|hook| attribute_named(&m.attributes, hook.attribute(), ctx, env).is_some())
}

/// Every hook `decl` declares, checked: its payload, its shape, one of each
/// kind, and no static write in a class hook's body. The hooks are then
/// recorded as one [`TestHooks`] for the runner.
pub(crate) fn check_class_hooks(decl: &ClassDecl, class: &QName, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut seen: Vec<(Hook, String, bool)> = Vec::new();
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let marked: Vec<(Hook, &Attribute)> = Hook::ALL
            .iter()
            .filter_map(|hook| {
                attribute_named(&m.attributes, hook.attribute(), ctx, env).map(|attr| (*hook, attr))
            })
            .collect();
        let Some(&(hook, _)) = marked.first() else {
            continue;
        };
        for (marker, attr) in &marked {
            check_payload(*marker, attr, env);
        }
        let method = span_text(env.src, m.name).to_owned();
        if let Some(&(other, _)) = marked.get(1) {
            report_shape(
                m.name,
                hook,
                &method,
                &format!("is also {}", other.written()),
                "a method is one hook, so keep one of the two attributes",
                env,
            );
            continue;
        }
        let also = if attribute_named(&m.attributes, crate::derive::TEST, ctx, env).is_some() {
            Some("`#[Test]`")
        } else if attribute_named(&m.attributes, crate::derive::BENCH, ctx, env).is_some() {
            Some("`#[Bench]`")
        } else if attribute_named(&m.attributes, crate::derive::FIXTURE, ctx, env).is_some() {
            Some("`#[Fixture]`")
        } else {
            None
        };
        if let Some(also) = also {
            report_shape(
                m.name,
                hook,
                &method,
                &format!("is also a {also}"),
                "the runner calls a hook around the tests, so a method is a hook or a test or a \
                 fixture",
                env,
            );
            continue;
        }
        check_shape(m, hook, &method, class, ctx, env);
        if hook.is_class_level() {
            for (span, property) in static_writes(m, env.src) {
                env.diags.report(
                    Diagnostic::error(
                        code::E_CLASS_HOOK_WRITES_STATIC,
                        format!(
                            "the {} method `{method}` writes the static property `{property}`",
                            hook.written()
                        ),
                    )
                    .with_primary(
                        span,
                        "each test reads the value the property is declared with",
                    )
                    .with_help(
                        "every test runs on its own and reads the declared value, so return the \
                         value from a `#[Fixture]` method instead",
                    ),
                );
            }
        }
        if let Some((_, prior, _)) = seen.iter().find(|(kind, _, _)| *kind == hook) {
            let prior = prior.clone();
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!(
                        "`{class}` has two {} methods, `{prior}` and `{method}`",
                        hook.written()
                    ),
                )
                .with_primary(m.name, format!("`{prior}` is already this class's hook"))
                .with_help(
                    "a class has one hook of each kind, so put every step in that one method, in \
                     the order the steps run",
                ),
            );
            continue;
        }
        let takes_value = !m.params.is_empty();
        seen.push((hook, method, takes_value));
    }
    if seen.is_empty() {
        return;
    }
    let mut hooks = TestHooks::default();
    for (hook, method, takes_value) in seen {
        let row = Some(HookMethod {
            method,
            declared_in: class.to_string(),
            takes_value,
        });
        match hook {
            Hook::BeforeClass => hooks.before_class = row,
            Hook::AfterClass => hooks.after_class = row,
            Hook::AfterEachTest => hooks.after_each_test = row,
        }
    }
    env.exprs.record_test_hooks(class.to_string(), hooks);
}

/// A hook attribute carries no payload, so every field written on one is
/// [`code::E_UNKNOWN_OPTION`].
fn check_payload(hook: Hook, attr: &Attribute, env: &mut Env<'_>) {
    for field in &attr.fields {
        let name = span_text(env.src, field.name).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_UNKNOWN_OPTION,
                format!("`{name}` is not an option of {}", hook.written()),
            )
            .with_primary(field.span, "no such option")
            .with_help(format!("{} takes no options", hook.written())),
        );
    }
}

/// The declaration-shape refusals for one hook method. A method the signature
/// table has no row for is left alone, for the reason
/// [`crate::testing`]'s `check_method_shape` gives.
fn check_shape(
    m: &MethodMember,
    hook: Hook,
    method: &str,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let signatures = env.signatures;
    let Some(sig) = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(method))
    else {
        return;
    };
    if hook.is_class_level() && !sig.is_static {
        report_shape(
            m.name,
            hook,
            method,
            "is not `static`",
            "a class hook runs once for the whole class, before any test object exists, so \
             declare it `static`",
            env,
        );
    }
    if !hook.is_class_level() && sig.is_static {
        report_shape(
            m.name,
            hook,
            method,
            "is `static`",
            "this hook runs on the object the test ran on, so remove the `static`",
            env,
        );
    }
    if sig.visibility != Visibility::Public {
        report_shape(
            m.name,
            hook,
            method,
            "is not `public`",
            "the runner calls a hook from outside its class, so declare it `public`",
            env,
        );
    }
    // A return type that did not resolve has reported already
    // (`rule:types/one-mistake-reports-once`).
    if !matches!(env.interner.get(sig.return_ty), Ty::Void | Ty::Error) {
        let returned = env.interner.describe(sig.return_ty);
        report_shape(
            m.name,
            hook,
            method,
            &format!("returns `{returned}`"),
            "the runner does not read what a hook returns, so declare `: void`",
            env,
        );
    }
    if !sig.has_body {
        report_shape(
            m.name,
            hook,
            method,
            "has no body",
            "the runner calls a hook, so remove the `abstract` and write its body",
            env,
        );
    }
    let first_ty = sig.params.first().copied();
    check_parameters(m, hook, method, first_ty, ctx, env);
}

/// The one parameter a hook may declare: none for `#[BeforeClass]`, and one of
/// [`Hook::parameter`]'s type for the other two.
///
/// `first_ty` is the first parameter's lowered type. When it is the error type,
/// its name did not resolve and has reported already, so its type is not
/// reported again (`rule:types/one-mistake-reports-once`).
fn check_parameters(
    m: &MethodMember,
    hook: Hook,
    method: &str,
    first_ty: Option<TypeId>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Some(first) = m.params.first() else {
        return;
    };
    let Some(allowed) = hook.parameter() else {
        report_shape(
            first.span,
            hook,
            method,
            "declares a parameter",
            "this hook runs before anything exists to pass it, so remove the parameter",
            env,
        );
        return;
    };
    let unresolved = first_ty.is_some_and(|ty| env.interner.is_error(ty));
    let typed_as_allowed = unresolved
        || first.ty.as_ref().is_some_and(|ty| {
            crate::derive::resolves_to(span_text(env.src, ty.span), allowed, ctx)
        });
    if !typed_as_allowed || first.variadic || first.inout {
        report_shape(
            first.span,
            hook,
            method,
            &format!("declares a parameter that is not a `{allowed}`"),
            &format!(
                "the one parameter this hook may declare is a `{allowed}`, which the runner fills"
            ),
            env,
        );
    }
    if let Some(second) = m.params.get(1) {
        report_shape(
            second.span,
            hook,
            method,
            "declares more than one parameter",
            &format!("this hook may declare one `{allowed}` parameter, and nothing else"),
            env,
        );
    }
}

/// One [`code::E_TEST_HOOK_SHAPE`], worded from what the declaration did.
fn report_shape(at: Span, hook: Hook, method: &str, did: &str, help: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_TEST_HOOK_SHAPE,
            format!("the {} method `{method}` {did}", hook.written()),
        )
        .with_primary(at, did)
        .with_help(help.to_owned()),
    );
}

/// Every write in `m`'s body whose target is a static property, as the span of
/// that property and its text.
fn static_writes(m: &MethodMember, src: &SourceFile) -> Vec<(Span, String)> {
    let Some(body) = &m.body else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for node in nvs_syntax::walk::of_stmts(&body.stmts) {
        collect_static_writes(&node, &mut found);
    }
    found
        .into_iter()
        .map(|span| (span, span_text(src, span).to_owned()))
        .collect()
}

fn collect_static_writes(node: &Node, found: &mut Vec<Span>) {
    let targets: &[Node] = match node.kind {
        "Assign" | "PreIncDec" | "PostIncDec" => node.children.get(..1).unwrap_or(&[]),
        "Unset" => &node.children,
        _ => &[],
    };
    found.extend(targets.iter().filter_map(static_root));
    for child in &node.children {
        collect_static_writes(child, found);
    }
}

/// The static property a write target writes, through any number of
/// subscripts and parentheses.
fn static_root(target: &Node) -> Option<Span> {
    match target.kind {
        "StaticPropertyAccess" => Some(target.span),
        "Index" | "Paren" => static_root(target.children.first()?),
        _ => None,
    }
}
