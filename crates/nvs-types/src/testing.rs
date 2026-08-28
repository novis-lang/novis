//! [ADR 0079](../../../../docs/adr/0079-testing-is-a-language-feature.md)
//! § 1's `#[Test]` attribute: what its payload may hold.
//!
//! `#[Test]` is one of [`crate::derive::ATTRIBUTES`]' compiler-recognized
//! names, which is what makes it matched **nominally** rather than by the
//! shape it satisfies — so it names no `type` alias for
//! [`crate::attributes`]'s § 1 rule to check the literal against, and the
//! payload is checked here instead. A test is marked by an attribute the
//! compiler acts on, not by a spelling convention, which is the position
//! [ADR 0029](../../../../docs/adr/0029-identifier-casing-is-checked.md)
//! takes everywhere else.
//!
//! # The option roster is the shape
//!
//! § 1 writes the payload as `{skip?: string, at?: string, seed?: int,
//! db?: string, server?: bool, retries?: int, because?: string}` — **every**
//! field optional. That cannot be a [`crate::ty::Ty::Shape`] target: ADR 0036
//! § 3's width subtyping requires every field the target names to be present
//! in the literal, so a shape of seven fields would refuse the bare `#[Test]`
//! the ADR's own example writes. It is ADR 0063 R2's options-bag rule
//! instead, which is the one this shape actually wants: a field the roster
//! does not name is refused (`E_UNKNOWN_OPTION`), a field it does name is
//! checked against that option's own type, one written twice is refused, and
//! an absent field is simply absent. [`OPTIONS`] is that roster's one home,
//! names and types together, so the help text and the check cannot disagree
//! about what an option is.
//!
//! Two of § 1's compile errors fall out of the types rather than needing a
//! rule of their own: `#[Test(skip: true)]` is a `bool` where `skip` declares
//! a `string`, which is § 20's "a skip states a reason" said by the type, and
//! a misspelled option is the unknown-option refusal rather than an option
//! silently doing nothing.
//!
//! # The shape a test method must have
//!
//! The other three of § 1's five compile errors are about the *declaration*
//! rather than the payload, and all three are decided in [`check_class_tests`]
//! because that walk is already holding each member's [`MethodMember`]: a
//! `#[Test]` that is `static`, that is not `public`, or that returns anything
//! but `void`. [`check_method_shape`] is their one home, and they share one
//! code ([`code::E_TEST_METHOD_SHAPE`]) because they are one question asked
//! once — the runner constructs the class and calls the member with no
//! arguments and no result, so a `static` member has no receiver for §§ 8-9's
//! `#[Fixture]` to be installed on, a non-`public` one cannot be called from
//! outside its class at all, and a returned value has nowhere to go. The
//! fourth, two `#[Test]` methods sharing one name, is `E_DUPLICATE_DECLARATION`
//! — the same code the option written twice already draws, one mistake drawing
//! one code — and the fifth is the roster's own type check above.
//!
//! Each is read off the **resolved** [`crate::signatures::MethodSig`] rather
//! than off the modifier list a second time, so an omitted visibility keyword
//! reads as `public` here exactly as it does everywhere else and is left to
//! the `E_MISSING_VISIBILITY` `nvs_syntax::casing` already reports, rather
//! than being named twice under two codes. § 1's remaining bullet — a
//! parameter no `#[Fixture]` supplies and no data row fills — is not decidable
//! here at all and waits on §§ 8-9, which is what will hold both rosters.
//!
//! # The table is built while checking, and rides in [`crate::ExprTypeTable`]
//!
//! § 1's "the compiler collects every `#[Test]` into a table" is
//! [`check_class_tests`], run from [`crate::check`]'s own walk for
//! [`crate::derive::check_class_derive`]'s reason exactly: which `Name` a
//! `#[Test]` is depends on the namespace and the import set, and that walk is
//! the only pass holding both. Discovery therefore costs nothing at startup —
//! there is no directory scan and no reflection, because the answer was
//! settled when the program was compiled.
//!
//! It rides in [`crate::ExprTypeTable`] beside the derived codecs, keyed by
//! **class label**, for the reason that table's own
//! [`record_codec`](crate::ExprTypeTable::record_codec) records: the fact is
//! about a *declaration* rather than about an AST node, and `nvs-ir` is
//! handed this table and not the signature table. Each option is folded to
//! the same [`ConstArg`] a parameter default is folded to, through the very
//! function that folds one, so a `#[Test]` option is never a second literal
//! grammar.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassDecl, ClassMemberKind, MethodMember, ObjectLiteralField,
    Visibility,
};
use rustc_hash::FxHashSet;

use crate::defaults::ConstArg;
use crate::expr::check_expr;
use crate::locals::LocalScope;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// What one [`OPTIONS`] entry declares. Three rows, because § 1's shape names
/// three types; a fourth is a decision about what a test may be annotated
/// with and belongs in that section before it belongs here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OptionTy {
    Str,
    Int,
    Bool,
}

impl OptionTy {
    fn intern(self, env: &mut Env<'_>) -> TypeId {
        env.interner.intern(match self {
            Self::Str => Ty::String,
            Self::Int => Ty::Int,
            Self::Bool => Ty::Bool,
        })
    }

    fn describe(self) -> &'static str {
        match self {
            Self::Str => "string",
            Self::Int => "int",
            Self::Bool => "bool",
        }
    }
}

/// ADR 0079 § 1's option shape, in the order that section writes it. Every
/// one is optional, so this roster says which names are admitted and at what
/// type — never which are required.
const OPTIONS: &[(&str, OptionTy)] = &[
    ("skip", OptionTy::Str),
    ("at", OptionTy::Str),
    ("seed", OptionTy::Int),
    ("db", OptionTy::Str),
    ("server", OptionTy::Bool),
    ("retries", OptionTy::Int),
    ("because", OptionTy::Str),
];

/// One `#[Test]` method of one class — ADR 0079 § 1's table, one row at a
/// time.
#[derive(Clone, Debug)]
pub struct TestCase {
    /// The method's own name, exactly as declared. ADR 0079 § 1: nothing
    /// about a test is inferred from this spelling.
    pub method: String,
    /// The options written on the attribute, folded, in source order. An
    /// option the author left out is absent rather than defaulted — what a
    /// missing `retries` means is the runner's question and not this table's.
    pub options: Vec<(String, ConstArg)>,
}

/// Every `#[Test]` method `decl` declares, recorded into
/// [`crate::ExprTypeTable`] under `class`'s label.
///
/// A no-op — not even a row — for a class with no `#[Test]` at all, which is
/// what makes a program that declares no tests pay nothing for this pass
/// beyond the walk it already makes over the members.
pub(crate) fn check_class_tests(decl: &ClassDecl, class: &QName, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut cases: Vec<TestCase> = Vec::new();
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let Some(attr) = test_attribute(&m.attributes, ctx, env) else {
            continue;
        };
        let method = span_text(env.src, m.name).to_owned();
        check_method_shape(m, &method, class, env);
        if cases.iter().any(|case| case.method == method) {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("`{class}` declares two `#[Test]` methods named `{method}`"),
                )
                .with_primary(m.name, "already declared above")
                .with_help("a test is reported by its own name, so two cannot share one"),
            );
            continue;
        }
        let options = fold_options(&attr.fields.clone(), env);
        cases.push(TestCase { method, options });
    }
    if !cases.is_empty() {
        env.exprs.record_tests(class.to_string(), cases);
    }
}

/// The `#[Test]` attached to one member, or `None`. The nominal match is
/// [`crate::derive::attribute_is`]', so one roster answers "is this that
/// attribute" for every recognized name.
fn test_attribute<'a>(
    groups: &'a [AttributeGroup],
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> Option<&'a Attribute> {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attr| crate::derive::attribute_is(attr, crate::derive::TEST, ctx, env))
}

/// ADR 0079 § 1's three declaration-shape refusals, for one `#[Test]` method.
///
/// All three are read off the resolved signature rather than off `m`'s own
/// modifier list, for the reason this module's docs give: an omitted
/// visibility keyword is already `E_MISSING_VISIBILITY` and reads as `public`
/// everywhere else, so re-deriving it here would name one mistake twice. `m`
/// is still what carries the span the diagnostic points at — the method's own
/// name, since a `Modifier` records no span of its own and the body is not
/// what any of these is about.
///
/// A method the signature table has no row for is left alone: it is either
/// a name `nvs_syntax` is already refusing or a duplicate of one, and there is
/// nothing here that a second diagnostic about its shape would add.
fn check_method_shape(m: &MethodMember, method: &str, class: &QName, env: &mut Env<'_>) {
    let signatures = env.signatures;
    let Some(sig) = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(method))
    else {
        return;
    };
    if sig.is_static {
        report_shape(
            m,
            method,
            "is `static`",
            "a test is run against a fresh instance, so drop the `static`",
            env,
        );
    }
    if sig.visibility != Visibility::Public {
        report_shape(
            m,
            method,
            "is not `public`",
            "the runner calls a test from outside its class, so declare it `public`",
            env,
        );
    }
    if !matches!(env.interner.get(sig.return_ty), Ty::Void) {
        let returned = env.interner.describe(sig.return_ty);
        report_shape(
            m,
            method,
            &format!("returns `{returned}`"),
            "a test reports by asserting rather than by returning, so declare `: void`",
            env,
        );
    }
}

/// One [`code::E_TEST_METHOD_SHAPE`], worded from what the declaration did.
fn report_shape(m: &MethodMember, method: &str, did: &str, help: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_TEST_METHOD_SHAPE,
            format!("the `#[Test]` method `{method}` {did}"),
        )
        .with_primary(m.name, did)
        .with_help(help.to_owned()),
    );
}

/// One payload's options, folded to constants in source order.
///
/// Silent throughout: [`check_payload`] has already reported every field this
/// cannot fold — an unknown option, a value of the wrong type, a value that
/// is not constant at all — so a second diagnostic here would name one
/// mistake twice. What it cannot fold it simply omits.
fn fold_options(fields: &[ObjectLiteralField], env: &mut Env<'_>) -> Vec<(String, ConstArg)> {
    let mut folded = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        let Some((_, ty)) = OPTIONS.iter().find(|(option, _)| *option == name) else {
            continue;
        };
        let declared = ty.intern(env);
        if let Some(value) = crate::defaults::literal_default(&field.value, declared, env) {
            folded.push((name, value));
        }
    }
    folded
}

/// One `#[Test]` payload, checked against [`OPTIONS`].
///
/// Called only for a payload [`crate::attributes`] has already proved
/// constant, for that module's own reason: the author is told about a value
/// they wrote before they are told what it failed to satisfy.
pub(crate) fn check_payload(fields: &[ObjectLiteralField], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    // The scope is empty and stays empty: § 2 has just proved this payload
    // reads no variable, so there is no binding to mark live and none to
    // capture.
    let mut live = FxHashSet::default();
    let scope = LocalScope::new();
    let mut seen: Vec<String> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        let declared = OPTIONS
            .iter()
            .find(|(option, _)| *option == name)
            .map(|(_, ty)| *ty);
        let expected = declared.map(|ty| ty.intern(env));
        check_expr(&field.value, expected, &mut live, &scope, ctx, env);
        if declared.is_none() {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNKNOWN_OPTION,
                    format!("`{name}` is not an option of `#[Test]`"),
                )
                .with_primary(field.span, "no such option")
                .with_help(format!("the options are: {}", option_names())),
            );
        } else if seen.iter().any(|already| already == &name) {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("the option `{name}` is given twice"),
                )
                .with_primary(field.span, "already set above"),
            );
        }
        seen.push(name);
    }
}

/// The roster rendered for a help text — `skip: string, at: string, …` — so a
/// typo is answered with the options themselves rather than with a type
/// spelling nobody wrote.
fn option_names() -> String {
    OPTIONS
        .iter()
        .map(|(option, ty)| format!("{option}: {}", ty.describe()))
        .collect::<Vec<_>>()
        .join(", ")
}
