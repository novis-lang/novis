//! `rule:tooling/commands-are-compiled`'s
//! command-table attributes: what `#[Command]` and `#[Option]` may carry.
//!
//! # Why these are recognized names rather than shape aliases
//!
//! § 6 builds a *table* from them while compiling — the same
//! `rule:programs/implementing` scan `rule:routing/routes-are-compiled-not-registered`'s route table is built by — so a userland
//! `type Command = {name: string};` must not contribute a command however it is
//! spelled. That is exactly
//! `rule:core-classes/derive-attribute`'s rule, so both
//! names sit on [`crate::derive::ATTRIBUTES`] and are matched *nominally* after
//! [`nvs_hir::resolve_ref`]. [`crate::attributes`]'s `rule:attributes/attach-sites-and-forms` rule — the
//! name is a shape-typed `type` alias — is the rule for the userland names,
//! which are the only ones that could ever be aliases.
//!
//! This module declares what is checked behind those roster entries: a
//! name on that closed list with nothing checking its payload would admit
//! `#[Command(nmae: "deploy")]` silently, which is the whole argument
//! [`crate::derive`]'s own gap 3 makes for keeping the list short.
//!
//! # What is checked here, and which pass asks it
//!
//! The passes below are separate because they read different things.
//!
//! **One payload at a time**, from [`crate::attributes`]'s per-attribute walk:
//! the field names against [`COMMAND_OPTIONS`] / [`OPTION_OPTIONS`], each
//! value's type, and a field given twice. The walk itself is
//! [`crate::attributes::check_roster`], shared with every other recognized name
//! that carries a payload; what this module owns is the two rosters.
//!
//! **One method at a time**, from [`crate::check`]'s per-class walk:
//! [`check_class_commands`] holds each `#[Command]` to those of § 6's compile
//! errors that are decidable from one parameter list — two options
//! sharing a spelling, and an `#[Option]` on a parameter no argument text could
//! be converted into — and builds that method's [`Command`] rows out of the
//! same walk, because the parameter list it reads for the first question is the
//! argument list a row carries.
//!
//! **The whole program at once**, from the end of [`crate::check::check_program`]:
//! [`check_table`] reports § 6's duplicate command name. It is a
//! question about the enumeration rather than about a declaration — `rule:programs/implementing`'s scan is what brings two files' commands into one program — so it waits
//! for every file, exactly as [`crate::routes::check_table`] does.
//!
//! **Every method, from [`crate::attributes`]' own walk:**
//! [`check_stray_options`], which is the one question the per-class walk cannot
//! ask, because that walk sees only the methods a `#[Command]` selects.
//!
//! # What § 6 leaves open, and what is decided here
//!
//! **`name` is required.** § 6 writes every example with one and says nothing
//! about leaving it out, and `rule:routing/route-attribute`'s "optional and never derived" is a
//! rule about *routes*, where a name is only a reverse lookup. Here it is the
//! word a command line selects the command *by*, so a row without one is
//! reachable by nothing: [`code::E_COMMAND_WITHOUT_NAME`], reported from the
//! pass that needs the name to build the row.
//!
//! **Every `#[Command]` on a method becomes a row**, which is `rule:attributes/repeatable`'s
//! repetition read as [`crate::routes::check_class_routes`] reads it — two
//! attributes on one method are two names for one implementation, which is what
//! an alias is. Nothing else would be safe: the alternative is taking the first
//! and ignoring the second, and a recognized attribute the compiler never looks
//! at is what this module's roster exists to prevent.
//!
//! # What crosses in a row, and what stays
//!
//! A row carries the *conversion* a parameter needs and not its type. § 6's
//! matching reads the declared type, and the consumer of the table is a native
//! member holding no signature at all — `Core\Command::run` is reached from a
//! command line, not from a call site — so something about the type has to
//! cross. What crosses is [`ArgConv`], the answer [`converts_from_string`]
//! already computes for § 6's third compile error, and not the type: a closed
//! set, decided in the same walk that builds the row, cannot disagree with the
//! declaration the way a second copy of the type lattice could. The one other
//! thing a matcher cannot do without crosses the same way: a parameter's
//! *default* rides as the text a command line would have written for it
//! ([`CommandArg::default`]), not as the constant. The rest of the signature
//! stays where it is.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Attribute, ClassDecl, ClassMemberKind, MethodMember, Param};
use rustc_hash::FxHashMap;

use crate::defaults::ConstArg;
use crate::testing::OptionTy;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// `#[Command(name: string, about: string)]` — `rule:tooling/commands-are-compiled`'s own spelling, in
/// the order that section writes it.
pub(crate) const COMMAND_OPTIONS: &[(&str, OptionTy)] =
    &[("name", OptionTy::Str), ("about", OptionTy::Str)];

/// `#[Option(short: string, long: string, about: string)]` — § 6 writes
/// `short` and `about`; `long` is the spelling an option is matched by when the
/// parameter's own name is not it, and it is named here for the same reason
/// `short` is: nothing about a command line is inferred.
pub(crate) const OPTION_OPTIONS: &[(&str, OptionTy)] = &[
    ("short", OptionTy::Str),
    ("long", OptionTy::Str),
    ("about", OptionTy::Str),
];

/// One parameter of a `#[Command]` method, in the order it was declared —
/// § 6's "a parameter is a positional argument unless it carries `#[Option]`",
/// resolved to the spellings a command line writes.
#[derive(Clone, Debug)]
pub struct CommandArg {
    /// The parameter's own name, sigil-less. What a positional argument is
    /// named by in usage text, and what an option's long spelling defaults to.
    pub param: String,
    /// Every spelling this argument answers to, rendered with the `-`/`--` a
    /// command line writes them with.
    ///
    /// **Empty exactly for a positional argument**, and never empty for an
    /// option: § 6 gives an option that writes no `long:` its parameter's own
    /// name. Which of the two an argument is, is therefore read off this rather
    /// than carried a second time beside it — two fields that must agree are
    /// two fields that can disagree.
    pub spellings: Vec<String>,
    /// The `#[Option]`'s `about:`, for the usage text § 6 generates. `None` on
    /// a positional argument, which carries no attribute to write one on.
    pub about: Option<String>,
    /// What this argument's text becomes before the handler is called — see
    /// [`ArgConv`], and the module's gap 1 for why this and not the type.
    pub conv: ArgConv,
    /// The parameter's declared default, as the **text** a command line would
    /// have written to supply it — `Some("3")` for § 6's own
    /// `#[Option] uint $retries = 3`, and `None` for a parameter a command line
    /// must fill itself.
    ///
    /// The text rather than the constant, for gap 1's reason one field along:
    /// the matcher already turns a word into the parameter's value through
    /// [`ArgConv`], so a defaulted argument reaching the handler through that
    /// same conversion *is* the value a written one would have been, and
    /// crossing [`crate::defaults::ConstArg`] instead would put a second copy of
    /// the constant lattice in `nvs-runtime` to say what one `String` says here.
    ///
    /// **What a default may be is [`crate::defaults`]' rule, not this module's**,
    /// and that is why there is no non-literal case to decide about: a parameter
    /// default is already a literal of its own declared type or it is
    /// `E_PARAM_DEFAULT_NOT_LITERAL` wherever it is written. So this is read back
    /// off the signature the same walk already holds
    /// ([`crate::signatures::MethodSig::defaults`]) and never folded a second
    /// time — a fold here could disagree with the one the call sites use.
    pub default: Option<String>,
}

/// § 6's "a matched value's text is converted during matching, and its type
/// comes from the parameter", as the one closed set a matcher needs.
///
/// Decided by [`conversion_of`] in the walk that builds the row, off the same
/// resolved signature [`converts_from_string`] holds an `#[Option]` to, so the
/// two answers are one answer: every type that section admits has a variant
/// here, and everything else is refused where it is written.
///
/// `nvs_runtime::commands::ArgConv` is this set as a running program holds it.
/// Not `Copy` since [`Self::OneOf`] carries its set: a conversion is read once
/// per argument of a command line, and the set is the answer itself rather than
/// a lookup key, so there is nothing for a copy to save.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArgConv {
    /// `string` — the argument's own text, which arrives `tainted` (§ 6).
    Text,
    /// `bool` — the flag § 6 gives an `#[Option]`, written rather than given.
    Flag,
    /// `int`.
    Int,
    /// `uint`.
    Uint,
    /// `decimal` — `rule:types/decimal`'s exact number.
    Decimal,
    /// A class implementing `Parses`, named as § 6's usage line writes it: the
    /// text is what its `parse` reads, and `rule:expressions/try-parse` is the
    /// contract that makes the pair a conversion rather than a convention.
    ///
    /// **The name crosses rather than a key**, for [`Self::Enum`]'s reason two
    /// arms down: the compiler resolved the class once, and nothing downstream
    /// has a type table to ask instead. `Core\Uuid` is the one implementor the
    /// library carries, and it is on this arm by the same predicate a user
    /// class reaches it by rather than by being named.
    Parses(String),
    /// § 3's union of literal types: the word each member admits, in the order
    /// the union declares them, and a usage error for anything else.
    ///
    /// The same set `nvs_runtime::routes::CaptureConv::OneOf` narrows a segment to,
    /// computed by the same [`crate::routes::closed_set`] — a word a command
    /// line supplies and a segment a route matches are admitted by one grammar,
    /// which is the arrangement `decimal` and `Core\Uuid` already have.
    OneOf(Vec<String>),
    /// An enum: every case as the word a command line writes it with, the value
    /// that word becomes, and the enum's own name beside them.
    ///
    /// **The word is the case name.** That is the decision
    /// [`crate::routes::closed_set`] answers for neither caller, and
    /// `crate::routes::enum_capture` takes the other way for a route segment:
    /// one written by a link and read by `rule:routing/a-capture-narrows-to-a-closed-set`
    /// is spelled by a written backing value where its subset has one. Here `rule:enums/no-class-machinery`'s
    /// backing value is the alternative and it loses on § 6's own argument — the
    /// refusal below names every value that would have been accepted *because a
    /// command line is a person typing*, and a list of integers is not that
    /// sentence. A backing value is storage: it is chosen for a table or a wire
    /// format, an author may renumber it without touching a name, and nobody
    /// typing `--level 2` has anything to check it against.
    ///
    /// **The class is on the row and is not decoration.** § 6's usage line and
    /// this conversion's refusal both say what the words are cases *of*, which
    /// no set of bare words can answer — and it is the half [`Self::OneOf`]
    /// deliberately has not got, because a union of literals is not named.
    ///
    /// **The value is the case's own backing integer, unwidened.** A case is
    /// indistinguishable from that integer by the time it is a value
    /// (`nvs_runtime::object::EnumCases`), so nothing is constructed and no
    /// class descriptor is reached for at run time: the compiler resolved the
    /// enum once, exactly as [`Self::OneOf`] resolved its union once, and the
    /// answer crosses rather than a key to look it up with.
    Enum {
        /// The enum's declared name, written as § 6's usage line writes it.
        class: String,
        /// Every case: the word, and the value it becomes. **Ascending by
        /// value**, because [`crate::enums::EnumInfo::cases`] is a map with no
        /// declaration order to take — `crate::derive`'s own `enum_cases` sorts
        /// the same roster the same way, and a set the compiler renders into a
        /// message has to read the same on two builds.
        cases: Vec<(String, crate::enums::EnumValue)>,
    },
    /// A type § 6 admits whose conversion is not written yet —
    /// `nvs_runtime::commands`'s own gap 1, which is where it is refused.
    Unconverted,
}

/// The conversion `ty` needs, for the row [`check_options`] is building.
///
/// Deliberately total rather than fallible: a type with no conversion at all is
/// [`check_convertible`]'s diagnostic, and answering [`ArgConv::Unconverted`]
/// here for one that has one is what keeps this walk's two questions
/// independent.
fn conversion_of(ty: TypeId, env: &Env<'_>) -> ArgConv {
    match env.interner.get(ty) {
        // A parameter written with no type at all interns as `mixed` and has
        // already been reported for it; its text is the honest answer.
        Ty::String | Ty::TaintedString | Ty::StringLiteral(_) | Ty::Mixed => ArgConv::Text,
        Ty::Bool | Ty::True | Ty::False => ArgConv::Flag,
        Ty::Int | Ty::IntLiteral(_) => ArgConv::Int,
        Ty::Uint => ArgConv::Uint,
        Ty::Decimal => ArgConv::Decimal,
        // Matched structurally, by the same predicate [`converts_from_string`]
        // admits a class with — two readings of one contract, so they cannot
        // come to disagree about which classes § 6 means.
        Ty::Class(name, _) if reaches_parses(name, env) => ArgConv::Parses(name.to_string()),
        // § 3's union, narrowed to the words its members admit by the same
        // computation the route table's captures use. `None` is a union of
        // *enum cases*, which that function answers for neither caller because
        // the two spell one differently — the arm below is this caller's answer,
        // and `crate::routes::enum_capture` is the route's.
        Ty::Union(_) => {
            crate::routes::closed_set(ty, env).map_or(ArgConv::Unconverted, ArgConv::OneOf)
        }
        // The other closed set, and the one that function answers `None` for: an
        // enum *names* its members, so the words come off the declaration rather
        // than off the type. [`ArgConv::Enum`] owns both decisions that took —
        // which word, and what it becomes.
        Ty::Enum(name, _) => cases_of(name, env),
        _ => ArgConv::Unconverted,
    }
}

/// `name`'s declared cases, as the row [`ArgConv::Enum`] carries them.
///
/// [`ArgConv::Unconverted`] for a name this program declares no enum for, which
/// is a resolution failure already reported where the parameter was written —
/// the same reading `crate::derive`'s `enum_cases` gives the same absence, and
/// for the same reason: naming it twice charges one mistake twice.
fn cases_of(name: &QName, env: &Env<'_>) -> ArgConv {
    let Some(info) = env.enums.get(name) else {
        return ArgConv::Unconverted;
    };
    let mut cases: Vec<(String, crate::enums::EnumValue)> = info
        .cases
        .iter()
        .map(|(case, value)| (case.clone(), *value))
        .collect();
    cases.sort_unstable_by_key(|(_, value)| match value {
        crate::enums::EnumValue::Int(number) => i128::from(*number),
        crate::enums::EnumValue::Uint(number) => i128::from(*number),
    });
    ArgConv::Enum {
        class: name.to_string(),
        cases,
    }
}

/// One row of `rule:tooling/commands-are-compiled`'s table: a `#[Command]` that named the one field a
/// row cannot exist without, with the parameter list a command line fills.
///
/// Public for [`crate::routes::Route`]'s reason — the finished row is what
/// crosses into `nvs-ir`, and what rides across is a decision with no
/// resolution left in it, so the consumer holds strings rather than a second
/// copy of this crate's tables.
#[derive(Debug)]
pub struct Command {
    /// § 6's `name:` — the word a command line selects this command by, and
    /// the key [`CommandTable::named`] answers.
    pub name: String,
    /// The span that wrote [`Self::name`], because a duplicate is reported at
    /// the field rather than at the attribute — [`crate::routes::Route::name`]
    /// carries its span for the same reason.
    pub name_span: Span,
    /// § 6's `about:`, the line `Core\Command::help` renders beside the name.
    /// `None` where the author wrote none, and nothing is derived from the
    /// method's own name: a generated sentence reads exactly like a written one.
    pub about: Option<String>,
    /// `Class::method` the attribute is attached to, rendered as
    /// [`crate::expr_table::ExprTypeTable::method_label`] renders one.
    pub handler: String,
    /// Every parameter in declaration order — positionals and options in one
    /// list, because that order *is* the positional order and splitting them
    /// would leave every consumer rebuilding it.
    pub args: Vec<CommandArg>,
    /// The whole attribute.
    pub span: Span,
}

/// Every command the program declares, in the order they were walked — file by
/// file in `nvs_hir::resolve_program`'s entry-first load order, and by
/// declaration within a file.
///
/// A `Vec` rather than a map keyed by the name, for
/// [`crate::routes::RouteTable`]'s reason: the name has a duplicate error
/// attached to it, and a map would drop the row that error is reported
/// against. Order is what makes the pair deterministic — the collision is
/// always reported at the row that arrives second, and the load order does not
/// depend on filesystem enumeration (`rule:programs/implementing`).
#[derive(Debug, Default)]
pub struct CommandTable {
    rows: Vec<Command>,
}

impl CommandTable {
    /// Every row, in load order.
    #[must_use]
    pub fn rows(&self) -> &[Command] {
        &self.rows
    }

    /// The row a command line's first word names, or `None` where the program
    /// declares no such command — which is the one lookup § 6's dispatch makes.
    ///
    /// The first match, which is the only one that can be reached: two rows
    /// claiming one name is [`code::E_DUPLICATE_COMMAND`], so a program in
    /// which this could be ambiguous does not compile.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Command> {
        self.rows.iter().find(|row| row.name == name)
    }
}

/// Every `#[Command]` method `decl` declares, held to those of § 6's compile
/// errors that one parameter list answers, and collected into `env`'s table.
///
/// A no-op — not even a lookup — for a class carrying no `#[Command]`, which is
/// what keeps § 6's "a program with no `#[Command]` pays nothing" true of this
/// pass as well as of the table.
///
/// Run from [`crate::check`]'s per-class walk rather than from
/// [`crate::attributes`]', because every question here is about the *method*:
/// the per-attribute walk holds one payload and can see neither the sibling
/// option it collides with, nor the parameter it is attached to, nor the
/// parameter list a row carries.
pub(crate) fn check_class_commands(
    decl: &ClassDecl,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let commands: Vec<&Attribute> = m
            .attributes
            .iter()
            .flat_map(|group| &group.attributes)
            .filter(|attr| crate::derive::attribute_is(attr, crate::derive::COMMAND, ctx, env))
            .collect();
        if commands.is_empty() {
            continue;
        }
        // Once for the method, however many names it answers to: the options
        // and the declaration's shape are both facts about the method, so
        // asking either per attribute would report one mistake once per alias.
        check_command_shape(m, class, env);
        let args = check_options(m, class, ctx, env);
        let handler = format!("{class}::{}", span_text(env.src, m.name));
        for attr in commands {
            collect_command(attr, &handler, &args, env);
        }
    }
}

/// `rule:tooling/commands-are-compiled`'s facts about the *declaration* a `#[Command]` sits on: it
/// is `static`, and it returns `void` or `uint`.
///
/// The return half is § 6 in prose — `Core\Command::run(): uint` is the entry
/// point and a handler answers with the process status or with nothing, which
/// is status 0. `static` is the half § 6's example writes and its dispatch
/// requires: this table's row carries a `Class::method` string and no
/// constructor arguments anywhere in it, so a handler has no instance to be
/// called on, and § 6's one deliberate divergence from `rule:routing/routes-are-compiled-not-registered` is that this
/// table *dispatches* rather than stopping at the match.
///
/// Read off the resolved signature rather than off `m`'s modifier list, and
/// silent for a method the signature table has no row for, both for
/// [`crate::testing`]'s `check_method_shape`'s reasons: an omitted visibility
/// keyword is already its own diagnostic, and a method missing from the table
/// is a name `nvs_syntax` is already refusing. `m` supplies the span, since a
/// `Modifier` records none of its own and neither refusal is about the body.
///
/// A method that writes **no** return type is left alone here — `rule:types/grammar`
/// requires one everywhere and its absence is already reported, so naming it
/// again as a wrong one would tell the author about a `mixed` they never wrote.
fn check_command_shape(m: &MethodMember, class: &QName, env: &mut Env<'_>) {
    let method = span_text(env.src, m.name).to_owned();
    let Some(sig) = env
        .signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(&method))
    else {
        return;
    };
    let (is_static, return_ty) = (sig.is_static, sig.return_ty);
    if !is_static {
        report_command_shape(
            m,
            &method,
            "is not `static`",
            "a command is dispatched by name off the compiled table, which holds no instance to \
             call one on, so declare it `static`",
            env,
        );
    }
    if m.return_type.is_some() && !matches!(env.interner.get(return_ty), Ty::Void | Ty::Uint) {
        let returned = env.interner.describe(return_ty);
        report_command_shape(
            m,
            &method,
            &format!("returns `{returned}`"),
            "a command answers with the process exit status `Core\\Command::run` returns, so \
             declare `: uint` — or `: void` for one that always succeeds",
            env,
        );
    }
}

/// One [`code::E_COMMAND_METHOD_SHAPE`], worded from what the declaration did —
/// [`crate::testing`]'s `report_shape` arrangement, for its reason.
fn report_command_shape(m: &MethodMember, method: &str, did: &str, help: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_COMMAND_METHOD_SHAPE,
            format!("the `#[Command]` method `{method}` {did}"),
        )
        .with_primary(m.name, did)
        .with_help(help.to_owned()),
    );
}

/// One `#[Command]` payload as a row, or the refusal that it is not one.
///
/// The module docs' *what is decided here* owns why `name` is required; this is
/// where that is enforced rather than in [`crate::attributes::check_roster`],
/// because the roster says what a field may *hold* and *required* is a fact
/// about the row being built — [`crate::routes::collect_route`]'s arrangement
/// exactly.
fn collect_command(attr: &Attribute, handler: &str, args: &[CommandArg], env: &mut Env<'_>) {
    let Some((name, name_span)) = folded_option(attr, "name", env) else {
        // A `name` written at the wrong type has already been reported by the
        // roster walk, and reporting it again as a missing one would name the
        // author's second problem before their first.
        if written(attr, "name", env).is_none() {
            env.diags.report(
                Diagnostic::error(
                    code::E_COMMAND_WITHOUT_NAME,
                    format!("this `#[Command]` on `{handler}` names no command"),
                )
                .with_primary(attr.span, "not enough to build a command")
                .with_help(
                    "a command line selects a command by the word it is named with, so `name` is \
                     the one field a command cannot leave out — write \
                     `#[Command(name: \"deploy\")]`",
                ),
            );
        }
        return;
    };
    let about = folded_option(attr, "about", env).map(|(about, _)| about);
    env.commands.rows.push(Command {
        name,
        name_span,
        about,
        handler: handler.to_owned(),
        args: args.to_vec(),
        span: attr.span,
    });
}

/// § 6's compile error over the finished table: two commands claiming one
/// name.
///
/// Run once at the end of [`crate::check::check_program`] rather than as each
/// class is walked, because `rule:programs/implementing`'s scan is what puts two files'
/// commands in the same program — the same reason
/// [`crate::routes::check_table`] waits, and the reason the table accumulates
/// across the files rather than per file.
pub(crate) fn check_table(table: &CommandTable, diags: &mut Diagnostics) {
    let mut names: FxHashMap<&str, &Command> = FxHashMap::default();
    for row in &table.rows {
        let Some(prior) = names.insert(row.name.as_str(), row) else {
            continue;
        };
        let (name, handler) = (&row.name, &prior.handler);
        diags.report(
            Diagnostic::error(
                code::E_DUPLICATE_COMMAND,
                format!("`{name}` is already the command `{handler}` runs"),
            )
            .with_primary(row.name_span, "this command name is claimed twice")
            .with_secondary(prior.name_span, "claimed here")
            .with_help(
                "a command line names one command and expects one answer, so two methods \
                 answering to a single name have none — give this one its own `name:`",
            ),
        );
    }
}

/// `rule:tooling/commands-are-compiled`'s marker held to the declaration that reads it: an `#[Option]`
/// on a parameter of a method carrying no `#[Command]`.
///
/// Asked from [`crate::attributes`]' per-method walk rather than from
/// [`check_class_commands`], because that walk cannot see this mistake at all:
/// it selects the methods a `#[Command]` marks, and a stray `#[Option]` is by
/// definition on one of the others. This does not wait for the table gap 1 still
/// owes — what a stray marker is stray of is the sibling attribute on its own
/// method, which one declaration answers — and it is
/// [`crate::routes::check_stray_query`]'s question asked of the other pass's
/// marker, the two written apart so each sits beside the attribute that gives
/// its marker a meaning.
///
/// Refused rather than ignored for the reason [`crate::derive::ATTRIBUTES`] is a
/// closed roster: a name the compiler knows, written where the compiler never
/// looks, reads to its author as a declaration that binds an argument.
pub(crate) fn check_stray_options(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    if crate::testing::attribute_named(&m.attributes, crate::derive::COMMAND, ctx, env).is_some() {
        return;
    }
    for param in &m.params {
        let Some(attr) =
            crate::testing::attribute_named(&param.attributes, crate::derive::OPTION, ctx, env)
        else {
            continue;
        };
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_OPTION_WITHOUT_COMMAND,
                format!("`#[Option] ${name}` is on a method that declares no command"),
            )
            .with_primary(attr.span, "nothing reads this marker")
            .with_help(
                "`rule:tooling/commands-are-compiled` gives `#[Option]` its meaning on a `#[Command]` method's parameter, \
                 where it is the spelling an argument arrives by — anywhere else nothing supplies \
                 it: write the `#[Command]` this parameter serves, or delete the marker",
            ),
        );
    }
}

/// One `#[Command]` method's parameter list: each `#[Option]`'s declared type,
/// and the spellings they claim between them — handed back as the rows a
/// [`Command`] carries, because the walk that answers the first question is the
/// walk that resolves them.
fn check_options(
    m: &MethodMember,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Vec<CommandArg> {
    // Copied out of `env` rather than read through it, so the table stays
    // readable while a diagnostic is being reported into the same `env`.
    let signatures = env.signatures;
    let method = span_text(env.src, m.name).to_owned();
    let sig = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(&method));
    // The spellings claimed so far, each with the span that claimed it. One
    // list rather than two, because `-n` and `--n` are rendered with the
    // dashes a command line writes and so cannot collide across forms.
    let mut taken: Vec<(String, Span)> = Vec::new();
    let mut args: Vec<CommandArg> = Vec::with_capacity(m.params.len());
    for (index, param) in m.params.iter().enumerate() {
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        // Read for every parameter and not only for an `#[Option]`: a positional
        // argument's text is converted by the same rule, and the row is what
        // carries the answer past this crate (the module's gap 1).
        let declared = sig.and_then(|sig| sig.params.get(index).copied());
        let conv = declared.map_or(ArgConv::Text, |ty| conversion_of(ty, env));
        // Read for every parameter too, and for the same reason: § 6 infers
        // nothing from a default, so a positional carries one exactly as an
        // option does. Already folded and already diagnosed by
        // `crate::defaults` — this walk reads the answer rather than the
        // expression.
        let default = sig
            .and_then(|sig| sig.defaults.get(index))
            .and_then(Option::as_ref)
            .and_then(default_text);
        let Some(attr) =
            crate::testing::attribute_named(&param.attributes, crate::derive::OPTION, ctx, env)
        else {
            // § 6's positional argument: no attribute, no spelling, and its
            // place in this list is the place a command line fills it from.
            args.push(CommandArg {
                param: name,
                spellings: Vec::new(),
                about: None,
                conv,
                default,
            });
            continue;
        };
        let attr = attr.clone();
        if let Some(ty) = declared {
            check_convertible(param, &method, ty, env);
        }
        let about = folded_option(&attr, "about", env).map(|(about, _)| about);
        let mut claimed: Vec<String> = Vec::with_capacity(2);
        for (spelling, span) in spellings(&attr, param, env) {
            claimed.push(spelling.clone());
            if let Some((_, prior)) = taken.iter().find(|(already, _)| *already == spelling) {
                let prior = *prior;
                env.diags.report(
                    Diagnostic::error(
                        code::E_OPTION_SPELLING_TAKEN,
                        format!("two options of `{method}` are spelled `{spelling}`"),
                    )
                    .with_primary(span, "this spelling is already claimed")
                    .with_secondary(prior, "claimed here")
                    .with_help(
                        "a command line matches an option by its spelling, so two parameters \
                         answering to one of them have no answer: give this one its own `short:` \
                         or `long:`",
                    ),
                );
            } else {
                taken.push((spelling, span));
            }
        }
        args.push(CommandArg {
            param: name,
            spellings: claimed,
            about,
            conv,
            default,
        });
    }
    args
}

/// One folded parameter default as the text a command line would have written
/// for it — the form [`CommandArg::default`] carries, and the form
/// `Core\Command::run`'s matcher converts through [`ArgConv`].
///
/// `None` for a constant no command line could have spelled at all — a
/// `float`, a `null`, and the shapes only [`crate::core_lib`] produces — so an
/// argument this answers `None` for stays required, which is what an argument
/// with no spellable default has to be.
///
/// [`ArgConv::Decimal`] and [`ArgConv::Parses`] convert a *written* argument
/// and still have no constant to answer with here, which is
/// [`crate::defaults`]' own known gap rather than this function's: `decimal
/// $vat = 0.19` is refused as a non-literal default before it ever folds, and
/// a class has no literal in any spelling — an object comes from a call, and
/// `parse` is a call. So an argument at either type stays required, and
/// nothing here has to decide what a defaulted one would have meant.
fn default_text(constant: &ConstArg) -> Option<String> {
    match constant {
        ConstArg::Bool(value) => Some(value.to_string()),
        ConstArg::Int(value) => Some(value.to_string()),
        ConstArg::Uint(value) => Some(value.to_string()),
        ConstArg::Str(value) => Some(value.clone()),
        _ => None,
    }
}

/// The spellings one `#[Option]` claims: its `short:` if it wrote one, and its
/// long form, which is the parameter's own name unless `long:` gives another.
///
/// Rendered with the `-`/`--` a command line writes them with, which is what
/// makes one list of both forms correct: `-n` and `--n` are two spellings, and
/// a bare `n` compared against a bare `n` would call them one.
fn spellings(attr: &Attribute, param: &Param, env: &mut Env<'_>) -> Vec<(String, Span)> {
    let mut claimed = Vec::with_capacity(2);
    if let Some((short, span)) = folded_option(attr, "short", env) {
        claimed.push((format!("-{short}"), span));
    }
    match folded_option(attr, "long", env) {
        Some((long, span)) => claimed.push((format!("--{long}"), span)),
        // The parameter's own name is the default long spelling, so an option
        // that writes nothing still claims one — the collision this catches is
        // an explicit `long:` written over a sibling's parameter name, which
        // reading either declaration alone would never show.
        None => claimed.push((
            format!("--{}", strip_sigil(span_text(env.src, param.name))),
            param.name,
        )),
    }
    claimed
}

/// Whether the payload wrote `option` at all, whatever it wrote there — which
/// is what tells a field written at the wrong type apart from one left out, the
/// two having the same folded answer and different first problems.
fn written<'a>(
    attr: &'a Attribute,
    option: &str,
    env: &Env<'_>,
) -> Option<&'a nvs_syntax::ast::ObjectLiteralField> {
    attr.fields
        .iter()
        .find(|field| span_text(env.src, field.name) == option)
}

/// One option's written value, folded — `None` where the field was not written
/// or where its value was not the `string` the roster declares, both of which
/// [`crate::attributes::check_roster`] has already reported.
fn folded_option(attr: &Attribute, option: &str, env: &mut Env<'_>) -> Option<(String, Span)> {
    let field = attr
        .fields
        .iter()
        .find(|field| span_text(env.src, field.name) == option)?;
    let declared = env.interner.intern(Ty::String);
    let span = field.span;
    match crate::defaults::literal_default(&field.value, declared, env) {
        Some(ConstArg::Str(value)) => Some((value, span)),
        _ => None,
    }
}

/// § 6's compile error about a parameter's type: the parameter an `#[Option]`
/// is attached to must have a type an argument's text can be converted to.
fn check_convertible(param: &Param, method: &str, ty: TypeId, env: &mut Env<'_>) {
    if converts_from_string(ty, env) {
        return;
    }
    let described = env.interner.describe(ty);
    env.diags.report(
        Diagnostic::error(
            code::E_OPTION_TYPE_HAS_NO_CONVERSION,
            format!("`{described}` is not a type an option of `{method}` can be given at"),
        )
        .with_primary(param.span, "no conversion from an argument's text")
        .with_help(
            "an argument arrives as text and its type comes from the parameter, so an option \
             declares `string`, `int`, `uint`, `decimal`, `bool`, an enum, a union of literal \
             types, or a class implementing `Parses`",
        ),
    );
}

/// `rule:security/route-capture-is-laundered-by-its-type`'s conversion roster, which § 6 takes unchanged, plus the one
/// row § 6 adds: a `bool` `#[Option]` is a flag, so it is given by being
/// written rather than by carrying text.
///
/// The one home for "what an argument's text may become", so the route table's
/// own pass reads this rather than growing a second list that agrees with it
/// today.
///
/// A `float` is deliberately absent where `decimal` is not: § 3 names one and
/// not the other, and an argument that quietly rounds is the class of bug
/// `rule:types/decimal` exists to make unwritable.
pub(crate) fn converts_from_string(ty: TypeId, env: &Env<'_>) -> bool {
    match env.interner.get(ty) {
        Ty::String
        | Ty::TaintedString
        | Ty::Int
        | Ty::Uint
        | Ty::Decimal
        | Ty::Bool
        | Ty::True
        | Ty::False
        | Ty::StringLiteral(_)
        | Ty::IntLiteral(_)
        | Ty::Enum(..)
        | Ty::EnumCase(..) => true,
        // A parameter written with no type at all interns as `mixed` and has
        // already been reported for the omission; naming it again here would
        // charge one mistake twice.
        Ty::Mixed => true,
        // A class is admitted by the contract it carries, never by its name:
        // `rule:expressions/try-parse`'s pair, read as `Parses` off the same
        // signature table `crate::expr::operators` asks `Comparable` of. So
        // `Core\Uuid` is admitted for what it declares, and so is any class
        // that declares the same thing.
        Ty::Class(name, _) => reaches_parses(name, env),
        // § 3 admits a union of `string` or `int` literal types and a subset of
        // an enum's cases, and nothing wider: a `string|int` would make the
        // conversion itself ambiguous, which is the question `rule:errors/ambiguous-input-refused` refuses
        // to answer by guessing.
        Ty::Union(members) => members.iter().all(|member| {
            matches!(
                env.interner.get(*member),
                Ty::StringLiteral(_) | Ty::IntLiteral(_) | Ty::EnumCase(..)
            )
        }),
        _ => false,
    }
}

/// [`converts_from_string`]'s class arm, asked on its own: whether a segment or
/// an argument's text reaches `ty` through that class's own `parse`.
///
/// The route table's rows carry the answer because nothing downstream can
/// derive it — [`crate::TypeInterner::describe`] renders a class and an enum
/// identically, as the qualified name and nothing else, so a reader holding the
/// rendering alone cannot tell a class built from text from an enum whose case
/// spellings are `Core\Router::match`'s to decide.
pub(crate) fn is_parses_class(ty: TypeId, env: &Env<'_>) -> bool {
    matches!(env.interner.get(ty), Ty::Class(name, _) if reaches_parses(name, env))
}

/// Whether `qname` implements `Parses` — **two tables, because a `Core` class
/// declares nothing**, which is the arrangement
/// `crate::expr::operators`'s own `Comparable` question is already read
/// through. [`nvs_hir::implements_interface`] answers for a written
/// `implements Parses` and for the reflexive case;
/// [`crate::signatures::resolve_interface_args`] answers for a class whose
/// conformance was *seeded* rather than written, which is `Core\Uuid` — a
/// `Core` class has no [`nvs_hir::ClassGraph`] entry at all, so the first
/// table cannot see it, and `crate::core_lib` writes the edge off
/// `nvs_stdlib::registry::implements_parses`.
fn reaches_parses(qname: &QName, env: &Env<'_>) -> bool {
    let parses = QName::parse(nvs_hir::interfaces::PARSES);
    nvs_hir::implements_interface(qname, &parses, env.graph)
        || crate::signatures::resolve_interface_args(qname, &parses, env.signatures, env.graph)
            .is_some()
}
