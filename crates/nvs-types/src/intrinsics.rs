//! `rule:expressions/intrinsic-list-is-closed`'s
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
//!   spelled. `use Core\Str;` and `Core\Str::format(...)` are one member, and
//!   no userland `Str` is any of them, which is the same rule the four
//!   attribute passes are held to (`docs/agent/loop-goal.md`
//!   § *Standing decisions*).
//! * **A refusal must be one the runtime would also have made.** § 4 makes
//!   preparation produce an earlier answer and never a different one, so where
//!   a static type admits *any* value the runtime would have accepted, this
//!   pass says nothing: `%d` against a `?int` is left alone, because the
//!   runtime only throws on the `null` and this pass cannot know there is one.
//! * **One row refuses what the runtime accepts, and it is the exception the
//!   bullet above is stated against.** [`Grammar::MetricName`] reads
//!   `rule:observability/metrics-three-members`'s `[a-z][a-z0-9_]*` out of a
//!   written literal, while `nvs_runtime::metrics::Registry` fixes whatever it
//!   is handed — because a metric write may not fail the request that made it,
//!   which is the reading
//!   `rule:observability/past-max-series-a-new-series-is-refused` already gives
//!   a series past the bound. A naming convention enforced with a throw would
//!   be a request lost to a spelling, so the grammar is enforced where it is
//!   free and a name a request computed is accumulated as written.
//!
//! # Known gaps
//!
//! 1. **A diagnostic underlines the whole literal, not the offset inside it.**
//!    § 3 asks for the exact offset; a string literal's span covers its
//!    escapes and its quotes, so mapping a byte of the *decoded* text back to
//!    a column needs a decoder that records positions
//!    ([`crate::string_lit`] is where that would live). The message quotes the
//!    offending placeholder instead, which is what a reader searches for.
//!    Decided: Re-decode with positions only when a diagnostic is emitted — Exact carets and zero cost
//!    on the success path, at the cost of a second decoder mode.
//!    — owner: unowned-closures
//! 2. **Nothing is prepared yet.** § 3's second effect — the compiled pattern
//!    and the parsed plan stored in `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact cache — needs a
//!    channel from here to `nvs-ir`; validation is the half that pays for
//!    itself without one, and is what `nvs check` reports.
//!    Decided: Build the checker-to-IR channel; store prepared patterns in the artifact — No per-call
//!    compile, and one new channel through the lowering.
//!    — owner: unowned-closures
//! 3. **A member's own restriction on a well-formed pattern is left to run
//!    time.** `Core\Time::parse` refuses a *zonal* field in a pattern the
//!    grammar reads perfectly well (`nvs_stdlib::cldr`'s `civil_fields_only`),
//!    because its zone is argument 3 rather than something the pattern names.
//!    That is a rule about the member and not about the pattern language, and
//!    [`Grammar`] carries one variant per language by § 1's own reading — so
//!    refusing it here would need a per-row restriction the table does not
//!    have a column for. Leaving it leaves § 4 intact: everything this pass
//!    refuses, the runtime refuses too.
//!    Decided: Add a restriction column to the roster — The error comes where it was written, and the
//!    closed table gets one more column.
//!    — owner: unowned-closures
//! 4. **A named or spread argument is not read.** `Core\Str::format(template:
//!    "…")` folds nothing and runs unvalidated, exactly as
//!    [`crate::links`]' own gap 1 describes: reading one needs the slot
//!    mapping `check_args_typed` built and this pass is not handed.
//!    Decided: Hand these passes the slot mapping check_args_typed already builds — Named arguments are
//!    checked like positional ones, and three passes take a new input.
//!    — owner: unowned-closures
//! 5. **`rule:core-classes/db-literal-query-checking`'s unterminated string literal is not refused**, and the
//!    reason is a disagreement rather than an absence: `nvs_db::sql`'s own
//!    module doc declines it in the other direction, because an unterminated
//!    quote ends that scan at the end of the text and the statement goes out to
//!    be diagnosed by a parser that can say what is actually wrong with it.
//!    Refusing it here would be the one thing this pass refuses that the
//!    rewriter does not, which is § 4 read backwards. It waits on which of the
//!    two docs is right, not on a scan.
//!    Decided: Refuse at compile time and amend nvs_db::sql's doc — A certain bug is caught early, and
//!    the rewriter must agree to refuse it too.
//!    — owner: unowned-closures
//! 6. **`rule:core-classes/db-literal-query-checking`'s host check reaches only a caller that hands over a
//!    configuration**, and `nvs check` is not yet one. [`crate::Env::grants`]
//!    is the channel and [`crate::check::check_program_granted`] is how a
//!    caller fills it, but `nvs-cli`'s check path reads no `nvs.toml` today, so
//!    the refusal is real and exercised and still fires for nobody. Wiring it
//!    is a decision about `nvs check` rather than about this pass — a command
//!    that reads configuration is a command a broken `nvs.toml` can fail — and
//!    it belongs where that command's own errors are decided.
//!    — owner: unowned

use nvs_config::capability::Cap;
use nvs_diagnostics::{Diagnostic, SourceFile, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Arg, CallArgs, Expr, ExprKind};

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
    /// `rule:core-classes/regex-two-tiers`'s pattern syntax, and the engine tier it decides.
    Regex,
    /// `Core\Uri`'s well-formedness.
    Uri,
    /// `nvs_stdlib::cldr`'s pattern letters.
    DateFormat,
    /// `rule:types/duration-literal`'s `30s`/`1h30m` grammar.
    Duration,
    /// `Core\Str::format`'s `printf` template, which is the one grammar that
    /// is also checked *against the call's other arguments*.
    Template,
    /// `rule:core-classes/db-parameters`'s placeholder spelling, checked against the *params array*
    /// written beside it — the second grammar read against another argument,
    /// and the only one whose other argument is a single array rather than the
    /// variadic tail. [ADR 0067 § 10](/docs/decisions/0067.md) is
    /// what puts it on § 1's list; the vendors' SQL itself is not read here and
    /// that section says why.
    Sql,
    /// A hostname, read against the compiling machine's `db.open` grant —
    /// [ADR 0067 § 10](/docs/decisions/0067.md)'s second sentence.
    ///
    /// The odd one out, twice over, and both are deliberate. It is the only
    /// variant whose second half is the *machine's configuration* rather than
    /// the call's other arguments, and the only one that reads nothing about
    /// the text's own shape: a hostname's grammar is not what § 10 asks about,
    /// and a host this rejects is one that parses perfectly. It stays a
    /// [`Grammar`] anyway because everything else about the row is the same
    /// question — a written literal at a known address on a closed list of
    /// members, refused only where the runtime would refuse it too — and a
    /// second table beside this one, holding one row, would be the open
    /// extension point § 1 refuses.
    Host,
    /// A queue name, read against the compiling machine's `queue.purge` grant
    /// — `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s grant,
    /// asked before the program runs.
    ///
    /// [`Host`](Self::Host)'s question one capability over, and on this table
    /// for the reason that variant states: what differs between the two is
    /// which grant is walked, not what kind of check it is. The name is
    /// matched exactly, through
    /// [`Capabilities::allows_name`](nvs_config::capability::Capabilities::allows_name),
    /// because a queue name is a flat string the program itself picked and has
    /// no labels for a `*.` to match at.
    QueueName,
    /// A series name, read against
    /// `rule:observability/metrics-three-members`'s `[a-z][a-z0-9_]*`.
    ///
    /// The only variant whose refusal the runtime does not also make, and the
    /// module doc's fourth bullet is where that is argued: a name a request
    /// computed is accumulated as written, because a metric write may not fail
    /// the request that made it. So this reads the text's own shape, like every
    /// variant above [`Host`](Self::Host), and is the one that answers about a
    /// convention rather than about what a member can parse.
    MetricName,
}

/// One row of § 1's table: a member, and which of its arguments is the small
/// program.
struct Intrinsic {
    /// The declaring class's fully-qualified name.
    owner: &'static str,
    member: &'static str,
    /// The **written** argument position the pattern is at — 0 for a subject
    /// that is itself the pattern, 1 for `Core\Time::parse`, whose subject is
    /// the text being parsed (`rule:core-api/shape-rules` R1).
    at: usize,
    /// Which field *inside* the argument at [`Self::at`] carries the literal,
    /// or `None` where the argument is itself it.
    ///
    /// `rule:core-api/shape-flattens-at-the-abi`'s merged ABI does not answer this and is not what this addresses:
    /// that flattening is `nvs_ir::lower`'s, and it happens to an argument
    /// already checked. Here the shape is still one written literal, so the
    /// address is a field *name* — `rule:types/object-literal` makes [`ExprKind::ObjectLiteral`] the only spelling a shape argument
    /// has, and it carries no shorthand, no spread and no computed key for the
    /// match to fall through.
    ///
    /// A row still names exactly one literal: this addresses a *deeper* one,
    /// never a second. [`addressed`] is where the two cases meet, and every
    /// check below reads the expression it answers with rather than the
    /// argument.
    field: Option<&'static str>,
    grammar: Grammar,
}

/// `rule:expressions/intrinsic-list-is-closed`'s
/// table, and the whole of it. Nothing outside this constant is an intrinsic,
/// and nothing adds to it at run time.
const INTRINSICS: &[Intrinsic] = &[
    Intrinsic {
        owner: r"Core\Regex",
        member: "compile",
        at: 0,
        field: None,
        grammar: Grammar::Regex,
    },
    Intrinsic {
        owner: r"Core\Uri",
        member: "parse",
        at: 0,
        field: None,
        grammar: Grammar::Uri,
    },
    Intrinsic {
        owner: r"Core\Time\DateTime",
        member: "format",
        at: 0,
        field: None,
        grammar: Grammar::DateFormat,
    },
    Intrinsic {
        owner: r"Core\Time",
        member: "parse",
        at: 1,
        field: None,
        grammar: Grammar::DateFormat,
    },
    Intrinsic {
        owner: r"Core\Time\Duration",
        member: "parse",
        at: 0,
        field: None,
        grammar: Grammar::Duration,
    },
    Intrinsic {
        owner: r"Core\Str",
        member: "format",
        at: 0,
        field: None,
        grammar: Grammar::Template,
    },
    // `rule:core-classes/db-literal-query-checking`'s members, on both classes that declare them: § 7's
    // `Core\Db\Transaction` forwards the interface to its connection, so the
    // same statement written inside a transaction is the same check. The rows
    // are nominal against the *declaring* class, so the receiver is not counted
    // and `at: 0` is the `sql` parameter on every one of them.
    //
    // `executeMany` is deliberately absent: its second argument is `sets`, a
    // list of parameter *sets* rather than one, so the count this pass makes is
    // not the count that member binds.
    Intrinsic {
        owner: r"Core\Db\Connection",
        member: "query",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Connection",
        member: "queryAs",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Connection",
        member: "execute",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    // `stream` binds the same one statement `query` does — § 4 gives them one
    // signature and one binding rule, and where the rows are when the member
    // answers is nothing this pass can see. `streamAs` is that member at a
    // written type, and what a row becomes is not a binding either.
    Intrinsic {
        owner: r"Core\Db\Connection",
        member: "stream",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Connection",
        member: "streamAs",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Transaction",
        member: "query",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Transaction",
        member: "queryAs",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Transaction",
        member: "execute",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Transaction",
        member: "stream",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    Intrinsic {
        owner: r"Core\Db\Transaction",
        member: "streamAs",
        at: 0,
        field: None,
        grammar: Grammar::Sql,
    },
    // § 10's second sentence, and the only row that addresses a field: § 18
    // writes the host inside the `Db\Settings` shape, so `at` names the shape
    // and `field` names the key. `connect` has no row beside it on purpose —
    // its endpoint is the one an operator wrote into root-owned configuration,
    // which § 3 pre-approves, and there is no program-supplied host to read.
    Intrinsic {
        owner: r"Core\Db",
        member: "open",
        at: 0,
        field: Some("host"),
        grammar: Grammar::Host,
    },
    // `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s written queue
    // name. `purge`'s subject is the queue itself (`rule:core-api/shape-rules`
    // R1), so `at: 0` addresses the argument rather than a field inside a
    // shape. `delete` has no row beside it, and cannot have one: its queue
    // arrives inside a `Queue\Id` at run time, so there is no written name for
    // this pass to read and the door is the only place its grant is asked
    // about.
    Intrinsic {
        owner: r"Core\Queue",
        member: "purge",
        at: 0,
        field: None,
        grammar: Grammar::QueueName,
    },
    // `rule:observability/metrics-three-members`'s series name, on all three
    // verbs: the grammar belongs to the series rather than to the verb it was
    // first written with, so reading it on one member and not the others would
    // make one name a compile error in one call and fine in the next.
    //
    // `at: 0` on every row, because `at` addresses the **written** argument and
    // the call-site constant `nvs_stdlib::registry::SOURCE_MEMBERS` puts in
    // front of the name is `nvs-ir`'s, spliced long after this pass has run.
    Intrinsic {
        owner: r"Core\Metrics",
        member: "increment",
        at: 0,
        field: None,
        grammar: Grammar::MetricName,
    },
    Intrinsic {
        owner: r"Core\Metrics",
        member: "observe",
        at: 0,
        field: None,
        grammar: Grammar::MetricName,
    },
    Intrinsic {
        owner: r"Core\Metrics",
        member: "gauge",
        at: 0,
        field: None,
        grammar: Grammar::MetricName,
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
    // Gap 4: a `name:` or a `...` moves an argument away from the position the
    // row names, so the whole call is left to run time rather than read out of
    // order.
    if list.iter().any(|arg| arg.name.is_some() || arg.spread) {
        return;
    }
    let Some(arg) = list.get(row.at) else {
        // A missing argument is the arity check's refusal, already made.
        return;
    };
    let Some(pattern) = addressed(row, &arg.value, env.src) else {
        return;
    };
    let span = pattern.span;
    let Some(ConstArg::Str(text)) = folded_str(pattern, env) else {
        return;
    };
    match row.grammar {
        Grammar::Template => check_template(&text, span, row, arg_types, env),
        Grammar::Sql => check_sql(&text, span, row, list, env),
        // Both CLDR rows read the same pattern language through the same
        // `compile`, which is why they share one variant: the two members
        // differ only in what they do with the pieces afterwards.
        Grammar::DateFormat => {
            if let Err(message) = nvs_stdlib::cldr::validate(&text) {
                report_malformed(span, &message, env);
            }
        }
        // `rule:core-classes/regex-literal-tiering`'s compile-time fact, both halves of it: the pattern is
        // offered to the same two engines the first call would have offered it
        // to, and the tier it landed in is written down where `nvs-ir` reads
        // it back. The tier is settleable here for the reason
        // `nvs_stdlib::regex::Tier` states — it is a property of the pattern
        // text, decided by which engine's parser refused a construct, so a
        // checking run and a request cannot disagree about it.
        Grammar::Regex => match nvs_stdlib::regex::validate(&text) {
            Ok(tier) => env.exprs.record_regex_tier(span, tier),
            Err(message) => report_malformed(span, &message, env),
        },
        // Both of `Core\Uri::parse`'s throwing steps, which is the rule its
        // own `tryParse` is written around: a validator that agrees with the
        // parser about only one of them is the divergence that member exists
        // to prevent.
        Grammar::Uri => {
            if let Err(message) = nvs_stdlib::uri::validate(&text) {
                report_malformed(span, &message, env);
            }
        }
        // `rule:types/duration-literal`'s three places that must agree already share one parser,
        // and it lives in `nvs-syntax` because the lexer is one of the three.
        // So this arm reaches no validator of its own: the function below is
        // the same call `nvs_core_time_duration_parse` makes and the same one
        // the lexer makes for a `1h30m` literal, which is why a folded
        // `Core\Time\Duration::parse("1h30m")` cannot disagree with either.
        Grammar::Duration => {
            if let Err(err) = nvs_syntax::duration::parse(&text) {
                report_malformed(span, &err.message(), env);
            }
        }
        // Both halves have to be facts here, and `grants` being `None` is the
        // second one missing: see `crate::check::check_program_granted` for
        // why an absent configuration says nothing instead of denying.
        Grammar::Host => {
            if let Some(grants) = env.grants
                && !grants.allows_host(Cap::DbOpen, &text)
            {
                report_ungranted(span, &text, env);
            }
        }
        // The arm above's two facts, asked of a different grant: an absent
        // configuration is the second one missing here too, and says nothing
        // rather than denying.
        Grammar::QueueName => {
            if let Some(grants) = env.grants
                && !grants.allows_name(Cap::QueuePurge, &text)
            {
                report_ungranted_queue(span, &text, env);
            }
        }
        // The one arm whose refusal the runtime does not repeat, for the reason
        // [`Grammar::MetricName`] gives. The grammar itself is
        // `nvs_stdlib::metrics`', so the checker and the reference card cannot
        // come to spell it differently.
        Grammar::MetricName => {
            if let Err(message) = nvs_stdlib::metrics::validate_name(&text) {
                report_metric_name(span, &message, env);
            }
        }
    }
}

/// A literal query, and the literal params array written beside it —
/// [ADR 0067 § 10](/docs/decisions/0067.md)'s refused second
/// statement, placeholder count and positional-vs-named consistency.
///
/// The two halves are two codes because they are two mistakes: a second
/// statement is a text this member cannot send whatever it is handed, and the
/// other two are a pairing that could have been written to agree. § 10's fourth
/// clause — an unterminated string literal — is not made here, and the module's
/// known gaps own why.
///
/// The refusal is `nvs_stdlib::db::check_literal_query`'s, which is the
/// rewriter the request itself would have run; that function's doc owns why it
/// is the rewriter rather than a second reader, and why a refusal has to hold
/// on all four dialects.
///
/// **The params array is read for its shape and never for its values.** Every
/// element stands in as one bound slot, including a `Core\Db::inList(…)`: § 5's
/// expansion changes how many *markers* an element renders to and never how
/// many slots the call binds, which is the only number either check counts.
/// An array this pass cannot read whole — a spread, a variable, keyed and
/// unkeyed elements mixed — is not read at all, per § 2.
fn check_sql(
    text: &str,
    span: nvs_diagnostics::Span,
    row: &Intrinsic,
    list: &[Arg],
    env: &mut Env<'_>,
) {
    // § 1's statement count first, because it is a fact about the literal alone
    // and holds however unreadable the params array beside it turns out to be.
    if let Err(message) = nvs_stdlib::db::check_single_statement(text) {
        report_malformed(span, &message, env);
        return;
    }
    let Some(params) = list.get(row.at + 1) else {
        // A missing argument is the arity check's refusal, already made.
        return;
    };
    let ExprKind::ArrayLiteral(items) = &params.value.unparenthesized().kind else {
        return;
    };
    if items.iter().any(|item| item.spread || item.by_ref) {
        return;
    }
    let keyed = items.iter().filter(|item| item.key.is_some()).count();
    let refused = if keyed == 0 {
        nvs_stdlib::db::check_literal_query(
            text,
            nvs_stdlib::db::LiteralParams::Positional(items.len()),
        )
    } else if keyed == items.len() {
        let mut names = Vec::with_capacity(items.len());
        for item in items {
            let Some(key) = item.key.as_ref() else {
                return;
            };
            let Some(ConstArg::Str(name)) = folded_str(key, env) else {
                return;
            };
            names.push(name);
        }
        let keys: Vec<&str> = names.iter().map(String::as_str).collect();
        nvs_stdlib::db::check_literal_query(text, nvs_stdlib::db::LiteralParams::Named(&keys))
    } else {
        // § 5's two spellings in one array. The array is neither list-keyed nor
        // string-keyed, and the `LogicError` for that is raised where the array
        // is rather than against the query.
        return;
    };
    if let Err(message) = refused {
        report_query(span, &message, env);
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
        // `rule:types/conversion`'s one unchecked position, and the two shapes standing
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

/// A series name outside `rule:observability/metrics-three-members`'s grammar,
/// refused where it was written.
///
/// [`report_malformed`]'s code, because it is that kind of mistake — a literal
/// this member does not read — and its own help, because that function's
/// closing promise is untrue here: a computed name is not checked when it runs,
/// and a reader who expects it to be has been told the wrong thing about their
/// program.
fn report_metric_name(span: nvs_diagnostics::Span, message: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_INTRINSIC_LITERAL_MALFORMED,
            format!("this literal is not one this member can read: {message}"),
        )
        .with_primary(span, "read while compiling, because it is a constant")
        .with_help(
            "a series name is lower case, digits and `_`, starting with a letter — \
             `http_requests_total` rather than `HttpRequests` — so that every backend a \
             scrape reaches spells it the same way; a name a request computes is not read \
             here and accumulates as written",
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

/// [ADR 0067 § 10](/docs/decisions/0067.md)'s refusals, which are the
/// same *kind* as [`report_mismatch`]'s and share its code: the literal is one
/// the rewriter reads perfectly well, and the mistake is in the pairing.
fn report_query(span: nvs_diagnostics::Span, message: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_FORMAT_TEMPLATE_MISMATCH,
            format!("this query and its parameters do not agree: {message}"),
        )
        .with_primary(span, "read while compiling, because it is a constant")
        .with_help(
            "the compiler binds a literal query with the same rewriter the request would have \
             used, so this is the `LogicError` the first call would have thrown — `?` and \
             `:name` are `rule:core-classes/db-parameters`'s two spellings and one statement uses one of them",
        ),
    );
}

/// [ADR 0067 § 10](/docs/decisions/0067.md)'s host refusal, which is
/// not [`report_query`]'s kind at all: nothing is wrong with the literal, and
/// what the message has to carry is the *deployment* it was checked against.
///
/// The help names `db.open` and not a launderer, because the answer is an
/// operator's and not the program's — § 3 makes an address a sink with no way
/// through from inside the program, and this is the other half of that: a host
/// nobody granted is a host this deployment does not reach, however it was
/// written.
fn report_ungranted(span: nvs_diagnostics::Span, host: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNGRANTED_HOST,
            format!("`{host}` is not a host this deployment's `db.open` grants"),
        )
        .with_primary(span, "read while compiling, because it is a constant")
        .with_help(
            "`rule:core-classes/db-capabilities`'s `db.open` lists the hosts a program-supplied `Db\\Settings` may \
             reach, and it denies by default — add this host to `[capabilities] db.open` in \
             `nvs.toml`, or name a `[db.<name>]` block and open it with `Core\\Db::connect`",
        ),
    );
}

/// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s refusal, which
/// is [`report_ungranted`]'s kind exactly: nothing is wrong with the literal,
/// and what the message has to carry is the *deployment* it was checked
/// against.
///
/// The help names the grant and no way around it, for the reason the rule
/// gives: the queue name is program-written, but destroying the record that
/// work existed is the operator's answer, and the entry belongs in the one
/// place in a deployment that holds the name rather than on the request path.
fn report_ungranted_queue(span: nvs_diagnostics::Span, queue: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_UNGRANTED_QUEUE,
            format!("`{queue}` is not a queue this deployment's `queue.purge` grants"),
        )
        .with_primary(span, "read while compiling, because it is a constant")
        .with_help(
            "`rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `queue.purge` lists the queues a program may \
             remove rows from, and it denies by default — add this queue to `[app.capabilities.queue] purge` in \
             `nvs.toml`, or leave the removal to the entry that already holds the name",
        ),
    );
}

/// The expression a row addresses: the written argument itself, or the one
/// field named inside the shape literal written there.
///
/// The two cases meet here rather than at each grammar's arm, so that
/// everything below reads *one* expression and neither knows nor cares how
/// deep it was written. [`Intrinsic::field`] owns why the address is a name.
///
/// `None` — nothing to read, and never a refusal of its own. An argument that
/// is not a literal shape at all is § 2's rule, and a field the literal did
/// not write is either the arm check's error, already reported, or a default
/// this pass never saw the text of.
fn addressed<'a>(row: &Intrinsic, arg: &'a Expr, src: &SourceFile) -> Option<&'a Expr> {
    let Some(name) = row.field else {
        return Some(arg);
    };
    let ExprKind::ObjectLiteral(fields) = &arg.unparenthesized().kind else {
        return None;
    };
    fields
        .iter()
        .find(|field| crate::span_text(src, field.name) == name)
        .map(|field| &field.value)
}

/// One expression folded as a `string`, through the one literal decoder — see
/// [`crate::links::folded_str`], which reads a route name the same way.
fn folded_str(expr: &Expr, env: &mut Env<'_>) -> Option<ConstArg> {
    let declared = env.interner.intern(Ty::String);
    crate::defaults::literal_default(expr, declared, env)
}
