//! `Core\Http\Method` — the closed set of verbs a route is declared under —
//! `Core\Router`'s link half, which is as much of
//! `rule:routing/routes-are-compiled-not-registered`'s router as
//! exists today, and `Core\Router\Match`, the match
//! `rule:routing/matched-once-before-the-handler`
//! has the door take once and `Core\Request::route()` hand back.
//!
//! # Why the enum is here rather than in a module of its own
//!
//! [`crate::registry::ENUMS`] states the rule: one line per enum, declared
//! beside the member that takes it. Three take this one — § 1's
//! `#[Route(method: …)]` payload, § 4's `Core\Router::match`/`methodsFor`, and
//! spec § 15's `Core\Request::method`, which is what answers with one. The
//! first of those is the only one on disk, and it reads the name through
//! `nvs_types::routes::OPTIONS` rather than through this constant, because
//! `nvs-types` interns a roster row by name.
//!
//! Spec § 17's `Core\Http\Client` shares the `Core\Http` prefix and is a
//! different domain (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`
//! owns it). When it lands it names *this* enum: a namespace prefix is not a
//! module boundary, and a second `Core\Http\Method` would be two rosters
//! disagreeing about what `Post` is worth.
//!
//! # What the cases are, and what the order buys
//!
//! `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`
//! names eight verbs and `rule:security/csrf-is-on-by-default`
//! names four of them as the ones CSRF enforcement covers. Those four are
//! this enum's contiguous *tail*, the same arrangement — and for the same
//! reason — as `crate::hash`'s private `STRONG`: a rule over a set of cases becomes a
//! bound rather than a match arm nobody remembers to extend.
//!
//! `CONNECT` is deliberately not a case. It establishes a proxy tunnel, so it
//! is neither something a `#[Route]` may be declared under nor something
//! `rule:http-server/allow-url-pins-the-address`'s pinned
//! client sends; a case a program can write and pass nowhere is surface with no
//! meaning behind it, which is [`crate::registry::ENUMS`]' own test for
//! admitting an entry.
//!
//! **The parse of a verb into one of these cases is [`crate::request`]'s**, not
//! this module's: `Core\Request::method` is the one reader that turns a method
//! token into a case, and its module doc owns the two decisions in it — `HEAD`
//! answering `Get`, and a token outside this roster being refused rather than
//! mapped. A served request carrying an unrecognized verb never reaches that
//! reader: it is answered **501 at the door**, before an isolate exists
//! (`nvs_server::serve`'s `Answer::not_implemented`, `501` and not `405`
//! because the route table has not been asked yet), so the throw inside the
//! program is only what a program reading `Core\Request::method` for itself
//! gets.
//!
//! # Known gaps
//!
//! 1. **A name this module is handed is one the compiler could not fold.** The
//!    route table is built and § 4's link is resolved against it while
//!    compiling: a literal name reaches [`link`]'s symbols carrying a
//!    prepared path, an unknown literal one is `E0754` before the program runs,
//!    and [`CLASS`]'s own link members are what is left over — a *computed*
//!    name, which § 4 says throws, and `nvs_types::links`' gap 1's named
//!    argument, which is folded as a computed name would be and throws for a
//!    reason a reader has to look up. Closing that gap is what would make
//!    [`no_such_route`] answer only the case § 4 named.
//!    — owner: unowned
//! 2. **The mount prefix is the half of the laundering that has nowhere to come
//!    from.** [`substitute`] percent-encodes every value it puts in a segment,
//!    which is § 4's launder and is real; what is not is
//!    (`rule:http-server/a-mount-table-expands-at-boot`
//!    )'s prefix in front of it, because a program run off the command line
//!    is mounted nowhere. `urlAbsolute` is in the same position for the same
//!    reason and says so where a program can see it: it reads
//!    [`Ctx::origin`](nvs_runtime::Ctx::origin) and throws when a unit has
//!    resolved none, rather than answering an empty authority.
//!    — owner: signed-urls

use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::keyring::KEY;
use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
    ShapeKeyDoc,
};
use crate::signature::{Confirmed, Domain};
use crate::uri::{Form, SIG_NAME, encode};

/// `Core\Http\Method`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const METHOD_NAME: &str = r"Core\Http\Method";

/// `rule:routing/route-attribute`'s `Core\Http\Method` — the eight verbs
/// `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`
/// names, safe ones first so that `rule:security/csrf-is-on-by-default`'s CSRF set is the contiguous
/// tail from `Post` on.
///
/// The integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`]. Unlike [`crate::hash::DIGEST`]'s
/// they are not read back out of an argument slot by anything yet, but
/// [`crate::request`]'s `method` now *writes* one — and the tail above is a
/// bound a later CSRF check is meant to take, so reordering this list is a
/// behaviour change rather than a cosmetic one. The test module below holds
/// that tail in place, and that module's own test holds its parse against these
/// names rather than against a second copy of the numbers.
pub(crate) const METHOD: CoreEnum = CoreEnum {
    name: METHOD_NAME,
    cases: &[
        ("Get", 0),
        ("Head", 1),
        ("Options", 2),
        ("Trace", 3),
        ("Post", 4),
        ("Put", 5),
        ("Patch", 6),
        ("Delete", 7),
    ],
    doc: Some(&METHOD_DOC),
};

/// [`METHOD`]'s reference card — `rule:core-api/reference-card`.
const METHOD_DOC: EnumDoc = EnumDoc {
    short: "The closed set of HTTP verbs a `#[Route]` may be declared under and a request may \
            carry — eight of them, safe ones first so that the four the CSRF check covers are \
            the contiguous tail from `Post` on; `CONNECT` is deliberately absent.",
    cases: &[
        CaseDoc {
            name: "Get",
            desc: "Reads a resource; safe, so no CSRF check applies.",
        },
        CaseDoc {
            name: "Head",
            desc: "`Get` without a response body; safe.",
        },
        CaseDoc {
            name: "Options",
            desc: "Asks what a resource supports; safe.",
        },
        CaseDoc {
            name: "Trace",
            desc: "Echoes the request back; safe.",
        },
        CaseDoc {
            name: "Post",
            desc: "Submits data and may change state; the first of the CSRF-covered tail.",
        },
        CaseDoc {
            name: "Put",
            desc: "Replaces a resource; CSRF-covered.",
        },
        CaseDoc {
            name: "Patch",
            desc: "Changes a resource in place; CSRF-covered.",
        },
        CaseDoc {
            name: "Delete",
            desc: "Removes a resource; CSRF-covered.",
        },
    ],
};

/// `Core\Audience`'s fully-qualified name, written once for
/// [`AUDIENCE`]'s row and for every message quoting it.
pub(crate) const AUDIENCE_NAME: &str = r"Core\Audience";

/// `rule:attributes/access-payload`'s `Core\Audience` — the one access decision `Core` names.
///
/// `Public` exists so that *this route is open* is a name that resolves rather
/// than a magic string or an absent attribute, which is the whole of § 3. It
/// **will not grow a second case**: a roster of access levels is exactly the
/// interpretation § 2 refuses to hold, so every other decision is the
/// application's own enum case or class constant and the compiler never asks
/// what it means.
pub(crate) const AUDIENCE: CoreEnum = CoreEnum {
    name: AUDIENCE_NAME,
    cases: &[("Public", 0)],
    doc: Some(&AUDIENCE_DOC),
};

/// [`AUDIENCE`]'s reference card — `rule:core-api/reference-card`.
const AUDIENCE_DOC: EnumDoc = EnumDoc {
    short: "The one access decision `Core` names for `#[Access(allow: …)]`: a route open to \
            everyone is a name that resolves rather than a magic string, and the enum will not \
            grow a second case, because a roster of access levels is an interpretation `Core` \
            does not hold.",
    cases: &[CaseDoc {
        name: "Public",
        desc: "The route is open to every caller; every other decision is the application's own \
               enum case or class constant.",
    }],
};

/// `Core\Router`'s fully-qualified name, in one place so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Router";

/// `$params` — `rule:routing/link-name-and-params-are-checked` writes it `array<string, mixed>`, and that is a
/// spelling the type system has no form for: `nvs_types::ty::Ty::Array` carries
/// one element type, because a Novis array's keys are `int|string` by
/// construction and are not part of its type. So the row declares the half that
/// *is* representable, `array<mixed>`, and the key rule is enforced where it
/// can be — § 4 makes a literal key that names neither a capture nor a declared
/// `#[Query]` parameter a compile error, which is a question about this route's
/// own captures rather than about a type.
const PARAMS: CoreTy = CoreTy::Array(&CoreTy::Mixed);

/// `array<Core\Http\Method>` — `rule:routing/a-refused-verb-is-not-a-missing-path`
/// 's answer, as the enum this module already owns rather than as text.
///
/// A list of cases, so that the `Allow:` header a caller writes out of it is
/// spelled by [`METHOD`]'s roster in one place — a member answering
/// `array<string>` would be a second spelling of the eight verbs, free to
/// disagree with the first about what `Delete` looks like.
const METHODS: CoreTy = CoreTy::Array(&CoreTy::Enum(METHOD_NAME));

/// `?Core\Router\Match`, written once — `match` answers it and so, one class
/// away, does `Core\Request::route()`.
const MATCHED: CoreTy = CoreTy::Nullable(&CoreTy::Instance(MATCH_NAME));

/// Spec § 15's `Core\Router`, as much of it as `rule:routing/matching-is-not-dispatching`'s link half and
/// `rule:routing/matched-once-before-the-handler` and `rule:routing/a-refused-verb-is-not-a-missing-path`'s two answers need.
///
/// `match` asks *of the table*, about a verb and a path the caller chose, and
/// so does `methodsFor` — neither reads the request, which is why both are here
/// rather than on `Core\Request`. § 1's own match is the other member entirely:
/// the door takes it once, before any of this program ran, and
/// `Core\Request::route()` hands that one back. A program asking this class
/// about the path it is already serving would be matching twice, which is what
/// § 1 exists to remove.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "url",
            names: &["name", "params"],
            params: &[CoreTy::Str, PARAMS],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_router_url",
            doc: Some(&URL_DOC),
        },
        CoreMethod {
            name: "urlAbsolute",
            names: &["name", "params"],
            params: &[CoreTy::Str, PARAMS],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_router_url_absolute",
            doc: Some(&URL_ABSOLUTE_DOC),
        },
        CoreMethod {
            name: "urlSigned",
            // `settings`, not `options`, and the same shape
            // [`crate::signature`] writes once for all three doors — the
            // spelling `$uri->sign` already takes, so a caller who has signed a
            // URL writes the same literal here
            // (`rule:core-api/signing-is-over-a-payload`). There is no fourth
            // parameter naming which of `$params` is covered: all of it is, and
            // an options bag saying otherwise is where every framework's bypass
            // has lived.
            names: &["name", "params", "settings"],
            params: &[
                // `rule:security/sink-predicate` applied to a route name: it
                // is not data the answer carries — the link is built out of
                // the declared path and `$params`, and the name appears in
                // neither — it is which handler runs, the predicate's own
                // "the executable path is an instruction". Refusing a
                // `tainted` one where the call is written is also the only
                // useful answer: a name that is not a literal never folds, so
                // the alternative is `no_such_route` at run time for a call
                // that could never have worked.
                CoreTy::Text(Qual::Sink),
                PARAMS,
                CoreTy::Shape(crate::signature::SIGNING),
            ],
            defaults: &[],
            // `url`'s own return type, which is the launder: a `string` rather
            // than a `tainted string` is what says the URL-path sink has been
            // written for (`rule:security/launderers-are-sink-named`).
            return_ty: CoreTy::Str,
            symbol: "nvs_core_router_url_signed",
            doc: Some(&URL_SIGNED_DOC),
        },
        CoreMethod {
            name: "signedRoute",
            names: &["keys"],
            // The ring alone. There is no path, no name and no `$params`
            // here, and that is the shape of the claim: what this member
            // verifies is the request the door already matched, so a
            // parameter naming any part of it would be a second reading of
            // the URL and something the two halves could disagree about
            // (`rule:routing/matched-once-before-the-handler`).
            params: &[CoreTy::Array(&KEY)],
            defaults: &[],
            // Not nullable, and not a `bool`: every way of not verifying
            // throws (`rule:core-api/one-refusal-except-expiry`), so there is
            // no falsy answer a caller could drop on the floor, and what comes
            // back is the match itself rather than a second reading of it.
            return_ty: CoreTy::Instance(MATCH_NAME),
            symbol: "nvs_core_router_signed_route",
            doc: Some(&SIGNED_ROUTE_DOC),
        },
        CoreMethod {
            name: "match",
            names: &["method", "path"],
            params: &[CoreTy::Enum(METHOD_NAME), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: MATCHED,
            symbol: "nvs_core_router_match",
            doc: Some(&MATCH_DOC),
        },
        CoreMethod {
            name: "methodsFor",
            names: &["path"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: METHODS,
            symbol: "nvs_core_router_methods_for",
            doc: Some(&METHODS_FOR_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `$name` and `$params`, documented once — both members take the same two.
const NAME_DOC: ParamDoc = ParamDoc {
    name: "name",
    desc: "The route's name as its `#[Route]` declared it; a literal is resolved against the \
           route table while compiling, and an unknown literal is a compile error.",
    shape: &[],
};

/// See [`NAME_DOC`].
const PARAMS_DOC: ParamDoc = ParamDoc {
    name: "params",
    desc: "The path's captures by name, plus any query parameters; a literal key that is neither \
           a capture nor a declared `#[Query]` parameter is a compile error.",
    shape: &[],
};

/// `Core\Router::url`'s reference card — `rule:core-api/reference-card`.
const URL_DOC: MethodDoc = MethodDoc {
    short: "Builds the URL path of the route named `$name`, substituting `$params` into its \
            `{captures}` and writing what is left over as a query string — the launderer for \
            the URL-path sink, every value percent-encoded into its own segment.",
    params: &[NAME_DOC, PARAMS_DOC],
    ret: "The path, `/users/42?page=2`, with an optional `{name?}` capture dropped when `$params` \
          omits it and a `{name...}` capture's own `/`s kept as structure.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "When `$name` is not a literal the compiler could resolve — a computed name, or one \
               given as a named argument; when `$params` lacks a capture the path requires; or \
               when a value has no text form a segment or query parameter could be built from.",
    }],
};

/// `Core\Router::urlAbsolute`'s reference card — `rule:core-api/reference-card`.
const URL_ABSOLUTE_DOC: MethodDoc = MethodDoc {
    short: "`url` with the mount's configured origin in front — the `[[app]] origin` setting, \
            resolved before the request ran and never derived from a `Host` or \
            `X-Forwarded-Host` header.",
    params: &[NAME_DOC, PARAMS_DOC],
    ret: "The absolute URL, `https://example.test/users/42?page=2`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "For everything `url` throws for, and when no origin is configured for the unit, \
               since an origin is never derived from a request header.",
    }],
};

/// `Core\Router::urlSigned`'s reference card — `rule:core-api/reference-card`.
const URL_SIGNED_DOC: MethodDoc = MethodDoc {
    short: "`url` with the reserved `_sig` query parameter on the end, over a signature taken \
            across the route's **name** and `$params` — never the path they render to, so the \
            same link still verifies after the module is remounted somewhere else.",
    params: &[
        NAME_DOC,
        PARAMS_DOC,
        ParamDoc {
            name: "settings",
            desc: "The key ring and the lifetime, written as one literal because neither has a \
                   sensible value this member could choose.",
            shape: &[
                ShapeKeyDoc {
                    key: "keys",
                    ty: "array<secret bytes>",
                    desc: "The key ring, **newest first**: `$keys[0]` signs, and the rest exist \
                           so that a link minted before the last rotation still verifies. The \
                           same ring `Core\\Signature` and `$uri->sign` take, and a token minted \
                           at one of those doors does not verify at this one.",
                },
                ShapeKeyDoc {
                    key: "until",
                    ty: "?Core\\Time\\Instant",
                    desc: "When the link stops working, inside the signed bytes where a holder \
                           cannot edit it. `null` is the forever spelling, and it has to be \
                           written — a permanent signed URL is a permanent bearer credential, \
                           and it ends up in browser history, `Referer` headers and chat \
                           unfurls.",
                },
            ],
        },
    ],
    ret: "`url`'s path with `_sig=…` appended, `/users/42?page=2&_sig=…` — laundered for the \
          URL-path sink exactly as `url` is, and carrying the mount prefix the same way. The same \
          name, parameters, ring and lifetime always mint the same token; the token carries the \
          signed form as well as the tag, so it adds about `4/3 × (name + params + 40)` \
          characters.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "For everything `url` throws for, and when `$params` carries the reserved \
                   `_sig` key, which this member is about to write and will not write twice.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$settings.keys` is empty, so there is no newest key; or its first entry is \
                   not 32 octets long — a `bytes` that was never a key.",
        },
    ],
};

/// `Core\Router::signedRoute`'s reference card — `rule:core-api/reference-card`.
const SIGNED_ROUTE_DOC: MethodDoc = MethodDoc {
    short: "Confirms that the request this program is answering carries a signature `$keys` made \
            for the route it matched, and answers that match — `urlSigned`'s read half, over the \
            route's name and its parameters rather than over the path they rendered to.",
    params: &[ParamDoc {
        name: "keys",
        desc: "The key ring, **newest first**, and the same one `urlSigned` was given: a token \
               authenticating under any entry is authentic, which is what lets a key be retired \
               without breaking every link already sent.",
        shape: &[],
    }],
    ret: "The request's own `Core\\Router\\Match`, the value `Core\\Request::route()` answers, \
          once the signature over its name and parameters has been confirmed. A link that does \
          not verify is a throw and never a value: nothing here renders a refusal, because the \
          program that renders one is the program that should decide when to ask.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "When this request carries no signature this ring made for the route it \
                   matched — no `_sig` parameter or two of them, an altered token, one minted at \
                   another door or under a retired key, a parameter added, removed or edited, and \
                   a request that matched no named route: one sentence for all of it. When the \
                   signature has expired, which is the one failure with a sentence of its own and \
                   is reached only after the token has been found authentic. And when a query \
                   parameter's escapes decode to octets that are not UTF-8, which says something \
                   about the URL that arrived and nothing about the token.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "When this program is not answering a request at all — a CLI program, a \
                   scheduled script, a job worker or a test — which is a different fact from a \
                   request that carries no signature. Or when `$keys` is empty or its first entry \
                   is not 32 octets long, as every door over a ring refuses.",
        },
    ],
};

/// `Core\Router::match`'s reference card — `rule:core-api/reference-card`.
const MATCH_DOC: MethodDoc = MethodDoc {
    short: "Matches `$method` and `$path` against this program's compiled route table, answering \
            the same `Core\\Router\\Match` a served request carries — a question asked of the \
            table, which dispatches nothing and never reads the request.",
    params: &[
        ParamDoc {
            name: "method",
            desc: "The verb to match under. A route declared for one verb is not claimed by \
                   another, so the same path under `Get` and `Post` are two questions.",
            shape: &[],
        },
        ParamDoc {
            name: "path",
            desc: "The path to match, as a URL path and with no query string; a mount's prefix is \
                   not stripped here, because nothing about a path the caller chose says which \
                   mount it was meant for.",
            shape: &[],
        },
    ],
    ret: "The match — its declared name, and the captures the path filled, each percent-decoded \
          once and converted to the type its `#[Route]` parameter declared. `null` where no route \
          claims that verb and path, and for a program that declares no route at all, since a \
          table nothing built claims nothing.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A capture percent-decodes to octets that are not UTF-8, so it has no `tainted \
               string` to bind to; the throw names the capture and the offset of the first byte a \
               `string` cannot hold.",
    }],
};

/// `Core\Router::methodsFor`'s reference card — `rule:core-api/reference-card`.
const METHODS_FOR_DOC: MethodDoc = MethodDoc {
    short: "Every verb the route table claims `$path` under, in the order the routes were \
            declared — the question left over once `Core\\Request::route()` has answered `null`, \
            and the one a `404` and a `405` are told apart by.",
    params: &[ParamDoc {
        name: "path",
        desc: "The path to ask about, as a URL path and with no query string; a mount's prefix is \
               already stripped from the one the request arrived with.",
        shape: &[],
    }],
    ret: "The verbs, once each: an empty array where no route claims the path at all — the `404` \
          — and otherwise the list an `Allow:` header spells for the `405`. Both forms of a \
          terminal `{name?}` answer the same verbs, and a path whose capture will not convert is \
          claimed by nobody — which is the conversions the matcher itself performs, since a capture \
          typed at any other class built from text matches on shape and refuses later.",
    errors: &[],
};

/// `Core\Router\Match`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const MATCH_NAME: &str = r"Core\Router\Match";

/// The global interface every class built from text implements, as
/// `nvs_hir::interfaces` declares it.
///
/// Written out rather than reached for, which is `registry.rs`'s
/// `EXCEPTION_TREE` reasoning unchanged: this crate does not depend on
/// `nvs-hir`, and the two rosters a `CoreTy::Instance` may name beyond
/// [`crate::registry::CLASSES`] are both short written lists whose only way of
/// being wrong is caught by the conformance case that calls the member.
pub(crate) const PARSES_NAME: &str = "Parses";

/// [`MATCH`]'s slots, in the order [`match_value`] fills them.
const MATCH_ROUTE_NAME: usize = 0;
const MATCH_PARAMS: usize = 1;

/// `rule:routing/a-capture-narrows-to-a-closed-set`'s capture as a program reaches it, and the one place the five
/// forms of [`nvs_runtime::routes::Param`] are spelled as a type.
///
/// A union rather than a `string`, because § 1 says the server computes "typed
/// parameters" and answering the segment text for a `{id: uint}` route would
/// make the program parse a second time what the matcher already parsed — the
/// two-readings-that-can-disagree shape this crate refuses everywhere else. A
/// union rather than [`CoreTy::Mixed`] because the text arm is `tainted`: a
/// capture is a piece of the request path the peer wrote, and `mixed` is a type
/// a program can cast the mark off. `int` and `uint` are both members and
/// neither subsumes the other — `nvs_types`' assignment relation widens each
/// only to `float` — so a `uint` capture past `i64::MAX` still has a type.
///
/// **Every conversion the matcher performs is a member here**, and a type § 5
/// admits that it does not convert yet arrives on the text arm — which is what
/// makes this union the readable statement of where
/// [`nvs_runtime::routes::CaptureConv`] currently stands. A `decimal` and a
/// `Core\Uuid` are the two that joined it once the parses reached the crate the
/// walk is in; a `bool` and an `enum` have not.
///
/// **The last member is an interface rather than a class**, and it is the one
/// the matcher does *not* convert: a capture typed as any other class built
/// from text is whatever that class's own `parse` answered at
/// [`capture_value`], and no registered class names that set. Writing
/// `Core\Uuid` beside it is not redundancy — that class implements the same
/// interface, and the point of naming it is that the *router* reads it, so a
/// segment it refuses never matched at all.
const CAPTURE: &CoreTy = &CoreTy::Union(&[
    CoreTy::TaintedStr,
    CoreTy::Int,
    CoreTy::Uint,
    CoreTy::Decimal,
    CoreTy::Instance(crate::uuid::NAME),
    CoreTy::Instance(PARSES_NAME),
]);

/// `rule:routing/matched-once-before-the-handler`
/// 's match, as the program answering the request reads it.
///
/// # It is built where the match crosses, and holds no route
///
/// [`nvs_runtime::routes::Match`] holds an `Arc<Route>` and travels from the
/// door on [`nvs_runtime::Inbound`]; this class holds the two answers a program
/// asked for and nothing else, built by [`match_value`] each time
/// `Core\Request::route()` is read. So the row stays where § 1 put it — on the
/// request, taken once — and nothing about the compiled table is reachable
/// through an object a program is holding. What it spends is one instance and
/// one array of the path's own captures per *read*, which is a handful of
/// values against a route's two or three captures and O(in-flight) either way.
///
/// # The name is the program's, the captures are the peer's
///
/// [`MATCH_NAME_DOC`]'s `?string` comes out of the unit's own
/// `#[Route(name: …)]` literal, so it is plain text and a sink takes it. Every
/// capture is a segment of the path the request arrived with, so its text arm
/// is `tainted` — [`CAPTURE`] owns that reasoning. A program that mixed the two
/// up would be laundering the request through the route table, which is the one
/// direction `AGENTS.md`'s priority 1 does not trade.
///
/// A capture's text is still **percent-encoded**, which is
/// [`nvs_runtime::routes`]' own gap 4: decoding belongs to [`crate::uri`] and a
/// second decoder below it would be two launderers that agree today.
///
/// # Why there is no `route()` reader here
///
/// § 1's "matching is not dispatching" as a shape: a member answering the
/// matched row would put the handler's `Class::method` label, its access
/// decision and its declared verb in front of a program, which is the surface
/// `rule:routing/matching-is-not-dispatching` refuses to
/// grow. The name and the captures are what the three rules § 1 names actually
/// read, and they are all that crosses.
pub(crate) const MATCH: CoreClass = CoreClass {
    name: MATCH_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "name",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_router_match_name",
            doc: Some(&MATCH_NAME_DOC),
        },
        CoreMethod {
            name: "params",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(CAPTURE),
            symbol: "nvs_core_router_match_params",
            doc: Some(&MATCH_PARAMS_DOC),
        },
        CoreMethod {
            name: "param",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(CAPTURE),
            symbol: "nvs_core_router_match_param",
            doc: Some(&MATCH_PARAM_DOC),
        },
    ],
    slots: &["name", "params"],
    constants: &[],
};

/// `Core\Router\Match::name`'s reference card — `rule:core-api/reference-card`.
const MATCH_NAME_DOC: MethodDoc = MethodDoc {
    short: "The declared name of the route this request matched, as its `#[Route(name: …)]` wrote \
            it — the same string `Core\\Router::url` resolves and the `route` metric label \
            carries.",
    params: &[],
    ret: "The name, or `null` where the matched route declares none. Not `tainted`: it is the \
          unit's own literal and not anything the request carried.",
    errors: &[],
};

/// `Core\Router\Match::params`'s reference card — `rule:core-api/reference-card`.
const MATCH_PARAMS_DOC: MethodDoc = MethodDoc {
    short: "Every capture the matched path filled, keyed by the parameter name it binds, in path \
            order.",
    params: &[],
    ret: "An array of the captures. A `{name}` declared `string` answers `tainted string` and is \
          still percent-encoded; one declared `int`, `uint`, `decimal` or `Core\\Uuid` answers the \
          value the match already converted, and a segment that would not convert never matched \
          the route at all. One declared at any other class implementing `Parses` answers what \
          that class's own `parse` made of the segment, which runs when this match is read: it \
          matched on shape, so a segment the class refuses throws here rather than sending the \
          request to another route. A route with no captures answers an empty array.",
    errors: &[],
};

/// `Core\Router\Match::param`'s reference card — `rule:core-api/reference-card`.
const MATCH_PARAM_DOC: MethodDoc = MethodDoc {
    short: "One capture by the parameter name it binds — `params()` read at one key, and the \
            spelling a handler reaching for a single segment writes.",
    params: &[ParamDoc {
        name: "name",
        desc: "The capture's name as the route's path declared it, without the braces.",
        shape: &[],
    }],
    ret: "The capture, on `params()`'s terms, or `null` where the matched route declares no \
          capture under that name — including an optional `{name?}` the request left off.",
    errors: &[],
};

/// The two symbols `rule:routing/link-name-and-params-are-checked`'s **folded** link is lowered to, and the wire
/// format they read argument 0 as.
///
/// Neither is a [`CoreMethod`] row, and that is the point: a program calls
/// `Core\Router::url`, and `nvs_ir::lower` redirects the call here whenever
/// `nvs_types::links` resolved its literal name against the compile-time table.
/// So the *member* is one, and which of the two implementations answers is a
/// property of what the compiler could prove — the same arrangement `rule:expressions/preparation-preserves-behaviour` states for every prepared literal: one implementation, reached at two
/// entry points, never two implementations.
///
/// **Argument 0 is the route's path, already split.** `nvs_types::routes`'
/// `link_pieces` reads § 2's grammar — the only reading of it anywhere — and
/// `nvs_types::UrlPiece::prepared` writes its answer out in this format:
/// pieces separated by [`link::PIECE_SEPARATOR`], each one a tag byte from the four
/// constants below followed by its text. Reading it back is a `split` and a
/// byte test, so no second parser of a path exists to disagree with the first.
pub mod link {
    /// `nvs_core_router_link` — `Core\Router::url` with the lookup already
    /// made.
    pub const SYMBOL: &str = "nvs_core_router_link";
    /// `nvs_core_router_link_absolute` — the same with `rule:routing/an-absolute-link-takes-a-configured-origin`'s
    /// configured origin in front.
    pub const ABSOLUTE_SYMBOL: &str = "nvs_core_router_link_absolute";
    /// `nvs_core_router_link_signed` — `Core\Router::urlSigned` with the same
    /// lookup made, and with the resolved route **name** as a second argument.
    ///
    /// The name travels beside the prepared path rather than inside it because
    /// the two are opposites: the path is what a remount changes and the name
    /// is what it does not, which is the whole of
    /// `rule:core-classes/router-signed-url`. The `{keys, until}` shape follows
    /// `$params`, one argument per field.
    pub const SIGNED_SYMBOL: &str = "nvs_core_router_link_signed";
    /// What separates two pieces. `\u{1}` because a path segment cannot hold
    /// one: § 2's capture names are identifiers and its literal segments come
    /// out of a `#[Route]` payload that a control byte would already have made
    /// unusable as a URL.
    pub const PIECE_SEPARATOR: char = '\u{1}';
    /// A literal segment, its leading `/` included — copied out verbatim.
    pub const LITERAL: u8 = b'L';
    /// `{name}`: `/` and the percent-encoded value at `name`.
    pub const REQUIRED: u8 = b'R';
    /// `{name?}`: [`REQUIRED`], or the whole segment dropped.
    pub const OPTIONAL: u8 = b'O';
    /// `{name...}`: [`REQUIRED`] with the value's own `/`s left alone.
    pub const REST: u8 = b'*';

    /// Every symbol here, for [`crate::symbols`], which builds the JIT's
    /// roster out of the member rows and so would never reach an
    /// implementation no [`super::CoreMethod`] names — the same reason
    /// [`crate::registry::CONSTRUCTORS`] is chained there.
    pub const SYMBOLS: [&str; 3] = [SYMBOL, ABSOLUTE_SYMBOL, SIGNED_SYMBOL];
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_router_url" => (nvs_core_router_url as *const ()).cast(),
        "nvs_core_router_url_absolute" => (nvs_core_router_url_absolute as *const ()).cast(),
        "nvs_core_router_link" => (nvs_core_router_link as *const ()).cast(),
        "nvs_core_router_link_absolute" => (nvs_core_router_link_absolute as *const ()).cast(),
        "nvs_core_router_url_signed" => (nvs_core_router_url_signed as *const ()).cast(),
        "nvs_core_router_link_signed" => (nvs_core_router_link_signed as *const ()).cast(),
        "nvs_core_router_signed_route" => (nvs_core_router_signed_route as *const ()).cast(),
        "nvs_core_router_match" => (nvs_core_router_match as *const ()).cast(),
        "nvs_core_router_methods_for" => (nvs_core_router_methods_for as *const ()).cast(),
        "nvs_core_router_match_name" => (nvs_core_router_match_name as *const ()).cast(),
        "nvs_core_router_match_params" => (nvs_core_router_match_params as *const ()).cast(),
        "nvs_core_router_match_param" => (nvs_core_router_match_param as *const ()).cast(),
        _ => return None,
    })
}

/// One `$params` value as the text a path segment carries.
///
/// [`nvs_runtime::value_to_string`]'s own rules, plus the release its contract
/// requires: a `Tag::Str` operand comes back as itself with one fresh
/// reference, so the bytes are copied out and the reference dropped here.
fn segment_text(value: Value, member: &str, key: &str) -> Result<String, Fault> {
    let text = nvs_runtime::value_to_string(value).map_err(|_| {
        Fault::thrown(format!(
            "Core\\Router::{member}(): `{key}` holds a value with no text form, so there is \
             nothing a path segment could be built out of it"
        ))
    })?;
    // A post-condition of `value_to_string` rather than a boundary, and so
    // unreachable from source with no diagnostic to name: every `Ok` arm of it
    // builds a `Value::str`, `rule:security/capture-answers-the-carrier`'s carrier arm included, so this is a
    // `Tag::Str` or it is the `Err` the `?` above already took — the same
    // judgement `crate::uri::scalar_text` records at its own copy of this pair.
    let owned = text
        .as_text()
        .ok_or_else(|| Fault::fatal("`value_to_string` answered something that is not a string"))?
        .to_owned();
    #[expect(
        unsafe_code,
        reason = "`value_to_string` hands back exactly one fresh reference, and \
                  the text has been copied out of it"
    )]
    unsafe {
        text.release();
    }
    Ok(owned)
}

/// The prepared path with every capture substituted — the whole of both link
/// helpers below, since they differ only in what stands in front of it.
///
/// Each substituted value is percent-encoded under
/// [`Form::Component`](crate::uri::Form::Component), which is § 4's launder for
/// the URL-path sink: a value holding a `/` cannot climb out of its segment.
/// A `{name...}` is the one exception, and § 2 is why — that form *is* every
/// remaining segment, so its `/`s are structure rather than content, and each
/// segment between them is encoded on its own.
///
/// **What the path did not take becomes the query string**, which is
/// `rule:routing/a-leftover-link-key-is-a-query-string`'s rule: the prepared pieces name every capture, so a
/// `$params` key left over once they have been substituted is by construction
/// not one, and § 6 makes it a query parameter. It is written by
/// [`crate::uri::build`] — `Core\Uri::buildQuery`'s own pass, run over the same
/// array with the captures omitted — so a link's query string is
/// `http_build_query`'s spelling down to its nesting and its `Form::FormValue`
/// escaping, rather than a second convention a reader would have to learn. The
/// walk is O(`$params`) whether or not anything is left over; `$params` is a
/// literal written at the call site and is small.
///
/// The **refusal** half of § 6 — a key that is neither a capture nor a declared
/// `#[Query]` parameter is a compile error — is not here and never will be: it
/// reads the handler's parameter attributes, so it is `E0759` in
/// `nvs_types::links`, over the same literal keys the fold already collected.
/// It is a check over what the *call site* wrote, though, not over what the
/// array holds: a `$params` whose keys are not literals has none for that pass
/// to read, so a key computed at run time still reaches the walk below and
/// still becomes a query parameter. § 6 makes that the answer rather than an
/// error — a link cannot know which of a program's own keys is a typo.
///
/// **A value outside `rule:routing/a-capture-narrows-to-a-closed-set`'s closed set is substituted here, and throws
/// nothing.** Its literal spelling is `E0772` in `nvs_types::links`, over the
/// same folded entries `E0759` reads, and a computed one reaches this walk with
/// nothing left to check it against: the closed set is the *handler's* declared
/// type, and what crosses into a prepared template is § 2's path pieces and no
/// type at all. Carrying the sets across so this could refuse would put a table
/// on every link a program builds, to re-answer at run time what § 5 already
/// answers at the only place the value is knowable — and the answer would be a
/// throw where § 5's own is a `404`, since an out-of-set *segment* is a request
/// that does not match rather than a program that is wrong. Nothing about the
/// laundering changes either way: the value is percent-encoded into its own
/// segment above whatever it holds, so what a bad link produces is a dead URL
/// and never an escape from one. This is § 6's rule for a computed key, applied
/// to a computed value for the same reason — the two halves of one entry answer
/// the same way.
fn substitute(template: &str, params: &Value, member: &str) -> Result<String, Fault> {
    let raw = params.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Router::{member} expected {:?} for `$params`, got tag {}",
            Tag::Array,
            params.tag_byte()
        ))
    })?;
    let params = crate::arr::borrowed(raw);
    let mut out = String::with_capacity(template.len());
    // Every key the path consumed, in the order the pieces name them — what
    // the query string below is the complement of.
    let mut captures: Vec<&str> = Vec::new();
    for piece in template.split(link::PIECE_SEPARATOR) {
        // Unreachable from source, and not through a diagnostic refusing an
        // argument: `template` is never a program's value. It is
        // `nvs_types::UrlPiece::prepared`'s output, carried as the `ConstStr`
        // argument `nvs_ir::lower` writes, and that writer pushes a tag byte
        // before every piece's text — so a piece is at least one byte, and the
        // empty template a path with no segments would prepare is `E0750` at
        // the declaration, which refuses a `path:` not beginning with `/`.
        let (tag, key) = piece.as_bytes().split_first().ok_or_else(|| {
            Fault::fatal("a prepared route link's piece is a tag byte and its text")
        })?;
        // The same writer, and the same reason it is unreachable from source:
        // `prepared` builds a Rust `String` out of § 2's capture names and
        // literal segments, so the bytes it hands over are UTF-8 by
        // construction rather than by a check anything here could fail.
        let key = std::str::from_utf8(key)
            .map_err(|_| Fault::fatal("a prepared route link is built out of `str`"))?;
        if *tag == link::LITERAL {
            out.push_str(key);
            continue;
        }
        captures.push(key);
        let Some(value) = params.get(key.as_bytes()) else {
            if *tag == link::OPTIONAL {
                continue;
            }
            return Err(Fault::thrown(format!(
                "Core\\Router::{member}(): `$params` holds no `{key}`, which this route's path \
                 captures"
            )));
        };
        let text = segment_text(value, member, key)?;
        out.push('/');
        if *tag == link::REST {
            let encoded: Vec<String> = text
                .split('/')
                .map(|segment| encode(segment.as_bytes(), Form::Component))
                .collect();
            out.push_str(&encoded.join("/"));
        } else {
            out.push_str(&encode(text.as_bytes(), Form::Component));
        }
    }
    let query = crate::uri::build(raw, "Core\\Router", member, &captures)?;
    if !query.is_empty() {
        out.push('?');
        out.push_str(&query);
    }
    Ok(out)
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::url` over a name the compiler resolved — see [`link`].
    ///
    /// The mount prefix `rule:http-server/a-mount-table-expands-at-boot` has this member prepend is empty here and
    /// only here: a program run off the command line is mounted nowhere, and
    /// there is no server yet to be mounted by. The prefix joins in front of
    /// [`substitute`]'s answer when one exists, which is why the substitution
    /// is its own function rather than this body.
    fn nvs_core_router_link(_ctx, args: [2]) {
        let template = link_template(args, "url")?;
        produced(&substitute(template, &args[1], "url")?)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::urlAbsolute` over a name the compiler resolved —
    /// [`nvs_core_router_link`] with `rule:routing/an-absolute-link-takes-a-configured-origin`'s configured origin in front.
    ///
    /// The origin is per mount, falling back to `[app] origin`, and is never
    /// derived from `Host` or `X-Forwarded-Host` — so this member reads what
    /// was resolved *before* the request ran, out of
    /// [`Ctx::origin`](nvs_runtime::Ctx::origin), and never a value the
    /// request could have influenced. A unit that resolves none throws here
    /// rather than answering with an empty authority in it, which is `rule:http-server/a-mount-table-expands-at-boot`
    /// 's rule at the one place a CLI run can enforce it: § 6 puts the
    /// *boot* error at mount expansion, and there is no mount off the command
    /// line to expand.
    fn nvs_core_router_link_absolute(ctx, args: [2]) {
        let template = link_template(args, "urlAbsolute")?;
        let path = substitute(template, &args[1], "urlAbsolute")?;
        let Some(origin) = ctx.origin() else {
            return Err(Fault::thrown(format!(
                "Core\\Router::urlAbsolute(): no origin is configured for this unit, so `{path}` \
                 has no absolute form. `rule:routing/an-absolute-link-takes-a-configured-origin` refuses to derive one from a request header, \
                 so give `nvs.toml` an `[[app]] origin`"
            )));
        };
        produced(&format!("{origin}{path}"))
    }
}

/// `Core\Router::urlSigned`, spelled the way [`crate::keyring`]'s refusals
/// name it.
const URL_SIGNED: &str = r"Core\Router::urlSigned";

/// What [`nvs_core_router_link_signed`] takes a signature over: the route's
/// declared name, and every entry of `$params` as the text it contributes to
/// the URL.
///
/// **The name and never the path.** One compiled table serves at `/ModuleA`,
/// at `/ModuleB` or at `/` (`rule:http-server/a-mount-table-expands-at-boot`),
/// so a signature over the assembled path stops verifying the moment a mount
/// moves, and one over the route's identity does not — which is the whole
/// reason this pair exists rather than a caller parsing `url`'s answer and
/// signing that (`rule:core-classes/router-signed-url`).
///
/// **The parameters are their text**, one step short of the encoding
/// [`substitute`] then applies, and that is what makes the payload
/// *derivable*: the verifying half holds a match whose captures were decoded
/// and converted on the way in, so the two sides can agree on `42` where they
/// could not agree on whether it arrived as an `int` or as a `string`. A
/// nested array stays an array, so the bracket convention a query parameter
/// carries survives into the signed bytes rather than being flattened into
/// text whose parse would be a second grammar.
fn signed_payload(name: &str, params: &Value, member: &str) -> Result<Value, Fault> {
    let mut payload = NvsArray::new();
    payload.set(
        NvsStr::new(b"route"),
        Value::str(NvsStr::new(name.as_bytes())),
    );
    payload.set(
        NvsStr::new(b"params"),
        Value::array(written_form(params, member)?),
    );
    Ok(Value::array(payload))
}

/// One level of [`signed_payload`]'s `$params`: every live entry under its own
/// key, scalars as [`segment_text`]'s text and arrays as arrays.
///
/// # Errors
///
/// [`segment_text`]'s throw for a value with no text form — the same refusal
/// [`substitute`] makes over the same value, so a `$params` this refuses is
/// one `url` would have refused too.
fn written_form(params: &Value, member: &str) -> Result<NvsArray, Fault> {
    // Unreachable from source: the row declares `array<mixed>`, so `E0401`
    // refuses anything else before this body runs.
    let raw = params.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Router::{member} expected {:?} for `$params`, got tag {}",
            Tag::Array,
            params.tag_byte()
        ))
    })?;
    let array = crate::arr::borrowed(raw);
    let mut out = NvsArray::new();
    let mut slot = 0;
    while let Some(live) = array.next_slot(slot) {
        slot = live + 1;
        let key = array.key_at(live).expect("a live slot has a key");
        let value = array.value_at(live).expect("a live slot has a value");
        let written = if value.array_ptr().is_some() {
            Value::array(written_form(&value, member)?)
        } else {
            let label = std::str::from_utf8(key.as_bytes()).unwrap_or("<not text>");
            Value::str(NvsStr::new(segment_text(value, member, label)?.as_bytes()))
        };
        out.set(key, written);
    }
    Ok(out)
}

/// Whether `$params` writes the reserved parameter itself.
///
/// The top level alone, because that is where the reservation lives: a key's
/// base name is what [`crate::uri::build`] writes the query pair under, so
/// `_sig` holding an array is `_sig[0]=…` and is the same collision. A
/// literal `$params` never reaches this — `nvs_types::links` refuses a key
/// that names neither a capture nor a declared `#[Query]` parameter while
/// compiling — so this answers for the computed one.
fn carries_reserved(params: &Value) -> bool {
    params
        .array_ptr()
        .is_some_and(|raw| crate::arr::borrowed(raw).has_key(SIG_NAME.as_bytes()))
}

/// Releases the one reference [`signed_payload`] or [`derived_payload`] built,
/// which is handed to nobody: [`crate::signature::mint`] and
/// [`crate::signature::confirm`] both borrow the payload they are given.
fn discard(payload: Value) {
    #[expect(
        unsafe_code,
        reason = "the two payload builders each hand over exactly the reference passed here"
    )]
    unsafe {
        payload.release();
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::urlSigned` over a name the compiler resolved — see
    /// [`link`], and [`nvs_core_router_link`] for the half of the answer that
    /// is an ordinary link.
    ///
    /// **The signature is taken before the path is built**, over
    /// [`signed_payload`]'s document rather than over [`substitute`]'s answer,
    /// which is `rule:core-api/signing-is-over-a-payload` at the door where
    /// ignoring it is most tempting: the assembled text is right there, and
    /// signing it is what makes a link stop verifying when the module moves.
    ///
    /// The token goes on the end as [`SIG_NAME`], the same parameter
    /// `$uri->sign` writes, and needs no escaping —
    /// [`crate::signature::mint`] answers unpadded URL-safe base64. A
    /// `$params` that writes that parameter itself is refused rather than
    /// overwritten: the two would be one query with two answers in it, and
    /// picking one is picking which an attacker gets to try.
    ///
    /// **Cost:** one document the size of the name and the parameters, one
    /// HMAC, and the substitution `url` already pays. All of it inside the
    /// call (`rule:programs/memory-priority`), and a program that signs no
    /// link pays none of it. What it spends on the *link* is the token, which
    /// carries the signed document as well as the tag — about
    /// `4/3 × (name + params + 40)` characters — buying one wire format for
    /// all three doors rather than a second, shorter one here.
    fn nvs_core_router_link_signed(_ctx, args: [5]) {
        const MEMBER: &str = "urlSigned";

        let template = link_template(args, MEMBER)?;
        // Unreachable from source: `nvs_types::links` records this argument as
        // the route name it resolved, and `nvs-ir` emits it as a `ConstStr`.
        let name = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Router::{MEMBER} expected {:?} for its resolved route name, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?;
        let ring = crate::keyring::borrow(args, 3, URL_SIGNED)?;
        let until = crate::signature::until_of(args, 4, MEMBER)?;
        if carries_reserved(&args[2]) {
            return Err(Fault::thrown(format!(
                "Core\\Router::{MEMBER}(): `$params` holds `{SIG_NAME}`, which is the parameter \
                 this member writes the signature into. A signed link carries exactly one of \
                 them, so the key is refused here rather than overwritten"
            )));
        }

        let payload = signed_payload(name, &args[2], MEMBER)?;
        // `mint` borrows the payload, so this frame still owns the one
        // reference `signed_payload` built — and owns it on the refusing path
        // too, which is why the `?` is below the release rather than on the
        // call.
        let minted = crate::signature::mint(
            Domain::Route,
            until,
            &payload,
            &ring,
            URL_SIGNED,
            "$params",
        );
        discard(payload);
        let token = minted?;

        let mut out = substitute(template, &args[2], MEMBER)?;
        out.push(if out.contains('?') { '&' } else { '?' });
        out.push_str(SIG_NAME);
        out.push('=');
        out.push_str(&token);
        produced(&out)
    }
}

/// `Core\Router::signedRoute`, spelled the way [`crate::keyring`]'s refusals
/// name it.
const SIGNED_ROUTE: &str = r"Core\Router::signedRoute";

/// The one sentence every refused verification produces before the lifetime has
/// been looked at — [`crate::signature`]'s own `refused`, read at this door.
///
/// One function for the same reason that one has: the refusals are told apart
/// only by which of them a caller can *write*, so a second wording at a second
/// site is how "the token is forged" and "the request matched nothing" would
/// come to be distinguishable here and nowhere else
/// (`rule:core-api/one-refusal-except-expiry`).
fn unverified() -> Fault {
    Fault::thrown(format!(
        "Core\\Router::signedRoute(): this request carries no signature `$keys` made for the \
         route it matched. Every way of not carrying one — no `{SIG_NAME}` parameter and two of \
         them, an altered token, one minted at another door or under a key that has been \
         retired, a parameter added, removed or edited, and a request that matched no named \
         route at all — is this one sentence, so a forgery says nothing about which half of it \
         failed."
    ))
}

/// One capture as the text that was signed for it — [`capture_value`]'s arms,
/// read as text rather than as the value a handler receives.
///
/// **It reaches no program code**, which [`capture_value`]'s `Parses` arm does.
/// The payload has to be derivable *before* the tag has been checked, so a
/// class's own `parse` running on the way to a refusal would be a side effect a
/// forger could ask for — and a distinguishable one, since what came back would
/// be the implementor's sentence rather than [`unverified`]'s. A class-typed
/// capture is therefore its segment, which is exactly what a text one is: the
/// text `Core\Router::urlSigned` signed, one step short of the encoding
/// [`substitute`] then applied to write it into the path.
///
/// The four converted arms answer [`nvs_runtime::value_to_string`]'s own
/// rendering of the value the matcher built — `Display` for a `decimal`, the
/// canonical hyphenated form for a `Core\Uuid` — because that is what
/// [`segment_text`] wrote for the same parameter on the minting side. Two
/// spellings of one typed value are one signed thing, which is `rule:core-classes/router-signed-url`'s
/// "the name and its parameters" taken at its word: `/shop/007` and `/shop/7`
/// are the same route with the same `int`, and a signature over the identity
/// says so.
///
/// # Errors
///
/// [`crate::uri::decode_capture`]'s throw, for a segment whose escapes decode
/// to octets that are not UTF-8 — the same one [`capture_value`] raises over
/// the same segment, so a request this refuses is one `Core\Request::route()`
/// would have refused too.
fn capture_text(name: &str, capture: &nvs_runtime::routes::Param) -> Result<String, Fault> {
    use nvs_runtime::routes::Param;

    Ok(match capture {
        Param::Text(text) | Param::Parses { text, .. } => crate::uri::decode_capture(text, name)?,
        Param::Int(number) => number.to_string(),
        Param::Uint(number) => number.to_string(),
        Param::Decimal(value) => value.to_string(),
        Param::Uuid(octets) => {
            let mut buffer = crate::uuid::TEXT;
            crate::uuid::canonical(*octets, &mut buffer).to_owned()
        }
    })
}

/// [`signed_payload`]'s document for the link this request arrived on, rebuilt
/// out of what the door already recorded.
///
/// **Nothing is re-parsed.** The captures are the match's own, taken once
/// before any application code ran (`rule:routing/matched-once-before-the-handler`),
/// and the query is the request's own text with the reserved parameter already
/// lifted out of it — so the two halves of `$params` come back from the two
/// places `urlSigned` wrote them to, and the URL is never read a second time to
/// find out what it says.
///
/// **The query is parsed first and a capture may not collide with it.** The
/// minting side writes the query out of what the path did *not* consume
/// ([`crate::uri::build`] takes the consumed keys and omits them), so a request
/// carrying a query pair under a capture's name is one no signed link could
/// have been minted as. Refusing it is what makes the signature cover the whole
/// of what arrived: every other added pair changes the payload and fails on the
/// comparison, and this is the one that would otherwise have been overwritten
/// by the capture and never noticed.
///
/// # Errors
///
/// [`unverified`] for that collision, [`capture_text`]'s decode, and
/// [`crate::uri::parse_query`]'s throw for a parameter name or value whose
/// escapes decode to octets that are not UTF-8. The partly built array is
/// released by its own `Drop` on the way out.
fn derived_payload(
    name: &str,
    matched: &nvs_runtime::routes::Match,
    query: &str,
    member: &str,
) -> Result<Value, Fault> {
    let mut params = crate::uri::parse_query(query, member, crate::uri::Values::Text)?;
    for (bound, capture) in matched.params() {
        if params.has_key(bound.as_bytes()) {
            return Err(unverified());
        }
        let text = capture_text(bound, capture)?;
        params.set(
            NvsStr::new(bound.as_bytes()),
            Value::str(NvsStr::new(text.as_bytes())),
        );
    }

    let mut payload = NvsArray::new();
    payload.set(
        NvsStr::new(b"route"),
        Value::str(NvsStr::new(name.as_bytes())),
    );
    payload.set(NvsStr::new(b"params"), Value::array(params));
    Ok(Value::array(payload))
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::signedRoute(array<secret bytes> $keys): Core\Router\Match`
    /// — the read half of [`nvs_core_router_link_signed`], and the member
    /// `rule:core-classes/router-signed-url` names as verifying "against the
    /// match the server already made".
    ///
    /// **It takes no URL**, and that is the property rather than a
    /// convenience. A member handed a path would have to match it, and a
    /// second match is a second answer about which route this is
    /// (`rule:routing/matched-once-before-the-handler`); a member handed a
    /// route name would let the caller nominate which signature to check. What
    /// is verified here is the request itself, so there is nothing for a caller
    /// to get wrong except the ring.
    ///
    /// The order is load-bearing, exactly as `$uri->verifySignature`'s is. The
    /// payload is derived without reaching a program's own `parse`
    /// ([`capture_text`]); [`crate::signature::confirm`] checks the tag, the
    /// door and *this* payload before any of it is trusted — a token that
    /// authenticates over some other route is the whole of the attack; and the
    /// expiry is judged last, which is what makes it safe to give it a sentence
    /// of its own (`rule:core-api/one-refusal-except-expiry`). The match is
    /// built after all of that, because building it is what runs a class-typed
    /// capture's `parse`, and no forged link should be able to reach it.
    ///
    /// **Cost:** one parse of the request's own query string, one document the
    /// size of the route's name and its parameters, and one HMAC per key tried
    /// until one authenticates. All of it inside the call
    /// (`rule:programs/memory-priority`), and a program that verifies nothing
    /// pays none of it — this member is called by hand, wherever the
    /// application keeps its refusal, because nothing verifies a signature for
    /// you.
    fn nvs_core_router_signed_route(ctx, args: [1]) {
        const MEMBER: &str = "signedRoute";

        let ring = crate::keyring::borrow(args, 0, SIGNED_ROUTE)?;
        // Taken off the carrier before anything else needs the context, for
        // `Core\Request::route()`'s own reason: the answer is `match_value`'s,
        // which reaches this program's classes for a capture typed as one, and
        // the borrow the carrier is read through cannot be alive while it does.
        let inbound = crate::request::served(ctx, SIGNED_ROUTE)?;
        let matched = inbound.route().cloned();
        let query = inbound.query().to_owned();

        // A request that matched nothing, and one whose route declares no name,
        // are folded into the refusal rather than answered apart: `urlSigned`
        // signs a name, so neither could be a link it minted, and saying which
        // would tell a holder something about the table.
        let Some(matched) = matched else {
            return Err(unverified());
        };
        let Some(name) = matched.name() else {
            return Err(unverified());
        };

        let (tokens, rest) = crate::uri::without_signature(&query);
        // Neither none nor two, for `$uri->verifySignature`'s reason: checking
        // one of two is letting an attacker pick which one gets tried.
        if tokens.len() != 1 {
            return Err(unverified());
        }

        let payload = derived_payload(name, &matched, &rest, MEMBER)?;
        // `confirm` borrows the payload, so this frame still owns the one
        // reference `derived_payload` built — and owns it on the refusing path
        // too, which is why the `?` is below the release rather than on the
        // call.
        let confirmed = crate::signature::confirm(
            tokens[0],
            Domain::Route,
            &payload,
            &ring,
            SIGNED_ROUTE,
            "$params",
        );
        discard(payload);
        let Confirmed::Signed(until) = confirmed? else {
            return Err(unverified());
        };
        crate::signature::judge(ctx, until, SIGNED_ROUTE)?;

        match_value(ctx, &matched)
    }
}

/// Argument 0 of a link helper — the prepared template `nvs-ir` emitted.
fn link_template<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Router::{member} expected {:?} for its prepared path, got tag {}",
            Tag::Str,
            args[0].tag_byte()
        ))
    })
}

/// A freshly built `string` result.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(NvsStr::new(text.as_bytes())))
}

/// The answer both members give for a name that reached run time at all — a
/// computed one, or gap 1's named argument. A folded literal never arrives
/// here: it is either a prepared path in [`link`]'s symbols or `E0754`.
///
/// A throw rather than an abort, and that is the difference from
/// [`crate::program`]: `implementing<T>()` is expanded away in `nvs check`, so
/// reaching its helper is a compiler bug. These two are ordinary runtime
/// members — `rule:routing/link-name-and-params-are-checked` says a *computed* `$name` throws — so the throw is a
/// real answer a program can catch, and it stays the answer for an unknown name
/// after the table lands. What changes then is only which names are unknown.
fn no_such_route(member: &str, args: &[nvs_runtime::Value]) -> Fault {
    let name = args[0].as_text().unwrap_or("<not a string>");
    Fault::thrown(format!(
        "Core\\Router::{member}(): no route is named `{name}`. A computed name is not resolved \
         against the compile-time route table (`rule:routing/link-name-and-params-are-checked`); \
         write the route's name as a literal"
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::url(string $name, array<mixed> $params): string` — `rule:routing/link-name-and-params-are-checked`
    /// 's launderer for the URL-path sink.
    ///
    /// This is the *unfolded* member — reached only by a name the compiler
    /// could not read as a literal — so there is no prepared path to substitute
    /// into and no lookup to make: § 4 says a computed name throws, and
    /// [`no_such_route`] is that throw. The resolved twin is
    /// [`nvs_core_router_link`].
    fn nvs_core_router_url(_ctx, args: [2]) {
        Err(no_such_route("url", args))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::urlAbsolute(string $name, array<mixed> $params): string` —
    /// [`nvs_core_router_url`] with `rule:routing/an-absolute-link-takes-a-configured-origin`'s configured origin in front.
    ///
    /// The origin is per mount, falling back to `[app] origin`, and is never
    /// derived from `Host` or `X-Forwarded-Host`. Nothing here resolves one:
    /// the route lookup refuses first, and there is no mount to read an origin
    /// from in a program run off the command line at all.
    fn nvs_core_router_url_absolute(_ctx, args: [2]) {
        Err(no_such_route("urlAbsolute", args))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::urlSigned(string $name, array<mixed> $params, {keys, until}): string`
    /// — [`nvs_core_router_url`]'s answer for a name that reached run time,
    /// with two more arguments it never gets as far as reading.
    ///
    /// The lookup refuses first on purpose. A computed name has no route, so
    /// there is nothing to sign the identity *of*, and checking the ring
    /// before saying so would answer a question about the key material when
    /// the mistake is in the call. The resolved twin is
    /// [`nvs_core_router_link_signed`].
    fn nvs_core_router_url_signed(_ctx, args: [4]) {
        Err(no_such_route("urlSigned", args))
    }
}

// ---------------------------------------------------------------- `rule:routing/matched-once-before-the-handler`'s match

/// The [`MATCH`] one request carries, built out of the match the door already
/// took — the whole of how [`nvs_runtime::routes`]' row reaches a program.
///
/// `pub(crate)` because the readers are [`crate::request`]'s `route()` and
/// [`nvs_core_router_match`]: § 1 puts the match on the request rather than on
/// the router, and the class both answer with is this module's because
/// `Core\Router\Match` is a `Core\Router` name.
///
/// # Errors
///
/// [`capture_value`]'s throw, whose doc owns the decode this walk performs. The
/// partly built array is released by its own `Drop` on the way out, so a
/// refused capture costs the ones already converted and nothing else.
pub(crate) fn match_value(
    ctx: &mut nvs_runtime::Ctx,
    matched: &nvs_runtime::routes::Match,
) -> Result<Value, Fault> {
    let mut params = NvsArray::new();
    for (name, capture) in matched.params() {
        let value = capture_value(ctx, name, capture)?;
        params.set(NvsStr::new(name.as_bytes()), value);
    }
    Ok(crate::instance::build(
        &MATCH,
        [
            match matched.name() {
                Some(name) => Value::str(NvsStr::new(name.as_bytes())),
                None => Value::null(),
            },
            Value::array(params),
        ],
    ))
}

/// Which case of [`METHOD`] a table row's declared verb is, by its ordinal.
///
/// The comparison is ASCII-case-insensitive for the reason
/// [`nvs_runtime::routes::Routes::match_request`]'s is: a row's verb is the
/// case's own name (`Get`) in a table the compiler built, and a table built by
/// hand may spell it as the wire token (`GET`). `None` is a verb this roster
/// does not name, which no compiled table can hold — `#[Route(method: …)]`
/// takes a case of this enum and nothing else.
pub(crate) fn method_case(verb: &str) -> Option<i64> {
    METHOD
        .cases
        .iter()
        .find(|(case, _)| case.eq_ignore_ascii_case(verb))
        .map(|(_, ordinal)| *ordinal)
}

/// The [`METHOD`] case in `value` as the verb a table row spells, or a fatal
/// naming `member`, which is every row that reads a verb out of an argument.
///
/// The inverse of [`method_case`], and its mirror image in what it may assume:
/// a case crosses as its ordinal (`rule:enums/closed-integer-type`), so what arrives is one of this
/// roster's own integers and the lookup cannot miss for anything a program
/// could have written.
///
/// # Errors
///
/// A [`Fault::fatal`], on [`crate::log`]'s `level_of` terms: the row's
/// parameter is a [`CoreTy::Enum`], so `E0401` refuses anything that is not a
/// case of it at the call, and an ordinal outside the roster is a lowering bug
/// rather than something a `catch` could answer.
pub(crate) fn method_verb(value: &Value, member: &str) -> Result<&'static str, Fault> {
    value
        .as_int()
        .and_then(|ordinal| {
            METHOD
                .cases
                .iter()
                .find(|(_, case)| *case == ordinal)
                .map(|(name, _)| *name)
        })
        .ok_or_else(|| {
            // Unreachable from source, on `crate::log`'s `level_of` terms: the
            // row's parameter is a `CoreTy::Enum`, so `E0401` refuses anything
            // that is not a case of it before this body runs, and a case
            // crosses as one of this roster's own ordinals.
            Fault::fatal(format!(
                "{member} expected a `{METHOD_NAME}` case, got tag {} value {:?}",
                value.tag_byte(),
                value.as_int()
            ))
        })
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::match(Core\Http\Method $method, tainted string $path): ?Core\Router\Match`
    /// — `rule:routing/matched-once-before-the-handler`'s second entry, over the table
    /// [`Ctx::routes`](nvs_runtime::Ctx::routes) holds.
    ///
    /// **Nothing here is a request**, and that is the whole difference from
    /// `Core\Request::route()`: the verb and the path are the caller's, the
    /// walk is [`nvs_runtime::routes::Routes::match_request`]'s — the same one
    /// the door takes, so the two cannot answer differently about one path —
    /// and matching still dispatches nothing, which keeps `rule:routing/matching-is-not-dispatching`'s
    /// refusal untouched.
    ///
    /// **A program with no `#[Route]` answers `null`**, not a throw, for
    /// [`nvs_core_router_methods_for`]'s reason: `rule:routing/table-is-opt-in`'s table is opt-in
    /// and "no route claims this path" is exactly true of a program that
    /// declares none.
    ///
    /// **The captures come back decoded**, because [`match_value`] is shared
    /// with the served-request reader and [`capture_value`] owns that rule for
    /// both.
    fn nvs_core_router_match(ctx, args: [2]) {
        let verb = method_verb(&args[0], "Core\\Router::match")?;
        // Unreachable from source for the reason `methodsFor`'s own read
        // states: the row's parameter is `CoreTy::Text(Qual::Neutral)`, so
        // `E0401` refuses anything but a `string` before this body runs.
        let path = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Router::match expected a `string` path, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        // Bound before the crossing rather than matched inside it: the walk
        // borrows the table the context holds, and [`match_value`] needs the
        // context itself to reach a capture's class, so the answer has to be
        // owned by the time it is handed over.
        let matched = ctx.routes().and_then(|table| table.match_request(verb, path));
        match matched {
            Some(matched) => match_value(ctx, &matched),
            None => Ok(Value::null()),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::methodsFor(tainted string $path): array<Core\Http\Method>`
    /// — `rule:routing/a-refused-verb-is-not-a-missing-path`'s second answer, over the table
    /// [`Ctx::routes`](nvs_runtime::Ctx::routes) holds.
    ///
    /// **An empty array is the `404` and a non-empty one is the `405`**, whose
    /// `Allow:` the caller writes out of it. § 2's own rule is that this is
    /// asked *only after* `Core\Request::route()` answered `null`, so nothing
    /// here is on a served request's path; the walk it performs is
    /// [`nvs_runtime::routes::Routes::methods_for`]'s, which is where the two
    /// answers and their reasoning live.
    ///
    /// **A program with no `#[Route]` answers the empty array**, not a throw:
    /// `rule:routing/table-is-opt-in`'s table is opt-in, and "no route claims this path" is
    /// exactly true of a program that declares none. That is the same reading
    /// [`Ctx::route`](nvs_runtime::Ctx::route) takes of the absent table.
    ///
    /// **Each verb appears once**, because `methods_for` answers verbs rather
    /// than rows, and an enum case is its ordinal on the way out (`rule:enums/closed-integer-type`) —
    /// the same crossing `Core\Request::method` makes in the other direction.
    fn nvs_core_router_methods_for(ctx, args: [1]) {
        // Unreachable from source, as every mistyped argument slot is: the
        // row's parameter is `CoreTy::Text(Qual::Neutral)`, so `E0401` refuses
        // anything but a `string` before this body runs. The tag is also the
        // whole UTF-8 guarantee (`rule:types/conversion`), so there is nothing left to
        // check about the path before comparing it against a declared one.
        let path = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Router::methodsFor expected a `string` path, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let mut verbs = NvsArray::new();
        if let Some(table) = ctx.routes() {
            for verb in table.methods_for(path) {
                if let Some(ordinal) = method_case(verb) {
                    verbs.append(Value::int(ordinal));
                }
            }
        }
        Ok(Value::array(verbs))
    }
}

/// One capture as [`CAPTURE`] spells it — the one place
/// [`nvs_runtime::routes::Param`]'s six forms become Novis values.
///
/// The `Core\Uuid` arm allocates an instance, which is why it is this crate's:
/// the octets crossed as bytes precisely so that the class stays where it is
/// declared, and [`crate::uuid::of_octets`] is the seam `rule:core-classes/db-column-types`'s `UUID`
/// column already arrives on.
///
/// # A capture is percent-decoded here, once, and this is that rule's home
///
/// The text arm decodes; nothing before it does, and nothing after it may. The
/// segment travels from the door still as the peer wrote it —
/// [`nvs_runtime::routes::Route::convert`] must see it that way, because it
/// runs *first* and a `{n: uint}` route reading a decoded `%34` would match `4`
/// — so this crossing is the single point where a capture stops being a piece
/// of a URL and starts being a value. Deciding it here also decides it once for
/// both readers: `Core\Request::route()` on a served request and
/// `Core\Router::match` on a path a program chose share [`match_value`].
///
/// **A capture whose octets are not UTF-8 refuses**, which is
/// [`crate::uri::decode_capture`]'s throw. [`CAPTURE`]'s text arm is a
/// `tainted string` and `rule:types/bytes` guarantees a `string` is valid UTF-8 by
/// construction, so `%ff` in a path segment has no capture to become; answering
/// the undecoded text instead would hand the program a `%20` that every other
/// capture had already lost, and answering `bytes` would widen [`CAPTURE`] to a
/// type the route declaration cannot spell.
///
/// # This is where a capture typed as a class built from text converts
///
/// The decode above is the first half of what that arm needs and the class's
/// own `parse` is the second, so the two run here in that order and nowhere
/// else. [`nvs_runtime::routes::CaptureConv::Parses`] is the home of why the
/// matcher may not make the call: it runs at the door with no program
/// installed, ahead of everything that rate-limits a request. By the time this
/// runs the route has already matched, so a segment the class refuses is not a
/// failed match — it is [`parsed`]'s throw over a route that claimed the path,
/// which is `rule:security/route-capture-is-laundered-by-its-type`'s
/// class-typed exception and the `400` a handler answers it with.
///
/// # Errors
///
/// That throw, from the text arm and from the class-typed one, which decodes
/// the same way: no other conversion the matcher performs has an encoded
/// spelling to decode — a digit, a `-` and a hex digit are all unreserved
/// bytes. Plus whatever the class itself threw, unchanged, per [`parsed`].
fn capture_value(
    ctx: &mut nvs_runtime::Ctx,
    name: &str,
    capture: &nvs_runtime::routes::Param,
) -> Result<Value, Fault> {
    Ok(match capture {
        nvs_runtime::routes::Param::Text(text) => Value::str(NvsStr::new(
            crate::uri::decode_capture(text, name)?.as_bytes(),
        )),
        nvs_runtime::routes::Param::Int(number) => Value::int(*number),
        nvs_runtime::routes::Param::Uint(number) => Value::uint(*number),
        nvs_runtime::routes::Param::Decimal(value) => Value::decimal(*value),
        nvs_runtime::routes::Param::Uuid(octets) => crate::uuid::of_octets(*octets),
        nvs_runtime::routes::Param::Parses { class, text } => {
            let text = Value::str(NvsStr::new(
                crate::uri::decode_capture(text, name)?.as_bytes(),
            ));
            parsed(ctx, class, text)?
        }
    })
}

/// What `class`'s own `parse` made of one decoded segment — the binding site
/// [`nvs_runtime::routes::CaptureConv::Parses`] defers to.
///
/// `crate::command`'s `parse_each` is the same reach one table along, and the
/// two answer a refusal differently on purpose: a command line is a person
/// typing, so a word the class refused becomes the sentence a usage page leads
/// with, while a request has already matched a route by the time this runs and
/// the throw is what a handler answers `400` with. The throw is the class's
/// own and travels unchanged, so what a program catches is the sentence the
/// implementor wrote.
///
/// # Errors
///
/// The class's own throw, and a [`Fault::fatal`] for a class this program's
/// table does not hold — unreachable from source, since a capture at a class
/// that does not implement `Parses` is refused at the signature, and one that
/// does owes `parse` to `nvs_types::conformance` before a route row naming it
/// is ever built.
fn parsed(ctx: &mut nvs_runtime::Ctx, class: &str, text: Value) -> Result<Value, Fault> {
    let outcome = nvs_runtime::call_static(ctx, &format!("{class}::parse"), &[text]);
    #[expect(
        unsafe_code,
        reason = "this frame owns the one reference `text` holds, and `call_static` \
                  retained its own for the callee to release"
    )]
    unsafe {
        text.release();
    }
    match outcome {
        Ok(Some(value)) => Ok(value),
        // Unreachable from source, on `crate::command`'s `parse_each` terms
        // exactly: `E0746` refuses a capture at a class that does not implement
        // `Parses`, and one that does owes `parse` to `nvs_types::conformance`
        // before a route row naming it is ever built. A miss here is a class
        // table that does not match the route table built beside it, which no
        // program can write its way into.
        Ok(None) => Err(Fault::fatal(format!(
            "internal error: `{class}::parse` is not in this program's class table"
        ))),
        Err(fault) => Err(fault),
    }
}

/// Slot `index` of a [`MATCH`] receiver, retained for the caller — the shape
/// [`crate::request`]'s `part_slot` already has, for the same reason: a slot
/// read is a borrow, and Novis code handed the value needs a reference.
fn match_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &MATCH, member)?;
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
    /// `Core\Router\Match::name(): ?string` — `rule:routing/matched-once-before-the-handler`'s declared name,
    /// which `rule:observability/route-label-is-the-declared-name`'s `route` label reads and `Core\Router::url` resolves.
    fn nvs_core_router_match_name(_ctx, args: [1]) {
        match_slot(args, MATCH_ROUTE_NAME, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router\Match::params(): array<tainted string|int|uint|decimal|Core\Uuid|Parses>`
    /// — every capture the path filled, keyed by the parameter it binds.
    ///
    /// The array is built once by [`match_value`] and read back here, so the
    /// two members answer the same values rather than two walks of one row.
    fn nvs_core_router_match_params(_ctx, args: [1]) {
        match_slot(args, MATCH_PARAMS, "params")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router\Match::param(string $name): ?(tainted string|int|uint|decimal|Core\Uuid|Parses)`
    /// — [`nvs_core_router_match_params`] read at one key.
    ///
    /// **`null` for a name the route does not declare**, rather than a throw,
    /// and that is [`MATCH_PARAM_DOC`]'s stated answer rather than
    /// `Core\Regex\Match::group`'s: an optional `{name?}` the request left off
    /// is absent from the array and is the ordinary case § 4 exists for, so a
    /// member that threw for an unknown name would have to tell two absences
    /// apart that the table itself does not.
    fn nvs_core_router_match_param(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &MATCH, "param")?;
        let params = crate::instance::slot(receiver, MATCH_PARAMS);
        // Unreachable from source: the slot's layout is this crate's on both
        // sides — [`match_value`] is the only writer of it and always writes a
        // `Value::array` — and a program can reach a `Core\Router\Match` no
        // other way, since `CoreTy::Instance` gives the class no constructor
        // and `E0401` refuses a receiver that is not one.
        let array = params.array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Router\\Match::param found tag {} in its `params` slot",
                params.tag_byte()
            ))
        })?;
        // Unreachable from source for the reason every mistyped argument slot
        // is: the row's parameter is `CoreTy::Text(Qual::Neutral)`, so `E0401`
        // refuses anything that is not a `string` before this body runs.
        let key = args[1].as_str_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Router\\Match::param expected a `string` name, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        let found = crate::arr::borrowed(array).get(key).unwrap_or_else(Value::null);
        #[expect(
            unsafe_code,
            reason = "the params array owns the reference this borrowed read returned, \
                      so the caller needs one of its own; a `null` owns none and \
                      retaining it is a no-op"
        )]
        unsafe {
            found.retain();
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::routes::Param;

    use nvs_runtime::{ClassTable, Ctx, ErrorClass, MethodRow, NvsFn, OK, OutputSink, Value};

    use super::{METHOD, capture_value};

    /// The text a capture becomes, or the message it refused with — the seam
    /// [`capture_value`] is, read back as something a case can assert.
    fn crossed(name: &str, capture: &Param) -> Result<String, String> {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        match capture_value(&mut ctx, name, capture) {
            Ok(value) => Ok(String::from_utf8(
                value
                    .as_str_bytes()
                    .expect("a text capture crosses as a `string`")
                    .to_vec(),
            )
            .expect("the decode refuses anything else")),
            Err(nvs_runtime::Fault::Thrown(_, message)) => Err(message.into_owned()),
            Err(_) => panic!("a capture refuses by throwing, so a program can catch it"),
        }
    }

    /// The claim the stage freezes: the crossing decodes, and decodes *once*.
    /// `%2520` is the case that tells "once" from "until it stops changing" —
    /// a second pass would answer a space where one pass answers `%20` — and
    /// `%ff` is the boundary, since a capture binds as a `tainted string` and
    /// `rule:types/bytes` leaves no `string` for those octets to be.
    #[test]
    fn a_route_capture_is_percent_decoded_once_where_it_crosses() {
        assert_eq!(
            crossed("slug", &Param::Text("hello%20world".to_owned())),
            Ok("hello world".to_owned())
        );
        assert_eq!(
            crossed("slug", &Param::Text("hello%2520world".to_owned())),
            Ok("hello%20world".to_owned())
        );
        // A path segment, not a form value: `+` is a literal plus here and only
        // `%2B` is one on the other row.
        assert_eq!(
            crossed("slug", &Param::Text("a+b".to_owned())),
            Ok("a+b".to_owned())
        );
        assert_eq!(
            crossed("slug", &Param::Text("caf%C3%A9".to_owned())),
            Ok("café".to_owned())
        );
        let refused = crossed("slug", &Param::Text("%ff".to_owned())).expect_err("no `string`");
        assert!(refused.contains("`slug`"), "{refused}");
        assert!(refused.contains("byte 0"), "{refused}");
    }

    /// The other half of the same rule, and the reason the decode is on this
    /// side of `Route::convert` rather than in `nvs_runtime::routes`: a
    /// converted capture is a number the matcher already read out of the raw
    /// segment, and nothing here touches it. Were the decode to run first,
    /// `%34` would reach a `{n: uint}` route as `4`; it does not reach it at
    /// all, because no digit has an encoded spelling.
    #[test]
    fn a_uint_capture_is_unchanged_by_the_decode() {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let crossed =
            capture_value(&mut ctx, "n", &Param::Uint(20)).expect("a number refuses nothing");
        assert_eq!(crossed.as_uint(), Some(20));
        let big =
            capture_value(&mut ctx, "n", &Param::Uint(u64::MAX)).expect("a number refuses nothing");
        assert_eq!(big.as_uint(), Some(u64::MAX));
        // The same is true of every other converted arm — the text arm is the
        // only one with an escape to read.
        let signed =
            capture_value(&mut ctx, "n", &Param::Int(-7)).expect("a number refuses nothing");
        assert_eq!(signed.as_int(), Some(-7));
    }

    thread_local! {
        /// The text [`parse_handler`] was handed, so that a crossing which
        /// answered plausibly without ever reaching the class — or reached it
        /// with the wrong slot — fails here rather than on the value alone.
        static PARSED: std::cell::RefCell<Option<String>> =
            const { std::cell::RefCell::new(None) };
    }

    /// A `Parses` implementor's `parse`, as `nvs-codegen` would have compiled
    /// it: slot 0 is the called class and slot 1 is the one `string` the
    /// interface declares.
    ///
    /// It answers `true` — a value no segment's own text could be — so that
    /// "the class's answer replaced the segment" is asserted on the slot and
    /// not only on what arrived.
    #[expect(
        unsafe_code,
        reason = "compiled code's own signature, which `call_static` calls through: \
                  exactly two live values and the address of a live `Value` for the \
                  result, neither expressible in the type"
    )]
    unsafe extern "C" fn parse_handler(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        let text = unsafe { *args.add(1) };
        PARSED.with_borrow_mut(|parsed| {
            *parsed = text.as_text().map(str::to_owned);
        });
        unsafe {
            for index in 0..2 {
                (*args.add(index)).release();
            }
            *out = Value::bool(true);
        }
        OK
    }

    /// [`parse_handler`]'s twin that refuses, which is the whole of what a
    /// `Parses` implementor does to reject text: it throws, and the sentence
    /// is the class's own.
    #[expect(
        unsafe_code,
        reason = "as `parse_handler`, and the pending message is set through the \
                  context the ABI hands every compiled function"
    )]
    unsafe extern "C" fn refusing_parse(
        ctx: *mut Ctx,
        args: *const Value,
        _out: *mut Value,
    ) -> i32 {
        unsafe {
            for index in 0..2 {
                (*args.add(index)).release();
            }
            (*ctx).set_pending("a slug is lower case and digits");
        }
        nvs_runtime::THROWN
    }

    /// A context carrying a class table with `App\Slug::parse` bound to
    /// `code` — the armed table [`capture_value`] needs and the door has not.
    ///
    /// `crate::command`'s `dispatching` is this same fixture one table along,
    /// and is the home of why the handle is `set_runtime_error_class`: that
    /// handle *is* a context's anchor into the compiled unit's classes, which
    /// is how `nvs_runtime::call_static` turns a `Class::method` label into an
    /// address.
    fn parsing(code: NvsFn) -> Ctx {
        let mut classes = ClassTable::new();
        let id = classes.define("App\\Slug", &[] as &[&str], &[]);
        classes.set_methods(
            id,
            vec![MethodRow {
                name: "parse".to_owned(),
                code: code as *const u8,
                arity: 1,
                param_tags: 0,
                public: true,
                native: false,
            }],
        );
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_runtime_error_class(ErrorClass::new(std::sync::Arc::new(classes), id));
        ctx
    }

    /// The class-typed capture's other half, on the side that may take it: the
    /// segment reaches the class's own `parse` here, before the handler is
    /// called, and what the parameter receives is what `parse` answered.
    ///
    /// The decode runs first and is asserted on the slot rather than on the
    /// result, because the two orders answer the same value for every segment
    /// with no escape in it: a class handed `caf%C3%A9` would be judging text
    /// no other capture ever sees.
    #[test]
    fn a_parses_capture_converts_the_segment_before_the_handler_is_reached() {
        let mut ctx = parsing(parse_handler);
        PARSED.with_borrow_mut(|parsed| *parsed = None);
        let crossed = capture_value(
            &mut ctx,
            "target",
            &Param::Parses {
                class: "App\\Slug".to_owned(),
                text: "caf%C3%A9".to_owned(),
            },
        )
        .expect("the class parsed the segment");
        assert_eq!(
            PARSED.with_borrow(Clone::clone),
            Some("café".to_owned()),
            "the decode runs first, so `parse` reads the segment and not its escapes"
        );
        assert_eq!(
            crossed.as_text(),
            None,
            "the segment does not survive the crossing"
        );
        assert_eq!(crossed.as_bool(), Some(true), "`parse`'s answer does");
    }

    /// The exception `rule:security/route-capture-is-laundered-by-its-type`
    /// carves out of its own "a failed conversion is not a match": a segment
    /// the class refuses is a **matched** request carrying a bad value, so the
    /// refusal is the throw a handler answers `400` with rather than a fall
    /// through to the next route and a `404`.
    ///
    /// Both sides are asserted here rather than the throw alone, because "over
    /// a route that matched" is the half that makes it a `400` — and the match
    /// is `nvs_runtime::routes`' answer, not an assumption this side is free
    /// to make about the other.
    #[test]
    fn a_segment_the_class_refuses_is_a_matched_bad_value_rather_than_no_match() {
        let routes = nvs_runtime::routes::Routes::new(vec![nvs_runtime::routes::Route::new(
            "Get",
            "/deploy/{target}",
            None,
            "App\\Deploys::show",
            None,
            vec![nvs_runtime::routes::Capture {
                name: "target".to_owned(),
                conv: nvs_runtime::routes::CaptureConv::Parses("App\\Slug".to_owned()),
            }],
        )]);
        let matched = routes
            .match_request("GET", "/deploy/PROD!!")
            .expect("the door matches on shape, so a segment the class refuses still matches");
        let capture = matched.param("target").expect("the capture the path names");

        let mut ctx = parsing(refusing_parse);
        match capture_value(&mut ctx, "target", capture) {
            // The class's throw travels unchanged rather than being reworded
            // here, which is what `Fault::Pending` *is*: the sentence is still
            // on the context, where the handler's own `catch` reads it.
            // `crate::command`'s `parse_each` is the same throw read the other
            // way, into a usage page, and the two arms are why that split is
            // one function apart rather than one variant apart.
            Err(nvs_runtime::Fault::Pending(_)) => {
                let said = ctx.take_pending().expect("the class's own sentence");
                assert!(
                    said.contains("a slug is lower case and digits"),
                    "the implementor's sentence is what the handler answers with: {said}"
                );
            }
            Err(_) => panic!("a refusal is the class's own throw, not the engine's"),
            Ok(_) => panic!("a `parse` that threw converted nothing"),
        }
    }

    /// The tail is what the doc comment promises, asserted rather than
    /// described: an inserted case that pushes `Post` along breaks the bound a
    /// later CSRF check is meant to take, and nothing else would notice. The
    /// bound is written here rather than exported as a constant — no caller
    /// exists to take it yet, and a `pub(crate)` nothing reads is dead code.
    #[test]
    fn the_csrf_set_is_the_contiguous_tail() {
        let unsafe_cases: Vec<&str> = METHOD
            .cases
            .iter()
            .filter(|(_, value)| *value >= 4)
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(unsafe_cases, ["Post", "Put", "Patch", "Delete"]);
    }

    /// Each case's value is its own index, so the tail bound above is also the
    /// position `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`'s roster reads at.
    #[test]
    fn every_case_carries_a_distinct_value_in_declaration_order() {
        for (index, (_, value)) in METHOD.cases.iter().enumerate() {
            assert_eq!(*value, i64::try_from(index).unwrap());
        }
    }

    /// `/docs/{slug}` as `nvs_types::UrlPiece::prepared` writes it — one tag
    /// byte per piece, `\u{1}` between them.
    ///
    /// Built by hand because `nvs-types` is the crate that depends on this
    /// one: the writer is one layer above and cannot be called from here, so
    /// the layout in [`super::link`] is what the two sides share.
    fn docs_template() -> String {
        format!(
            "{}/docs{}{}slug",
            super::link::LITERAL as char,
            super::link::PIECE_SEPARATOR,
            super::link::REQUIRED as char
        )
    }

    /// One folded link helper, driven the way compiled code drives it.
    ///
    /// The callee borrows its arguments, so the caller still owns every one of
    /// them afterwards and the answer is the one fresh reference this returns.
    fn linked(symbol: NvsFn, args: &[Value]) -> String {
        let mut ctx = Ctx::buffered();
        let answer = nvs_runtime::call(symbol, &mut ctx, args)
            .expect("a link over a resolved name and a well-formed ring throws nothing");
        assert!(
            ctx.take_pending().is_none(),
            "and leaves nothing pending behind it"
        );
        let text = String::from_utf8(
            answer
                .as_str_bytes()
                .expect("a link answers a `string`")
                .to_vec(),
        )
        .expect("a link is built out of `str`");
        #[expect(
            unsafe_code,
            reason = "the helper hands back exactly one fresh reference, and the text \
                      has been copied out of it"
        )]
        unsafe {
            answer.release();
        }
        text
    }

    /// Releases a value this test frame built and handed to nobody.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built"
        )]
        unsafe {
            value.release();
        }
    }

    /// § 4 calls `url` the launderer for the URL-path sink, and `urlSigned` is
    /// that member with a parameter on the end — so the claim is an
    /// *agreement* rather than a second escaping rule, and it is asserted on
    /// both halves.
    ///
    /// The registry half is the qualifier: both answer a plain `string`, which
    /// is what says a `tainted` value went in and a laundered one came out
    /// (`rule:security/launderers-are-sink-named`). The run-time half is the
    /// text: for one name and one `$params`, everything the signed member
    /// writes in front of `_sig` is the other member's answer byte for byte,
    /// so a hostile value that cannot leave its segment there cannot leave it
    /// here either. A member that grew its own encoder would still pass the
    /// first half.
    #[test]
    fn url_signed_launders_for_the_url_path_sink_exactly_as_url_does() {
        let row = |name: &str| {
            super::CLASS
                .members()
                .find(|member| member.name == name)
                .expect("the member is registered")
        };
        assert!(matches!(row("url").return_ty, crate::registry::CoreTy::Str));
        assert!(matches!(
            row("urlSigned").return_ty,
            crate::registry::CoreTy::Str
        ));

        let template = docs_template();
        // Every byte a URL gives a meaning to, in one capture: a member that
        // escaped one set too few would leave the segment here.
        let mut params = nvs_runtime::NvsArray::new();
        params.set(
            nvs_runtime::NvsStr::new(b"slug"),
            Value::str(nvs_runtime::NvsStr::new(b"a/b?c#d e")),
        );
        let params = Value::array(params);
        let ring = crate::keyring::tests::ring_of(&[&[7; 32]]);

        let plain = linked(
            super::nvs_core_router_link,
            &[
                Value::str(nvs_runtime::NvsStr::new(template.as_bytes())),
                params,
            ],
        );
        let signed = linked(
            super::nvs_core_router_link_signed,
            &[
                Value::str(nvs_runtime::NvsStr::new(template.as_bytes())),
                Value::str(nvs_runtime::NvsStr::new(b"Docs::show")),
                params,
                ring,
                Value::null(),
            ],
        );

        assert_eq!(plain, "/docs/a%2Fb%3Fc%23d%20e");
        let (path, token) = signed
            .split_once("?_sig=")
            .expect("the signed link ends in the reserved parameter");
        assert_eq!(path, plain);
        assert!(
            !token.is_empty()
                && token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)),
            "the token is unpadded URL-safe base64: {token}"
        );

        dropped(params);
        dropped(ring);
    }

    /// The table the two `signedRoute` cases below match against — one named
    /// route with one `int` capture, which is the shape `urlSigned` signs.
    fn shop() -> nvs_runtime::routes::Routes {
        nvs_runtime::routes::Routes::new(vec![nvs_runtime::routes::Route::new(
            "Get",
            "/shop/{id}",
            Some("Shop::show".to_owned()),
            "Shop::show",
            None,
            vec![nvs_runtime::routes::Capture {
                name: "id".to_owned(),
                conv: nvs_runtime::routes::CaptureConv::Int,
            }],
        )])
    }

    /// A context answering a request the door has already dealt with.
    ///
    /// `arrived` is the path the request carries and `matched` is the path the
    /// table was asked about — the same thing everywhere except in the case
    /// that proves this member reads the door's answer rather than the URL.
    /// `mount` is what the door stripped, which is a fact about where the unit
    /// is served and never about which route it is.
    fn serving(mount: &str, arrived: &str, matched: Option<&str>, query: &str) -> Ctx {
        let mut inbound = nvs_runtime::Inbound::new("GET", arrived, query);
        inbound.set_mount(mount, &[]);
        if let Some(path) = matched {
            inbound.set_route(
                shop()
                    .match_request("GET", path)
                    .expect("the table claims the path the case names"),
            );
        }
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_inbound(inbound);
        ctx
    }

    /// `Core\Router::signedRoute($ring)` over that context — the matched
    /// route's name, or the sentence it refused with.
    fn verified(ctx: &mut Ctx, ring: Value) -> Result<String, String> {
        let answered = nvs_runtime::call(super::nvs_core_router_signed_route, ctx, &[ring]);
        let refusal = ctx.take_pending().map(std::borrow::Cow::into_owned);
        let matched = match answered {
            Ok(matched) => matched,
            Err(_) => {
                return Err(refusal.expect("a refusal leaves its message on the context"));
            }
        };
        // Read through the member a program reads it through, so that a match
        // built with its slots the wrong way round fails here.
        let named = nvs_runtime::call(super::nvs_core_router_match_name, ctx, &[matched])
            .expect("a match answers its own name");
        let name = String::from_utf8(
            named
                .as_str_bytes()
                .expect("this route declares a name")
                .to_vec(),
        )
        .expect("a route's name is `str`");
        dropped(named);
        dropped(matched);
        Ok(name)
    }

    /// One signed link for route `Shop::show`, as the path and query a request
    /// would arrive carrying.
    fn signed_link(ring: Value, id: i64) -> (String, String) {
        let template = format!(
            "{}/shop{}{}id",
            super::link::LITERAL as char,
            super::link::PIECE_SEPARATOR,
            super::link::REQUIRED as char
        );
        let mut params = nvs_runtime::NvsArray::new();
        params.set(nvs_runtime::NvsStr::new(b"id"), Value::int(id));
        let params = Value::array(params);
        let link = linked(
            super::nvs_core_router_link_signed,
            &[
                Value::str(nvs_runtime::NvsStr::new(template.as_bytes())),
                Value::str(nvs_runtime::NvsStr::new(b"Shop::show")),
                params,
                ring,
                Value::null(),
            ],
        );
        dropped(params);
        let (path, query) = link.split_once('?').expect("a signed link carries `_sig`");
        (path.to_owned(), query.to_owned())
    }

    /// The property the pair exists for, and the one a signature over the
    /// assembled path cannot have: one compiled table serves at `/ModuleA`, at
    /// `/ModuleB` or at `/` (`rule:http-server/a-mount-table-expands-at-boot`),
    /// and a link minted before an operator remounted still verifies after.
    ///
    /// Asserted over two requests that differ *only* in what the door stripped,
    /// because that is exactly what a remount changes: the same link, the same
    /// ring, the same match, and the same answer. `crate::uri`'s
    /// `the_same_link_signed_as_a_path_stops_verifying_when_the_mount_moves` is
    /// the other half — the door where the same move is fatal.
    #[test]
    fn url_signed_verifies_through_signed_route_after_the_mount_prefix_changes() {
        let ring = crate::keyring::tests::ring_of(&[&[7; 32]]);
        let (path, query) = signed_link(ring, 7);
        assert_eq!(path, "/shop/7", "the link is minted mounted nowhere");

        let mut here = serving("/ModuleA", &path, Some(&path), &query);
        let mut moved = serving("/ModuleB", &path, Some(&path), &query);
        assert_eq!(
            verified(&mut here, ring),
            Ok("Shop::show".to_owned()),
            "the link verifies where it was minted"
        );
        assert_eq!(
            verified(&mut moved, ring),
            verified(&mut here, ring),
            "and says the same thing one remount later"
        );

        dropped(ring);
    }

    /// `rule:routing/matched-once-before-the-handler` read at this door: the
    /// captures the payload is rebuilt from are the door's match, and the URL
    /// is never asked what it says.
    ///
    /// The two halves are asserted against each other rather than described. A
    /// request whose *path* says `id = 9` while the match the door recorded
    /// says `id = 7` still verifies a token minted for `7`, which a member that
    /// re-parsed the path could not do; and a request whose path would match
    /// the table but which carries no match refuses, which a member that
    /// matched for itself could not do. Either one alone is satisfied by a
    /// member that reads the right thing for the wrong reason.
    #[test]
    fn signed_route_verifies_against_the_match_the_server_already_made_and_reparses_nothing() {
        let ring = crate::keyring::tests::ring_of(&[&[7; 32]]);
        let (path, query) = signed_link(ring, 7);

        let mut elsewhere = serving("", "/shop/9", Some(&path), &query);
        assert_eq!(
            verified(&mut elsewhere, ring),
            Ok("Shop::show".to_owned()),
            "the payload is the recorded match's, so the path it arrived on is not read"
        );

        let mut unmatched = serving("", &path, None, &query);
        let refused = verified(&mut unmatched, ring)
            .expect_err("a request the door claimed nothing for carries no signed route");
        assert!(
            refused.contains("carries no signature"),
            "and it is the one sentence rather than a second one: {refused}"
        );

        dropped(ring);
    }
}
