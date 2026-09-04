//! `Core\Request` — the request a program is answering, replacing `$_GET`,
//! `$_POST`, `$_COOKIE`, `$_FILES`, `$_REQUEST` and `filter_input`
//! ([ADR 0012](../../../docs/adr/0012-no-superglobals.md)).
//!
//! # What is here, and what is not
//!
//! Nine of
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) § 15's
//! fifteen members: `method`, `isHead`, `path` and `query` — the request *line*,
//! and the one fact reporting a `HEAD` as a `Get` would otherwise lose —
//! `header`, `headers` and `cookie`, the fields that arrived with it, and
//! `body` and `bodyStream`, the two members here that read what arrived
//! **after** all of those — the same [`nvs_runtime::RequestBody`] pulled to its
//! end into one value, or walked a chunk at a time.
//! `files`, `clientIp`, `scheme`, `host`, `mount` and
//! `route` are known gaps of this module rather than of § 15, and each waits on
//! a different thing: `files` on
//! [ADR 0105](../../../docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
//! § 3's third way of reading that same body — as parts —
//! `route`/`mount` on the match `nvs_server` makes once
//! before the handler, and `clientIp`/`scheme`/`host` on
//! `[server] trusted_proxies` and the forwarded-header walk. Those three read a
//! field this module now holds and are still gaps for that reason: which peer is
//! allowed to have asserted one is not this module's to decide.
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
//! # Two members read the body, and the request records which one did
//!
//! Spec § 15 makes `body`, `bodyStream` and `files` exclusive on one request:
//! whichever is called first has consumed the stream, so a later read of any of
//! them is a program bug rather than a small answer. Left unenforced, the
//! second of them would answer *plausibly* — an empty string, or a walk that
//! yields nothing — because that is all an exhausted stream can say, and a
//! program would read it as "the peer sent nothing" about bytes it had already
//! been handed.
//!
//! The record is [`nvs_runtime::Inbound::claim_body`] and not a field here,
//! because what is exclusive is the *request*: that carrier's own doc argues
//! for the shape, and [`claim_body`] below is only this class's wording of the
//! refusal. The claim is taken where the reading is **named** — `bodyStream()`
//! claims when the walk is built, long before an `advance()` moves a byte — so
//! a program that names two readings is refused whether or not it walked
//! either, and a program that names one is never refused its second chunk.
//! `files` joins the same call when it lands.
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
//! `method()` never answers, and the truth is `isHead`'s to carry — a member
//! this member's shape creates rather than one § 15 would otherwise need, and
//! the reason the two are read off the same token in the same file.
//!
//! # One field, or every one of them
//!
//! `header(string $name): ?tainted string` answers **one** field and `headers():
//! array<array<tainted string>>` answers all of them. Both read
//! [`nvs_runtime::Inbound`]'s list of field lines, which holds one entry per
//! *line* — a name the peer sent twice is two entries there and neither is lost,
//! which is the decision that ADR's carrier states and this module is the reader
//! of.
//!
//! **A name matches without regard to case**, RFC 9110 § 5.1 making a field name
//! case-insensitive, so `header("Content-Type")` and `header("content-type")` are
//! one question. `headers()` keys its answer by the **lower-cased** name for the
//! same reason: a program that indexed the map would otherwise have to guess the
//! peer's spelling, and two spellings of one name would be two entries of a map
//! HTTP says has one. The convention is here rather than on the carrier because
//! it is a reading of what arrived, exactly as the verb roster is.
//!
//! **A field the peer sent twice is joined by `header` and kept apart by
//! `headers`**, and that is the whole reason both exist. RFC 9110 § 5.3 defines
//! two field lines of one name as equivalent to one value with the lines joined
//! by a comma in the order received, so joining is that section's own equivalence
//! and not [ADR 0095](../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)'s
//! repair — nothing is dropped and nothing is invented. What joining *does* lose
//! is the line boundary, which matters for a value that may itself contain a
//! comma (`Date` is the standard example), so the exact answer is `headers()`'s
//! and the convenient one is `header`'s. Answering the *first* line was rejected:
//! it is the reading that silently drops what a peer sent, which is the failure
//! mode [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) exists to
//! close.
//!
//! **What they spend.** `header` walks the list once and allocates only the
//! answer; `headers` allocates one array per distinct name plus one string per
//! line, and groups by scanning the names it has already seen, which is
//! quadratic in the number of *distinct* names and bounded by the door's own
//! header-count cap. Neither memoizes, for the reason `query` does not.
//!
//! # A cookie is the `Cookie` field, read
//!
//! `cookie(string $name): ?tainted string` parses the same field lines the two
//! members above read, and the carrier holds **no second field for cookies** —
//! so nothing here can hold a set of cookies that disagrees with the headers the
//! request arrived with, which is the state a parsed-once cache would introduce.
//! The parse costs one walk of the `Cookie` lines per call, as `query`'s does
//! and for the same reason.
//!
//! The name is matched **byte for byte**
//! ([ADR 0095](../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)
//! § 3): no dot, space or bracket is substituted in either direction. That
//! mangling is PHP's `register_globals`-era name repair, it is what
//! CVE-2024-2756 was, and the superglobals it served are what
//! [ADR 0012](../../../docs/adr/0012-no-superglobals.md) deleted.
//!
//! **The prefixes are enforced here as far as the field can show them**, which
//! is [`cookie_of`]'s doc: a `__Host-` name arriving twice is not visible,
//! because a browser holds at most one per host. The other half of § 3 — that
//! the connection was secure — is a known gap waiting on `scheme()`, since the
//! carrier does not hold what the door concluded about the connection.
//! `Core\Response::addCookie` refuses to *write* a cookie that would not be
//! visible, which is the same rule from the end that can see every attribute.
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

use nvs_runtime::{Ctx, Fault, Inbound, NvsArray, NvsStr, ThrownClass, Value};

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
            name: "isHead",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_request_is_head",
            doc: Some(&IS_HEAD_DOC),
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
        CoreMethod {
            name: "header",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_request_header",
            doc: Some(&HEADER_DOC),
        },
        CoreMethod {
            name: "headers",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&HEADER_LINES),
            symbol: "nvs_core_request_headers",
            doc: Some(&HEADERS_DOC),
        },
        CoreMethod {
            name: "cookie",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_request_cookie",
            doc: Some(&COOKIE_DOC),
        },
        CoreMethod {
            name: "body",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_request_body",
            doc: Some(&BODY_DOC),
        },
        CoreMethod {
            name: "bodyStream",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(BODY_STREAM_NAME),
            symbol: "nvs_core_request_body_stream",
            doc: Some(&BODY_STREAM_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `array<tainted string>` — what one key of `headers()`'s answer holds, and so
/// the element type of that member's own `array<…>`.
///
/// A named constant because it is nested one level: the qualifier has to sit on
/// the value a program actually reaches, exactly as
/// [`CoreTy::TaintedStr`]'s own docs argue for `Core\Jwt::verify`, and there is
/// no `tainted array<T>` for it to sit on instead.
const HEADER_LINES: CoreTy = CoreTy::Array(&CoreTy::TaintedStr);

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

/// `Core\Request::isHead`'s reference card — ADR 0117.
const IS_HEAD_DOC: MethodDoc = MethodDoc {
    short: "Whether the peer wrote `HEAD`, which `method` reports as `Get` — the one difference \
            between the two, for a handler that would rather not build a body nothing will read.",
    params: &[],
    ret: "`true` when the request line carried `HEAD`, `false` for every other verb.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request — a CLI program, a scheduled script, a job \
               worker or a test.",
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

/// `Core\Request::header`'s reference card — ADR 0117.
const HEADER_DOC: MethodDoc = MethodDoc {
    short: "One request header by name, matched without regard to case — and where the peer sent \
            the field more than once, its lines joined by `, ` as RFC 9110 § 5.3 defines them to \
            be equivalent.",
    params: &[ParamDoc {
        name: "name",
        desc: "The field name, in any case — `Content-Type` and `content-type` are one question.",
        shape: &[],
    }],
    ret: "The field's value as it arrived, `tainted`, or `null` where the request carried no such \
          field. `headers()` is the answer that keeps two lines of one name apart.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request.",
    }],
};

/// `Core\Request::headers`'s reference card — ADR 0117.
const HEADERS_DOC: MethodDoc = MethodDoc {
    short: "Every header the request carried, keyed by the lower-cased field name, replacing \
            `getallheaders` and the `HTTP_*` half of `$_SERVER`.",
    params: &[],
    ret: "An `array<array<tainted string>>`: one key per distinct field name, holding one entry \
          per field *line* in the order the peer sent them, so a repeated name keeps every value \
          rather than the last. Empty where the request carried no headers at all.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request.",
    }],
};

/// `Core\Request::cookie`'s reference card — ADR 0117.
const COOKIE_DOC: MethodDoc = MethodDoc {
    short: "One cookie by name, matched **byte for byte** — no dot, space or bracket is \
            substituted in either direction, which is what PHP's `$_COOKIE` mangling did and \
            CVE-2024-2756 is.",
    params: &[ParamDoc {
        name: "name",
        desc: "The cookie's name, exactly as it was written — the match is case-sensitive and \
               substitutes nothing.",
        shape: &[],
    }],
    ret: "The cookie's value as it arrived, `tainted` and undecoded, or `null` where the request \
          carried no such cookie. A `__Host-` name that arrived more than once is `null` as well: \
          a browser holds at most one, so two did not come from one.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request.",
    }],
};

/// `Core\Request::body`'s reference card — ADR 0117.
const BODY_DOC: MethodDoc = MethodDoc {
    short: "The whole request body, pulled to its end into one string — the buffered way of \
            reading one, replacing `file_get_contents('php://input')` and the \
            `$HTTP_RAW_POST_DATA` it succeeded.",
    params: &[],
    ret: "Every byte the peer sent, in order, `tainted` and decoded by nothing. Empty where the \
          request carried no body, which is a different fact from a program that is answering no \
          request at all — that one throws.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is not answering a request, or this request's body has already \
                   been read by `bodyStream` or `files` — the three are exclusive on one request.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The body is larger than `[limits] request_body` (8M). The bytes over the bound \
                   are never held: the refusal happens at the chunk that would cross it.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed under the body, or the peer stopped short of the length \
                   it declared.",
        },
    ],
};

/// `Core\Request::bodyStream`'s reference card — ADR 0117.
const BODY_STREAM_DOC: MethodDoc = MethodDoc {
    short: "The request body as a walk over its chunks — the streaming way of reading one, for a \
            body too large to want resident and for a program that can work as the bytes arrive.",
    params: &[],
    ret: "An `Iterable<tainted bytes>` a `foreach` walks once, yielding each chunk as it comes off \
          the wire. A chunk boundary is the wire's and carries no meaning. The walk is empty where \
          the request carried no body.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request, or this request's body has already been \
               read by `body` or `files` — the three are exclusive on one request, and naming this \
               walk is the reading.",
    }],
};

/// `Core\Request::bodyStream`'s answer, as [`CoreTy::Instance`] spells it.
pub(crate) const BODY_STREAM_NAME: &str = r"Core\Request\BodyStream";

/// The symbol behind `Iterable<tainted bytes>::iterate()`, reached by name
/// through this class's method table rather than as a registered member — see
/// [`crate::cursor`] and [`crate::instance`]'s dispatch roster.
pub(crate) const BODY_STREAM_ITERATE_SYMBOL: &str = "nvs_core_request_body_stream_iterate";
/// The symbol behind `Iterator<tainted bytes>::advance()`, which is where the
/// next chunk is pulled off the wire.
pub(crate) const BODY_STREAM_ADVANCE_SYMBOL: &str = "nvs_core_request_body_stream_advance";
/// The symbol behind `Iterator<tainted bytes>::current()`.
pub(crate) const BODY_STREAM_CURRENT_SYMBOL: &str = "nvs_core_request_body_stream_current";

/// [`BODY_STREAM`]'s one slot: the chunk the last `advance()` pulled, which
/// `current()` answers, and `null` before the first one.
const BODY_STREAM_CHUNK: usize = 0;

/// The class `bodyStream` answers with — spec § 15's `Iterable<bytes>`, given
/// the name the registry needs to write it.
///
/// # Decision: it is its own iterator, where [`crate::io::LINES`] is a snapshot
///
/// Every other `Iterable` in `Core` answers `iterate()` with a
/// [`crate::cursor`] over a list it is already holding, because its subject was
/// read whole before the value existed. This one holds nothing: `iterate()`
/// answers the receiver itself and `advance()` pulls one chunk off
/// [`nvs_runtime::RequestBody`], which is the shape `Core\Task\Channel` already
/// takes for the same reason — a walk whose next element does not exist yet.
///
/// **That is the whole difference between this member and `body`.** A snapshot
/// would make `bodyStream` a spelling of `body` with an extra allocation, and
/// then the `[limits] request_body` bound would have to apply to it — which is
/// exactly what ADR 0105 § 3 offers this member as the way *around*. So
/// [`REQUEST_BODY`] is not checked here and nothing accumulates: what the
/// program holds is whatever it does with each chunk, and that is its own
/// decision to make and its own memory limit to make it under.
///
/// **A chunk is copied out, once.** `next_chunk` lends its slice only until the
/// following pull, so the value handed to the loop body is an [`NvsStr`] of its
/// own — a program that keeps one keeps a value the wire cannot revoke. **What
/// it spends:** one chunk, resident, for as long as the loop body holds it.
///
/// # Why it has no members
///
/// [`crate::io::LINES`]'s answer: everything it does is the three names on
/// [`crate::instance`]'s dispatch roster, so it is a *handle* in the sense
/// `registry`'s `a_class_with_slots_has_instance_members_and_the_reverse`
/// names. Spec § 15 writes `bodyStream(): Iterable<bytes>` and no member on the
/// thing it answers with.
pub(crate) const BODY_STREAM: CoreClass = CoreClass {
    name: BODY_STREAM_NAME,
    methods: &[],
    instance: &[],
    slots: &["chunk"],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_request_method" => (nvs_core_request_method as *const ()).cast(),
        "nvs_core_request_is_head" => (nvs_core_request_is_head as *const ()).cast(),
        "nvs_core_request_path" => (nvs_core_request_path as *const ()).cast(),
        "nvs_core_request_query" => (nvs_core_request_query as *const ()).cast(),
        "nvs_core_request_header" => (nvs_core_request_header as *const ()).cast(),
        "nvs_core_request_headers" => (nvs_core_request_headers as *const ()).cast(),
        "nvs_core_request_cookie" => (nvs_core_request_cookie as *const ()).cast(),
        "nvs_core_request_body" => (nvs_core_request_body as *const ()).cast(),
        "nvs_core_request_body_stream" => (nvs_core_request_body_stream as *const ()).cast(),
        BODY_STREAM_ITERATE_SYMBOL => (nvs_core_request_body_stream_iterate as *const ()).cast(),
        BODY_STREAM_ADVANCE_SYMBOL => (nvs_core_request_body_stream_advance as *const ()).cast(),
        BODY_STREAM_CURRENT_SYMBOL => (nvs_core_request_body_stream_current as *const ()).cast(),
        _ => return None,
    })
}

/// Claims this request's body for `member`, or spec § 15's refusal naming the
/// member that already read it.
///
/// One function for the same reason [`inbound_of`] is one: what the readers
/// share is the rule, and [`nvs_runtime::Inbound::claim_body`] is where it
/// lives — this is only the wording, and a second copy of the wording is how
/// two members would come to describe one rule differently.
///
/// **The claim is taken where the reading is named, not where a byte moves.**
/// `bodyStream` claims when the walk is built and pulls a chunk per `advance()`
/// long afterwards, so a claim tied to the first pull would leave a program
/// free to name both readings and only lose on the one it actually took.
///
/// # Errors
///
/// `LogicError` where another of the three members has already read the body.
fn claim_body(ctx: &mut Ctx, member: &'static str) -> Result<(), Fault> {
    let claimed = ctx
        .inbound_mut()
        .expect("the caller reads the request before it claims the body")
        .claim_body(member);
    // No case can reach this: a `.nvst` program answers no request, so
    // `inbound_of` refuses both readers before either reaches the claim.
    // Asserted by `a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other`.
    claimed.map_err(|first| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Request::{member}(): this request's body has already been read by \
                 `Core\\Request::{first}()`. Spec § 15 makes `body`, `bodyStream` and `files` \
                 exclusive on one request, because each of them consumes the stream the other two \
                 would read — so this is refused rather than answered empty, which is all an \
                 exhausted stream could say"
            ),
        )
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

/// Whether `verb` names one of [`crate::router::METHOD`]'s eight cases at all —
/// the question the *door* asks, before an isolate exists.
///
/// ADR 0097 § 2's server refuses a token outside the roster with a `501`
/// (`nvs_server::Reply::not_implemented`), which is what makes
/// [`nvs_core_request_method`]'s closed answer total in practice: the throw it
/// still carries is for the program that reached it another way. This predicate
/// is the roster's one home answering a second question about itself, and
/// deliberately not a copy of the list in the crate that accepts connections.
///
/// `HEAD` is known, on [`method_ordinal`]'s own row: RFC 9110 requires it and
/// ADR 0097 § 7 runs it as a `GET`.
#[must_use]
pub fn is_known_verb(verb: &str) -> bool {
    method_ordinal(verb).is_some()
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

/// Every line the request carried under `name`, joined by `, ` — RFC 9110
/// § 5.3's own equivalence — or `None` where it carried none.
///
/// The comparison is ASCII-case-insensitive on both sides (RFC 9110 § 5.1), and
/// the join preserves arrival order, which is part of the value for every
/// ordered field there is.
fn joined_field(inbound: &Inbound, name: &[u8]) -> Option<Vec<u8>> {
    let mut joined: Option<Vec<u8>> = None;
    for (field, value) in inbound.headers() {
        if !field.as_bytes().eq_ignore_ascii_case(name) {
            continue;
        }
        match &mut joined {
            None => joined = Some(value.to_vec()),
            Some(seen) => {
                seen.extend_from_slice(b", ");
                seen.extend_from_slice(value);
            }
        }
    }
    joined
}

/// The request's field lines grouped by lower-cased name, each group in arrival
/// order, and the groups themselves in the order their names were first seen.
///
/// One entry per *line* inside a group rather than a joined value: this is the
/// half of the pair that keeps two lines of one name apart, which the module doc
/// argues is the only reading `joined_field`'s answer cannot be recovered from.
///
/// A linear scan of the names already seen rather than a hash map, because the
/// number of distinct field names on one request is small and bounded at the
/// door, and a map would cost an allocation per group to save a comparison per
/// line.
fn grouped_fields<'a>(inbound: &'a Inbound) -> Vec<(String, Vec<&'a [u8]>)> {
    let mut groups: Vec<(String, Vec<&'a [u8]>)> = Vec::new();
    for (field, value) in inbound.headers() {
        let name = field.to_ascii_lowercase();
        match groups.iter_mut().find(|(seen, _)| *seen == name) {
            Some((_, lines)) => lines.push(value),
            None => groups.push((name, vec![value])),
        }
    }
    groups
}

/// The field a request's cookies arrive under, matched as every field name is.
const COOKIE_FIELD: &[u8] = b"cookie";

/// ADR 0095 § 3's stricter prefix, on the read side — the write side's spelling
/// of it is `crate::response`'s `HOST_PREFIX`, and the two are one rule read
/// from its two ends.
const HOST_PREFIX: &[u8] = b"__Host-";

/// `bytes` without the ASCII spaces and tabs it opens with.
///
/// The delimiter between two cookie pairs is `"; "` (RFC 6265 § 4.2.1), so the
/// padding belongs to the delimiter and not to the name that follows it. Only
/// the leading side is trimmed: anything after the `=` is the value, and a
/// trailing space inside one is a byte the peer sent.
fn without_leading_padding(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| *byte != b' ' && *byte != b'\t')
        .unwrap_or(bytes.len());
    &bytes[start..]
}

/// Every value the request's `Cookie` field lines carry under exactly `name`, in
/// arrival order.
///
/// **The comparison is byte for byte**, which is ADR 0095 § 3 and the whole of
/// what CVE-2024-2756 was: PHP substituted a dot and a space in a cookie name
/// for an underscore, so a name a browser refused to give the `__Host-` meaning
/// could be mangled into one that had it. Nothing is substituted here in either
/// direction, and a name that differs by one byte is a different cookie.
///
/// A pair with no `=` in it is skipped rather than read as a name with an empty
/// value — RFC 6265 § 5.4's own rule for parsing a `Cookie` field, and the same
/// refusal-not-repair the rest of this module makes.
fn cookie_lines<'a>(inbound: &'a Inbound, name: &[u8]) -> Vec<&'a [u8]> {
    let mut found = Vec::new();
    for (field, value) in inbound.headers() {
        if !field.as_bytes().eq_ignore_ascii_case(COOKIE_FIELD) {
            continue;
        }
        for pair in value.split(|byte| *byte == b';') {
            let pair = without_leading_padding(pair);
            let Some(equals) = pair.iter().position(|byte| *byte == b'=') else {
                continue;
            };
            if &pair[..equals] == name {
                found.push(&pair[equals + 1..]);
            }
        }
    }
    found
}

/// The cookie the program asked for, or `None` where the request carries none it
/// is allowed to see.
///
/// **A `__Host-` name that arrived twice is not visible**, which is ADR 0095
/// § 3's "a non-conforming cookie carrying the prefix is not visible on read"
/// stated over the one non-conformance a `Cookie` field can actually show. The
/// prefix means host-locked and `Path=/`, so a conforming browser holds at most
/// one of them per host; two lines are either a client that is not one or a
/// shadowing attempt from a name that was supposed to be unshadowable, and
/// picking between them is the arrangement the prefix exists to end. The other
/// half of § 3 — that the connection carrying a `__Host-` or `__Secure-` cookie
/// was secure — is not decidable here and is a known gap of this module, waiting
/// on `scheme()`, because the carrier does not hold what the door concluded
/// about the connection. `Core\Response::addCookie` refuses to *write* a
/// non-conforming one either way, which is the same rule from the other end.
///
/// Otherwise the **first** line wins. RFC 6265 § 5.4 has a user agent send the
/// most specific cookie first, so the first is the one that applies most closely
/// to this path; that is reading the peer's own order rather than choosing among
/// answers, exactly as [`joined_field`] preserves it.
fn cookie_of<'a>(inbound: &'a Inbound, name: &[u8]) -> Option<&'a [u8]> {
    let lines = cookie_lines(inbound, name);
    if name.starts_with(HOST_PREFIX) && lines.len() > 1 {
        return None;
    }
    lines.first().copied()
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
    /// `Core\Request::isHead(): bool` — ADR 0097 § 7's other half, and the only
    /// member that can tell a `HEAD` request from the `Get` [`method`] reports.
    ///
    /// A byte comparison against the token the peer wrote rather than a second
    /// reading of the roster: [`method_ordinal`] answers `Get`'s ordinal for
    /// `HEAD` deliberately, so by the time a verb is a case the difference is
    /// gone. `HEAD` is a case-sensitive token like every other (RFC 9110 § 9.1),
    /// and a lower-cased one never reaches here — the door refuses it with a
    /// `501` ([`is_known_verb`]).
    ///
    /// The server discards the body of a `HEAD` answer either way, so nothing a
    /// program does with this changes what the peer receives. What it saves is
    /// the work of producing a body that will be thrown away, which for a
    /// handler that renders a page is the whole request.
    fn nvs_core_request_is_head(ctx, _args: [0]) {
        Ok(Value::bool(inbound_of(ctx, "isHead")?.method() == "HEAD"))
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

nvs_runtime::nvs_helper! {
    /// `Core\Request::header(string $name): ?tainted string` — spec § 15's
    /// single-field reader, replacing the `HTTP_*` half of `$_SERVER` and
    /// `filter_input(INPUT_SERVER, …)`.
    ///
    /// The case rule and the join rule are the module doc's, and both are
    /// readings of what arrived rather than properties of the carrier —
    /// [`nvs_runtime::Inbound`] holds the lines and interprets none of them.
    fn nvs_core_request_header(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Request::header expected a `string` for the name, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let inbound = inbound_of(ctx, "header")?;
        Ok(match joined_field(inbound, name.as_bytes()) {
            None => Value::null(),
            Some(value) => Value::str(NvsStr::new(&value)),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request::headers(): array<array<tainted string>>` — spec § 15's
    /// whole-header reader, replacing `getallheaders`.
    ///
    /// Nested rather than one string per name, because a field the peer sent
    /// twice is two values and a map of strings could only hold one of them.
    /// The qualifier rides on the innermost value for the same reason
    /// `Core\Jwt::verify`'s does: there is no `tainted array<T>`, so a shape
    /// that put the strings any deeper would drop it.
    fn nvs_core_request_headers(ctx, _args: [0]) {
        let inbound = inbound_of(ctx, "headers")?;
        let mut out = NvsArray::new();
        for (name, lines) in grouped_fields(inbound) {
            let mut values = NvsArray::new();
            for line in lines {
                values.append(Value::str(NvsStr::new(line)));
            }
            out.set(NvsStr::new(name.as_bytes()), Value::array(values));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request::cookie(string $name): ?tainted string` — spec § 15's
    /// cookie reader, replacing `$_COOKIE` and `filter_input(INPUT_COOKIE, …)`.
    ///
    /// The parse is [`cookie_lines`]'s and the visibility rule is
    /// [`cookie_of`]'s; both are ADR 0095 § 3, whose other end is
    /// `Core\Response::addCookie`'s refusal to write what would not be visible
    /// here. There is no second field on the carrier for cookies: they are the
    /// `Cookie` header, read as such, so nothing can hold a set of cookies that
    /// disagrees with the headers the request arrived with.
    ///
    /// A value is handed back **undecoded**, which is what makes it the same
    /// bytes `addCookie` wrote: that member percent-encodes nothing and refuses
    /// a value it could not write verbatim, so a decode here would be a
    /// substitution in the direction ADR 0095 § 3 closes.
    fn nvs_core_request_cookie(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Request::cookie expected a `string` for the name, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let inbound = inbound_of(ctx, "cookie")?;
        Ok(match cookie_of(inbound, name.as_bytes()) {
            None => Value::null(),
            Some(value) => Value::str(NvsStr::new(value)),
        })
    }
}

/// [ADR 0105](../../../docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
/// § 5's `[limits] request_body` default, as a constant until that row exists.
///
/// The twin of `nvs_server::body::UPLOAD_TOTAL` and deliberately the smaller of
/// the two: this one bounds bytes *parsed into memory*, which are resident and
/// are paid once per in-flight request, while that one bounds the total of a
/// body streamed past memory entirely. § 5's table is the home of both numbers
/// and of why one cap could not have governed both.
///
/// ADR 0105 § 2's buffered multipart form fields are charged against it too, by
/// [`crate::multipart`], for the reason that row gives: a form field's text is
/// bytes parsed into memory, which is exactly what this cap means.
pub(crate) const REQUEST_BODY: usize = 8 * 1024 * 1024;

nvs_runtime::nvs_helper! {
    /// `Core\Request::body(): tainted string` — spec § 15's whole-body reader,
    /// replacing `file_get_contents('php://input')`.
    ///
    /// ADR 0105 § 3's first of three ways to read a body, and the only one that
    /// ends with all of it resident — which is why it is the one [`REQUEST_BODY`]
    /// bounds. The pull is [`nvs_runtime::RequestBody::next_chunk`]'s, so it
    /// parks this isolate rather than a thread, and chunk boundaries are the
    /// wire's and mean nothing here: every chunk lands in the same buffer.
    ///
    /// **The bound is checked before the copy, not after.** A chunk that would
    /// carry the total past [`REQUEST_BODY`] is refused while it is still the
    /// supplier's own borrowed slice, so what this member holds never exceeds
    /// the number it was given. A check made after appending would be a report
    /// about memory already spent, which ADR 0105 § 5 argues is not a bound at
    /// all.
    ///
    /// **Nothing is reserved from `Content-Length`.** The obvious shape reads
    /// the declared length and allocates it up front, and it hands a peer a line
    /// of its own: a request declaring 8M and sending one byte would cost the
    /// whole cap, per in-flight request, for nothing. So the buffer grows by
    /// doubling against bytes that actually arrived, and a lie costs what it
    /// delivers.
    ///
    /// **What it spends:** the body's own bytes twice at the peak — the buffer
    /// they arrive in, plus the [`NvsStr`] copied out of it — and nothing at all
    /// once the call returns. Both are bounded by [`REQUEST_BODY`] and both are
    /// O(in-flight).
    fn nvs_core_request_body(ctx, _args: [0]) {
        // Asked before the body is, so the module doc's two facts stay apart:
        // "the request sent nothing" is the empty answer below, and "no request
        // arrived" is this throw.
        inbound_of(ctx, "body")?;
        // Before the first pull, and before the empty answer below: a second
        // reading is refused whether or not this request carried any bytes,
        // because what spec § 15 makes exclusive is the reading.
        claim_body(ctx, "body")?;
        let inbound = ctx
            .inbound_mut()
            .expect("the read above refuses a context that is answering no request");
        let mut whole: Vec<u8> = Vec::new();
        if let Some(body) = inbound.body() {
            loop {
                match body.next_chunk() {
                    Ok(None) => break,
                    Ok(Some(chunk)) => {
                        if whole.len().saturating_add(chunk.len()) > REQUEST_BODY {
                            // No case can reach this: a `.nvst` program answers
                            // no request, so it has no body to send over the
                            // bound. Asserted by
                            // `the_request_body_cap_is_the_last_body_read_and_the_first_one_refused`
                            // below, on both sides of the bound.
                            return Err(Fault::thrown(format!(
                                "Core\\Request::body(): this request's body is larger than \
                                 `[limits] request_body` ({REQUEST_BODY} bytes), so it is refused \
                                 rather than held. That directive bounds what a body may cost in \
                                 memory; a body bigger than it is one to stream rather than to \
                                 read whole"
                            )));
                        }
                        whole.extend_from_slice(chunk);
                    }
                    Err(why) => {
                        // No case can reach this either, and for the same
                        // reason: a connection has to exist before it can fail
                        // under a body. Asserted by
                        // `a_body_that_fails_mid_stream_throws_rather_than_answering_its_prefix`.
                        return Err(Fault::thrown_as(
                            ThrownClass::Io,
                            format!(
                                "Core\\Request::body(): the body did not arrive whole — {why}"
                            ),
                        ));
                    }
                }
            }
        }
        Ok(Value::str(NvsStr::new(&whole)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request::bodyStream(): Iterable<tainted bytes>` — spec § 15's
    /// streaming body reader, and ADR 0105 § 3's second of three ways.
    ///
    /// **It reads nothing.** The pull is `advance()`'s, one chunk at a time, so
    /// this call is the walk being *named* rather than taken — which is what
    /// makes it the member for a body that must not be resident whole.
    /// [`BODY_STREAM`]'s own docs are the argument for that shape and for what
    /// each chunk costs.
    ///
    /// The refusal is still checked here, so that a program answering no
    /// request learns it where it asked rather than at the first `foreach`: an
    /// empty walk and no request at all are the two facts this module's doc
    /// keeps apart.
    fn nvs_core_request_body_stream(ctx, _args: [0]) {
        inbound_of(ctx, "bodyStream")?;
        // The claim is here rather than in `advance()`, which is where the
        // bytes move: naming the walk is the reading, and a program that named
        // two of them and walked neither has still written the bug § 15 refuses.
        claim_body(ctx, "bodyStream")?;
        Ok(crate::instance::build(&BODY_STREAM, [Value::null()]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<tainted bytes>::iterate(): Iterator<tainted bytes>` — the
    /// stream itself, because a body has no snapshot to walk.
    ///
    /// [`crate::channel`]'s shape rather than [`crate::io`]'s: the receiver's
    /// transferred reference is handed straight back out rather than released,
    /// so nothing is allocated and the cursor *is* the stream.
    fn nvs_core_request_body_stream_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &BODY_STREAM, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted bytes>::advance(): bool` — pulls the next chunk off
    /// the wire, answering `false` at the end of the body.
    ///
    /// This is the only place a `foreach` over a body suspends, and it parks
    /// the isolate rather than a thread, exactly as `body`'s loop does.
    ///
    /// The chunk is copied into the receiver's slot while it is still the
    /// supplier's borrowed slice, because [`nvs_runtime::RequestBody`] lends it
    /// only until the following pull. `current()` then hands that copy out with
    /// a reference of its own, so a loop body that keeps a chunk keeps a value
    /// nothing else can invalidate.
    fn nvs_core_request_body_stream_advance(ctx, args: [1]) {
        let stepped = body_stream_step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted bytes>::current(): tainted bytes` — the chunk the last
    /// `advance()` pulled.
    fn nvs_core_request_body_stream_current(_ctx, args: [1]) {
        let read = body_stream_chunk(args[0]);
        crate::cursor::consume(args[0]);
        read
    }
}

/// [`BODY_STREAM_ADVANCE_SYMBOL`]'s body: one pull, stored in the receiver's
/// slot, and whether there was anything to store.
///
/// A slot cleared to `null` at the end of the walk rather than left holding the
/// last chunk: the loop is over, so keeping it would hold a chunk's worth of a
/// request's memory for as long as the program held the stream value, and that
/// is a cost with nothing to buy.
///
/// # Errors
///
/// `LogicError` where the context is answering no request — the same refusal
/// [`inbound_of`] writes everywhere else — and `IOError` where the connection
/// failed under the body, which is a short walk this member refuses to report
/// as a complete one.
fn body_stream_step(ctx: &mut Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &BODY_STREAM, member)?;
    // No case can reach this: a `.nvst` program answers no request, so it can
    // hold no stream to advance. Asserted by
    // `a_body_stream_yields_the_chunks_the_wire_delivered`, which drives the
    // three names a `foreach` drives.
    inbound_of(ctx, "bodyStream")?;
    let inbound = ctx
        .inbound_mut()
        .expect("the read above refuses a context that is answering no request");
    let pulled = match inbound.body() {
        None => None,
        Some(body) => match body.next_chunk() {
            Ok(chunk) => chunk.map(NvsStr::new),
            Err(why) => {
                // No case can reach this either, for `body`'s reason: a
                // connection has to exist before it can fail under a body.
                // Asserted by `a_body_stream_that_fails_mid_walk_throws_rather_than_ending`.
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!("Core\\Request::bodyStream(): the body did not arrive whole — {why}"),
                ));
            }
        },
    };
    let more = pulled.is_some();
    let held = pulled.map_or_else(Value::null, Value::bytes);
    crate::instance::set_slot(receiver, BODY_STREAM_CHUNK, held);
    Ok(Value::bool(more))
}

/// [`BODY_STREAM_CURRENT_SYMBOL`]'s body: the slot [`body_stream_step`] last
/// wrote, retained, because the receiver keeps it until the next pull.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not a stream — only a bug in this
/// crate can produce one, the receiver having been checked at compile time.
fn body_stream_chunk(value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::CURRENT;
    let receiver = crate::instance::receiver(value, &BODY_STREAM, member)?;
    let held = crate::instance::slot(receiver, BODY_STREAM_CHUNK);
    #[expect(
        unsafe_code,
        reason = "the slot keeps its reference until the next `advance`, so the \
                  value handed back needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

#[cfg(test)]
mod tests {
    use super::{
        REQUEST_BODY, cookie_of, grouped_fields, joined_field, method_ordinal,
        nvs_core_request_body, nvs_core_request_body_stream, nvs_core_request_body_stream_advance,
        nvs_core_request_body_stream_current, nvs_core_request_body_stream_iterate,
    };
    use crate::router::METHOD;
    use nvs_runtime::{Ctx, Inbound, RequestBody, Value};

    /// A [`RequestBody`] that hands back a fixed list of chunks and then ends —
    /// or, where `fails_at` names a pull, fails at that one instead, which is
    /// the connection dying under a body that had already started arriving.
    struct Chunks {
        chunks: Vec<Vec<u8>>,
        at: usize,
        fails_at: Option<usize>,
    }

    impl Chunks {
        fn of(pieces: &[&[u8]]) -> Self {
            Self {
                chunks: pieces.iter().map(|piece| piece.to_vec()).collect(),
                at: 0,
                fails_at: None,
            }
        }

        fn failing_at(pieces: &[&[u8]], pull: usize) -> Self {
            Self {
                fails_at: Some(pull),
                ..Self::of(pieces)
            }
        }
    }

    impl RequestBody for Chunks {
        fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>> {
            if self.fails_at == Some(self.at) {
                return Err("the connection failed under it".into());
            }
            let at = self.at;
            if at >= self.chunks.len() {
                return Ok(None);
            }
            self.at = at + 1;
            Ok(Some(&self.chunks[at]))
        }
    }

    /// A context answering a request, carrying `body` where there is one.
    fn answering(body: Option<Chunks>) -> Ctx {
        let mut inbound = Inbound::new("POST", "/", "");
        if let Some(body) = body {
            inbound.set_body(Box::new(body));
        }
        let mut ctx = Ctx::buffered();
        ctx.set_inbound(inbound);
        ctx
    }

    /// An `Inbound` carrying `lines` as its header field lines and nothing
    /// interesting on its request line.
    fn carrying(lines: &[(&str, &str)]) -> Inbound {
        let mut inbound = Inbound::new("GET", "/", "");
        for (name, value) in lines {
            inbound.push_header(name, value.as_bytes());
        }
        inbound
    }

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

    /// A field name is case-insensitive both ways round — the spelling the peer
    /// wrote and the spelling the program asked for are independent, which is
    /// RFC 9110 § 5.1 and the one rule a caller would otherwise have to guess.
    #[test]
    fn a_field_name_matches_without_regard_to_case() {
        let inbound = carrying(&[("Content-Type", "text/html")]);
        for asked in [
            "Content-Type",
            "content-type",
            "CONTENT-TYPE",
            "cOnTeNt-TyPe",
        ] {
            assert_eq!(
                joined_field(&inbound, asked.as_bytes()).as_deref(),
                Some(&b"text/html"[..]),
                "`{asked}` did not match the field the peer wrote"
            );
        }
        assert_eq!(
            joined_field(&carrying(&[("content-type", "text/html")]), b"Content-Type").as_deref(),
            Some(&b"text/html"[..]),
            "the peer's own spelling is not what the match is made against"
        );
    }

    /// A repeated field joins in arrival order and loses nothing, and the same
    /// request read through the other member keeps the lines apart — the two
    /// answers are the same set of values in two shapes, which is the whole
    /// reason both members exist.
    #[test]
    fn a_repeated_field_is_joined_in_order_and_kept_apart_beside_it() {
        let inbound = carrying(&[
            ("X-Forwarded-For", "203.0.113.1"),
            ("Accept", "text/html"),
            ("x-forwarded-for", "198.51.100.7"),
            ("X-FORWARDED-FOR", "192.0.2.9"),
        ]);
        assert_eq!(
            joined_field(&inbound, b"x-forwarded-for").as_deref(),
            Some(&b"203.0.113.1, 198.51.100.7, 192.0.2.9"[..]),
            "the three lines did not join in the order the peer sent them"
        );
        let grouped = grouped_fields(&inbound);
        assert_eq!(
            grouped,
            vec![
                (
                    "x-forwarded-for".to_owned(),
                    vec![&b"203.0.113.1"[..], b"198.51.100.7", b"192.0.2.9"],
                ),
                ("accept".to_owned(), vec![&b"text/html"[..]]),
            ],
            "three spellings of one name are one group, keyed lower-cased, in first-seen order"
        );
    }

    /// A field that did not arrive is absent, and a field that arrived empty is
    /// present and empty — the two facts this whole class refuses to collapse,
    /// asked one level down from `Core\Request`'s own "there is no request".
    #[test]
    fn a_field_that_did_not_arrive_is_absent_rather_than_empty() {
        let inbound = carrying(&[("X-Trace", "")]);
        assert_eq!(
            joined_field(&inbound, b"x-trace").as_deref(),
            Some(&b""[..]),
            "a field sent empty is a field the peer sent"
        );
        assert_eq!(
            joined_field(&inbound, b"x-absent"),
            None,
            "a field the request never carried has no value at all"
        );
        assert_eq!(
            joined_field(&inbound, b""),
            None,
            "the empty name matches no field, rather than the first one"
        );
        assert!(
            grouped_fields(&carrying(&[])).is_empty(),
            "a request carrying no headers groups to nothing"
        );
    }

    /// A cookie name is matched byte for byte: nothing is substituted in either
    /// direction, so the four spellings PHP's mangling collapsed into one are
    /// four cookies here. This is CVE-2024-2756 asked as a test — the mangling
    /// is what let a name a browser would not give the `__Host-` meaning become
    /// one that had it.
    #[test]
    fn a_cookie_name_is_matched_byte_for_byte() {
        let inbound = carrying(&[("Cookie", "a.b=dotted; a_b=scored; a b=spaced; ab=bare")]);
        for (name, value) in [
            ("a.b", "dotted"),
            ("a_b", "scored"),
            ("a b", "spaced"),
            ("ab", "bare"),
        ] {
            assert_eq!(
                cookie_of(&inbound, name.as_bytes()),
                Some(value.as_bytes()),
                "`{name}` did not read the cookie of exactly that name"
            );
        }
        assert_eq!(
            cookie_of(&inbound, b"A.B"),
            None,
            "the match is case-sensitive, a cookie name being bytes"
        );
    }

    /// The parse follows the field's own grammar and stops there: pairs are
    /// separated by `;`, the delimiter's padding is not part of a name, a value
    /// is every byte after the first `=` and is decoded by nothing, and a pair
    /// with no `=` in it is skipped rather than read as an empty value.
    #[test]
    fn a_cookie_value_is_every_byte_after_the_first_equals() {
        let inbound = carrying(&[(
            "cookie",
            "session=a=b=c; empty=; flag; padded=  spaced out  ; encoded=%2F%2F",
        )]);
        assert_eq!(
            cookie_of(&inbound, b"session"),
            Some(&b"a=b=c"[..]),
            "only the first `=` separates a name from a value"
        );
        assert_eq!(
            cookie_of(&inbound, b"empty"),
            Some(&b""[..]),
            "a cookie sent empty is a cookie the request carried"
        );
        assert_eq!(
            cookie_of(&inbound, b"flag"),
            None,
            "a pair with no `=` is not a cookie, per RFC 6265 § 5.4"
        );
        assert_eq!(
            cookie_of(&inbound, b"padded"),
            Some(&b"  spaced out  "[..]),
            "the padding trimmed belongs to the delimiter, never to a value"
        );
        assert_eq!(
            cookie_of(&inbound, b"encoded"),
            Some(&b"%2F%2F"[..]),
            "a value is handed back undecoded, as `addCookie` wrote it"
        );
        assert_eq!(
            cookie_of(&inbound, b"absent"),
            None,
            "a cookie the request never carried has no value at all"
        );
    }

    /// A repeated name reads as the first line — the user agent's own order,
    /// most specific first — unless it carries `__Host-`, which a conforming
    /// browser holds at most one of per host, so two of them are not a browser's
    /// and neither is visible. ADR 0095 § 3, and the read end of the refusal
    /// `Core\Response::addCookie` makes on write.
    #[test]
    fn a_repeated_host_prefixed_cookie_is_not_visible_and_an_ordinary_one_is_the_first() {
        let ordinary = carrying(&[
            ("Cookie", "seen=first; seen=second"),
            ("Cookie", "seen=third"),
        ]);
        assert_eq!(
            cookie_of(&ordinary, b"seen"),
            Some(&b"first"[..]),
            "the first line the peer sent is the most specific one"
        );
        let shadowed = carrying(&[("Cookie", "__Host-id=real; __Host-id=planted")]);
        assert_eq!(
            cookie_of(&shadowed, b"__Host-id"),
            None,
            "two `__Host-` cookies of one name did not come from a browser"
        );
        let single = carrying(&[("Cookie", "__Host-id=real; __Secure-t=a; __Secure-t=b")]);
        assert_eq!(
            cookie_of(&single, b"__Host-id"),
            Some(&b"real"[..]),
            "one `__Host-` cookie is an ordinary one to read"
        );
        assert_eq!(
            cookie_of(&single, b"__Secure-t"),
            Some(&b"a"[..]),
            "`__Secure-` says nothing about `Path`, so two of them are legitimate"
        );
    }

    /// A body that arrived in pieces is one value, and where the pieces fell is
    /// the wire's business: the split below lands inside a multi-byte character,
    /// so a member that decoded, trimmed or measured per chunk would answer
    /// something the peer never sent. Beside it, the module doc's second fact —
    /// a request that carried no body answers empty rather than throwing, which
    /// is what makes the *first* fact worth a refusal.
    #[test]
    fn a_body_is_every_chunk_joined_and_a_chunk_boundary_means_nothing() {
        let whole = "héllo — a body split mid-character";
        let bytes = whole.as_bytes();
        let cut = whole.find('—').expect("the dash is in the subject") + 1;
        let mut arriving = answering(Some(Chunks::of(&[&bytes[..cut], &bytes[cut..]])));
        let answer = nvs_runtime::call(nvs_core_request_body, &mut arriving, &[])
            .expect("a body that arrives whole is read whole");
        assert_eq!(
            answer.as_text(),
            Some(whole),
            "a chunk boundary is the wire's and means nothing to a reader"
        );

        let mut bodiless = answering(None);
        let nothing = nvs_runtime::call(nvs_core_request_body, &mut bodiless, &[])
            .expect("a request that carried no body is still a request");
        assert_eq!(
            nothing.as_text(),
            Some(""),
            "\"the request sent nothing\" is an answer, and only \"no request\" is a throw"
        );

        #[expect(
            unsafe_code,
            reason = "each call transferred the reference it answered"
        )]
        unsafe {
            answer.release();
            nothing.release();
        }
    }

    /// ADR 0105 § 5's cap, named on both sides: a body of exactly
    /// `[limits] request_body` is read, and the same body plus one byte is
    /// refused. A member that stopped one byte early — or one late — prints
    /// plausibly against either half on its own.
    ///
    /// The throw's own message is not read here. Doing so needs an exception
    /// class table installed on the context first, which the playbook's
    /// `Ctx::pending_slot` bullet owns; what this asserts is the boundary, and
    /// the boundary is where the member can be wrong.
    #[test]
    fn the_request_body_cap_is_the_last_body_read_and_the_first_one_refused() {
        let full = vec![b'x'; REQUEST_BODY];
        let mut at_the_bound = answering(Some(Chunks::of(&[&full[..]])));
        let answer = nvs_runtime::call(nvs_core_request_body, &mut at_the_bound, &[])
            .expect("a body of exactly the cap has not crossed it");
        assert_eq!(
            answer.as_text().map(str::len),
            Some(REQUEST_BODY),
            "the last body inside the bound is read whole"
        );
        #[expect(unsafe_code, reason = "the call transferred the reference it answered")]
        unsafe {
            answer.release();
        }

        let mut over_it = answering(Some(Chunks::of(&[&full[..], &b"x"[..]])));
        assert!(
            nvs_runtime::call(nvs_core_request_body, &mut over_it, &[]).is_err(),
            "one byte past `[limits] request_body` is still past it"
        );
    }

    /// A body that stops short is a throw and not a shorter body. `next_chunk`'s
    /// `Err` ends the stream, so what had arrived before it is a prefix of what
    /// the peer meant to send — and answering a prefix is exactly the
    /// silent-wrong-answer this module's refusals exist to close.
    #[test]
    fn a_body_that_fails_mid_stream_throws_rather_than_answering_its_prefix() {
        let mut cut_off = answering(Some(Chunks::failing_at(&[&b"the first half"[..]], 1)));
        assert!(
            nvs_runtime::call(nvs_core_request_body, &mut cut_off, &[]).is_err(),
            "a connection that failed under a body did not deliver one"
        );
    }

    /// Walks a stream exactly as `foreach` walks one — `iterate()` once, then an
    /// `advance()`/`current()` pair per element, each call retained because the
    /// callee consumes its receiver — and hands back what `current()` answered.
    ///
    /// The `Err` is the throw a member raised, so a caller can assert on the
    /// walk failing rather than only on it ending.
    fn walked(ctx: &mut Ctx, stream: Value) -> Result<Vec<Vec<u8>>, i32> {
        /// The retain a virtual call's receiver owes — see [`crate::cursor`].
        fn lend(value: Value) {
            #[expect(
                unsafe_code,
                reason = "each of the three names consumes a reference, so the \
                          driver holds one of its own and retains per call — \
                          exactly what `nvs_ir::lower::control`'s loop emits"
            )]
            unsafe {
                value.retain();
            }
        }
        lend(stream);
        let cursor = nvs_runtime::call(nvs_core_request_body_stream_iterate, ctx, &[stream])
            .expect("a stream is its own iterator, so naming the walk cannot fail");
        let mut seen = Vec::new();
        let walk = loop {
            lend(cursor);
            match nvs_runtime::call(nvs_core_request_body_stream_advance, ctx, &[cursor]) {
                Err(why) => break Err(why),
                Ok(more) if more.as_bool() != Some(true) => break Ok(()),
                Ok(_) => {}
            }
            lend(cursor);
            match nvs_runtime::call(nvs_core_request_body_stream_current, ctx, &[cursor]) {
                Err(why) => break Err(why),
                Ok(chunk) => {
                    seen.push(chunk.as_bytes().expect("a chunk is `bytes`").to_vec());
                    #[expect(
                        unsafe_code,
                        reason = "`current` transferred the reference it answered"
                    )]
                    unsafe {
                        chunk.release();
                    }
                }
            }
        };
        #[expect(
            unsafe_code,
            reason = "the driver owns the reference it was handed and the one \
                      `iterate` answered with, and both are done with here"
        )]
        unsafe {
            cursor.release();
            stream.release();
        }
        walk.map(|()| seen)
    }

    /// A stream yields the chunks the wire delivered, in order and unjoined —
    /// the whole difference from `body`, which is handed the same two pieces and
    /// answers one value. Beside it, the walk that yields nothing: a request
    /// that carried no body is still a request, and an empty walk is what says
    /// so.
    ///
    /// The receiver is driven by hand rather than by a `.nvst` `foreach`,
    /// because a case is a program with no request in front of it — the
    /// `ASSERTED_OFF_THE_CORPUS` reading `conformance_coverage.rs` owns.
    #[test]
    fn a_body_stream_yields_the_chunks_the_wire_delivered() {
        let pieces: &[&[u8]] = &[b"h\xc3\xa9llo \xe2\x80", b"\x94 and the rest"];
        let mut arriving = answering(Some(Chunks::of(pieces)));
        let stream = nvs_runtime::call(nvs_core_request_body_stream, &mut arriving, &[])
            .expect("a request that arrived can be streamed");
        assert_eq!(
            walked(&mut arriving, stream).expect("a body that arrives whole walks whole"),
            vec![pieces[0].to_vec(), pieces[1].to_vec()],
            "a chunk is yielded as it came off the wire, boundary and all"
        );

        let mut bodiless = answering(None);
        let empty = nvs_runtime::call(nvs_core_request_body_stream, &mut bodiless, &[])
            .expect("a request that carried no body is still a request");
        assert!(
            walked(&mut bodiless, empty)
                .expect("no body is no chunks")
                .is_empty(),
            "\"the request sent nothing\" is an empty walk, and only \"no request\" is a throw"
        );
    }

    /// Spec § 15's exclusivity, asked in both directions and on a request with
    /// no body at all: whichever of the two readings a program takes first is
    /// the one that has the body, and the other is refused rather than answered
    /// empty. Both directions matter, because a record kept by one member would
    /// pass the direction it was written for and fail the other.
    ///
    /// The bodiless request is the case that says what the rule is *about*: no
    /// bytes were consumed either way, so a claim tied to the stream rather than
    /// to the reading would let the second call through on exactly the requests
    /// where the empty answer is most convincing.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it is
    /// refused by `inbound_of` before either member reaches the claim.
    #[test]
    fn a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other() {
        let mut buffered = answering(Some(Chunks::of(&[&b"a body"[..]])));
        let answer = nvs_runtime::call(nvs_core_request_body, &mut buffered, &[])
            .expect("the first reading of a body is the one that gets it");
        #[expect(unsafe_code, reason = "the call transferred the reference it answered")]
        unsafe {
            answer.release();
        }
        assert!(
            nvs_runtime::call(nvs_core_request_body_stream, &mut buffered, &[]).is_err(),
            "`bodyStream` after `body` is the program bug § 15 names, not an empty walk"
        );

        let mut streamed = answering(Some(Chunks::of(&[&b"a body"[..]])));
        let stream = nvs_runtime::call(nvs_core_request_body_stream, &mut streamed, &[])
            .expect("naming the walk is the reading, and it is the first one here");
        assert_eq!(
            walked(&mut streamed, stream).expect("the claimed walk is the one that works"),
            vec![b"a body".to_vec()],
            "the member that claimed the body is the one that reads it"
        );
        assert!(
            nvs_runtime::call(nvs_core_request_body, &mut streamed, &[]).is_err(),
            "`body` after `bodyStream` is refused in the same direction as its twin"
        );

        let mut bodiless = answering(None);
        let nothing = nvs_runtime::call(nvs_core_request_body, &mut bodiless, &[])
            .expect("a request that carried no body is still a request");
        #[expect(unsafe_code, reason = "the call transferred the reference it answered")]
        unsafe {
            nothing.release();
        }
        assert!(
            nvs_runtime::call(nvs_core_request_body_stream, &mut bodiless, &[]).is_err(),
            "what is exclusive is the reading, so an empty body is claimed like any other"
        );
    }

    /// A walk that fails mid-body throws rather than ending, which is
    /// `a_body_that_fails_mid_stream_throws_rather_than_answering_its_prefix`'s
    /// property on the streaming side: `advance()` answering `false` means the
    /// body is over, so reporting a dead connection that way would tell a loop
    /// it had seen everything the peer sent. The chunks before the failure are
    /// still yielded — they did arrive — and the throw lands where it happened.
    #[test]
    fn a_body_stream_that_fails_mid_walk_throws_rather_than_ending() {
        let mut cut_off = answering(Some(Chunks::failing_at(&[&b"the first half"[..]], 1)));
        let stream = nvs_runtime::call(nvs_core_request_body_stream, &mut cut_off, &[])
            .expect("the failure is the wire's, and it has not happened yet");
        assert!(
            walked(&mut cut_off, stream).is_err(),
            "a connection that failed under a body did not deliver the end of one"
        );
    }
}
