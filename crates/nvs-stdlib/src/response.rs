//! `Core\Response` — the response a request is answering with, replacing
//! `header`, `http_response_code` and `setcookie`
//! ([ADR 0012](../../../docs/adr/0012-no-superglobals.md)).
//!
//! # What is here, and what is not
//!
//! Three of
//! [ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 4's five body members: `text`, `json` and `bytes`. The other two — `html`,
//! whose parameter is a carrier this class cannot take until `Core\Html\Markup`
//! is spellable in a registry row, and `sendFile`, whose path is § 1's sink
//! over a file the server resolves — and `addCookie` are known gaps of this
//! module rather than of
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) § 15.
//!
//! Beside them, § 15's `setStatus`, `setHeader` and `redirect`: the three
//! members here that shape a response without writing one.
//!
//! # Why five members and not one `write`
//!
//! § 4's whole argument, and the three that are here are what makes it visible:
//! each owns one body shape and **sets its own `Content-Type`**, so no call
//! site ever names a header and no endpoint can answer JSON labelled as HTML.
//! `json` serializes the value itself, which is why it takes `mixed` and a
//! tainted value inside it is safe; `bytes` is the one that cannot know, so its
//! content type is a parameter — and § 1 makes that parameter a **sink**.
//!
//! # What a written body is
//!
//! **The bytes are `echo`'s and the declaration is this class's.** § 3's table
//! already binds a request's `echo` to the response body, and
//! `nvs_runtime::Ctx` is where those bytes accumulate, so a body member writes
//! through [`Ctx::write_output`](nvs_runtime::Ctx::write_output) like every
//! other writer in the language. What it adds over an `echo` is one string —
//! the `Content-Type` § 4 gives it — and that is the whole of the new state:
//! `Ctx::declare_content_type` records it, the isolate's finish path carries
//! it out on `nvs_runtime::host::Completion`, and `nvs_server`'s `answer`
//! turns it into the header. A second buffer beside the output would have been
//! two answers to the question of what the body is.
//!
//! That channel is a `Completion` field rather than something read off a
//! `Ctx`, because a served request **is** an isolate (ADR 0097 § 4 step 5) and
//! the accept loop never holds its context — the completion is the one thing
//! that crosses.
//!
//! # A status crosses that same channel, and is not a body
//!
//! `setStatus` reuses all of it: `Ctx::declare_status` records one word, the
//! finish path takes it beside the media type, `Completion::status` carries it,
//! and `nvs_server`'s `answer` turns it into the status line. Two fields rather
//! than one struct on the context, because a media type is set by every body
//! member and a status by exactly one member, and a single declaration holding
//! both would let a body member reach a status it has no business setting.
//!
//! **A status is not a body, so § 4's sixth row does not reach it.** `E0801`
//! refuses an `echo` beside one of the five body members and `setStatus` is not
//! one of them — `nvs_types::response`'s roster is the five names — so a
//! handler that sets a status and then `echo`es is the ordinary spelling rather
//! than a mixture of two writers. Two body members would still be a question
//! that ADR is owed; a status beside either is not.
//!
//! **A request that failed answers `500` whatever it declared**, because
//! `nvs_server`'s `answer` reaches its failure path before it reads the field.
//! That direction is the fail-closed one: a handler that set `201` and then
//! threw has not created anything, and telling the peer otherwise is worse than
//! losing the declaration.
//!
//! # A header is a list on that same channel, and `Content-Type` is not one
//!
//! `setHeader` crosses the way the two words above do — `Ctx::declare_header`
//! records it, the finish path takes it, `Completion::headers` carries it and
//! `nvs_server`'s `answer` writes it — but what it records is a **list of
//! pairs**, because spec § 15 makes it an override of *one* policy-owned
//! header and a response has as many of those as a program names. The list
//! replaces rather than appends: the member is `setHeader`, a second value
//! under one name is `addCookie`'s question, and a replaced pair keeps the
//! position it was first set at.
//!
//! **It is applied after everything the server wrote for itself**, which is
//! the whole of what
//! [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 4
//! means by an override: the policy states what a response starts with, a
//! request may set any value for itself, and this member is the last writer.
//! The reverse order would leave it with no effect on exactly the headers it
//! exists to change.
//!
//! **`Content-Type` is refused here**, and that is this module's call rather
//! than something either ADR states. § 4's whole argument is that a body
//! member owns one shape *and* its media type, so that no call site names that
//! header; admitting it here would let a handler answer `json` and then
//! relabel it `text/html`, which is the mislabelling § 4 exists to make
//! unspellable. Nothing is lost, because `bytes` is the member that takes a
//! media type and it takes it beside the body it describes. The refusal is
//! ASCII-case-insensitive, a field name being.
//!
//! **Both parameters are sinks.** § 1's rule is that a parameter whose content
//! becomes an instruction is one, and a header line is two instructions: a
//! `tainted` name or value would let what arrived on the request choose what
//! the peer is told about the answer. [`nameable`] and [`carriable`] are the
//! runtime half of the same question, and they are narrower than RFC 9110 on
//! purpose — a field value may carry a tab and an `obs-text` byte, and neither
//! is something a program means, so this class spells a header the one way
//! [`spellable`] already spells a media type.
//!
//! # A redirect is a status and a header, and the status is a closed set
//!
//! `redirect` makes both declarations at once — the code onto the context and
//! `Location` into the header list above — because a response carrying one
//! without the other is not a redirect. A `303` with no destination sends a
//! peer somewhere unnamed, and a `Location` under a `200` is a header every
//! client ignores; one member is what makes the pair unforgettable, which is
//! the argument § 4 already makes for a body member owning its own media type.
//!
//! **The status is [`REDIRECT`]'s three cases and not any `3xx`.** `303`, `307`
//! and `308` are the codes whose meaning is stated without reference to what a
//! peer historically did — fetch the other resource with a `GET`, repeat this
//! request elsewhere for now, repeat it elsewhere for good. `301` and `302` are
//! the two RFC 9110 still lets a client rewrite a `POST` into a `GET` under, so
//! what a program means by one of them is not what every peer does with it, and
//! the rest of the class — `300`, `304`, `305` — names no destination this
//! member could write. Nothing is thereby unspellable: a program that means
//! `301` exactly writes `setStatus(301)` beside `setHeader("Location", …)`,
//! which is the general pair this member is the safe shorthand for.
//!
//! **The URL is § 1's sink**, and it is the sink on this class that carries the
//! most: a destination chosen by the request is an open redirect, which is a
//! peer trusting this origin about where it goes next. Beyond that the value is
//! checked exactly as a header value is — [`carriable`], and non-empty, an
//! empty `Location` naming nothing. What a URL may otherwise be is not asked
//! here, because the qualifier has already answered the question that made it
//! worth asking.
//!
//! # `text` takes `tainted`, and the mark is not the word § 4 uses
//!
//! § 4's table says the body is **contagious**, and
//! [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) is why
//! it may be: `X-Content-Type-Options: nosniff` is on with nothing configured,
//! so a `text/plain` body is not re-parsed as HTML. Removing that default is
//! visibly a change to two ADRs.
//!
//! The registry spelling of that row is nonetheless
//! [`Qual::Neutral`](crate::registry::Qual::Neutral), because the two
//! taxonomies are over different things. § 4's word is about the **body**: the
//! argument's bytes go out unchanged, so the member is not a sink. `Qual` is
//! about the **answer**, and `nvs_types`' `admits_tainted_argument` says a
//! [`Qual::Contagious`](crate::registry::Qual::Contagious) parameter admits
//! `tainted` only where the return type has somewhere to carry the bit — a
//! `void` member has nowhere, so that mark would *refuse* the very argument
//! § 4 admits. `Core\Cli::write`'s `string` arm is the same shape and the same
//! mark: a writer that answers nothing, and takes what it is given.
//!
//! # Known gaps
//!
//! 1. **The bytes are written verbatim, under every sink.** § 3's table says
//!    the sink in force selects a rendering, and today `echo`'s rendering is
//!    the terminal's everywhere; a request's HTML rendering and this member's
//!    "no rendering, the media type says so" are the same gap seen from two
//!    sides. So a program that calls this member outside a request — where it
//!    means nothing, and where the compile-time rule below cannot yet refuse
//!    it — puts its argument on the terminal unsubstituted. `setStatus` and
//!    `setHeader` off a request are the quiet half of the same gap: each
//!    declares onto a context nobody will ask, so the call means nothing and
//!    says nothing.
//! 2. **Nothing here adjudicates between two declarations**, and the last one
//!    wins by construction. § 4's sixth row is enforced by `E0801`, in
//!    `nvs_types::response`, whose module doc owns what that rule reaches and
//!    what it does not — including the two typed body members in one handler
//!    that it still admits.

use nvs_runtime::{Fault, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc,
    ParamDoc, Qual,
};

/// What `text` declares — § 4's `text/plain`, with the charset every other
/// text-shaped answer in this runtime carries.
///
/// A constant rather than a literal at the write, because the four members
/// still to land each own one of these and reading them in a column is how a
/// reviewer checks the table against § 4.
const TEXT_MEDIA_TYPE: &str = "text/plain; charset=utf-8";

/// What `json` declares — § 4's `application/json`, and no `charset`: the
/// media type's own registration fixes the encoding at UTF-8, so a parameter
/// saying so again is one more thing two members could disagree about.
const JSON_MEDIA_TYPE: &str = "application/json";

/// The lowest status `setStatus` admits — RFC 9110 § 15's first class, and the
/// floor rather than `0` because a code below it names no class at all.
const STATUS_MIN: u16 = 100;

/// The highest status `setStatus` admits.
///
/// `599` and not `999`, which is what the wire format and `hyper` both allow:
/// § 15 gives a status code a **class**, taken from its first digit, and the
/// five classes stop at `5`. A `6xx` is three digits a peer has no rule for, so
/// admitting it would be admitting a status whose only defined meaning is that
/// nothing downstream knows what it means — the same fail-closed direction
/// [`spellable`] takes for a media type, at the same layer.
const STATUS_MAX: u16 = 599;

/// The one header name `setHeader` refuses — ADR 0088 § 4's, owned by whichever
/// body member wrote the body. The module doc owns why it is refused rather
/// than admitted as one more override.
const CONTENT_TYPE_HEADER: &str = "Content-Type";

/// The header a redirect names its destination in, and the one header name
/// this class writes on a program's behalf rather than being told.
///
/// A constant beside [`CONTENT_TYPE_HEADER`] rather than a literal at the
/// declaration, so the two names this module knows about read in a column.
const LOCATION_HEADER: &str = "Location";

/// The fifteen non-alphanumeric bytes RFC 9110's `token` admits, which is what
/// a field name is made of.
const TOKEN_MARKS: &[u8] = b"!#$%&'*+-.^_`|~";

/// `Core\Response`'s registry rows — § 15's body members, in § 4's own table
/// order for the three that exist.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Response",
    methods: &[
        CoreMethod {
            name: "json",
            names: &["value"],
            params: &[CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_json",
            doc: Some(&JSON_DOC),
        },
        CoreMethod {
            name: "text",
            names: &["body"],
            // `Neutral`, not `Contagious` — the module doc owns why § 4's word
            // and this enum's case part company at a `void` member.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_text",
            doc: Some(&TEXT_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &["body", "contentType"],
            // The body takes what it is given for `text`'s reason; the content
            // type is § 1's sink, because it is the one string here that
            // becomes an *instruction* to the peer about how to read the rest.
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_bytes",
            doc: Some(&BYTES_DOC),
        },
        // § 4's body table ends above; § 15's other members follow it in that
        // section's own order, `setStatus` first. Two orders rather than one
        // because the two lists answer different questions — which shape a
        // body is, and what else a response says — and interleaving them
        // would leave a reader unable to check either against its source.
        CoreMethod {
            name: "setStatus",
            names: &["code"],
            // `Uint`, not `Int`: ADR 0007 § 4's type refuses a negative
            // literal at compile time, and there is no status code below 100
            // for a signed parameter to have been useful about. Not a `Sink`
            // either — the doc below owns why a number cannot be one.
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_set_status",
            doc: Some(&SET_STATUS_DOC),
        },
        CoreMethod {
            name: "setHeader",
            names: &["name", "value"],
            // Both are § 1 sinks, unlike `setStatus`' number: a header line is
            // an instruction to the peer at both ends, so a `tainted` name and
            // a `tainted` value are refused at compile time alike.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_set_header",
            doc: Some(&SET_HEADER_DOC),
        },
        CoreMethod {
            name: "redirect",
            names: &["url", "status"],
            // The URL is § 1's sink on `setHeader`'s reasoning and then some:
            // a `Location` is an instruction about where the peer goes next,
            // so a `tainted` one is an open redirect written by whoever sent
            // the request. The status is not classified because it cannot be
            // — a closed enum leaves a caller nothing to put there but one of
            // three cases, and a case carries no origin.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Enum(REDIRECT_NAME)],
            defaults: &[Const::EnumCase(REDIRECT_NAME, "SeeOther")],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_redirect",
            doc: Some(&REDIRECT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Response::json`'s reference card — ADR 0117.
const JSON_DOC: MethodDoc = MethodDoc {
    short: "Answers with `$value` serialized as JSON, declaring `application/json` — the same \
            encoder `Core\\Json::encode` uses, on one line.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to serialize, in every shape `Core\\Json::encode` accepts. A `tainted` \
               value anywhere inside it is safe: the framing belongs to the serializer, so a \
               tainted string becomes a JSON string and cannot escape it.",
        shape: &[],
    }],
    ret: "Nothing. Mixing this with `echo` on one response is a compile error.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$value` holds something JSON cannot spell: a `NaN` or infinite `float`, a value \
               of a type with no JSON encoding, an instance of a class without \
               `#[Json\\Derive]`, or nesting past 1024 levels.",
    }],
};

/// `Core\Response::bytes`'s reference card — ADR 0117.
const BYTES_DOC: MethodDoc = MethodDoc {
    short: "Answers with `$body` verbatim, declaring `$contentType` — the one body member that \
            cannot know the media type, so it is told.",
    params: &[
        ParamDoc {
            name: "body",
            desc: "The octets to send, unchanged.",
            shape: &[],
        },
        ParamDoc {
            name: "contentType",
            desc: "The media type to declare. A sink: it becomes a header the peer obeys, so a \
                   `tainted` value is refused at compile time and one holding anything a header \
                   cannot carry is refused here.",
            shape: &[],
        },
    ],
    ret: "Nothing. Mixing this with `echo` on one response is a compile error.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$contentType` is empty or holds a byte outside a header field value — a control \
               character, a newline, or anything above ASCII.",
    }],
};

/// `Core\Response::text`'s reference card — ADR 0117.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "Answers with `$body` as the response body, declaring `text/plain; charset=utf-8` — one \
            of the five body members that replace a single `write`, each owning one shape.",
    params: &[ParamDoc {
        name: "body",
        desc: "The text to send. A `tainted` value is accepted: `nosniff` is on by default, so a \
               `text/plain` body is never re-parsed as HTML.",
        shape: &[],
    }],
    ret: "Nothing. Mixing this with `echo` on one response is a compile error.",
    errors: &[],
};

/// `Core\Response::setStatus`'s reference card — ADR 0117.
const SET_STATUS_DOC: MethodDoc = MethodDoc {
    short: "Answers with `$code` as the response's status, replacing \
            `http_response_code` — the one member here that says nothing about the body.",
    params: &[ParamDoc {
        name: "code",
        desc: "The status to answer with, from 100 to 599. Not a sink, unlike `bytes`' content \
               type: a status line carries a number and never a string, so there is nothing here \
               a `tainted` value could become.",
        shape: &[],
    }],
    ret: "Nothing. The last call on one response is the one that answers, and a request that \
          failed answers `500` whatever it had set.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$code` is outside 100 to 599, which is not a status any peer can classify.",
    }],
};

/// `Core\Response::setHeader`'s reference card — ADR 0117.
const SET_HEADER_DOC: MethodDoc = MethodDoc {
    short: "Sets `$name` to `$value` on this response, replacing whatever the server's own \
            policy wrote for that header — spec § 15's override, replacing `header`.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The field name: a non-empty token, so letters, digits and the marks \
                   RFC 9110 admits. A sink, and `Content-Type` is refused whatever its case — \
                   the body member that wrote the body is what declares that one.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The field value: printable ASCII, so a newline cannot smuggle a second \
                   header and a control character cannot end the line early. A sink; empty is \
                   admitted, an empty header being a header.",
            shape: &[],
        },
    ],
    ret: "Nothing. Setting one name twice keeps the last value, at the first call's position, \
          and a request that failed answers `500` carrying none of them.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$name` is empty, holds a byte a token cannot, or is `Content-Type`; or \
               `$value` holds a byte outside printable ASCII.",
    }],
};

/// `Core\Response::redirect`'s reference card — ADR 0117.
const REDIRECT_DOC: MethodDoc = MethodDoc {
    short: "Answers by sending the peer to `$url`, declaring the redirect status and the \
            `Location` header together — spec § 15's redirect, replacing a `Location` written \
            by hand beside `http_response_code`.",
    params: &[
        ParamDoc {
            name: "url",
            desc: "Where the peer is being sent: printable ASCII and non-empty, absolute or \
                   relative to the request. A sink, because a destination chosen by whoever \
                   sent the request is an open redirect.",
            shape: &[],
        },
        ParamDoc {
            name: "status",
            desc: "Which redirect this is. Defaults to `SeeOther`, the one that answers a form \
                   post by sending the browser to fetch a page.",
            shape: &[],
        },
    ],
    ret: "Nothing, and no byte of body. The last call on one response is the one that answers, \
          and a request that failed answers `500` carrying neither the status nor the header.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$url` is empty, or holds a byte outside printable ASCII — a newline included, \
               which would end the header line and begin one the program never wrote.",
    }],
};

/// `Core\Response\Redirect`'s fully-qualified name, written once — [`REDIRECT`]
/// declares it and the [`CoreTy::Enum`] naming it resolves against
/// [`crate::registry::ENUMS`], so the two cannot drift apart.
pub(crate) const REDIRECT_NAME: &str = r"Core\Response\Redirect";

/// The closed set of statuses [`nvs_core_response_redirect`] will declare —
/// the module doc owns which three and why the other five `3xx` codes are not
/// among them.
///
/// **The ordinal is the status code itself**, unlike every other `Core` enum
/// here, whose cases are numbered from zero. The wire has already assigned
/// these three names a number apiece, and a second numbering beside it would be
/// a table two files would have to agree about; [`redirect_status`] is still
/// the conversion, so the coincidence is stated once rather than cast.
pub(crate) const REDIRECT: CoreEnum = CoreEnum {
    name: REDIRECT_NAME,
    cases: &[("SeeOther", 303), ("Temporary", 307), ("Permanent", 308)],
    doc: Some(&REDIRECT_CASES_DOC),
};

/// [`REDIRECT`]'s reference card — ADR 0117.
///
/// Named for its cases rather than `REDIRECT_DOC`, which the member above it
/// already is: the class and the enum share the one word the spec gives them.
const REDIRECT_CASES_DOC: EnumDoc = EnumDoc {
    short: "Which redirect a response is. The three cases are the redirect statuses whose \
            meaning is defined without reference to what browsers historically did with them, \
            so what a program writes is what every peer performs.",
    cases: &[
        CaseDoc {
            name: "SeeOther",
            desc: "`303` — the other resource is fetched with a `GET`, whatever method asked. \
                   The answer to a form post, and the default.",
        },
        CaseDoc {
            name: "Temporary",
            desc: "`307` — repeat this request, method and body intact, at the new address \
                   this time only. Nothing is cached and nothing is renamed.",
        },
        CaseDoc {
            name: "Permanent",
            desc: "`308` — repeat this request, method and body intact, and the new address is \
                   the one from now on. Caches and crawlers are entitled to remember it.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_response_json" => (nvs_core_response_json as *const ()).cast(),
        "nvs_core_response_text" => (nvs_core_response_text as *const ()).cast(),
        "nvs_core_response_bytes" => (nvs_core_response_bytes as *const ()).cast(),
        "nvs_core_response_set_status" => (nvs_core_response_set_status as *const ()).cast(),
        "nvs_core_response_set_header" => (nvs_core_response_set_header as *const ()).cast(),
        "nvs_core_response_redirect" => (nvs_core_response_redirect as *const ()).cast(),
        _ => return None,
    })
}

/// Whether `media_type` is something a `Content-Type` header can carry — ADR
/// 0088 § 1's sink, checked at the member rather than only at the connection.
///
/// RFC 9110's field-value rule, narrowed: a media type is `token/token` with
/// optional parameters, all of it printable ASCII, so a byte outside
/// `0x20..=0x7e` is refused whatever it is. Refusing *here* rather than
/// leaving it to the server is the difference between a program learning that
/// its header was wrong and a peer silently receiving
/// `application/octet-stream` — `nvs_server::serve`'s own fallback, which
/// stays as the layer below this one rather than as the only one.
fn spellable(media_type: &str) -> bool {
    !media_type.is_empty() && carriable(media_type)
}

/// Whether `value` is something a header field value can carry — the byte rule
/// [`spellable`] applies to a media type, without its non-empty half.
///
/// That is the whole difference between the two, and it is the right one: an
/// empty media type is not a media type, while an empty header value is a
/// header a program may well mean.
fn carriable(value: &str) -> bool {
    value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

/// One [`REDIRECT`] case as the status code it stands for.
///
/// Written out rather than cast off the ordinal, on the reasoning
/// [`crate::cli`]'s own enum conversions give: the two numbers coincide today
/// by [`REDIRECT`]'s design, and a cast would put a fourth case straight on the
/// wire the day one is declared, where this refuses it until the arm beside it
/// is written.
fn redirect_status(status: &Value) -> Result<u16, Fault> {
    match status.as_int() {
        Some(303) => Ok(303),
        Some(307) => Ok(307),
        Some(308) => Ok(308),
        // Unreachable from source: the row's second parameter is
        // `CoreTy::Enum(REDIRECT_NAME)`, so `E0401` refuses anything that is
        // not one of the three cases before this runs, and compiled code
        // writes the case's own constant rather than a number a program chose.
        _ => Err(Fault::fatal(format!(
            "Core\\Response::redirect expected a `{REDIRECT_NAME}` case, got tag {} value {:?}",
            status.tag_byte(),
            status.as_int()
        ))),
    }
}

/// Whether `name` is an RFC 9110 field name — a non-empty `token`.
///
/// The set is that grammar's own: letters, digits and [`TOKEN_MARKS`]. Refusing
/// here is [`spellable`]'s direction at [`spellable`]'s layer, and it is what
/// makes `nvs_server`'s own conversion of one of these unreachable from source.
fn nameable(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || TOKEN_MARKS.contains(&byte))
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::text(string $body): void` — ADR 0088 § 4's third row.
    ///
    /// Two effects and no third: the bytes go to this request's output, and
    /// the media type is declared on its context. The module doc owns why
    /// those are one write and one word rather than a response object.
    ///
    /// The declaration is made **before** the write, so a body that trips
    /// `[limits] max_output` has still said what it was: the ceiling is
    /// noticed at the safepoint poll rather than refused here
    /// (`Ctx::write_output`), and a half-written body whose type is unknown
    /// would be strictly worse to answer with than one whose type is not.
    fn nvs_core_response_text(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is a `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs.
        let body = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::text expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        ctx.declare_content_type(TEXT_MEDIA_TYPE);
        // Verbatim, and gap 1 in the module doc owns what that means off a
        // request. Unreachable from source on `Core\Cli::write`'s reasoning:
        // `OutputSink::Buffer` and `Sink` never fail, and nothing in the
        // language closes a descriptor the host handed the process.
        ctx.write_output(body.as_bytes())
            .map_err(|error| Fault::fatal(format!("Core\\Response::text could not write: {error}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::setStatus(uint $code): void` — spec § 15's status,
    /// replacing `http_response_code`.
    ///
    /// One effect and no second: the code is declared on this request's
    /// context, and nothing is written. That is the whole difference between
    /// this member and the four above it, and it is why ADR 0088 § 4's sixth
    /// row does not reach here — `nvs_types::response`'s roster of five body
    /// members is what `E0801` refuses beside an `echo`, and a status is not a
    /// body. A handler that `echo`es and sets a status is ordinary.
    ///
    /// The code is checked before it is declared, on `bytes`' reasoning: this
    /// member has nothing written to be too late for, but a status the peer
    /// cannot classify is the same kind of instruction a media type it cannot
    /// read is, and the program learns which of the two layers refused it from
    /// which of the two answers it gets.
    fn nvs_core_response_set_status(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is a `CoreTy::Uint`, so
        // `E0401` refuses anything that is not one — a negative literal
        // included — before this runs.
        let code = args[0].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::setStatus expected a `uint`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        // `try_from` and not an `as`: everything this member admits fits a
        // `u16`, and a truncating cast would turn `65636` into `100`.
        let Some(declared) = u16::try_from(code)
            .ok()
            .filter(|code| (STATUS_MIN..=STATUS_MAX).contains(code))
        else {
            // A literal stem before the first hole, which is
            // `conformance_coverage`'s error-path gate matching a site.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::setStatus(): `{code}` is not an HTTP status — a status \
                     is three digits naming one of five classes, so it is between \
                     {STATUS_MIN} and {STATUS_MAX}"
                ),
            ));
        };
        ctx.declare_status(declared);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::setHeader(string $name, string $value): void` — spec
    /// § 15's header, replacing `header`.
    ///
    /// The module doc owns the three decisions this member is: the list rather
    /// than a word, the refusal of `Content-Type`, and why both parameters are
    /// sinks. What is here is the order — every check runs before anything is
    /// declared, on `bytes`' reasoning — and the two checks themselves, which
    /// are [`nameable`] and [`carriable`].
    fn nvs_core_response_set_header(ctx, args: [2]) {
        // Unreachable from source for both: the row's parameters are
        // `CoreTy::Text`, so `E0401` refuses anything that is not a `string`,
        // and refuses a `tainted` one besides, both being § 1's sink.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::setHeader expected a `string` for the name, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        // Unreachable from source for the second parameter on the same
        // reasoning: `E0401` refuses it before this runs.
        let value = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::setHeader expected a `string` for the value, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        if !nameable(name) {
            // A literal stem before the first hole, which is
            // `conformance_coverage`'s error-path gate matching a site. The two
            // refusals below share it, which that gate reads as one site.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::setHeader(): `{name}` is not a header name — a field \
                     name is a non-empty token, so it carries letters, digits and the \
                     marks RFC 9110 admits and nothing else"
                ),
            ));
        }
        if name.eq_ignore_ascii_case(CONTENT_TYPE_HEADER) {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::setHeader(): `{name}` is declared by the body member \
                     that wrote the body — answer with `Core\\Response::bytes($body, \
                     $contentType)` to say what a body is"
                ),
            ));
        }
        if !carriable(value) {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::setHeader(): `{value}` is not a value a header line can \
                     carry — a field value is printable ASCII, so a control character or a \
                     newline that would smuggle a second header is refused"
                ),
            ));
        }
        ctx.declare_header(name, value);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::redirect(string $url, Core\Response\Redirect $status): void`
    /// — spec § 15's redirect, replacing a `Location` written through `header`
    /// beside an `http_response_code`.
    ///
    /// Two declarations and no write, which is the whole of what makes this
    /// one member rather than shorthand for either half. The module doc owns
    /// why the status is a closed set and why the URL is a sink; what is here
    /// is the order — both checks run before either declaration, on `bytes`'
    /// reasoning — and that the header goes in through the same
    /// [`Ctx::declare_header`](nvs_runtime::Ctx::declare_header) `setHeader`
    /// uses, so a program that names `Location` itself afterwards overrides
    /// this one exactly as it overrides the server's own.
    fn nvs_core_response_redirect(ctx, args: [2]) {
        // Unreachable from source: the row's first parameter is a
        // `CoreTy::Text`, so `E0401` refuses anything that is not a `string`,
        // and refuses a `tainted` one besides, that being § 1's sink.
        let url = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::redirect expected a `string` for the URL, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let status = redirect_status(&args[1])?;
        if url.is_empty() || !carriable(url) {
            // A literal stem before the first hole, which is
            // `conformance_coverage`'s error-path gate matching a site.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::redirect(): `{url}` is not a destination a `Location` \
                     header can carry — a field value is printable ASCII and never empty, so \
                     an empty URL and a newline that would begin a second header are refused \
                     alike"
                ),
            ));
        }
        ctx.declare_status(status);
        ctx.declare_header(LOCATION_HEADER, url);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::json(mixed $value): void` — ADR 0088 § 4's second row.
    ///
    /// The same encoder `Core\Json::encode` is, reached through the same
    /// [`crate::json::Encodable`] rather than through a second walk: one
    /// serializer is what makes § 4's claim about tainted values true, since a
    /// second one could frame a string differently and there would be nothing
    /// to compare it against. `pretty` is not an option here — a response body
    /// is read by a program, and whitespace for a human belongs to whatever is
    /// showing it to one.
    ///
    /// The declaration is made only once the value has serialized, unlike
    /// `text`'s: an unencodable value throws with nothing written, so there is
    /// no body for a `Content-Type` to have described.
    fn nvs_core_response_json(ctx, args: [1]) {
        let subject = crate::json::Encodable::document(args[0]);
        let written = serde_json::to_string(&subject).map_err(|why| {
            Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!("Core\\Response::json(): {why}"),
            )
        })?;
        ctx.declare_content_type(JSON_MEDIA_TYPE);
        // Unreachable from source, on `text`'s reasoning: `OutputSink::Buffer`
        // and `Sink` never fail, which `Ctx::write_output`'s own `# Errors`
        // states, and nothing in the language closes a descriptor the host
        // handed the process.
        ctx.write_output(written.as_bytes())
            .map_err(|error| Fault::fatal(format!("Core\\Response::json could not write: {error}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::bytes(bytes $body, string $contentType): void` — ADR
    /// 0088 § 4's fourth row, and the only body member that is *told* its
    /// media type.
    ///
    /// The type is checked before anything is written, which is the opposite
    /// order from `text`'s and for the same reason `json`'s is: a refusal that
    /// has already put the body on the wire is not a refusal. [`spellable`]
    /// owns what the check is and why it is here and not only at the
    /// connection.
    fn nvs_core_response_bytes(ctx, args: [2]) {
        // Unreachable from source for both: the row's parameters are a
        // `CoreTy::Blob` and a `CoreTy::Text`, so `E0401` refuses anything else
        // before this runs.
        let body = args[0].as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::bytes expected `bytes` for the body, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        // Unreachable from source for the same reason: the second parameter is
        // a `CoreTy::Text`, so `E0401` refuses anything that is not a `string`
        // — and refuses a `tainted` one besides, that being § 1's sink.
        let media_type = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::bytes expected a `string` for the content type, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        if !spellable(media_type) {
            // A literal stem before the first hole, which is
            // `conformance_coverage`'s error-path gate matching a site.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::bytes(): `{media_type}` is not a media type a \
                     `Content-Type` header can carry — a field value is printable ASCII \
                     and never empty"
                ),
            ));
        }
        ctx.declare_content_type(media_type);
        // Unreachable from source, on `text`'s reasoning: `OutputSink::Buffer`
        // and `Sink` never fail, which `Ctx::write_output`'s own `# Errors`
        // states, and nothing in the language closes a descriptor the host
        // handed the process.
        ctx.write_output(body)
            .map_err(|error| Fault::fatal(format!("Core\\Response::bytes could not write: {error}")))?;
        Ok(Value::null())
    }
}
