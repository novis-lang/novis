//! [ADR 0086](../../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
//! two command-table attributes: what `#[Command]` and `#[Option]` may carry.
//!
//! # Why these are recognized names rather than shape aliases
//!
//! § 6 builds a *table* from them while compiling — the same
//! [ADR 0061](../../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
//! § 3 scan ADR 0077's route table is built by — so a userland
//! `type Command = {name: string};` must not contribute a command however it is
//! spelled. That is exactly
//! [ADR 0071](../../../../docs/adr/0071-derived-codecs.md) § 1's rule, so both
//! names sit on [`crate::derive::ATTRIBUTES`] and are matched *nominally* after
//! [`nvs_hir::resolve_ref`]. [`crate::attributes`]'s ADR 0046 § 1 rule — the
//! name is a shape-typed `type` alias — is the rule for the userland names,
//! which are the only ones that could ever be aliases.
//!
//! This module declares what is checked behind those two roster entries: a
//! name on that closed list with nothing checking its payload would admit
//! `#[Command(nmae: "deploy")]` silently, which is the whole argument
//! [`crate::derive`]'s own gap 3 makes for keeping the list short.
//!
//! # What is checked here, and what the table still owes
//!
//! Two passes, and they are two because they read different things.
//!
//! **One payload at a time**, from [`crate::attributes`]'s per-attribute walk:
//! the field names against [`COMMAND_OPTIONS`] / [`OPTION_OPTIONS`], each
//! value's type, and a field given twice. The walk itself is
//! [`crate::attributes::check_roster`], shared with every other recognized name
//! that carries a payload; what this module owns is the two rosters.
//!
//! **One method at a time**, from [`crate::check`]'s per-class walk:
//! [`check_class_commands`] holds each `#[Command]` to the two of § 6's three
//! compile errors that are decidable from one parameter list — two options
//! sharing a spelling, and an `#[Option]` on a parameter no argument text could
//! be converted into — and builds that method's [`Command`] rows out of the
//! same walk, because the parameter list it reads for the first question is the
//! argument list a row carries.
//!
//! **The whole program at once**, from the end of [`crate::check::check_program`]:
//! [`check_table`] reports § 6's third error, a duplicate command name. It is a
//! question about the enumeration rather than about a declaration — ADR 0061
//! § 3's scan is what brings two files' commands into one program — so it waits
//! for every file, exactly as [`crate::routes::check_table`] does.
//!
//! **Every method, from [`crate::attributes`]' own walk:**
//! [`check_stray_options`], which is the one question the per-class walk cannot
//! ask, because that walk sees only the methods a `#[Command]` selects.
//!
//! # What § 6 leaves open, and what is decided here
//!
//! **`name` is required.** § 6 writes every example with one and says nothing
//! about leaving it out, and ADR 0077 § 1's "optional and never derived" is a
//! rule about *routes*, where a name is only a reverse lookup. Here it is the
//! word a command line selects the command *by*, so a row without one is
//! reachable by nothing: [`code::E_COMMAND_WITHOUT_NAME`], reported from the
//! pass that needs the name to build the row.
//!
//! **Every `#[Command]` on a method becomes a row**, which is ADR 0046 § 3's
//! repetition read as [`crate::routes::check_class_routes`] reads it — two
//! attributes on one method are two names for one implementation, which is what
//! an alias is. Nothing else would be safe: the alternative is taking the first
//! and ignoring the second, and a recognized attribute the compiler never looks
//! at is what this module's roster exists to prevent.
//!
//! # Known gaps
//!
//! 1. **A row carries no parameter *types*.** § 6's conversion during matching
//!    reads the declared type, and the consumer of the table already holds the
//!    signature the row's handler names; carrying a second copy across would be
//!    a table that can disagree with the declaration it was built from.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Attribute, ClassDecl, ClassMemberKind, MethodMember, Param};
use rustc_hash::FxHashMap;

use crate::defaults::ConstArg;
use crate::testing::OptionTy;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// `#[Command(name: string, about: string)]` — ADR 0086 § 6's own spelling, in
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
}

/// One row of ADR 0086 § 6's table: a `#[Command]` that named the one field a
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
/// depend on filesystem enumeration ([ADR 0061](../../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
/// § 3).
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

/// Every `#[Command]` method `decl` declares, held to the two of § 6's three
/// compile errors that one parameter list answers, and collected into `env`'s
/// table.
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
        // are a fact about the parameter list, so asking per attribute would
        // report one collision once per alias.
        let args = check_options(m, class, ctx, env);
        let handler = format!("{class}::{}", span_text(env.src, m.name));
        for attr in commands {
            collect_command(attr, &handler, &args, env);
        }
    }
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

/// § 6's first compile error, over the finished table: two commands claiming
/// one name.
///
/// Run once at the end of [`crate::check::check_program`] rather than as each
/// class is walked, because ADR 0061 § 3's scan is what puts two files'
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

/// ADR 0086 § 6's marker held to the declaration that reads it: an `#[Option]`
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
                "ADR 0086 § 6 gives `#[Option]` its meaning on a `#[Command]` method's parameter, \
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
        let Some(attr) =
            crate::testing::attribute_named(&param.attributes, crate::derive::OPTION, ctx, env)
        else {
            // § 6's positional argument: no attribute, no spelling, and its
            // place in this list is the place a command line fills it from.
            args.push(CommandArg {
                param: name,
                spellings: Vec::new(),
                about: None,
            });
            continue;
        };
        let attr = attr.clone();
        if let Some(ty) = sig.and_then(|sig| sig.params.get(index).copied()) {
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
        });
    }
    args
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

/// § 6's third compile error: the parameter an `#[Option]` is attached to must
/// have a type an argument's text can be converted to.
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
             types, or `Core\\Uuid`",
        ),
    );
}

/// ADR 0077 § 3's conversion roster, which § 6 takes unchanged, plus the one
/// row § 6 adds: a `bool` `#[Option]` is a flag, so it is given by being
/// written rather than by carrying text.
///
/// The one home for "what an argument's text may become", so the route table's
/// own pass reads this rather than growing a second list that agrees with it
/// today.
///
/// A `float` is deliberately absent where `decimal` is not: § 3 names one and
/// not the other, and an argument that quietly rounds is the class of bug
/// ADR 0054 exists to make unwritable.
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
        Ty::Class(name, _) => *name == QName::parse(r"Core\Uuid"),
        // § 3 admits a union of `string` or `int` literal types and a subset of
        // an enum's cases, and nothing wider: a `string|int` would make the
        // conversion itself ambiguous, which is the question ADR 0095 refuses
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
