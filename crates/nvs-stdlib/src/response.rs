//! `Core\Response` — the response a request is answering with, replacing
//! `header`, `http_response_code` and `setcookie`
//! (`rule:statements/no-host-populated-variables`).
//!
//! # What is here, and what is not
//!
//! `rule:security/response-body-is-one-typed-member`'s body members: `html`,
//! `text`, `json`, `bytes` and `sendFile`, plus `stream`, which that rule's
//! table does not have a row for because a body written over time is
//! `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
//! subject rather than that one's. `sendFile` is the one that hands over a
//! **name**: its path is a `rule:security/sink-predicate` sink, the file is
//! opened by whoever answers the request, and what the bytes are called is the static-file policy's table
//! rather than anything this module declares.
//!
//! Beside them, `setStatus`, `setHeader`, `redirect` and `addCookie`,
//! and `rule:core-classes/html-later`'s `slotted`: the five members here that
//! shape a response without writing one.
//!
//! # Why `addCookie` appends where `setHeader` overrides
//!
//! The two write to the same header list and are deliberately not the same
//! operation, which is the one thing about this module worth knowing before
//! reading it. `setHeader` overrides a policy-owned header, so it *replaces*
//! every value already under that name; `addCookie` *appends*, because a
//! response carries as many cookies as it was told to and a scan would collapse
//! two into the last one. `nvs_runtime::Ctx`'s two members own that split —
//! `declare_header` and `append_header`, whose doc comments are the rule — and
//! this class is where the two meanings meet a program.
//!
//! # Why five members and not one `write`
//!
//! `rule:security/response-body-is-one-typed-member` is the whole argument, and
//! the three that are here are what makes it visible: each owns one body shape and **sets its own `Content-Type`**, so no call
//! site ever names a header and no endpoint can answer JSON labelled as HTML.
//! `json` serializes the value itself, which is why it takes `mixed` and a
//! tainted value inside it is safe; `bytes` is the one that cannot know, so its
//! content type is a parameter — and `rule:security/sink-predicate` makes that
//! parameter a **sink**.
//!
//! # What a written body is
//!
//! **The bytes are `echo`'s and the declaration is this class's.**
//! `rule:tooling/echo-always-has-a-sink`'s table already binds a request's `echo` to the response body, and
//! `nvs_runtime::Ctx` is where those bytes accumulate, so a body member writes
//! through [`Ctx::write_output`](nvs_runtime::Ctx::write_output) like every
//! other writer in the language. What it adds over an `echo` is one string —
//! the `Content-Type` `rule:security/response-body-is-one-typed-member` gives
//! it — and that is the whole of the new state:
//! `Ctx::declare_content_type` records it, the isolate's finish path carries
//! it out on `nvs_runtime::host::Completion`, and `nvs_server`'s `answer`
//! turns it into the header. A second buffer beside the output would have been
//! two answers to the question of what the body is.
//!
//! That channel is a `Completion` field rather than something read off a
//! `Ctx`, because a served request **is** an isolate (`rule:http-server/a-request-resolves-in-five-steps` step 5) and
//! the accept loop never holds its context — the completion is the one thing
//! that crosses.
//!
//! # A body written over time crosses earlier, and through a cell
//!
//! `stream` is the one body member whose bytes cannot go out on the completion,
//! because the completion is what a request *ends* with and a streaming body has
//! to reach the peer while the request is still running
//! (`rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`). So
//! it uses the seam `nvs_runtime::stream` is: the member opens the cell the
//! connection offered, leaves the media type and the reading half in it for the
//! head, and puts the writing half on this request's context — where
//! `Core\Response\Stream::write` finds it, one chunk at a time.
//!
//! The declaration is still made the way the three above make it, so nothing
//! about the `Content-Type` is a second path. What is new is only *when* the
//! head goes out, and that belongs to whoever is framing the response rather
//! than to this class.
//!
//! **Off a connection it degrades to the members beside it.** A carrier nothing
//! offered a cell to has no stream to open, so the chunks go to this request's
//! output and come out in the order they were written — the reading below of a
//! declaration made where no response is framed, applied to the bytes as well.
//! That is what makes a streamed body assertable from a `.nvst` case, none of
//! which is a request a server is answering.
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
//! **A status is not a body, so the `echo` refusal of
//! `rule:security/response-body-is-one-typed-member` does not reach it.** `E0801`
//! refuses an `echo` beside one of the five body members and `setStatus` is not
//! one of them — `nvs_types::response`'s roster is the five names — so a
//! handler that sets a status and then `echo`es is the ordinary spelling rather
//! than a mixture of two writers. Two body members are still
//! `rule:security/response-body-is-one-typed-member`'s question; a status
//! beside either is not.
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
//! pairs**, because each call is an override of *one* policy-owned
//! header and a response has as many of those as a program names. The list
//! replaces rather than appends: the member is `setHeader`, a second value
//! under one name is `addCookie`'s question, and a replaced pair keeps the
//! position it was first set at.
//!
//! **It is applied after everything the server wrote for itself**, which is
//! the whole of what
//! `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`
//! means by an override: the policy states what a response starts with, a
//! request may set any value for itself, and this member is the last writer.
//! The reverse order would leave it with no effect on exactly the headers it
//! exists to change.
//!
//! **`Content-Type` is refused here**, and that is this module's call: neither
//! that rule nor `rule:security/response-body-is-one-typed-member` states it.
//! The second rule's whole argument is that a body
//! member owns one shape *and* its media type, so that no call site names that
//! header; admitting it here would let a handler answer `json` and then
//! relabel it `text/html`, which is the mislabelling that rule exists to make
//! unspellable. Nothing is lost, because `bytes` is the member that takes a
//! media type and it takes it beside the body it describes. The refusal is
//! ASCII-case-insensitive, a field name being.
//!
//! **The four framing headers are refused beside it** ([`FRAMING_HEADERS`]):
//! `Content-Length`, `Transfer-Encoding`, `Connection` and `Upgrade`. `hyper`
//! frames a body by a `Content-Length` it is handed, so a program's that
//! disagrees with the body desynchronises the connection, which is a response
//! split with no newline in it. The other three describe the connection, and
//! only the server knows it.
//!
//! **Both parameters are sinks.** `rule:security/sink-predicate` is that a
//! parameter whose content becomes an instruction is one, and a header line is two instructions: a
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
//! the argument `rule:security/response-body-is-one-typed-member` already makes
//! for a body member owning its own media type.
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
//! **The URL is a `rule:security/sink-predicate` sink**, and it is the sink on
//! this class that carries the most: a destination chosen by the request is an open redirect, which is a
//! peer trusting this origin about where it goes next. Beyond that the value is
//! checked exactly as a header value is — [`carriable`], and non-empty, an
//! empty `Location` naming nothing — and a relative reference that starts with
//! two of `/` and `\` is refused, since it names another host and a
//! launderer's plain answer can hold one ([`names_another_host`]). What a URL
//! may otherwise be is not asked here, because the qualifier has already
//! answered the question that made it worth asking.
//!
//! # `text` takes `tainted`, and the mark is not the word its rule uses
//!
//! `rule:security/response-body-is-one-typed-member`'s table says the body is
//! **contagious**, and
//! `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` is why
//! it may be: `X-Content-Type-Options: nosniff` is on with nothing configured,
//! so a `text/plain` body is not re-parsed as HTML. Removing that default is
//! visibly a change to two rules: that one and
//! `rule:security/response-body-is-one-typed-member`.
//!
//! The registry spelling of that row is nonetheless
//! [`Qual::Neutral`](crate::registry::Qual::Neutral), because the two
//! taxonomies are over different things. The table's word is about the **body**: the
//! argument's bytes go out unchanged, so the member is not a sink. `Qual` is
//! about the **answer**, and `nvs_types`' `admits_tainted_argument` says a
//! [`Qual::Contagious`](crate::registry::Qual::Contagious) parameter admits
//! `tainted` only where the return type has somewhere to carry the bit — a
//! `void` member has nowhere, so that mark would *refuse* the very argument
//! the table admits. `Core\Cli::write`'s `string` arm is the same shape and the same
//! mark: a writer that answers nothing, and takes what it is given.
//!
//! **A body member declares and then writes, and nothing here arbitrates
//! between two declarations — the last one wins by construction.** That is a
//! bound rather than a gap, because the disagreement cannot reach this module:
//! `rule:security/response-body-is-one-typed-member`'s last paragraph makes two
//! writers of one body a compile error, `nvs_types::response` is that error, and
//! its module doc is the one home for what the refusal reaches and what it
//! deliberately leaves alone. The one case that does reach here is a mount's
//! entry script, which that module states as the bound its scope costs and
//! answers with the same last-one-wins this module already is.
//!
//! # A body goes out verbatim, and off a request the declaration is inert
//!
//! **Every body member writes its bytes with no rendering over them, under
//! whichever sink is in force**, and that is each media type's own contract
//! rather than a transform nobody got around to.
//! `rule:tooling/echo-always-has-a-sink`'s table picks a rendering for `echo`, whose operand is data interpolated into a page, and
//! `nvs_runtime`'s `write_rendered` is where that pick happens — escaping
//! inside an HTTP request and substituting everywhere else. A body member is
//! the table's other half and takes neither: a `Markup`'s bytes already passed
//! whichever rule made them markup ([`nvs_core_response_html`]), `json`'s
//! framing is the serializer's and never concatenation's, and `bytes` and
//! `sendFile` carry a media type that says what the octets are. A rendering
//! over any of them would be a second framing of bytes already framed, which is
//! what `rule:security/response-body-is-one-typed-member` exists to make
//! unspellable.
//!
//! **Off a request every member here still runs, onto a context nobody will
//! ask**: the media type, a status and a header are declared and then read by
//! no one, and the bytes reach whatever sink that program is on as they were
//! written. That is load-bearing rather than tolerated — it is what makes a
//! body assertable from a `.nvst` case, none of which is a request a server is
//! answering, and `stream` off a connection and `sendFile` off a request are
//! the same degradation, stated on each of those members. Nothing refuses the
//! call, because the refusal `rule:security/response-body-is-one-typed-member`
//! asks for is about two *writers* of one body
//! (`nvs_types::response`), and one member on a terminal is not that.
//!
//! **A body that will become a carrier takes the sink's rendering first.**
//! Inside a `Core\Out::capture`, and in a `spawn script` child, the bytes are
//! not a response: the capture or the parent receives them as the sink's
//! carrier, or appends them to its own stream. `text`, `json`, `bytes` and
//! `stream` write through `Ctx::write_body`, which renders them there the way
//! `echo` renders a `string`. A child's `text("<script>…")` shows as text in the
//! parent's page, and an `echo`ed one is escaped once, not twice
//! (`rule:security/capture-answers-the-carrier`). `html` writes a `Markup`,
//! and `sendFile` names a file the program chose, so both stay verbatim.

use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// What `html` declares — the `text/html` of
/// `rule:security/response-body-is-one-typed-member`, with the charset every
/// other text-shaped answer in this runtime carries, and the same string
/// `nvs_server::serve` answers a request that only echoed with: the sink and
/// the member write one kind of body, so they say one thing about it.
const HTML_MEDIA_TYPE: &str = "text/html; charset=utf-8";

/// What `text` declares — the `text/plain` of
/// `rule:security/response-body-is-one-typed-member`, with the charset every
/// other text-shaped answer in this runtime carries.
///
/// A constant rather than a literal at the write, because each body member owns
/// one of these and reading them in a column is how a reviewer checks the table
/// against that rule.
const TEXT_MEDIA_TYPE: &str = "text/plain; charset=utf-8";

/// How much of a file one write moves where `sendFile` is the one writing it —
/// off a request, where there is no connection to hand the name to.
///
/// A fixed buffer on the stack rather than the file's own size, so the fallback
/// holds what it is about to write and never what it is about to read: the
/// member's promise is that a response of any size costs a bounded amount of
/// memory, and the path that reads the bytes itself keeps it too.
const SEND_FILE_CHUNK: usize = 64 * 1024;

/// What `json` declares — the `application/json` of
/// `rule:security/response-body-is-one-typed-member`, and no `charset`: the
/// media type's own registration fixes the encoding at UTF-8, so a parameter
/// saying so again is one more thing two members could disagree about.
const JSON_MEDIA_TYPE: &str = "application/json";

/// The lowest status `setStatus` admits — RFC 9110 § 15's first class, and the
/// floor rather than `0` because a code below it names no class at all.
const STATUS_MIN: u16 = 100;

/// The highest status `setStatus` admits.
///
/// `599` and not `999`, which is what the wire format and `hyper` both allow:
/// A status code has a **class**, taken from its first digit, and the
/// five classes stop at `5`. A `6xx` is three digits a peer has no rule for, so
/// admitting it would be admitting a status whose only defined meaning is that
/// nothing downstream knows what it means — the same fail-closed direction
/// [`spellable`] takes for a media type, at the same layer.
const STATUS_MAX: u16 = 599;

/// The one header name `setHeader` refuses — `rule:security/response-body-is-one-typed-member`'s, owned by whichever
/// body member wrote the body. The module doc owns why it is refused rather
/// than admitted as one more override.
const CONTENT_TYPE_HEADER: &str = "Content-Type";

/// The headers that frame the message or describe its connection, which
/// `setHeader` refuses beside [`CONTENT_TYPE_HEADER`]. The server writes each
/// from the body it sends, and `hyper` trusts a `Content-Length` it is handed,
/// so a program's would frame the body wrong. `nvs_server`'s `overrides` drops
/// the same four.
const FRAMING_HEADERS: [&str; 4] = [
    "Content-Length",
    "Transfer-Encoding",
    "Connection",
    "Upgrade",
];

/// The header a redirect names its destination in, and the one header name
/// this class writes on a program's behalf rather than being told.
///
/// A constant beside [`CONTENT_TYPE_HEADER`] rather than a literal at the
/// declaration, so the two names this module knows about read in a column.
const LOCATION_HEADER: &str = "Location";

/// The fifteen non-alphanumeric bytes RFC 9110's `token` admits, which is what
/// a field name is made of.
const TOKEN_MARKS: &[u8] = b"!#$%&'*+-.^_`|~";

/// `Core\Response`'s fully-qualified name, written once — [`CLASS`] declares it
/// and [`crate::registry::CAPABILITIES`] names it on every row of this class, so
/// the table and the roster cannot drift apart.
pub(crate) const NAME: &str = r"Core\Response";

/// `Core\Response`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "The response your program sends back to a request. `html()`, `text()`, `json()`, \
            `bytes()`, `sendFile()` and `stream()` write the body, and each one sets the matching \
            `Content-Type`. `setStatus()`, `setHeader()`, `redirect()` and `addCookie()` change \
            the status line and the headers.",
};

/// `Core\Response`'s registry rows — its body members, for the ones that
/// exist.
///
/// The fourth request-facing class, and the first one that *writes*: the
/// typed body members of `rule:security/response-body-is-one-typed-member`.
/// Ranked beside `Core\Server` because the two are the same request's two
/// halves, and [`crate::response`]'s module doc owns what a written body is —
/// the bytes are `echo`'s output and the member adds the `Content-Type`.
// rank: 760
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "html",
            names: &["body"],
            // Unmarked, and the row where that is the *point*:
            // `rule:security/response-body-is-one-typed-member` says the HTML
            // member "takes the carrier and so has nothing to refuse",
            // because `Core\Html\Markup` is reached only through
            // `rule:core-classes/html-auto-escape`'s own doors — an html
            // template, `as` on a string literal, a launderer — each of which
            // already decided what may be raw. A `Qual` here would classify a
            // parameter whose type has settled the question.
            params: &[CoreTy::Instance(crate::html::MARKUP_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_html",
            doc: Some(&HTML_DOC),
        },
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
            // `Neutral`, not `Contagious` — the module doc owns why the word of
            // `rule:security/response-body-is-one-typed-member` and this enum's case part company at a `void` member.
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
            // type is a `rule:security/sink-predicate` sink, because it is the one string here that
            // becomes an *instruction* to the peer about how to read the rest.
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_bytes",
            doc: Some(&BYTES_DOC),
        },
        CoreMethod {
            name: "sendFile",
            names: &["path"],
            // A `rule:security/sink-predicate` sink, and the one on this class
            // whose argument reaches the filesystem: `..` and the separators direct the resolver, so a
            // path the request chose is the traversal that reads whatever this
            // program was granted. The bytes at it are data and carry no mark
            // of their own — nothing here re-parses them, and what they are
            // called is the static policy's table rather than this member's
            // argument.
            params: &[CoreTy::Path(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_send_file",
            doc: Some(&SEND_FILE_DOC),
        },
        CoreMethod {
            name: "stream",
            names: &["contentType", "length", "etag", "mustEnd"],
            // `bytes`' second mark, for `bytes`' reason and no other: this is
            // the other member that cannot know the media type, so it is told
            // one, and a media type is the one string here that becomes an
            // *instruction* to the peer about how to read everything after it
            // (`rule:security/sink-predicate`). The tag is a header too, and
            // the peer sends it back to choose which bytes it gets, so it is
            // marked the same way. What the chunks carry is [`STREAM`]'s
            // question and is marked there. The label only reaches the log,
            // which escapes what it writes, so it is not a sink.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Nullable(&CoreTy::Int),
                CoreTy::Nullable(&CoreTy::Text(Qual::Sink)),
                CoreTy::Nullable(&CoreTy::Text(Qual::Neutral)),
            ],
            defaults: &[Const::Null, Const::Null, Const::Null],
            return_ty: CoreTy::Instance(STREAM_NAME),
            symbol: STREAM_SYMBOL,
            doc: Some(&STREAM_DOC),
        },
        // The body table of `rule:security/response-body-is-one-typed-member`
        // ends above; the members that shape a response
        // without writing one follow it, `setStatus` first. Two groups rather
        // than one because the two lists answer different questions — which
        // shape a body is, and what else a response says — and interleaving
        // them would leave a reader unable to read either as one list.
        CoreMethod {
            name: "setStatus",
            names: &["code"],
            // `Uint`, not `Int`: `rule:types/arithmetic`'s type refuses a negative
            // literal at compile time, and there is no status code below 100
            // for a signed parameter to have been useful about. Not a `Sink`
            // either — the doc below owns why a number cannot be one.
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_set_status",
            doc: Some(&SET_STATUS_DOC),
        },
        // Beside `setStatus` because it is the other member that says how the
        // response goes out without writing any of it.
        CoreMethod {
            name: "slotted",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_slotted",
            doc: Some(&SLOTTED_DOC),
        },
        CoreMethod {
            name: "setHeader",
            names: &["name", "value"],
            // Both are `rule:security/sink-predicate` sinks, unlike `setStatus`' number: a header line is
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
            // The URL is a `rule:security/sink-predicate` sink on
            // `setHeader`'s reasoning and then some:
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
        CoreMethod {
            name: "addCookie",
            names: &["name", "value"],
            params: &[
                // Both `rule:security/sink-predicate` sinks on `setHeader`'s
                // reasoning, and the value for
                // one more: a cookie is the one header the *server* reads back
                // on the next request, so a `tainted` value laundered by
                // nothing would arrive at that request looking like state this
                // program chose. The framing is this member's — a value that
                // could close it is refused below — so the sink is about where
                // the bytes go next rather than about escaping.
                CoreTy::Text(Qual::Sink),
                CoreTy::Text(Qual::Sink),
                CoreTy::Options(&[
                    // Every one of the four `[http.cookies]` states defaults
                    // to `Const::Null` rather than to the shipped value of
                    // `rule:http-server/cookies-are-secure-httponly-and-lax`:
                    // written here, the default would be the *shipped* one at
                    // every call site and the configured block would reach
                    // nothing. Null is "the call site said nothing", which is
                    // the only spelling that leaves `nvs_config::http::Cookies`
                    // something to answer — `Core\Queue::push`'s `maxAttempts`
                    // is the same arrangement over `[queue]`.
                    CoreOption {
                        name: "secure",
                        ty: CoreTy::Bool,
                        default: Const::Null,
                    },
                    CoreOption {
                        name: "httpOnly",
                        ty: CoreTy::Bool,
                        default: Const::Null,
                    },
                    CoreOption {
                        name: "sameSite",
                        // An enum and never the string the `[http.cookies]`
                        // block writes
                        // (`rule:core-api/shape-rules` R11), which is also why the boot refusal
                        // for a fourth spelling is `E0624` and not this
                        // member's problem: by the time a case arrives here
                        // there are only three it can be.
                        ty: CoreTy::Enum(SAME_SITE_NAME),
                        default: Const::Null,
                    },
                    CoreOption {
                        name: "path",
                        ty: CoreTy::Text(Qual::Sink),
                        default: Const::Null,
                    },
                    // The two `rule:http-server/cookies-are-secure-httponly-and-lax`
                    // does not configure, and they are here for
                    // separate reasons. `domain` exists because `rule:errors/cookie-name-bytes`
                    // *forbids* it under a `__Host-` prefix, and a rule the
                    // runtime enforces about a spelling the language does not
                    // have is not a rule. `maxAge` exists because a cookie
                    // with no expiry is a session cookie and there would
                    // otherwise be no way to write any other kind. Neither
                    // takes a configured default — that rule states none, and a
                    // shipped lifetime is a policy nobody asked for.
                    CoreOption {
                        name: "domain",
                        ty: CoreTy::Text(Qual::Sink),
                        default: Const::Null,
                    },
                    CoreOption {
                        name: "maxAge",
                        ty: CoreTy::Instance(crate::time::DURATION_NAME),
                        default: Const::Null,
                    },
                ]),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_add_cookie",
            doc: Some(&ADD_COOKIE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Response\Stream`'s fully-qualified name, written once — [`STREAM`]
/// declares it and the [`CoreTy::Instance`] naming it resolves against
/// [`crate::registry::CLASSES`], so the two cannot drift apart.
pub(crate) const STREAM_NAME: &str = r"Core\Response\Stream";

/// The symbols the two rows above and below are reached through.
const STREAM_SYMBOL: &str = "nvs_core_response_stream";
/// See [`STREAM_SYMBOL`].
const STREAM_WRITE_SYMBOL: &str = "nvs_core_response_stream_write";
/// See [`STREAM_SYMBOL`].
const STREAM_OFFSET_SYMBOL: &str = "nvs_core_response_stream_offset";
/// See [`STREAM_SYMBOL`].
const STREAM_LENGTH_SYMBOL: &str = "nvs_core_response_stream_length";
/// See [`STREAM_SYMBOL`].
const STREAM_END_SYMBOL: &str = "nvs_core_response_stream_end";

/// `Core\Response\Stream`'s class card — `rule:core-api/reference-card`.
const STREAM_CARD: ClassDoc = ClassDoc {
    short: "A response body that your program sends in parts. `Core\\Response::stream()` returns \
            it, and its `write()` method sends each part. The body ends when the program ends, \
            or when your program calls `end()`.",
};

/// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
/// first spelling, as the handle a program writes through — what
/// [`nvs_core_response_stream`] answers with.
///
/// **No slots**, because its whole state is the *context*'s:
/// `nvs_runtime::Ctx` holds the writing half of the body and the window a
/// stated length gives it ([`nvs_runtime::stream::Window`]), so the handle is a
/// token saying a stream was opened rather than an owner of one. A slot holding
/// a copy of the writing half would be a second owner of the cell, which is the
/// one thing [`crate::instance`]'s first decision refuses; `Core\Socket` is the
/// same arrangement over a connection's peer, and
/// `a_class_with_slots_has_instance_members_and_the_reverse` is where the two
/// are listed.
///
/// **The body ends when the isolate does, or at `end`.** The isolate's finish
/// path ends a body nothing ended, and `nvs_runtime::stream::Emit`'s own `Drop`
/// is the floor under it. `end` is there for a stream opened with `$mustEnd`:
/// such a body is complete only once the program says so, and one the program
/// never ended is cut off and logged when it stops
/// (`nvs_runtime::Ctx::end_unfinished_stream`), so a client never takes a body
/// cut short by a returning program for a whole one. A client reading one with
/// `EventSource` reconnects when the body ends; that is a property of a
/// request-scoped stream rather than a fault, and the reader for it is `fetch`
/// or a progress UI that closes itself.
///
/// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
/// request-scoped half, immediately after the class whose `stream` is the
/// only thing that produces one. [`crate::response`] owns why the handle has
/// no slots and why there is no member that closes it.
// rank: 770
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
    doc: Some(&STREAM_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "write",
            names: &["chunk"],
            // A union rather than `Core\Socket`'s two members, because a chunk of a
            // response body is one thing written two ways where a WebSocket frame's
            // two payload kinds are two things on the wire. The classification is
            // written on both arms and the union answers it
            // ([`CoreTy::classification`]): [`Qual::Sink`], so a `tainted` chunk
            // does not compile. `Core\Response::text` may admit one because its
            // media type is fixed at `text/plain`; a stream's is whatever the
            // program declared, `text/html` included, so a chunk is markup the
            // peer may parse (`rule:security/response-body-is-one-typed-member`).
            params: &[CoreTy::Union(&[
                CoreTy::Text(Qual::Sink),
                CoreTy::Blob(Qual::Sink),
            ])],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: STREAM_WRITE_SYMBOL,
            doc: Some(&STREAM_WRITE_DOC),
        },
        CoreMethod {
            name: "offset",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: STREAM_OFFSET_SYMBOL,
            doc: Some(&STREAM_OFFSET_DOC),
        },
        CoreMethod {
            name: "length",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: STREAM_LENGTH_SYMBOL,
            doc: Some(&STREAM_LENGTH_DOC),
        },
        CoreMethod {
            name: "end",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: STREAM_END_SYMBOL,
            doc: Some(&STREAM_END_DOC),
        },
    ],
    slots: &[],
    constants: &[],
};

/// `Core\Response::stream`'s reference card — `rule:core-api/reference-card`.
const STREAM_DOC: MethodDoc = MethodDoc {
    short: "Starts a response body that your program sends in parts, and sets the header \
            `Content-Type` to `$contentType`. The status and the headers are sent when this is \
            called. The body ends when the program ends, or when your program calls `end()` on \
            the stream.",
    params: &[
        ParamDoc {
            name: "contentType",
            desc: "The content type of the body, for example `text/csv`. It becomes a header, so \
                   a `tainted` value does not compile. It must be printable ASCII and not empty.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "The size of the whole body in bytes, or `null` if you do not know it. With a \
                   size, the response has a `Content-Length` header and a browser can ask for \
                   one part of the body, for example to seek in a video. The request header \
                   `Range` says which part. Then `offset()` and `length()` of the stream say \
                   which bytes to write. Without a size, the whole body is sent and `Range` is \
                   ignored.",
            shape: &[],
        },
        ParamDoc {
            name: "etag",
            desc: "A version name for the body, or `null`. It is sent as the `ETag` header. Give \
                   a new name when the bytes change. A browser that resumes a download sends the \
                   name back, and gets the whole body if the name is old. It becomes a header, so \
                   a `tainted` value does not compile.",
            shape: &[],
        },
        ParamDoc {
            name: "mustEnd",
            desc: "A name for the body, such as the name of a download, or `null`. With a name, \
                   the body is complete only after your program calls `end()` on the stream. If \
                   the program stops before that, the body is cut off. The browser then shows a \
                   failed download, and the error log names the body.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Response\\Stream`. Call its `write()` method to send each part. Set the status \
          and the headers before you call this method. Using this and `echo` in one response does \
          not compile. A browser that reads this body with `EventSource` connects again when the \
          body ends. For events that a page receives while it is open, use `Core\\Sse::upgrade`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$contentType` is empty, or it contains a character a header cannot contain: a \
               control character, a line break, or a character that is not ASCII. It is also \
               thrown when `$length` is negative, when `$etag` contains such a character or a \
               `\"`, and when this request already started a body stream, because a response \
               has one body.",
    }],
};

/// `Core\Response\Stream::offset`'s reference card — `rule:core-api/reference-card`.
const STREAM_OFFSET_DOC: MethodDoc = MethodDoc {
    short: "Returns the position of the first byte your program writes, counted from the start of \
            the whole body. It is `0` unless the browser asked for a part of the body.",
    params: &[],
    ret: "An `int`. Start writing the body at this byte.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The stream was started without a `$length`.",
    }],
};

/// `Core\Response\Stream::length`'s reference card — `rule:core-api/reference-card`.
const STREAM_LENGTH_DOC: MethodDoc = MethodDoc {
    short: "Returns how many bytes your program writes. This is the whole body, or the part the \
            browser asked for.",
    params: &[],
    ret: "An `int`. It is `0` when nothing is sent: the browser already has this version, or it \
          asked for a part the body does not have. Your writes must add up to exactly this \
          number. If the program ends before that, the connection is closed, so the browser \
          knows the body is not complete.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The stream was started without a `$length`.",
    }],
};

/// `Core\Response\Stream::end`'s reference card — `rule:core-api/reference-card`.
const STREAM_END_DOC: MethodDoc = MethodDoc {
    short: "Ends the body. The client receives the end of the body at once, and your program can \
            go on running. Calling `end()` a second time does nothing.",
    params: &[],
    ret: "Nothing. A stream started with `$mustEnd` needs this call, or the body is cut off when \
          the program stops. A `write()` after `end()` throws an error.",
    errors: &[],
};

/// `Core\Response\Stream::write`'s reference card — `rule:core-api/reference-card`.
const STREAM_WRITE_DOC: MethodDoc = MethodDoc {
    short: "Sends one part of the body. If the client has not finished reading the last part, \
            this method waits until it has. So only one part waits in memory at a time.",
    params: &[ParamDoc {
        name: "chunk",
        desc: "The part to send, as a `string` or as `bytes`. It is sent exactly as it is. An \
               empty part sends nothing and is not an error. A `tainted` value does not \
               compile, because the browser may read the part as HTML.",
        shape: &[],
    }],
    ret: "Nothing. When this method returns, the part has been given to the connection. If the \
          client has gone, this method returns at once and the part is thrown away. The client \
          has gone when it closed the connection, or when it did not read a part before the \
          server's send timeout. Your request still runs to its end.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The part is larger than the server allows for one part. Nothing is sent, and \
                   you can still send smaller parts.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "The stream was started with a `$length`, and this part goes past the end of \
                   `length()`. Nothing is sent. It is also thrown when `end()` was already \
                   called on the stream.",
        },
    ],
};

/// `Core\Response::html`'s reference card — `rule:core-api/reference-card`.
const HTML_DOC: MethodDoc = MethodDoc {
    short: "Sends the HTML page `$body` and adds the header `Content-Type: text/html; \
            charset=utf-8`.",
    params: &[ParamDoc {
        name: "body",
        desc: "The page to send. It is a `Core\\Html\\Markup`, which is HTML that is already safe: \
               an html template, ``html`…` ``, escaped the values in its `{...}`, or \
               `Core\\Html::escape` made it. It is sent as it is and nothing is escaped again. \
               A `string` does not compile.",
        shape: &[],
    }],
    ret: "Nothing. Using this and `echo` in one response does not compile.",
    errors: &[],
};

/// `Core\Response::json`'s reference card — `rule:core-api/reference-card`.
const JSON_DOC: MethodDoc = MethodDoc {
    short: "Converts `$value` to JSON, sends it, and adds the header `Content-Type: \
            application/json`. It uses the same encoder as `Core\\Json::encode`, and the JSON is \
            on one line.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to send, of any type `Core\\Json::encode` accepts. A `tainted` value is \
               allowed anywhere inside it. Each string is written as a JSON string, so its quotes \
               are escaped and it cannot change the JSON around it.",
        shape: &[],
    }],
    ret: "Nothing. Using this and `echo` in one response does not compile.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$value` contains something JSON cannot write: a `float` that is NaN or infinite, \
               a value of a type JSON has no form for, an object of a class without \
               `#[Core\\Json\\Derive]`, an object that points to itself, or nesting deeper than \
               1024 levels. Nothing is sent.",
    }],
};

/// `Core\Response::bytes`'s reference card — `rule:core-api/reference-card`.
const BYTES_DOC: MethodDoc = MethodDoc {
    short: "Sends `$body` exactly as it is, with `$contentType` as the `Content-Type` header. Use \
            it for an image, a CSV file or any other body that is not HTML, text or JSON.",
    params: &[
        ParamDoc {
            name: "body",
            desc: "The bytes to send. Nothing is escaped or changed.",
            shape: &[],
        },
        ParamDoc {
            name: "contentType",
            desc: "The content type, such as `image/png` or `text/csv; charset=utf-8`. A \
                   `tainted` value does not compile, because the browser obeys this header.",
            shape: &[],
        },
    ],
    ret: "Nothing. Using this and `echo` in one response does not compile.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$contentType` is empty, or has a character that is not printable ASCII: a line \
               break, a control character or a letter like `é`. Nothing is sent.",
    }],
};

/// `Core\Response::sendFile`'s reference card — `rule:core-api/reference-card`.
const SEND_FILE_DOC: MethodDoc = MethodDoc {
    short: "Sends the file at `$path` as the whole response. The server reads the file in small \
            pieces, and chooses the content type from the end of the file name.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to send. `fs.read` in `nvs.toml` must allow it. A `tainted` value does not \
               compile, because the path decides which file is read. To give a download its own \
               name, set `Content-Disposition` with `Core\\Response::setHeader`.",
        shape: &[],
    }],
    ret: "Nothing. The server also responds to a browser that requests only part of the file, so a \
          browser can jump to any point in a video and resume a download that stopped. If the \
          file changed since the download started, the browser gets the whole new file. Using \
          this and `echo` in one response does not compile.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`fs.read` does not allow `$path`. This error comes whether or not the file \
                   exists, the same as for `Core\\IO::read`.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "There is no file at `$path`, or the operating system does not allow this \
                   program to read it.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$path` is a folder, or something else that is not a regular file. Nothing is \
                   sent.",
        },
    ],
};

/// `Core\Response::text`'s reference card — `rule:core-api/reference-card`.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "Sends `$body` as plain text and adds the header `Content-Type: text/plain; \
            charset=utf-8`.",
    params: &[ParamDoc {
        name: "body",
        desc: "The text to send. It is sent exactly as it is. A `tainted` value is allowed: the \
               server also sends `X-Content-Type-Options: nosniff`, so a browser never reads the \
               text as HTML.",
        shape: &[],
    }],
    ret: "Nothing. Using this and `echo` in one response does not compile.",
    errors: &[],
};

/// `Core\Response::setStatus`'s reference card — `rule:core-api/reference-card`.
const SET_STATUS_DOC: MethodDoc = MethodDoc {
    short: "Sets the HTTP status of the response to `$code`, such as `404` or `201`. It does \
            not write a body, so you can use it together with `echo` or a body method.",
    params: &[ParamDoc {
        name: "code",
        desc: "The status code, from 100 to 599.",
        shape: &[],
    }],
    ret: "Nothing. If you call it more than once, the last call sets the status. If the request \
          fails with an error, the status is `500`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$code` is less than 100 or greater than 599. The status does not change.",
    }],
};

/// `Core\Response::slotted`'s reference card — `rule:core-api/reference-card`.
const SLOTTED_DOC: MethodDoc = MethodDoc {
    short: "Sends the page as soon as the main script ends. Each `Core\\Html::later` part \
            follows when it is ready. A route with `slotted: true` does the same for every \
            request.",
    params: &[],
    ret: "Nothing. Calling it twice is the same as calling it once.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The main script has already ended. This is the case in a `Core\\Html::later` \
               function and in `Core\\Task::afterResponse` work.",
    }],
};

/// `Core\Response::setHeader`'s reference card — `rule:core-api/reference-card`.
const SET_HEADER_DOC: MethodDoc = MethodDoc {
    short: "Sets the response header `$name` to `$value`. If the server already sets this \
            header, your value replaces the server's value.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The header name, such as `Cache-Control`. It contains letters, digits and a \
                   few marks such as `-` and `_`. Spaces and `:` are not allowed. It cannot be \
                   `Content-Type`, because the body method you use sets that header. It cannot \
                   be `Content-Length`, `Transfer-Encoding`, `Connection` or `Upgrade`. The \
                   server sets these from the body it sends. A `tainted` value is not allowed.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The header value. It contains only printable ASCII characters, so it cannot \
                   contain a newline. It can be empty. A `tainted` value is not allowed.",
            shape: &[],
        },
    ],
    ret: "Nothing. If you set the same name twice, the header has the last value. The \
          comparison of names ignores upper and lower case. If the request fails with an error, \
          the response has none of these headers.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$name` is empty, contains a character that is not allowed, or is one of the \
               headers the server sets, such as `Content-Type` or `Content-Length`. Or `$value` \
               contains a character that is not printable ASCII.",
    }],
};

/// `Core\Response::redirect`'s reference card — `rule:core-api/reference-card`.
const REDIRECT_DOC: MethodDoc = MethodDoc {
    short: "Sends the browser to another address. It sets the redirect status and the \
            `Location` header together. It does not write a body.",
    params: &[
        ParamDoc {
            name: "url",
            desc: "The address to send the browser to. It can be a full URL or a path such as \
                   `/orders/42`. It is not empty and contains only printable ASCII characters. \
                   A `tainted` value is not allowed, because a visitor must not choose where the \
                   browser goes.",
            shape: &[],
        },
        ParamDoc {
            name: "status",
            desc: "The kind of redirect: `SeeOther` (303), `Temporary` (307) or `Permanent` \
                   (308). The default is `SeeOther`, which you use after a form is sent.",
            shape: &[],
        },
    ],
    ret: "Nothing. If you call it more than once, the last call wins. If the request fails with \
          an error, the status is `500` and there is no `Location` header.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$url` is empty, or contains a character that is not printable ASCII, such as a \
               newline. Or it starts with two slashes or backslashes, such as `//example.com`, \
               which a browser reads as another host. Nothing is set.",
    }],
};

/// `Core\Response::addCookie`'s reference card — `rule:core-api/reference-card`.
const ADD_COOKIE_DOC: MethodDoc = MethodDoc {
    short: "Adds one `Set-Cookie` header to this response. Each option you leave out comes from \
            `[http.cookies]`. With the default settings, a cookie without options is `Secure; \
            HttpOnly; SameSite=Lax; Path=/`.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The cookie name. When the cookie comes back, the name is compared byte for \
                   byte, and nothing in it is changed. A `__Host-` prefix needs `Secure` and \
                   `Path=/`, and does not allow `Domain`. A `__Secure-` prefix needs `Secure`. \
                   A cookie that breaks these rules throws an error.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The cookie value. It can contain printable ASCII characters, as RFC 6265 \
                   says. A space, a comma, a semicolon, a backslash and a quote are not allowed. \
                   Each of them would end the value and start text your program did not write.",
            shape: &[],
        },
        ParamDoc {
            name: "secure",
            desc: "Whether the browser sends the cookie only over HTTPS. The default is \
                   `[http.cookies] secure`, which is `true` if you do not set it.",
            shape: &[],
        },
        ParamDoc {
            name: "httpOnly",
            desc: "Whether the cookie is hidden from scripts in the page. The default is \
                   `[http.cookies] http_only`, which is `true` if you do not set it. Set it to \
                   `false` here for a cookie that a script must read.",
            shape: &[],
        },
        ParamDoc {
            name: "sameSite",
            desc: "Which requests from other sites send the cookie. The default is \
                   `[http.cookies] same_site`, which is `Lax` if you do not set it. `None` \
                   without `Secure` throws an error, because every browser ignores that cookie. \
                   The same pair in the configuration stops the server from starting.",
            shape: &[],
        },
        ParamDoc {
            name: "path",
            desc: "The path prefix under which the browser sends the cookie. The default is \
                   `[http.cookies] path`, which is `/` if you do not set it.",
            shape: &[],
        },
        ParamDoc {
            name: "domain",
            desc: "The domain the browser sends the cookie to. By default the cookie has no \
                   domain, so the browser sends it only to this host and not to its subdomains. \
                   `[http.cookies]` has no setting for it.",
            shape: &[],
        },
        ParamDoc {
            name: "maxAge",
            desc: "How long the cookie lives. By default it has no age, so it is a session \
                   cookie. `0s` deletes the cookie. A negative duration throws an error.",
            shape: &[],
        },
    ],
    ret: "Nothing. Each call adds one cookie, so two calls send two `Set-Cookie` lines. If you \
          add the same name twice, the response sends it twice.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$name` is not a valid cookie name, or it breaks the rules of its `__Host-` or \
               `__Secure-` prefix. It is also thrown when `$value`, `path` or `domain` contains a \
               character that would end the attribute and start another one. It is also thrown \
               when `sameSite` is `None` without `secure`, and when `maxAge` is negative.",
    }],
};

/// `Core\Response::SameSite`'s fully-qualified name, written once — [`SAME_SITE`]
/// declares it and the [`CoreTy::Enum`] naming it resolves against
/// [`crate::registry::ENUMS`], so the two cannot drift apart.
pub(crate) const SAME_SITE_NAME: &str = r"Core\Response::SameSite";

/// `rule:http-server/cookies-are-secure-httponly-and-lax`'s `SameSite`, as an enum and never the
/// string `[http.cookies]` writes.
///
/// Numbered from zero, unlike [`REDIRECT`] beside it: this attribute's three
/// names have no number on the wire, so there is nothing for an ordinal to
/// coincide with and [`same_site_of`] is the whole conversion.
///
/// The cases are `nvs_config::http::SameSite`'s, and the two are converted
/// across rather than shared, because that crate is not a dependency this one
/// reaches for a registry constant — the same arrangement `crate::log`'s level
/// enum already has with `[log] level`.
pub(crate) const SAME_SITE: CoreEnum = CoreEnum {
    name: SAME_SITE_NAME,
    cases: &[("Lax", 0), ("Strict", 1), ("None", 2)],
    doc: Some(&SAME_SITE_CASES_DOC),
};

/// [`SAME_SITE`]'s reference card — `rule:core-api/reference-card`.
const SAME_SITE_CASES_DOC: EnumDoc = EnumDoc {
    short: "Which requests from other sites send a cookie. The three cases are the values of the \
            cookie's `SameSite` attribute. The default is `Lax`. A cookie that is sent with \
            requests from other sites makes a CSRF attack possible.",
    cases: &[
        CaseDoc {
            name: "Lax",
            desc: "The browser sends the cookie when a person opens a page of this site, also \
                   from another site. It does not send it with an image, a form post or a \
                   `fetch` from another site. This is the default, with or without configuration.",
        },
        CaseDoc {
            name: "Strict",
            desc: "The browser never sends the cookie with a request from another site. This is \
                   also true when a person opens a page. So a visitor who follows a link from \
                   another site is logged out. For this reason, it is not the default.",
        },
        CaseDoc {
            name: "None",
            desc: "The browser sends the cookie with requests from other sites. It needs \
                   `Secure`, because browsers ignore the cookie without it. Without `Secure`, it \
                   throws an error, and in the configuration it stops the server from starting.",
        },
    ],
};

/// `Core\Response::Redirect`'s fully-qualified name, written once — [`REDIRECT`]
/// declares it and the [`CoreTy::Enum`] naming it resolves against
/// [`crate::registry::ENUMS`], so the two cannot drift apart.
pub(crate) const REDIRECT_NAME: &str = r"Core\Response::Redirect";

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

/// [`REDIRECT`]'s reference card — `rule:core-api/reference-card`.
///
/// Named for its cases rather than `REDIRECT_DOC`, which the member above it
/// already is: the class and the enum share the one word `Redirect`.
const REDIRECT_CASES_DOC: EnumDoc = EnumDoc {
    short: "The kind of redirect a response sends. Every browser handles these three statuses \
            in the same way, so the browser does exactly what your program says.",
    cases: &[
        CaseDoc {
            name: "SeeOther",
            desc: "`303`. The browser loads the new address with `GET`, whatever the method of \
                   the request was. Use it after a form post. This is the default.",
        },
        CaseDoc {
            name: "Temporary",
            desc: "`307`. The browser sends the same request again to the new address, with the \
                   same method and body. This is for this one time only. Nothing is cached, and \
                   the old address stays valid.",
        },
        CaseDoc {
            name: "Permanent",
            desc: "`308`. The browser sends the same request again to the new address, with the \
                   same method and body. The new address replaces the old one from now on. \
                   Caches and search engines may remember it.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_response_html" => (nvs_core_response_html as *const ()).cast(),
        "nvs_core_response_json" => (nvs_core_response_json as *const ()).cast(),
        "nvs_core_response_text" => (nvs_core_response_text as *const ()).cast(),
        "nvs_core_response_bytes" => (nvs_core_response_bytes as *const ()).cast(),
        "nvs_core_response_send_file" => (nvs_core_response_send_file as *const ()).cast(),
        STREAM_SYMBOL => (nvs_core_response_stream as *const ()).cast(),
        STREAM_WRITE_SYMBOL => (nvs_core_response_stream_write as *const ()).cast(),
        STREAM_OFFSET_SYMBOL => (nvs_core_response_stream_offset as *const ()).cast(),
        STREAM_LENGTH_SYMBOL => (nvs_core_response_stream_length as *const ()).cast(),
        STREAM_END_SYMBOL => (nvs_core_response_stream_end as *const ()).cast(),
        "nvs_core_response_set_status" => (nvs_core_response_set_status as *const ()).cast(),
        "nvs_core_response_slotted" => (nvs_core_response_slotted as *const ()).cast(),
        "nvs_core_response_set_header" => (nvs_core_response_set_header as *const ()).cast(),
        "nvs_core_response_redirect" => (nvs_core_response_redirect as *const ()).cast(),
        "nvs_core_response_add_cookie" => (nvs_core_response_add_cookie as *const ()).cast(),
        _ => return None,
    })
}

/// Whether `media_type` is something a `Content-Type` header can carry — the
/// sink `rule:security/sink-predicate` makes of a header value, checked at the
/// member rather than only at the connection.
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
pub(crate) fn nameable(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || TOKEN_MARKS.contains(&byte))
}

/// `rule:core-classes/html-later`'s closed head: every member that sets a
/// status, a header, a cookie or a body, and `Core\Session::regenerate`, calls
/// this first, and a `Core\Html::later` slot calling one throws `LogicError`
/// before anything is declared. The main script owns the head, and on a slotted
/// route it is already on the wire by the time a slot runs.
///
/// `Core\Task::afterResponse` work calling one throws `LogicError` the same way
/// (`rule:concurrency/deferred-work-cannot-write-the-response`), because the
/// whole response is sent before that work runs.
pub(crate) fn head_open(ctx: &nvs_runtime::Ctx, member: &str) -> Result<(), Fault> {
    if ctx.in_deferred_work() {
        return Err(Fault::thrown_as(
            nvs_runtime::ThrownClass::Logic,
            format!(
                "A function passed to `Core\\Task::afterResponse` cannot call `{member}`. It runs \
                 after the response was sent."
            ),
        ));
    }
    if ctx.in_later_slot() {
        // A literal stem before the first hole, which is
        // `conformance_coverage`'s error-path gate matching a site.
        return Err(Fault::thrown_as(
            nvs_runtime::ThrownClass::Logic,
            format!(
                "A function passed to `Core\\Html::later` cannot call `{member}`. Only the main \
                 script sets the status, the headers, the cookies and the body."
            ),
        ));
    }
    Ok(())
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::html(Core\Html\Markup $body): void` — `rule:security/response-body-is-one-typed-member`'s first row.
    ///
    /// [`nvs_core_response_text`]'s two effects over the carrier's bytes
    /// instead of a `string`'s, and in that member's order: the media type is
    /// declared before the write, for the reason written there.
    ///
    /// **Nothing is escaped here, and that is the rule rather than an
    /// omission.** A `Core\Html\Markup` exists only where
    /// `rule:core-classes/html-auto-escape` let one be built — an html
    /// template, whose holes this member's argument already went through, `as`
    /// on a string literal, or a launderer — so escaping the slot again would
    /// corrupt the page it was built for, exactly as it would in
    /// [`crate::html::nvs_core_html_markup_concat`]. The bytes are read through
    /// [`crate::html::markup_slot`], which is the one reader of that slot, so
    /// this member adds no second way out of the carrier for
    /// `rule:core-classes/html-to-source` to have to be about.
    fn nvs_core_response_html(ctx, args: [1]) {
        head_open(ctx, "Core\\Response::html")?;
        let held = crate::html::markup_slot(args[0], r"`Core\Response::html`'s `$body`")?;
        // Unreachable from source twice over: the row's parameter is the
        // carrier, so `E0401` refuses anything else at the call, and the slot
        // holds what `crate::html` put there.
        let body = held.as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::html expected a `string` in the carrier's slot, got tag {}",
                held.tag_byte()
            ))
        })?;
        ctx.declare_content_type(HTML_MEDIA_TYPE);
        // Verbatim, and the module doc's *A body goes out verbatim* section owns
        // what that means off a request. Unreachable from source for
        // `nvs_core_response_text`'s reason: `OutputSink::Buffer` and `Sink`
        // never fail, and nothing in the language closes a descriptor the host
        // handed the process.
        ctx.write_output(body.as_bytes())
            .map_err(|error| Fault::fatal(format!("Core\\Response::html could not write: {error}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::text(string $body): void` — `rule:security/response-body-is-one-typed-member`'s third row.
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
        head_open(ctx, "Core\\Response::text")?;
        // Unreachable from source: the row's parameter is a `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs.
        let body = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::text expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        ctx.declare_content_type(TEXT_MEDIA_TYPE);
        // Verbatim, and the module doc's *A body goes out verbatim* section owns
        // what that means off a request and where it becomes markup.
        // Unreachable from source on `Core\Cli::write`'s reasoning:
        // `OutputSink::Buffer` and `Sink` never fail, and nothing in the
        // language closes a descriptor the host handed the process.
        ctx.write_body(body.as_bytes())
            .map_err(|error| Fault::fatal(format!("Core\\Response::text could not write: {error}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::setStatus(uint $code): void` — the response's status,
    /// replacing `http_response_code`.
    ///
    /// One effect and no second: the code is declared on this request's
    /// context, and nothing is written. That is the whole difference between
    /// this member and the four above it, and it is why `rule:security/response-body-is-one-typed-member`'s sixth
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
        head_open(ctx, "Core\\Response::setStatus")?;
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
    /// `Core\Response::slotted(): void` — `rule:core-classes/html-later`'s
    /// runtime switch, the call a route's `slotted: true` stands beside.
    ///
    /// One flag on the request's context and nothing written:
    /// `Ctx::make_slotted` owns the `LogicError` once the main script is over,
    /// which also covers a `later` slot, so this member does not call
    /// [`head_open`].
    fn nvs_core_response_slotted(ctx, _args: [0]) {
        ctx.make_slotted()?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::setHeader(string $name, string $value): void` — a
    /// response header, replacing `header`.
    ///
    /// The module doc owns the three decisions this member is: the list rather
    /// than a word, the refusal of `Content-Type`, and why both parameters are
    /// sinks. What is here is the order — every check runs before anything is
    /// declared, on `bytes`' reasoning — and the two checks themselves, which
    /// are [`nameable`] and [`carriable`].
    fn nvs_core_response_set_header(ctx, args: [2]) {
        head_open(ctx, "Core\\Response::setHeader")?;
        // Unreachable from source for both: the row's parameters are
        // `CoreTy::Text`, so `E0401` refuses anything that is not a `string`,
        // and refuses a `tainted` one besides, both being
        // `rule:security/sink-predicate` sinks.
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
        if FRAMING_HEADERS
            .iter()
            .any(|framing| name.eq_ignore_ascii_case(framing))
        {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::setHeader(): `{name}` frames the response, and the \
                     server writes it from the body it sends"
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
    /// `Core\Response::redirect(string $url, Core\Response::Redirect $status): void`
    /// — a redirect, replacing a `Location` written through `header`
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
        head_open(ctx, "Core\\Response::redirect")?;
        // Unreachable from source: the row's first parameter is a
        // `CoreTy::Text`, so `E0401` refuses anything that is not a `string`,
        // and refuses a `tainted` one besides, that being a
        // `rule:security/sink-predicate` sink.
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
        if names_another_host(url) {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::redirect(): `{url}` starts with two slashes or \
                     backslashes, so a browser goes to another host. For another host, write \
                     the whole address with its scheme, such as `https:`. For a path on this \
                     site, write one `/`"
                ),
            ));
        }
        ctx.declare_status(status);
        ctx.declare_header(LOCATION_HEADER, url);
        Ok(Value::null())
    }
}

/// Whether a `Location` value is a relative reference that names a host: it
/// starts with two of `/` and `\`, once the spaces a client trims from a field
/// value are gone, and a browser reads the backslash as a slash. The tainted
/// check cannot see this, because a launderer's plain answer (a shell escape,
/// say) can hold `//host` unchanged. An absolute URL is the program's own
/// writing, so it is not asked about.
fn names_another_host(url: &str) -> bool {
    let mut first = url.trim_start_matches(' ').chars();
    let slash = |ch: Option<char>| matches!(ch, Some('/' | '\\'));
    slash(first.next()) && slash(first.next())
}

/// `secure`'s argument slot, and the five after it in row order.
const SECURE_ARG: usize = 2;
/// `httpOnly`'s argument slot.
const HTTP_ONLY_ARG: usize = 3;
/// `sameSite`'s argument slot.
const SAME_SITE_ARG: usize = 4;
/// `path`'s argument slot.
const PATH_ARG: usize = 5;
/// `domain`'s argument slot.
const DOMAIN_ARG: usize = 6;
/// `maxAge`'s argument slot.
const MAX_AGE_ARG: usize = 7;

/// What [`nvs_core_response_add_cookie`] appends under — never `declare_header`,
/// for the reason [`nvs_runtime::Ctx::append_header`]'s own doc gives.
pub(crate) const SET_COOKIE_HEADER: &str = "Set-Cookie";

/// One `Set-Cookie` line's parts, each already known to carry no byte that
/// could end the part it is in.
///
/// A shape rather than eight arguments because it has a second caller:
/// `Core\Session::start` writes the identifier's cookie
/// ([`crate::session`]), and a second rendering there would be a second place
/// the `SameSite` spelling, the attribute order and the `Secure`/`HttpOnly`
/// flags could drift. The *policy* already has one home in
/// [`nvs_config::http::Cookies`]; this is the other half of the same rule, and
/// the two callers differ only in where the parts came from.
///
/// **Checking is the caller's, and it is not symmetric.**
/// [`nvs_core_response_add_cookie`] validates every part because a program
/// supplied them; the session's parts are a configured name and 22 base64url
/// characters this core drew, so it has nothing to check and no refusal to
/// raise. Putting the checks in here would have made the second caller carry a
/// throw it can never take.
pub(crate) struct Cookie<'a> {
    /// The name, matched byte for byte on the way back.
    pub name: &'a str,
    /// The value, as RFC 6265's `cookie-octet` admits it.
    pub value: &'a str,
    /// The `Path` attribute, which is never omitted.
    pub path: &'a str,
    /// The `Domain` attribute, omitted for the narrower of its two meanings.
    pub domain: Option<&'a str>,
    /// `Max-Age` in seconds, omitted for a session cookie.
    pub max_age: Option<i64>,
    /// Whether the line carries `Secure`.
    pub secure: bool,
    /// Whether the line carries `HttpOnly`.
    pub http_only: bool,
    /// Which cross-site requests carry it.
    pub same_site: nvs_config::http::SameSite,
}

impl Cookie<'_> {
    /// The header line, concatenated with no escape anywhere — which is what
    /// the caller's checks bought.
    pub(crate) fn line(&self) -> String {
        let mut line = format!("{}={}; Path={}", self.name, self.value, self.path);
        if let Some(domain) = self.domain {
            line.push_str("; Domain=");
            line.push_str(domain);
        }
        if let Some(seconds) = self.max_age {
            line.push_str("; Max-Age=");
            line.push_str(&seconds.to_string());
        }
        if self.secure {
            line.push_str("; Secure");
        }
        if self.http_only {
            line.push_str("; HttpOnly");
        }
        line.push_str("; SameSite=");
        line.push_str(self.same_site.as_str());
        line
    }
}

/// `rule:errors/cookie-name-bytes`'s stricter prefix: `Secure`, `Path=/`, and no `Domain`.
const HOST_PREFIX: &str = "__Host-";

/// `rule:errors/cookie-name-bytes`'s other prefix: `Secure` alone.
const SECURE_PREFIX: &str = "__Secure-";

/// The stem every one of this member's refusals opens with — one literal, which
/// `conformance_coverage`'s error-path gate reads as one site, exactly as
/// `setHeader`'s two refusals share theirs.
const ADD_COOKIE: &str = "Core\\Response::addCookie()";

/// The four defaults of `rule:http-server/cookies-are-secure-httponly-and-lax`
/// as they stand for this request.
///
/// A program with no configuration at all still gets them: `Cookies::of(None)`
/// is `rule:http-server/cookies-are-secure-httponly-and-lax`'s shipped set, which is the answer a `nvs.toml`-less run
/// should have. Throwing there — `Core\Queue::push`'s arrangement over
/// `[queue]` — would be wrong here, because a missing `[queue]` block means the
/// deployment runs no jobs while a missing `[http.cookies]` block means it
/// accepted the defaults.
pub(crate) fn configured_cookies(ctx: &nvs_runtime::Ctx) -> nvs_config::http::Cookies {
    ctx.config().map_or_else(
        || nvs_config::http::Cookies::of(None),
        |config| nvs_config::http::Cookies::of(config.snapshot().config.http.as_ref()),
    )
}

/// One `bool` option, or the configured default when the call site said nothing.
fn flag_of(args: &[Value], at: usize, option: &str, configured: bool) -> Result<bool, Fault> {
    if matches!(args[at].tag(), Some(Tag::Null)) {
        return Ok(configured);
    }
    args[at].as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "{ADD_COOKIE}: expected a `bool` for `{option}`, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// One text option as written, or [`None`] when the call site said nothing.
fn text_of<'a>(args: &'a [Value], at: usize, option: &str) -> Result<Option<&'a str>, Fault> {
    if matches!(args[at].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    args[at].as_text().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "{ADD_COOKIE}: expected a `string` for `{option}`, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// [`SAME_SITE`]'s case as `nvs_config`'s, or [`None`] for an omitted option.
///
/// Written out rather than cast off the ordinal, on [`redirect_status`]'s
/// reasoning: the two enums coincide in order today and a fourth case added to
/// either would otherwise arrive on the wire as whichever case shares its
/// number.
fn same_site_of(value: &Value) -> Result<Option<nvs_config::http::SameSite>, Fault> {
    use nvs_config::http::SameSite;
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    match value.as_int() {
        Some(0) => Ok(Some(SameSite::Lax)),
        Some(1) => Ok(Some(SameSite::Strict)),
        Some(2) => Ok(Some(SameSite::None)),
        _ => Err(Fault::fatal(format!(
            "{ADD_COOKIE}: expected a `Core\\Response::SameSite` case, got tag {}",
            value.tag_byte()
        ))),
    }
}

/// `maxAge` in whole seconds, or [`None`] for the session cookie an omitted
/// option means.
///
/// A negative duration throws rather than being sent: browsers read `Max-Age`
/// below zero as "delete this now", which `0s` already spells, so a negative
/// one is a program that computed a lifetime backwards and would silently get
/// the deletion instead. Sub-second lifetimes truncate toward zero, `Max-Age`
/// being defined in seconds — `500ms` is `Max-Age=0`, which is that same
/// deletion and is the honest reading of a cookie that expires before it
/// arrives.
fn max_age_of(args: &[Value]) -> Result<Option<i64>, Fault> {
    if matches!(args[MAX_AGE_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, MAX_AGE_ARG, "addCookie")?;
    if nanos < 0 {
        return Err(Fault::thrown_as(
            nvs_runtime::ThrownClass::Logic,
            format!(
                "{ADD_COOKIE}: `maxAge` cannot be negative, and this one is {nanos}ns — a \
                 browser reads a negative `Max-Age` as a deletion, which `0s` already says"
            ),
        ));
    }
    Ok(Some(nanos / 1_000_000_000))
}

/// Whether `value` is bytes a cookie value can carry — RFC 6265's `cookie-octet`.
///
/// [`carriable`]'s printable-ASCII rule less the five bytes that end a value:
/// a space and a tab end it, a semicolon begins the next attribute, and a
/// comma, a backslash and a double quote are what proxies and parsers have
/// historically disagreed about. Refusing all five is what makes this member's
/// framing its own — which is in turn why the `Set-Cookie` line below is built
/// by concatenation and needs no escape anywhere.
fn cookieable(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| (0x21..=0x7e).contains(&byte) && !matches!(byte, b'"' | b',' | b';' | b'\\'))
}

/// Whether `value` is bytes a `Path` or a `Domain` attribute can carry.
///
/// Non-empty, and [`carriable`] less the semicolon that would begin an
/// attribute the program never wrote. Wider than [`cookieable`] on purpose: a
/// path holds `/` and `=` legitimately, and neither ends an attribute.
fn attributable(value: &str) -> bool {
    !value.is_empty() && carriable(value) && !value.contains(';')
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::addCookie(string $name, string $value, {secure?: bool,
    /// httpOnly?: bool, sameSite?: Core\Response::SameSite, path?: string,
    /// domain?: string, maxAge?: Core\Time\Duration}): void` — a
    /// cookie, replacing `setcookie` and `setrawcookie` both.
    ///
    /// **One member where PHP has two, because the difference between them was
    /// an escaping decision and this member does not have one to make.**
    /// `setcookie` URL-encoded the value and `setrawcookie` did not, so every
    /// call site chose between a value the next read had to decode and a value
    /// that could close the header. Here the value is checked against RFC
    /// 6265's own byte set ([`cookieable`]) and written verbatim: what goes out
    /// is what comes back, and nothing that could end the attribute gets in.
    ///
    /// **Appends, and that is the whole reason it is not `setHeader`.** A
    /// response carries as many cookies as it was told to, so this writes
    /// through [`nvs_runtime::Ctx::append_header`] — whose own doc owns why a
    /// scan would be wrong here — and a name written twice reaches the peer
    /// twice.
    ///
    /// **Every option not written comes from `[http.cookies]`**, which is what
    /// makes a bare `addCookie($name, $value)` a `Secure; HttpOnly;
    /// SameSite=Lax; Path=/` cookie. `nvs_config::http::Cookies` owns those
    /// defaults; this member owns none of them, so a deployment changes its
    /// cookie policy in one block rather than at every call site.
    ///
    /// **`rule:errors/cookie-name-bytes`'s prefixes are enforced here rather than documented.**
    /// `__Host-` requires `Secure` and `Path=/` and forbids `Domain`;
    /// `__Secure-` requires `Secure`. Both are refused on write, which is the
    /// half of that rule this class owns — the read half is `Core\Request`'s.
    /// A prefix each call site has to remember is what produced CVE-2024-2756.
    ///
    /// **What it spends:** one `String` per call, the rendered line, moved into
    /// the context's header list and freed with the request. The four
    /// configured defaults are read off the request's own snapshot, which is an
    /// `Arc` every request on the core already shares.
    fn nvs_core_response_add_cookie(ctx, args: [8]) {
        head_open(ctx, "Core\\Response::addCookie")?;
        // Unreachable from source for both: the row types them `CoreTy::Text`,
        // so `E0401` refuses a non-`string` — and a `tainted` one besides,
        // both being `rule:security/sink-predicate` sinks.
        let name = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{ADD_COOKIE}: expected a `string` for the name, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let value = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{ADD_COOKIE}: expected a `string` for the value, got tag {}",
                args[1].tag_byte()
            ))
        })?;

        let configured = configured_cookies(ctx);
        let secure = flag_of(args, SECURE_ARG, "secure", configured.secure)?;
        let http_only = flag_of(args, HTTP_ONLY_ARG, "httpOnly", configured.http_only)?;
        let same_site = same_site_of(&args[SAME_SITE_ARG])?.unwrap_or(configured.same_site);
        let path = text_of(args, PATH_ARG, "path")?.unwrap_or(configured.path.as_str());
        let domain = text_of(args, DOMAIN_ARG, "domain")?;
        let max_age = max_age_of(args)?;

        if !nameable(name) {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{ADD_COOKIE}: `{name}` is not a cookie name — a name is a non-empty \
                     token, and it is matched byte for byte on the way back, so nothing \
                     here is substituted into something that would be"
                ),
            ));
        }
        if !cookieable(value) {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{ADD_COOKIE}: `{value}` is not a value a cookie can carry — a space, a \
                     comma, a semicolon, a backslash, a quote and anything outside printable \
                     ASCII would end the value and begin an attribute the program never wrote"
                ),
            ));
        }
        if !attributable(path) {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{ADD_COOKIE}: `{path}` is not a `Path` — a path is non-empty printable \
                     ASCII with no semicolon in it, whether it was written here or read from \
                     `[http.cookies] path`"
                ),
            ));
        }
        if let Some(domain) = domain
            && !attributable(domain)
        {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{ADD_COOKIE}: `{domain}` is not a `Domain` — a domain is non-empty \
                     printable ASCII with no semicolon in it"
                ),
            ));
        }
        // `rule:http-server/cookies-are-secure-httponly-and-lax`'s pair, at the call site rather than only at boot: the
        // configured half is refused as `E0624`'s neighbour `E0612`, and this
        // is the same combination arrived at one option at a time. Browsers
        // drop it either way, so a cookie written like this is never stored.
        if same_site == nvs_config::http::SameSite::None && !secure {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{ADD_COOKIE}: `sameSite: SameSite::None` needs `secure: true`, and this \
                     cookie has `secure: false` — a browser drops the pair rather than \
                     honouring it, so the cookie would never be stored"
                ),
            ));
        }
        // `rule:errors/cookie-name-bytes`, on write. The three conditions are named together
        // because a cookie failing any of them is invisible on read, and a
        // write that succeeded into an invisible cookie is the failure mode
        // that rule exists to remove.
        if name.starts_with(HOST_PREFIX) && (!secure || path != "/" || domain.is_some()) {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{ADD_COOKIE}: `{name}` carries the `__Host-` prefix, which the runtime \
                     enforces rather than documents: it requires `secure: true`, requires \
                     `path: \"/\"` and forbids `domain`. A cookie that does not conform is \
                     not visible on read, so it is refused on write"
                ),
            ));
        }
        if name.starts_with(SECURE_PREFIX) && !secure {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{ADD_COOKIE}: `{name}` carries the `__Secure-` prefix, which requires \
                     `secure: true`. A cookie that does not conform is not visible on read, \
                     so it is refused on write"
                ),
            ));
        }

        // Rendered by [`Cookie::line`], which is the four checks above cashed
        // in: every part is already known to hold no byte that could end the
        // part it is in, so nothing is escaped anywhere.
        let line = Cookie {
            name,
            value,
            path,
            domain,
            max_age,
            secure,
            http_only,
            same_site,
        }
        .line();
        ctx.append_header(SET_COOKIE_HEADER, &line);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::json(mixed $value): void` — `rule:security/response-body-is-one-typed-member`'s second row.
    ///
    /// The same encoder `Core\Json::encode` is, reached through the same
    /// [`crate::json::Encodable`] rather than through a second walk: one
    /// serializer is what makes the claim of
    /// `rule:security/response-body-is-one-typed-member` about tainted values
    /// true, since a
    /// second one could frame a string differently and there would be nothing
    /// to compare it against. `pretty` is not an option here — a response body
    /// is read by a program, and whitespace for a human belongs to whatever is
    /// showing it to one.
    ///
    /// The declaration is made only once the value has serialized, unlike
    /// `text`'s: an unencodable value throws with nothing written, so there is
    /// no body for a `Content-Type` to have described.
    fn nvs_core_response_json(ctx, args: [1]) {
        head_open(ctx, "Core\\Response::json")?;
        let written = crate::json::written(ctx, args[0], "Core\\Response::json")?;
        ctx.declare_content_type(JSON_MEDIA_TYPE);
        // Unreachable from source, on `text`'s reasoning: `OutputSink::Buffer`
        // and `Sink` never fail, which `Ctx::write_output`'s own `# Errors`
        // states, and nothing in the language closes a descriptor the host
        // handed the process.
        ctx.write_body(written.as_bytes())
            .map_err(|error| Fault::fatal(format!("Core\\Response::json could not write: {error}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::bytes(bytes $body, string $contentType): void` — the
    /// bytes member of `rule:security/response-body-is-one-typed-member`, and
    /// the only body member that is *told* its media type.
    ///
    /// The type is checked before anything is written, which is the opposite
    /// order from `text`'s and for the same reason `json`'s is: a refusal that
    /// has already put the body on the wire is not a refusal. [`spellable`]
    /// owns what the check is and why it is here and not only at the
    /// connection.
    fn nvs_core_response_bytes(ctx, args: [2]) {
        head_open(ctx, "Core\\Response::bytes")?;
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
        // — and refuses a `tainted` one besides, that being a
        // `rule:security/sink-predicate` sink.
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
        ctx.write_body(body)
            .map_err(|error| Fault::fatal(format!("Core\\Response::bytes could not write: {error}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::sendFile(string $path): void` — the file row of
    /// `rule:security/response-body-is-one-typed-member`, the one body member whose bytes do not pass through this call.
    ///
    /// **A name is what it leaves**, and that is the whole shape:
    /// [`nvs_runtime::Ctx::declare_file_body`] records the path, the request's
    /// finish path lifts it onto a `Completion`, and the server opens the file
    /// and streams it under the static-file policy — which is where the media
    /// type, the range and the conditional already live
    /// (`rule:security/response-body-is-one-typed-member`). So a response of any size costs a
    /// request one path and one open handle at the connection, never a copy of
    /// the file, and nothing here declares a content type: this member cannot
    /// know what the bytes are called and the policy's table can.
    ///
    /// **The three refusals are asked here rather than at the connection**, on
    /// [`spellable`]'s reasoning: a program learns that the file it named is
    /// missing, is a directory, or is one the process may not read at the call
    /// it made, where it can still answer something else, instead of from a
    /// `500` after its handler returned. The grant is the first of them and
    /// comes from the door, so a path outside `fs.read` is refused whether or
    /// not there is a file at it — `nvs_runtime::capability::open_read` owns why
    /// that order is the one that cannot be used as a probe. What the server
    /// does with the name afterwards is checked again by the policy, this
    /// member's answer being about the program's authority and never about the
    /// request's.
    ///
    /// **Off a request there is no response, and then the bytes land here**:
    /// a CLI program, a `#[Test]` method, a `.nvst` case and a `spawn script`
    /// child each write the file into their own output, a chunk at a time. That
    /// is the module doc's *A body goes out verbatim* section for the one member
    /// whose body is a file — the declaration means nothing where nobody frames
    /// a response, and the bytes still come out in the order the program wrote
    /// them.
    fn nvs_core_response_send_file(ctx, args: [1]) {
        head_open(ctx, "Core\\Response::sendFile")?;
        // Unreachable from source: the row's parameter is a `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs —
        // and refuses a `tainted` one besides, that being a
        // `rule:security/sink-predicate` sink.
        let named = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::sendFile expected a `string` for the path, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let path = std::path::Path::new(named);
        let found = nvs_runtime::capability::metadata(ctx, path, "Core\\Response::sendFile")?;
        if !found.is_file() {
            // A literal stem before the first hole, which is
            // `conformance_coverage`'s error-path gate matching a site.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::sendFile(): `{named}` is not a regular file — a response \
                     body is a file's contents, and a directory has none to send"
                ),
            ));
        }
        if !nvs_runtime::capability::readable(ctx, path, "Core\\Response::sendFile")? {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Io,
                format!(
                    "Core\\Response::sendFile(): `{named}` is granted but this process may not \
                     read it"
                ),
            ));
        }
        if ctx.declare_file_body(path) {
            return Ok(Value::null());
        }
        let mut file = nvs_runtime::capability::open_read(ctx, path, "Core\\Response::sendFile")?;
        let mut chunk = [0_u8; SEND_FILE_CHUNK];
        loop {
            let read = std::io::Read::read(&mut file, &mut chunk).map_err(|error| {
                nvs_runtime::capability::io_failure("Core\\Response::sendFile", path, &error)
            })?;
            if read == 0 {
                break;
            }
            // Unreachable from source, on `text`'s reasoning: `OutputSink::Buffer`
            // and `Sink` never fail, and nothing in the language closes a
            // descriptor the host handed the process.
            ctx.write_output(&chunk[..read]).map_err(|error| {
                Fault::fatal(format!("Core\\Response::sendFile could not write: {error}"))
            })?;
        }
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response::stream(string $contentType, ?int $length = null,
    /// ?string $etag = null, ?string $mustEnd = null): Core\Response\Stream` —
    /// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
    /// stream that ends with its response, as the member that opens one.
    ///
    /// **A stated length gets the file policy's decision**
    /// (`rule:http-server/static-serving-is-one-policy`): `If-None-Match`,
    /// `If-Range` and `Range` are read off the request and
    /// [`nvs_runtime::conditional::decide`] answers which bytes the body is.
    /// Its status wins over the program's except where it is `200`, and its
    /// headers go first so the program's own apply after them, as for a file
    /// body. The window it gives is put on the context, where `offset`,
    /// `length` and `write` read it. Off a connection there is no `Range`, so
    /// the window is the whole length.
    ///
    /// Three effects where its neighbours have two: the media type is declared
    /// on this request's context exactly as every other body member declares
    /// one, the writing half of `nvs_runtime::stream` is put on that context for
    /// [`nvs_core_response_stream_write`] to find, and the head — the media type
    /// beside the reading half — is left in the cell the connection is watching.
    ///
    /// **Off a connection there is no cell, and then this member is inert**, on
    /// the reading the module doc's *A body goes out verbatim* section already
    /// gives a declaration made where no response is being framed: a CLI
    /// program, a `#[Test]` method and a `.nvst` case each open a stream that
    /// writes to their own output, and the bytes come out in the order they were
    /// written. That is the same fallback `text` and `bytes` have and not a
    /// second one — it is what lets a case assert a streamed body's bytes at
    /// all, since no case is a request a server is answering.
    ///
    /// A second stream on one request is refused, because a response has one
    /// body and the connection has already been told what the first one is.
    /// Where there is no cell there is nothing that refusal could be about, and
    /// the later declaration simply wins — `Ctx::declare_content_type`'s own
    /// rule for every body member written twice.
    ///
    /// **`$mustEnd` is put on the context too**, on a connection and off one:
    /// the finish path reads it to cut off and log a body the program never
    /// ended (`nvs_runtime::Ctx::end_unfinished_stream`).
    fn nvs_core_response_stream(ctx, args: [4]) {
        head_open(ctx, "Core\\Response::stream")?;
        // Unreachable from source: the row's parameter is a `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs — and
        // refuses a `tainted` one besides, that being a
        // `rule:security/sink-predicate` sink.
        let media_type = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Response::stream expected a `string` for the content type, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        if !spellable(media_type) {
            // A literal stem before the first hole, which is
            // `conformance_coverage`'s error-path gate matching a site.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "Core\\Response::stream(): `{media_type}` is not a media type a \
                     `Content-Type` header can carry — a field value is printable ASCII \
                     and never empty"
                ),
            ));
        }
        let length = stated_length(&args[1])?;
        let etag = stated_etag(&args[2])?;
        let must_end = stated_label(&args[3])?;
        // Cloned rather than borrowed, for `Core\Sse::upgrade`'s reason: the
        // cell is a shared handle by construction, so a clone is one refcount
        // and no borrow of the carrier held across the write below.
        let cell = ctx
            .inbound()
            .and_then(nvs_runtime::Inbound::response_stream_slot)
            .cloned();
        if let Some(cell) = cell {
            // The declarations go with the head, because the head is on the
            // wire from here: a status or a header set before this call reaches
            // the peer, and one set after it reaches nothing at all. Taken
            // rather than read, so the completion this request files carries
            // none of them and the connection does not apply a header twice.
            let status = ctx.take_status();
            let headers = ctx.take_headers();
            let opened = match length {
                None => cell.open(media_type, status, headers),
                Some(total) => {
                    let asked = ctx
                        .inbound()
                        .map(|inbound| {
                            nvs_runtime::conditional::Asked::from_fields(inbound.headers())
                        })
                        .unwrap_or_default();
                    let ranged = nvs_runtime::conditional::ranged(asked, etag.as_deref(), total);
                    cell.open_ranged(media_type, status, headers, ranged)
                        .map(|(emit, window)| {
                            ctx.set_stream_window(Some(window));
                            emit
                        })
                }
            };
            let emit = opened.ok_or_else(|| {
                // No case can reach this: a `.nvst` case runs a script no
                // connection is framing a response for, so it is offered no cell
                // and never gets here.
                // `a_second_stream_on_one_request_is_refused_and_the_first_still_stands`
                // is the `#[test]` that asserts it instead.
                Fault::thrown_as(
                    nvs_runtime::ThrownClass::Logic,
                    "Core\\Response::stream(): this request has already opened a response \
                     body stream, and a response has one body",
                )
            })?;
            ctx.set_body_stream(emit);
        } else {
            ctx.set_stream_window(length.map(|total| nvs_runtime::stream::Window::new(0, total)));
        }
        ctx.set_stream_must_end(must_end);
        ctx.declare_content_type(media_type);
        Ok(crate::instance::build(&STREAM, []))
    }
}

/// `Core\Response::stream`'s `$mustEnd`, and `None` where it was not given.
fn stated_label(value: &Value) -> Result<Option<Box<str>>, Fault> {
    if matches!(value.tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(None);
    }
    // Unreachable from source: the row's parameter is a `?string`.
    let label = value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Response::stream expected a `string` for `$mustEnd`, got tag {}",
            value.tag_byte()
        ))
    })?;
    Ok(Some(label.into()))
}

/// `Core\Response::stream`'s `$length`, and `None` where it was not given.
fn stated_length(value: &Value) -> Result<Option<u64>, Fault> {
    if matches!(value.tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(None);
    }
    // Unreachable from source: the row's parameter is a `?int`.
    let length = value.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Response::stream expected an `int` for the length, got tag {}",
            value.tag_byte()
        ))
    })?;
    u64::try_from(length).map(Some).map_err(|_| {
        Fault::thrown_as(
            nvs_runtime::ThrownClass::Logic,
            format!(
                "Core\\Response::stream(): the length {length} is negative. It must be 0 or more."
            ),
        )
    })
}

/// `Core\Response::stream`'s `$etag`, quoted as the strong tag the response
/// sends, and `None` where it was not given.
fn stated_etag(value: &Value) -> Result<Option<String>, Fault> {
    if matches!(value.tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(None);
    }
    // Unreachable from source: the row's parameter is a `?string`.
    let etag = value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Response::stream expected a `string` for the etag, got tag {}",
            value.tag_byte()
        ))
    })?;
    if !carriable(etag) || etag.contains('"') {
        return Err(Fault::thrown_as(
            nvs_runtime::ThrownClass::Logic,
            format!(
                "Core\\Response::stream(): the etag `{etag}` cannot be sent in an `ETag` header. \
                 It must be printable ASCII and must not contain `\"`."
            ),
        ));
    }
    Ok(Some(format!("\"{etag}\"")))
}

/// The window of the stream `member` was called on, or the `LogicError` a
/// stream without a stated length throws.
fn window_of(
    ctx: &mut nvs_runtime::Ctx,
    member: &str,
) -> Result<nvs_runtime::stream::Window, Fault> {
    ctx.stream_window().ok_or_else(|| {
        Fault::thrown_as(
            nvs_runtime::ThrownClass::Logic,
            format!(
                "Core\\Response\\Stream::{member}(): the stream was started without a `$length`."
            ),
        )
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Response\Stream::offset(): int` — where the window a stated
    /// length gave this stream starts.
    fn nvs_core_response_stream_offset(ctx, args: [1]) {
        crate::instance::receiver(args[0], &STREAM, "offset")?;
        let window = window_of(ctx, "offset")?;
        Ok(Value::int(i64::try_from(window.offset()).unwrap_or(i64::MAX)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response\Stream::length(): int` — how many bytes the window a
    /// stated length gave this stream is.
    fn nvs_core_response_stream_length(ctx, args: [1]) {
        crate::instance::receiver(args[0], &STREAM, "length")?;
        let window = window_of(ctx, "length")?;
        Ok(Value::int(i64::try_from(window.length()).unwrap_or(i64::MAX)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response\Stream::end(): void` — ends the body now, which a stream
    /// opened with `$mustEnd` needs before the program stops.
    fn nvs_core_response_stream_end(ctx, args: [1]) {
        crate::instance::receiver(args[0], &STREAM, "end")?;
        ctx.end_body_stream();
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Response\Stream::write(string|bytes $chunk): void` — one chunk of a
    /// body being written over time.
    ///
    /// Where the chunk goes is the receiver's context rather than the receiver:
    /// [`STREAM`]'s own docs own why the handle carries no state, and this body
    /// is the whole of what that costs — one read of the context to tell a
    /// streaming response from a buffered one.
    ///
    /// **The write parks while the connection still holds the last chunk**, so
    /// a program that produces faster than the peer reads is slowed by the peer
    /// and nothing accumulates in between (`nvs_runtime::stream`). A peer that
    /// has stopped reading altogether meets the connection's send timeout, which
    /// closes the stream: `rule:concurrency/connection-bounds-are-finite`'s
    /// defined close rather than a wait with no end.
    ///
    /// **A client that has gone is not the program's error.** A closed stream
    /// and a send timeout both mean the client left, so the write returns at
    /// once and its bytes are discarded, and the request runs on to its end
    /// (`rule:http-server/a-request-outlives-a-client-that-goes-away`). What
    /// still throws is the program's own mistake: a chunk over the message
    /// bound, or a write from outside the task that owns the stream.
    fn nvs_core_response_stream_write(ctx, args: [2]) {
        crate::instance::receiver(args[0], &STREAM, "write")?;
        // Unreachable from source: the row's parameter is a union of exactly
        // these two spellings, so `E0401` refuses every other one before this
        // runs. The two readers are separate on purpose — `Value::as_bytes`
        // answers `None` for text, so a member meaning either has to ask twice.
        let chunk = args[1]
            .as_str_bytes()
            .or_else(|| args[1].as_bytes())
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Response\\Stream::write expected a `string` or `bytes`, got tag {}",
                    args[1].tag_byte()
                ))
            })?;
        if ctx.stream_ended() {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                "Core\\Response\\Stream::write(): the stream has ended, because `end()` was \
                 called. Nothing was sent.",
            ));
        }
        // The count is the program's, so a write past the window throws before
        // any of it is sent, on a connection and off one alike.
        ctx.claim_stream_bytes(chunk.len()).map_err(|past| {
            Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!("Core\\Response\\Stream::write(): {past}"),
            )
        })?;
        if let Some(emit) = ctx.body_stream() {
            // Owned, because the connection takes the chunk rather than reading
            // it: the copy is the one allocation a streamed chunk costs, and it
            // is what lets the writing task go on while the bytes are still on
            // their way to the wire.
            //
            // No case can reach the client leaving: a stream that can close is
            // one a connection is draining, and a `.nvst` case is offered no
            // cell. `a_stream_write_after_the_client_left_returns_without_throwing`
            // and `a_stream_write_after_the_send_timeout_returns_without_throwing`
            // are the `#[test]`s that assert it instead.
            return match emit.send(chunk.to_vec()) {
                Ok(()) => Ok(Value::null()),
                Err(left)
                    if *left == *nvs_runtime::stream::CLOSED
                        || *left == *nvs_runtime::stream::SEND_TIMED_OUT =>
                {
                    Ok(Value::null())
                }
                // No case can reach this either, for the same reason: only an
                // offered cell has a message bound to go over.
                // `a_stream_write_over_the_message_bound_throws` asserts it.
                Err(refused) => Err(Fault::thrown(format!(
                    "Core\\Response\\Stream::write(): {refused}"
                ))),
            };
        }
        // Unreachable from source, on `text`'s reasoning: `OutputSink::Buffer`
        // and `Sink` never fail, which `Ctx::write_output`'s own `# Errors`
        // states, and nothing in the language closes a descriptor the host
        // handed the process.
        ctx.write_body(chunk).map_err(|error| {
            Fault::fatal(format!("Core\\Response\\Stream::write could not write: {error}"))
        })?;
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::{HTML_MEDIA_TYPE, JSON_MEDIA_TYPE, TEXT_MEDIA_TYPE};
    use nvs_runtime::{Ctx, NvsStr, OutputSink, Value, call};

    /// One body member's whole effect on the response *head*, which is the one
    /// word `rule:security/response-body-is-one-typed-member` puts there: the media type it declared, read back off
    /// the context the isolate's finish path takes it from.
    ///
    /// A fresh context per call, because the claim is that each member declares
    /// its own rather than that the last one to run wins — sharing one would
    /// assert the opposite by construction.
    fn declared(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        args: &[Value],
    ) -> Option<Box<str>> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        call(member, &mut ctx, args).expect("a body member with a well-formed argument answers");
        ctx.take_content_type()
    }

    /// Drops a reference this module built and the borrowing member did not
    /// take — `arr`'s tests own theirs the same way.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "this test owns the reference it built, and a `Core` member \
                      borrows its arguments rather than consuming them"
        )]
        unsafe {
            value.release();
        }
    }

    /// `rule:security/response-body-is-one-typed-member`'s table, asserted as a table: each landed body member owns
    /// one body shape and sets **its own** `Content-Type`, which is the whole
    /// argument for five members rather than one `write`.
    ///
    /// Three claims rather than one row each, because a member that answered a
    /// neighbour's media type would still read plausibly on its own line: the
    /// declarations are asserted against the column of
    /// `rule:security/response-body-is-one-typed-member`, asserted to be
    /// *distinct*, and — for the one member that is told its type — asserted to
    /// carry two different ones rather than a constant that happened to match.
    ///
    /// `sendFile` is that rule's one row that is not here, and not for want of having
    /// landed: it declares no media type at all, the static-file policy's table
    /// naming what a file's bytes are, so there is nothing of this claim to ask
    /// it. What that member leaves instead is a name, which
    /// `send_file_leaves_a_name_on_a_request_and_writes_the_bytes_off_one`
    /// asserts.
    /// The sweep below is over the table rather than over a list of names, so a
    /// member that does declare one joins by being added to it.
    // covers: Core\Response::bytes
    // covers: Core\Response::html
    // covers: Core\Response::json
    // covers: Core\Response::text
    #[test]
    fn each_body_member_sets_its_own_content_type() {
        // Nothing else on the path declares one: a context no body member has
        // answered on carries no media type at all, so every declaration below
        // is that member's own act and not a default read back.
        let mut untouched = Ctx::new(OutputSink::Sink);
        assert_eq!(untouched.take_content_type(), None);

        let body = Value::text("a paragraph");
        let blob = Value::bytes(NvsStr::new(b"\x89PNG"));
        let told = Value::text("application/octet-stream");
        // The carrier itself, built the way `crate::html` builds one: `html`'s
        // parameter is the class and never a `string`, so a test handing it
        // text would assert something no call site can write.
        let markup =
            crate::instance::build(&crate::html::MARKUP, [Value::text("<p>a paragraph</p>")]);

        let table = [
            (
                "html",
                declared(super::nvs_core_response_html, &[markup]),
                HTML_MEDIA_TYPE,
            ),
            (
                "text",
                declared(super::nvs_core_response_text, &[body]),
                TEXT_MEDIA_TYPE,
            ),
            (
                "json",
                declared(super::nvs_core_response_json, &[Value::int(1)]),
                JSON_MEDIA_TYPE,
            ),
            (
                "bytes",
                declared(super::nvs_core_response_bytes, &[blob, told]),
                "application/octet-stream",
            ),
        ];
        for (member, said, expected) in &table {
            assert_eq!(
                said.as_deref(),
                Some(*expected),
                "Core\\Response::{member} declares its own media type"
            );
        }

        // And they are its own: two members answering one type is the endpoint
        // that serves JSON labelled as HTML, which is what
        // `rule:security/response-body-is-one-typed-member` exists to make
        // unwritable. Counted rather than compared pairwise, so another row
        // added above is covered by this line as it stands.
        let distinct: std::collections::BTreeSet<_> =
            table.iter().map(|(_, said, _)| said.clone()).collect();
        assert_eq!(
            distinct.len(),
            table.len(),
            "each body member declares a media type no other one does: {table:?}"
        );

        // `bytes` is the row that is *told*, so its declaration is asserted on
        // a second value: a member that declared a constant matching the first
        // one would pass every line above.
        let other = Value::text("image/png");
        assert_eq!(
            declared(super::nvs_core_response_bytes, &[blob, other]).as_deref(),
            Some("image/png"),
            "Core\\Response::bytes carries the content type it was given"
        );

        dropped(markup);
        dropped(body);
        dropped(blob);
        dropped(told);
        dropped(other);
    }

    /// `rule:http-server/cookies-are-secure-httponly-and-lax`, at the member that owns it: a cookie written with no options bag carries
    /// `Secure`, `HttpOnly`, `SameSite=Lax` and `Path=/`. The four come off `[http.cookies]`, and a
    /// context with no configuration attached is the tree that wrote no such block — which is the
    /// case the rule is a statement about, since a deployment that configured the block chose its own.
    ///
    /// The whole line is compared rather than four `contains` calls, because the failure this is
    /// written against is an attribute that went *missing*: every `contains` still passes on a line
    /// that grew a fifth attribute nobody asked for, and on one whose parts fell in an order no peer
    /// parses. `Path` first and `SameSite` last is the render's own order and is not the rule's claim —
    /// what the rule claims is that all four are there with nothing configured.
    // covers: Core\Response::addCookie
    #[test]
    fn a_cookie_is_secure_httponly_samesite_lax_by_default() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        assert!(
            ctx.config().is_none(),
            "the defaults are the ones a tree with no `[http.cookies]` resolves to"
        );

        let name = Value::text("session");
        let value = Value::text("abc123");
        // Six nulls: `secure`, `httpOnly`, `sameSite`, `path`, `domain` and `maxAge`, every one of
        // them left out, which is the call site this case is about.
        let args = [
            name,
            value,
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
        ];
        call(super::nvs_core_response_add_cookie, &mut ctx, &args)
            .expect("a cookie whose name and value are both well formed is written");

        let headers = ctx.take_headers();
        assert_eq!(
            headers.len(),
            1,
            "one call writes one cookie and nothing else: {headers:?}"
        );
        assert_eq!(&*headers[0].name, "Set-Cookie");
        assert_eq!(
            &*headers[0].value,
            "session=abc123; Path=/; Secure; HttpOnly; SameSite=Lax",
        );
        // Appends rather than replaces, which is the reason this member is not
        // `setHeader`: a second cookie may not silently take the first one's place.
        assert!(headers[0].append);

        dropped(name);
        dropped(value);
    }

    /// Every option written at once, each one landing as its own attribute, and a
    /// second call appending a second line — the half of `addCookie` no program
    /// can read back, since a response head is not visible from Novis.
    ///
    /// The lifetime is the edge: `Max-Age` is whole seconds, so `90.5s` is sent as
    /// `90` and a negative one throws with nothing written, which is the order
    /// `max_age_of`'s doc states.
    // covers: Core\Response::addCookie
    #[test]
    fn every_written_option_is_one_attribute_and_each_call_is_one_line() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let duration =
            |nanos: i64| crate::instance::build(&crate::time::DURATION, [Value::int(nanos)]);

        let theme = Value::text("theme");
        let dark = Value::text("dark");
        let path = Value::text("/account");
        let domain = Value::text("example.com");
        let lifetime = duration(90_500_000_000);
        // `secure: true`, `httpOnly: false`, `sameSite: Strict` (case 1), then
        // the path, the domain and the lifetime.
        let written = [
            theme,
            dark,
            Value::bool(true),
            Value::bool(false),
            Value::int(1),
            path,
            domain,
            lifetime,
        ];
        call(super::nvs_core_response_add_cookie, &mut ctx, &written)
            .expect("a cookie with every option well formed is written");

        let sid = Value::text("sid");
        let token = Value::text("r4nd0m");
        let bare = [
            sid,
            token,
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
        ];
        call(super::nvs_core_response_add_cookie, &mut ctx, &bare)
            .expect("a second cookie is written beside the first");

        let backwards = duration(-1_000_000_000);
        let refused = [
            sid,
            token,
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
            Value::null(),
            backwards,
        ];
        assert!(
            call(super::nvs_core_response_add_cookie, &mut ctx, &refused).is_err(),
            "a negative `maxAge` throws"
        );
        assert!(
            ctx.take_pending()
                .is_some_and(|message| message.contains("cannot be negative"))
        );

        let headers = ctx.take_headers();
        let lines: Vec<&str> = headers.iter().map(|header| &*header.value).collect();
        assert_eq!(
            lines,
            [
                "theme=dark; Path=/account; Domain=example.com; Max-Age=90; Secure; SameSite=Strict",
                "sid=r4nd0m; Path=/; Secure; HttpOnly; SameSite=Lax",
            ],
            "two calls write two lines, and the refused third writes none"
        );
        assert!(
            headers
                .iter()
                .all(|header| &*header.name == "Set-Cookie" && header.append)
        );

        for value in [theme, dark, path, domain, lifetime, sid, token, backwards] {
            dropped(value);
        }
    }

    /// The cell a connection offers, at both of its bounds: a send timeout no
    /// case below can reach, and room for every chunk one of them writes.
    /// `nvs_server::bounds` is where either number is the claim.
    fn offered_cell() -> nvs_runtime::stream::BodySlot {
        nvs_runtime::stream::BodySlot::new(std::time::Duration::from_secs(30), 1 << 20)
    }

    /// A context carrying the cell a connection offers, bounded generously
    /// enough that no case below can meet the send timeout — every claim here
    /// is about the seam and never about the clock.
    fn framing(slot: &nvs_runtime::stream::BodySlot) -> Ctx {
        let mut ctx = Ctx::buffered();
        let mut inbound = nvs_runtime::Inbound::new("GET", "/export", "");
        inbound.offer_response_stream(slot.clone());
        ctx.set_inbound(inbound);
        ctx
    }

    /// `Core\Response::stream`'s four arguments for `media_type`, with no
    /// length, no tag and no `$mustEnd`.
    fn unsized_stream(media_type: Value) -> [Value; 4] {
        [media_type, Value::null(), Value::null(), Value::null()]
    }

    /// `Core\Response::stream` with a length: the decision runs over the
    /// request's `Range`, the head states the window, `offset` and `length`
    /// give it, and a write past its end throws and sends nothing. Off a
    /// connection the window is the whole length. A negative length, a tag
    /// with a `"`, and `length` on a stream without one each throw.
    // covers: Core\Response::stream
    // covers: Core\Response\Stream::offset
    // covers: Core\Response\Stream::length
    // covers: Core\Response\Stream::write
    #[test]
    fn a_stream_with_a_length_writes_the_window_the_request_asked_for() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);
        let mut inbound = nvs_runtime::Inbound::new("GET", "/video", "");
        inbound.offer_response_stream(slot.clone());
        inbound.push_header("Range", b"bytes=2-5");
        ctx.set_inbound(inbound);

        let media_type = Value::text("video/mp4");
        let etag = Value::text("v1");
        let handle = call(
            super::nvs_core_response_stream,
            &mut ctx,
            &[media_type, Value::int(10), etag, Value::null()],
        )
        .expect("an offered cell takes the stream");
        let offset = call(super::nvs_core_response_stream_offset, &mut ctx, &[handle])
            .expect("a stream with a length has a window");
        let length = call(super::nvs_core_response_stream_length, &mut ctx, &[handle])
            .expect("a stream with a length has a window");
        assert_eq!((offset.as_int(), length.as_int()), (Some(2), Some(4)));

        let head = slot
            .take(std::task::Waker::noop())
            .expect("the member filled the cell");
        assert_eq!((head.status, head.length), (Some(206), Some(4)));
        let stated: Vec<(&str, &str)> = head
            .headers
            .iter()
            .map(|header| (&*header.name, &*header.value))
            .collect();
        assert_eq!(
            stated,
            [
                ("Accept-Ranges", "bytes"),
                ("ETag", "\"v1\""),
                ("Content-Range", "bytes 2-5/10")
            ]
        );

        let window = Value::text("2345");
        call(
            super::nvs_core_response_stream_write,
            &mut ctx,
            &[handle, window],
        )
        .expect("the window fits");
        let past = Value::text("6");
        call(
            super::nvs_core_response_stream_write,
            &mut ctx,
            &[handle, past],
        )
        .expect_err("a byte past the window throws");
        assert!(
            ctx.take_pending()
                .is_some_and(|message| message.contains("Nothing was sent"))
        );

        // Off a connection there is no `Range`, so the window is the whole body.
        let mut off = Ctx::buffered();
        let whole = call(
            super::nvs_core_response_stream,
            &mut off,
            &[media_type, Value::int(3), Value::null(), Value::null()],
        )
        .expect("a stream off a connection opens");
        let length = call(super::nvs_core_response_stream_length, &mut off, &[whole])
            .expect("a stream with a length has a window");
        assert_eq!(length.as_int(), Some(3));

        let mut refused = Ctx::buffered();
        let bad_tag = Value::text("v\"1");
        for args in [
            [media_type, Value::int(-1), Value::null(), Value::null()],
            [media_type, Value::int(1), bad_tag, Value::null()],
        ] {
            call(super::nvs_core_response_stream, &mut refused, &args)
                .expect_err("a negative length and a tag with a quote throw");
        }
        let chunked = call(
            super::nvs_core_response_stream,
            &mut refused,
            &unsized_stream(media_type),
        )
        .expect("a stream without a length opens");
        call(
            super::nvs_core_response_stream_offset,
            &mut refused,
            &[chunked],
        )
        .expect_err("a stream without a length has no window");

        for value in [
            media_type, etag, handle, window, past, whole, bad_tag, chunked,
        ] {
            dropped(value);
        }
    }

    /// `Core\Response::stream` opening the cell: after the call the connection's
    /// half holds the media type that was written, and a chunk the program
    /// wrote arrives on the reading half rather than in the request's output.
    ///
    /// The two are one claim and not two, because a member that declared the
    /// type and wrote the bytes somewhere else would pass either half alone:
    /// this is the seam `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`
    /// describes, asserted from the end that frames the response.
    // covers: Core\Response::stream
    // covers: Core\Response\Stream::write
    #[test]
    fn a_streamed_chunk_reaches_the_connections_half_and_not_the_requests_output() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);

        let media_type = Value::text("text/csv");
        let handle = call(
            super::nvs_core_response_stream,
            &mut ctx,
            &unsized_stream(media_type),
        )
        .expect("an offered cell takes the stream");
        let chunk = Value::text("id,name\n");
        call(
            super::nvs_core_response_stream_write,
            &mut ctx,
            &[handle, chunk],
        )
        .expect("the first chunk goes into an empty cell without parking");

        let mut head = slot
            .take(std::task::Waker::noop())
            .expect("the member filled the cell");
        assert_eq!(&*head.content_type, "text/csv");
        let nvs_runtime::stream::Drained::Chunk(framed) =
            head.drain.next_chunk(std::task::Waker::noop())
        else {
            panic!("the chunk the program wrote never reached the connection");
        };
        assert_eq!(framed, b"id,name\n");
        assert_eq!(
            ctx.take_buffered_output().unwrap_or_default(),
            b"",
            "a streamed chunk went to the request's own output as well"
        );

        dropped(media_type);
        dropped(chunk);
        dropped(handle);
    }

    /// A stream opened with `$mustEnd` is complete only after `end()`. Ended,
    /// the connection reads the end of the body, a later write throws, and the
    /// finish path has nothing to report. Not ended, the finish path cuts the
    /// body off and reports it — `rule:core-classes/spreadsheet-writes-as-a-stream`.
    /// A stream opened without one is never reported.
    // covers: Core\Response::stream
    // covers: Core\Response\Stream::end
    // covers: Core\Response\Stream::write
    #[test]
    fn a_stream_that_must_end_is_cut_off_unless_the_program_ends_it() {
        let media_type = Value::text("text/csv");
        let label = Value::text("orders.csv");
        let marked = [media_type, Value::null(), Value::null(), label];

        let slot = offered_cell();
        let mut ended = framing(&slot);
        let handle = call(super::nvs_core_response_stream, &mut ended, &marked)
            .expect("an offered cell takes the stream");
        let chunk = Value::text("id\n");
        call(
            super::nvs_core_response_stream_write,
            &mut ended,
            &[handle, chunk],
        )
        .expect("a write before `end()` is sent");
        call(super::nvs_core_response_stream_end, &mut ended, &[handle])
            .expect("`end()` ends the body");
        call(super::nvs_core_response_stream_end, &mut ended, &[handle])
            .expect("a second `end()` does nothing");
        call(
            super::nvs_core_response_stream_write,
            &mut ended,
            &[handle, chunk],
        )
        .expect_err("a write after `end()` throws");
        assert!(
            ended
                .take_pending()
                .is_some_and(|message| message.contains("`end()` was called"))
        );
        assert!(
            !ended.end_unfinished_stream(),
            "an ended stream is reported"
        );
        let mut head = slot
            .take(std::task::Waker::noop())
            .expect("the member filled the cell");
        assert!(matches!(
            head.drain.next_chunk(std::task::Waker::noop()),
            nvs_runtime::stream::Drained::Chunk(_)
        ));
        assert!(matches!(
            head.drain.next_chunk(std::task::Waker::noop()),
            nvs_runtime::stream::Drained::Ended
        ));

        let slot = offered_cell();
        let mut unended = framing(&slot);
        let forgotten = call(super::nvs_core_response_stream, &mut unended, &marked)
            .expect("an offered cell takes the stream");
        assert!(
            unended.end_unfinished_stream(),
            "the unended stream was not reported"
        );
        let mut head = slot
            .take(std::task::Waker::noop())
            .expect("the member filled the cell");
        assert!(matches!(
            head.drain.next_chunk(std::task::Waker::noop()),
            nvs_runtime::stream::Drained::Aborted
        ));

        let mut plain = Ctx::buffered();
        let unmarked = call(
            super::nvs_core_response_stream,
            &mut plain,
            &unsized_stream(media_type),
        )
        .expect("a stream off a connection opens");
        assert!(
            !plain.end_unfinished_stream(),
            "an unmarked stream was reported"
        );

        for value in [media_type, label, handle, chunk, forgotten, unmarked] {
            dropped(value);
        }
    }

    /// The cell's refusal reaching a program: a response has one body, so the
    /// second `stream` is told so and the first one is what the connection
    /// still frames. Unreachable from a `.nvst` case — a second stream needs a
    /// first, and a first needs a cell nothing offers a script.
    // covers: Core\Response::stream
    #[test]
    fn a_second_stream_on_one_request_is_refused_and_the_first_still_stands() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);

        let first = Value::text("text/csv");
        let handle = call(
            super::nvs_core_response_stream,
            &mut ctx,
            &unsized_stream(first),
        )
        .expect("an offered cell takes the first stream");
        let second = Value::text("application/json");
        call(
            super::nvs_core_response_stream,
            &mut ctx,
            &unsized_stream(second),
        )
        .expect_err("one response opens at most one body stream");

        // Intact rather than displaced, which is the half a refusal that
        // overwrote would still pass without.
        let head = slot
            .take(std::task::Waker::noop())
            .expect("the first open stands");
        assert_eq!(&*head.content_type, "text/csv");

        dropped(first);
        dropped(second);
        dropped(handle);
    }

    /// A write whose client has gone returns without throwing and without
    /// parking — `rule:http-server/a-request-outlives-a-client-that-goes-away`.
    /// The connection dropping its half closes the stream, and every later
    /// write is discarded at once. The context has no task to park on, so a
    /// write that waited would have thrown instead.
    ///
    /// Unreachable from a `.nvst` case for the reason above: a stream that can
    /// close is one a connection is draining.
    // covers: Core\Response\Stream::write
    #[test]
    fn a_stream_write_after_the_client_left_returns_without_throwing() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);

        let media_type = Value::text("text/csv");
        let handle = call(
            super::nvs_core_response_stream,
            &mut ctx,
            &unsized_stream(media_type),
        )
        .expect("an offered cell takes the stream");
        drop(
            slot.take(std::task::Waker::noop())
                .expect("the member filled the cell"),
        );

        let chunk = Value::text("too late");
        for _ in 0..2 {
            let wrote = call(
                super::nvs_core_response_stream_write,
                &mut ctx,
                &[handle, chunk],
            )
            .expect("a write to a client that left is discarded, not thrown");
            assert_eq!(wrote.bits(), Value::null().bits());
        }

        dropped(media_type);
        dropped(chunk);
        dropped(handle);
    }

    /// A send timeout counts as the client having gone: the write that meets
    /// it returns without throwing, and so does every write after it, which
    /// finds the stream closed. A zero send timeout is met by the first write
    /// that finds the cell still full, so no task or clock is needed.
    // covers: Core\Response\Stream::write
    #[test]
    fn a_stream_write_after_the_send_timeout_returns_without_throwing() {
        let slot = nvs_runtime::stream::BodySlot::new(std::time::Duration::ZERO, 1 << 20);
        let mut ctx = framing(&slot);

        let media_type = Value::text("text/csv");
        let handle = call(
            super::nvs_core_response_stream,
            &mut ctx,
            &unsized_stream(media_type),
        )
        .expect("an offered cell takes the stream");
        // Held and never drained: a client that stopped reading, not one that
        // left, so the stream closes at the send timeout and not before it.
        let mut stalled = slot
            .take(std::task::Waker::noop())
            .expect("the member filled the cell");

        let chunk = Value::text("id,name\n");
        for _ in 0..3 {
            let wrote = call(
                super::nvs_core_response_stream_write,
                &mut ctx,
                &[handle, chunk],
            )
            .expect("a write past the send timeout is discarded, not thrown");
            assert_eq!(wrote.bits(), Value::null().bits());
        }
        assert!(
            ctx.body_stream().is_some_and(|emit| emit.is_closed()),
            "the second write did not meet the send timeout"
        );
        let nvs_runtime::stream::Drained::Chunk(first) =
            stalled.drain.next_chunk(std::task::Waker::noop())
        else {
            panic!("the first chunk never reached the cell");
        };
        assert_eq!(first, b"id,name\n", "only the first chunk was in flight");

        dropped(media_type);
        dropped(chunk);
        dropped(handle);
    }

    /// A chunk over the message bound is the program's own mistake, so it
    /// still throws while the client is there, and the stream stays writable:
    /// a chunk inside the bound goes through afterwards.
    // covers: Core\Response\Stream::write
    #[test]
    fn a_stream_write_over_the_message_bound_throws() {
        let slot = nvs_runtime::stream::BodySlot::new(std::time::Duration::from_secs(30), 4);
        let mut ctx = framing(&slot);

        let media_type = Value::text("text/csv");
        let handle = call(
            super::nvs_core_response_stream,
            &mut ctx,
            &unsized_stream(media_type),
        )
        .expect("an offered cell takes the stream");

        let oversized = Value::text("id,name\n");
        call(
            super::nvs_core_response_stream_write,
            &mut ctx,
            &[handle, oversized],
        )
        .expect_err("a chunk twice the message bound");

        let small = Value::text("id\n");
        call(
            super::nvs_core_response_stream_write,
            &mut ctx,
            &[handle, small],
        )
        .expect("a chunk inside the bound still goes through");
        assert!(ctx.body_stream().is_some_and(|emit| !emit.is_closed()));

        dropped(media_type);
        dropped(oversized);
        dropped(small);
        dropped(handle);
    }

    /// A context granting `fs.read` under `root` and nothing else, with the
    /// root canonicalized the way a real snapshot's is — `zip`'s cases build
    /// theirs the same way, for the same reason.
    fn reading_under(root: &std::path::Path) -> Ctx {
        let mut caps = nvs_config::tree::Capabilities {
            fs: Some(nvs_config::tree::CapFs {
                read: Some(nvs_config::tree::Setting::List(vec![
                    root.to_string_lossy().into_owned(),
                ])),
                write: None,
            }),
            ..nvs_config::tree::Capabilities::default()
        };
        caps.canonicalize(&nvs_config::resolve::Disk);
        let mut ctx = Ctx::buffered();
        ctx.set_config(std::sync::Arc::new(nvs_config::Snapshot {
            config: nvs_config::tree::Config {
                capabilities: Some(caps),
                ..nvs_config::tree::Config::default()
            },
            ..nvs_config::Snapshot::default()
        }));
        ctx
    }

    /// `sendFile`'s two forks, told apart by what each leaves on the context.
    /// Answering a request, the call leaves the file's name and writes no byte
    /// of it, because the connection is what reads the file. Off a request, the
    /// bytes land in the context's own output, verbatim and whole across more
    /// than one chunk.
    ///
    /// The refusals are asserted to leave no name behind: a directory, a
    /// missing file and a file outside the grant each answer an error, and a
    /// name left over from one would be sent by the finish path anyway.
    // covers: Core\Response::sendFile
    #[cfg_attr(miri, ignore = "calls `access`, which Miri does not emulate")]
    #[test]
    fn send_file_leaves_a_name_on_a_request_and_writes_the_bytes_off_one() {
        let base = nvs_repo::scratch("response-send-file");
        let root = base.join("granted");
        std::fs::create_dir_all(&root).expect("a temporary directory the test owns");
        let page = root.join("page.html");
        // Longer than one `SEND_FILE_CHUNK`, so the fallback's loop runs more
        // than once and a chunk that was dropped or repeated shows.
        let body: Vec<u8> = (0..super::SEND_FILE_CHUNK * 2 + 17)
            .map(|at| b"<p>&\x00\xff"[at % 6])
            .collect();
        std::fs::write(&page, &body).expect("the fixture is written");
        let outside = base.join("outside.txt");
        std::fs::write(&outside, b"not granted").expect("the fixture is written");
        let spelled = |path: &std::path::Path| Value::text(&path.to_string_lossy());

        let mut served = reading_under(&root);
        served.set_inbound(nvs_runtime::Inbound::new("GET", "/page", ""));
        let named = spelled(&page);
        call(super::nvs_core_response_send_file, &mut served, &[named])
            .expect("a granted regular file is sent");
        assert_eq!(served.take_file_body().as_deref(), Some(page.as_path()));
        assert_eq!(served.take_buffered_output().unwrap_or_default(), b"");

        let mut run = reading_under(&root);
        call(super::nvs_core_response_send_file, &mut run, &[named])
            .expect("a granted regular file is sent");
        assert_eq!(run.take_buffered_output().unwrap_or_default(), body);

        let mut refused = reading_under(&root);
        refused.set_inbound(nvs_runtime::Inbound::new("GET", "/page", ""));
        for path in [root.clone(), root.join("missing.txt"), outside.clone()] {
            let argument = spelled(&path);
            call(
                super::nvs_core_response_send_file,
                &mut refused,
                &[argument],
            )
            .expect_err("a path that is not a granted regular file is refused");
            dropped(argument);
        }
        assert_eq!(refused.take_file_body(), None);

        dropped(named);
    }

    /// `setStatus`'s bound asserted on both sides, read back off the context
    /// the finish path takes the status from: `100` and `599` are declared,
    /// `99` and `600` are refused, and a refusal leaves the status an earlier
    /// call declared in place rather than clearing it.
    ///
    /// `65636` is the case a truncating cast would get wrong, since it wraps
    /// to `100`, which is inside the bound.
    // covers: Core\Response::setStatus
    #[test]
    fn set_status_declares_both_ends_of_its_bound_and_keeps_the_last_good_one() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        for code in [100, 599, 204] {
            call(
                super::nvs_core_response_set_status,
                &mut ctx,
                &[Value::uint(code)],
            )
            .expect("a status between 100 and 599 is declared");
        }
        for code in [0, 99, 600, 65636] {
            call(
                super::nvs_core_response_set_status,
                &mut ctx,
                &[Value::uint(code)],
            )
            .expect_err("a code outside 100 to 599 is refused");
        }
        assert_eq!(ctx.take_status(), Some(204));
        assert_eq!(ctx.take_buffered_output().unwrap_or_default(), b"");
    }

    /// `setHeader` replacing rather than adding, and refusing each byte that
    /// would let a value start a second header line: after two writes to one
    /// name in two spellings the name has one value, at its first position,
    /// and none of the refused writes left anything behind.
    // covers: Core\Response::setHeader
    #[test]
    fn set_header_replaces_in_place_and_refuses_what_would_split_the_line() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let set = |ctx: &mut Ctx, name: &str, value: &str| {
            let args = [Value::text(name), Value::text(value)];
            let answer = call(super::nvs_core_response_set_header, ctx, &args);
            args.into_iter().for_each(dropped);
            answer.is_ok()
        };
        assert!(set(&mut ctx, "Cache-Control", "no-store"));
        assert!(set(&mut ctx, "X-Request-Id", "42"));
        assert!(set(&mut ctx, "cache-control", "max-age=60"));
        assert!(set(&mut ctx, "X-Empty", ""));
        for (name, value) in [
            ("", "x"),
            ("Bad Name", "x"),
            ("Bad:Name", "x"),
            ("content-TYPE", "text/plain"),
            ("X-Split", "a\r\nSet-Cookie: admin=1"),
            ("X-Split", "a\nb"),
            ("X-Split", "a\0b"),
            ("Content-Length", "0"),
            ("transfer-encoding", "chunked"),
            ("Connection", "close"),
            ("UPGRADE", "websocket"),
        ] {
            assert!(
                !set(&mut ctx, name, value),
                "{name:?}: {value:?} was declared"
            );
        }
        let headers: Vec<(Box<str>, Box<str>)> = ctx
            .take_headers()
            .into_iter()
            .map(|header| (header.name, header.value))
            .collect();
        assert_eq!(
            headers,
            [
                ("Cache-Control".into(), "max-age=60".into()),
                ("X-Request-Id".into(), "42".into()),
                ("X-Empty".into(), "".into()),
            ]
        );
    }

    /// `redirect`'s two declarations made together or not at all: each case
    /// of the closed set declares its own status beside one `Location`, and
    /// a destination that would split the header line declares neither.
    // covers: Core\Response::redirect
    #[test]
    fn redirect_declares_status_and_location_together_or_neither() {
        for status in [303, 307, 308] {
            let mut ctx = Ctx::new(OutputSink::Sink);
            let url = Value::text("/orders/42");
            call(
                super::nvs_core_response_redirect,
                &mut ctx,
                &[url, Value::int(status)],
            )
            .expect("a printable destination is declared");
            dropped(url);
            assert_eq!(ctx.take_status(), u16::try_from(status).ok());
            let headers = ctx.take_headers();
            assert_eq!(headers.len(), 1);
            assert_eq!(
                (&*headers[0].name, &*headers[0].value),
                ("Location", "/orders/42")
            );
        }
        for bad in ["", "/a\r\nSet-Cookie: admin=1", "/a\tb"] {
            let mut ctx = Ctx::new(OutputSink::Sink);
            let url = Value::text(bad);
            call(
                super::nvs_core_response_redirect,
                &mut ctx,
                &[url, Value::int(303)],
            )
            .expect_err("a destination a `Location` line cannot carry is refused");
            dropped(url);
            assert_eq!(ctx.take_status(), None);
            assert!(ctx.take_headers().is_empty());
        }
    }

    /// TAINT-5: a relative destination that starts with two of `/` and `\`
    /// names another host, since a browser reads the backslash as a slash.
    /// A launderer's plain answer can hold one, so `redirect` refuses it,
    /// and a path, a query and an absolute URL the program wrote stay.
    // covers: Core\Response::redirect
    #[test]
    fn a_relative_destination_that_names_another_host_is_refused() {
        for bad in [
            "//evil.example/",
            "\\\\evil.example/",
            "/\\evil.example/",
            "\\/evil.example/",
            "  //evil.example/",
        ] {
            let mut ctx = Ctx::new(OutputSink::Sink);
            let url = Value::text(bad);
            call(
                super::nvs_core_response_redirect,
                &mut ctx,
                &[url, Value::int(303)],
            )
            .expect_err("a relative destination naming another host is refused");
            dropped(url);
            assert_eq!(ctx.take_status(), None, "{bad:?}");
            assert!(ctx.take_headers().is_empty(), "{bad:?}");
        }
        for good in [
            "/orders",
            "?page=2",
            "#top",
            "/a//b",
            "https://example.com/",
        ] {
            let mut ctx = Ctx::new(OutputSink::Sink);
            let url = Value::text(good);
            call(
                super::nvs_core_response_redirect,
                &mut ctx,
                &[url, Value::int(303)],
            )
            .expect("a path, a query, a fragment and a written URL are declared");
            dropped(url);
            assert_eq!(ctx.take_status(), Some(303), "{good:?}");
        }
    }
}
