//! `Core\Http\Method` — the closed set of verbs a route is declared under —
//! `Core\Router`'s link half, which is as much of
//! [ADR 0077](../../../../docs/adr/0077-compile-time-routing.md)'s router as
//! exists today, and `Core\Router\Match`, the match
//! [ADR 0102](../../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
//! § 1 has the door take once and `Core\Request::route()` hand back.
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
//! different domain ([ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md)
//! owns it). When it lands it names *this* enum: a namespace prefix is not a
//! module boundary, and a second `Core\Http\Method` would be two rosters
//! disagreeing about what `Post` is worth.
//!
//! # What the cases are, and what the order buys
//!
//! [ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 7
//! names eight verbs and [ADR 0096](../../../../docs/adr/0096-a-route-without-a-declared-access-decision-does-not-compile.md)
//! § 4 names four of them as the ones CSRF enforcement covers. Those four are
//! this enum's contiguous *tail*, the same arrangement — and for the same
//! reason — as `crate::hash`'s private `STRONG`: a rule over a set of cases becomes a
//! bound rather than a match arm nobody remembers to extend.
//!
//! `CONNECT` is deliberately not a case. It establishes a proxy tunnel, so it
//! is neither something a `#[Route]` may be declared under nor something
//! [ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md)'s pinned
//! client sends; a case a program can write and pass nowhere is surface with no
//! meaning behind it, which is [`crate::registry::ENUMS`]' own test for
//! admitting an entry.
//!
//! # Known gaps
//!
//! 1. **A name this module is handed is one the compiler could not fold.** The
//!    route table is built and § 4's link is resolved against it while
//!    compiling: a literal name reaches [`link`]'s two symbols carrying a
//!    prepared path, an unknown literal one is `E0754` before the program runs,
//!    and [`CLASS`]'s own two members are what is left over — a *computed*
//!    name, which § 4 says throws, and `nvs_types::links`' gap 1's named
//!    argument, which is folded as a computed name would be and throws for a
//!    reason a reader has to look up. Closing that gap is what would make
//!    [`no_such_route`] answer only the case § 4 named.
//! 2. **The mount prefix is the half of the laundering that has nowhere to come
//!    from.** [`substitute`] percent-encodes every value it puts in a segment,
//!    which is § 4's launder and is real; what is not is
//!    ([ADR 0097](../../../../docs/adr/0097-development-server-and-proxied-origin.md)
//!    § 3)'s prefix in front of it, because a program run off the command line
//!    is mounted nowhere. `urlAbsolute` is in the same position for the same
//!    reason and says so where a program can see it: it reads
//!    [`Ctx::origin`](nvs_runtime::Ctx::origin) and throws when a unit has
//!    resolved none, rather than answering an empty authority.
//! 3. **`match` is absent.** § 4's last member answers a *request* against a
//!    second, program-chosen path, and `docs/agent/loop-goal.md` § *Standing
//!    decisions* keeps it out of scope on purpose. The type it would answer
//!    with is here — [`MATCH`], reached through `Core\Request::route()` — and
//!    the table it would walk is now reached, by [`nvs_core_router_methods_for`]
//!    through [`Ctx::routes`](nvs_runtime::Ctx::routes), so what it still owes
//!    is the member and nothing under it.
//! 4. **The parse of a verb into one of these cases is [`crate::request`]'s**,
//!    not this module's — `Core\Request::method` is the one reader that turns a
//!    method token into a case, and its module doc owns the two decisions in
//!    it: `HEAD` answering `Get`, and a token outside this roster being refused
//!    rather than mapped. What is still open is the half above that: a served
//!    request carrying an unrecognized verb should be answered **501 at the
//!    door**, before an isolate exists, and that belongs to `nvs_server`. Until
//!    it lands, the refusal is a throw inside the program rather than a status
//!    outside it.

use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};
use crate::uri::{Form, encode};

/// `Core\Http\Method`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const METHOD_NAME: &str = r"Core\Http\Method";

/// ADR 0077 § 1's `Core\Http\Method` — the eight verbs
/// [ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 7
/// names, safe ones first so that ADR 0096 § 4's CSRF set is the contiguous
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

/// [`METHOD`]'s reference card — ADR 0117.
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

/// ADR 0096 § 1a's `Core\Audience` — the one access decision `Core` names.
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

/// [`AUDIENCE`]'s reference card — ADR 0117.
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

/// `$params` — ADR 0077 § 4 writes it `array<string, mixed>`, and that is a
/// spelling the type system has no form for: `nvs_types::ty::Ty::Array` carries
/// one element type, because a Novis array's keys are `int|string` by
/// construction and are not part of its type. So the row declares the half that
/// *is* representable, `array<mixed>`, and the key rule is enforced where it
/// can be — § 4 makes a literal key that names neither a capture nor a declared
/// `#[Query]` parameter a compile error, which is a question about this route's
/// own captures rather than about a type.
const PARAMS: CoreTy = CoreTy::Array(&CoreTy::Mixed);

/// `array<Core\Http\Method>` — [ADR 0102](../../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
/// § 2's answer, as the enum this module already owns rather than as text.
///
/// A list of cases, so that the `Allow:` header a caller writes out of it is
/// spelled by [`METHOD`]'s roster in one place — a member answering
/// `array<string>` would be a second spelling of the eight verbs, free to
/// disagree with the first about what `Delete` looks like.
const METHODS: CoreTy = CoreTy::Array(&CoreTy::Enum(METHOD_NAME));

/// Spec § 15's `Core\Router`, as much of it as ADR 0077 § 4's link half and
/// ADR 0102 § 2's other answer need.
///
/// `match` is deliberately absent — it answers *a request*, which belongs with
/// the server, and `docs/agent/loop-goal.md` § *Standing decisions* keeps it
/// out of scope. `methodsFor` is here rather than beside it because the
/// question it asks is not a request's: it is asked *of the table*, about a
/// path, once § 1's match has already answered `null`, and every input it takes
/// is the caller's.
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

/// `Core\Router::url`'s reference card — ADR 0117.
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

/// `Core\Router::urlAbsolute`'s reference card — ADR 0117.
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

/// `Core\Router::methodsFor`'s reference card — ADR 0117.
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
          claimed by nobody.",
    errors: &[],
};

/// `Core\Router\Match`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const MATCH_NAME: &str = r"Core\Router\Match";

/// [`MATCH`]'s slots, in the order [`match_value`] fills them.
const MATCH_ROUTE_NAME: usize = 0;
const MATCH_PARAMS: usize = 1;

/// ADR 0102 § 5's capture as a program reaches it, and the one place the three
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
const CAPTURE: &CoreTy = &CoreTy::Union(&[CoreTy::TaintedStr, CoreTy::Int, CoreTy::Uint]);

/// [ADR 0102](../../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
/// § 1's match, as the program answering the request reads it.
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
/// [ADR 0077](../../../../docs/adr/0077-compile-time-routing.md) § 4 refuses to
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

/// `Core\Router\Match::name`'s reference card — ADR 0117.
const MATCH_NAME_DOC: MethodDoc = MethodDoc {
    short: "The declared name of the route this request matched, as its `#[Route(name: …)]` wrote \
            it — the same string `Core\\Router::url` resolves and the `route` metric label \
            carries.",
    params: &[],
    ret: "The name, or `null` where the matched route declares none. Not `tainted`: it is the \
          unit's own literal and not anything the request carried.",
    errors: &[],
};

/// `Core\Router\Match::params`'s reference card — ADR 0117.
const MATCH_PARAMS_DOC: MethodDoc = MethodDoc {
    short: "Every capture the matched path filled, keyed by the parameter name it binds, in path \
            order.",
    params: &[],
    ret: "An array of the captures. A `{name}` declared `string` answers `tainted string` and is \
          still percent-encoded; one declared `int` or `uint` answers the number the match \
          already converted. A route with no captures answers an empty array.",
    errors: &[],
};

/// `Core\Router\Match::param`'s reference card — ADR 0117.
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

/// The two symbols ADR 0077 § 4's **folded** link is lowered to, and the wire
/// format they read argument 0 as.
///
/// Neither is a [`CoreMethod`] row, and that is the point: a program calls
/// `Core\Router::url`, and `nvs_ir::lower` redirects the call here whenever
/// `nvs_types::links` resolved its literal name against the compile-time table.
/// So the *member* is one, and which of the two implementations answers is a
/// property of what the compiler could prove — the same arrangement ADR 0057
/// § 4 states for every prepared literal: one implementation, reached at two
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
    /// `nvs_core_router_link_absolute` — the same with ADR 0102 § 6's
    /// configured origin in front.
    pub const ABSOLUTE_SYMBOL: &str = "nvs_core_router_link_absolute";
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

    /// Both symbols, for [`crate::symbols`], which builds the JIT's roster out
    /// of the member rows and so would never reach an implementation no
    /// [`super::CoreMethod`] names — the same reason
    /// [`crate::registry::CONSTRUCTORS`] is chained there.
    pub const SYMBOLS: [&str; 2] = [SYMBOL, ABSOLUTE_SYMBOL];
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_router_url" => (nvs_core_router_url as *const ()).cast(),
        "nvs_core_router_url_absolute" => (nvs_core_router_url_absolute as *const ()).cast(),
        "nvs_core_router_link" => (nvs_core_router_link as *const ()).cast(),
        "nvs_core_router_link_absolute" => (nvs_core_router_link_absolute as *const ()).cast(),
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
    // builds a `Value::str`, ADR 0088 § 5's carrier arm included, so this is a
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
/// ADR 0102 § 6's other half: the prepared pieces name every capture, so a
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
/// **A value outside ADR 0102 § 5's closed set is substituted here, and throws
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
    /// The mount prefix ADR 0097 § 3 has this member prepend is empty here and
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
    /// [`nvs_core_router_link`] with ADR 0102 § 6's configured origin in front.
    ///
    /// The origin is per mount, falling back to `[app] origin`, and is never
    /// derived from `Host` or `X-Forwarded-Host` — so this member reads what
    /// was resolved *before* the request ran, out of
    /// [`Ctx::origin`](nvs_runtime::Ctx::origin), and never a value the
    /// request could have influenced. A unit that resolves none throws here
    /// rather than answering with an empty authority in it, which is ADR 0097
    /// § 3's rule at the one place a CLI run can enforce it: § 6 puts the
    /// *boot* error at mount expansion, and there is no mount off the command
    /// line to expand.
    fn nvs_core_router_link_absolute(ctx, args: [2]) {
        let template = link_template(args, "urlAbsolute")?;
        let path = substitute(template, &args[1], "urlAbsolute")?;
        let Some(origin) = ctx.origin() else {
            return Err(Fault::thrown(format!(
                "Core\\Router::urlAbsolute(): no origin is configured for this unit, so `{path}` \
                 has no absolute form. ADR 0102 § 6 refuses to derive one from a request header, \
                 so give `nvs.toml` an `[[app]] origin`"
            )));
        };
        produced(&format!("{origin}{path}"))
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
/// members — ADR 0077 § 4 says a *computed* `$name` throws — so the throw is a
/// real answer a program can catch, and it stays the answer for an unknown name
/// after the table lands. What changes then is only which names are unknown.
fn no_such_route(member: &str, args: &[nvs_runtime::Value]) -> Fault {
    let name = args[0].as_text().unwrap_or("<not a string>");
    Fault::thrown(format!(
        "Core\\Router::{member}(): no route is named `{name}`. The compile-time route table is \
         not built yet (ADR 0077 § 5), so no name resolves"
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::url(string $name, array<mixed> $params): string` — ADR 0077
    /// § 4's launderer for the URL-path sink.
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
    /// [`nvs_core_router_url`] with ADR 0102 § 6's configured origin in front.
    ///
    /// The origin is per mount, falling back to `[app] origin`, and is never
    /// derived from `Host` or `X-Forwarded-Host`. Nothing here resolves one:
    /// the route lookup refuses first, and there is no mount to read an origin
    /// from in a program run off the command line at all.
    fn nvs_core_router_url_absolute(_ctx, args: [2]) {
        Err(no_such_route("urlAbsolute", args))
    }
}

// ---------------------------------------------------------------- ADR 0102 § 1's match

/// The [`MATCH`] one request carries, built out of the match the door already
/// took — the whole of how [`nvs_runtime::routes`]' row reaches a program.
///
/// `pub(crate)` because the reader is [`crate::request`]'s `route()`: § 1 puts
/// the match on the request rather than on the router, and the class it answers
/// with is this module's because `Core\Router\Match` is a `Core\Router` name.
pub(crate) fn match_value(matched: &nvs_runtime::routes::Match) -> Value {
    let mut params = NvsArray::new();
    for (name, capture) in matched.params() {
        params.set(NvsStr::new(name.as_bytes()), capture_value(capture));
    }
    crate::instance::build(
        &MATCH,
        [
            match matched.name() {
                Some(name) => Value::str(NvsStr::new(name.as_bytes())),
                None => Value::null(),
            },
            Value::array(params),
        ],
    )
}

/// Which case of [`METHOD`] a table row's declared verb is, by its ordinal.
///
/// The comparison is ASCII-case-insensitive for the reason
/// [`nvs_runtime::routes::Routes::match_request`]'s is: a row's verb is the
/// case's own name (`Get`) in a table the compiler built, and a table built by
/// hand may spell it as the wire token (`GET`). `None` is a verb this roster
/// does not name, which no compiled table can hold — `#[Route(method: …)]`
/// takes a case of this enum and nothing else.
fn method_case(verb: &str) -> Option<i64> {
    METHOD
        .cases
        .iter()
        .find(|(case, _)| case.eq_ignore_ascii_case(verb))
        .map(|(_, ordinal)| *ordinal)
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::methodsFor(tainted string $path): array<Core\Http\Method>`
    /// — ADR 0102 § 2's second answer, over the table
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
    /// ADR 0077 § 5's table is opt-in, and "no route claims this path" is
    /// exactly true of a program that declares none. That is the same reading
    /// [`Ctx::route`](nvs_runtime::Ctx::route) takes of the absent table.
    ///
    /// **Each verb appears once**, because `methods_for` answers verbs rather
    /// than rows, and an enum case is its ordinal on the way out (ADR 0010) —
    /// the same crossing `Core\Request::method` makes in the other direction.
    fn nvs_core_router_methods_for(ctx, args: [1]) {
        // Unreachable from source, as every mistyped argument slot is: the
        // row's parameter is `CoreTy::Text(Qual::Neutral)`, so `E0401` refuses
        // anything but a `string` before this body runs. The tag is also the
        // whole UTF-8 guarantee (ADR 0009 § 3), so there is nothing left to
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
/// [`nvs_runtime::routes::Param`]'s four forms become Novis values.
fn capture_value(capture: &nvs_runtime::routes::Param) -> Value {
    match capture {
        nvs_runtime::routes::Param::Text(text) => Value::str(NvsStr::new(text.as_bytes())),
        nvs_runtime::routes::Param::Int(number) => Value::int(*number),
        nvs_runtime::routes::Param::Uint(number) => Value::uint(*number),
        nvs_runtime::routes::Param::Decimal(value) => Value::decimal(*value),
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
    /// `Core\Router\Match::name(): ?string` — ADR 0102 § 1's declared name,
    /// which ADR 0076 § 1's `route` label reads and `Core\Router::url` resolves.
    fn nvs_core_router_match_name(_ctx, args: [1]) {
        match_slot(args, MATCH_ROUTE_NAME, "name")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router\Match::params(): array<tainted string|int|uint>` — every
    /// capture the path filled, keyed by the parameter it binds.
    ///
    /// The array is built once by [`match_value`] and read back here, so the
    /// two members answer the same values rather than two walks of one row.
    fn nvs_core_router_match_params(_ctx, args: [1]) {
        match_slot(args, MATCH_PARAMS, "params")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router\Match::param(string $name): ?(tainted string|int|uint)` —
    /// [`nvs_core_router_match_params`] read at one key.
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
    use super::METHOD;

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
    /// position ADR 0074 § 7's roster reads at.
    #[test]
    fn every_case_carries_a_distinct_value_in_declaration_order() {
        for (index, (_, value)) in METHOD.cases.iter().enumerate() {
            assert_eq!(*value, i64::try_from(index).unwrap());
        }
    }
}
