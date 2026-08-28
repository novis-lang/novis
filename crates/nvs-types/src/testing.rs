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
//! The one rule that roster *cannot* state is a dependency between two of its
//! fields, every field being admitted on its own and all of them optional:
//! § 20's `retries:` charges a written `because:`, and
//! [`check_retries_state_a_reason`] is where that is refused
//! ([`code::E_TEST_RETRIES_WITHOUT_REASON`]), once the payload has been
//! walked rather than at either field.
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
//! # § 8's `#[Fixture]` roster is the second table, keyed by type
//!
//! A fixture is built **once, in the parent isolate**, and injected into each
//! test that declares a parameter of its type, so what a roster of them has to
//! record is the *type* each one supplies — [`Fixture`] is one row, and
//! [`check_fixture_shape`] is where the declaration is held to being able to
//! supply one at all. Its three refusals are [`code::E_FIXTURE_METHOD_SHAPE`]
//! and are the inverse of [`check_method_shape`]'s: a fixture that is not
//! `static` has no instance to be built against, since § 20 gives each test
//! its own and that is the opposite of once; a non-`public` one cannot be
//! called from outside its class; and one returning `void` supplies nothing.
//! The fourth is a member carrying **both** markers, which is one method
//! claiming to be two things whose shapes contradict.
//!
//! `#[Fixture]` is matched nominally exactly as `#[Test]` is
//! ([`crate::derive::FIXTURE`]), and carries no payload at all: what it
//! supplies is its return type, so a field written on it is
//! [`code::E_UNKNOWN_OPTION`] rather than an option that quietly does
//! nothing.
//!
//! Two fixtures of one class returning **one** type is the roster's own
//! refusal rather than a shape one — § 8 resolves by type, so it is one
//! declaration made twice — and it is decidable here, with no parameter
//! anywhere in it. What is *not* decidable here is § 1's remaining bullet, a
//! `#[Test]` parameter no fixture supplies: that is a question about this
//! roster asked from a method's parameter list, and it wants § 9's data rows
//! beside it before it can be answered in one place.
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

/// One `#[Fixture]` method of one class — ADR 0079 § 8's roster, one row at a
/// time.
#[derive(Clone, Debug)]
pub struct Fixture {
    /// The method's own name, exactly as declared. Nothing about a fixture is
    /// inferred from this spelling: § 8 resolves a request for one by
    /// **type**, and the name is what a diagnostic points at.
    pub method: String,
    /// What the method returns, which is what it supplies. This is the key of
    /// the whole roster, so it is the declared return type interned — a
    /// parameter asking for a fixture is matched against it with the same
    /// [`TypeId`] equality every other type question in this crate uses.
    pub ty: TypeId,
}

/// Every `#[Test]` and `#[Fixture]` method `decl` declares, recorded into
/// [`crate::ExprTypeTable`] under `class`'s label.
///
/// A no-op — not even a row — for a class carrying neither, which is what
/// makes a program that declares no tests pay nothing for this pass beyond
/// the walk it already makes over the members.
///
/// The two rosters are collected in **one** walk rather than in a pass each,
/// because they are two questions about one member list: a method carrying
/// both markers is refused here at all, and that refusal is not one either
/// pass could make alone.
pub(crate) fn check_class_tests(decl: &ClassDecl, class: &QName, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut cases: Vec<TestCase> = Vec::new();
    let mut fixtures: Vec<Fixture> = Vec::new();
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let test = attribute_named(&m.attributes, crate::derive::TEST, ctx, env);
        let fixture = attribute_named(&m.attributes, crate::derive::FIXTURE, ctx, env);
        if test.is_none() && fixture.is_none() {
            continue;
        }
        let method = span_text(env.src, m.name).to_owned();
        if test.is_some() && fixture.is_some() {
            // The two shapes are inverted — § 1's test is an instance method
            // returning nothing, § 8's fixture a `static` one returning
            // something — so a member carrying both is not two facts about it
            // but one contradiction, and neither roster gets a row rather
            // than one roster getting a row the other has just refused.
            report_fixture_shape(
                m,
                &method,
                "is also a `#[Test]`",
                "a fixture builds the value a test takes, so a method is one or the other",
                env,
            );
            continue;
        }
        if let Some(attr) = fixture {
            check_fixture_payload(attr, env);
            let Some(ty) = check_fixture_shape(m, &method, class, env) else {
                continue;
            };
            if let Some(prior) = fixtures.iter().find(|already| already.ty == ty) {
                let described = env.interner.describe(ty);
                let prior = prior.method.clone();
                env.diags.report(
                    Diagnostic::error(
                        code::E_DUPLICATE_DECLARATION,
                        format!(
                            "`{class}` declares two `#[Fixture]` methods returning `{described}`"
                        ),
                    )
                    .with_primary(m.name, format!("`{prior}` already supplies `{described}`"))
                    .with_help(
                        "ADR 0079 § 8 resolves a fixture by its type, so two of one type leave a \
                         parameter asking for it with no answer",
                    ),
                );
                continue;
            }
            fixtures.push(Fixture { method, ty });
            continue;
        }
        let Some(attr) = test else { continue };
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
    if !fixtures.is_empty() {
        env.exprs.record_fixtures(class.to_string(), fixtures);
    }
}

/// The recognized attribute `want` attached to one member, or `None`. The
/// nominal match is [`crate::derive::attribute_is`]', so one roster answers
/// "is this that attribute" for every recognized name.
fn attribute_named<'a>(
    groups: &'a [AttributeGroup],
    want: &str,
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> Option<&'a Attribute> {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attr| crate::derive::attribute_is(attr, want, ctx, env))
}

/// ADR 0079 § 8's declaration-shape refusals, for one `#[Fixture]` method,
/// and the type it supplies when there is one.
///
/// Read off the resolved signature for [`check_method_shape`]'s reason
/// exactly. `None` — no row at all — for a fixture with nothing to supply: a
/// method the signature table has no row for, which `nvs_syntax` is already
/// refusing, and one returning `void`, which has no type for § 8's by-type
/// resolution to key on. The other two refusals still record their row, so
/// that a test asking for the type is answered by the fixture the author
/// plainly wrote rather than by a second diagnostic saying nothing supplies
/// it.
fn check_fixture_shape(
    m: &MethodMember,
    method: &str,
    class: &QName,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let signatures = env.signatures;
    let sig = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(method))?;
    let return_ty = sig.return_ty;
    let is_static = sig.is_static;
    let visibility = sig.visibility;
    if !is_static {
        report_fixture_shape(
            m,
            method,
            "is not `static`",
            "a fixture is built once for the whole class, so declare it `static`",
            env,
        );
    }
    if visibility != Visibility::Public {
        report_fixture_shape(
            m,
            method,
            "is not `public`",
            "the runner builds a fixture from outside its class, so declare it `public`",
            env,
        );
    }
    if matches!(env.interner.get(return_ty), Ty::Void) {
        report_fixture_shape(
            m,
            method,
            "returns `void`",
            "a fixture is injected by its type, so declare the type it builds",
            env,
        );
        return None;
    }
    Some(return_ty)
}

/// One [`code::E_FIXTURE_METHOD_SHAPE`], worded from what the declaration did.
fn report_fixture_shape(m: &MethodMember, method: &str, did: &str, help: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_FIXTURE_METHOD_SHAPE,
            format!("the `#[Fixture]` method `{method}` {did}"),
        )
        .with_primary(m.name, did)
        .with_help(help.to_owned()),
    );
}

/// `#[Fixture]` carries no payload at all — § 8 writes it bare, and what it
/// supplies is its return type rather than anything written on the marker.
///
/// So the roster [`OPTIONS`] is for `#[Test]` has no counterpart here, and a
/// field is refused by the same [`code::E_UNKNOWN_OPTION`] a misspelt
/// `#[Test]` option draws: it is the same mistake, an option that does
/// nothing, and silence is the one answer that would let it look as though it
/// had.
fn check_fixture_payload(attr: &Attribute, env: &mut Env<'_>) {
    for field in &attr.fields {
        let name = span_text(env.src, field.name).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_UNKNOWN_OPTION,
                format!("`{name}` is not an option of `#[Fixture]`"),
            )
            .with_primary(field.span, "no such option")
            .with_help("`#[Fixture]` takes none: what it supplies is its declared return type"),
        );
    }
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
    check_retries_state_a_reason(fields, env);
}

/// § 20's other half of the retry bullet: `retries:` requires `because:`.
///
/// The roster above cannot say this. An options bag admits each field on its
/// own and every one of them is optional, which is exactly what lets the bare
/// `#[Test]` parse — so a *dependency* between two of them is a rule about the
/// payload as a whole and is checked once it has been walked. It is the one
/// § 20 bullet that is worth a diagnostic rather than a convention: a retry is
/// sometimes the right engineering call and always a claim about the world,
/// and the reason is what a reader of the attribute has in place of the run
/// that produced it.
fn check_retries_state_a_reason(fields: &[ObjectLiteralField], env: &mut Env<'_>) {
    let named = |option: &str| {
        fields
            .iter()
            .find(|field| span_text(env.src, field.name) == option)
    };
    let (Some(retries), None) = (named("retries"), named("because")) else {
        return;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_TEST_RETRIES_WITHOUT_REASON,
            "`retries` is given with no `because`",
        )
        .with_primary(retries.span, "retried for no stated reason")
        .with_help(
            "ADR 0079 § 20 reports a retried test as flaky rather than green, and charges a \
             written reason for it: `#[Test(retries: 2, because: \"real DNS\")]`",
        ),
    );
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
