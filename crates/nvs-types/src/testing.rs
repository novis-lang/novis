//! `rule:testing/test-attribute`'s `#[Test]` attribute: what its payload may hold.
//!
//! `#[Test]` is one of [`crate::derive::ATTRIBUTES`]' compiler-recognized
//! names, which is what makes it matched **nominally** rather than by the
//! shape it satisfies — so it names no `type` alias for
//! [`crate::attributes`]'s § 1 rule to check the literal against, and the
//! payload is checked here instead. A test is marked by an attribute the
//! compiler acts on, not by a spelling convention, which is the position
//! `rule:core-api/identifier-casing`
//! takes everywhere else.
//!
//! # The option roster is the shape
//!
//! § 1 writes the payload as `{skip?: string, at?: string, seed?: int,
//! db?: string, server?: bool, retries?: int, because?: string}` — **every**
//! field optional. That cannot be a [`crate::ty::Ty::Shape`] target: `rule:types/shape-type`'s width subtyping requires every field the target names to be present
//! in the literal, so a shape of seven fields would refuse the bare `#[Test]`
//! the ADR's own example writes. It is `rule:core-api/shape-rules` R2's options-bag rule
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
//! parameter no `#[Fixture]` supplies and no data row fills — is
//! [`resolve_injections`] below, which is the one pass holding both rosters.
//! Two parameter *shapes* are refused by these codes rather than by that one,
//! because no roster would make either injectable:
//! [`reject_uninjectable_parameters`] owns which and why.
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
//! anywhere in it.
//!
//! # The injection is resolved once the whole class is collected
//!
//! [`resolve_injections`] is § 8's by-type resolution and runs *after* the
//! walk above, because a test may be written above the fixture that supplies
//! it. What each parameter resolves to is recorded as an order on the row
//! ([`TestCase::params`], [`Fixture::fixtures`]) so the runner reads one
//! rather than re-deriving it below a crate that holds no types; a parameter
//! nothing supplies is [`code::E_FIXTURE_PARAMETER_UNSUPPLIED`] and a fixture
//! that requires itself is [`code::E_FIXTURE_CYCLE`].
//!
//! # § 9's data rows are the second answer, and they resolve by name
//!
//! `#[TestWith(...)]` ([`crate::derive::TEST_WITH`]) is the one recognized
//! attribute that may repeat on a declaration, each occurrence being a row and
//! each row its own reported case. Its payload is checked against no shape and
//! no roster of option names: what it is matched against is the *parameter
//! list of the method it is attached to*, by name and by type, which is why it
//! is checked here rather than in [`crate::attributes`] — that pass holds one
//! attribute and this walk holds the member.
//!
//! So [`resolve_parameters`] answers both questions in one place, and asks the
//! row one first: a row names a parameter of *this* method while a fixture
//! answers every method of the class at once, so the more specific of the two
//! wins a parameter both could fill. § 9's "each parameter's source is
//! unambiguous" is that precedence, and [`Injection`] is what it is recorded
//! as. One rule § 9 does not write out falls out of each row being a case:
//! every row of one method fills the same parameters, because the parameters a
//! method declares do not vary row by row — a row omitting a field its
//! siblings supply is [`code::E_TEST_ROW_FIELD`] along with every other way a
//! row can fail to describe its method.
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

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassDecl, ClassMemberKind, MethodMember, ObjectLiteralField,
    Visibility,
};

use crate::defaults::ConstArg;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// What one roster entry declares — [`OPTIONS`] here, and
/// [`crate::commands`]' and [`crate::routes`]' rosters through the same type,
/// because a second copy of three variants is how two attributes come to
/// disagree about what `int` means.
///
/// Three scalar rows, because § 1's shape names three types, and two that are
/// not scalars at all: `rule:attributes/payload-is-a-compile-time-constant` admits an enum case in a payload and
/// `rule:routing/route-attribute`'s `method` is one, and `rule:attributes/access-payload` declares a field whose
/// type is `mixed` on purpose. A sixth row is a decision about what some
/// attribute may carry and belongs in the section that decides it before it
/// belongs here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OptionTy {
    Str,
    Int,
    Bool,
    /// One enum, by fully-qualified name — the type of an option whose value
    /// is a case of it.
    Enum(&'static str),
    /// No declared type at all — `rule:types/declaration`'s
    /// one unchecked position, and `rule:attributes/access-payload`'s `allow`.
    ///
    /// The value is still `rule:attributes/payload-is-a-compile-time-constant`'s compile-time constant, which
    /// [`crate::attributes::check_attribute`] has asked before any roster is
    /// read. The narrower rule that it *names* something belongs to the module
    /// declaring the roster, because it is a rule about one option rather than
    /// about a type — [`crate::routes::check_access`] is where it lives.
    Mixed,
}

impl OptionTy {
    /// The type a value at this option is placed at, or `None` where there is
    /// no type to place it at: [`Self::Mixed`], which is that by declaration,
    /// and a row naming an enum this program declares nowhere.
    ///
    /// The second is `None` rather than a diagnostic, because a roster is a
    /// *static* table
    /// and the enum it names may be one the tree does not have yet. **No row
    /// is in that state today** — `Core\Http\Method` was, and landed as
    /// `nvs_stdlib::router::METHOD` — so this arm is what a row added ahead of
    /// its enum gets rather than something a program can reach: the value is
    /// checked as the compile-time constant `rule:attributes/payload-is-a-compile-time-constant` already requires, and
    /// placed at nothing. A diagnostic here would report a gap in *this* crate
    /// against the source that tripped over it.
    pub(crate) fn intern(self, env: &mut Env<'_>) -> Option<TypeId> {
        let scalar = match self {
            Self::Str => Ty::String,
            Self::Int => Ty::Int,
            Self::Bool => Ty::Bool,
            // `rule:attributes/access-payload`'s `mixed`: the value is checked as a constant and
            // then placed at nothing, which is the whole of what `mixed` asks.
            Self::Mixed => return None,
            Self::Enum(name) => {
                let qname = QName::parse(name);
                let backing = env.enums.get(&qname)?.backing;
                return Some(env.interner.enum_(qname, backing));
            }
        };
        Some(env.interner.intern(scalar))
    }

    /// The spelling a help text names this option's type by — the enum's own
    /// qualified name for [`Self::Enum`], which is what an author has to
    /// write a case of.
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::Str => "string",
            Self::Int => "int",
            Self::Bool => "bool",
            Self::Mixed => "mixed",
            Self::Enum(name) => name,
        }
    }
}

/// `rule:testing/test-attribute`'s option shape, in the order that section writes it. Every
/// one is optional, so this roster says which names are admitted and at what
/// type — never which are required.
pub(crate) const OPTIONS: &[(&str, OptionTy)] = &[
    ("skip", OptionTy::Str),
    ("at", OptionTy::Str),
    ("seed", OptionTy::Int),
    ("db", OptionTy::Str),
    ("server", OptionTy::Bool),
    ("retries", OptionTy::Int),
    ("because", OptionTy::Str),
];

/// One `#[Test]` method of one class — `rule:testing/test-attribute`'s table, one row at a
/// time.
#[derive(Clone, Debug)]
pub struct TestCase {
    /// The method's own name, exactly as declared. `rule:testing/test-attribute`: nothing
    /// about a test is inferred from this spelling.
    pub method: String,
    /// Where the method's name is written — the span every refusal about this
    /// row already points at, kept on the row rather than in a side table.
    ///
    /// The checker holds it anyway, and a consumer below this crate cannot
    /// recover it: a class label and a method name say nothing about which file
    /// the two were declared in. A report that locates a test, and an editor
    /// that opens one, both resolve it through the `SourceMap` their caller
    /// holds, which is the only thing that turns a byte offset into a line.
    pub span: Span,
    /// The options written on the attribute, folded, in source order. An
    /// option the author left out is absent rather than defaulted — what a
    /// missing `retries` means is the runner's question and not this table's.
    pub options: Vec<(String, ConstArg)>,
    /// `rule:testing/fixtures` and `rule:testing/data-rows`'s injection, resolved: where each declared parameter's
    /// value comes from, in **parameter order**.
    ///
    /// An *order* rather than a set, because the runner passes values
    /// positionally and re-deriving which source answers which parameter
    /// would mean re-doing the by-type and by-name resolution below a crate
    /// that holds no types. Empty for the parameterless test § 1's own example
    /// writes, which is what makes a program that declares no fixture and no
    /// row pay nothing for this.
    pub params: Vec<Injection>,
    /// § 9's data rows, one per `#[TestWith(...)]` attached to the method, in
    /// source order — each of them **parameter-length**, holding a value at
    /// exactly the positions [`Self::params`] marks [`Injection::Row`].
    ///
    /// Parameter-length rather than only as long as the fields written, so the
    /// runner walks one list per call rather than joining two: position `i` is
    /// answered by `params[i]`, and a `Row` there reads this row's own `i`.
    /// Empty for a test with no rows at all, which is the one call § 1
    /// describes.
    pub rows: Vec<Vec<Option<ConstArg>>>,
}

/// Where one declared parameter's value comes from — `rule:testing/data-rows`'s "each
/// parameter's source is unambiguous because fixtures resolve by type and
/// rows by name".
///
/// A name is the more specific of the two, so a parameter some row names is a
/// [`Self::Row`] even where a `#[Fixture]` supplies its type as well: the row
/// was written against *this* method's parameter list, while a fixture answers
/// every method of the class at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Injection {
    /// The named `#[Fixture]` method of the same class, built once (§ 8).
    Fixture(String),
    /// This case's own data row (§ 9), read at the same position.
    Row,
}

/// One `#[Fixture]` method of one class — `rule:testing/fixtures`'s roster, one row at a
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
    /// What *this* fixture's own parameters resolve to, in parameter order —
    /// § 8's "a fixture may itself declare fixture parameters", read by the
    /// runner as a build order. A cycle among these is
    /// [`code::E_FIXTURE_CYCLE`], and the row a cycle was reported for carries
    /// an empty list, so what rides across is always a graph that can be
    /// built.
    pub fixtures: Vec<String>,
}

/// One `#[TestWith(...)]` as written, before it has been matched against
/// anything: the payload's own span, which is what a refusal about the *row*
/// points at, and its fields, which is what a refusal about one value points
/// into.
struct RawRow {
    span: Span,
    fields: Vec<ObjectLiteralField>,
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
    let mut case_rows: Vec<Vec<RawRow>> = Vec::new();
    let mut fixtures: Vec<Fixture> = Vec::new();
    let mut fixture_spans: Vec<Span> = Vec::new();
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let test = attribute_named(&m.attributes, crate::derive::TEST, ctx, env);
        let fixture = attribute_named(&m.attributes, crate::derive::FIXTURE, ctx, env);
        let rows = rows_attached(&m.attributes, ctx, env);
        if test.is_none() && fixture.is_none() {
            // § 9 attaches a row to a `#[Test]`, and to nothing else: what a
            // row is matched against is that method's parameter list, and a
            // method the runner never calls has no call for one to fill. A
            // marker that silently did nothing is the mistake the whole
            // recognized roster exists to prevent.
            report_stray_rows(&rows, "is not a `#[Test]`", env);
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
            // A fixture is built **once** for the whole class, so a row — one
            // case per row — has nothing here to vary.
            report_stray_rows(&rows, "is a `#[Fixture]` rather than a `#[Test]`", env);
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
                        "`rule:testing/fixtures` resolves a fixture by its type, so two of one type leave a \
                         parameter asking for it with no answer",
                    ),
                );
                continue;
            }
            fixtures.push(Fixture {
                method,
                ty,
                fixtures: Vec::new(),
            });
            fixture_spans.push(m.name);
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
        cases.push(TestCase {
            method,
            span: m.name,
            options,
            params: Vec::new(),
            rows: Vec::new(),
        });
        case_rows.push(rows);
    }
    resolve_injections(
        class,
        &mut cases,
        &case_rows,
        &mut fixtures,
        &fixture_spans,
        env,
    );
    if !cases.is_empty() {
        env.exprs.record_tests(class.to_string(), cases);
    }
    if !fixtures.is_empty() {
        env.exprs.record_fixtures(class.to_string(), fixtures);
    }
}

/// `rule:testing/fixtures`'s resolution, for one class: every `#[Test]` and `#[Fixture]`
/// parameter matched **by type** against the roster the walk above collected.
///
/// It is a second pass over the two rosters rather than a step inside that
/// walk, and for one reason: a test may be declared **above** the fixture that
/// supplies it, which is how § 8's own worked example is not written but is
/// the first thing a reader will try. Resolving as the walk descends would
/// answer that one "nothing supplies `Schema`" and the same file with the
/// members swapped "here it is", which is a rule about source order and not
/// about types.
///
/// What each parameter resolves to is recorded as an **order** on the row
/// ([`TestCase::params`]), because the runner passes values positionally and
/// nothing below this crate holds a [`TypeId`] to redo the match against.
///
/// § 9's data rows fill a parameter by *name* and are the second answer this
/// refusal consults, so a parameter **neither** roster reaches is refused
/// outright rather than left to a runner that would call the method with a
/// hole in its argument list. A name is the more specific of the two, so a
/// row wins a parameter a fixture's type would also have answered.
fn resolve_injections(
    class: &QName,
    cases: &mut [TestCase],
    case_rows: &[Vec<RawRow>],
    fixtures: &mut [Fixture],
    fixture_spans: &[Span],
    env: &mut Env<'_>,
) {
    if cases.is_empty() && fixtures.is_empty() {
        return;
    }
    let roster: Vec<(String, TypeId)> = fixtures
        .iter()
        .map(|fixture| (fixture.method.clone(), fixture.ty))
        .collect();
    for (index, span) in fixture_spans.iter().enumerate() {
        let method = fixtures[index].method.clone();
        let (params, _) =
            resolve_parameters(class, &method, "#[Fixture]", *span, &roster, &[], env);
        // A `#[Fixture]` is resolved against no rows at all — § 9 attaches one
        // to a `#[Test]`, and `check_class_tests` has already refused every
        // other member carrying the marker — so every position here is a
        // fixture's, and the other arm is an internal-consistency check on
        // that call rather than a shape a program can have.
        fixtures[index].fixtures = params
            .into_iter()
            .map(|source| match source {
                Injection::Fixture(name) => name,
                Injection::Row => {
                    unreachable!("a `#[Fixture]` is resolved against no data rows")
                }
            })
            .collect();
    }
    reject_fixture_cycles(class, fixtures, fixture_spans, env);
    for (index, case) in cases.iter_mut().enumerate() {
        let rows = case_rows.get(index).map_or(&[][..], Vec::as_slice);
        let (params, rows) = resolve_parameters(
            class,
            &case.method,
            "#[Test]",
            case.span,
            &roster,
            rows,
            env,
        );
        case.params = params;
        case.rows = rows;
    }
}

/// One marked method's parameters, each resolved to the source that fills it
/// — the `#[Fixture]` method supplying its type (§ 8), or this method's own
/// data rows naming it (§ 9) — plus those rows folded, parameter-length.
///
/// The fixture match is [`TypeId`] equality, which is § 8's "resolution is by
/// type" exactly: an interned type is the same id wherever it is written, so a
/// parameter declaring the fixture's own return type is the one that resolves
/// and a subtype of it deliberately is not — a fixture supplies *a* type, and
/// admitting an assignable one would make two fixtures able to answer one
/// parameter, which is the very ambiguity the duplicate refusal above exists
/// to prevent. The row match is by **name**, and it is asked first: a row is
/// written against this method's own parameter list while a fixture answers
/// every method of the class at once, so the more specific of the two wins.
///
/// Nothing partial rides across. A refusal anywhere — a row field naming no
/// parameter, a value that is not a literal of that parameter's type, a row
/// omitting a field its siblings supply, a parameter neither roster reaches —
/// answers with two empty lists, so what [`crate::ExprTypeTable`] holds is
/// always a call the runner can make.
///
/// A method the signature table has no row for resolves to nothing, for
/// [`check_method_shape`]'s reason. The span a parameter's own refusal points
/// at is the method's name rather than the parameter's: a
/// [`crate::signatures::MethodSig`] records no per-parameter span, and the
/// declaration is what the author has to change either way. A row's own
/// refusals point into the row, which is where they are written.
fn resolve_parameters(
    class: &QName,
    method: &str,
    marker: &str,
    span: Span,
    roster: &[(String, TypeId)],
    rows: &[RawRow],
    env: &mut Env<'_>,
) -> (Vec<Injection>, Vec<Vec<Option<ConstArg>>>) {
    let Some(sig) = env
        .signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(method))
    else {
        return (Vec::new(), Vec::new());
    };
    let params = sig.params.clone();
    let names = sig.param_names.clone();
    let position_of = |want: &str| names.iter().position(|name| name == want);
    let describe = |position: usize| {
        names.get(position).map_or_else(
            || format!("parameter {}", position + 1),
            |name| format!("${name}"),
        )
    };

    // § 9's rows, field by field: each names a parameter, and its value is a
    // literal of that parameter's declared type or the row is not one for this
    // method at all.
    let mut ok = true;
    let mut folded: Vec<Vec<Option<ConstArg>>> = vec![vec![None; params.len()]; rows.len()];
    let mut written: Vec<Vec<bool>> = vec![vec![false; params.len()]; rows.len()];
    for (index, row) in rows.iter().enumerate() {
        for field in &row.fields {
            let name = span_text(env.src, field.name).to_owned();
            let Some(position) = position_of(&name) else {
                report_row_field(
                    field.span,
                    format!("`{method}` declares no parameter `${name}`"),
                    "`rule:testing/data-rows` matches a data row against the method's parameters by name: \
                     write the field the parameter is called, or declare the parameter",
                    env,
                );
                ok = false;
                continue;
            };
            if written[index][position] {
                env.diags.report(
                    Diagnostic::error(
                        code::E_DUPLICATE_DECLARATION,
                        format!("the data row field `{name}` is given twice"),
                    )
                    .with_primary(field.span, "already set above"),
                );
                ok = false;
                continue;
            }
            written[index][position] = true;
            // A value `rule:attributes/payload-is-a-compile-time-constant` has already refused as computed is not then
            // judged against the parameter's type: the author is told about
            // the value they wrote before they are told what it failed to
            // satisfy.
            if !crate::attributes::is_constant(&field.value) {
                ok = false;
                continue;
            }
            let declared = params[position];
            if let Some(value) = crate::defaults::literal_default(&field.value, declared, env) {
                folded[index][position] = Some(value);
            } else {
                let want = env.interner.describe(declared);
                let parameter = describe(position);
                report_row_field(
                    field.span,
                    format!(
                        "the data row field `{name}` is not a `{want}` literal, which is what \
                         `{parameter}` is declared as"
                    ),
                    "`rule:testing/data-rows` matches a data row against the method's parameters by name \
                     **and** by type, so that a row is compiled into the call it will be made \
                     with",
                    env,
                );
                ok = false;
            }
        }
    }

    // A parameter one row names is filled by every row: the parameters a
    // method declares do not vary row by row, so a row leaving one out
    // describes a call that cannot be made.
    let by_row: Vec<bool> = (0..params.len())
        .map(|position| written.iter().any(|row| row[position]))
        .collect();
    for (index, row) in rows.iter().enumerate() {
        for position in 0..params.len() {
            if !by_row[position] || written[index][position] {
                continue;
            }
            let parameter = describe(position);
            report_row_field(
                row.span,
                format!("this data row of `{method}` gives no `{parameter}`, which another does"),
                "`rule:testing/data-rows` reports each row as its own case, so every one of them fills the \
                 same parameters: give this row the field too, or take it off the others and \
                 supply the parameter with a `#[Fixture]`",
                env,
            );
            ok = false;
        }
    }

    // A method whose rows have already been refused is not then told which
    // parameter that left unfilled: a field naming no parameter takes its
    // parameter's answer away with it, so the second diagnostic is the first
    // one seen from the other end. The author fixes the row.
    if !ok {
        return (Vec::new(), Vec::new());
    }

    let mut resolved = Vec::with_capacity(params.len());
    for (position, ty) in params.iter().enumerate() {
        if by_row[position] {
            resolved.push(Injection::Row);
            continue;
        }
        if let Some((supplier, _)) = roster.iter().find(|(_, supplies)| supplies == ty) {
            resolved.push(Injection::Fixture(supplier.clone()));
            continue;
        }
        let described = env.interner.describe(*ty);
        let parameter = describe(position);
        env.diags.report(
            Diagnostic::error(
                code::E_FIXTURE_PARAMETER_UNSUPPLIED,
                format!(
                    "nothing supplies `{parameter}` of the {marker} method `{method}`, which \
                     `{class}` declares as `{described}`"
                ),
            )
            .with_primary(span, format!("`{parameter}` asks for `{described}`"))
            // § 9 attaches a row to a `#[Test]` and to nothing else, so the
            // second answer is offered only where it can be taken: naming it
            // to a `#[Fixture]`'s parameter would be a fix that is itself
            // refused.
            .with_help(if marker == "#[Test]" {
                format!(
                    "`rule:testing/fixtures` resolves a parameter by its type and § 9 by its name: \
                     declare a `public static` `#[Fixture]` on this class returning \
                     `{described}`, or write a `#[TestWith({parameter}: ...)]` row on the \
                     method"
                )
            } else {
                format!(
                    "`rule:testing/fixtures` resolves a parameter by its type: declare a `public static` \
                     `#[Fixture]` on this class returning `{described}`"
                )
            }),
        );
        return (Vec::new(), Vec::new());
    }
    (resolved, folded)
}

/// One [`code::E_TEST_ROW_FIELD`] — § 9's "by name and by type", wherever a
/// row and the method it is attached to fail to line up.
fn report_row_field(at: Span, message: String, help: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_TEST_ROW_FIELD, message)
            .with_primary(at, "this data row does not describe the method it is on")
            .with_help(help.to_owned()),
    );
}

/// Every `#[TestWith(...)]` attached to one member, in source order — § 9's
/// "each row is its own reported case" is what makes this the one recognized
/// name that may repeat on a declaration.
fn rows_attached(groups: &[AttributeGroup], ctx: &Ctx<'_>, env: &Env<'_>) -> Vec<RawRow> {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .filter(|attr| crate::derive::attribute_is(attr, crate::derive::TEST_WITH, ctx, env))
        .map(|attr| RawRow {
            span: attr.payload,
            fields: attr.fields.clone(),
        })
        .collect()
}

/// § 9 attaches a data row to a `#[Test]` method and to nothing else: this is
/// every other member carrying one, refused where the marker is written.
///
/// A marker that quietly did nothing is what `rule:core-classes/derive-attribute`'s closed, recognized
/// roster exists to prevent — an attribute the compiler acts on either acts or
/// says why it cannot.
fn report_stray_rows(rows: &[RawRow], did: &str, env: &mut Env<'_>) {
    for row in rows {
        report_row_field(
            row.span,
            format!("this `#[TestWith]` is on a method that {did}"),
            "`rule:testing/data-rows` reports each row of a `#[Test]` as its own case, so a row on \
             anything else names a call that is never made: mark the method `#[Test]`, or \
             drop the row",
            env,
        );
    }
}

/// § 8's last sentence: a fixture may declare fixture parameters of its own,
/// and a cycle among them is a compile error
/// ([`code::E_FIXTURE_CYCLE`]).
///
/// A cycle is reported once, at the row where the walk closes it and naming
/// the whole chain, and every row on it then has its own dependencies
/// **cleared** — which both stops the same cycle being reported once per
/// member and keeps what rides across in [`crate::ExprTypeTable`] a graph the
/// runner can build in some order. The compile fails on the diagnostic in any
/// case; the clearing is what makes that table's own invariant true rather
/// than true-because-nobody-reads-it.
fn reject_fixture_cycles(
    class: &QName,
    fixtures: &mut [Fixture],
    spans: &[Span],
    env: &mut Env<'_>,
) {
    let index_of = |name: &str| fixtures.iter().position(|row| row.method == name);
    let mut deps: Vec<Vec<usize>> = fixtures
        .iter()
        .map(|row| {
            row.fixtures
                .iter()
                .filter_map(|name| index_of(name))
                .collect()
        })
        .collect();
    while let Some(chain) = first_cycle(&deps) {
        let named: Vec<&str> = chain
            .iter()
            .map(|&node| fixtures[node].method.as_str())
            .collect();
        let closes = chain[0];
        let path = format!("{} -> {}", named.join(" -> "), named[0]);
        let at = spans.get(closes).copied().unwrap_or(spans[0]);
        env.diags.report(
            Diagnostic::error(
                code::E_FIXTURE_CYCLE,
                format!("`{class}`'s `#[Fixture]` methods require each other: {path}"),
            )
            .with_primary(at, "this fixture is needed to build itself")
            .with_help(
                "`rule:testing/fixtures` builds a fixture before the parameter it fills, so a cycle has no \
                 order to be built in: break it by taking the shared value out into a fixture of \
                 its own",
            ),
        );
        for node in chain {
            deps[node].clear();
            fixtures[node].fixtures.clear();
        }
    }
}

/// The first cycle a depth-first walk of `deps` closes, as the chain of nodes
/// on it starting at the one it returns to — or `None` for an acyclic graph.
fn first_cycle(deps: &[Vec<usize>]) -> Option<Vec<usize>> {
    /// Not started, on the current path, finished — the three colours the walk
    /// distinguishes, since only a back edge to a node still *on the path* is
    /// a cycle.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Colour {
        Fresh,
        OnPath,
        Done,
    }

    fn walk(
        node: usize,
        deps: &[Vec<usize>],
        colour: &mut [Colour],
        path: &mut Vec<usize>,
    ) -> Option<Vec<usize>> {
        colour[node] = Colour::OnPath;
        path.push(node);
        for &next in &deps[node] {
            if colour[next] == Colour::OnPath {
                let from = path.iter().position(|&on| on == next).unwrap_or(0);
                return Some(path[from..].to_vec());
            }
            if colour[next] == Colour::Fresh
                && let Some(found) = walk(next, deps, colour, path)
            {
                return Some(found);
            }
        }
        path.pop();
        colour[node] = Colour::Done;
        None
    }

    let mut colour = vec![Colour::Fresh; deps.len()];
    for start in 0..deps.len() {
        if colour[start] != Colour::Fresh {
            continue;
        }
        let mut path = Vec::new();
        if let Some(found) = walk(start, deps, &mut colour, &mut path) {
            return Some(found);
        }
    }
    None
}

/// The recognized attribute `want` attached to one member, or `None`. The
/// nominal match is [`crate::derive::attribute_is`]', so one roster answers
/// "is this that attribute" for every recognized name.
pub(crate) fn attribute_named<'a>(
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

/// `rule:testing/fixtures`'s declaration-shape refusals, for one `#[Fixture]` method,
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
    reject_uninjectable_parameters(m, method, sig, &report_fixture_shape, env);
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

/// `rule:testing/test-attribute`'s three declaration-shape refusals, for one `#[Test]` method.
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
///
/// # Known gaps
///
/// 1. An `abstract` `#[Test]` is accepted. The method has no body, so the
///    runner finds nothing to call and `nvs_runtime`'s dispatch raises its
///    `internal error` fault in place of the test — a program with no unsafe
///    construct in it reaching a branch written as unreachable.
///    `rule:testing/test-attribute` enumerates four declaration-shape
///    refusals and a body-less method is none of them, so adding a fifth is a
///    decision of its own rather than a fix to this function's wording.
///    `tests/hostile/lang/testing/a-test-is-a-method-marked-test/02-a-marked-method-with-no-body.nvs`
///    is the case, marked `known-gap` until the refusal lands.
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
    reject_uninjectable_parameters(m, method, sig, &report_shape, env);
}

/// The two parameter *shapes* §§ 8-9's injection cannot fill, for a `#[Test]`
/// or a `#[Fixture]` alike — reported through whichever of the two shape
/// codes the marker owns, `report` being that choice.
///
/// A variadic tail is packed into an array at the **call site** and an `inout`
/// parameter is written back there, and the site here is a runner supplying
/// one value per parameter from a roster keyed by type: neither the packing
/// nor the write-back has anywhere to happen. It is the same limit
/// [`code::E_DELEGATE_MEMBER_NOT_FORWARDABLE`] already names for `rule:classes/delegation-by-field`'s
/// synthesized forward, arrived at from the other side, and it is a shape
/// refusal rather than a resolution one because no roster would make either
/// spelling injectable.
fn reject_uninjectable_parameters(
    m: &MethodMember,
    method: &str,
    sig: &crate::signatures::MethodSig,
    report: &dyn Fn(&MethodMember, &str, &str, &str, &mut Env<'_>),
    env: &mut Env<'_>,
) {
    if sig.variadic {
        report(
            m,
            method,
            "declares a variadic parameter",
            "a value is injected per declared parameter, so write each one out",
            env,
        );
    }
    if sig.inout.iter().any(|is_inout| *is_inout) {
        report(
            m,
            method,
            "declares an `inout` parameter",
            "an injected value has no caller's storage to be written back to, so drop the \
             `inout`",
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
        let Some(declared) = ty.intern(env) else {
            continue;
        };
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
    crate::attributes::check_roster("Test", OPTIONS, fields, ctx, env);
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
            "`rule:testing/runner-is-strict` reports a retried test as flaky rather than green, and charges a \
             written reason for it: `#[Test(retries: 2, because: \"real DNS\")]`",
        ),
    );
}

/// One written `Core\Test::assertMatchesInline(…, "…")`, as § 14's
/// `nvs test --update` needs it: **where the snapshot literal is**, which is
/// the one fact nothing at run time can supply.
///
/// A helper is called with a value and never with the expression that built
/// it, so `nvs_runtime::SnapshotMismatch` can only say what the snapshot *was*
/// and what it should have been. This row is the other half of that join, and
/// it is collected here rather than searched for in the source afterwards for
/// § 14's own workflow's sake: a snapshot is written empty and filled by the
/// updater, and `""` occurs in every file.
#[derive(Clone, Debug)]
pub struct InlineSnapshot {
    /// The `$expected` literal's own span, delimiters included — exactly the
    /// bytes the updater replaces.
    pub span: Span,
    /// What that literal folds to. The join key, because it is the only part
    /// of the literal a run can see.
    pub expected: String,
    /// The `Class::method` this call is written inside, or `None` where it is
    /// not inside a method at all.
    ///
    /// Stamped by `crate::check::check_method` once the body has been
    /// walked, rather than read here: this is an *expression* walk and the
    /// declaration it is inside is not one of the things it is handed. It
    /// narrows the join to the test that produced the mismatch, which is what
    /// keeps two snapshots that both start out `""` tellable apart.
    pub owner: Option<String>,
}

/// Records § 14's row for a `Core\Test::assertMatchesInline` whose `$expected`
/// is a written literal, and does nothing for every other call.
///
/// The hook [`crate::expr::calls::infer_static_call`] reaches after the target
/// has resolved, beside the other rules that read a `Core` call's own written
/// arguments. A computed expectation records nothing rather than being
/// refused: the member's contract is a `string` and one built at run time is a
/// legal — if pointless — way to reach it, so what it loses is only the
/// ability to be rewritten, which the runner then says out loud.
pub(crate) fn note_inline_snapshot(
    owner: &QName,
    member: &str,
    args: &nvs_syntax::ast::CallArgs,
    env: &mut Env<'_>,
) {
    if member != "assertMatchesInline"
        || !owner.is_core()
        || owner.segments().len() != 2
        || owner.short_name() != "Test"
    {
        return;
    }
    let nvs_syntax::ast::CallArgs::List(list) = args else {
        return;
    };
    // `expected:` written by name fills the same parameter, and is read the
    // same way `crate::capability` reads its one — a spread is not a written
    // argument at all and leaves this call unrecorded.
    let named = list.iter().find(|arg| {
        arg.name
            .is_some_and(|name| span_text(env.src, name) == "expected")
    });
    let Some(arg) = named.or_else(|| {
        list.iter()
            .filter(|arg| !arg.spread && arg.name.is_none())
            .nth(1)
    }) else {
        return;
    };
    let declared = env.interner.intern(Ty::String);
    let Some(ConstArg::Str(expected)) = crate::defaults::literal_default(&arg.value, declared, env)
    else {
        return;
    };
    env.exprs.record_inline_snapshot(InlineSnapshot {
        span: arg.value.span,
        expected,
        owner: None,
    });
}
