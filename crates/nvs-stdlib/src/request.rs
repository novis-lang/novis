//! `Core\Request` — the request a program is answering, replacing `$_GET`,
//! `$_POST`, `$_COOKIE`, `$_FILES`, `$_REQUEST` and `filter_input`
//! ([ADR 0012](../../../docs/adr/0012-no-superglobals.md)).
//!
//! # What is here, and what is not
//!
//! Three of
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) § 15's
//! fifteen members: `method`, `path` and `query` — the request *line*, which is
//! what a request has before anything has been read off its body or its
//! headers. `body`, `bodyStream`, `header`, `headers`, `cookie`, `files`,
//! `clientIp`, `scheme`, `host`, `mount`, `route` and `isHead` are known gaps of
//! this module rather than of § 15, and each waits on a different thing:
//! `header`/`cookie` on the inbound headers reaching
//! [`nvs_runtime::Inbound`], `files`/`body`/`bodyStream` on
//! [ADR 0105](../../../docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)'s
//! streaming reader, `route`/`mount` on the match `nvs_server` makes once
//! before the handler, and `isHead` on the two lines below it.
//!
//! # There is no request here, and that is a throw
//!
//! Every member refuses when the context is answering no request —
//! [ADR 0012](../../../docs/adr/0012-no-superglobals.md) § 7's rule, which that
//! ADR argues over a *spawned isolate* and which reaches the same conclusion
//! one step wider. A CLI program, a scheduled script, a job worker and a
//! `#[Test]` method are all running with nothing inbound, and an empty string
//! would say the request arrived and sent nothing. Those are different facts,
//! and collapsing them is the silent-wrong-answer failure mode
//! [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) exists to close:
//! a program that read a path out of a scheduled script and got `""` would
//! route on it.
//!
//! `LogicError` rather than a class of this module's own. It is § 10's "a bad
//! state" exactly — the program asked a question its own situation has no
//! answer to — and a named class would be a `catch` name for a condition no
//! correct program ever recovers from.
//!
//! # Where a verb becomes a case
//!
//! [`crate::router`]'s `Core\Http\Method` is the closed roster of eight, and
//! this module is the only place a string is turned into one — the gap that
//! module's own doc names. The set is closed precisely so that a verb outside
//! it never reaches a route table, so the parse here is exact and
//! case-sensitive (RFC 9110 § 9.1 makes a method a case-sensitive token) and an
//! unrecognized one is refused rather than mapped to something near it, which
//! is [ADR 0095](../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)'s
//! rule. **A peer never sees that refusal**: the server answers `501` at the
//! door, before an isolate exists, so what this throw covers is an embedder
//! that wrote a verb of its own onto a context.
//!
//! `HEAD` answers `Get`, which is § 15's own sentence and not a convenience: a
//! `Get`-only route table must match a `HEAD` request, because a `HEAD` *is* a
//! `Get` whose body is dropped. So `Core\Http\Method::Head` is a case
//! `method()` never answers, and the truth is `isHead`'s to carry — which is
//! the one gap above that this member's shape creates rather than inherits.
//!
//! # What `query` costs, and what it does not carry
//!
//! `query` parses the raw query string on **every call**, through
//! [`crate::uri::parse_query`] — the same code `Core\Uri::parseQuery` runs,
//! which is what spec § 9 promises when it says reproducing PHP's bracket
//! convention there is what lets this member answer the same shape. Two lookups
//! parse twice. That is O(query) per read rather than per request, and it is
//! deliberate for now: a memoized parse is state on the context, and the
//! context does not hold a *parsed* request yet — only the bytes one arrived
//! as. A request with two dozen reads is what would make it worth having, and
//! the inbound carrier is where it will live.
//!
//! **The `tainted` qualifier does not survive `mixed`.** `path` is a
//! `tainted string` and the checker holds it to
//! [ADR 0024](../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)'s
//! sinks; `query` answers `mixed`, because § 9's bracket convention makes a
//! value a `string` *or* a nested array, and `nvs_types` has no `tainted
//! array<T>` — the qualifier axes are defined over `string` and `bytes`. So a
//! program checks the answer out with `as`, and what it lands in is a plain
//! `string`. That is the same hole `Core\Uri::parseQuery` already has and is
//! not new here, but it is worth naming at the one member most likely to be the
//! source of an injection: `Core\Request::header` and `::cookie`, which answer
//! a `tainted string` directly, do carry it.

use nvs_runtime::{Ctx, Fault, Inbound, NvsStr, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// `Core\Request`'s fully-qualified name, in one place so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Request";

/// Spec § 15's `Core\Request`, as much of it as the request line answers.
///
/// The three here share one property that the twelve still to land do not: they
/// are answerable from the request *line*, so nothing about them waits on a
/// body being read or on headers crossing. That is why they are the first
/// three — they close [`crate::router`]'s "nothing converts a verb into a case"
/// gap without needing anything else to exist.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "method",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(crate::router::METHOD_NAME),
            symbol: "nvs_core_request_method",
            doc: Some(&METHOD_DOC),
        },
        CoreMethod {
            name: "path",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_request_path",
            doc: Some(&PATH_DOC),
        },
        CoreMethod {
            name: "query",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_request_query",
            doc: Some(&QUERY_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Request::method`'s reference card — ADR 0117.
const METHOD_DOC: MethodDoc = MethodDoc {
    short: "The verb this request carries, as one of `Core\\Http\\Method`'s eight cases — with \
            `HEAD` reported as `Get`, so a `Get`-only route table still matches one and \
            `isHead` carries the difference.",
    params: &[],
    ret: "The matching `Core\\Http\\Method` case. Never `Head`, by the rule above.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request — a CLI program, a scheduled script, a \
               job worker or a test — or the verb it carries is outside the eight \
               `Core\\Http\\Method` names, which the server refuses with a `501` before a \
               program runs.",
    }],
};

/// `Core\Request::path`'s reference card — ADR 0117.
const PATH_DOC: MethodDoc = MethodDoc {
    short: "The request path with the matched mount's prefix removed, so an application reads \
            the same paths wherever it is mounted.",
    params: &[],
    ret: "The remainder of the path after the mount prefix, percent-encoded as it arrived and \
          `tainted`. What was removed is `mount()`'s to report.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request.",
    }],
};

/// `Core\Request::query`'s reference card — ADR 0117.
const QUERY_DOC: MethodDoc = MethodDoc {
    short: "One query-string parameter by name, read with PHP's bracket convention — the same \
            parse `Core\\Uri::parseQuery` performs, so `a[b]=c` is reached as a nested array \
            under `a`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The parameter's name, decoded — the key as a form writes it, without brackets \
               for a nested value.",
        shape: &[],
    }],
    ret: "The parameter's value as a `string`, a nested `array<mixed>` for a bracketed key, or \
          `null` where the query carried no such name. Check it out with `as`, which throws on \
          input the type does not fit rather than quietly yielding zero.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request, or the query string holds percent \
               escapes that decode to octets that are not UTF-8.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_request_method" => (nvs_core_request_method as *const ()).cast(),
        "nvs_core_request_path" => (nvs_core_request_path as *const ()).cast(),
        "nvs_core_request_query" => (nvs_core_request_query as *const ()).cast(),
        _ => return None,
    })
}

/// The request this context is answering, or ADR 0012 § 7's refusal.
///
/// One function rather than three copies of the same `let else`, because what
/// the three members share is not the message but the *rule*: the module doc
/// owns it, and a second wording of it at a second site is how two members
/// would come to disagree about what "no request" means.
fn inbound_of<'a>(ctx: &'a Ctx, member: &str) -> Result<&'a Inbound, Fault> {
    ctx.inbound().ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Request::{member}(): there is no request here — this program is not \
                 answering one, as a CLI program, a scheduled script, a job worker and a test \
                 are not. That is refused rather than answered empty, because \"no request \
                 arrived\" and \"the request sent nothing\" are different facts"
            ),
        )
    })
}

/// Which of [`crate::router::METHOD`]'s eight cases a verb is, by its ordinal,
/// or `None` for a token outside the roster.
///
/// `HEAD` answers `Get`'s ordinal, which the module doc argues; that is the one
/// row here that is a decision rather than a transcription, and it is why the
/// roster's `Head` never appears on the right.
fn method_ordinal(verb: &str) -> Option<i64> {
    Some(match verb {
        "GET" | "HEAD" => 0,
        "OPTIONS" => 2,
        "TRACE" => 3,
        "POST" => 4,
        "PUT" => 5,
        "PATCH" => 6,
        "DELETE" => 7,
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Request::method(): Core\Http\Method` — spec § 15's verb, and the
    /// one place a request's method token becomes a case of the closed roster.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (ADR 0010), which is why the answer is an `int` here.
    fn nvs_core_request_method(ctx, _args: [0]) {
        let verb = inbound_of(ctx, "method")?.method();
        let Some(ordinal) = method_ordinal(verb) else {
            // Unreachable from source, and by a route no diagnostic takes: a
            // program cannot build the request it is answering, and the only
            // writer of one is `nvs_server`, which answers `501` at the door
            // for a verb outside the roster. So no case can reach this, and it
            // is kept for the embedder that writes an `Inbound` of its own and
            // is bound by no door — where answering `Get` for a verb nobody
            // recognised is exactly ADR 0095's repair.
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "Core\\Request::method(): `{verb}` is not one of the eight verbs \
                     `Core\\Http\\Method` names, and the set is closed so that a verb \
                     outside it never reaches a route table — a served request carrying \
                     one is answered `501` before this program starts"
                ),
            ));
        };
        Ok(Value::int(ordinal))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request::path(): tainted string` — spec § 15's path, with the
    /// matched mount's prefix already removed.
    ///
    /// The strip is ADR 0097 § 4 step 2's and happens before the program runs,
    /// so this reads the remainder rather than computing it: an application
    /// mounted at `/admin` and one mounted at `/` see the same paths, which is
    /// the whole point of the mount table.
    fn nvs_core_request_path(ctx, _args: [0]) {
        let path = inbound_of(ctx, "path")?.path();
        Ok(Value::str(NvsStr::new(path.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request::query(string $name): mixed` — spec § 15's query-string
    /// reader, replacing `$_GET` and `filter_input(INPUT_GET, …)`.
    ///
    /// The parse is [`crate::uri::parse_query`]'s and not a second one; the
    /// module doc owns what that costs per call and why the answer is `mixed`.
    fn nvs_core_request_query(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Request::query expected a `string` for the name, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let parsed = crate::uri::parse_query(inbound_of(ctx, "query")?.query(), "query")?;
        let answer = parsed.get(name.as_bytes()).unwrap_or_else(Value::null);
        // `get` borrows rather than retains, and `parsed` releases every value
        // it holds when it drops at the end of this block — so the one being
        // handed back needs a reference of its own first, and the caller owns
        // exactly that one.
        #[expect(
            unsafe_code,
            reason = "the payload is live: `parsed` still holds its own reference \
                      to it at this point and is dropped after"
        )]
        unsafe {
            answer.retain();
        }
        Ok(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::method_ordinal;
    use crate::router::METHOD;

    /// Every ordinal the parse answers is a case of the roster it claims to be
    /// reading, and `HEAD` is the one token that answers another verb's.
    ///
    /// Asserted against [`METHOD`] rather than against eight written numbers:
    /// what could go wrong is the two lists drifting, and a copy of the
    /// ordinals here would drift with them.
    #[test]
    fn every_parsed_verb_is_a_case_of_the_roster() {
        for verb in ["GET", "OPTIONS", "TRACE", "POST", "PUT", "PATCH", "DELETE"] {
            let ordinal = method_ordinal(verb).expect("a roster verb parses");
            let (name, _) = METHOD
                .cases
                .iter()
                .find(|(_, value)| *value == ordinal)
                .expect("the ordinal names a case");
            assert!(
                name.eq_ignore_ascii_case(verb),
                "`{verb}` parsed to `{name}`"
            );
        }
        assert_eq!(
            method_ordinal("HEAD"),
            method_ordinal("GET"),
            "spec § 15 reports `HEAD` as `Get`, so a `Get`-only route table matches one"
        );
    }

    /// The roster is closed, so a token outside it has no ordinal — including
    /// the lower-case spelling of one inside it, RFC 9110 § 9.1 making a method
    /// case-sensitive.
    #[test]
    fn a_verb_outside_the_roster_has_no_ordinal() {
        for verb in ["CONNECT", "get", "Post", "", "GET "] {
            assert_eq!(method_ordinal(verb), None, "`{verb}` parsed to a case");
        }
    }
}
