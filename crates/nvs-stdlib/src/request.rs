//! `Core\Request` — the request a program is answering, replacing `$_GET`,
//! `$_POST`, `$_COOKIE`, `$_FILES`, `$_REQUEST` and `filter_input`
//! ([ADR 0012](/docs/adr/0012-no-superglobals.md)).
//!
//! # What is here, and what is not
//!
//! Eleven of
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 15's
//! sixteen members: `method`, `isHead`, `path` and `query` — the request *line*,
//! and the one fact reporting a `HEAD` as a `Get` would otherwise lose —
//! `header`, `headers` and `cookie`, the fields that arrived with it, and
//! `body`, `bodyStream`, `files` and `post`, the four members here that read
//! what arrived **after** all of those — the same [`nvs_runtime::RequestBody`]
//! pulled to its end into one value, walked a chunk at a time, walked as the
//! parts a `multipart/form-data` body declares
//! ([ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
//! § 1, the parse itself being [`crate::multipart`]'s), or read to its end as
//! the form it submitted (§ 2, and `post` is the one of the four that joins
//! another's reading rather than claiming against it — [`claim_form`]).
//! `clientIp`, `scheme`, `host`, `mount` and
//! `route` are known gaps of this module rather than of § 15, and each waits on
//! a different thing:
//! `route`/`mount` on the match `nvs_server` makes once
//! before the handler, and `host` not on a carrier at all — a `Host` line is
//! already one of [`nvs_runtime::Inbound`]'s headers, and what that member
//! waits on is whether a *forwarded* host may be believed, which ADR 0097 § 6's
//! walk answers for an address and a scheme and deliberately not for this.
//! **`clientIp` and `scheme` wait on nothing now**: ADR
//! 0097 § 6's walk answers both per request, `nvs_server::serve_connection`
//! hands its `Origin` to the handler, and the carrier holds the answer —
//! [`nvs_runtime::Inbound::client`] and [`nvs_runtime::Inbound::scheme`]. What
//! is left for each is this module's own five edits, and § 6 fixes the one
//! signature they cannot settle here: `clientIp(): ?tainted string`, `null` for
//! the two ways a request arrives with no address at all — a Unix-socket peer
//! that forwarded nothing, and a trusted hop that withheld it.
//!
//! # There is no request here, and that is a throw
//!
//! Every member refuses when the context is answering no request —
//! [ADR 0012](/docs/adr/0012-no-superglobals.md) § 7's rule, which that
//! ADR argues over a *spawned isolate* and which reaches the same conclusion
//! one step wider. A CLI program, a scheduled script, a job worker and a
//! `#[Test]` method are all running with nothing inbound, and an empty string
//! would say the request arrived and sent nothing. Those are different facts,
//! and collapsing them is the silent-wrong-answer failure mode
//! [ADR 0007](/docs/adr/0007-explicit-type-system.md) exists to close:
//! a program that read a path out of a scheduled script and got `""` would
//! route on it.
//!
//! `LogicError` rather than a class of this module's own. It is § 10's "a bad
//! state" exactly — the program asked a question its own situation has no
//! answer to — and a named class would be a `catch` name for a condition no
//! correct program ever recovers from.
//!
//! # Three members read the body, and the request records which one did
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
//! is `rule:errors/ambiguous-input-refused`'s
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
//! and not `rule:errors/ambiguous-input-refused`'s
//! repair — nothing is dropped and nothing is invented. What joining *does* lose
//! is the line boundary, which matters for a value that may itself contain a
//! comma (`Date` is the standard example), so the exact answer is `headers()`'s
//! and the convenient one is `header`'s. Answering the *first* line was rejected:
//! it is the reading that silently drops what a peer sent, which is the failure
//! mode [ADR 0007](/docs/adr/0007-explicit-type-system.md) exists to
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
//! (`rule:errors/ambiguous-input-refused`
//! § 3): no dot, space or bracket is substituted in either direction. That
//! mangling is PHP's `register_globals`-era name repair, it is what
//! CVE-2024-2756 was, and the superglobals it served are what
//! [ADR 0012](/docs/adr/0012-no-superglobals.md) deleted.
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
//! convention there is what lets this member answer the same shape. The *shape*
//! is what is shared and not the element type: `Core\Uri::parseQuery` is a
//! § 12 member and answers a value's decoded octets as `bytes`, while a served
//! request's parameters are read as text at the door, so this member passes
//! `crate::uri::Values::Text` and keeps the refusal for octets no `string`
//! holds. That enum is the whole of the difference. Two lookups
//! parse twice. That is O(query) per read rather than per request, and it is
//! deliberate for now: a memoized parse is state on the context, and the
//! context does not hold a *parsed* request yet — only the bytes one arrived
//! as. A request with two dozen reads is what would make it worth having, and
//! the inbound carrier is where it will live.
//!
//! **The `tainted` qualifier does not survive `mixed`.** `path` is a
//! `tainted string` and the checker holds it to
//! [ADR 0024](/docs/adr/0024-taint-tracking-for-injection-sinks.md)'s
//! sinks; `query` answers `mixed`, because § 9's bracket convention makes a
//! value a `string` *or* a nested array, and `nvs_types` has no `tainted
//! array<T>` — the qualifier axes are defined over `string` and `bytes`. So a
//! program checks the answer out with `as`, and what it lands in is a plain
//! `string`. That is the same hole `Core\Uri::parseQuery` already has and is
//! not new here, but it is worth naming at the one member most likely to be the
//! source of an injection: `Core\Request::header` and `::cookie`, which answer
//! a `tainted string` directly, do carry it.

use nvs_runtime::{Ctx, Fault, Inbound, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

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
        CoreMethod {
            name: "files",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(FILES_NAME),
            symbol: "nvs_core_request_files",
            doc: Some(&FILES_DOC),
        },
        CoreMethod {
            name: "post",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_request_post",
            doc: Some(&POST_DOC),
        },
        CoreMethod {
            name: "route",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(crate::router::MATCH_NAME)),
            symbol: "nvs_core_request_route",
            doc: Some(&ROUTE_DOC),
        },
        CoreMethod {
            name: "mount",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(MOUNT_NAME),
            symbol: "nvs_core_request_mount",
            doc: Some(&MOUNT_DOC),
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

/// `Core\Request::files`'s reference card — ADR 0117.
const FILES_DOC: MethodDoc = MethodDoc {
    short: "The uploaded files this request carries, as a walk over its parts — the one way to \
            receive one, replacing `$_FILES` and `move_uploaded_file` with a stream that never \
            lands in a temporary directory.",
    params: &[],
    ret: "An `Iterable<Core\\Request\\Part>` a `foreach` walks once, yielding each file part as \
          it comes off the wire. Ordinary form fields are not parts of this walk: they are \
          buffered as the walk passes them and read back through `post`. Empty where the \
          request declared no `multipart/form-data` body, which is what a request carrying no \
          upload is.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is not answering a request, or this request's body has already \
                   been read by `body`, `bodyStream` or `post` — those readings are exclusive \
                   with this one, and naming this walk is the reading. `post` after this walk \
                   is the one order that is allowed, because the walk buffers the form fields \
                   on its way past.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "The request declared a `multipart/form-data` body and then did not say how \
                   to read one — no `boundary`, two of them, or one outside RFC 2046's grammar \
                   — or what arrived is not the body it declared. An ambiguous body is refused \
                   rather than guessed at.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed under the body, or the peer stopped short of the \
                   length it declared.",
        },
    ],
};

/// `Core\Request::post`'s reference card — ADR 0117.
const POST_DOC: MethodDoc = MethodDoc {
    short: "One submitted form field by name, read with PHP's bracket convention — the same parse \
            `query` performs, over a `multipart/form-data` body's non-file parts or over a \
            urlencoded one, replacing `$_POST` and `filter_input(INPUT_POST, …)`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The field's name, as the form declared it and without brackets for a nested value.",
        shape: &[],
    }],
    ret: "The field's value as a `string`, a nested `array<mixed>` for a bracketed key, or `null` \
          where the form carried no such name. Reading the body to its end is what this member \
          does, so on a `multipart/form-data` request it is called **after** the `files()` walk, \
          never before: the uploads are drained on the way to the last field.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is not answering a request, or this request's body has already \
                   been read by `body` or `bodyStream` — those two hand the bytes over \
                   uninterpreted and leave no fields behind. A body `files` is walking is the one \
                   case this member joins rather than refuses.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "The request declared a `multipart/form-data` body and then did not say how to \
                   read one, or what arrived is not the body it declared, or a urlencoded field \
                   holds percent escapes that decode to octets that are not UTF-8.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed under the body, or the peer stopped short of the \
                   length it declared.",
        },
    ],
};

/// `Core\Request::route`'s reference card — ADR 0117.
const ROUTE_DOC: MethodDoc = MethodDoc {
    short: "The route this request matched, which the server took once at the door before any of \
            this program ran — the same match the CSRF check and the `route` metric label read, \
            so a handler never matches its own path a second time.",
    params: &[],
    ret: "A `Core\\Router\\Match` answering the declared name and the path's captures, or `null` \
          where nothing in the table claimed this method and path — which is a served request \
          like any other, since matching dispatches nothing. A program declaring no `#[Route]` \
          builds no table and reads `null` here for the same reason.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is not answering a request, as a CLI program, a scheduled script, \
                   a job worker and a test are not — refused rather than answered `null`, because \
                   \"no request arrived\" and \"nothing matched\" are different facts.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "A capture percent-decodes to octets that are not UTF-8, so it has no `tainted \
                   string` to bind to; the throw names the capture and the offset of the first \
                   byte a `string` cannot hold.",
        },
    ],
};

/// `Core\Request::mount`'s reference card — ADR 0117.
const MOUNT_DOC: MethodDoc = MethodDoc {
    short: "Which mount is serving this request: the prefix the server took off the path before \
            `path()` answered it, and the glob captures of the mount row that took it — the pair a \
            host-mounted deployment learns which tenant it is running for from.",
    params: &[],
    ret: "A `Core\\Request\\Mount`, always — a request that reached this program reached it through \
          some mount, and one that came in by a door with no prefix and no glob answers `\"\"` and \
          an empty array rather than `null`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "This program is not answering a request — a CLI program, a scheduled script, a job \
               worker and a test are not, and none of them was mounted anywhere.",
    }],
};

// ---------------------------------------------------------------- ADR 0102 § 7's mount

/// `Core\Request\Mount`'s fully-qualified name, written once so the registry
/// row and every message quoting it cannot drift apart.
pub(crate) const MOUNT_NAME: &str = r"Core\Request\Mount";

/// [`MOUNT`]'s slots, in the order [`mount_value`] fills them.
const MOUNT_PREFIX: usize = 0;
const MOUNT_CAPTURES: usize = 1;

/// `array<tainted string>` — what [`MOUNT`]'s `captures` answers, and a named
/// constant for [`HEADER_LINES`]'s reason: the qualifier has to sit on the
/// value a program actually reaches, and there is no `tainted array<T>` for it
/// to sit on instead.
const CAPTURE_SEGMENTS: CoreTy = CoreTy::Array(&CoreTy::TaintedStr);

/// [ADR 0102](/docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
/// § 7's mount, as the program answering the request reads it.
///
/// # Why a class, where § 7 spells a shape
///
/// § 7 writes `{prefix: string, captures: array<tainted string>}`, and that is
/// not spellable in return position: [`CoreTy::Shape`] is a parameter-only
/// variant whose own doc says no runtime representation of a shape appears
/// anywhere, and `nvs_ir::lower_checked_ty` erases a `Ty::Shape` value to
/// `Ty::Object` — so a program holding one would reach both fields through
/// `mixed` and the `tainted` § 7 exists to state would be gone with them. The
/// alternative was a return-position shape, which is a type-system feature
/// bought for one member. `Core\Router\Match` is the same pair of answers six
/// sections earlier and is already a class, so this costs a reader nothing new
/// to learn; § 7's body spells this class now, so the ADR and the row agree.
///
/// # The prefix is the deployment's, the captures are read as the peer's
///
/// Both come off the `nvs_config::mount` row that
/// [ADR 0097](/docs/adr/0097-development-server-and-proxied-origin.md)
/// § 4 step 1 selected, and every such row was expanded against the disk at
/// boot — § 2's rule that a path is never derived from a URL at request time is
/// exactly what makes that so. The prefix is therefore one of a set the
/// operator wrote, and plain. **The captures are `tainted` anyway**, which is
/// § 7's own call and the fail-closed one: which row answers is the peer's
/// choice, a capture is what a multi-tenant program keys its data by, and
/// laundering one for a query is the ordinary path rather than a cost.
///
/// What it spends is one instance and one array of the mount's own captures per
/// *read* — a handful of values against a glob's one or two captures, and
/// O(in-flight) either way. Nothing is cached: the two facts are on
/// [`Inbound`](nvs_runtime::Inbound) and this class is a reading of them, the
/// same arrangement `Core\Request::route()` has with the match.
pub(crate) const MOUNT: CoreClass = CoreClass {
    name: MOUNT_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "prefix",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_request_mount_prefix",
            doc: Some(&MOUNT_PREFIX_DOC),
        },
        CoreMethod {
            name: "captures",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CAPTURE_SEGMENTS,
            symbol: "nvs_core_request_mount_captures",
            doc: Some(&MOUNT_CAPTURES_DOC),
        },
    ],
    slots: &["prefix", "captures"],
    constants: &[],
};

/// `Core\Request\Mount::prefix`'s reference card — ADR 0117.
const MOUNT_PREFIX_DOC: MethodDoc = MethodDoc {
    short: "What the server took off the front of the path before `Core\\Request::path()` answered \
            it — so a program mounted at `/acme` sees `/orders` and learns the `/acme` here.",
    params: &[],
    ret: "The prefix, with its leading slash and no trailing one, or `\"\"` where the request \
          reached this program through a mount that strips nothing. Not `tainted`: every mount is \
          a row of the table the boot expanded against the disk, so a prefix is the operator's \
          text and not the peer's.",
    errors: &[],
};

/// `Core\Request\Mount::captures`'s reference card — ADR 0117.
const MOUNT_CAPTURES_DOC: MethodDoc = MethodDoc {
    short: "The glob captures of the mount serving this request, in order — `{1}` is `captures[0]` \
            — which is how one compiled program running at many prefixes learns which tenant it is \
            answering for.",
    params: &[],
    ret: "The captures as `tainted string`s, and an empty array for a mount whose prefix holds no \
          glob. They are marked because the peer chose which mount answers it, so one reaching a \
          query or a path launders the way anything else off a request does.",
    errors: &[],
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

/// `Core\Request::files`'s answer, as [`CoreTy::Instance`] spells it.
pub(crate) const FILES_NAME: &str = r"Core\Request\Files";

/// The symbol behind `Iterable<Part>::iterate()`, reached by name through this
/// class's method table — see [`crate::instance`]'s dispatch roster.
pub(crate) const FILES_ITERATE_SYMBOL: &str = "nvs_core_request_files_iterate";
/// The symbol behind `Iterator<Part>::advance()`, which is where the parse is
/// walked to the next file part.
pub(crate) const FILES_ADVANCE_SYMBOL: &str = "nvs_core_request_files_advance";
/// The symbol behind `Iterator<Part>::current()`.
pub(crate) const FILES_CURRENT_SYMBOL: &str = "nvs_core_request_files_current";

/// [`FILES`]'s one slot: the part the last `advance()` opened, which `current()`
/// answers, and `null` before the first one and after the last.
const FILES_PART: usize = 0;

/// The class [ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
/// § 1's `files()` answers with — `Iterable<Core\Request\Part>`, given the name
/// the registry needs to write it.
///
/// # It is its own iterator, for [`BODY_STREAM`]'s reason
///
/// The next part does not exist when the walk is named, so there is no snapshot
/// for a [`crate::cursor`] to run over: `iterate()` answers the receiver and
/// `advance()` walks [`crate::multipart::Multipart`] to the next file part.
/// That class's own doc is the argument, one layer down, and this is the third
/// `Core` class to take the shape after `Core\Task\Channel` and the body walk.
///
/// **Advancing past a part nobody read drains it** — § 1, where skipping an
/// upload the application does not recognise is simply not touching it. The
/// drain is the parse's and costs a walk over bytes the door already charged
/// against `upload_total`, never memory.
///
/// # Why it has no members
///
/// [`BODY_STREAM`]'s answer: everything it does is the three names on
/// [`crate::instance`]'s dispatch roster, so it is a *handle* in the sense
/// `registry`'s `a_class_with_slots_has_instance_members_and_the_reverse`
/// names. Spec § 15 writes `files(): Iterable<Part>` and puts every member on
/// the part rather than on the walk.
pub(crate) const FILES: CoreClass = CoreClass {
    name: FILES_NAME,
    methods: &[],
    instance: &[],
    slots: &["part"],
    constants: &[],
};

/// One file part of a multipart body, as [`CoreTy::Instance`] spells it.
pub(crate) const PART_NAME: &str = r"Core\Request\Part";

/// [`PART`]'s slots, in the order [`part_value`] fills them.
const PART_FIELD: usize = 0;
const PART_FILENAME: usize = 1;
const PART_CONTENT_TYPE: usize = 2;
/// Which part of the body this one is, counted from the start and including the
/// form-field parts the walk consumed on the way — the identity ADR 0105 § 3's
/// "valid only while this part is the iterator's current one" is checked
/// against, by [`part_parse`] and on behalf of both members that read bytes.
const PART_ORDINAL: usize = 3;

/// [ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
/// § 2's file part: what one upload declared about itself, ahead of its bytes.
///
/// # A part is a file part iff it declared a `filename`
///
/// RFC 7578's own distinction, and § 2 refuses to invent a second one. Every
/// other part is an ordinary form field, which [`crate::multipart`] buffers as
/// the walk passes it and `post()` reads back — so this class is never a form
/// field's carrier and has no member that would answer for one.
///
/// # All three readers are `tainted`, and the spec taints two
///
/// `filename` and `contentType` are what § 2 marks, and `name` is marked here
/// as well: a field name arrives off the same wire, from a peer that is under
/// no obligation to send back the names the form declared, and
/// [ADR 0024](/docs/adr/0024-taint-tracking-for-injection-sinks.md)'s
/// rule is over untrusted *input* rather than over a list of fields. Leaving it
/// plain would have made the part's own name the one launderer on the class —
/// reachable by using it as a path or an identifier — which is the direction
/// `AGENTS.md`'s priority 1 does not trade.
///
/// # There is no `size`, and there is no `filename` that is a path
///
/// § 2 refuses a `size`: there is no honest value before the part has been
/// consumed, and inventing one is the repair
/// `rule:errors/ambiguous-input-refused`
/// exists to forbid. `filename` is the client's *claim* and is never treated as
/// a path — as a `tainted string` it reaches no path sink without
/// `Core\IO::within` laundering it, which is the same refusal every other
/// untrusted string meets.
///
/// **`contentType` answers `text/plain` where the part declared none**, which
/// is RFC 7578 § 4.4's stated default rather than a repair of a missing value.
/// The alternative — a nullable reader — would put a `??` at every call site to
/// re-supply the number the RFC already fixed.
///
/// # Two consumers beside the three declarations, and both are the same pull
///
/// § 3's `content()` and `readAll()`, which are the two ways to reach a part's
/// *bytes*: the walk that holds one chunk at a time, and the buffer that holds
/// the part. Both pull [`crate::multipart::Multipart::next_chunk`] and neither
/// accumulates anything the other does not — `readAll` is that walk with a
/// `Vec` and a bound around it, which is `body`'s relationship to `bodyStream`
/// one level down.
///
/// **Both are valid only while this part is the walk's current one**, which is
/// § 3's own words and [`PART_ORDINAL`]'s whole purpose. A program that keeps a
/// part past the `advance()` that opened the next one is holding a name for
/// bytes the parse has already drained, and handing it the *current* part's
/// bytes would be the silent wrong answer — so it is refused, by
/// [`part_parse`], which is the one place the stamp is compared.
pub(crate) const PART: CoreClass = CoreClass {
    name: PART_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_request_part_name",
            doc: Some(&PART_FIELD_DOC),
        },
        CoreMethod {
            name: "filename",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_request_part_filename",
            doc: Some(&PART_FILENAME_DOC),
        },
        CoreMethod {
            name: "contentType",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_request_part_content_type",
            doc: Some(&PART_CONTENT_TYPE_DOC),
        },
        CoreMethod {
            name: "content",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(PART_CONTENT_NAME),
            symbol: "nvs_core_request_part_content",
            doc: Some(&CONTENT_DOC),
        },
        CoreMethod {
            name: "readAll",
            names: &[],
            params: &[CoreTy::Options(READ_ALL_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::TaintedBytes,
            symbol: "nvs_core_request_part_read_all",
            doc: Some(&READ_ALL_DOC),
        },
        CoreMethod {
            name: "saveTo",
            names: &["path"],
            // The path is a sink and `filename()` is `tainted`, which is § 2's
            // whole point standing where it bites: the one place an upload
            // could choose where it lands is the one place the qualifier
            // refuses, and `Core\IO::within` is the launderer. The options are
            // `Core\IO::writeStream`'s own bag rather than a copy of it —
            // § 4 makes this member that one's delegation, and a default
            // written twice is a default that can disagree with itself.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Options(crate::io::WRITE_STREAM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_request_part_save_to",
            doc: Some(&SAVE_TO_DOC),
        },
    ],
    slots: &["name", "filename", "contentType", "ordinal"],
    constants: &[],
};

/// `Core\Request\Part::name`'s reference card — ADR 0117.
const PART_FIELD_DOC: MethodDoc = MethodDoc {
    short: "The form field this file arrived under — the `name` attribute of the `<input>`, as \
            the peer sent it back.",
    params: &[],
    ret: "The field name, `tainted` because the peer chose it: a client is free to send a name \
          the form never declared, so it is untrusted input like every other byte of the part.",
    errors: &[],
};

/// `Core\Request\Part::filename`'s reference card — ADR 0117.
const PART_FILENAME_DOC: MethodDoc = MethodDoc {
    short: "The file name the client claimed — a claim about a file on someone else's machine, \
            and never a path on this one.",
    params: &[],
    ret: "The claimed name, `tainted`. It reaches no path sink without `Core\\IO::within` \
          laundering it, which is what keeps a peer from choosing where its own upload lands.",
    errors: &[],
};

/// `Core\Request\Part::contentType`'s reference card — ADR 0117.
const PART_CONTENT_TYPE_DOC: MethodDoc = MethodDoc {
    short: "The media type this part declared, which is what the client said the bytes are and \
            not what they turn out to be.",
    params: &[],
    ret: "The declared type, `tainted`, or `text/plain` where the part declared none — RFC 7578 \
          § 4.4's default. A program that needs to know what the bytes *are* reads the bytes.",
    errors: &[],
};

/// `Core\Request\Part::content`'s reference card — ADR 0117.
const CONTENT_DOC: MethodDoc = MethodDoc {
    short: "This part's bytes, a chunk at a time — the reading for an upload that must never be \
            resident whole, and what `saveTo` and `readAll` are both written over.",
    params: &[],
    ret: "An `Iterable<tainted bytes>` over the part's chunks as they come off the wire, each one \
          a value of its own. It walks empty for a part that carried no bytes, and it is valid \
          only while this part is the one the `files()` walk is on.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is not answering a request, or the walk has moved on to a later \
                   part and this one's bytes are gone.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed under the body, or the peer stopped short of the \
                   closing boundary.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "What arrived is not the multipart body the request declared.",
        },
    ],
};

/// `Core\Request\Part::readAll`'s options — ADR 0105 § 3's one bound.
///
/// **`max` is a count of bytes and `0` is the absent one.** A bound of zero
/// accepts only an empty part, so no caller means it, and a sentinel is what
/// lets the two bounds § 3 states be told apart at all: a call that says
/// nothing is held to `[limits] request_body`, and a call that names a number
/// is held to that number and checked against the request's own `[limits]
/// memory` instead. Without the sentinel there is one bound and the 200M
/// buffer § 3 makes expressible would have to be the default for everyone.
const READ_ALL_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "max",
    ty: CoreTy::Uint,
    default: Const::Uint(0),
}];

/// `Core\Request\Part::readAll`'s reference card — ADR 0117.
const READ_ALL_DOC: MethodDoc = MethodDoc {
    short: "This part's whole content, pulled to its end into one value — the reading for an \
            upload small enough to hold, replacing `$_FILES` plus a `file_get_contents` of the \
            temporary file PHP wrote.",
    params: &[ParamDoc {
        name: "max",
        desc: "How many bytes this call is willing to hold. Omitted, the bound is `[limits] \
               request_body` (8M); named, it is this number, and a number larger than the \
               request's own `[limits] memory` is refused rather than clamped.",
        shape: &[],
    }],
    ret: "Every byte of this part, in order, `tainted` and decoded by nothing. Empty for a part \
          that carried none, which is a part the peer sent and not an absent one.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is not answering a request, the walk has moved on to a later \
                   part, or `max` is larger than this request may hold at all.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The part is larger than the bound in force. The bytes over it are never \
                   held: the refusal happens at the chunk that would cross it, and `content()` \
                   is the reading for a part that does not fit.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The connection failed under the body, or the peer stopped short of the \
                   closing boundary.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "What arrived is not the multipart body the request declared.",
        },
    ],
};

/// `Core\Request\Part::saveTo`'s reference card — ADR 0117.
const SAVE_TO_DOC: MethodDoc = MethodDoc {
    short: "Writes this part straight to `$path`, holding one chunk at a time — the path 99.9% of \
            uploads take, replacing `move_uploaded_file` of a temporary file the host chose. Needs \
            the `fs.write` capability for the path, and the part's own `filename()` is `tainted`, \
            so it reaches this only through `Core\\IO::within`.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "Where the part is to land. It must not already exist unless `overwrite` says \
                   otherwise.",
            shape: &[],
        },
        ParamDoc {
            name: "max",
            desc: "The most bytes to accept from this part. Unbounded when it is not given, \
                   because nothing else bounds a file on disk — unlike `readAll`, which is \
                   holding what it reads and so inherits `[limits] request_body`.",
            shape: &[],
        },
        ParamDoc {
            name: "overwrite",
            desc: "Whether an existing file may be replaced. `false` by default, because the \
                   destination is usually built from a name the client claimed.",
            shape: &[],
        },
    ],
    ret: "Nothing. A failure part-way through removes the partial file before it throws, so no \
          later reader finds a truncated upload the program believes it received whole.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "This program is not answering a request, or the walk has moved on to a later \
                   part and this one's bytes are gone.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.write` for this path, or the part ran \
                   past `max` bytes.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "Something is already at the path and `overwrite` is `false`, the operating \
                   system refused the create or a write, or the connection failed under the body.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "What arrived is not the multipart body the request declared.",
        },
    ],
};

/// `Core\Request\Part::content`'s answer, as [`CoreTy::Instance`] spells it.
pub(crate) const PART_CONTENT_NAME: &str = r"Core\Request\PartContent";

/// The symbol behind `Iterable<tainted bytes>::iterate()` on a part's content.
pub(crate) const PART_CONTENT_ITERATE_SYMBOL: &str = "nvs_core_request_part_content_iterate";
/// The symbol behind `Iterator<tainted bytes>::advance()`, which is where one
/// run of the current part's bytes is pulled.
pub(crate) const PART_CONTENT_ADVANCE_SYMBOL: &str = "nvs_core_request_part_content_advance";
/// The symbol behind `Iterator<tainted bytes>::current()`.
pub(crate) const PART_CONTENT_CURRENT_SYMBOL: &str = "nvs_core_request_part_content_current";

/// [`PART_CONTENT`]'s slots: the chunk the last `advance()` pulled, and the
/// [`PART_ORDINAL`] of the part this walk was named on.
const PART_CONTENT_CHUNK: usize = 0;
const PART_CONTENT_ORDINAL: usize = 1;

/// The class [ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
/// § 3's `content()` answers with — `Iterable<bytes>` over one part, given the
/// name the registry needs to write it.
///
/// [`BODY_STREAM`]'s shape for the third time, and its docs are the argument:
/// the next chunk does not exist when the walk is named, so `iterate()` answers
/// the receiver and `advance()` pulls. What is new here is the **second slot**.
/// A walk over a whole body needs no identity — there is one body and it is the
/// request's — but a part is a *position* in one, and the parse moves whether or
/// not this walk is the thing that moved it. So the ordinal the part was stamped
/// with is copied in when the walk is named and compared on every pull, which is
/// § 3's "valid only while this part is the iterator's current one" made
/// checkable rather than documented.
///
/// **What it spends:** one chunk, resident, for as long as the loop body holds
/// it — [`BODY_STREAM`]'s figure exactly, because it is the same pull with a
/// multipart parse in front of it.
pub(crate) const PART_CONTENT: CoreClass = CoreClass {
    name: PART_CONTENT_NAME,
    methods: &[],
    instance: &[],
    slots: &["chunk", "ordinal"],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_request_method" => (nvs_core_request_method as *const ()).cast(),
        "nvs_core_request_route" => (nvs_core_request_route as *const ()).cast(),
        "nvs_core_request_mount" => (nvs_core_request_mount as *const ()).cast(),
        "nvs_core_request_mount_prefix" => (nvs_core_request_mount_prefix as *const ()).cast(),
        "nvs_core_request_mount_captures" => (nvs_core_request_mount_captures as *const ()).cast(),
        "nvs_core_request_is_head" => (nvs_core_request_is_head as *const ()).cast(),
        "nvs_core_request_path" => (nvs_core_request_path as *const ()).cast(),
        "nvs_core_request_query" => (nvs_core_request_query as *const ()).cast(),
        "nvs_core_request_header" => (nvs_core_request_header as *const ()).cast(),
        "nvs_core_request_headers" => (nvs_core_request_headers as *const ()).cast(),
        "nvs_core_request_cookie" => (nvs_core_request_cookie as *const ()).cast(),
        "nvs_core_request_body" => (nvs_core_request_body as *const ()).cast(),
        "nvs_core_request_body_stream" => (nvs_core_request_body_stream as *const ()).cast(),
        "nvs_core_request_files" => (nvs_core_request_files as *const ()).cast(),
        "nvs_core_request_post" => (nvs_core_request_post as *const ()).cast(),
        "nvs_core_request_part_name" => (nvs_core_request_part_name as *const ()).cast(),
        "nvs_core_request_part_filename" => (nvs_core_request_part_filename as *const ()).cast(),
        "nvs_core_request_part_content_type" => {
            (nvs_core_request_part_content_type as *const ()).cast()
        }
        "nvs_core_request_part_content" => (nvs_core_request_part_content as *const ()).cast(),
        "nvs_core_request_part_read_all" => (nvs_core_request_part_read_all as *const ()).cast(),
        "nvs_core_request_part_save_to" => (nvs_core_request_part_save_to as *const ()).cast(),
        PART_CONTENT_ITERATE_SYMBOL => (nvs_core_request_part_content_iterate as *const ()).cast(),
        PART_CONTENT_ADVANCE_SYMBOL => (nvs_core_request_part_content_advance as *const ()).cast(),
        PART_CONTENT_CURRENT_SYMBOL => (nvs_core_request_part_content_current as *const ()).cast(),
        FILES_ITERATE_SYMBOL => (nvs_core_request_files_iterate as *const ()).cast(),
        FILES_ADVANCE_SYMBOL => (nvs_core_request_files_advance as *const ()).cast(),
        FILES_CURRENT_SYMBOL => (nvs_core_request_files_current as *const ()).cast(),
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
                 exhausted stream could say. `post` is the one reading that joins another: it \
                 reads the fields a `files` walk buffers"
            ),
        )
    })
}

/// Which reading of the body `Core\Request::post()` is making — see
/// [`claim_form`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Reading {
    /// Nothing had read the body; `post` has taken the claim.
    First,
    /// `files()` holds the claim, and this reading joins its walk.
    Joining,
    /// `post` itself holds the claim: this is a second call on one request.
    Again,
}

/// [`claim_body`] as `Core\Request::post()` makes it, which is the same rule
/// with two of its outcomes moved.
///
/// `files` moves from the refusals to the joins, because what the two members
/// read is not the same stream twice: ADR 0105 § 2's non-file parts are
/// buffered by that walk on its way past, so `post` reads what `files` set
/// aside rather than the bytes `files` yielded. `body` and `bodyStream` hand
/// the body over uninterpreted and leave nothing behind, so both still refuse.
/// And `post` itself is not a second reading at all — the first call held what
/// it read, and every later one re-parses that.
///
/// # Errors
///
/// `LogicError` where `body` or `bodyStream` has already read this body.
fn claim_form(ctx: &mut Ctx) -> Result<Reading, Fault> {
    let claimed = ctx
        .inbound_mut()
        .expect("the caller reads the request before it claims the body")
        .claim_body("post");
    match claimed {
        Ok(()) => Ok(Reading::First),
        Err("files") => Ok(Reading::Joining),
        Err("post") => Ok(Reading::Again),
        // No case can reach this: a `.nvst` program answers no request, so
        // `inbound_of` refuses both readers before either reaches the claim.
        // Asserted by `a_form_is_refused_to_a_body_that_was_handed_over_whole`.
        Err(first) => Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Request::post(): this request's body has already been read by \
                 `Core\\Request::{first}()`, which hands the bytes over uninterpreted and leaves \
                 no form fields behind — so there is nothing left here to read them out of. A \
                 program that wants both parses what it was handed. `files` is the one reading \
                 `post` joins, because that walk buffers the non-file parts on its way past"
            ),
        )),
    }
}

/// The whole form this request submitted, parsed afresh on every call.
///
/// `Core\Request::post()`'s reading, split out so that member reads as the one
/// question it answers. What it costs per call is [`nvs_core_request_post`]'s
/// own doc, and it is `Core\Request::query`'s cost over a body instead of a
/// query string.
///
/// # Errors
///
/// [`claim_form`]'s refusal, a body that is not the multipart one it declared
/// or that did not arrive whole, and a urlencoded field whose escapes decode to
/// octets that are not UTF-8.
fn form_of(ctx: &mut Ctx, declared: Option<Vec<u8>>) -> Result<NvsArray, Fault> {
    let reading = claim_form(ctx)?;
    match declared.filter(|value| crate::multipart::is_multipart(value)) {
        Some(declared) => multipart_form(ctx, &declared, reading),
        None => urlencoded_form(ctx, reading),
    }
}

/// ADR 0105 § 2's buffered fields, with the walk driven to the closing
/// delimiter first.
///
/// The drain is what makes *every* field answerable rather than the ones that
/// happened to arrive before the part a `files()` walk stopped on, and it is
/// idempotent: a parse already at its end walks nothing.
fn multipart_form(ctx: &mut Ctx, declared: &[u8], reading: Reading) -> Result<NvsArray, Fault> {
    if reading == Reading::First {
        // No case can reach this: a `.nvst` program answers no request, so it
        // carries no `Content-Type` to declare a body with. Asserted by
        // `a_multipart_body_that_declares_no_boundary_is_refused_where_it_is_named`.
        let boundary = crate::multipart::boundary_of(declared).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Parse,
                format!(
                    "Core\\Request::post(): this request declared a multipart body and then did \
                     not say how to read one — {why}"
                ),
            )
        })?;
        ctx.inbound_mut()
            .expect("the caller reads the request before it reads the form")
            .hold_parts(Box::new(crate::multipart::Multipart::new(&boundary)));
    }
    let inbound = ctx
        .inbound_mut()
        .expect("the caller reads the request before it reads the form");
    let mut out = NvsArray::new();
    // No parse, or no body at all: a request that submitted no form, which is
    // an empty answer rather than a refusal — `files()`'s own reading of the
    // same two cases.
    let Some((parse, body)) = inbound.parts_mut() else {
        return Ok(out);
    };
    let parse = parse
        .downcast_mut::<crate::multipart::Multipart>()
        .expect("`files()` and `post()` hold one parse between them, and it is this one");
    if let Err(why) = parse.drain(body) {
        // No case can reach either of these, for `files()`'s reason: a `.nvst`
        // program holds no parse to walk and no connection to fail under one.
        let (class, what) = if parse.failed_on_the_wire() {
            (ThrownClass::Io, "the body did not arrive whole")
        } else {
            (
                ThrownClass::Parse,
                "this is not the multipart body the request declared",
            )
        };
        return Err(Fault::thrown_as(
            class,
            format!("Core\\Request::post(): {what} — {why}"),
        ));
    }
    for (name, value) in parse.fields() {
        // `parse_query`'s own skip, so one nameless field means the same thing
        // in a form as it does in a query string.
        if name.is_empty() {
            continue;
        }
        crate::uri::place(&mut out, name, Value::str(NvsStr::new(value)));
    }
    Ok(out)
}

/// A `application/x-www-form-urlencoded` body, read once and parsed per call.
///
/// The bytes are held on [`nvs_runtime::Inbound`] rather than the array,
/// because this crate hands a fresh value to each call and the carrier below it
/// holds no value of the program's — [`nvs_runtime::Inbound::hold_form`] owns
/// that argument.
///
/// It does not check the content type. A body that declares nothing, or
/// declares something else, is still read the way `$_POST` reads one, because
/// what a peer wrote in a header is not what decides whether a form is a form —
/// and a body that is not one parses to no fields rather than to a refusal.
fn urlencoded_form(ctx: &mut Ctx, reading: Reading) -> Result<NvsArray, Fault> {
    if reading != Reading::Again {
        let whole = whole_body(ctx, "post")?;
        ctx.inbound_mut()
            .expect("the caller reads the request before it reads the form")
            .hold_form(whole.into_boxed_slice());
    }
    let held = ctx
        .inbound()
        .expect("the caller reads the request before it reads the form")
        .form()
        .expect("the branch above holds the body before the first read of it");
    // Refused rather than repaired, under ADR 0095: a urlencoded body is
    // percent-escaped ASCII by construction, so a raw octet outside UTF-8 in
    // one is a body that is not what it claims to be — and lossily replacing it
    // would answer a field the peer never sent.
    //
    // No case can reach this: a `.nvst` program answers no request, so it has
    // no body to send octets in. Asserted by
    // `a_form_is_refused_to_a_body_that_was_handed_over_whole`.
    let held = std::str::from_utf8(held).map_err(|_| {
        Fault::thrown_as(
            ThrownClass::Parse,
            "Core\\Request::post(): this request's body is not text, so it holds no urlencoded \
             form to read. A body that is not a form is read with `body()` or `bodyStream()`"
                .to_owned(),
        )
    })?;
    crate::uri::parse_query(held, "post", crate::uri::Values::Text)
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

/// `rule:errors/cookie-name-bytes`'s stricter prefix, on the read side — the write side's spelling
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
/// **The comparison is byte for byte**, which is `rule:errors/cookie-name-bytes` and the whole of
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
/// **A `__Host-` name that arrived twice is not visible**, which is `rule:errors/ambiguous-input-refused`
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
pub(crate) fn cookie_of<'a>(inbound: &'a Inbound, name: &[u8]) -> Option<&'a [u8]> {
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
            // recognised is exactly `rule:errors/ambiguous-input-refused`'s repair.
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
    /// `Core\Request::route(): ?Core\Router\Match` — ADR 0102 § 1's match, read
    /// where the section says a program reads it.
    ///
    /// **Nothing here matches.** The row was chosen by `nvs_server::route`
    /// before this program started and travels on the [`Inbound`]; this member
    /// hands it over. A second match here is precisely what § 1 exists to
    /// remove, and it would also be a *different* match — the door held the
    /// unit's own table and the mount-stripped path, and neither is a fact this
    /// call can re-derive.
    ///
    /// **`null` is one answer for three absences**, which is
    /// [`Ctx::route`](nvs_runtime::Ctx::route)'s own reading: the program
    /// declared no `#[Route]`, or the table claimed nothing for this method and
    /// path, or the request reached this program by a door that takes no match.
    /// A program cannot act differently on those three — in all of them there
    /// is no route, and § 1's rule is that having none is a served request like
    /// any other — so splitting them would be surface with no decision behind
    /// it. **No request at all is the fourth case and is not one of them**: it
    /// refuses through [`inbound_of`], on the terms every other reader of this
    /// class refuses.
    fn nvs_core_request_route(ctx, _args: [0]) {
        let inbound = inbound_of(ctx, "route")?;
        Ok(match inbound.route() {
            Some(matched) => crate::router::match_value(matched)?,
            None => Value::null(),
        })
    }
}

/// The [`MOUNT`] one request carries, built out of the two facts the door
/// already recorded on the carrier — the whole of how ADR 0097 § 4 step 1's
/// selection reaches a program.
///
/// Built per read rather than held, which is [`crate::router::match_value`]'s
/// arrangement and for the same reason: the row lives on
/// [`Inbound`](nvs_runtime::Inbound), where the door put it once, and an object
/// a program is holding reaches nothing of the mount table through it.
fn mount_value(inbound: &Inbound) -> Value {
    let mut captures = NvsArray::new();
    for capture in inbound.mount_captures() {
        captures.append(Value::str(NvsStr::new(capture.as_bytes())));
    }
    crate::instance::build(
        &MOUNT,
        [
            Value::str(NvsStr::new(inbound.mount_prefix().as_bytes())),
            Value::array(captures),
        ],
    )
}

/// The [`crate::instance::receiver`] fault a wrongly-tagged receiver is, which
/// compiled code cannot produce — [`part_slot`]'s note owns why the copy handed
/// back needs a reference of its own.
fn mount_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &MOUNT, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot is owned by the receiver, which the argument slot holds a \
                  reference to for the length of the call, so the copy handed back to \
                  Novis code needs a reference of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\Request::mount(): Core\Request\Mount` — ADR 0102 § 7's two facts
    /// about which mount is serving this request.
    ///
    /// **The answer is never `null`**, where the sibling `route()` is nullable:
    /// nothing matching the route table is an ordinary served request, but
    /// every request that reached a program reached it *through* a mount, and
    /// a door that strips nothing carries `""` and no captures rather than an
    /// absence. That is [`Inbound`](nvs_runtime::Inbound)'s own reading of the
    /// two fields, and [`MOUNT`]'s docs own why the pair is one object.
    fn nvs_core_request_mount(ctx, _args: [0]) {
        let inbound = inbound_of(ctx, "mount")?;
        Ok(mount_value(inbound))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request\Mount::prefix(): string` — what ADR 0097 § 4 step 2 took
    /// off the path before [`nvs_core_request_path`] answered it.
    fn nvs_core_request_mount_prefix(_ctx, args: [1]) {
        mount_slot(args, MOUNT_PREFIX, "prefix")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request\Mount::captures(): array<tainted string>` — ADR 0097
    /// § 3's glob captures of the row that selected this program.
    ///
    /// The array is built once by [`mount_value`] and read back here, so the
    /// two members answer one carrier rather than two readings of it.
    fn nvs_core_request_mount_captures(_ctx, args: [1]) {
        mount_slot(args, MOUNT_CAPTURES, "captures")
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
        let parsed = crate::uri::parse_query(
            inbound_of(ctx, "query")?.query(),
            "query",
            crate::uri::Values::Text,
        )?;
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
    /// `Core\Request::post(string $name): mixed` — spec § 15's submitted-form
    /// reader, replacing `$_POST` and `filter_input(INPUT_POST, …)`.
    ///
    /// [`nvs_core_request_query`]'s answer over a body instead of a query
    /// string, down to the bracket convention, which is
    /// [`crate::uri::place`]'s once rather than twice.
    ///
    /// **It reads the body to its end**, and that is the whole of its rule. ADR
    /// 0105 § 2 promises *every* field of a mixed form, and a form is free to
    /// write a text input after a file input — so a member answering the fields
    /// that had arrived so far would report one the peer sent as absent, which
    /// is the silent wrong answer [`nvs_runtime::Inbound`] is written against.
    /// On a `multipart/form-data` request it therefore drains what is left of
    /// the walk, and a program that wants the uploads too takes `files()`
    /// **first**: a part's bytes exist only while the walk is on it.
    ///
    /// **It is the one reading that joins another rather than claiming against
    /// it** — [`claim_form`] owns why, and it is the only place that rule
    /// lives.
    ///
    /// **The parse is per call.** A urlencoded body is pulled whole and held on
    /// the carrier, a multipart one has its fields buffered by the parse
    /// already, and each call builds its array afresh over those bytes.
    /// `query`'s judgement exactly, and its module doc argues it: an array
    /// cached here would be a value of the program's held by the carrier
    /// underneath it.
    ///
    /// **What it spends:** for a urlencoded body, the body's own bytes resident
    /// until the request ends; for either kind, one array per call, dropped
    /// before the call returns. Both are bounded by [`REQUEST_BODY`] — ADR 0105
    /// § 2's cap on form field text — and both are O(in-flight).
    fn nvs_core_request_post(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Request::post expected a `string` for the name, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        // Read before the claim, because it borrows the carrier immutably and
        // reading a header has no effect on the body.
        let declared = joined_field(inbound_of(ctx, "post")?, b"content-type");
        let parsed = form_of(ctx, declared)?;
        let answer = parsed.get(name.as_bytes()).unwrap_or_else(Value::null);
        // `query`'s reason, and its wording: `get` borrows rather than retains
        // and `parsed` releases everything it holds when it drops at the end of
        // this block, so the value being handed back needs a reference of its
        // own and the caller owns exactly that one.
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
    /// [`cookie_of`]'s; both are `rule:errors/cookie-name-bytes`, whose other end is
    /// `Core\Response::addCookie`'s refusal to write what would not be visible
    /// here. There is no second field on the carrier for cookies: they are the
    /// `Cookie` header, read as such, so nothing can hold a set of cookies that
    /// disagrees with the headers the request arrived with.
    ///
    /// A value is handed back **undecoded**, which is what makes it the same
    /// bytes `addCookie` wrote: that member percent-encodes nothing and refuses
    /// a value it could not write verbatim, so a decode here would be a
    /// substitution in the direction `rule:errors/cookie-name-bytes` closes.
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

/// [ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
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
        Ok(Value::str(NvsStr::new(&whole_body(ctx, "body")?)))
    }
}

/// This request's body, pulled to its end into one buffer under
/// [`REQUEST_BODY`].
///
/// One function rather than two copies of the loop, because `body` and the
/// urlencoded half of `post` read the same bytes the same way and differ only
/// in what they do with them afterwards. The caller owes the claim: this reads,
/// and says nothing about who may.
///
/// A request that arrived without a body reads as an empty one. "The peer sent
/// nothing" and "no request arrived" are different facts and [`inbound_of`]
/// answers the second, so there is nothing left here to refuse.
///
/// # Errors
///
/// `RuntimeError` for a body past [`REQUEST_BODY`], and `IOError` for one that
/// did not arrive whole.
fn whole_body(ctx: &mut Ctx, member: &str) -> Result<Vec<u8>, Fault> {
    let inbound = ctx
        .inbound_mut()
        .expect("the caller reads the request before it reads the body");
    let mut whole: Vec<u8> = Vec::new();
    let Some(body) = inbound.body() else {
        return Ok(whole);
    };
    loop {
        match body.next_chunk() {
            Ok(None) => return Ok(whole),
            Ok(Some(chunk)) => {
                if whole.len().saturating_add(chunk.len()) > REQUEST_BODY {
                    // No case can reach this: a `.nvst` program answers no
                    // request, so it has no body to send over the bound.
                    // Asserted by
                    // `the_request_body_cap_is_the_last_body_read_and_the_first_one_refused`
                    // below, on both sides of the bound.
                    return Err(Fault::thrown(format!(
                        "Core\\Request::{member}(): this request's body is larger than \
                         `[limits] request_body` ({REQUEST_BODY} bytes), so it is refused \
                         rather than held. That directive bounds what a body may cost in \
                         memory; a body bigger than it is one to stream rather than to \
                         read whole"
                    )));
                }
                whole.extend_from_slice(chunk);
            }
            Err(why) => {
                // No case can reach this either, and for the same reason: a
                // connection has to exist before it can fail under a body.
                // Asserted by
                // `a_body_that_fails_mid_stream_throws_rather_than_answering_its_prefix`.
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!("Core\\Request::{member}(): the body did not arrive whole — {why}"),
                ));
            }
        }
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

nvs_runtime::nvs_helper! {
    /// `Core\Request::files(): Iterable<Core\Request\Part>` — ADR 0105 § 1's
    /// walk over this request's uploads, replacing `$_FILES` and
    /// `move_uploaded_file` with a stream that never reaches a temporary
    /// directory.
    ///
    /// ADR 0105 § 3's third way of reading one body, and the only one that
    /// reads it as *structure*. The parse is built here and stored on
    /// [`nvs_runtime::Inbound`], because the walk this answers reaches it again
    /// through a different value on every `advance()` and `post()` will read the
    /// form fields it buffered on the way past — that carrier's `parts` field
    /// owns the argument.
    ///
    /// **A request that declared no multipart body walks empty.** Nothing is
    /// held for it, and `advance()` then answers `false` at once: "this request
    /// sent no files" is exactly true of a `GET`, and a throw there would make
    /// every handler write the content-type check this member has already done.
    /// A request that declares a multipart body and then does not say how to
    /// read one is the other case, and is refused — `crate::multipart`'s
    /// `is_multipart` is where the two are split apart.
    fn nvs_core_request_files(ctx, _args: [0]) {
        // Read before the claim because it borrows the carrier immutably and
        // reading a header has no effect on the body; the claim below is still
        // the first thing that happens *to* the request.
        let declared = joined_field(inbound_of(ctx, "files")?, b"content-type");
        // The claim is here rather than at the first part, for `bodyStream`'s
        // reason: naming the walk is the reading.
        claim_body(ctx, "files")?;
        if let Some(declared) = declared.filter(|value| crate::multipart::is_multipart(value)) {
            // No case can reach this: a `.nvst` program answers no request, so
            // it carries no `Content-Type` to declare a body with. Asserted by
            // `a_multipart_body_that_declares_no_boundary_is_refused_where_it_is_named`.
            let boundary = crate::multipart::boundary_of(&declared).map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Parse,
                    format!(
                        "Core\\Request::files(): this request declared a multipart body and \
                         then did not say how to read one — {why}"
                    ),
                )
            })?;
            ctx.inbound_mut()
                .expect("the read above refuses a context that is answering no request")
                .hold_parts(Box::new(crate::multipart::Multipart::new(&boundary)));
        }
        Ok(crate::instance::build(&FILES, [Value::null()]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<Part>::iterate(): Iterator<Part>` — the walk itself, because
    /// the next part does not exist when the walk is named.
    ///
    /// [`nvs_core_request_body_stream_iterate`]'s shape and for its reason: the
    /// receiver's transferred reference is handed straight back out, so nothing
    /// is allocated and the cursor *is* the parse.
    fn nvs_core_request_files_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &FILES, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Part>::advance(): bool` — walks the parse to the next file
    /// part, answering `false` at the closing delimiter.
    ///
    /// This is the only place a `foreach` over an upload suspends, and it parks
    /// the isolate rather than a thread: every pull underneath is
    /// [`nvs_runtime::RequestBody::next_chunk`]'s.
    ///
    /// **Advancing past a part whose bytes nobody read drains it**, which is the
    /// parse's own behaviour and ADR 0105 § 1's rule — skipping an upload the
    /// application does not recognise is simply not touching it.
    fn nvs_core_request_files_advance(ctx, args: [1]) {
        let stepped = files_step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Part>::current(): Core\Request\Part` — the part the last
    /// `advance()` opened.
    fn nvs_core_request_files_current(_ctx, args: [1]) {
        let read = files_part(args[0]);
        crate::cursor::consume(args[0]);
        read
    }
}

/// [`FILES_ADVANCE_SYMBOL`]'s body: one part, built into the receiver's slot,
/// and whether there was one.
///
/// The slot is cleared to `null` at the end of the walk rather than left holding
/// the last part, on [`body_stream_step`]'s reasoning: the loop is over, so
/// keeping it would hold a part's declarations for as long as the program held
/// the walk value.
///
/// # Errors
///
/// `LogicError` where the context is answering no request — [`inbound_of`]'s
/// refusal — `IOError` where the connection failed under the body, and
/// `ParseError` where what arrived is not the multipart body the request
/// declared. The parse itself classifies nothing; which of the last two applies
/// is read off `Multipart::failed_on_the_wire`, which records the source at the
/// one place it is known.
fn files_step(ctx: &mut Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &FILES, member)?;
    // No case can reach this: a `.nvst` program answers no request, so it can
    // hold no walk to advance. Asserted by
    // `a_files_walk_yields_the_file_parts_and_drains_what_it_passes`, which
    // drives the three names a `foreach` drives.
    inbound_of(ctx, "files")?;
    let inbound = ctx
        .inbound_mut()
        .expect("the read above refuses a context that is answering no request");
    let opened = match inbound.parts_mut() {
        // No parse, or no body at all: both are a request with no file parts in
        // it, which is an empty walk rather than a refusal.
        None => None,
        Some((parse, body)) => {
            let parse = parse
                .downcast_mut::<crate::multipart::Multipart>()
                .expect("`files()` is the only member that holds a parse, and it holds this one");
            match parse.next_part(body) {
                Ok(head) => head.map(|head| (head, parse.opened())),
                Err(why) => {
                    // No case can reach either of these: a `.nvst` program
                    // answers no request, so it holds no parse to walk and no
                    // connection to fail under one. Asserted by
                    // `a_files_walk_that_fails_mid_body_throws_rather_than_ending`
                    // and by
                    // `a_multipart_body_that_declares_no_boundary_is_refused_where_it_is_named`.
                    let (class, what) = if parse.failed_on_the_wire() {
                        (ThrownClass::Io, "the body did not arrive whole")
                    } else {
                        (
                            ThrownClass::Parse,
                            "this is not the multipart body the request declared",
                        )
                    };
                    return Err(Fault::thrown_as(
                        class,
                        format!("Core\\Request::files(): {what} — {why}"),
                    ));
                }
            }
        }
    };
    let more = opened.is_some();
    let held = opened.map_or_else(Value::null, |(head, ordinal)| part_value(&head, ordinal));
    crate::instance::set_slot(receiver, FILES_PART, held);
    Ok(Value::bool(more))
}

/// [`FILES_CURRENT_SYMBOL`]'s body: the slot [`files_step`] last wrote,
/// retained, because the receiver keeps it until the next part is opened.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not a walk — only a bug in this
/// crate can produce one, the receiver having been checked at compile time.
fn files_part(value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::CURRENT;
    let receiver = crate::instance::receiver(value, &FILES, member)?;
    let held = crate::instance::slot(receiver, FILES_PART);
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

/// One [`PART`] instance, built out of the header block the parse read.
///
/// The three declarations are copied here rather than borrowed from the parse,
/// because the part outlives the buffer they were read out of: the very next
/// `advance()` compacts it. That is one short allocation per *file* part, which
/// is O(in-flight) and bounded by `MAX_PARTS`.
fn part_value(head: &crate::multipart::PartHead, ordinal: usize) -> Value {
    crate::instance::build(
        &PART,
        [
            Value::str(NvsStr::new(&head.name)),
            Value::str(NvsStr::new(&head.filename)),
            // RFC 7578 § 4.4's default where the part declared none, which is
            // the class doc's decision and not a repair of a missing value.
            Value::str(NvsStr::new(
                head.content_type.as_deref().unwrap_or(b"text/plain"),
            )),
            Value::uint(
                u64::try_from(ordinal).expect("`MAX_PARTS` bounds a body at a thousand parts"),
            ),
        ],
    )
}

/// What one of [`PART`]'s three readers answers: the slot the part was built
/// with, retained for the caller.
///
/// One helper rather than three bodies, so none of them spells a slot index
/// itself — [`crate::io`]'s `metadata_slot` is the same shape for the same
/// reason.
///
/// # Errors
///
/// The [`crate::instance::receiver`] fault a wrongly-tagged receiver is, which
/// compiled code cannot produce.
fn part_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &PART, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot is owned by the receiver, which the argument slot holds a \
                  reference to for the length of the call, so the copy handed back to \
                  Novis code needs a reference of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\Request\Part::name(): tainted string` — the form field this file
    /// arrived under.
    ///
    /// `tainted` although ADR 0105 § 2 marks only the other two: the class doc
    /// owns why, and it is that a peer chooses this string as freely as it
    /// chooses the filename.
    fn nvs_core_request_part_name(_ctx, args: [1]) {
        part_slot(args, PART_FIELD, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request\Part::filename(): tainted string` — the client's claimed
    /// name, which is never a path here.
    fn nvs_core_request_part_filename(_ctx, args: [1]) {
        part_slot(args, PART_FILENAME, "filename")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Request\Part::contentType(): tainted string` — what the client said
    /// the bytes are, or RFC 7578 § 4.4's `text/plain` where it said nothing.
    fn nvs_core_request_part_content_type(_ctx, args: [1]) {
        part_slot(args, PART_CONTENT_TYPE, "contentType")
    }
}

/// The [`PART_ORDINAL`] the part in `value` was stamped with.
///
/// # Errors
///
/// The [`crate::instance::receiver`] fault a wrongly-tagged receiver is, which
/// compiled code cannot produce.
fn part_ordinal(value: Value, member: &'static str) -> Result<u64, Fault> {
    let receiver = crate::instance::receiver(value, &PART, member)?;
    // unreachable from source: the slot is filled by `part_value` with a
    // `Value::uint` and by nothing else, and a `Core` instance has no reachable
    // property for a program to write one through — `E0322` refuses the
    // spelling. A tag here is this crate having laid the part out wrongly.
    crate::instance::slot(receiver, PART_ORDINAL)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Request\\Part::{member} found no ordinal on the part it was called on"
            ))
        })
}

/// The parse of this request's body and the body it reads, borrowed together
/// and **only** while the part stamped `ordinal` is the one the walk is on.
///
/// The one place ADR 0105 § 3's "valid only while this part is the iterator's
/// current one" is compared, so `content()`, its `advance()` and `readAll()`
/// cannot come to disagree about what a stale part is. `Multipart::opened`
/// counts field parts as well, which is what makes it a position in the body
/// rather than a position among the parts that were answered.
///
/// `Ok(None)` where the request holds no parse or no body: a part cannot exist
/// without one, so this is not a state a program reaches — it is the same
/// "nothing to read" [`files_step`] answers an empty walk with.
///
/// # Errors
///
/// `LogicError` where the context is answering no request — [`inbound_of`]'s
/// refusal — and where the walk has moved past the part the caller holds.
fn part_parse<'ctx>(
    ctx: &'ctx mut Ctx,
    ordinal: u64,
    member: &'static str,
) -> Result<
    Option<(
        &'ctx mut crate::multipart::Multipart,
        &'ctx mut (dyn nvs_runtime::RequestBody + 'static),
    )>,
    Fault,
> {
    // No case can reach this: a `.nvst` program answers no request, so it can
    // hold no part to read. Asserted by
    // `a_parts_content_walks_the_bytes_of_the_part_the_walk_is_on`.
    inbound_of(ctx, member)?;
    let inbound = ctx
        .inbound_mut()
        .expect("the read above refuses a context that is answering no request");
    let Some((parse, body)) = inbound.parts_mut() else {
        return Ok(None);
    };
    let parse = parse
        .downcast_mut::<crate::multipart::Multipart>()
        .expect("`files()` is the only member that holds a parse, and it holds this one");
    let opened =
        u64::try_from(parse.opened()).expect("`MAX_PARTS` bounds a body at a thousand parts");
    if opened != ordinal {
        // No case can reach this either, and for the same reason. Asserted by
        // `a_part_the_walk_has_moved_past_refuses_rather_than_reading_the_current_one`.
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Request\\Part::{member}(): this part is not the one the walk is on — the \
                 walk has opened part {opened} since this one was part {ordinal}, and a part's \
                 bytes are gone once it has. A part is read where it is yielded, or its bytes \
                 are copied there into something the program keeps"
            ),
        ));
    }
    Ok(Some((parse, body)))
}

nvs_runtime::nvs_helper! {
    /// `Core\Request\Part::content(): Iterable<tainted bytes>` — ADR 0105
    /// § 3's chunk-at-a-time reading of one upload.
    ///
    /// **It reads nothing**, exactly as `bodyStream` reads nothing: the pull is
    /// `advance()`'s. What happens here is the stamp being copied into the walk
    /// and checked once, so a program that named a walk over a part the parse
    /// had already left learns it where it asked rather than at the first
    /// chunk — [`nvs_core_request_body_stream`]'s reasoning about where a
    /// refusal lands, over the one thing a body walk has no equivalent of.
    fn nvs_core_request_part_content(ctx, args: [1]) {
        let ordinal = part_ordinal(args[0], "content")?;
        part_parse(ctx, ordinal, "content")?;
        Ok(crate::instance::build(
            &PART_CONTENT,
            [Value::null(), Value::uint(ordinal)],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<tainted bytes>::iterate(): Iterator<tainted bytes>` — the walk
    /// itself, because a part's next chunk does not exist when it is named.
    fn nvs_core_request_part_content_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &PART_CONTENT, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted bytes>::advance(): bool` — pulls the next run of this
    /// part's bytes, answering `false` at its closing delimiter.
    ///
    /// The chunk is copied into the receiver's slot while it is still the
    /// parse's borrowed span, for [`body_stream_step`]'s reason:
    /// `Multipart::next_chunk` lends it only until the following pull.
    fn nvs_core_request_part_content_advance(ctx, args: [1]) {
        let stepped = part_content_step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<tainted bytes>::current(): tainted bytes` — the chunk the last
    /// `advance()` pulled.
    fn nvs_core_request_part_content_current(_ctx, args: [1]) {
        let read = part_content_chunk(args[0]);
        crate::cursor::consume(args[0]);
        read
    }
}

/// [`PART_CONTENT_ADVANCE_SYMBOL`]'s body: one pull, stored in the receiver's
/// slot, and whether there was anything to store.
///
/// The slot is cleared to `null` at the end of the part, on [`body_stream_step`]'s
/// reasoning: the loop is over, so keeping the last chunk would hold a chunk's
/// worth of a request's memory for as long as the program held the walk.
///
/// # Errors
///
/// [`part_parse`]'s two refusals, `IOError` where the connection failed under
/// the body, and `ParseError` where what arrived is not the multipart body the
/// request declared — [`files_step`]'s split, read off the same flag for the
/// same reason.
fn part_content_step(ctx: &mut Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &PART_CONTENT, member)?;
    let ordinal = crate::instance::slot(receiver, PART_CONTENT_ORDINAL)
        .as_uint()
        .expect("`content()` builds this walk with the ordinal it read off the part");
    let pulled = match part_parse(ctx, ordinal, "content")? {
        None => None,
        Some((parse, body)) => match parse.next_chunk(body) {
            Ok(chunk) => chunk.map(NvsStr::new),
            Err(why) => {
                // No case can reach this: a `.nvst` program answers no request,
                // so it holds no part for a connection to fail under. Asserted
                // by `a_parts_content_that_fails_mid_part_throws_rather_than_ending`.
                let (class, what) = if parse.failed_on_the_wire() {
                    (ThrownClass::Io, "the body did not arrive whole")
                } else {
                    (
                        ThrownClass::Parse,
                        "this is not the multipart body the request declared",
                    )
                };
                // No case can reach this, as above. Asserted by
                // `a_parts_content_that_fails_mid_part_throws_rather_than_ending`.
                return Err(Fault::thrown_as(
                    class,
                    format!("Core\\Request\\Part::content(): {what} — {why}"),
                ));
            }
        },
    };
    let more = pulled.is_some();
    let held = pulled.map_or_else(Value::null, Value::bytes);
    crate::instance::set_slot(receiver, PART_CONTENT_CHUNK, held);
    Ok(Value::bool(more))
}

/// [`PART_CONTENT_CURRENT_SYMBOL`]'s body: the slot [`part_content_step`] last
/// wrote, retained, because the receiver keeps it until the next pull.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not a content walk — only a bug in
/// this crate can produce one, the receiver having been checked at compile time.
fn part_content_chunk(value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::CURRENT;
    let receiver = crate::instance::receiver(value, &PART_CONTENT, member)?;
    let held = crate::instance::slot(receiver, PART_CONTENT_CHUNK);
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

nvs_runtime::nvs_helper! {
    /// `Core\Request\Part::readAll({max?}): tainted bytes` — ADR 0105 § 3's
    /// buffered reading of one upload.
    ///
    /// **Two bounds, and which one applies is what the argument says**, which
    /// is § 3's own split: a bare call is held to `[limits] request_body`,
    /// because what arrives unasked is what that directive governs, and a call
    /// naming a `max` is held to that number and checked against the request
    /// tree's `[limits] memory` instead, because what an application chooses to
    /// hold is what *that* one governs. The bare call is therefore safe by
    /// construction while § 3's deliberate 200M buffer stays expressible, and
    /// neither reading needs a directive of its own.
    ///
    /// **A `max` over the request's memory ceiling is refused, not clamped.** A
    /// clamp would answer a program that asked for 200M with a refusal naming
    /// 64M at some later chunk, which is the same failure one call further from
    /// the mistake; ADR 0106 § 13 clamps because an operator's two directives
    /// disagreeing must not stop a boot, and a program's own call has no such
    /// claim on being started.
    fn nvs_core_request_part_read_all(ctx, args: [2]) {
        let ordinal = part_ordinal(args[0], "readAll")?;
        // unreachable from source: `max` is `CoreTy::Uint` in
        // `READ_ALL_OPTIONS`, so `E0401` refuses anything else at the call site
        // and an absent option arrives as that row's own default. What is left
        // is a lowering bug.
        let asked = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Request\\Part::readAll expected {:?} for its max, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        let bound = read_all_bound(ctx, asked)?;
        let whole = part_read_all(ctx, ordinal, bound)?;
        Ok(Value::bytes(NvsStr::new(&whole)))
    }
}

/// How many bytes `readAll` may hold for this call, and what to call the bound
/// in a refusal — [`nvs_core_request_part_read_all`]'s doc is the decision.
///
/// # Errors
///
/// `LogicError` where an explicit `max` is larger than the whole of what this
/// request may hold, which is a program asking for a buffer the tree it runs
/// in has no room for.
fn read_all_bound(ctx: &Ctx, asked: u64) -> Result<(usize, &'static str), Fault> {
    if asked == 0 {
        return Ok((REQUEST_BODY, "`[limits] request_body`"));
    }
    let memory = ctx.memory_limit();
    let asked = usize::try_from(asked).unwrap_or(usize::MAX);
    if memory > 0 && asked > memory {
        // No case can reach this: a `.nvst` program answers no request, so it
        // holds no part to read. Asserted by
        // `a_read_all_max_over_the_requests_own_memory_is_refused_at_the_call`.
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Request\\Part::readAll(): a `max` of {asked} bytes is larger than \
                 `[limits] memory` ({memory} bytes), which is the whole of what this request \
                 may hold. That is refused here rather than clamped, because a bound the \
                 request cannot honour is a decision to take at the call and not at the chunk \
                 that would cross it"
            ),
        ));
    }
    Ok((asked, "the `max` this call asked for"))
}

/// [`nvs_core_request_part_read_all`]'s body: the part's bytes, pulled to its
/// end under `bound`.
///
/// The bytes over the bound are never held — the refusal happens at the chunk
/// that would cross it, exactly as [`nvs_core_request_body`]'s does, so a part
/// far larger than the bound costs one chunk rather than the part.
///
/// # Errors
///
/// [`part_parse`]'s two refusals, `RuntimeError` past `bound`, and the `IOError`
/// / `ParseError` split [`part_content_step`] makes off the same flag.
fn part_read_all(
    ctx: &mut Ctx,
    ordinal: u64,
    bound: (usize, &'static str),
) -> Result<Vec<u8>, Fault> {
    let (bound, named) = bound;
    let mut whole: Vec<u8> = Vec::new();
    let Some((parse, body)) = part_parse(ctx, ordinal, "readAll")? else {
        return Ok(whole);
    };
    loop {
        match parse.next_chunk(body) {
            Ok(None) => return Ok(whole),
            Ok(Some(chunk)) => {
                if whole.len().saturating_add(chunk.len()) > bound {
                    // No case can reach this: a `.nvst` program answers no
                    // request, so it has no part to send over the bound.
                    // Asserted by `a_parts_read_all_holds_its_bound_on_both_sides`,
                    // on both sides of it.
                    return Err(Fault::thrown(format!(
                        "Core\\Request\\Part::readAll(): this part is larger than {named} \
                         ({bound} bytes), so it is refused rather than held. A part bigger \
                         than what this call may hold is one to walk with `content()` or to \
                         write out with `saveTo()`, neither of which holds more than a chunk"
                    )));
                }
                whole.extend_from_slice(chunk);
            }
            Err(why) => {
                // No case can reach this either, and for `content()`'s reason.
                // Asserted by `a_parts_read_all_that_fails_mid_part_throws_rather_than_ending`.
                let (class, what) = if parse.failed_on_the_wire() {
                    (ThrownClass::Io, "the body did not arrive whole")
                } else {
                    (
                        ThrownClass::Parse,
                        "this is not the multipart body the request declared",
                    )
                };
                // No case can reach this, as above. Asserted by
                // `a_parts_read_all_that_fails_mid_part_throws_rather_than_ending`.
                return Err(Fault::thrown_as(
                    class,
                    format!("Core\\Request\\Part::readAll(): {what} — {why}"),
                ));
            }
        }
    }
}

/// `Core\Request\Part::saveTo`'s member name, in one place: it is what
/// [`crate::io::stream_to_disk`] quotes in every refusal it raises on this
/// member's behalf.
const SAVE_TO: &str = r"Core\Request\Part::saveTo";

nvs_runtime::nvs_helper! {
    /// `Core\Request\Part::saveTo(string $path, {max?, overwrite?}): void` —
    /// [ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
    /// § 4, and the path 99.9% of uploads take.
    ///
    /// **A delegation, not an implementation.** § 4 says `Core\IO::writeStream`
    /// is where a stream reaches disk and that this member delegates to it, so
    /// what is here is the walk `content()` already answers with, handed to
    /// [`crate::io::stream_to_disk`] under the same `{max?, overwrite?}` bag.
    /// The two rules § 4 gives that member — `overwrite` defaulting to `false`,
    /// and a failure removing the partial file — are therefore this member's
    /// too without being restated anywhere: they are one implementation with
    /// two doors, which is the ADR's own argument for writing it that way.
    ///
    /// **The identity check happens before the file is created.**
    /// [`part_parse`] refuses a part the walk has moved past, and it runs first
    /// so that a stale part leaves nothing on disk — `stream_to_disk` opens the
    /// destination before it pulls a chunk, and a refusal after that point
    /// would have to unlink a file the program was right to be refused.
    ///
    /// **What it spends:** one chunk, resident — [`PART_CONTENT`]'s figure,
    /// because this is that walk with a file on the other end of it. `max` is
    /// unbounded by default for [`crate::io::WRITE_STREAM_OPTIONS`]'s reason
    /// and not `readAll`'s: nothing is being held, so there is no memory limit
    /// for a default to inherit from.
    fn nvs_core_request_part_save_to(ctx, args: [4]) {
        let ordinal = part_ordinal(args[0], "saveTo")?;
        // unreachable from source for the reason `readAll`'s two option reads
        // give: the rows are `CoreTy::Text`, `CoreTy::Uint` and `CoreTy::Bool`,
        // so `E0401` refuses anything else at the call site and an absent
        // option arrives as its row's own default.
        let path = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{SAVE_TO} expected {:?} for its path, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?;
        let max = args[2].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{SAVE_TO} expected {:?} for its max, got tag {}",
                Tag::Uint,
                args[2].tag_byte()
            ))
        })?;
        let overwrite = args[3].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "{SAVE_TO} expected {:?} for its overwrite, got tag {}",
                Tag::Bool,
                args[3].tag_byte()
            ))
        })?;
        let path = std::path::Path::new(path);
        part_parse(ctx, ordinal, "saveTo")?;
        let walk = crate::instance::build(&PART_CONTENT, [Value::null(), Value::uint(ordinal)]);
        let wrote = crate::io::stream_to_disk(ctx, path, walk, max, overwrite, SAVE_TO);
        #[expect(
            unsafe_code,
            reason = "the walk was built in this frame and `stream_to_disk` \
                      borrows its source, so this frame owns the one reference \
                      to it on either arm"
        )]
        unsafe {
            walk.release();
        }
        wrote?;
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CLASS, CoreTy, FILES_NAME, MOUNT, MOUNT_NAME, PART_NAME, REQUEST_BODY, cookie_of,
        grouped_fields, joined_field, method_ordinal, nvs_core_request_body,
        nvs_core_request_body_stream, nvs_core_request_body_stream_advance,
        nvs_core_request_body_stream_current, nvs_core_request_body_stream_iterate,
        nvs_core_request_files, nvs_core_request_files_advance, nvs_core_request_files_current,
        nvs_core_request_files_iterate, nvs_core_request_is_head, nvs_core_request_method,
        nvs_core_request_mount, nvs_core_request_mount_captures, nvs_core_request_mount_prefix,
        nvs_core_request_part_content, nvs_core_request_part_content_advance,
        nvs_core_request_part_content_current, nvs_core_request_part_content_iterate,
        nvs_core_request_part_content_type, nvs_core_request_part_filename,
        nvs_core_request_part_name, nvs_core_request_part_read_all, nvs_core_request_part_save_to,
        nvs_core_request_post,
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

    /// A context answering a request that declared `content_type`, with `body`
    /// still on the wire under it.
    fn uploading(content_type: &str, body: Option<Chunks>) -> Ctx {
        let mut inbound = Inbound::new("POST", "/", "");
        inbound.push_header("content-type", content_type.as_bytes());
        if let Some(body) = body {
            inbound.set_body(Box::new(body));
        }
        let mut ctx = Ctx::buffered();
        ctx.set_inbound(inbound);
        ctx
    }

    /// What one file part declared about itself: its field name, the name it
    /// claimed and the type it declared, in the order [`PART`]'s rows read.
    type Declarations = (Vec<u8>, Vec<u8>, Vec<u8>);

    /// Drives the three names a `foreach` over `files()` drives, answering one
    /// row per **file** part: what its three readers said, in row order.
    ///
    /// Nothing here reads a part's bytes, so every part this walks is one the
    /// parse drained on the way to the next — which is exactly ADR 0105 § 1's
    /// "advancing past an unconsumed part drains it", asserted by the walk
    /// finishing rather than by a counter.
    ///
    /// The `Err` is the throw a member raised, as [`walked`]'s is.
    fn parts_of(ctx: &mut Ctx, files: Value) -> Result<Vec<Declarations>, i32> {
        /// The retain a virtual call's receiver owes — see [`crate::cursor`].
        fn lend(value: Value) {
            #[expect(
                unsafe_code,
                reason = "each of the three names consumes a reference, so the \
                          driver holds one of its own and retains per call"
            )]
            unsafe {
                value.retain();
            }
        }
        /// One of the part's three readers, which **borrows** its receiver the
        /// way every registered `Core` member does.
        fn read(
            member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
            ctx: &mut Ctx,
            part: Value,
        ) -> Result<Vec<u8>, i32> {
            let answer = nvs_runtime::call(member, ctx, &[part])?;
            let bytes = answer
                .as_text()
                .expect("a part's readers answer `string`")
                .as_bytes()
                .to_vec();
            #[expect(unsafe_code, reason = "the reader transferred what it answered")]
            unsafe {
                answer.release();
            }
            Ok(bytes)
        }
        lend(files);
        let cursor = nvs_runtime::call(nvs_core_request_files_iterate, ctx, &[files])
            .expect("a walk is its own iterator, so naming it cannot fail");
        let mut seen = Vec::new();
        let walk = loop {
            lend(cursor);
            match nvs_runtime::call(nvs_core_request_files_advance, ctx, &[cursor]) {
                Err(why) => break Err(why),
                Ok(more) if more.as_bool() != Some(true) => break Ok(()),
                Ok(_) => {}
            }
            lend(cursor);
            let part = match nvs_runtime::call(nvs_core_request_files_current, ctx, &[cursor]) {
                Err(why) => break Err(why),
                Ok(part) => part,
            };
            let row = (
                read(nvs_core_request_part_name, ctx, part),
                read(nvs_core_request_part_filename, ctx, part),
                read(nvs_core_request_part_content_type, ctx, part),
            );
            #[expect(unsafe_code, reason = "`current` transferred the part it answered")]
            unsafe {
                part.release();
            }
            match row {
                (Ok(name), Ok(filename), Ok(content_type)) => {
                    seen.push((name, filename, content_type));
                }
                (Err(why), ..) | (_, Err(why), _) | (.., Err(why)) => break Err(why),
            }
        };
        #[expect(
            unsafe_code,
            reason = "the driver owns the reference it was handed and the one \
                      `iterate` answered with, and both are done with here"
        )]
        unsafe {
            cursor.release();
            files.release();
        }
        walk.map(|()| seen)
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

    /// The two members that answer a `HEAD` request, asked **together** and
    /// over the whole roster: no request reports the `Head` case, and `isHead`
    /// is true for exactly the one whose request line carried that token.
    ///
    /// [`every_parsed_verb_is_a_case_of_the_roster`] owns the parse — that
    /// [`method_ordinal`] answers `Get`'s ordinal for `HEAD` — and this owns
    /// what a *program* sees, which is a different claim: the difference the
    /// parse throws away has to come back somewhere, and `isHead` is the only
    /// member that can carry it. So the question is put to both members per
    /// verb rather than to one, because what this exists to catch is
    /// **agreement**. An `isHead` written off `method()`'s answer would report
    /// `false` for every request and still look right on its own line, and a
    /// `method` that stopped folding `HEAD` would leave `isHead` telling the
    /// truth beside it; each half is plausible alone and the pair is useless.
    ///
    /// **`Head` stays a case of the enum**, and that is not in tension with
    /// this. ADR 0077 § 1's roster is what a `#[Route]` may be declared under
    /// and a route may name `Head` there; the rule is only that no request
    /// *reports* it. The arm is therefore reachable from a declaration and
    /// unreachable from this member, which is why the sweep asserts what
    /// `method()` answers and never that [`METHOD`] has one case fewer.
    #[test]
    fn method_reports_get_for_a_head_request_and_is_head_carries_the_truth() {
        /// A context answering a request whose request line carried `verb` and
        /// which is uninteresting in every other way.
        fn asked(verb: &str) -> Ctx {
            let mut ctx = Ctx::buffered();
            ctx.set_inbound(Inbound::new(verb, "/", ""));
            ctx
        }

        let case_of = |wanted: &str| {
            METHOD
                .cases
                .iter()
                .find(|(name, _)| *name == wanted)
                .expect("the roster names the case")
                .1
        };
        let (get, head) = (case_of("Get"), case_of("Head"));

        for (name, ordinal) in METHOD.cases {
            let verb = name.to_ascii_uppercase();
            let is_head = verb == "HEAD";
            let mut ctx = asked(&verb);
            let reported = nvs_runtime::call(nvs_core_request_method, &mut ctx, &[])
                .expect("a roster verb reaches a case")
                .as_int()
                .expect("an enum answers as its ordinal");
            let carried = nvs_runtime::call(nvs_core_request_is_head, &mut ctx, &[])
                .expect("every request being answered can be asked")
                .as_bool()
                .expect("the member answers a bool");

            assert_eq!(
                reported,
                if is_head { get } else { *ordinal },
                "`{verb}` reported the case at {reported}, and every verb but `HEAD` reports \
                 its own"
            );
            assert_ne!(
                reported, head,
                "`{verb}` reported the `Head` case, which a request never does — the case \
                 exists for a `#[Route]` to be declared under and `method()` folds it into \
                 `Get` so that a `Get`-only table still matches one"
            );
            assert_eq!(
                carried, is_head,
                "`{verb}` answered `isHead()` as {carried}: the byte the parse discarded is \
                 this member's whole content, and it is what a handler skips building a body on"
            );
        }
    }

    /// ADR 0102 § 7's two facts as a program reads them: the prefix the door
    /// stripped and the glob captures of the row that stripped it, with the
    /// mark on the captures and not on the prefix.
    ///
    /// **A bound asserted on both sides.** The mounted case and the door that
    /// strips nothing are both here, because `""` and an empty array are the
    /// answer for the second one rather than an absence — which is the whole
    /// of why this member is not nullable where its sibling `route()` is, and
    /// a member that answered `null` there would look right on the first half
    /// alone.
    ///
    /// **The taint half is a declaration, not a value.** A qualifier lives on
    /// the registry row's signature and no [`Value`] carries one, so it is
    /// asserted where it is written. That is also the half `-p nvs-server`
    /// could never have made: its manifest names neither `nvs-stdlib` nor
    /// `nvs-types`, so the door writes two strings with nothing to write a
    /// qualifier with, and `nvs_server::mount::carry`'s own test asserts the
    /// carrying.
    #[test]
    fn core_request_mount_answers_the_prefix_and_its_captures_as_tainted_strings() {
        /// A context answering a request that reached the mount at `prefix`,
        /// whose glob filled `captures`.
        fn served(prefix: &str, captures: &[&str]) -> Ctx {
            let mut ctx = Ctx::buffered();
            let mut inbound = Inbound::new("GET", "/orders", "");
            let filled: Vec<String> = captures.iter().map(|text| (*text).to_owned()).collect();
            inbound.set_mount(prefix, &filled);
            ctx.set_inbound(inbound);
            ctx
        }

        /// What the three members answer together, as Rust values.
        fn read(ctx: &mut Ctx) -> (String, Vec<String>) {
            let mount = nvs_runtime::call(nvs_core_request_mount, ctx, &[])
                .expect("a request being answered reached some mount");
            let prefix = nvs_runtime::call(nvs_core_request_mount_prefix, ctx, &[mount])
                .expect("the receiver is the class the member is declared on");
            let captured = nvs_runtime::call(nvs_core_request_mount_captures, ctx, &[mount])
                .expect("the receiver is the class the member is declared on");
            let held =
                crate::arr::borrowed(captured.array_ptr().expect("`captures()` answers an array"));
            let captures = (0..held.count())
                .map(|slot| {
                    held.get_index(i64::try_from(slot).expect("a capture count fits an i64"))
                        .expect("a list holds every index below its count")
                        .as_text()
                        .expect("a capture is text")
                        .to_owned()
                })
                .collect();
            (
                prefix
                    .as_text()
                    .expect("`prefix()` answers a string")
                    .to_owned(),
                captures,
            )
        }

        let (prefix, captures) = read(&mut served("/acme", &["acme"]));
        assert_eq!(
            prefix, "/acme",
            "the prefix is what ADR 0097 § 4 step 2 took off the path, verbatim"
        );
        assert_eq!(
            captures,
            vec!["acme".to_owned()],
            "`{{1}}` is `captures[0]`, which is § 7's whole spelling of how one \
             table serves many tenants"
        );

        let (bare, none) = read(&mut served("", &[]));
        assert_eq!(
            bare, "",
            "a door that strips nothing answers the empty prefix, not an absence"
        );
        assert!(
            none.is_empty(),
            "a mount whose prefix holds no glob captured nothing, and that is an \
             empty array rather than a `null` mount"
        );

        let row = CLASS
            .methods
            .iter()
            .find(|method| method.name == "mount")
            .expect("the member is registered");
        let CoreTy::Instance(answered) = &row.return_ty else {
            panic!("`mount()` answers an instance class, because § 7's shape has no spelling here")
        };
        assert_eq!(
            *answered, MOUNT_NAME,
            "the row and the class agree on the name, or a program resolves neither"
        );

        let member = |name: &str| {
            MOUNT
                .instance
                .iter()
                .find(|method| method.name == name)
                .unwrap_or_else(|| panic!("`{name}` is declared on the class"))
                .return_ty
        };
        assert!(
            matches!(member("prefix"), CoreTy::Str),
            "the prefix is a row of the table the boot expanded, so it is the \
             operator's text and carries no mark"
        );
        let CoreTy::Array(element) = member("captures") else {
            panic!("`captures()` answers an array")
        };
        assert!(
            matches!(*element, CoreTy::TaintedStr),
            "§ 7 states the captures as `tainted string`, and the qualifier sits on \
             the value a program reaches because there is no `tainted array<T>`"
        );
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
    /// and neither is visible. `rule:errors/cookie-name-bytes`, and the read end of the refusal
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

    /// Spec § 15's exclusivity, asked of all three readings in both directions
    /// and on a request with no body at all: whichever a program takes first is
    /// the one that has the body, and the others are refused rather than
    /// answered empty. Both directions matter, because a record kept by one
    /// member would pass the direction it was written for and fail the other.
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

        // The third reading of the same body is on the same terms as the other
        // two, in both directions: a walk over parts consumes the stream a
        // `body()` would have read, and a `body()` consumes the one it would
        // have parsed.
        let mut parted = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut parted, &[])
            .expect("naming the walk is the reading, and it is the first one here");
        assert!(
            nvs_runtime::call(nvs_core_request_body, &mut parted, &[]).is_err(),
            "`body` after `files` is the program bug § 15 names, not an empty answer"
        );
        assert_eq!(
            parts_of(&mut parted, files)
                .expect("the claimed walk is the one that works")
                .len(),
            2,
            "the member that claimed the body is the one that reads it"
        );

        let mut read_whole = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let whole = nvs_runtime::call(nvs_core_request_body, &mut read_whole, &[])
            .expect("the first reading of a body is the one that gets it");
        #[expect(unsafe_code, reason = "the call transferred the reference it answered")]
        unsafe {
            whole.release();
        }
        assert!(
            nvs_runtime::call(nvs_core_request_files, &mut read_whole, &[]).is_err(),
            "`files` after `body` is refused in the same direction as its twin"
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

    /// `bodyStream` against the other two readings, in both directions and
    /// with **no chunk ever pulled** — ADR 0105 § 3's second of three ways,
    /// held to spec § 15.
    ///
    /// The pair its twin above leaves unasked is `bodyStream` and `files`,
    /// which is the one where an implementation could plausibly let both
    /// through: neither member moves a byte, so a claim taken at the first pull
    /// rather than at the naming would refuse nothing here and then hand a
    /// multipart parse a stream a `foreach` was already walking. That is
    /// `claim_body`'s own doc asserted rather than restated — the claim is
    /// taken where the reading is *named* — and nothing in this test reads a
    /// byte, so what can fail is the rule and never the parse.
    ///
    /// The `body` half is asked on the same never-pulled terms, which is what
    /// it adds over the direction its twin already walks.
    ///
    /// The refusals' own wording is not read here, for the reason
    /// `the_request_body_cap_is_the_last_body_read_and_the_first_one_refused`
    /// gives: a message needs an exception class table installed on the
    /// context first, and what this asks about is the claim.
    #[test]
    fn body_stream_is_exclusive_with_body_and_with_files() {
        let mut named = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let unwalked = nvs_runtime::call(nvs_core_request_body_stream, &mut named, &[])
            .expect("naming the walk is the reading, and it is the first one here");
        dropped(unwalked);
        assert!(
            nvs_runtime::call(nvs_core_request_files, &mut named, &[]).is_err(),
            "`files` after `bodyStream` is refused though the walk pulled nothing"
        );

        let mut walking = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut walking, &[])
            .expect("a request that declared a multipart body can be walked");
        dropped(files);
        assert!(
            nvs_runtime::call(nvs_core_request_body_stream, &mut walking, &[]).is_err(),
            "and `bodyStream` after `files` is refused on the same terms, the other way round"
        );

        let mut whole = answering(Some(Chunks::of(&[&b"a body"[..]])));
        let stream = nvs_runtime::call(nvs_core_request_body_stream, &mut whole, &[])
            .expect("naming the walk cannot fail on a request that carried a body");
        dropped(stream);
        assert!(
            nvs_runtime::call(nvs_core_request_body, &mut whole, &[]).is_err(),
            "`body` after an unwalked `bodyStream` reads a stream that member already owns"
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

    /// One multipart body, split across the wire the way a real one arrives:
    /// a form field, a file part that declares its type, and a second file part
    /// that declares none.
    const UPLOAD: &[&[u8]] = &[
        b"--X\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nQ3 report\r\n--X\r\n\
          Content-Disposition: form-data; name=\"doc\"; filename=\"report.pdf\"\r\n\
          Content-Type: application/pdf\r\n\r\n%PDF-1.4 and ",
        b"the rest of it\r\n--X\r\n\
          Content-Disposition: form-data; name=\"notes\"; filename=\"notes.txt\"\r\n\r\n\
          jotted down\r\n--X--\r\n",
    ];

    /// The walk yields ADR 0105 § 2's **file** parts and only those: the form
    /// field is buffered as the walk passes it and never appears as a part, and
    /// a part that declared no type answers RFC 7578 § 4.4's default rather
    /// than nothing. Beside it, the two requests that walk empty — a request
    /// whose `Content-Type` is not multipart at all, and one that declared a
    /// multipart body and then sent none.
    ///
    /// The parts are walked without their bytes ever being read, which is § 1's
    /// drain: a walk that could not skip an unconsumed part would stall on the
    /// first one here.
    ///
    /// The receiver is driven by hand rather than by a `.nvst` `foreach`,
    /// because a case is a program with no request in front of it — the
    /// `ASSERTED_OFF_THE_CORPUS` reading `conformance_coverage.rs` owns.
    #[test]
    fn a_files_walk_yields_the_file_parts_and_drains_what_it_passes() {
        let mut arriving = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
            .expect("a request that declared a multipart body can be walked");
        assert_eq!(
            parts_of(&mut arriving, files).expect("a body that arrives whole walks whole"),
            vec![
                (
                    b"doc".to_vec(),
                    b"report.pdf".to_vec(),
                    b"application/pdf".to_vec()
                ),
                (
                    b"notes".to_vec(),
                    b"notes.txt".to_vec(),
                    b"text/plain".to_vec()
                ),
            ],
            "a part is a file part iff it declared a `filename`, and `title` declared none"
        );

        let mut urlencoded = uploading(
            "application/x-www-form-urlencoded",
            Some(Chunks::of(&[&b"title=Q3+report"[..]])),
        );
        let none = nvs_runtime::call(nvs_core_request_files, &mut urlencoded, &[])
            .expect("a request that is not multipart is still a request");
        assert!(
            parts_of(&mut urlencoded, none)
                .expect("a body with no parts in it is no parts")
                .is_empty(),
            "\"this request sent no files\" is an empty walk, and only \"no request\" is a throw"
        );

        let mut bodiless = uploading("multipart/form-data; boundary=X", None);
        let empty = nvs_runtime::call(nvs_core_request_files, &mut bodiless, &[])
            .expect("a request that carried no body is still a request");
        assert!(
            parts_of(&mut bodiless, empty)
                .expect("no body is no parts")
                .is_empty(),
            "a declared multipart body that never arrived is no parts, not a refusal"
        );
    }

    /// ADR 0105 § 1's two halves in one test, because they are one sentence: the
    /// walk is **lazy**, and it is the **only** way an uploaded file reaches a
    /// program.
    ///
    /// Laziness is asserted against a body whose second pull never lands. The
    /// first file part's header arrived inside the first chunk, so the walk
    /// yields it and its declarations read back — and the failure lands on the
    /// `advance()` that needed the chunk after it. A walk that buffered the body
    /// before answering could not have answered at all here, which is what makes
    /// the *succeeding* half of this the interesting one:
    /// `a_files_walk_that_fails_mid_body_throws_rather_than_ending` already owns
    /// the throw.
    ///
    /// "The only way in" is asked of the registry rather than of a body, by
    /// **counting** rather than by reading one row: exactly one member in the
    /// whole `Core` surface answers the walk, and no member anywhere answers a
    /// part — a part exists only as the walk's current one, which is what § 3's
    /// ordinal check is written around. A second door added later fails this
    /// without anyone having to remember the rule, which a test naming
    /// `Core\Request::files` alone would not.
    #[test]
    fn files_is_a_lazy_iterator_and_the_only_way_to_receive_an_upload() {
        /// The retain a virtual call's receiver owes — [`parts_of`]'s `lend`,
        /// which this test needs one step at a time rather than as a loop.
        fn lend(value: Value) {
            #[expect(
                unsafe_code,
                reason = "each of the three names consumes a reference, so the \
                          driver holds one of its own and retains per call"
            )]
            unsafe {
                value.retain();
            }
        }
        /// A reference a member transferred, given back.
        fn spend(value: Value) {
            #[expect(unsafe_code, reason = "the call transferred what it answered")]
            unsafe {
                value.release();
            }
        }

        let mut arriving = uploading(
            "multipart/form-data; boundary=X",
            Some(Chunks::failing_at(&UPLOAD[..1], 1)),
        );
        let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
            .expect("the failure is the wire's, and it has not happened yet");
        lend(files);
        let cursor = nvs_runtime::call(nvs_core_request_files_iterate, &mut arriving, &[files])
            .expect("a walk is its own iterator, so naming it cannot fail");
        lend(cursor);
        let opened = nvs_runtime::call(nvs_core_request_files_advance, &mut arriving, &[cursor])
            .expect("the first file part's header arrived inside the first chunk");
        assert_eq!(
            opened.as_bool(),
            Some(true),
            "a body holding a whole part header holds a part, however it ends later"
        );
        lend(cursor);
        let part = nvs_runtime::call(nvs_core_request_files_current, &mut arriving, &[cursor])
            .expect("a walk that opened a part is on one");
        let claimed = nvs_runtime::call(nvs_core_request_part_filename, &mut arriving, &[part])
            .expect("a part the walk yielded reads back its own declarations");
        assert_eq!(
            claimed
                .as_text()
                .expect("`filename` answers `string`")
                .as_bytes(),
            b"report.pdf",
            "the part yielded before the rest of the body is the first one on the wire"
        );
        spend(claimed);
        spend(part);
        lend(cursor);
        assert!(
            nvs_runtime::call(nvs_core_request_files_advance, &mut arriving, &[cursor]).is_err(),
            "the second part needed a chunk the connection never delivered"
        );
        spend(cursor);
        spend(files);

        let answering = |name: &'static str| {
            crate::registry::CLASSES
                .iter()
                .flat_map(|class| {
                    class
                        .methods
                        .iter()
                        .chain(class.instance)
                        .map(move |member| (class.name, member))
                })
                .filter(|(_, member)| answers(&member.return_ty, name))
                .map(|(class, member)| format!("{class}::{}", member.name))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            answering(FILES_NAME),
            vec![r"Core\Request::files".to_owned()],
            "one door, and a second one is a second way to receive an upload"
        );
        assert!(
            answering(PART_NAME).is_empty(),
            "a part is reached by walking and by nothing that hands one out: {:?}",
            answering(PART_NAME)
        );
    }

    /// ADR 0105 § 1's other half: **there is no temp file**, so nothing hands a
    /// program a path the host chose and there is nothing left over to move.
    ///
    /// Asked of the rosters rather than of a body — the way the walk's own half
    /// above asks its question — and by **counting** rather than by naming the
    /// row that must not be there: a filesystem path crosses this surface
    /// exactly once, as `saveTo`'s *parameter*, which is the destination the
    /// application chose and § 1's whole argument. A `tmpName` added later
    /// fails this without anyone having to remember the rule.
    ///
    /// The sweep is over spellings rather than over types because no
    /// [`CoreTy`] says "path": what separates a host-stored path from any other
    /// text is what it is called. `path` alone is deliberately not one of them
    /// — `Core\Request::path` is the URL's, which is the peer speaking and not
    /// the disk.
    ///
    /// `move_uploaded_file` is swept for over the whole registry instead of
    /// these two classes: Novis has no global functions, so the only place
    /// PHP's name could come back is as some class's member, and `CLASSES` is
    /// where the members are. The second `Core` roster,
    /// `nvs_hir::errors::TREE`, is exception classes with no members at all,
    /// so a name cannot hide there.
    #[test]
    fn there_is_no_temp_path_and_no_move_uploaded_file() {
        /// The spellings a host-stored upload arrives under: PHP's own
        /// `tmp_name`, and the ports of it a port would reach for.
        const HOST_PATHS: &[&str] = &[
            "tmpname",
            "tmpfile",
            "tmppath",
            "tempname",
            "tempfile",
            "temppath",
            "temporaryname",
            "temporaryfile",
            "temporarypath",
            "localpath",
            "diskpath",
            "storedpath",
            "savedpath",
            "filepath",
            "realpath",
            "pathname",
        ];

        /// A name with its case and its underscores taken out, so that
        /// `tmp_name` and `tmpName` are one claim rather than two.
        fn flattened(name: &str) -> String {
            name.chars()
                .filter(|character| *character != '_')
                .collect::<String>()
                .to_ascii_lowercase()
        }

        let surface: Vec<_> = [&super::CLASS, &super::PART]
            .into_iter()
            .flat_map(|class| {
                class
                    .methods
                    .iter()
                    .chain(class.instance)
                    .map(|member| (class.name, member))
            })
            .collect();

        let answering_a_path = surface
            .iter()
            .filter(|(_, member)| HOST_PATHS.contains(&flattened(member.name).as_str()))
            .map(|(class, member)| format!("{class}::{}", member.name))
            .collect::<Vec<_>>();
        assert!(
            answering_a_path.is_empty(),
            "an upload is a stream, so nothing may answer where the host put it: {answering_a_path:?}"
        );

        let taking_a_path = surface
            .iter()
            .filter(|(_, member)| {
                member
                    .names
                    .iter()
                    .any(|name| flattened(name).contains("path"))
            })
            .map(|(class, member)| format!("{class}::{}", member.name))
            .collect::<Vec<_>>();
        assert_eq!(
            taking_a_path,
            vec![format!("{PART_NAME}::saveTo")],
            "a path crosses this surface once and inward, into the one member the application \
             names its own destination through"
        );

        let moving = crate::registry::CLASSES
            .iter()
            .flat_map(|class| {
                class
                    .methods
                    .iter()
                    .chain(class.instance)
                    .map(move |member| format!("{}::{}", class.name, member.name))
            })
            .chain(
                crate::registry::CLASSES
                    .iter()
                    .map(|class| class.name.to_owned()),
            )
            .filter(|name| flattened(name).contains("moveuploaded"))
            .collect::<Vec<_>>();
        assert!(
            moving.is_empty(),
            "there is no temp file left behind, so there is nothing for a move to be about: {moving:?}"
        );
    }

    /// Whether `ty` is the instance type called `name`, through a `?` where the
    /// member's answer is nullable.
    fn answers(ty: &CoreTy, name: &str) -> bool {
        match ty {
            CoreTy::Instance(answered) => *answered == name,
            CoreTy::Nullable(inner) => answers(inner, name),
            _ => false,
        }
    }

    /// A request that says it is multipart and then does not say how to read one
    /// is refused where it was named, not walked as far as the ambiguity —
    /// `rule:errors/ambiguous-input-refused`, and the split `crate::multipart::is_multipart` exists to make.
    /// The `boundary` is the one token the parse trusts to appear inside a body,
    /// so guessing at a missing one would be a truncation rule chosen by the
    /// peer.
    ///
    /// The refusal lands on `files()` itself rather than on the first
    /// `advance()`, for `bodyStream`'s reason: naming the walk is the reading.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so
    /// `inbound_of` refuses it before it reaches the `Content-Type`.
    #[test]
    fn a_multipart_body_that_declares_no_boundary_is_refused_where_it_is_named() {
        let mut unreadable = uploading(
            "multipart/form-data",
            Some(Chunks::of(&[&b"--X--\r\n"[..]])),
        );
        assert!(
            nvs_runtime::call(nvs_core_request_files, &mut unreadable, &[]).is_err(),
            "a multipart body with no boundary is ambiguous, and ambiguity is refused"
        );

        let mut malformed = uploading(
            "multipart/form-data; boundary=X",
            Some(Chunks::of(&[&b"there is no delimiter in here at all"[..]])),
        );
        let files = nvs_runtime::call(nvs_core_request_files, &mut malformed, &[])
            .expect("the `Content-Type` was readable; the body is what is not");
        assert!(
            parts_of(&mut malformed, files).is_err(),
            "a body that ends anywhere but after its closing delimiter is refused"
        );
    }

    /// A walk that fails mid-body throws rather than ending, which is
    /// `a_body_stream_that_fails_mid_walk_throws_rather_than_ending`'s property
    /// on the third reading of the same body: `advance()` answering `false`
    /// means the parts are over, so reporting a dead connection that way would
    /// tell a loop it had seen every file the peer sent.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it can
    /// hold no walk for a connection to fail under.
    #[test]
    fn a_files_walk_that_fails_mid_body_throws_rather_than_ending() {
        let mut cut_off = uploading(
            "multipart/form-data; boundary=X",
            Some(Chunks::failing_at(&UPLOAD[..1], 1)),
        );
        let files = nvs_runtime::call(nvs_core_request_files, &mut cut_off, &[])
            .expect("the failure is the wire's, and it has not happened yet");
        assert!(
            parts_of(&mut cut_off, files).is_err(),
            "a connection that failed under an upload did not deliver the end of one"
        );
    }

    /// A multipart body whose one file part stops in the middle of its bytes:
    /// the header block and the start of the content arrive, and the pull after
    /// them is the connection dying.
    const CUT_OFF: &[&[u8]] = &[
        b"--X\r\nContent-Disposition: form-data; name=\"doc\"; filename=\"report.pdf\"\r\n\r\n\
          the first half",
    ];

    /// The retain a virtual call's receiver owes — see [`crate::cursor`]. The
    /// three names a `foreach` drives consume a reference each; a registered
    /// `Core` member borrows its receiver and is called without this.
    fn lend(value: Value) {
        #[expect(
            unsafe_code,
            reason = "each of the three names consumes a reference, so the driver \
                      holds one of its own and retains per call"
        )]
        unsafe {
            value.retain();
        }
    }

    /// Drops a reference the driver owns.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the driver owns this reference and is done with it"
        )]
        unsafe {
            value.release();
        }
    }

    /// Advances `files` once and answers the part it opened — the two names a
    /// `foreach` drives per iteration, without the third that names the walk.
    ///
    /// The walk is its own iterator, so the value handed in is the cursor and
    /// there is nothing to build beside it.
    fn next_part(ctx: &mut Ctx, files: Value) -> Result<Value, i32> {
        lend(files);
        let more = nvs_runtime::call(nvs_core_request_files_advance, ctx, &[files])?;
        assert_eq!(
            more.as_bool(),
            Some(true),
            "the walk was asked for a part it does not have"
        );
        lend(files);
        nvs_runtime::call(nvs_core_request_files_current, ctx, &[files])
    }

    /// Drives the three names a `foreach` over `$part->content()` drives,
    /// answering one entry per chunk the part yielded.
    ///
    /// The `Err` is the throw a member raised, as [`walked`]'s is — either the
    /// walk being named on a part the parse has left, or the body failing under
    /// it.
    fn content_of(ctx: &mut Ctx, part: Value) -> Result<Vec<Vec<u8>>, i32> {
        let walk = nvs_runtime::call(nvs_core_request_part_content, ctx, &[part])?;
        lend(walk);
        let cursor = nvs_runtime::call(nvs_core_request_part_content_iterate, ctx, &[walk])
            .expect("a content walk is its own iterator, so naming it cannot fail");
        let mut seen = Vec::new();
        let walked = loop {
            lend(cursor);
            match nvs_runtime::call(nvs_core_request_part_content_advance, ctx, &[cursor]) {
                Err(why) => break Err(why),
                Ok(more) if more.as_bool() != Some(true) => break Ok(()),
                Ok(_) => {}
            }
            lend(cursor);
            match nvs_runtime::call(nvs_core_request_part_content_current, ctx, &[cursor]) {
                Err(why) => break Err(why),
                Ok(chunk) => {
                    seen.push(chunk.as_bytes().expect("a chunk is `bytes`").to_vec());
                    dropped(chunk);
                }
            }
        };
        dropped(cursor);
        dropped(walk);
        walked.map(|()| seen)
    }

    /// `readAll` on `part`, at `max` bytes — `0` being the absent option, which
    /// is [`READ_ALL_OPTIONS`]'s own default and the bare call.
    fn read_all(ctx: &mut Ctx, part: Value, max: u64) -> Result<Vec<u8>, i32> {
        let answer = nvs_runtime::call(
            nvs_core_request_part_read_all,
            ctx,
            &[part, Value::uint(max)],
        )?;
        let bytes = answer
            .as_bytes()
            .expect("`readAll` answers `bytes`")
            .to_vec();
        dropped(answer);
        Ok(bytes)
    }

    /// § 3's two consumers over the part the walk is on: the content walk
    /// yields the part's bytes and nothing of the delimiter that ends them, the
    /// walk carries on to the next part afterwards, and `readAll` answers the
    /// same bytes in one value.
    ///
    /// The two parts are read the two different ways on purpose — what would
    /// otherwise go unasserted is that a part *consumed* by one of them leaves
    /// the parse where the outer walk can still find the next one, which is the
    /// same seam § 1's drain sits on.
    ///
    /// The receivers are driven by hand rather than by a `.nvst` `foreach`,
    /// because a case is a program with no request in front of it — the
    /// `ASSERTED_OFF_THE_CORPUS` reading `conformance_coverage.rs` owns.
    #[test]
    fn a_parts_content_walks_the_bytes_of_the_part_the_walk_is_on() {
        let mut arriving = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
            .expect("a request that declared a multipart body can be walked");

        let first = next_part(&mut arriving, files).expect("the body carries two file parts");
        let chunks =
            content_of(&mut arriving, first).expect("a part that arrives whole walks whole");
        assert_eq!(
            chunks.concat(),
            b"%PDF-1.4 and the rest of it".to_vec(),
            "a content walk yields the part's own bytes, and the delimiter is not one of them"
        );
        dropped(first);

        let second = next_part(&mut arriving, files)
            .expect("a part read to its end leaves the walk where the next one starts");
        assert_eq!(
            read_all(&mut arriving, second, 0).expect("a part inside the bound is held whole"),
            b"jotted down".to_vec(),
            "`readAll` is the same pull with a buffer around it"
        );
        dropped(second);
        dropped(files);
    }

    /// ADR 0105 § 3's "valid only while this part is the iterator's current
    /// one", asked of both consumers: a part the walk has moved past refuses
    /// rather than answering the current part's bytes.
    ///
    /// The silent wrong answer is the whole point of the check. Nothing about a
    /// stale part *looks* stale — its three declarations still read back as
    /// they did — so a program that kept one and read it a loop later would be
    /// handed the next upload's bytes under the previous upload's filename.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it can
    /// hold no part at all.
    #[test]
    fn a_part_the_walk_has_moved_past_refuses_rather_than_reading_the_current_one() {
        let mut arriving = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
            .expect("a request that declared a multipart body can be walked");
        let stale = next_part(&mut arriving, files).expect("the body carries two file parts");
        let current = next_part(&mut arriving, files).expect("and the walk reaches the second");

        assert!(
            content_of(&mut arriving, stale).is_err(),
            "a part the parse has left has no bytes, and the current part's are not its own"
        );
        assert!(
            read_all(&mut arriving, stale, 0).is_err(),
            "both consumers ask the same question, so both refuse the same part"
        );
        assert_eq!(
            read_all(&mut arriving, current, 0).expect("the part the walk is on is still readable"),
            b"jotted down".to_vec(),
            "the refusal is about which part it is, not about the walk having been used"
        );

        dropped(stale);
        dropped(current);
        dropped(files);
    }

    /// A part whose body stops mid-stream throws rather than ending, which is
    /// `a_body_stream_that_fails_mid_walk_throws_rather_than_ending`'s property
    /// one level in: `advance()` answering `false` means the *part* is over, so
    /// reporting a dead connection that way would hand a program a truncated
    /// upload it had no way to know was truncated.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it
    /// holds no part for a connection to fail under.
    #[test]
    fn a_parts_content_that_fails_mid_part_throws_rather_than_ending() {
        let mut cut_off = uploading(
            "multipart/form-data; boundary=X",
            Some(Chunks::failing_at(CUT_OFF, 1)),
        );
        let files = nvs_runtime::call(nvs_core_request_files, &mut cut_off, &[])
            .expect("the failure is the wire's, and it has not happened yet");
        let part = next_part(&mut cut_off, files).expect("the part's header block did arrive");
        assert!(
            content_of(&mut cut_off, part).is_err(),
            "a connection that failed under a part did not deliver the end of one"
        );
        dropped(part);
        dropped(files);
    }

    /// `readAll`'s answer to the same failure, and it is the same answer: the
    /// prefix that arrived is not the part, so it is a throw rather than a
    /// shorter value — `a_body_that_fails_mid_stream_throws_rather_than_answering_its_prefix`
    /// over one part instead of one body.
    ///
    /// No case can reach this, for the reason above.
    #[test]
    fn a_parts_read_all_that_fails_mid_part_throws_rather_than_ending() {
        let mut cut_off = uploading(
            "multipart/form-data; boundary=X",
            Some(Chunks::failing_at(CUT_OFF, 1)),
        );
        let files = nvs_runtime::call(nvs_core_request_files, &mut cut_off, &[])
            .expect("the failure is the wire's, and it has not happened yet");
        let part = next_part(&mut cut_off, files).expect("the part's header block did arrive");
        assert!(
            read_all(&mut cut_off, part, 0).is_err(),
            "a part that stopped short is not a part that was smaller than it said"
        );
        dropped(part);
        dropped(files);
    }

    /// ADR 0105 § 3's bound, named on both sides: a part of exactly `max` is
    /// held, and the same part against one byte less is refused. A member that
    /// stopped one byte early — or one late — prints plausibly against either
    /// half on its own.
    ///
    /// The refusal happens at the chunk that would cross the bound, so the
    /// second half of this also asserts that a part far over the bound costs a
    /// chunk rather than a part; there is no larger fixture here because what
    /// the size would demonstrate is the arithmetic these two calls already pin.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it has
    /// no part to send over a bound.
    #[test]
    fn a_parts_read_all_holds_its_bound_on_both_sides() {
        const CONTENT: &[u8] = b"%PDF-1.4 and the rest of it";

        let mut at_the_bound =
            uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut at_the_bound, &[])
            .expect("a request that declared a multipart body can be walked");
        let part = next_part(&mut at_the_bound, files).expect("the body carries two file parts");
        let held = read_all(&mut at_the_bound, part, CONTENT.len() as u64)
            .expect("a part of exactly the bound has not crossed it");
        assert_eq!(
            held,
            CONTENT.to_vec(),
            "the last part inside the bound is held whole"
        );
        dropped(part);
        dropped(files);

        let mut over_it = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut over_it, &[])
            .expect("a request that declared a multipart body can be walked");
        let part = next_part(&mut over_it, files).expect("the body carries two file parts");
        assert!(
            read_all(&mut over_it, part, CONTENT.len() as u64 - 1).is_err(),
            "one byte past the bound in force is still past it"
        );
        dropped(part);
        dropped(files);
    }

    /// § 3's second bound: a `max` larger than the whole of what this request
    /// may hold is refused **at the call**, before a byte is pulled, and the
    /// part is still readable under a bound the request can honour.
    ///
    /// The second half is what says the refusal is arithmetic rather than a
    /// state change — a check that consumed the part on its way to refusing
    /// would pass the first assertion and fail every program that caught the
    /// throw and retried inside its means.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it
    /// holds no part to read.
    #[test]
    fn a_read_all_max_over_the_requests_own_memory_is_refused_at_the_call() {
        let mut arriving = uploading("multipart/form-data; boundary=X", Some(Chunks::of(UPLOAD)));
        arriving.set_memory_limit(1 << 20);
        let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
            .expect("a request that declared a multipart body can be walked");
        let part = next_part(&mut arriving, files).expect("the body carries two file parts");

        assert!(
            read_all(&mut arriving, part, 4 << 20).is_err(),
            "a buffer larger than `[limits] memory` is one this request cannot hold"
        );
        assert_eq!(
            read_all(&mut arriving, part, 512).expect("a bound the request can honour is honoured"),
            b"%PDF-1.4 and the rest of it".to_vec(),
            "the refused call pulled nothing, so the part is where it was"
        );
        dropped(part);
        dropped(files);
    }

    /// ADR 0105 § 3's three consumers, asked as **agreement**: the same part on
    /// the same wire hands back the same octets whichever of them reads it, and
    /// the rows that read the octets at all are those three.
    ///
    /// The three are one walk with three doors — `content()` *is* the walk,
    /// `readAll` is that walk collected under a bound, and `saveTo` is that
    /// walk with a file on the end of it — which is exactly what makes this
    /// worth asserting rather than obvious. Three doors onto one
    /// implementation are indistinguishable until one of them grows a step of
    /// its own, and a `readAll` that trimmed, a `saveTo` that wrote a chunk
    /// twice or a `content()` that dropped the last one are each plausible
    /// against their own case while failing here.
    ///
    /// **Asked over three requests rather than three calls**, because
    /// consuming is what these members do: a part is the walk's current one and
    /// its octets are still on the wire, so a second reader of one part would
    /// be asserting the claim rule instead — which is
    /// [`a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other`]'s
    /// at the body. The three wires are the same `UPLOAD` bytes, so "the same
    /// part" is a fact here and not an assumption.
    ///
    /// **Which rows the three are is read off [`PART`] rather than written
    /// out**: a row that reaches the octets answers the walk, the bytes, or the
    /// `void` of having put them somewhere else, while the three answering
    /// `tainted string` are what the peer *declared* about the part and read
    /// nothing. A seventh row lands in one partition or the other and is looked
    /// at either way. That nothing outside these rows can be handed a part at
    /// all is
    /// [`files_is_a_lazy_iterator_and_the_only_way_to_receive_an_upload`]'s
    /// closing assertion and is not restated here.
    #[test]
    fn a_part_is_consumed_by_read_all_by_iteration_or_by_save_to() {
        let (mut consumers, mut declarations) = (Vec::new(), Vec::new());
        for member in super::PART.methods.iter().chain(super::PART.instance) {
            if matches!(member.return_ty, CoreTy::TaintedStr) {
                declarations.push(member.name);
            } else {
                consumers.push(member.name);
            }
        }
        assert_eq!(
            consumers,
            vec!["content", "readAll", "saveTo"],
            "§ 3's consumers are three, and the partition is by what the row answers: the walk \
             itself, the octets held whole, and the `void` of having written them to a \
             destination the application named"
        );
        assert_eq!(
            declarations,
            vec!["name", "filename", "contentType"],
            "the rest of the class is what the peer said about the part, which reads none of it \
             — a row that answered `tainted string` while pulling a chunk would be a fourth \
             consumer nobody could see"
        );

        /// The first part of an `UPLOAD` body, consumed by `consume` and
        /// answered as the octets that came back.
        fn through(consume: impl FnOnce(&mut Ctx, Value) -> Vec<u8>) -> Vec<u8> {
            let mut arriving = saving(Chunks::of(UPLOAD));
            let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
                .expect("a request that declared a multipart body can be walked");
            let part = next_part(&mut arriving, files).expect("the body carries two file parts");
            let octets = consume(&mut arriving, part);
            dropped(part);
            dropped(files);
            octets
        }

        let walked = through(|ctx, part| {
            content_of(ctx, part)
                .expect("a part still on the wire walks")
                .concat()
        });
        let held = through(|ctx, part| {
            read_all(ctx, part, u64::MAX).expect("a part still on the wire is read whole")
        });
        let saved = through(|ctx, part| {
            let path = scratch("consumed-once.pdf");
            save_to(ctx, part, &path, false).expect("a granted destination is written");
            std::fs::read(&path).expect("the file `saveTo` made")
        });

        assert!(
            !walked.is_empty(),
            "the fixture has to deliver something for the agreement below to mean anything"
        );
        assert_eq!(
            walked, held,
            "the bound collects the walk and adds nothing: `readAll` is `content()` with a \
             ceiling, so a chunk boundary is as invisible to one as to the other"
        );
        assert_eq!(
            held, saved,
            "ADR 0105 § 4's delegation carries the same octets to disk that a program reading \
             them into memory would have seen, which is what makes the choice between them a \
             memory decision and nothing else"
        );
    }

    /// A context answering a multipart request **and** granting `fs.write`,
    /// which is what `saveTo` needs and no other member of this module does.
    ///
    /// The grant is everywhere rather than under a root, for
    /// `crate::io::tests::writing`'s reason: ADR 0118's own suite is where the
    /// grant decides anything, and a root here would only add a way for these
    /// cases to fail for a reason they are not about.
    fn saving(body: Chunks) -> Ctx {
        let mut ctx = uploading("multipart/form-data; boundary=X", Some(body));
        ctx.set_config(std::sync::Arc::new(nvs_config::Snapshot {
            config: nvs_config::tree::Config {
                capabilities: Some(nvs_config::tree::Capabilities {
                    fs: Some(nvs_config::tree::CapFs {
                        read: None,
                        write: Some(nvs_config::tree::Setting::Bool(true)),
                    }),
                    ..nvs_config::tree::Capabilities::default()
                }),
                ..nvs_config::tree::Config::default()
            },
            ..nvs_config::Snapshot::default()
        }));
        ctx
    }

    /// A destination one case owns, with anything a previous run left there
    /// removed.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("nvs-part-save-to");
        std::fs::create_dir_all(&dir).expect("a temporary directory the tests own");
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    /// `saveTo` on `part`, with the four slots the row flattens to.
    fn save_to(
        ctx: &mut Ctx,
        part: Value,
        path: &std::path::Path,
        overwrite: bool,
    ) -> Result<(), i32> {
        let written = Value::str(nvs_runtime::NvsStr::new(path.to_string_lossy().as_bytes()));
        let answered = nvs_runtime::call(
            nvs_core_request_part_save_to,
            ctx,
            &[part, written, Value::uint(u64::MAX), Value::bool(overwrite)],
        );
        dropped(written);
        answered.map(|answer| {
            assert_eq!(
                answer.tag_byte(),
                Value::null().tag_byte(),
                "`saveTo` answers nothing"
            );
        })
    }

    /// ADR 0105 § 4's delegation, from this end of it: the part's bytes reach
    /// the destination whole, and a second call to the same name is refused
    /// because `overwrite` defaults to `false`.
    ///
    /// The default is asserted here as well as in `crate::io`'s own suite
    /// because it is a *row* on this side — `saveTo` names
    /// [`crate::io::WRITE_STREAM_OPTIONS`] rather than declaring a pair, and a
    /// copy that had drifted would pass over there and fail a program here.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it
    /// holds no part to write.
    #[test]
    fn save_to_writes_the_part_whole_and_defaults_to_not_replacing() {
        let mut arriving = saving(Chunks::of(UPLOAD));
        let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
            .expect("a request that declared a multipart body can be walked");
        let part = next_part(&mut arriving, files).expect("the body carries two file parts");
        let path = scratch("upload.pdf");

        save_to(&mut arriving, part, &path, false).expect("a granted destination is written");
        assert_eq!(
            std::fs::read(&path).expect("the file `saveTo` made"),
            b"%PDF-1.4 and the rest of it".to_vec(),
            "the part reaches disk whole, in order, and decoded by nothing"
        );

        assert!(
            save_to(&mut arriving, part, &path, false).is_err(),
            "a caller who said nothing about `overwrite` does not replace what is there"
        );
        assert_eq!(
            std::fs::read(&path).expect("the file"),
            b"%PDF-1.4 and the rest of it".to_vec(),
            "and the refusal happened before anything was written"
        );

        dropped(part);
        dropped(files);
        let _ = std::fs::remove_file(&path);
    }

    /// § 3's identity, on the third consumer: a part the walk has moved past
    /// refuses, and it refuses **before the destination is created**.
    ///
    /// The second half is the one worth a case. `stream_to_disk` opens the file
    /// before it pulls a chunk, so an identity check made any later would have
    /// left an empty file at a name the program was right to be refused — a
    /// destination created by a call that failed is exactly the partial write
    /// § 4's cleanup exists to prevent, arriving through the door that cleanup
    /// does not cover.
    ///
    /// No case can reach this: a `.nvst` program answers no request, so it
    /// holds no part to keep past its walk.
    #[test]
    fn save_to_refuses_a_part_the_walk_has_moved_past_and_creates_nothing() {
        let mut arriving = saving(Chunks::of(UPLOAD));
        let files = nvs_runtime::call(nvs_core_request_files, &mut arriving, &[])
            .expect("a request that declared a multipart body can be walked");
        let stale = next_part(&mut arriving, files).expect("the body carries two file parts");
        let current = next_part(&mut arriving, files).expect("and the walk reaches the second");
        let path = scratch("stale.bin");

        assert!(
            save_to(&mut arriving, stale, &path, false).is_err(),
            "the first part's bytes are gone once the walk has opened the second"
        );
        assert!(
            !path.exists(),
            "a refused part leaves no destination behind: {}",
            path.display()
        );

        save_to(&mut arriving, current, &path, false)
            .expect("the part the walk is on is still writable");
        assert!(
            path.exists(),
            "and the refusal is the stamp's, not this fixture being unable to write at all"
        );

        dropped(current);
        dropped(stale);
        dropped(files);
        let _ = std::fs::remove_file(&path);
    }

    /// A form written the way ADR 0105 § 2 describes one: a text field, a file,
    /// and **a second text field after the file**.
    ///
    /// The last part is what the fixture exists for. A form is free to write an
    /// input after a file input, and a `post()` answering the fields a walk had
    /// reached would report it absent — so a body whose fields all preceded the
    /// upload would let that member pass.
    const MIXED: &[&[u8]] = &[
        b"--X\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nQ3 report\r\n--X\r\n\
          Content-Disposition: form-data; name=\"doc\"; filename=\"report.pdf\"\r\n\
          Content-Type: application/pdf\r\n\r\n%PDF-1.4 and ",
        b"the rest of it\r\n--X\r\n\
          Content-Disposition: form-data; name=\"notes[first]\"\r\n\r\nafter the file\r\n--X--\r\n",
    ];

    /// `Core\Request::post(name)` on `ctx`, with the argument's own reference
    /// released the way a compiled call site releases it.
    fn posted(ctx: &mut Ctx, name: &str) -> Result<Value, i32> {
        let asked = Value::str(nvs_runtime::NvsStr::new(name.as_bytes()));
        let answered = nvs_runtime::call(nvs_core_request_post, ctx, &[asked]);
        dropped(asked);
        answered
    }

    /// [`posted`]'s answer as bytes, or `None` where the member answered the
    /// `null` a form with no such field is owed.
    fn field(ctx: &mut Ctx, name: &str) -> Option<Vec<u8>> {
        let answer = posted(ctx, name).expect("this form is readable");
        let text = answer.as_text().map(|text| text.as_bytes().to_vec());
        if answer.tag_byte() != Value::null().tag_byte() {
            dropped(answer);
        }
        text
    }

    /// ADR 0105 § 2's split read from the other side: a part carrying no
    /// `filename` is a form field, is buffered as the walk passes it, and is
    /// what `post()` answers — **including the one written after the file**,
    /// which is the position the member is built around.
    ///
    /// Two halves, because the claim differs on each side. With `files()`
    /// holding the reading, `post` joins it; with nothing holding it, `post`
    /// takes it and draining to the last field is what consumed the uploads —
    /// so `files()` afterwards is refused rather than answered empty, which is
    /// all a drained walk could say.
    #[test]
    fn a_non_file_part_is_buffered_into_post() {
        let mut walked = uploading("multipart/form-data; boundary=X", Some(Chunks::of(MIXED)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut walked, &[])
            .expect("a request that declared a multipart body can be walked");
        // `parts_of` consumes the reference it is handed, as `iterate` does.
        let parts = parts_of(&mut walked, files).expect("the walk reaches the closing delimiter");
        assert_eq!(
            parts.len(),
            1,
            "one of the three parts carried a `filename`, so one of them is a file"
        );

        assert_eq!(
            field(&mut walked, "title").as_deref(),
            Some(&b"Q3 report"[..]),
            "the field written before the upload is buffered on the way past it"
        );
        let nested = posted(&mut walked, "notes").expect("this form is readable");
        assert!(
            nested.array_ptr().is_some(),
            "§ 9's bracket convention is `query`'s over a form field too: `notes[first]` is \
             reached under `notes`, not under its whole written name"
        );
        dropped(nested);
        assert_eq!(
            field(&mut walked, "notes[first]"),
            None,
            "and the written name is a path rather than a key, so nothing answers to it whole"
        );
        assert_eq!(
            field(&mut walked, "doc"),
            None,
            "a file part is not a form field, whichever of the two members is asked"
        );

        // The other half: nothing has read this body, so `post` claims it and
        // reads to the closing delimiter itself — the uploads drained on the
        // way, which is what makes the walk afterwards a refusal.
        let mut alone = uploading("multipart/form-data; boundary=X", Some(Chunks::of(MIXED)));
        let after = posted(&mut alone, "notes").expect("a form is readable without a walk");
        assert!(
            after.array_ptr().is_some(),
            "the field after the upload is answered whether or not a walk went first"
        );
        dropped(after);
        assert!(
            nvs_runtime::call(nvs_core_request_files, &mut alone, &[]).is_err(),
            "reading to the last field consumed the parts, so the walk is refused rather than \
             answered empty"
        );
    }

    /// A urlencoded body is the other half of what `post()` reads, and it is
    /// read **once**: the body is a stream that can be pulled a single time,
    /// while the member answers one named field per call.
    ///
    /// Asserted by asking twice with a different field in between, because a
    /// member that re-read the supplier prints correctly on its first call and
    /// answers `null` on its third for a field the peer plainly sent.
    #[test]
    fn a_urlencoded_body_is_read_once_and_answers_every_call() {
        let mut submitting = uploading(
            "application/x-www-form-urlencoded",
            Some(Chunks::of(&[
                b"title=Q3+report&note",
                b"s=after%20the%20file",
            ])),
        );
        assert_eq!(
            field(&mut submitting, "title").as_deref(),
            Some(&b"Q3 report"[..]),
            "`+` is a space in a form value, and a chunk boundary is the wire's rather than a \
             field's"
        );
        assert_eq!(
            field(&mut submitting, "notes").as_deref(),
            Some(&b"after the file"[..]),
            "a field split across two pulls is one field"
        );
        assert_eq!(
            field(&mut submitting, "title").as_deref(),
            Some(&b"Q3 report"[..]),
            "and the third call reads the same body the first one held"
        );
        assert_eq!(
            field(&mut submitting, "absent"),
            None,
            "a field the form did not carry is `null`, which is the one empty answer this member \
             owes"
        );
    }

    /// ADR 0105 § 2's charge, named on both sides: a submitted form is bytes
    /// parsed into memory, so it is bounded by `[limits] request_body` — the
    /// same directive, and the same buffer, `body()` is bounded by.
    ///
    /// `crate::multipart`'s `a_field_part_is_buffered_against_the_in_memory_bound`
    /// asserts the multipart half at the parse. This is the half that has no
    /// parse in front of it, and the one where the member could have grown a
    /// cap of its own.
    #[test]
    fn a_urlencoded_form_is_bounded_by_the_directive_a_body_is() {
        let written = "title=".len();
        let last = format!("title={}", "x".repeat(REQUEST_BODY - written));
        let mut inside = uploading(
            "application/x-www-form-urlencoded",
            Some(Chunks::of(&[last.as_bytes()])),
        );
        assert_eq!(
            field(&mut inside, "title").map(|value| value.len()),
            Some(REQUEST_BODY - written),
            "a form of exactly the cap is read, so the bound is not one byte early"
        );

        let over = format!("{last}x");
        let mut outside = uploading(
            "application/x-www-form-urlencoded",
            Some(Chunks::of(&[over.as_bytes()])),
        );
        assert!(
            posted(&mut outside, "title").is_err(),
            "and one byte past it is refused before the copy, so the bound is not one byte late"
        );
    }

    /// `post` refuses after `body` and after `bodyStream`, and joins after
    /// `files` — spec § 15's exclusivity with the one exception ADR 0105 § 2
    /// creates, asserted on both sides because either half alone reads as
    /// correct.
    ///
    /// A member that simply never claimed would pass the joining half; one that
    /// claimed like the other three would pass the refusing half.
    #[test]
    fn a_form_is_refused_to_a_body_that_was_handed_over_whole() {
        let mut handed = uploading(
            "application/x-www-form-urlencoded",
            Some(Chunks::of(&[b"title=Q3+report"])),
        );
        let whole = nvs_runtime::call(nvs_core_request_body, &mut handed, &[])
            .expect("a request that carried a body can be read whole");
        dropped(whole);
        assert!(
            posted(&mut handed, "title").is_err(),
            "`body` hands the bytes over uninterpreted and leaves no fields behind"
        );

        let mut streamed = uploading(
            "application/x-www-form-urlencoded",
            Some(Chunks::of(&[b"title=Q3+report"])),
        );
        let chunks = nvs_runtime::call(nvs_core_request_body_stream, &mut streamed, &[])
            .expect("naming the walk cannot fail on a request that carried a body");
        dropped(chunks);
        assert!(
            posted(&mut streamed, "title").is_err(),
            "and naming that walk is the reading, whether or not a chunk was pulled"
        );

        let mut walking = uploading("multipart/form-data; boundary=X", Some(Chunks::of(MIXED)));
        let files = nvs_runtime::call(nvs_core_request_files, &mut walking, &[])
            .expect("a request that declared a multipart body can be walked");
        dropped(files);
        assert_eq!(
            field(&mut walking, "title").as_deref(),
            Some(&b"Q3 report"[..]),
            "`files` is the one reading `post` joins, because that walk sets the fields aside"
        );

        let mut binary = uploading(
            "application/x-www-form-urlencoded",
            Some(Chunks::of(&[b"title=\xff\xfe"])),
        );
        assert!(
            posted(&mut binary, "title").is_err(),
            "a body that is not text holds no urlencoded form, and `rule:errors/ambiguous-input-refused` refuses it rather \
             than replacing the octets it cannot read"
        );
    }
}
