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
//! **name**: its path is § 1's sink, the file is opened by whoever answers the
//! request, and what the bytes are called is the static-file policy's table
//! rather than anything this module declares.
//!
//! Beside them, § 15's `setStatus`, `setHeader`, `redirect` and `addCookie`:
//! the four members here that shape a response without writing one.
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
//! output and come out in the order they were written — gap 1's reading of an
//! inert declaration, applied to the bytes as well. That is what makes a
//! streamed body assertable from a `.nvst` case, none of which is a request a
//! server is answering.
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
//! `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`
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
//! `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` is why
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
//! **A body member declares and then writes, and nothing here arbitrates
//! between two declarations — the last one wins by construction.** That is a
//! bound rather than a gap, because the disagreement cannot reach this module:
//! `rule:security/response-body-is-one-typed-member`'s last paragraph makes two
//! writers of one body a compile error, `nvs_types::response` is that error, and
//! its module doc is the one home for what the refusal reaches and what it
//! deliberately leaves alone. The one case that does reach here is a mount's
//! entry script, which is that module's own known gap and not this one's.
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
//!    — owner: decided-closures

use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
};

/// What `html` declares — § 4's `text/html`, with the charset every other
/// text-shaped answer in this runtime carries, and the same string
/// `nvs_server::serve` answers a request that only echoed with: the sink and
/// the member write one kind of body, so they say one thing about it.
const HTML_MEDIA_TYPE: &str = "text/html; charset=utf-8";

/// What `text` declares — § 4's `text/plain`, with the charset every other
/// text-shaped answer in this runtime carries.
///
/// A constant rather than a literal at the write, because each body member owns
/// one of these and reading them in a column is how a reviewer checks the table
/// against § 4.
const TEXT_MEDIA_TYPE: &str = "text/plain; charset=utf-8";

/// How much of a file one write moves where `sendFile` is the one writing it —
/// off a request, where there is no connection to hand the name to.
///
/// A fixed buffer on the stack rather than the file's own size, so the fallback
/// holds what it is about to write and never what it is about to read: the
/// member's promise is that a response of any size costs a bounded amount of
/// memory, and the path that reads the bytes itself keeps it too.
const SEND_FILE_CHUNK: usize = 64 * 1024;

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

/// The one header name `setHeader` refuses — `rule:security/response-body-is-one-typed-member`'s, owned by whichever
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
/// order for the ones that exist.
/// `Core\Response`'s fully-qualified name, written once — [`CLASS`] declares it
/// and [`crate::registry::CAPABILITIES`] names it on every row of this class, so
/// the table and the roster cannot drift apart.
pub(crate) const NAME: &str = r"Core\Response";

pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "html",
            names: &["body"],
            // Unmarked, and the row where that is the *point*: § 4 says the
            // HTML member "takes the carrier and so has nothing to refuse",
            // because `Core\Html\Markup` is reached only through
            // `rule:core-classes/html-auto-escape`'s own doors — a markup
            // literal, `as` on a source literal, a launderer — each of which
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
        CoreMethod {
            name: "sendFile",
            names: &["path"],
            // § 1's sink, and the one on this class whose argument reaches the
            // filesystem: `..` and the separators direct the resolver, so a
            // path the request chose is the traversal that reads whatever this
            // program was granted. The bytes at it are data and carry no mark
            // of their own — nothing here re-parses them, and what they are
            // called is the static policy's table rather than this member's
            // argument.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_response_send_file",
            doc: Some(&SEND_FILE_DOC),
        },
        CoreMethod {
            name: "stream",
            names: &["contentType"],
            // `bytes`' second mark, for `bytes`' reason and no other: this is
            // the other member that cannot know the media type, so it is told
            // one, and a media type is the one string here that becomes an
            // *instruction* to the peer about how to read everything after it
            // (`rule:security/sink-predicate`). What the chunks carry is
            // [`STREAM`]'s question and is marked there.
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Instance(STREAM_NAME),
            symbol: STREAM_SYMBOL,
            doc: Some(&STREAM_DOC),
        },
        // § 4's body table ends above; § 15's other members follow it in that
        // section's own order, `setStatus` first. Two orders rather than one
        // because the two lists answer different questions — which shape a
        // body is, and what else a response says — and interleaving them
        // would leave a reader unable to check either against its source.
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
        CoreMethod {
            name: "addCookie",
            names: &["name", "value"],
            params: &[
                // Both § 1 sinks on `setHeader`'s reasoning, and the value for
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
                    // to `Const::Null` rather than to § 3's shipped value:
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
                        // An enum and never the string § 3's block writes
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
                    // The two § 3 does not configure, and they are here for
                    // separate reasons. `domain` exists because `rule:errors/cookie-name-bytes`
                    // *forbids* it under a `__Host-` prefix, and a rule the
                    // runtime enforces about a spelling the language does not
                    // have is not a rule. `maxAge` exists because a cookie
                    // with no expiry is a session cookie and there would
                    // otherwise be no way to write any other kind. Neither
                    // takes a configured default — § 3 states none, and a
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

/// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
/// first spelling, as the handle a program writes through — what
/// [`nvs_core_response_stream`] answers with.
///
/// **One member and no slots**, because its whole state is the *context*'s:
/// `nvs_runtime::Ctx` holds the writing half of the body, so the handle is a
/// token saying a stream was opened rather than an owner of one. A slot holding
/// a copy of the writing half would be a second owner of the cell, which is the
/// one thing [`crate::instance`]'s first decision refuses; `Core\Socket` is the
/// same arrangement over a connection's peer, and
/// `a_class_with_slots_has_instance_members_and_the_reverse` is where the two
/// are listed.
///
/// **The body ends when the isolate does**, and that is the whole lifetime
/// rule: there is no `close` member, because `nvs_runtime::stream::Emit`'s own
/// `Drop` ends the body when the request's context goes, and a member that
/// ended it early would be a second answer to when a response is over. A client
/// reading one with `EventSource` reconnects at that point; that is a property
/// of a request-scoped stream rather than a fault, and the reader for it is
/// `fetch` or a progress UI that closes itself.
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
    methods: &[],
    instance: &[CoreMethod {
        name: "write",
        names: &["chunk"],
        // A union rather than `Core\Socket`'s two members, because a chunk of a
        // response body is one thing written two ways where a WebSocket frame's
        // two payload kinds are two things on the wire. What it costs is the
        // classification: [`CoreTy::classification`] answers `None` for a
        // union, so this parameter refuses a `tainted` argument
        // (`rule:security/unclassified-parameter-refuses-tainted`) where
        // `Core\Response::text` accepts one. That is the fail-closed direction
        // of the two, and it is the only mark a union leaves room for — the
        // card below says so where a caller reads it.
        params: &[CoreTy::Union(&[
            CoreTy::Text(Qual::Neutral),
            CoreTy::Blob(Qual::Neutral),
        ])],
        defaults: &[],
        return_ty: CoreTy::Void,
        symbol: STREAM_WRITE_SYMBOL,
        doc: Some(&STREAM_WRITE_DOC),
    }],
    slots: &[],
    constants: &[],
};

/// `Core\Response::stream`'s reference card — `rule:core-api/reference-card`.
const STREAM_DOC: MethodDoc = MethodDoc {
    short: "Answers with a body written over time, declaring `$contentType` — the head goes out as \
            soon as this is called and the body ends when the request does.",
    params: &[ParamDoc {
        name: "contentType",
        desc: "The media type to declare. A sink, exactly as `bytes`' is: it becomes a header the \
               peer obeys, so a `tainted` value is refused at compile time and one holding \
               anything a header cannot carry is refused here.",
        shape: &[],
    }],
    ret: "The handle to write chunks through. Mixing this with `echo` on one response is a compile \
          error, and the body is complete when the request ends — a browser reading one with \
          `EventSource` reconnects at that point, so a stream a client keeps open across page \
          lifetimes is `Core\\Sse::upgrade` instead.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$contentType` is empty or holds a byte outside a header field value — a control \
               character, a newline, or anything above ASCII. Or this request has already opened a \
               body stream, a response having one body.",
    }],
};

/// `Core\Response\Stream::write`'s reference card — `rule:core-api/reference-card`.
const STREAM_WRITE_DOC: MethodDoc = MethodDoc {
    short: "Writes one chunk of the body, waiting while the client is still reading the last one — \
            the whole of the backpressure, since nothing accumulates in between.",
    params: &[ParamDoc {
        name: "chunk",
        desc: "The bytes to send, unchanged, as text or as `bytes`. An empty chunk reaches no \
               wire and is not an error. A `tainted` value is refused at compile time: a \
               parameter taking two shapes carries no classification, and refusing is the safe \
               half of that.",
        shape: &[],
    }],
    ret: "Nothing. The chunk has been handed to the connection by the time this returns.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The client stopped reading — it went away, or it did not take this chunk within \
               the connection's send timeout, which closes the stream rather than waiting \
               without end.",
    }],
};

/// `Core\Response::html`'s reference card — `rule:core-api/reference-card`.
const HTML_DOC: MethodDoc = MethodDoc {
    short: "Answers with `$body`'s bytes as they are, declaring `text/html; charset=utf-8` — the \
            page a handler built, sent without a second escaping pass.",
    params: &[ParamDoc {
        name: "body",
        desc: "The markup to send, verbatim. A `Core\\Html\\Markup` is trusted by the time it \
               exists — a markup literal escaped its holes, `as` took a source literal, a \
               launderer rebuilt it — so there is nothing left here to refuse or to escape, and \
               a `string` is not accepted at all.",
        shape: &[],
    }],
    ret: "Nothing. Mixing this with `echo` on one response is a compile error, `echo` in a request \
          being the other way to write this same body.",
    errors: &[],
};

/// `Core\Response::json`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Response::bytes`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Response::sendFile`'s reference card — `rule:core-api/reference-card`.
const SEND_FILE_DOC: MethodDoc = MethodDoc {
    short: "Answers with the file at `$path`, streamed by the server under the static-file policy's \
            media type — the one body member that hands over a name instead of bytes.",
    params: &[ParamDoc {
        name: "path",
        desc: "The file to send. A sink: a path component directs the resolver, so a `tainted` \
               value is refused at compile time, and `fs.read` must cover it like any other path \
               this program opens. A download name is `Content-Disposition` through `setHeader`, \
               this member taking the path alone.",
        shape: &[],
    }],
    ret: "Nothing. The response carries the media type the static-file policy's table gives the \
          file's extension, and answers a range or a conditional request over it; mixing this with \
          `echo` on one response is a compile error.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`fs.read` does not cover `$path` — the same refusal `Core\\IO::read` gives, from \
                   the same door, whether or not there is a file there.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "There is nothing at `$path`, or the operating system will not let this process \
                   read it.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$path` is a directory, or something else that is not a regular file — a \
                   response body is a file's contents, and there are none to send.",
        },
    ],
};

/// `Core\Response::text`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Response::setStatus`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Response::setHeader`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Response::redirect`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Response::addCookie`'s reference card — `rule:core-api/reference-card`.
const ADD_COOKIE_DOC: MethodDoc = MethodDoc {
    short: "Adds one `Set-Cookie` to this response, every option it leaves out taken from \
            `[http.cookies]` — so a cookie written with no options is `Secure; HttpOnly; \
            SameSite=Lax; Path=/`.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The cookie's name, matched byte for byte on the way back with no substitution \
                   anywhere. A `__Host-` or `__Secure-` prefix is enforced rather than \
                   documented: the first requires `Secure` and `Path=/` and forbids `Domain`, \
                   the second requires `Secure`, and a cookie that does not conform is refused on \
                   write.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value, as the bytes RFC 6265 admits — printable ASCII without a space, a \
                   comma, a semicolon, a backslash or a quote, since each of those ends the value \
                   and begins something the program did not write.",
            shape: &[],
        },
        ParamDoc {
            name: "secure",
            desc: "Whether the cookie is sent over HTTPS alone. Defaults to `[http.cookies] \
                   secure`, which is `true` with nothing configured.",
            shape: &[],
        },
        ParamDoc {
            name: "httpOnly",
            desc: "Whether the cookie is hidden from script. Defaults to `[http.cookies] \
                   http_only`, which is `true` with nothing configured — a cookie that genuinely \
                   needs to be readable says so here, in one field, visibly.",
            shape: &[],
        },
        ParamDoc {
            name: "sameSite",
            desc: "Which cross-site requests carry it. Defaults to `[http.cookies] same_site`, \
                   which is `Lax` with nothing configured. `None` without `Secure` is refused, \
                   for the reason the same pair is refused at boot: every browser drops it.",
            shape: &[],
        },
        ParamDoc {
            name: "path",
            desc: "The path prefix the cookie is sent under. Defaults to `[http.cookies] path`, \
                   which is `/` with nothing configured.",
            shape: &[],
        },
        ParamDoc {
            name: "domain",
            desc: "The domain the cookie is sent to. Omitted by default, which is the narrower \
                   of the two meanings — this host and no subdomain — and `[http.cookies]` \
                   deliberately configures no default for it.",
            shape: &[],
        },
        ParamDoc {
            name: "maxAge",
            desc: "How long the cookie lives. Omitted by default, which is a session cookie. \
                   `0s` is the spelling that deletes one; a negative duration is refused rather \
                   than read as that second spelling.",
            shape: &[],
        },
    ],
    ret: "Nothing. Each call adds a cookie — two calls write two `Set-Cookie` lines, and a name \
          written twice is sent twice rather than collapsed.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$name` is not a cookie name, or does not conform to the `__Host-`/`__Secure-` \
               prefix it carries; `$value`, `path` or `domain` holds a byte that would end the \
               attribute and begin one the program never wrote; `sameSite` is `None` without \
               `secure`; or `maxAge` is negative.",
    }],
};

/// `Core\Response\SameSite`'s fully-qualified name, written once — [`SAME_SITE`]
/// declares it and the [`CoreTy::Enum`] naming it resolves against
/// [`crate::registry::ENUMS`], so the two cannot drift apart.
pub(crate) const SAME_SITE_NAME: &str = r"Core\Response\SameSite";

/// `rule:http-server/cookies-are-secure-httponly-and-lax`'s `SameSite`, as the enum the spec says it is and never the
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
    short: "Which cross-site requests carry a cookie. The three cases are the attribute's own, \
            and the default is `Lax` because a cookie that travels on a cross-site subrequest is \
            what CSRF is made of.",
    cases: &[
        CaseDoc {
            name: "Lax",
            desc: "Sent with a top-level navigation to this site and with nothing else — not \
                   with an image, a form post or a `fetch` from somewhere else. The default, \
                   configured or not.",
        },
        CaseDoc {
            name: "Strict",
            desc: "Never sent cross-site at all, a navigation included. A visitor arriving from \
                   a link therefore arrives logged out, which is the cost that makes this the \
                   deliberate choice rather than the default.",
        },
        CaseDoc {
            name: "None",
            desc: "Sent cross-site. Requires `Secure`, and is refused without it here and at \
                   boot alike, because a browser drops the pair rather than honouring it.",
        },
    ],
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

/// [`REDIRECT`]'s reference card — `rule:core-api/reference-card`.
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
        "nvs_core_response_html" => (nvs_core_response_html as *const ()).cast(),
        "nvs_core_response_json" => (nvs_core_response_json as *const ()).cast(),
        "nvs_core_response_text" => (nvs_core_response_text as *const ()).cast(),
        "nvs_core_response_bytes" => (nvs_core_response_bytes as *const ()).cast(),
        "nvs_core_response_send_file" => (nvs_core_response_send_file as *const ()).cast(),
        STREAM_SYMBOL => (nvs_core_response_stream as *const ()).cast(),
        STREAM_WRITE_SYMBOL => (nvs_core_response_stream_write as *const ()).cast(),
        "nvs_core_response_set_status" => (nvs_core_response_set_status as *const ()).cast(),
        "nvs_core_response_set_header" => (nvs_core_response_set_header as *const ()).cast(),
        "nvs_core_response_redirect" => (nvs_core_response_redirect as *const ()).cast(),
        "nvs_core_response_add_cookie" => (nvs_core_response_add_cookie as *const ()).cast(),
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
pub(crate) fn nameable(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || TOKEN_MARKS.contains(&byte))
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
    /// `rule:core-classes/html-auto-escape` let one be built — a markup
    /// literal, whose holes this member's argument already went through, `as`
    /// on a source literal, or a launderer — so escaping the slot again would
    /// corrupt the page it was built for, exactly as it would in
    /// [`crate::html::nvs_core_html_markup_concat`]. The bytes are read through
    /// [`crate::html::markup_slot`], which is the one reader of that slot, so
    /// this member adds no second way out of the carrier for
    /// `rule:core-classes/html-to-source` to have to be about.
    fn nvs_core_response_html(ctx, args: [1]) {
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
        // Verbatim, and gap 1 in the module doc owns what that means off a
        // request. Unreachable from source for `nvs_core_response_text`'s
        // reason: `OutputSink::Buffer` and `Sink` never fail, and nothing in
        // the language closes a descriptor the host handed the process.
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

/// § 3's four defaults as they stand for this request.
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
            "{ADD_COOKIE}: expected a `Core\\Response\\SameSite` case, got tag {}",
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
    /// httpOnly?: bool, sameSite?: Core\Response\SameSite, path?: string,
    /// domain?: string, maxAge?: Core\Time\Duration}): void` — spec § 15's
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
        // Unreachable from source for both: the row types them `CoreTy::Text`,
        // so `E0401` refuses a non-`string` — and a `tainted` one besides,
        // both being § 1 sinks.
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
        let written = crate::json::written(ctx, args[0], "Core\\Response::json")?;
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

nvs_runtime::nvs_helper! {
    /// `Core\Response::sendFile(string $path): void` — § 4's file row, the one
    /// body member whose bytes do not pass through this call.
    ///
    /// **A name is what it leaves**, and that is the whole shape:
    /// [`nvs_runtime::Ctx::declare_file_body`] records the path, the request's
    /// finish path lifts it onto a `Completion`, and the server opens the file
    /// and streams it under the static-file policy — which is where the media
    /// type, the range and the conditional already live
    /// ([0186](/docs/decisions/0186.md) § 4). So a response of any size costs a
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
    /// is the module doc's gap 1 for the one member whose body is a file — the
    /// declaration means nothing where nobody frames a response, and the bytes
    /// still come out in the order the program wrote them.
    fn nvs_core_response_send_file(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is a `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs —
        // and refuses a `tainted` one besides, that being § 1's sink.
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
    /// `Core\Response::stream(string $contentType): Core\Response\Stream` —
    /// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
    /// stream that ends with its response, as the member that opens one.
    ///
    /// Three effects where its neighbours have two: the media type is declared
    /// on this request's context exactly as every other body member declares
    /// one, the writing half of `nvs_runtime::stream` is put on that context for
    /// [`nvs_core_response_stream_write`] to find, and the head — the media type
    /// beside the reading half — is left in the cell the connection is watching.
    ///
    /// **Off a connection there is no cell, and then this member is inert**, on
    /// the reading the module doc's gap 1 already gives a declaration made where
    /// no response is being framed: a CLI program, a `#[Test]` method and a
    /// `.nvst` case each open a stream that writes to their own output, and the
    /// bytes come out in the order they were written. That is the same fallback
    /// `text` and `bytes` have and not a second one — it is what lets a case
    /// assert a streamed body's bytes at all, since no case is a request a
    /// server is answering.
    ///
    /// A second stream on one request is refused, because a response has one
    /// body and the connection has already been told what the first one is.
    /// Where there is no cell there is nothing that refusal could be about, and
    /// the later declaration simply wins — `Ctx::declare_content_type`'s own
    /// rule for every body member written twice.
    fn nvs_core_response_stream(ctx, args: [1]) {
        // Unreachable from source: the row's parameter is a `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` before this runs — and
        // refuses a `tainted` one besides, that being § 1's sink.
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
            let emit = cell.open(media_type, status, headers).ok_or_else(|| {
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
        }
        ctx.declare_content_type(media_type);
        Ok(crate::instance::build(&STREAM, []))
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
    /// closes the stream and is reported here as itself:
    /// `rule:concurrency/connection-bounds-are-finite`'s defined close rather
    /// than a wait with no end.
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
        if let Some(emit) = ctx.body_stream() {
            // Owned, because the connection takes the chunk rather than reading
            // it: the copy is the one allocation a streamed chunk costs, and it
            // is what lets the writing task go on while the bytes are still on
            // their way to the wire.
            //
            // No case can reach this: a stream that can close is one a
            // connection is draining, and a `.nvst` case is offered no cell.
            // `a_write_whose_reader_has_gone_is_refused_rather_than_parked` is
            // the `#[test]` that asserts it instead.
            return emit
                .send(chunk.to_vec())
                .map(|()| Value::null())
                .map_err(|closed| {
                    Fault::thrown(format!("Core\\Response\\Stream::write(): {closed}"))
                });
        }
        // Unreachable from source, on `text`'s reasoning: `OutputSink::Buffer`
        // and `Sink` never fail, which `Ctx::write_output`'s own `# Errors`
        // states, and nothing in the language closes a descriptor the host
        // handed the process.
        ctx.write_output(chunk).map_err(|error| {
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
    /// declarations are asserted against § 4's column, asserted to be
    /// *distinct*, and — for the one member that is told its type — asserted to
    /// carry two different ones rather than a constant that happened to match.
    ///
    /// `sendFile` is § 4's one row that is not here, and not for want of having
    /// landed: it declares no media type at all, the static-file policy's table
    /// naming what a file's bytes are, so there is nothing of this claim to ask
    /// it. What that member leaves instead is a name, asserted from a program
    /// with the grant in place in
    /// `tests/conformance/core/response-send-file-streams-the-file-under-its-media-type.nvst`.
    /// The sweep below is over the table rather than over a list of names, so a
    /// member that does declare one joins by being added to it.
    #[test]
    fn each_body_member_sets_its_own_content_type() {
        // Nothing else on the path declares one: a context no body member has
        // answered on carries no media type at all, so every declaration below
        // is that member's own act and not a default read back.
        let mut untouched = Ctx::new(OutputSink::Sink);
        assert_eq!(untouched.take_content_type(), None);

        let body = Value::str(NvsStr::new(b"a paragraph"));
        let blob = Value::bytes(NvsStr::new(b"\x89PNG"));
        let told = Value::str(NvsStr::new(b"application/octet-stream"));
        // The carrier itself, built the way `crate::html` builds one: `html`'s
        // parameter is the class and never a `string`, so a test handing it
        // text would assert something no call site can write.
        let markup = crate::instance::build(
            &crate::html::MARKUP,
            [Value::str(NvsStr::new(b"<p>a paragraph</p>"))],
        );

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
                "Core\\Response::{member} declares § 4's own media type"
            );
        }

        // And they are its own: two members answering one type is the endpoint
        // that serves JSON labelled as HTML, which is what § 4 exists to make
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
        let other = Value::str(NvsStr::new(b"image/png"));
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
    /// case § 3 is a statement about, since a deployment that configured the block chose its own.
    ///
    /// The whole line is compared rather than four `contains` calls, because the failure this is
    /// written against is an attribute that went *missing*: every `contains` still passes on a line
    /// that grew a fifth attribute nobody asked for, and on one whose parts fell in an order no peer
    /// parses. `Path` first and `SameSite` last is the render's own order and is not § 3's claim —
    /// what § 3 claims is that all four are there with nothing configured.
    #[test]
    fn a_cookie_is_secure_httponly_samesite_lax_by_default() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        assert!(
            ctx.config().is_none(),
            "the defaults are the ones a tree with no `[http.cookies]` resolves to"
        );

        let name = Value::str(NvsStr::new(b"session"));
        let value = Value::str(NvsStr::new(b"abc123"));
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
        // Appends rather than replaces, which is § 3's neighbour and the reason this member is not
        // `setHeader`: a second cookie may not silently take the first one's place.
        assert!(headers[0].append);

        dropped(name);
        dropped(value);
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

    /// `Core\Response::stream` opening the cell: after the call the connection's
    /// half holds the media type that was written, and a chunk the program
    /// wrote arrives on the reading half rather than in the request's output.
    ///
    /// The two are one claim and not two, because a member that declared the
    /// type and wrote the bytes somewhere else would pass either half alone:
    /// this is the seam `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`
    /// describes, asserted from the end that frames the response.
    #[test]
    fn a_streamed_chunk_reaches_the_connections_half_and_not_the_requests_output() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);

        let media_type = Value::str(NvsStr::new(b"text/csv"));
        let handle = call(super::nvs_core_response_stream, &mut ctx, &[media_type])
            .expect("an offered cell takes the stream");
        let chunk = Value::str(NvsStr::new(b"id,name\n"));
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

    /// The cell's refusal reaching a program: a response has one body, so the
    /// second `stream` is told so and the first one is what the connection
    /// still frames. Unreachable from a `.nvst` case — a second stream needs a
    /// first, and a first needs a cell nothing offers a script.
    #[test]
    fn a_second_stream_on_one_request_is_refused_and_the_first_still_stands() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);

        let first = Value::str(NvsStr::new(b"text/csv"));
        let handle = call(super::nvs_core_response_stream, &mut ctx, &[first])
            .expect("an offered cell takes the first stream");
        let second = Value::str(NvsStr::new(b"application/json"));
        call(super::nvs_core_response_stream, &mut ctx, &[second])
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

    /// A write whose reader has gone is refused rather than parked — the
    /// connection dropping its half closes the stream at once, so a program
    /// learns it on the next chunk instead of at the send timeout.
    ///
    /// Unreachable from a `.nvst` case for the reason above: a stream that can
    /// close is one a connection is draining.
    #[test]
    fn a_write_whose_reader_has_gone_is_refused_rather_than_parked() {
        let slot = offered_cell();
        let mut ctx = framing(&slot);

        let media_type = Value::str(NvsStr::new(b"text/csv"));
        let handle = call(super::nvs_core_response_stream, &mut ctx, &[media_type])
            .expect("an offered cell takes the stream");
        drop(
            slot.take(std::task::Waker::noop())
                .expect("the member filled the cell"),
        );

        let chunk = Value::str(NvsStr::new(b"too late"));
        call(
            super::nvs_core_response_stream_write,
            &mut ctx,
            &[handle, chunk],
        )
        .expect_err("a write with nothing left to read it is refused");

        dropped(media_type);
        dropped(chunk);
        dropped(handle);
    }
}
