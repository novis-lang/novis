//! `rule:routing/route-attribute`'s
//! `#[Route]`: what one route declaration may carry.
//!
//! # Why this is a recognized name rather than a shape alias
//!
//! § 1 writes the attribute as an ordinary
//! `rule:attributes/inert-metadata`
//! `type Core\Route = {path: string, method: Core\Http\Method, name?: string};`
//! and then adds the one thing that makes it not one: the compiler acts on the
//! attribute only when its name **resolves** to `Core\Route`, so a userland
//! `type Route = {…};` is not it however it is spelled and a framework carrying
//! its own `Route`-shaped literal does not contribute a route. That is
//! `rule:core-classes/derive-attribute`'s rule, so the
//! name sits on [`crate::derive::ATTRIBUTES`] and is matched *nominally* after
//! [`nvs_hir::resolve_ref`] — and, being matched nominally, it names no shape,
//! which is why what it may hold is the roster below rather than an alias
//! lookup.
//!
//! # What is checked here, and which pass asks it
//!
//! Two passes, and they are two because they read different things — the same
//! split [`crate::commands`] makes, for the same reason.
//!
//! **One payload at a time**, from [`crate::attributes`]'s per-attribute walk:
//! the field names against [`OPTIONS`], each value's type, and a field given
//! twice — [`crate::attributes::check_roster`], the walk every recognized name
//! with a payload shares. That is one declaration read on its own.
//!
//! **The whole program's routes**, as [`RouteTable`]: [`check_class_routes`]
//! adds one row per `#[Route]` — every one the method carries, so `rule:attributes/repeatable`'s repetition is one method serving two verbs — as [`crate::check`]'s
//! per-class walk reaches it,
//! and [`check_table`] then holds the collected rows to the two of § 1-§ 3's
//! four compile errors that are questions about the *enumeration* — a duplicate
//! route and a duplicate `name`. A row needs a `path` and a `method` to exist
//! at all, so this is also where an attribute that named neither is refused.
//!
//! Between the two, and inside [`collect_route`], the path is read twice: once
//! on its own against § 2's grammar ([`parse_path`], which splits it into
//! [`Capture`]s and refuses the four ways it is not a path), and once against
//! the method the attribute is attached to ([`check_captures`], which asks § 3
//! whether each capture names a parameter and whether that parameter's declared
//! type is one a segment converts to). Both live here rather than in the
//! per-attribute walk for [`check_class_routes`]' own reason: a payload with no
//! declaration around it can see neither the parameter list nor the class.
//!
//! The same walk reads
//! `rule:routing/a-query-parameter-is-declared-like-a-capture`
//! 's `#[Query]` ([`query_params`]): a parameter the *declaration* binds from
//! the query string rather than one the path names, so it is a second reading of
//! the same parameter list and not a third reading of the path. The keys land on
//! the row, because the only question asked about them is § 6's and it is asked
//! from [`crate::links`] once the whole table exists.
//!
//! And it reads
//! `rule:attributes/access-is-a-required-sibling`'s `#[Access]`, which is here for `#[Query]`'s reason — it is a
//! `#[Route]`'s sibling and means nothing away from one. Its *payload* is
//! [`ACCESS_OPTIONS`] and [`check_access`], from the per-attribute walk; § 1's
//! presence rule is a question about the method's attribute list and so is
//! asked by the same walk that finds the `#[Route]` ([`check_access_declared`]);
//! § 1a's one-per-method rule is a question about that list as a whole and so
//! is [`check_one_access`], from the walk that visits every method's list. The
//! decision itself rides on the row as [`Route::access`], because `rule:security/access-is-checked-for-presence-not-meaning`
//! leaves enforcement to whoever dispatches. § 4's `csrf` opt-out is a question
//! about a *method's* verbs rather than a row's — one `#[Access]` covers every
//! `#[Route]` the method carries — and so is [`check_csrf_opt_out`], from the
//! same walk.
//!
//! The conversion roster is [`crate::commands::converts_from_string`], read and
//! never copied — `rule:tooling/commands-are-compiled` takes § 3's list unchanged and `rule:routing/a-query-parameter-is-declared-like-a-capture` takes
//! it unchanged again for a query parameter, so all three passes ask one
//! question. The single thing this pass adds to it is that a `{name...}` arrives
//! as the one `tainted string` § 3 says it does, so it binds a `string` and
//! nothing else.
//!
//! And it reads
//! `rule:attributes/api-adds-and-cannot-contradict`'s `#[Api]`, which is here for the same reason again and is the one
//! attribute on this list that changes nothing a program does: it supplies
//! what the route table and the signature cannot say, and § 2's whole rule is
//! that it **may add and may not contradict**. Its payload's two roster
//! questions are [`API_OPTIONS`], and its four contradictions are
//! [`check_api`] — three of them, each a comparison against the declaration —
//! plus [`check_stray_api`] for the fourth, which is [`check_stray_query`]'s
//! question about a marker away from the thing that reads it. One `#[Api]`
//! describes the operation however many `#[Route]`s the method carries, so it
//! is asked once per method beside `#[Access]` rather than once per row.
//!
//! The same walk reads one thing that is not an attribute at all: § 1's last
//! row gives the summary and description to **the declaration's own doc
//! comment**, so [`doc_comment`] reads the `///` run the parser attached to the
//! member (`rule:tooling/doc-comment-is-three-slashes`) and [`Route`] carries
//! the two strings it splits into. Here rather than at the emitter because the
//! row is what crosses out of this crate.
//!
//! A `#[Query]` written where no `#[Route]` reads it is [`check_stray_query`],
//! and an `#[Access]` written there is [`check_stray_access`] — `rule:attributes/access-is-a-required-sibling`'s
//! sibling rule asked from the other side. Those are the questions the per-class
//! walk cannot ask: it selects the methods a `#[Route]` marks, so a stray marker
//! is invisible to it by construction and the question belongs to the walk that
//! visits every method.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_hir::QName;
use nvs_syntax::DOC_MARKER;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassDecl, ClassMemberKind, DocComment, ExprKind, MethodMember,
};
use rustc_hash::FxHashMap;

use crate::expr_table::UrlPiece;
use crate::testing::OptionTy;
use crate::{ConstArg, Ctx, Env, span_text};

/// The enum a `method:` value is a case of, named once: [`OPTIONS`] places the
/// written value at it, and [`verb_of`] reads the case back out of the same
/// name.
const METHOD_ENUM: &str = r"Core\Http\Method";

const PATH: &str = "path";
const METHOD: &str = "method";
const NAME: &str = "name";

const ALLOW: &str = "allow";
const CSRF: &str = "csrf";

const TAGS: &str = "tags";
const ERRORS: &str = "errors";
const SECURITY: &str = "security";
const EXAMPLE: &str = "example";

const STATUS: &str = "status";
const TYPE: &str = "type";

/// The four verbs `rule:security/csrf-is-on-by-default` turns CSRF on for, named once: [`CSRF`] is an
/// opt-out from *these*, and [`check_csrf_opt_out`] is the only reader.
///
/// Spelled as [`verb_of`] answers — the enum case's own name — because that is
/// the form the row carries and the form the check compares.
const UNSAFE_VERBS: [&str; 4] = ["Post", "Put", "Patch", "Delete"];

/// `#[Route(path: string, method: Core\Http\Method, name?: string)]` — `rule:routing/route-attribute`
/// 's own spelling, in the order that section writes it.
///
/// `method` is an enum case rather than a string
/// (`rule:core-api/shape-rules` R11), and an
/// enum case is one of the three things a payload may contain (`rule:attributes/payload-is-a-compile-time-constant`);
/// it is the same `Core\Http\Method` `Core\Request::method` answers with, which
/// is why the row names that enum rather than a spelling of its own. There is
/// no `methods:` row: § 1 serves two verbs by repeating the attribute
/// (`rule:attributes/repeatable`), so a union or an array here would be a second way to write
/// what the existing rule already covers.
pub(crate) const OPTIONS: &[(&str, OptionTy)] = &[
    (PATH, OptionTy::Str),
    (METHOD, OptionTy::Enum(METHOD_ENUM)),
    (NAME, OptionTy::Str),
];

/// `#[Access(allow: mixed, csrf?: bool)]` — `rule:attributes/access-payload`'s own spelling, and
/// the one roster here whose required field the roster itself cannot mark.
///
/// `allow` is [`OptionTy::Mixed`] because § 1a declares it `mixed` and says
/// why: § 2 is a promise not to know what the decision means, so placing the
/// value at a type would be a claim this compiler does not make. What stands
/// in place of that type is [`check_access`]. `csrf` is § 4's per-route
/// opt-out and is an ordinary `bool` — the verbs it applies to are the route's
/// own, so there is no field here naming them.
pub(crate) const ACCESS_OPTIONS: &[(&str, OptionTy)] =
    &[(ALLOW, OptionTy::Mixed), (CSRF, OptionTy::Bool)];

/// One `#[Access]` payload, held to the two of `rule:attributes/access-payload`'s rules that are
/// questions about this payload alone.
///
/// **`allow` is required**, which [`crate::attributes::check_roster`] cannot
/// say for [`collect_route`]'s reason: a roster says what a field may hold,
/// and *required* is a fact about the thing being declared. An `#[Access]`
/// carrying none declares nothing, so it is refused wherever it is attached —
/// § 3's omission is the mistake whether or not a row was going to be built
/// out of a sibling `#[Route]`.
///
/// **Its value names something.** § 1a narrows `rule:attributes/payload-is-a-compile-time-constant`'s compile-time
/// constant to an enum case or a class constant, and those are one syntactic
/// form ([`ExprKind::ClassConstAccess`]): `Role::Admin` and `Policy::ADMIN`
/// differ only in what they resolve to, which § 2 promises not to ask. What is
/// refused is the bare literal — `allow: "admin"` is the magic string the
/// attribute exists to replace, and § 2's guarantee is precisely that the name
/// resolves.
///
/// § 1a's other two rules are not here, because neither is about one payload:
/// "exactly one `#[Access]` per method" is a question about an attribute list
/// and is [`check_one_access`], and `csrf: false` on a route whose every verb is
/// safe is a question about that method's verbs and is [`check_csrf_opt_out`].
pub(crate) fn check_access(attr: &Attribute, env: &mut Env<'_>) {
    let Some(field) = written(attr, ALLOW, env) else {
        env.diags.report(
            Diagnostic::error(
                code::E_ACCESS_INCOMPLETE,
                "this `#[Access]` declares no decision",
            )
            .with_primary(attr.span, "no `allow`")
            .with_help(
                "the decision is the attribute's one required field — write \
                 `#[Access(allow: Audience::Public)]` for a route that is genuinely open",
            ),
        );
        return;
    };
    let value = field.value.unparenthesized();
    if !matches!(value.kind, ExprKind::ClassConstAccess { .. }) {
        env.diags.report(
            Diagnostic::error(
                code::E_ACCESS_ALLOW_NOT_A_NAME,
                "an access decision is a name rather than a value",
            )
            .with_primary(value.span, "this names nothing that could resolve")
            .with_help(
                "`rule:security/access-is-checked-for-presence-not-meaning` never asks what a decision means, so what it asks instead is that \
                 the name resolves: an enum case or a class constant, as `Role::Admin` or \
                 `Audience::Public`",
            ),
        );
    }
}

/// `rule:attributes/access-payload`'s last rule: exactly one `#[Access]` per method, and a second
/// one is a compile error naming both.
///
/// Asked over a method's whole attribute list, which is why it is not in
/// [`check_access`] with the other two payload rules: a repeat is a fact about
/// the *list*, and the walk that checks one payload holds one attribute with no
/// list around it. Asked of every method rather than only of a `#[Route]`'s,
/// because § 1a says one per method and an `#[Access]` on a method with no route
/// is already an attribute that will be read by whoever dispatches — two of them
/// there are the same two readings.
///
/// `rule:attributes/repeatable` makes every attribute repeatable in general, and this is the narrowing
/// § 1a writes over it: a *third* is reported too, each against the first, so an
/// author deleting the extras is told about all of them at once rather than one
/// per rebuild.
pub(crate) fn check_one_access(groups: &[AttributeGroup], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut first: Option<Span> = None;
    for attr in groups.iter().flat_map(|group| &group.attributes) {
        if !crate::derive::attribute_is(attr, crate::derive::ACCESS, ctx, env) {
            continue;
        }
        let Some(declared) = first else {
            first = Some(attr.span);
            continue;
        };
        env.diags.report(
            Diagnostic::error(
                code::E_ACCESS_REPEATED,
                "this method declares more than one access decision",
            )
            .with_primary(attr.span, "a second `#[Access]`")
            .with_secondary(declared, "the decision this method already declares")
            .with_help(
                "two decisions are two readings — every one of them, or any one of them — and \
                 `rule:attributes/access-payload` refuses to choose between them silently: write the one decision \
                 the method makes, and let whatever reads it interpret one name",
            ),
        );
    }
}

/// `rule:routing/a-query-parameter-is-declared-like-a-capture`'s marker held to the declaration that reads it: a `#[Query]` on
/// a parameter of a method carrying no `#[Route]`.
///
/// Asked from [`crate::attributes`]' per-method walk rather than from
/// [`check_class_routes`], because that walk cannot see this mistake at all: it
/// selects the methods a `#[Route]` marks, and a stray `#[Query]` is by
/// definition on one of the others. [`crate::commands::check_stray_options`]
/// asks the identical question of `#[Option]`, and the two stay apart because
/// each belongs beside the pass whose attribute gives its marker a meaning.
///
/// Refused rather than ignored for the reason [`crate::derive::ATTRIBUTES`] is a
/// closed roster: a name the compiler knows, written where the compiler never
/// looks, reads to its author as a declaration that binds something.
///
/// Reported once per parameter, as [`check_one_access`] reports once per extra
/// attribute: each marker is its own mistake with its own span, so an author who
/// wrote three is told about all three rather than one per rebuild.
pub(crate) fn check_stray_query(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if m.attributes
        .iter()
        .flat_map(|group| &group.attributes)
        .any(|attr| crate::derive::attribute_is(attr, crate::derive::ROUTE, ctx, env))
    {
        return;
    }
    for param in &m.params {
        let Some(attr) =
            crate::testing::attribute_named(&param.attributes, crate::derive::QUERY, ctx, env)
        else {
            continue;
        };
        let name = crate::strip_sigil(span_text(env.src, param.name)).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_QUERY_WITHOUT_ROUTE,
                format!("`#[Query] ${name}` is on a method that declares no route"),
            )
            .with_primary(attr.span, "nothing reads this marker")
            .with_help(
                "`rule:routing/a-query-parameter-is-declared-like-a-capture` gives `#[Query]` its meaning on a `#[Route]` method's parameter, \
                 where the key it binds by is the parameter's own name — anywhere else nothing \
                 binds it: write the `#[Route]` this parameter serves, or delete the marker",
            ),
        );
    }
}

/// One row of `rule:routing/table-is-opt-in`'s table: a `#[Route]` that named both of the fields
/// a row cannot exist without, resolved to the strings the table is keyed by.
///
/// Public because the finished row is what crosses into `nvs-ir` — the same
/// arrangement [`crate::expr_table::Delegation`] has, and for its reason: what
/// rides across is a decision with no resolution left in it, so the consumer
/// holds strings rather than a second copy of this crate's tables.
#[derive(Debug)]
pub struct Route {
    /// The `Core\Http\Method` case by its own name — `Get`, `Post`. Kept as
    /// the case rather than as `rule:enums/no-class-machinery`'s backing integer because every
    /// reader of a row is a diagnostic or a link, and neither has anything to
    /// say about the integer.
    pub verb: String,
    /// `path` exactly as written, captures and all, and admitted by § 2's
    /// grammar — [`parse_path`] is what a reader splits it with rather than a
    /// second reading of the same string.
    pub path: String,
    /// § 1's optional `name`, with the span that wrote it — the span, because
    /// a duplicate is reported at the field rather than at the attribute.
    pub name: Option<(String, Span)>,
    /// `Class::method` the attribute is attached to, rendered as
    /// [`crate::expr_table::ExprTypeTable::method_label`] renders one.
    pub handler: String,
    /// Every parameter this route declares: § 2's path captures in path order
    /// first, then `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]` parameters in declaration order.
    ///
    /// On the row rather than left in the declaration because both readers are
    /// past the walk that built it. [`crate::links`] asks § 6's question after
    /// every file has been walked, by which time the method that declared these
    /// is in a file the walk has moved past; and
    /// `rule:routing/api-document-is-generated-from-the-route-table`
    /// reads the same rows out of the finished table, where the declaration
    /// is not in reach at all.
    pub params: Vec<RouteParam>,
    /// `rule:attributes/access-is-a-required-sibling`'s access decision, as the name it resolves to —
    /// `Core\Audience::Public`, `App\Role::Admin` — because
    /// `rule:security/access-is-checked-for-presence-not-meaning`
    /// leaves enforcement to whoever dispatches. The decision has to cross
    /// into `nvs-ir` on the row for that reason, exactly as [`Self::params`]
    /// does, and it is resolved here rather than left as written because a name
    /// depends on the file's imports and the row outlives the walk over that
    /// file.
    ///
    /// A string, and nothing structured, is the whole of § 2's promise: this
    /// compiler never asks what a decision *means*, only that the name
    /// resolves, so what rides across is the answer to that question and
    /// nothing else. `None` only where the declaration was already refused —
    /// no sibling `#[Access]` (§ 1), no `allow`, or an `allow` naming nothing
    /// (§ 1a) — so every row of a program that compiles carries a decision.
    pub access: Option<String>,
    /// `rule:attributes/access-payload`'s `csrf`, as the declaration answered it: `false` only
    /// where the sibling `#[Access]` wrote exactly that, and `true` everywhere
    /// else including where it wrote nothing.
    ///
    /// The *author's* answer and not § 4's whole one: whether there is a check
    /// here at all is a question about [`UNSAFE_VERBS`], which every reader of
    /// a row can ask of [`Self::verb`] and which the runtime's own row derives
    /// once at boot. Splitting it that way is what keeps the two independent —
    /// [`check_csrf_opt_out`] refuses the field where there is nothing to opt
    /// out of, so a `false` under four safe verbs belongs to a program that did
    /// not compile and never reaches a table.
    pub csrf: bool,
    /// `rule:routing/api-document-is-generated-from-the-route-table`
    /// 's summary: the first sentence of the declaration's own doc comment,
    /// or `None` where the method carries none.
    ///
    /// Split here rather than at the emitter, for the reason this whole row
    /// exists: what rides across is a decision with no resolution left in it,
    /// and *which sentence is the summary* is a reading of the source that the
    /// renderer would otherwise have to make a second time — once per document
    /// it writes, against text it can no longer see the source of.
    pub summary: Option<String>,
    /// The remainder of the same doc comment, whitespace-trimmed and with its
    /// paragraphs intact, or `None` where the comment was one sentence.
    pub description: Option<String>,
    /// § 1's response body: the handler's **declared** return type as
    /// [`crate::TypeInterner::describe`] renders it, the same spelling and for
    /// the same reason [`RouteParam::ty`] is one — the interner is dropped with
    /// the checking pass, and the emitter's whole use of a type is to choose a
    /// JSON Schema for it.
    ///
    /// `None` only where the signature table holds no row for the handler,
    /// which a program that compiles does not reach. Nothing is inferred from a
    /// `return` statement: § 1's promise is that the document says what the
    /// *code declares*, and a body's inferred type is not something an author
    /// wrote.
    pub returns: Option<String>,
    /// `rule:attributes/api-adds-and-cannot-contradict`'s `tags`, in the order the attribute wrote them.
    ///
    /// Empty where the method carries no `#[Api]`, and empty where it carries
    /// one that named no tags — the same value, because § 2's four are
    /// *additions* and adding nothing is what both of those do. The emitter
    /// omits an empty member for that reason rather than writing `[]`.
    ///
    /// One `#[Api]` describes the operation however many verbs the method
    /// serves ([`check_class_routes`] asks for it once, beside `#[Access]`), so
    /// every row a method produces carries the same four values.
    pub tags: Vec<String>,
    /// § 2's `security`: the scheme names, in written order and uninterpreted.
    /// [`check_api`]'s docs own why a name is carried rather than compared
    /// against a roster of configured schemes.
    pub security: Vec<String>,
    /// § 2's `errors`, in written order: the responses the declared return type
    /// cannot state.
    pub errors: Vec<ApiError>,
    /// § 2's `example`, folded to the constant it is — a [`ConstArg::Shape`]
    /// whose entries are the fields written, in written order.
    ///
    /// Folded here for [`Self::summary`]'s reason: what rides across is a value
    /// with no resolution left in it, and a payload's constant form is
    /// `rule:attributes/retrieval-folds-while-checking`'s fold — a reading of the source, over that file's own imports, that
    /// nothing past this pass can still make.
    pub example: Option<ConstArg>,
    /// The whole attribute.
    pub span: Span,
}

/// One entry of `rule:attributes/api-adds-and-cannot-contradict`'s `errors`: a response the declared return type cannot state, as the two
/// halves § 2 writes it with.
#[derive(Clone, Debug)]
pub struct ApiError {
    /// The response's own status code, which § 2 admits in `100..=599`.
    pub status: u16,
    /// The class the handler answers that status with, resolved — the spelling
    /// [`Route::access`] carries a decision by, and for its reason: a written
    /// name depends on the declaring file's imports, and the row outlives the
    /// walk over that file.
    pub class: String,
}

/// § 2's four values, read off one `#[Api]`.
///
/// Apart from [`Route`] because the attribute is a fact about the *method* and
/// a row is per verb: [`check_api`] produces one of these, and each row the
/// method declares takes its own copy of it.
#[derive(Debug)]
struct Api {
    tags: Vec<String>,
    security: Vec<String>,
    errors: Vec<ApiError>,
    example: Option<ConstArg>,
}

/// Where one of a route's parameters arrives from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamIn {
    /// § 2's capture: a segment of the matched path.
    Path,
    /// `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]` marker: a key of the query string.
    Query,
}

/// One parameter of a route, as the two readers past the walk need it.
///
/// Deliberately *not* a second copy of the declaration: it holds the name the
/// value binds by, where it arrives from, whether it may be absent, the
/// declared type rendered by [`crate::TypeInterner::describe`], and — where
/// that type is one — `rule:routing/a-capture-narrows-to-a-closed-set`
/// 's closed set of values it admits, the enum those values are cases of, and
/// whether that type is a class a segment reaches through its own `parse`.
/// Those are what
/// `rule:routing/api-document-is-generated-from-the-route-table`
/// 's document is built out of — the closed set is that section's
/// *Enumerations* row, `enum: [en, de, fr]` — and nothing else. A rendered type
/// rather than a `TypeId` because the interner that would answer it is dropped
/// with the checking pass, and because the emitter's whole use of a type is to
/// choose a JSON Schema for it.
#[derive(Debug)]
pub struct RouteParam {
    /// The parameter's own name, sigil-less — the capture's name (§ 3) or the
    /// query key, which are the same spelling for the same reason: the
    /// attribute carries nothing that could give either another one.
    pub name: String,
    /// Path or query.
    pub source: ParamIn,
    /// `false` for a `{name?}` capture and for a `#[Query]` parameter with a
    /// default, both of which are the declaration saying the value may be
    /// absent; `true` for everything else, including `{name...}`.
    pub required: bool,
    /// The declared type as [`crate::TypeInterner::describe`] renders it, or
    /// `None` where the signature gave none — which only happens in a program
    /// that has already been refused, since a capture naming no parameter is
    /// [`code::E_ROUTE_CAPTURE_UNBOUND`].
    pub ty: Option<String>,
    /// `rule:routing/a-capture-narrows-to-a-closed-set`'s closed set, as the segment text each admitted value is
    /// written with, in the order the union declares them — or `None` where the
    /// declared type is not a closed set at all, which is every `string`,
    /// `int`, `uint`, `decimal` capture and every capture at a class
    /// implementing `Parses`.
    ///
    /// Computed here, where the interner is still alive, for [`Self::ty`]'s
    /// reason exactly. An enum-case member contributes nothing and takes the
    /// whole set with it — see [`closed_set`], which owns why, and
    /// [`Self::admits`], which is how a reader asking for the whole set gets
    /// the enum half too.
    pub allowed: Option<Vec<String>>,
    /// `true` where the declared type is a class a segment reaches through its
    /// own `parse` — `rule:expressions/try-parse`'s pair read as `Parses`, which
    /// is [`crate::commands::is_parses_class`]'s question.
    ///
    /// Carried rather than derived for [`Self::ty`]'s reason and one more: a
    /// class and an enum render *identically*, as the qualified name, so a
    /// reader holding the rendering cannot tell the type whose wire form is a
    /// bare string from the one whose case spellings are still undecided.
    pub parses: bool,
    /// The enum this parameter narrows to, where its declared type is one or a
    /// subset of one — see [`enum_capture`], which decides both the spelling a
    /// segment matches on and the value it becomes.
    ///
    /// Beside [`Self::allowed`] rather than inside it because the two answer
    /// different halves: that field is the *set*, which is what a link is
    /// checked against and what the generated document lists, and this one is
    /// the set **plus what each spelling converts to**, which is the only thing
    /// that lets the match hand a program its case rather than the segment's
    /// text.
    pub cases: Option<EnumCapture>,
}

impl RouteParam {
    /// Every segment this parameter admits, in the order the declaration gives
    /// them, or `None` where its declared type names no set at all.
    ///
    /// [`Self::allowed`]'s literal union and [`Self::cases`]' enum subset are
    /// never both filled ([`closed_set`] owns why), and this is the one question
    /// every reader of either asks: `crate::links`' refusal of a link outside
    /// the set, and `nvs_cli::openapi`'s `enum:` row. One function rather than
    /// two readings, because
    /// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
    /// has a route, its links and its generated document spelled the same way,
    /// and two readers each deriving that from one field is how they would stop
    /// being.
    #[must_use]
    pub fn admits(&self) -> Option<Vec<&str>> {
        if let Some(allowed) = &self.allowed {
            return Some(allowed.iter().map(String::as_str).collect());
        }
        Some(
            self.cases
                .as_ref()?
                .cases
                .iter()
                .map(|(spelling, _)| spelling.as_str())
                .collect(),
        )
    }
}

/// The enum a capture or a `#[Query]` value narrows to: the enum's own name,
/// and every admitted case as the text that matches it beside the constant that
/// text becomes.
///
/// `nvs_types::commands::ArgConv::Enum`'s row for a command-line word, arrived
/// at independently and deliberately not shared: that one takes the case name
/// always, because a command line is a person typing, and this one takes
/// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`'s
/// spelling, because a route segment is written by a link and read by a
/// matcher. One type holding both would have to carry which rule filled it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumCapture {
    /// The enum's declared name, fully qualified.
    pub class: String,
    /// Every admitted case: the segment text it matches on, and the constant
    /// that text converts to. Ascending by value, with the name breaking a tie,
    /// so a subset reads the same on two builds — [`crate::enums::EnumInfo`]'s
    /// map has no declaration order to take, which is the same reason
    /// `nvs_types::commands`' twin sorts.
    pub cases: Vec<(String, crate::enums::EnumValue)>,
    /// `true` where [`Self::cases`]' spellings are the written backing values,
    /// and `false` where they are the case names — the rule's two halves, taken
    /// per subset by [`enum_capture`].
    ///
    /// Carried because a **link** asks a question of that choice the match does
    /// not. A `$params` entry written `Lang::Fr` is a case by name and an
    /// integer by value at once, so only this field says which of the two the
    /// segment it would build is — `crate::links`' `within_set` reads it to
    /// refuse a case outside the subset, and its `spellings` reads it to decide
    /// whether run time has a conversion left to make at all.
    pub by_value: bool,
}

/// Every route the program declares, in the order they were walked — file by
/// file in `nvs_hir::resolve_program`'s entry-first load order, and by
/// declaration within a file.
///
/// A `Vec` rather than a map keyed by either of the two things a route is
/// looked up by, because *both* keys have a duplicate error attached to them:
/// a map would drop the row a collision is reported against, and § 2's
/// precedence makes the path key a shape rather than the written text. Order
/// is what makes the pair deterministic — a duplicate is always reported at
/// the row that arrives second, and the load order it arrives in does not
/// depend on filesystem enumeration (`rule:programs/implementing`).
#[derive(Debug, Default)]
pub struct RouteTable {
    rows: Vec<Route>,
}

impl RouteTable {
    /// Every row, in load order.
    #[must_use]
    pub fn rows(&self) -> &[Route] {
        &self.rows
    }

    /// The row § 1's `name` names, or `None` where no route claims it — § 4's
    /// `url`/`urlAbsolute` reverse the table by exactly this question.
    ///
    /// The first match, and never an ambiguous one: two rows may share a name
    /// only when they share a path (`rule:routing/repeated-routes-share-a-name-when-they-share-a-path`),
    /// so name-to-path is a function and every other duplicate is
    /// [`code::E_DUPLICATE_ROUTE_NAME`] and does not compile.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Route> {
        self.rows.iter().find(|row| {
            row.name
                .as_ref()
                .is_some_and(|(claimed, _)| claimed == name)
        })
    }
}

/// Every `#[Route]` `decl` declares, collected into `env`'s table.
///
/// A no-op — not even a lookup — for a class carrying no `#[Route]`, which is
/// what keeps § 5's "a program with no `#[Route]` anywhere pays nothing at all,
/// including no pass" true of this walk as well as of the scan.
///
/// Run from [`crate::check`]'s per-class walk rather than from
/// [`crate::attributes`]', for [`crate::commands::check_class_commands`]'
/// reason plus one of its own: a row is keyed by the class and method it is
/// attached to, and the per-attribute walk holds a payload with no declaration
/// around it.
///
/// **Every** `#[Route]` on the method becomes a row, which is `rule:attributes/repeatable`'s
/// repetition read as `rule:routing/route-attribute` writes it — one method serving two verbs
/// declares two routes. The method's *other* two questions are asked once
/// against the first of them: § 1's sibling `#[Access]` and § 4's `csrf`
/// opt-out are facts about the method, so asking them per attribute would
/// report one mistake once per verb.
pub(crate) fn check_class_routes(
    decl: &ClassDecl,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let routes: Vec<&Attribute> = m
            .attributes
            .iter()
            .flat_map(|group| &group.attributes)
            .filter(|attr| crate::derive::attribute_is(attr, crate::derive::ROUTE, ctx, env))
            .collect();
        let Some(&first) = routes.first() else {
            continue;
        };
        let handler = format!("{class}::{}", span_text(env.src, m.name));
        let access = check_access_declared(first, m, &handler, ctx, env);
        if let Some(access) = access {
            check_csrf_opt_out(access, m, ctx, env);
        }
        // `rule:attributes/api-adds-and-cannot-contradict`'s annotation is a fact about the *method* — one
        // `#[Api]` describes the operation however many verbs it serves — so
        // it is asked once here beside `#[Access]`, and not per row.
        let mut api = None;
        if let Some(attr) =
            crate::testing::attribute_named(&m.attributes, crate::derive::API, ctx, env)
        {
            api = Some(check_api(attr, m, class, ctx, env));
        }
        // § 1's summary and description, read once for the method: one doc
        // comment describes the operation however many verbs it serves, which
        // is `#[Api]`'s arrangement two lines up and for its reason.
        let doc = doc_comment(member.doc.as_ref(), env.src);
        let returns = declared_return(m, class, env);
        let handler = Handler {
            m,
            class,
            label: &handler,
            doc: doc.as_ref(),
            returns: returns.as_deref(),
            api: api.as_ref(),
        };
        for attr in routes {
            collect_route(attr, &handler, access, ctx, env);
        }
    }
}

/// `rule:attributes/access-is-a-required-sibling`'s presence rule: a `#[Route]` whose method carries no
/// `#[Access]`.
///
/// Asked here rather than in the per-attribute walk because it is a question
/// about a *method* — the walk that checks one payload cannot see the sibling
/// it is missing — and asked of every `#[Route]`, including one this pass then
/// refuses to build a row out of: a route with a broken path still declares a
/// route, and § 3's reasoning does not wait for the path to parse.
///
/// The diagnostic points at the `#[Route]` and not at the method, because the
/// attribute is what makes the declaration owe a decision. Suggesting
/// `Audience::Public` in the help is § 1's own wording, and the case it names
/// is [`nvs_stdlib::router::AUDIENCE`] — the point of naming it in `Core` is
/// that the fix for this error is a name that already resolves.
///
/// The sibling it found is handed back rather than dropped, because this is
/// the one walk that looks for it and [`collect_route`] needs the same
/// attribute to fill [`Route::access`]; searching the method's attributes a
/// second time would be the same lookup answered twice.
fn check_access_declared<'a>(
    attr: &Attribute,
    m: &'a MethodMember,
    handler: &str,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<&'a Attribute> {
    if let Some(access) =
        crate::testing::attribute_named(&m.attributes, crate::derive::ACCESS, ctx, env)
    {
        return Some(access);
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ROUTE_WITHOUT_ACCESS,
            format!("the route `{handler}` declares no access decision"),
        )
        .with_primary(attr.span, "this route has no sibling `#[Access]`")
        .with_help(
            "§ 3 gives an omission no default, because a route that is open by decision and one \
             that is open by oversight would otherwise read alike — write \
             `#[Access(allow: Core\\Audience::Public)]` on the method if it is genuinely open",
        ),
    );
    None
}

/// `rule:attributes/access-payload`'s `csrf` for [`Route::csrf`]: `false` only where the sibling
/// `#[Access]` wrote that value, and `true` for every other row.
///
/// The same field [`check_csrf_opt_out`] reads, asked for the other question,
/// and deliberately a second walk rather than one answer threaded out of the
/// first: that one *reports* and answers for no row, this one is total and
/// reports nothing. A row whose opt-out was refused still carries `false`
/// here, which costs nothing — its program does not compile, so no table is
/// built from it.
fn csrf_of(access: Option<&Attribute>, env: &mut Env<'_>) -> bool {
    let Some(access) = access else {
        return true;
    };
    let Some(field) = written(access, CSRF, env) else {
        return true;
    };
    let value = field.value.clone();
    let declared = env.interner.intern(crate::ty::Ty::Bool);
    !matches!(
        crate::defaults::literal_default(&value, declared, env),
        Some(crate::defaults::ConstArg::Bool(false))
    )
}

/// `rule:security/csrf-is-on-by-default`'s opt-out, held to the thing it opts out of: `csrf: false`
/// beside a method whose every `#[Route]` names one of the verbs outside
/// [`UNSAFE_VERBS`].
///
/// The verbs are the *method's* rather than the row's, which is why this is
/// asked here and not in [`collect_route`]: § 1a gives a method one `#[Access]`
/// covering every `#[Route]` it carries, so a single unsafe verb among them is
/// a check to opt out of and the field is doing its job. Asking it of the row
/// instead would report one mistake once per verb and would still have to read
/// the sibling attributes to know it was the same mistake.
///
/// Only `false` is refused. `csrf: true` on a safe route restates § 4's default
/// rather than claiming anything untrue, and refusing a *restatement* would be
/// this compiler holding an opinion about style.
///
/// A verb [`verb_of`] cannot read counts as unsafe and the opt-out stands: the
/// roster walk has already reported that field, and refusing the opt-out too
/// would name the author's second problem before their first.
fn check_csrf_opt_out(access: &Attribute, m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let Some(field) = written(access, CSRF, env) else {
        return;
    };
    let value = field.value.clone();
    let span = field.span;
    let declared = env.interner.intern(crate::ty::Ty::Bool);
    if !matches!(
        crate::defaults::literal_default(&value, declared, env),
        Some(crate::defaults::ConstArg::Bool(false))
    ) {
        return;
    }
    let has_something_to_opt_out_of =
        m.attributes
            .iter()
            .flat_map(|group| &group.attributes)
            .any(|attr| {
                crate::derive::attribute_is(attr, crate::derive::ROUTE, ctx, env)
                    && match verb_of(attr, ctx, env) {
                        Some(verb) => UNSAFE_VERBS.contains(&verb.as_str()),
                        None => true,
                    }
            });
    if has_something_to_opt_out_of {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_CSRF_WITHOUT_UNSAFE_VERB,
            "this route has no CSRF check to opt out of",
        )
        .with_primary(span, "every `#[Route]` on this method names a safe verb")
        .with_help(
            "`rule:security/csrf-is-on-by-default` turns CSRF on for `Post`, `Put`, `Patch` and `Delete` and for no other \
             verb — delete the field, or write it on the route that is actually unsafe",
        ),
    );
}

/// `#[Api(tags?: string[], errors?: {status: int, type: Class::class}[],
/// security?: string[], example?: {…})]` — `rule:attributes/api-adds-and-cannot-contradict`'s own spelling, in the
/// order that section writes it.
///
/// Every row is [`OptionTy::Mixed`], and that is not this roster giving up. Of
/// the five types [`OptionTy`] can place a value at, none is an array or a
/// shape, and adding two rows for one attribute would put the *structure* of
/// `#[Api]`'s payload in a table shared by four other attributes that have no
/// use for it — while leaving every question worth asking here unanswered
/// anyway, because § 2's rule is not "this field is an array of strings" but
/// "this field does not contradict the code". So the roster keeps the two
/// questions a roster is for — a name no row declares, and a name given twice
/// — and [`check_api`] reads the values, having the declaration in hand.
pub(crate) const API_OPTIONS: &[(&str, OptionTy)] = &[
    (TAGS, OptionTy::Mixed),
    (ERRORS, OptionTy::Mixed),
    (SECURITY, OptionTy::Mixed),
    (EXAMPLE, OptionTy::Mixed),
];

/// `rule:attributes/api-adds-and-cannot-contradict`'s fourth contradiction: an `#[Api]` on a method carrying no
/// `#[Route]`.
///
/// [`check_stray_query`]'s question about the other marker that means nothing
/// on its own, asked from the same walk for the same reason — the pass that
/// would refuse it selects methods by their `#[Route]`, so a method with no
/// `#[Route]` is exactly the one it never visits.
///
/// It is § 2's cheapest contradiction and its most literal: an annotation
/// describing an operation, on something that is not one. Nothing reads it,
/// no document carries it, and the author's mistake is either a missing
/// `#[Route]` or an attribute on the wrong method — so the help names both
/// rather than guessing.
pub(crate) fn check_stray_api(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if m.attributes
        .iter()
        .flat_map(|group| &group.attributes)
        .any(|attr| crate::derive::attribute_is(attr, crate::derive::ROUTE, ctx, env))
    {
        return;
    }
    let Some(attr) = crate::testing::attribute_named(&m.attributes, crate::derive::API, ctx, env)
    else {
        return;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_API_CONTRADICTS_THE_CODE,
            "this `#[Api]` annotates no operation",
        )
        .with_primary(attr.span, "the method carries no `#[Route]`")
        .with_help(
            "`rule:attributes/api-adds-and-cannot-contradict`'s `#[Api]` supplies what a route's own types cannot say, so away from a \
             `#[Route]` there is nothing for it to say it about — add the `#[Route]`, or move the \
             `#[Api]` to the method that has one",
        ),
    );
}

/// `rule:attributes/access-is-a-required-sibling`'s sibling rule read from the other side: an `#[Access]` on a
/// method carrying no `#[Route]`.
///
/// § 1 states the rule as a route owing a decision, which is
/// [`check_class_routes`]' `E_ROUTE_WITHOUT_ACCESS`. The same sentence asked of
/// a method that declares no route at all is this, and it is a refusal rather
/// than a silence because § 2 promises the compiler will never interpret what
/// `allow` names: the route table is the only thing that ever reads one, so an
/// `#[Access]` away from a `#[Route]` guards nothing and cannot be made to.
///
/// Asked from [`crate::attributes`]' per-method walk for [`check_stray_query`]'s
/// reason — [`check_class_routes`] answers over the methods a `#[Route]` marks,
/// and a stray `#[Access]` is by definition on one of the others.
///
/// Reported for the first `#[Access]` alone, where [`check_stray_query`] reports
/// per parameter: § 1a admits one per method and a second is
/// [`check_one_access`]'s own error, so naming every one of them here would
/// report the author's single mistake once per attribute they are about to
/// delete together.
pub(crate) fn check_stray_access(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if m.attributes
        .iter()
        .flat_map(|group| &group.attributes)
        .any(|attr| crate::derive::attribute_is(attr, crate::derive::ROUTE, ctx, env))
    {
        return;
    }
    let Some(attr) =
        crate::testing::attribute_named(&m.attributes, crate::derive::ACCESS, ctx, env)
    else {
        return;
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ACCESS_WITHOUT_ROUTE,
            "this `#[Access]` guards no route",
        )
        .with_primary(attr.span, "the method carries no `#[Route]`")
        .with_help(
            "`rule:attributes/access-is-a-required-sibling` makes `#[Access]` a `#[Route]`'s required sibling, and § 2 keeps the \
             compiler from reading what `allow` names — so the route table is the only thing that \
             ever asks this decision: add the `#[Route]` it guards, or delete it",
        ),
    );
}

/// `rule:attributes/api-adds-and-cannot-contradict`'s first three contradictions, asked of one method's `#[Api]`.
///
/// § 2's whole rule is that the annotation **may add and may not contradict**,
/// and that is checkable only where both halves are in hand — the payload and
/// the declaration it describes. That is this walk and not
/// [`crate::attributes`]', for [`check_class_routes`]' own reason.
///
/// The three asked here:
///
/// - an **`errors`** entry whose `type` is not a class a handler could
///   produce. § 2's own wording, read as the widest question that is both
///   sound and *not already asked*: the type is instantiable —
///   [`nvs_hir::ClassLinks::concrete`], so not an interface, not `abstract`,
///   and not an enum. A name resolving to nothing is deliberately **not** this
///   error: it is `E_UNDEFINED_CLASS` from the expression walk that already
///   read the `::class`, and a second diagnostic would name the author's one
///   mistake twice. The narrower question § 2's phrase could also mean — does
///   *this* handler reach *that* class — is asked by nothing, and cannot be:
///   Novis has no `throws` clause, and
///   `rule:routing/matching-is-not-dispatching` keeps
///   this compiler out of the handler's body on purpose. The roster is
///   [`crate::signatures::SignatureTable`], which [`crate::error_lib::seed`]
///   has already filled with spec § 10's tree, so `Core\NotFound` answers
///   exactly as an application's own class does.
/// - an **`example`** naming a field the return type does not declare. § 2
///   asks for the example to decode "with the same decoder `rule:core-classes/derive-attribute` already
///   built", and the decoder's first question is this one: a key naming no
///   property is a bad field, and a bad field is the whole of what a derived
///   decode reports. Asked only where the return type is a class this program
///   declares — against a scalar, an array or an absent annotation there is no
///   field roster to disagree with, and inventing one would refuse an example
///   that is correct.
/// - a **`tags`** or **`security`** entry that is not a string, which is the
///   structural half the roster cannot state.
///
/// **§ 2's `security` scheme check is written down to the point the tree can
/// reach and no further, deliberately.** "A scheme name that no configured
/// scheme defines" needs a configured scheme, and nothing in this compiler
/// declares one yet — there is no configuration surface for a security scheme
/// anywhere, which `crates/nvs-cli/src/openapi.rs`'s own gap list already
/// records. Refusing every name against an empty roster would refuse `rule:attributes/api-adds-and-cannot-contradict`'s own example, so what stands here today is the shape and the name is
/// carried uninterpreted, exactly as `rule:security/access-is-checked-for-presence-not-meaning` carries an access decision.
/// The comparison lands in this function, unchanged, on the day a scheme has a
/// home.
///
/// Each of the four walks hands back what it read, which is the whole of the
/// recording half: a value only reaches [`Route`] once the walk that checks it
/// has accepted it, so a row of a program that compiles never carries a value
/// § 2 refused, and neither does the document built from one.
fn check_api(
    api: &Attribute,
    m: &MethodMember,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Api {
    Api {
        tags: check_string_list(api, TAGS, env),
        security: check_string_list(api, SECURITY, env),
        errors: check_api_errors(api, ctx, env),
        example: check_api_example(api, m, class, ctx, env),
    }
}

/// One `#[Api]` option written as an array of `string`s — [`TAGS`] and
/// [`SECURITY`], which have the same shape and differ only in what a later
/// pass will do with the strings.
fn check_string_list(api: &Attribute, option: &str, env: &mut Env<'_>) -> Vec<String> {
    let Some(field) = written(api, option, env) else {
        return Vec::new();
    };
    let (value, span) = (field.value.clone(), field.span);
    let ExprKind::ArrayLiteral(items) = &value.kind else {
        report_api(
            format!("`{option}` is not a list"),
            span,
            "written as a single value",
            format!(
                "`rule:attributes/api-adds-and-cannot-contradict` writes `{option}` as an array, because an operation may carry more \
                 than one — write `{option}: [...]` even for a list of one"
            ),
            env,
        );
        return Vec::new();
    };
    let declared = env.interner.intern(crate::ty::Ty::String);
    let mut names = Vec::with_capacity(items.len());
    for item in items {
        let entry = item.value.clone();
        let entry_span = entry.span;
        match crate::defaults::literal_default(&entry, declared, env) {
            Some(ConstArg::Str(name)) => names.push(name),
            _ => report_api(
                format!("this `{option}` entry is not a string"),
                entry_span,
                "not a string",
                format!("every entry of `{option}` is a name, and a name is written as a string"),
                env,
            ),
        }
    }
    names
}

/// § 2's `errors`: each entry a `{status, type}` whose `type` names a class
/// this program declares.
fn check_api_errors(api: &Attribute, ctx: &Ctx<'_>, env: &mut Env<'_>) -> Vec<ApiError> {
    let Some(field) = written(api, ERRORS, env) else {
        return Vec::new();
    };
    let (value, span) = (field.value.clone(), field.span);
    let ExprKind::ArrayLiteral(items) = &value.kind else {
        report_api(
            "`errors` is not a list".to_owned(),
            span,
            "written as a single value",
            "`rule:attributes/api-adds-and-cannot-contradict` writes `errors` as an array of `{status: …, type: …}` entries — write \
             `errors: [{...}]` even for one",
            env,
        );
        return Vec::new();
    };
    let int_ty = env.interner.intern(crate::ty::Ty::Int);
    let mut recorded = Vec::with_capacity(items.len());
    for item in items {
        let entry = item.value.clone();
        let ExprKind::ObjectLiteral(fields) = &entry.kind else {
            report_api(
                "this `errors` entry is not a `{status, type}`".to_owned(),
                entry.span,
                "not a shape literal",
                "§ 2 writes each entry as `{status: 404, type: Api\\NotFound::class}` — the two \
                 halves of one error response",
                env,
            );
            continue;
        };
        let mut status = None;
        let mut ty = None;
        for entry_field in fields {
            match span_text(env.src, entry_field.name) {
                STATUS => status = Some(entry_field.clone()),
                TYPE => ty = Some(entry_field.clone()),
                other => {
                    let (other, other_span) = (other.to_owned(), entry_field.span);
                    report_api(
                        format!("`{other}` is not part of an `errors` entry"),
                        other_span,
                        "no such field",
                        "§ 2's entry carries a `status` and a `type` and nothing else",
                        env,
                    );
                }
            }
        }
        let status = match &status {
            Some(field) => {
                let written = field.value.clone();
                // Narrowed to `u16` here rather than at the row: § 2's range is
                // what makes a status one, so the value the row carries is the
                // one this walk accepted and nothing wider.
                let code = match crate::defaults::literal_default(&written, int_ty, env) {
                    Some(ConstArg::Int(code)) => u16::try_from(code)
                        .ok()
                        .filter(|code| (100..=599).contains(code)),
                    _ => None,
                };
                if code.is_none() {
                    report_api(
                        "this `status` is not an HTTP status code".to_owned(),
                        field.span,
                        "not in 100-599",
                        "an `errors` entry's `status` is the response's own code, so it is one \
                         HTTP defines",
                        env,
                    );
                }
                code
            }
            None => {
                report_api(
                    "this `errors` entry declares no `status`".to_owned(),
                    entry.span,
                    "no `status`",
                    "§ 2's entry pairs a status with the type returned at it — an entry naming \
                     only one of the two describes no response",
                    env,
                );
                None
            }
        };
        let Some(ty) = ty else {
            report_api(
                "this `errors` entry declares no `type`".to_owned(),
                entry.span,
                "no `type`",
                "§ 2's entry pairs a status with the type returned at it — an entry naming only \
                 one of the two describes no response",
                env,
            );
            continue;
        };
        // Recorded only where both halves survived their own walk: an entry
        // this section refused describes no response, and a document built out
        // of a program that compiles never reaches one.
        let Some(class) = check_error_type(&ty, ctx, env) else {
            continue;
        };
        if let Some(status) = status {
            recorded.push(ApiError { status, class });
        }
    }
    recorded
}

/// One `errors` entry's `type`, held to § 2's "a class the handler could
/// produce" as far as [`check_api`]'s docs say that is askable: it names a
/// class this program has.
///
/// The resolved name is handed back where the entry is one § 2 accepts, and
/// `None` everywhere a refusal was reported or already had been — so what a row
/// carries is exactly the set of responses this walk agreed the handler could
/// produce.
fn check_error_type(
    field: &nvs_syntax::ast::ObjectLiteralField,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<String> {
    let (value, span) = (field.value.clone(), field.span);
    let ExprKind::ClassNameConst { class } = &value.kind else {
        report_api(
            "this `type` does not name a class".to_owned(),
            span,
            "not a `::class`",
            "§ 2's `type` is the class the error response carries, written `Api\\NotFound::class` \
             — a name resolves through `rule:programs/no-runtime-autoload`'s autoload map, which a string would not",
            env,
        );
        return None;
    };
    let Some(qname) = crate::expr::members::resolve_class_expr(class, ctx, env) else {
        // A dynamic class side is already `E_CLASS_NAME_CONST_NOT_STATIC` from
        // the expression walk, and naming it again here would report one
        // mistake twice.
        return None;
    };
    // Declared at all — asked of both tables, because neither holds every
    // kind: an enum has no [`nvs_hir::ClassGraph`] entry by construction, and
    // an interface declaring no member reaches no
    // [`crate::signatures::SignatureTable`] row. A name in neither is
    // `E_UNDEFINED_CLASS`, already reported by the walk that checked this
    // expression — § 2's question is the one that survives *after* the name
    // resolves, and asking it again here would name the author's one mistake
    // twice.
    if env.graph.get(&qname).is_none() && env.signatures.get(&qname).is_none() {
        return None;
    }
    if env.graph.get(&qname).is_some_and(|links| links.concrete) {
        return Some(qname.to_string());
    }
    report_api(
        format!("`{qname}` is not a class a handler could produce"),
        span,
        "abstract, an interface, or an enum",
        "§ 2's `errors` may add a response the types cannot state, but not one whose type nothing \
         could ever be — write the concrete class the handler answers this status with",
        env,
    );
    None
}

/// § 2's `example`, against the field roster `rule:core-classes/derive-attribute`'s decoder reads: every
/// key names a property of the return type.
///
/// The fold that records it runs **before** that roster walk and independently
/// of it: the three ways below to have no roster at all — a handler the
/// signature table has no row for, a return type that is not a class, a class
/// with no properties — are silence rather than a refusal, so a `#[Api]` on one
/// of them still carries its example into the document.
fn check_api_example(
    api: &Attribute,
    m: &MethodMember,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let field = written(api, EXAMPLE, env)?;
    let (value, span) = (field.value.clone(), field.span);
    let ExprKind::ObjectLiteral(fields) = &value.kind else {
        report_api(
            "`example` is not a shape literal".to_owned(),
            span,
            "not a shape literal",
            "§ 2's `example` is one instance of what the operation answers with, written \
             `{field: value, …}`",
            env,
        );
        return None;
    };
    let recorded = fold_example(fields, ctx, env);
    // The return type as the signature table holds it, which is the same
    // annotation `rule:core-classes/derive-attribute`'s derive reads. Anything that is not a declared
    // class has no field roster to disagree with — see this module's
    // [`check_api`] docs for why that is silence rather than a refusal.
    let method = span_text(env.src, m.name);
    let Some(returns) = env
        .signatures
        .get(class)
        .and_then(|sig| sig.methods.get(method))
        .map(|sig| sig.return_ty)
    else {
        return recorded;
    };
    let crate::ty::Ty::Class(returns, _) = env.interner.get(returns).clone() else {
        return recorded;
    };
    let Some(properties) = env
        .signatures
        .get(&returns)
        .map(|sig| sig.properties.keys().cloned().collect::<Vec<_>>())
    else {
        return recorded;
    };
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        if properties.iter().any(|declared| declared == &name) {
            continue;
        }
        let field_span = field.span;
        report_api(
            format!("`{returns}` declares no `{name}`"),
            field_span,
            "no such field",
            "§ 2's example is decoded by the codec the return type generates, so a key naming no \
             property is a field that decode would reject",
            env,
        );
    }
    recorded
}

/// § 2's `example` as the constant it is: one entry per written field, in
/// written order, over the declaring file's own imports.
///
/// `rule:attributes/retrieval-folds-while-checking`'s fold, which is the same one a retrieval's payload goes through — so an
/// enum case and a `Foo::class` in an example are the values they name rather
/// than the text that names them, and the emitter is handed a document's worth
/// of already-decided JSON.
///
/// A field with no constant form takes the whole example with it, and silently:
/// `crate::attributes`' own walk over this payload has already refused a value
/// that is not constant, and § 2 has four ways to be false without this one
/// inventing a fifth.
fn fold_example(
    fields: &[nvs_syntax::ast::ObjectLiteralField],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let mut folded = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        let value = field.value.clone();
        folded.push((
            name,
            crate::defaults::fold_constant_value(&value, Some(ctx), env)?,
        ));
    }
    Some(ConstArg::Shape(folded))
}

/// One `rule:attributes/api-adds-and-cannot-contradict` contradiction, reported.
///
/// Every one of them is [`code::E_API_CONTRADICTS_THE_CODE`] with its own
/// message, which is that code's own reasoning: § 2 states four ways for one
/// sentence to be false, and what tells them apart is what the message says
/// rather than a number.
fn report_api(
    message: String,
    span: Span,
    label: &'static str,
    help: impl Into<String>,
    env: &mut Env<'_>,
) {
    env.diags.report(
        Diagnostic::error(code::E_API_CONTRADICTS_THE_CODE, message)
            .with_primary(span, label)
            .with_help(help.into()),
    );
}

/// One `#[Route]` payload as a row, or the refusal that it is not one.
///
/// § 1's shape marks only `name` optional, and this is where that is enforced
/// rather than in [`crate::attributes::check_roster`]: the roster says what a
/// field may hold, and *required* is a fact about the row being built, which is
/// the reading [`crate::commands`]' own gap gives `#[Command]`'s `name` for the
/// same reason. Both missing fields are named in one diagnostic, because an
/// author who wrote neither wrote the empty attribute once.
fn collect_route(
    attr: &Attribute,
    handler: &Handler<'_>,
    access: Option<&Attribute>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Handler {
        m,
        class,
        label: handler,
        doc,
        returns,
        api,
    } = *handler;
    let path = folded_str(attr, PATH, env);
    let verb = verb_of(attr, ctx, env);
    let (Some((path, path_span)), Some(verb)) = (path, verb) else {
        // A field written at the wrong type has already been reported by the
        // roster walk, and reporting it again as a missing one would name the
        // author's second problem before their first.
        if written(attr, PATH, env).is_some() && written(attr, METHOD, env).is_some() {
            return;
        }
        let missing = match (written(attr, PATH, env), written(attr, METHOD, env)) {
            (None, None) => "a `path` and a `method`",
            (None, _) => "a `path`",
            _ => "a `method`",
        };
        env.diags.report(
            Diagnostic::error(
                code::E_ROUTE_INCOMPLETE,
                format!("this `#[Route]` on `{handler}` gives no {missing}"),
            )
            .with_primary(attr.span, "not enough to build a route")
            .with_help(
                "a route is a path and a verb together, and only `name` is optional — write \
                 `#[Route(path: \"/users\", method: Core\\Http\\Method::Get)]`",
            ),
        );
        return;
    };
    let captures = match parse_path(&path) {
        Ok(captures) => captures,
        Err(Refusal { what, help }) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_ROUTE_PATH_GRAMMAR,
                    format!("`{handler}`'s path {what}"),
                )
                .with_primary(path_span, "the router cannot match this path")
                .with_help(help),
            );
            // The row is not pushed. A path with no grammar has no shape
            // either, so keeping it would report the author a duplicate of
            // something as their second problem before they have fixed their
            // first.
            return;
        }
    };
    let mut params = check_captures(&captures, path_span, m, class, handler, env);
    params.extend(query_params(m, class, ctx, env));
    let name = folded_str(attr, NAME, env);
    // Read before `access` is resolved to its name, because it is a field of
    // the attribute and that binding is about to become the string the row
    // carries instead.
    let csrf = csrf_of(access, env);
    let access = access.and_then(|access| access_name(access, ctx, env));
    env.routes.rows.push(Route {
        verb,
        path,
        name,
        handler: handler.to_owned(),
        params,
        access,
        csrf,
        summary: doc.map(|doc| doc.summary.clone()),
        description: doc.and_then(|doc| doc.description.clone()),
        returns: returns.map(str::to_owned),
        // § 2's four, copied per row: one `#[Api]` describes the operation
        // however many verbs the method serves, and a row is per verb.
        tags: api.map(|api| api.tags.clone()).unwrap_or_default(),
        security: api.map(|api| api.security.clone()).unwrap_or_default(),
        errors: api.map(|api| api.errors.clone()).unwrap_or_default(),
        example: api.and_then(|api| api.example.clone()),
        span: attr.span,
    });
}

/// The method a `#[Route]` is attached to, as [`collect_route`] needs it.
///
/// One argument rather than five because every field is a fact about the
/// *method* and is therefore the same for every row it produces — a method
/// serving two verbs declares two routes off one of these — and because the row
/// builder was the one function in this module that had grown past what a
/// reader can hold in their head at the call site.
#[derive(Clone, Copy)]
struct Handler<'a> {
    /// The declaration itself: its parameter list is what a capture and a
    /// `#[Query]` marker are checked against.
    m: &'a MethodMember,
    /// The declaring class, for the same two walks.
    class: &'a QName,
    /// `Class::method` as [`crate::expr_table::ExprTypeTable::method_label`]
    /// renders it, built once for the method rather than per row.
    label: &'a str,
    /// `rule:routing/api-document-is-generated-from-the-route-table`'s summary and description, or `None` where the method
    /// carries no doc comment.
    doc: Option<&'a Doc>,
    /// § 1's response body: the declared return type, rendered.
    returns: Option<&'a str>,
    /// § 2's four values as [`check_api`] read them, or `None` where the method
    /// carries no `#[Api]` — borrowed rather than owned for this struct's own
    /// reason: they are a fact about the method, and each row copies out the
    /// four it needs.
    api: Option<&'a Api>,
}

/// `rule:routing/api-document-is-generated-from-the-route-table`
/// 's last row, as the two strings it is: *first sentence is the summary,
/// remainder the description*.
struct Doc {
    /// The first sentence, collapsed onto one line — a summary that wrapped
    /// across three source lines is one string here, because the reader of a
    /// generated document has no margin to wrap it back to.
    summary: String,
    /// Everything after that sentence, trimmed but with its paragraph breaks
    /// intact, or `None` where the comment was one sentence and no more.
    description: Option<String>,
}

/// The member's attached `///` run, split by [`Doc`]'s rule, or `None` where
/// there is none.
///
/// The run is the one the parser attached
/// (`rule:tooling/doc-comment-attaches-to-the-next-declaration`), so it is the
/// comment written above `#[Route]` — where a reader writes it — and not one
/// wedged between the attributes and `public function`. A `/** … */` block or
/// a `//` run above the member is an ordinary comment nothing reads, and
/// documents nothing here either.
fn doc_comment(doc: Option<&DocComment>, src: &SourceFile) -> Option<Doc> {
    let lines: Vec<&str> = doc?
        .lines
        .iter()
        .map(|&line| prose(span_text(src, line)))
        .collect();
    split_doc(lines.join("\n").trim())
}

/// One `///` line with its marker off, and the one space after it that a
/// writer leaves and does not mean.
///
/// Exactly one space and never a trim of the front: indentation is Markdown's
/// own syntax, so an indented example in the description stays one. The
/// trailing whitespace does go, so a line's invisible tail never reaches the
/// document.
fn prose(line: &str) -> &str {
    let body = line.strip_prefix(DOC_MARKER).unwrap_or(line);
    body.strip_prefix(' ').unwrap_or(body).trim_end()
}

/// § 1's response body row: the handler's declared return type, rendered.
///
/// Read out of [`crate::signatures`] rather than off [`MethodMember`]'s own
/// `return_type` because that is the syntax and this wants the *resolved* type —
/// the same lookup [`check_api_example`] makes for § 2's `example`, and by the
/// same three steps, so a method's return type is asked for once in this module
/// however many questions are asked of it.
fn declared_return(m: &MethodMember, class: &QName, env: &Env<'_>) -> Option<String> {
    let method = span_text(env.src, m.name);
    let ty = env
        .signatures
        .get(class)
        .and_then(|sig| sig.methods.get(method))
        .map(|sig| sig.return_ty)?;
    Some(env.interner.describe(ty))
}

/// § 1's split, over the run's prose.
///
/// The summary ends at the first `.` that a space or the end of the text
/// follows, or at the first blank line, whichever comes first — the blank line
/// because a comment whose opening line is a heading with no full stop would
/// otherwise swallow the whole block as its summary, and a document reader
/// renders `summary` on one line.
fn split_doc(text: &str) -> Option<Doc> {
    if text.is_empty() {
        return None;
    }
    let sentence = text
        .match_indices('.')
        .find(|(i, _)| text[i + 1..].starts_with(char::is_whitespace) || i + 1 == text.len())
        .map(|(i, _)| i + 1);
    let end = match (sentence, text.find("\n\n")) {
        (Some(sentence), Some(para)) => sentence.min(para),
        (found, None) | (None, found) => found.unwrap_or(text.len()),
    };
    let summary = text[..end].split_whitespace().collect::<Vec<_>>().join(" ");
    let description = text[end..].trim();
    Some(Doc {
        summary,
        description: (!description.is_empty()).then(|| description.to_owned()),
    })
}

/// § 2's three capture forms, each holding the name it binds. A segment that is
/// none of them is a literal, compared byte for byte and case-sensitively
/// (`rule:classes/names-resolve-case-sensitively`),
/// and is not held here at all.
#[derive(Clone, Copy, Debug)]
enum Capture<'a> {
    /// `{name}` — one whole segment.
    One(&'a str),
    /// `{name?}` — one whole segment or none.
    Optional(&'a str),
    /// `{name...}` — every remaining segment as one `tainted string`.
    Rest(&'a str),
}

impl<'a> Capture<'a> {
    /// The parameter name this capture binds to, sigil-less, as § 3 compares
    /// it.
    fn name(self) -> &'a str {
        match self {
            Self::One(name) | Self::Optional(name) | Self::Rest(name) => name,
        }
    }

    /// The capture as an author wrote it, for a diagnostic that has to point
    /// at one of three forms and cannot point at a span inside a folded
    /// literal.
    fn written(self) -> String {
        match self {
            Self::One(name) => format!("{{{name}}}"),
            Self::Optional(name) => format!("{{{name}?}}"),
            Self::Rest(name) => format!("{{{name}...}}"),
        }
    }
}

/// A path § 2's grammar does not admit: the clause that completes "this
/// handler's path …", and the help that follows it.
struct Refusal {
    what: String,
    help: &'static str,
}

/// § 2's grammar over one whole path: the captures it declares, in order, or
/// the one way it is not a path.
///
/// Pure, and separate from the diagnostic it feeds, so the grammar reads as the
/// rules § 2 writes rather than through the reporting around them. It stops at
/// the first refusal for the reason [`collect_route`] drops the row: a path is
/// one thing, and a reader fixing its first fault re-reads the rest anyway.
///
/// § 2's "at most once" and "never in the same path as a `{name...}`" are not
/// checked separately, because both fall out of *last position only*: one
/// segment is last, so a second trailing form is already somewhere it is
/// refused.
fn parse_path(path: &str) -> Result<Vec<Capture<'_>>, Refusal> {
    if !path.starts_with('/') {
        return Err(Refusal {
            what: "does not begin with `/`".to_owned(),
            help: "a route is matched against a request's path, which always begins at the \
                   root, so a path that does not start with `/` could match nothing",
        });
    }
    let segments: Vec<&str> = path.split('/').collect();
    let last = segments.len() - 1;
    let mut captures: Vec<Capture<'_>> = Vec::new();
    for (index, &segment) in segments.iter().enumerate() {
        let Some(capture) = capture_of(segment)? else {
            continue;
        };
        if index != last && !matches!(capture, Capture::One(_)) {
            return Err(Refusal {
                what: format!(
                    "writes `{}` somewhere other than the last position",
                    capture.written()
                ),
                help: "a capture that may absorb the end of a path has nothing to follow it, \
                       so `{name?}` and `{name...}` are written last or not at all",
            });
        }
        if captures.iter().any(|prior| prior.name() == capture.name()) {
            return Err(Refusal {
                what: format!("captures `{}` twice", capture.name()),
                help: "a capture arrives as the parameter it is named after, so two of one \
                       name are two values for one parameter",
            });
        }
        captures.push(capture);
    }
    Ok(captures)
}

/// One segment read as § 2's grammar: the capture it declares, `None` for a
/// literal segment, or the clause saying what it is instead.
///
/// A segment carrying a brace anywhere is held to being a capture *whole*.
/// There is no escape and no partial form, which is the half of § 2 that keeps
/// `{` an ordinary byte in a literal segment impossible rather than ambiguous:
/// a path meaning one of two things is refused rather than repaired
/// (`rule:errors/ambiguous-input-refused`).
fn capture_of(segment: &str) -> Result<Option<Capture<'_>>, Refusal> {
    if !segment.contains('{') && !segment.contains('}') {
        return Ok(None);
    }
    let Some(inner) = segment
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        return Err(Refusal {
            what: format!("has a segment `{segment}` that is neither a literal nor a capture"),
            help: "a capture is a whole segment — `/users/{id}`, never `/users/u{id}` or \
                   `/users/{id}.json`",
        });
    };
    let capture = if let Some(name) = inner.strip_suffix("...") {
        Capture::Rest(name)
    } else if let Some(name) = inner.strip_suffix('?') {
        Capture::Optional(name)
    } else {
        Capture::One(inner)
    };
    let name = capture.name();
    if name.is_empty()
        || name.starts_with(|c: char| c.is_ascii_digit())
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(Refusal {
            what: format!("writes `{segment}`, which captures no name a parameter could have"),
            help: "a capture names the parameter it binds to, so it holds an identifier and \
                   nothing else: `{id}`, `{page?}` or `{rest...}`",
        });
    }
    Ok(Some(capture))
}

/// § 4's reading of a declared `path`, for the link half: one
/// [`UrlPiece`] per segment, each carrying its own leading `/`, or `None` for a
/// path § 2's grammar does not admit.
///
/// The **only** reading of a path outside [`parse_path`], and deliberately over
/// [`capture_of`] rather than beside it: `docs/agent/loop-goal.md`
/// § *Standing decisions* makes a fold and its runtime path one implementation,
/// so `Core\Router::url`'s substitution is handed a path already split by § 2's
/// own grammar and never a second parser of it. A row in the table is always
/// `Some` — [`collect_route`] drops a path that refused — so the option exists
/// for the caller that has not been through that gate rather than for a state a
/// table can be in.
pub(crate) fn link_pieces(path: &str) -> Option<Vec<UrlPiece>> {
    let mut pieces = Vec::new();
    // `skip(1)`: a path begins at `/`, so the first split is the empty text
    // before it and every remaining segment owns the `/` that introduced it.
    for segment in path.split('/').skip(1) {
        let piece = match capture_of(segment).ok()? {
            None => UrlPiece::Literal(format!("/{segment}")),
            Some(Capture::One(name)) => UrlPiece::Required(name.to_owned()),
            Some(Capture::Optional(name)) => UrlPiece::Optional(name.to_owned()),
            Some(Capture::Rest(name)) => UrlPiece::Rest(name.to_owned()),
        };
        pieces.push(piece);
    }
    Some(pieces)
}

/// § 3's two questions about the method the attribute is attached to — every
/// capture names a parameter of that name, and that parameter's declared type
/// is one a segment converts to — and § 2's one question of the same kind: a
/// `{name?}` is well-typed only where its parameter has a default.
///
/// The primary span of the two type-shaped refusals is the *parameter*, as it
/// is for [`crate::commands::check_convertible`]: the path is written correctly
/// and it is the declaration that cannot answer it. The unbound capture is the
/// other way round, and is reported at the `path:` field, because that is where
/// the name with no counterpart was written.
fn check_captures(
    captures: &[Capture<'_>],
    path_span: Span,
    m: &MethodMember,
    class: &QName,
    handler: &str,
    env: &mut Env<'_>,
) -> Vec<RouteParam> {
    // Copied out of `env` rather than read through it, so both stay readable
    // while a diagnostic is reported into the same `env`.
    let (src, signatures) = (env.src, env.signatures);
    let method = span_text(src, m.name).to_owned();
    let sig = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(&method));
    let mut params = Vec::new();
    for capture in captures {
        let name = capture.name();
        // Pushed before anything is checked, so the row set describes the
        // *path* rather than the subset of it that type-checked: a capture the
        // declaration does not answer for is a refused program, and a path
        // template with no parameter beside it is not a shape any reader of
        // this row should have to handle.
        params.push(RouteParam {
            name: name.to_owned(),
            source: ParamIn::Path,
            required: !matches!(capture, Capture::Optional(_)),
            ty: None,
            allowed: None,
            parses: false,
            cases: None,
        });
        let Some((index, param)) = m
            .params
            .iter()
            .enumerate()
            .find(|(_, param)| crate::strip_sigil(span_text(src, param.name)) == name)
        else {
            env.diags.report(
                Diagnostic::error(
                    code::E_ROUTE_CAPTURE_UNBOUND,
                    format!(
                        "`{handler}` has no parameter `${name}` for `{}` to arrive as",
                        capture.written()
                    ),
                )
                .with_primary(path_span, "this capture names no parameter")
                .with_help(
                    "a capture is the parameter it is named after and the comparison is exact \
                     (`rule:core-api/identifier-casing`) — the reverse is fine, and a parameter the path does not name \
                     is simply not the router's",
                ),
            );
            continue;
        };
        if matches!(capture, Capture::Optional(_)) && param.default.is_none() {
            env.diags.report(
                Diagnostic::error(
                    code::E_OPTIONAL_CAPTURE_NEEDS_DEFAULT,
                    format!(
                        "`${name}` has no default, so `{}` has nothing to be when it is absent",
                        capture.written()
                    ),
                )
                .with_primary(param.span, "this parameter needs a default")
                .with_secondary(path_span, "the capture that may match no segment")
                .with_help(
                    "an optional capture matches one whole segment or none, and the default is \
                     what makes the absent case well-typed rather than nullable by accident",
                ),
            );
        }
        let Some(ty) = sig.and_then(|sig| sig.params.get(index).copied()) else {
            continue;
        };
        let described = env.interner.describe(ty);
        let allowed = closed_set(ty, env);
        let parses = crate::commands::is_parses_class(ty, env);
        let cases = enum_capture(ty, param.span, env);
        if let Some(row) = params.last_mut() {
            row.ty = Some(described.clone());
            row.allowed = allowed;
            row.parses = parses;
            row.cases = cases;
        }
        let admitted = match capture {
            // The one row the shared roster does not answer for: nothing about
            // a catch-all is checked, so there is no conversion to choose and
            // it arrives as the `tainted string` it was read as.
            Capture::Rest(_) => matches!(
                env.interner.get(ty),
                crate::ty::Ty::String | crate::ty::Ty::TaintedString
            ),
            _ => crate::commands::converts_from_string(ty, env),
        };
        if admitted {
            continue;
        }
        let written = capture.written();
        env.diags.report(
            Diagnostic::error(
                code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION,
                format!("`{described}` is not a type `{written}` can arrive at"),
            )
            .with_primary(param.span, "no conversion from a path segment")
            .with_secondary(path_span, "the capture bound here")
            .with_help(match capture {
                Capture::Rest(_) => {
                    "a `{name...}` is every remaining segment as one value and nothing about it \
                     was checked, so it arrives as a `string` and at no other type"
                }
                _ => {
                    "a segment is converted to the parameter's declared type, so a capture \
                     binds `string`, `int`, `uint`, `decimal`, an enum, a union of literal \
                     types, or a class implementing `Parses` — and never a regex \
                       (`rule:routing/a-capture-narrows-to-a-closed-set`)"
                }
            }),
        );
    }
    params
}

/// `rule:routing/a-capture-narrows-to-a-closed-set`
/// 's closed set: every value `ty` admits, spelled as the path segment or
/// query value that arrives at it — or `None` where `ty` is not a closed set.
///
/// The narrower question than [`crate::commands::converts_from_string`], which
/// asks whether a segment converts *at all*: `string`, `int` and every class
/// implementing `Parses` all convert, and none of them names a set anything
/// could be checked against — a `Parses` class narrows nothing, because the
/// contract says the text either parses or does not and never which texts do.
/// What is left is § 5's other addition:
///
/// - a **union of `string` or `int` literal types**, and a lone literal type,
///   which is the same set written with one member;
/// - an **enum-case subset**, which returns `None` here and is [`enum_capture`]'s
///   instead. Two callers, two answers: a route segment is spelled by
///   `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`,
///   and a command-line word by the case name always
///   (`crate::commands::ArgConv::Enum`), so the shared function is the one that
///   answers neither and the difference stays where each rule is stated.
pub(crate) fn closed_set(ty: crate::ty::TypeId, env: &Env<'_>) -> Option<Vec<String>> {
    let one = |member: crate::ty::TypeId| match env.interner.get(member) {
        crate::ty::Ty::StringLiteral(text) => Some(text.clone()),
        crate::ty::Ty::IntLiteral(value) => Some(value.to_string()),
        _ => None,
    };
    match env.interner.get(ty) {
        crate::ty::Ty::Union(members) => members.iter().map(|member| one(*member)).collect(),
        _ => one(ty).map(|only| vec![only]),
    }
}

/// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
/// 's spelling, for a capture or a `#[Query]` value declared at a whole enum or
/// at a subset of one — or `None` where `ty` is neither.
///
/// The rule in one reading: a subset whose every admitted case **wrote** its
/// value is spelled by those values, and one where any case counted on from the
/// case before is spelled by case name. Both halves are decided over the subset
/// rather than case by case, because a subset mixing them would put an integer
/// and a name in the same position — and [`crate::enums::EnumInfo::written`] is
/// the only record of which is which, since a counted value is an ordinary
/// integer everywhere after the declaration is read.
///
/// A union whose members are cases of two different enums answers `None` and is
/// [`code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION`]'s refusal where a capture is
/// declared at one: there is no enum for the match to convert to, which is a
/// question about the *type* and not about the spelling this decides.
fn enum_capture(ty: crate::ty::TypeId, span: Span, env: &mut Env<'_>) -> Option<EnumCapture> {
    let (class, admitted) = admitted_cases(ty, env)?;
    let info = env.enums.get(&class)?;
    let spell_by_value = admitted.iter().all(|case| info.written.contains(case));
    let mut cases: Vec<(String, crate::enums::EnumValue)> = admitted
        .iter()
        .filter_map(|case| {
            let value = info.cases.get(case)?;
            let spelling = if spell_by_value {
                match value {
                    crate::enums::EnumValue::Int(number) => number.to_string(),
                    crate::enums::EnumValue::Uint(number) => number.to_string(),
                }
            } else {
                case.clone()
            };
            Some((spelling, *value))
        })
        .collect();
    cases.sort_by(|left, right| {
        ordinal(left.1)
            .cmp(&ordinal(right.1))
            .then_with(|| left.0.cmp(&right.0))
    });
    let class = class.to_string();
    for pair in cases.windows(2) {
        if pair[0].0 == pair[1].0 {
            ambiguous_case_value(&class, &admitted, &pair[0].0, span, env);
            return None;
        }
    }
    Some(EnumCapture {
        class,
        cases,
        by_value: spell_by_value,
    })
}

/// Every case `ty` admits, and the enum they are cases of — a whole enum, one
/// case, or a union of cases of one and the same enum. `None` for anything
/// else, and that includes a union whose members are cases of two different
/// enums: there is no single enum for the admitted words to be cases *of*.
///
/// **The one home of that test**, read by [`enum_capture`] for a route segment
/// and by `crate::commands::conversion_of` for a command-line word. The two
/// callers disagree about how an admitted case is *spelled* —
/// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`
/// against `crate::commands::ArgConv::Enum`'s case name always — and agree
/// exactly here, about which cases a type admits at all, which is a question
/// about the type and not about either spelling.
pub(crate) fn admitted_cases(ty: crate::ty::TypeId, env: &Env<'_>) -> Option<(QName, Vec<String>)> {
    match env.interner.get(ty) {
        crate::ty::Ty::Enum(name, _) => {
            let info = env.enums.get(name)?;
            let mut cases: Vec<String> = info.cases.keys().cloned().collect();
            cases.sort();
            Some((name.clone(), cases))
        }
        crate::ty::Ty::EnumCase(name, _, case) => Some((name.clone(), vec![case.clone()])),
        crate::ty::Ty::Union(members) => {
            let mut owner: Option<QName> = None;
            let mut cases = Vec::with_capacity(members.len());
            for member in members {
                let crate::ty::Ty::EnumCase(name, _, case) = env.interner.get(*member) else {
                    return None;
                };
                if *owner.get_or_insert_with(|| name.clone()) != *name {
                    return None;
                }
                cases.push(case.clone());
            }
            Some((owner?, cases))
        }
        _ => None,
    }
}

/// One case's value as a number both backings order the same way — the widening
/// `crate::enums::EnumValue` refuses to carry, made here because a sort key is
/// the one place a `u64` past `i64::MAX` and a negative `i64` have to compare.
fn ordinal(value: crate::enums::EnumValue) -> i128 {
    match value {
        crate::enums::EnumValue::Int(number) => i128::from(number),
        crate::enums::EnumValue::Uint(number) => i128::from(number),
    }
}

/// Two admitted cases carrying one written value, which under that spelling is
/// two cases at one segment.
fn ambiguous_case_value(
    class: &str,
    admitted: &[String],
    spelling: &str,
    span: Span,
    env: &mut Env<'_>,
) {
    let named = admitted
        .iter()
        .filter(|case| {
            env.enums
                .get(&QName::parse(class))
                .and_then(|info| info.cases.get(*case))
                .is_some_and(|value| ordinal(*value).to_string() == spelling)
        })
        .map(|case| format!("`{class}::{case}`"))
        .collect::<Vec<_>>()
        .join(" and ");
    env.diags.report(
        Diagnostic::error(
            code::E_ROUTE_CAPTURE_CASES_SHARE_A_VALUE,
            format!("{named} both carry `{spelling}`, so that segment names two cases"),
        )
        .with_primary(span, "this capture admits two cases at one spelling")
        .with_help(
            "`rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`: a \
             subset whose every case wrote its value is spelled by those values, and an alias \
             leaves the match nothing to convert to — narrow the capture to one of the two, or \
             let a case count its value on so the whole subset is spelled by name",
        ),
    );
}

/// `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]` parameters of the method the attribute is attached
/// to, by the key each binds — which is the parameter's own name, the attribute
/// carrying nothing that could give it another.
///
/// Beside [`check_captures`] rather than inside it because the two read one
/// parameter list to opposite ends: a capture starts from the path and looks
/// for the parameter it names, while a `#[Query]` starts from the declaration
/// and names no part of the path at all. It is the counterpart of
/// [`crate::commands::check_options`]' walk, and § 3's "the same type list as a
/// path capture" is that list read from the one place that holds it.
///
/// **A failed conversion is still a compile error here**, not § 3's `400`: the
/// `400` is what a bad *value* gets at run time, and this is the declaration
/// saying it would have nothing to arrive at whatever the value was.
fn query_params(
    m: &MethodMember,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Vec<RouteParam> {
    // Copied out of `env` for [`check_captures`]' reason exactly.
    let (src, signatures) = (env.src, env.signatures);
    let method = span_text(src, m.name).to_owned();
    let sig = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(&method));
    let mut keys = Vec::new();
    for (index, param) in m.params.iter().enumerate() {
        if crate::testing::attribute_named(&param.attributes, crate::derive::QUERY, ctx, env)
            .is_none()
        {
            continue;
        }
        let name = crate::strip_sigil(span_text(env.src, param.name)).to_owned();
        let declared = sig.and_then(|sig| sig.params.get(index).copied());
        if let Some(ty) = declared
            && !crate::commands::converts_from_string(ty, env)
        {
            let described = env.interner.describe(ty);
            env.diags.report(
                Diagnostic::error(
                    code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION,
                    format!("`{described}` is not a type `#[Query] ${name}` can arrive at"),
                )
                .with_primary(param.span, "no conversion from a query value")
                .with_help(
                    "a query value arrives as text and its type comes from the parameter, so a \
                     `#[Query]` declares the same list a capture does (`rule:routing/a-query-parameter-is-declared-like-a-capture`): `string`, \
                     `int`, `uint`, `decimal`, an enum, a union of literal types, or \
                     a class implementing `Parses`",
                ),
            );
        }
        let key_cases = declared.and_then(|ty| enum_capture(ty, param.span, env));
        keys.push(RouteParam {
            name,
            source: ParamIn::Query,
            // § 3: a default is the declaration saying the key may be absent,
            // and it is the only thing that says so — the marker itself carries
            // no optionality.
            required: param.default.is_none(),
            ty: declared.map(|ty| env.interner.describe(ty)),
            allowed: declared.and_then(|ty| closed_set(ty, env)),
            parses: declared.is_some_and(|ty| crate::commands::is_parses_class(ty, env)),
            cases: key_cases,
        });
    }
    keys
}

/// The field `option` as written, or `None` where the payload has no such
/// field. Says nothing about its value — [`collect_route`] uses it to tell a
/// field left out from one written at the wrong type.
fn written<'a>(
    attr: &'a Attribute,
    option: &str,
    env: &Env<'_>,
) -> Option<&'a nvs_syntax::ast::ObjectLiteralField> {
    attr.fields
        .iter()
        .find(|field| span_text(env.src, field.name) == option)
}

/// One `string` option's written value, folded — `None` where the field was
/// not written or where its value was not a `string`, the second of which
/// [`crate::attributes::check_roster`] has already reported.
fn folded_str(attr: &Attribute, option: &str, env: &mut Env<'_>) -> Option<(String, Span)> {
    let field = written(attr, option, env)?;
    let value = field.value.clone();
    let span = field.span;
    let declared = env.interner.intern(crate::ty::Ty::String);
    match crate::defaults::literal_default(&value, declared, env) {
        Some(crate::defaults::ConstArg::Str(text)) => Some((text, span)),
        _ => None,
    }
}

/// The `method:` case's own name — `Get` for `Core\Http\Method::Get`.
///
/// Read as a case rather than folded to its integer and mapped back, because
/// the case is what the row holds and the enum table answers both questions at
/// once: a value that is not a case of [`METHOD_ENUM`] is `None` here and has
/// already been reported by the roster walk, which places it at that enum.
fn verb_of(attr: &Attribute, ctx: &Ctx<'_>, env: &mut Env<'_>) -> Option<String> {
    let field = written(attr, METHOD, env)?;
    let ExprKind::ClassConstAccess { class, name } = &field.value.kind else {
        return None;
    };
    let class = class.clone();
    let case = span_text(env.src, *name).to_owned();
    let qname = crate::expr::resolve_class_expr(&class, ctx, env)?;
    if qname != QName::parse(METHOD_ENUM) {
        return None;
    }
    env.enums.case(&qname, &case)?;
    Some(case)
}

/// The `allow:` decision as `Class::MEMBER`, with the class resolved — `Role`
/// under a `use App\Role;` is `App\Role::Admin` on the row.
///
/// Resolved rather than folded, because folding is the one thing this compiler
/// must not do here: [`check_access`] admits an enum case and a class constant
/// as one syntactic form precisely so that § 2's promise — the name resolves,
/// and nothing asks what it means — survives to the dispatcher. So the class
/// side goes through the resolution every other `Class::…` in this crate gets
/// and the member side is its own written text, with no lookup of a value
/// behind it and no requirement that the two together name anything declared.
///
/// `None` for a payload [`check_access`] has already refused, and for the
/// dynamic class side [`crate::expr::resolve_class_expr`] answers `None` to:
/// both are declarations with no name to carry, and the row says so rather
/// than carrying a spelling that resolved to nothing.
fn access_name(attr: &Attribute, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<String> {
    let field = written(attr, ALLOW, env)?;
    let ExprKind::ClassConstAccess { class, name } = &field.value.unparenthesized().kind else {
        return None;
    };
    let qname = crate::expr::resolve_class_expr(class, ctx, env)?;
    Some(format!("{qname}::{}", span_text(env.src, *name)))
}

/// The two of § 1-§ 3's compile errors that are questions about the whole
/// enumeration: one route declared twice, and one `name` claimed twice.
///
/// Run once, after every file has been walked, because a route in the entry
/// file collides with one in a file § 5's scan found and neither declaration
/// can see the other. Both are reported at the row that arrives *second* in
/// load order, so which of two colliding declarations is named does not depend
/// on the filesystem.
///
/// The `name` half carries
/// `rule:routing/repeated-routes-share-a-name-when-they-share-a-path`
/// 's exception — repetitions on one method sharing a path share a name —
/// and it is asked here rather than in [`check_class_routes`] because it is the
/// same question the rest of this walk asks: two rows, and whether they are the
/// one endpoint `Core\Router::url` can answer for. The duplicate-*route* rule
/// above is untouched by it, so two attributes sharing both `path` and `method`
/// stay an error however they are grouped.
pub(crate) fn check_table(table: &RouteTable, diags: &mut Diagnostics) {
    let mut routes: FxHashMap<(&str, String), &Route> = FxHashMap::default();
    let mut names: FxHashMap<&str, &Route> = FxHashMap::default();
    for row in &table.rows {
        if let Some(prior) = routes.insert((row.verb.as_str(), shape(&row.path)), row) {
            let (verb, path, handler) = (&row.verb, &row.path, &prior.handler);
            diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_ROUTE,
                    format!("`{verb} {path}` is already served by `{handler}`"),
                )
                .with_primary(row.span, "this route is declared twice")
                .with_help(
                    "§ 2 matches a path by shape, so two captures that differ only in name are \
                     one route — serve the second verb from its own `#[Route]`, or give the two \
                     paths different literal segments",
                ),
            );
            routes.insert((row.verb.as_str(), shape(&row.path)), prior);
        }
        let Some((name, span)) = &row.name else {
            continue;
        };
        if let Some(prior) = names.insert(name.as_str(), row) {
            // `rule:routing/repeated-routes-share-a-name-when-they-share-a-path`'s exception: repetitions on one method may share a
            // name when they share a path, because then `url()` has one answer
            // to give. Both halves are required — the same name on two methods
            // is the copy-paste the rule was written for, and one method whose
            // repetitions carry different paths is the ambiguity itself.
            if prior.handler == row.handler && prior.path == row.path {
                continue;
            }
            let handler = &prior.handler;
            diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_ROUTE_NAME,
                    format!("the route name `{name}` is already `{handler}`'s"),
                )
                .with_primary(*span, "this name is claimed twice")
                .with_help(
                    "a name is what `Core\\Router::url` reverses the table by, so it names one \
                     route or it names nothing",
                ),
            );
            names.insert(name.as_str(), prior);
        }
    }
}

/// § 2's path *shape*: the path with every capture's name erased but its
/// *form* kept, so `/users/{id}` and `/users/{userId}` are the one route they
/// match as while `/posts/{page}` and `/posts/{page?}` stay two.
///
/// The form is kept because § 2's precedence is structural — a literal beats a
/// `{name}`, which beats a `{name?}`, which beats a `{name...}` — so the three
/// are three nodes of the trie and a pair of them has an answer that does not
/// depend on declaration order. Erasing the form would report that pair as the
/// duplicate it is not.
///
/// Only a row [`parse_path`] admitted reaches here, so a segment that is not a
/// capture is a literal and there is no third case to leave alone.
fn shape(path: &str) -> String {
    path.split('/')
        .map(|segment| match capture_of(segment) {
            Ok(Some(Capture::One(_))) => "{}",
            Ok(Some(Capture::Optional(_))) => "{?}",
            Ok(Some(Capture::Rest(_))) => "{...}",
            _ => segment,
        })
        .collect::<Vec<_>>()
        .join("/")
}
