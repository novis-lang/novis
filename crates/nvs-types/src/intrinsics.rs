//! [ADR 0057](../../../docs/adr/0057-intrinsic-literal-folding.md) § 1's
//! closed list: the `Core` members whose pattern-like argument the compiler
//! reads while checking, and the hook that consults it.
//!
//! The sibling of [`crate::retrieval`], [`crate::program`] and
//! [`crate::links`] — a `Core` call the compiler knows by name — and it
//! differs from all three in what it does with the answer: those three
//! *replace* the call, and this one leaves it exactly as written. § 3 is why:
//! a literal pattern is **validated** always and **prepared** where there is
//! something to prepare, and neither is an evaluation. `$when->format("y")`
//! still formats at run time, because the instant and the zone are runtime
//! values.
//!
//! # What is decided here
//!
//! * **The roster is § 1's table verbatim**, and adding a row is a compiler
//!   change with a fixture — that section's own rule. The consequence is
//!   deliberate asymmetry: `Core\Time\Date::format` reads the same CLDR
//!   patterns `Core\Time\DateTime::format` does and is not on the list, so its
//!   malformed literal is still a throw. Widening the roster to "every member
//!   whose parameter happens to be a pattern" is exactly the open extension
//!   point § 1 refuses, and the cost of the asymmetry is one row per member
//!   whenever someone decides a member has earned one.
//! * **A row is matched nominally**, against the *declaring* class
//!   [`crate::expr::calls`] resolved — never against what the call site
//!   spelled. `use Core\Str;` and `\Core\Str::format(...)` are one member, and
//!   no userland `Str` is any of them, which is the same rule the four
//!   attribute passes are held to (`docs/agent/loop-goal.md`
//!   § *Standing decisions*).
//! * **A refusal must be one the runtime would also have made.** § 4 makes
//!   preparation produce an earlier answer and never a different one, so where
//!   a static type admits *any* value the runtime would have accepted, this
//!   pass says nothing: `%d` against a `?int` is left alone, because the
//!   runtime only throws on the `null` and this pass cannot know there is one.
//!
//! # Known gaps
//!
//! 1. **A diagnostic underlines the whole literal, not the offset inside it.**
//!    § 3 asks for the exact offset; a string literal's span covers its
//!    escapes and its quotes, so mapping a byte of the *decoded* text back to
//!    a column needs a decoder that records positions
//!    ([`crate::string_lit`] is where that would live). The message quotes the
//!    offending placeholder instead, which is what a reader searches for.
//! 2. **Nothing is prepared yet.** § 3's second effect — the compiled pattern
//!    and the parsed plan stored in ADR 0042's artifact cache — needs a
//!    channel from here to `nvs-ir`; validation is the half that pays for
//!    itself without one, and is what `nvs check` reports.
//! 3. **A named or spread argument is not read.** `Core\Str::format(template:
//!    "…")` folds nothing and runs unvalidated, exactly as
//!    [`crate::links`]' own gap 1 describes: reading one needs the slot
//!    mapping `check_args_typed` built and this pass is not handed.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_syntax::ast::{CallArgs, Expr};

use crate::Env;
use crate::defaults::ConstArg;
use crate::ty::{Ty, TypeId};

/// What kind of small program a folded argument is, and therefore which
/// grammar reads it.
///
/// One variant per column of § 1's table rather than one per row: two members
/// reading CLDR patterns share [`Grammar::DateFormat`], because two grammars
/// for one pattern language is the divergence § 4 forbids.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Grammar {
    /// ADR 0056's pattern syntax, and the engine tier it decides.
    Regex,
    /// `Core\Uri`'s well-formedness.
    Uri,
    /// `nvs_stdlib::cldr`'s pattern letters.
    DateFormat,
    /// ADR 0070's `30s`/`1h30m` grammar.
    Duration,
    /// `Core\Str::format`'s `printf` template, which is the one grammar that
    /// is also checked *against the call's other arguments*.
    Template,
}

/// One row of § 1's table: a member, and which of its arguments is the small
/// program.
struct Intrinsic {
    /// The declaring class's fully-qualified name.
    owner: &'static str,
    member: &'static str,
    /// The **written** argument position the pattern is at — 0 for a subject
    /// that is itself the pattern, 1 for `Core\Time::parse`, whose subject is
    /// the text being parsed (ADR 0063 R1).
    at: usize,
    grammar: Grammar,
}

/// [ADR 0057](../../../docs/adr/0057-intrinsic-literal-folding.md) § 1's
/// table, and the whole of it. Nothing outside this constant is an intrinsic,
/// and nothing adds to it at run time.
const INTRINSICS: &[Intrinsic] = &[
    Intrinsic {
        owner: r"Core\Regex",
        member: "compile",
        at: 0,
        grammar: Grammar::Regex,
    },
    Intrinsic {
        owner: r"Core\Uri",
        member: "parse",
        at: 0,
        grammar: Grammar::Uri,
    },
    Intrinsic {
        owner: r"Core\Time\DateTime",
        member: "format",
        at: 0,
        grammar: Grammar::DateFormat,
    },
    Intrinsic {
        owner: r"Core\Time",
        member: "parse",
        at: 1,
        grammar: Grammar::DateFormat,
    },
    Intrinsic {
        owner: r"Core\Time\Duration",
        member: "parse",
        at: 0,
        grammar: Grammar::Duration,
    },
    Intrinsic {
        owner: r"Core\Str",
        member: "format",
        at: 0,
        grammar: Grammar::Template,
    },
];

/// § 1's row for a resolved target, or `None` for the overwhelming majority of
/// calls — the nominal test [`crate::links::is_link`] makes, answered with the
/// row rather than with a `bool` because every caller needs the row next.
fn row(owner: &QName, member: &str) -> Option<&'static Intrinsic> {
    let owner = owner.to_string();
    INTRINSICS
        .iter()
        .find(|row| row.owner == owner && row.member == member)
}

/// Reads an intrinsic's literal argument, where the call has one — the hook
/// both call sites in [`crate::expr::calls`] reach after the target has
/// resolved and the arguments have been typed.
///
/// `arg_types` is indexed by *written* argument, so an instance call's
/// receiver is not in it and neither is a default the site did not write.
/// Reports nothing at all for a call whose argument does not fold, which is
/// § 2's rule: nothing is refused for being dynamic.
pub(crate) fn check_call(
    owner: &QName,
    member: &str,
    args: &CallArgs,
    arg_types: &[TypeId],
    env: &mut Env<'_>,
) {
    let Some(row) = row(owner, member) else {
        return;
    };
    let CallArgs::List(list) = args else {
        return;
    };
    // Gap 3: a `name:` or a `...` moves an argument away from the position the
    // row names, so the whole call is left to run time rather than read out of
    // order.
    if list.iter().any(|arg| arg.name.is_some() || arg.spread) {
        return;
    }
    let Some(pattern) = list.get(row.at) else {
        // A missing argument is the arity check's refusal, already made.
        return;
    };
    let Some(ConstArg::Str(text)) = folded_str(&pattern.value, env) else {
        return;
    };
    match row.grammar {
        Grammar::Template => check_template(&text, pattern.value.span, row, arg_types, env),
        // Each remaining grammar's reader lands with its own slice — the row
        // is here first because § 1's *list* is the decision and a parser is
        // only what implements it. Until then these members behave exactly as
        // they did: the literal reaches the runtime unvalidated, which is what
        // this pass is replacing rather than something it breaks.
        Grammar::Regex | Grammar::Uri | Grammar::DateFormat | Grammar::Duration => {}
    }
}

/// `Core\Str::format`'s template, against the arguments written beside it.
///
/// Three refusals, and they are `nvs_stdlib::format`'s own three: a
/// placeholder the grammar cannot read, an argument the template asks for and
/// the call did not give, and an argument no placeholder consumes. The fourth
/// — a value with no reading for its conversion — is the one this pass makes
/// *narrower* than the runtime does, per the module docs' third bullet.
fn check_template(
    text: &str,
    span: nvs_diagnostics::Span,
    row: &Intrinsic,
    arg_types: &[TypeId],
    env: &mut Env<'_>,
) {
    let placeholders = match nvs_stdlib::format::placeholders(text) {
        Ok(placeholders) => placeholders,
        Err(message) => {
            report_malformed(span, &message, env);
            return;
        }
    };
    // The arguments the template reads are the ones after it — `format` is
    // variadic in exactly the tail, so the written position of argument `n` is
    // the template's own plus one.
    let given = arg_types.len().saturating_sub(row.at + 1);
    let mut read = vec![false; given];
    for placeholder in &placeholders {
        let Some(seen) = read.get_mut(placeholder.index) else {
            report_mismatch(
                span,
                &nvs_stdlib::format::reads_missing(placeholder.index, given),
                "the template reads further than the arguments go",
                env,
            );
            return;
        };
        *seen = true;
    }
    if let Some(unused) = read.iter().position(|seen| !seen) {
        report_mismatch(
            span,
            &nvs_stdlib::format::never_read(unused),
            "an argument no placeholder in this template reads",
            env,
        );
        return;
    }
    for placeholder in &placeholders {
        let ty = arg_types[row.at + 1 + placeholder.index];
        if reads_number(placeholder.conversion) && !may_be_number(ty, env) {
            let described = env.interner.describe(ty);
            let conversion = placeholder.conversion;
            report_mismatch(
                span,
                &format!(
                    "`%{conversion}` reads a number, and argument {} is a `{described}`",
                    placeholder.index + 1
                ),
                "this template and these arguments do not fit",
                env,
            );
            return;
        }
    }
}

/// Whether a conversion needs a number rather than anything renderable — every
/// one but `%s`, which is `nvs_runtime::value_to_string`'s own question and is
/// left to it.
const fn reads_number(conversion: char) -> bool {
    !matches!(conversion, 's')
}

/// Whether *some* value of this static type is one the runtime's own reader
/// would accept for a numeric conversion — `nvs_stdlib::format`'s `integer`
/// and `floating` rows, read as a question about a type.
///
/// The direction matters and is § 4's soundness rule: this answers `true`
/// whenever a refusal would be a guess, so `?int`, `mixed` and a type variable
/// all pass and only a type with no numeric reading at all is refused.
fn may_be_number(ty: TypeId, env: &Env<'_>) -> bool {
    match env.interner.get(ty) {
        Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::Bool
        | Ty::True
        | Ty::False
        | Ty::IntLiteral(_) => true,
        // ADR 0007 § 2's one unchecked position, and the two shapes standing
        // for a type this call site does not name: nothing is knowable here,
        // so nothing is refused.
        Ty::Mixed | Ty::TypeVar(_) | Ty::Never => true,
        Ty::Union(members) => members.iter().any(|member| may_be_number(*member, env)),
        _ => false,
    }
}

/// § 3's first effect: a malformed constant, reported where it was written.
fn report_malformed(span: nvs_diagnostics::Span, message: &str, env: &mut Env<'_>) {
    let stated = message
        .strip_prefix("Core\\Str::format(): ")
        .unwrap_or(message);
    env.diags.report(
        Diagnostic::error(
            code::E_INTRINSIC_LITERAL_MALFORMED,
            format!("this literal is not one this member can read: {stated}"),
        )
        .with_primary(span, "read while compiling, because it is a constant")
        .with_help(
            "the compiler reads a literal pattern with the same parser the runtime would have \
             used, so this is the error the first call would have thrown — a computed argument \
             is checked when it runs instead",
        ),
    );
}

/// The template refusals that are about the *call* rather than the literal:
/// a count or a type the arguments beside it do not satisfy.
fn report_mismatch(span: nvs_diagnostics::Span, message: &str, label: &str, env: &mut Env<'_>) {
    let stated = message
        .strip_prefix("Core\\Str::format(): ")
        .unwrap_or(message);
    env.diags.report(
        Diagnostic::error(
            code::E_FORMAT_TEMPLATE_MISMATCH,
            format!("this template and its arguments do not agree: {stated}"),
        )
        .with_primary(span, label)
        .with_help(
            "every argument must be read by at least one placeholder and every placeholder must \
             have an argument to read — `%1$s` numbers them where the order differs",
        ),
    );
}

/// One expression folded as a `string`, through the one literal decoder — see
/// [`crate::links::folded_str`], which reads a route name the same way.
fn folded_str(expr: &Expr, env: &mut Env<'_>) -> Option<ConstArg> {
    let declared = env.interner.intern(Ty::String);
    crate::defaults::literal_default(expr, declared, env)
}
