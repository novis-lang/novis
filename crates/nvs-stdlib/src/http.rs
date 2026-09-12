//! `Core\Http` — `rule:http-server/allow-url-pins-the-address`'s outbound door,
//! which is one member: the launderer every outbound URL has to pass through, and the pinned
//! `Core\Http\Target` it answers with.
//!
//! `rule:http-server/allow-url-pins-the-address` places the class and § 2 gives the signature; what belongs here is why the answer is a
//! value rather than a `string`, why `Core\Http\Target` has no members at all, and which half of
//! the policy this module holds and which half it does not.
//!
//! # The answer is a value, and that is the whole design
//!
//! A launderer that answered a plain `string` would remove `tainted` and leave the URL exactly as
//! resolvable as it was — the check would have happened at one moment and the connection at
//! another, and in between a second DNS lookup can answer differently. That is DNS rebinding, and
//! it defeats every outbound policy written as a check on a hostname.
//!
//! So [`allowUrl`](CLASS) hands back a [`TARGET`] carrying **both the URL and the address that was
//! approved**, and the connection is made to the address inside it. There is no second resolution
//! for an attacker to poison, and a retry reuses the same `Target` rather than asking again
//! (`rule:http-server/retry-is-opt-in-jittered-and-closed`). `rule:http-server/allow-url-pins-the-address` calls
//! this the first `rule:security/tainted-qualifier` launderer whose output is a value rather than a plain string; it is the
//! reason the roster has one.
//!
//! # `Core\Http\Target` has no members, on purpose
//!
//! It is registered so a program can *name* the type it is holding, and it exposes nothing: there
//! is no `->url()` and no `->address()`. A program that could read the approved address back out
//! would be a program that could rebuild a request around a different one, and the only thing a
//! caller ever needs to do with a `Target` is hand it to the member that connects.
//! `Core\Script\Handle` is the same shape for the same reason ([`crate::script`]), and the slots
//! here are read by the client rather than by anything a program writes.
//!
//! # Two halves of one policy, and only one of them is here
//!
//! The **address** policy is `nvs_config::capability::denied_by_default` and the door that applies
//! it is `nvs_runtime::capability::pin_host` — `rule:security/the-policy-lives-in-the-capability` puts it in the capability rather than
//! in the client precisely so that `Core\Net` and `Core\Db::open` are governed by the same table,
//! and a copy of it in this module would be the second writer that agrees until it does not.
//!
//! What is here is the **URL** half: the scheme roster, the demand that there be a host at all, and
//! the order the two are asked in. A refusal on the scheme happens before the capability is
//! consulted because it is a statement about the text and not about the deployment; everything
//! after that is the door's.
//!
//! # The client's five rows, and the one shape behind all of them
//!
//! `rule:http-server/no-spelling-for-an-unbounded-wait` gives every request
//! member the same two parameters: the URL, and one trailing [`OPTIONS`] bag. The URL is `rule:security/outbound-url-is-a-sink`
//! 's sink — `string | Core\Http\Target`, **both unqualified**, so a `tainted` operand is a
//! diagnostic and [`allowUrl`](CLASS) is the only way past it. A `Target` argument was pinned by
//! the launderer that built it and is not pinned again; a plain `string` — the form § 1 keeps for a
//! URL the program itself authored — goes through the same four questions [`pin`] asks for
//! `allowUrl`, so there is one implementation of the policy and not two.
//!
//! **`retry` is three flat options rather than the nested shape § 5 first wrote.** A bag flattens
//! to one ABI argument per option ([`crate::registry::CoreTy::Options`]), so a bag nested inside one
//! has nothing to flatten into — the registry refuses it outright. The three keys are therefore
//! `retryAttempts`, `retryBackoff` and `retryIdempotencyKey`, which keeps the grouping legible at a
//! call site and costs seven characters at each of them; § 5's own type block is amended to say so,
//! because the ADR's body is the home of the rule and an overlay a reader has to apply is not.
//!
//! **§ 7's refusal is a diagnostic, and so it is not in this module.** A `post` that asks for
//! retries needs [`RETRY_KEY_OPTION`], and both halves of that question are written at the call
//! site — the verb is the member's own name, the bag is an `rule:core-api/shape-rules` R2 literal — so
//! `nvs_types::expr::args`' `reject_keyless_retry` reports it while compiling and the body has
//! nothing left to judge. What this module owns is the two option names the rule is written over,
//! handed to the checker by [`crate::registry::idempotent_retry_rule`] rather than copied into it.
//! The dynamic half § 7 also names — a verb chosen at run time — arrives with the spec's
//! `send(Core\Http\Request)` row and throws before the first attempt rather than before the second.
//!
//! # The reply is read through members, and its body is `tainted`
//!
//! [`RESPONSE`] carries two slots and the two members that read them back, and both halves of that
//! are decisions rather than layout.
//!
//! **`status` is a call, not a property.** A `Core` instance has no property a program can reach —
//! [`crate::registry::CoreClass::slots`] is that rule's home — so `$response->status` is `E0405`
//! and `$response->status()` is the spelling, and `examples/http.nvs` was corrected to it rather
//! than the rule being bent. The alternative was giving `Core` its first reachable field, which
//! would have made this crate's slot layout part of the language and left every class after this
//! one choosing between two surfaces for one piece of state.
//!
//! **`text()` answers a `tainted string`.** A reply is bytes another host chose, and pinning says
//! where they came from and nothing about what is in them, so a body is input in exactly the sense
//! `rule:security/tainted-qualifier` means — its
//! roster names these readers for that reason. `status()` is an `int` and carries no qualifier,
//! because there is nothing in three digits for a sink to misread.
//!
//! The `body` slot holds a `string` rather than the bytes that arrived. Decoding is one question,
//! answered by the transport where the charset is known; a reader that decoded on every call would
//! answer it again, and a second answer is the one that will disagree.
//!
//! # The transport is a module of its own, and it is handed an address rather than a `Ctx`
//!
//! [`transport`] composes the request, writes it, reads the reply and decides what is worth trying
//! again; what stays here is every decision about *whether* a request may happen at all. The seam
//! is [`transport::send`]'s `repin` closure: a redirect hop is re-checked by calling back into
//! [`pin`], so `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`'s rule is enforced by the same four questions the first URL passed and
//! there is no second copy of the policy under the socket. What that module's own doc owns is the
//! rest — one connection per attempt, how `https` reaches `nvs-host`'s TLS client and which host
//! name its certificate is checked against, and what a reply is allowed to make this process hold.
//!
//! # What is not here yet, and why each is deliberate rather than forgotten
//!
//! **A verb chosen at run time.** [`CLIENT`] is five rows whose verb is the row's own name, which
//! is what makes `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`'s question
//! answerable while compiling. `patch`, and the `request(Core\Http\Method, …)` row where a verb a
//! program computes belongs, are not here yet; the same two checks move to the call when they
//! arrive.
//!
//! **The reply's headers.** [`RESPONSE`] answers `status()` and `text()` and nothing else: a slot
//! and the member that reads it are one decision, and a `header()` over a map nothing fills would
//! be a surface with no behaviour under it. It arrives with the transport that writes the map.
//!
//! **What it spends:** one `Core\Http\Target` allocation per laundered URL, two slots wide, charged
//! to the request that laundered it — and one synchronous resolution per call, which
//! `pin_host`'s own docs own. A request member allocates nothing of its own before the transport:
//! a `Target` argument is borrowed, and a `string` one is pinned without building a target, since
//! nothing downstream of the check would read it. A call that writes a body spends that body once,
//! charged to the request and held until the last attempt is done with it — except a
//! `Core\Http\Part::file`, which is a descriptor and a chunk rather than the file
//! ([`transport::Piece`]). What the exchange itself spends is [`transport`]'s to state.

mod pool;
pub(crate) mod stream;
mod transport;

use std::net::IpAddr;
use std::time::{Duration, Instant};

use fluent_uri::UriRef;
use fluent_uri::component::{Authority, Scheme};
use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};
use nvs_syntax::duration;
use rand::RngExt;

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Http";

/// What a refusal from this module and from the door below it is written under.
const MEMBER: &str = r"Core\Http::allowUrl";

/// `rule:http-server/allow-url-pins-the-address`'s launderer, as the one row `Core\Http` has today.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "allowUrl",
        names: &["url"],
        // The one parameter in the language that **admits** a tainted URL, and
        // the module doc above is the home of why its answer is a value: a
        // `Qual::Launder` that returned a `string` would have removed the
        // qualifier and left the rebinding gap open.
        params: &[CoreTy::Text(Qual::Launder)],
        defaults: &[],
        return_ty: CoreTy::Instance(TARGET_NAME),
        symbol: "nvs_core_http_allow_url",
        doc: Some(&ALLOW_URL_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Http::allowUrl`'s reference card — `rule:core-api/reference-card`.
const ALLOW_URL_DOC: MethodDoc = MethodDoc {
    short: "Checks `$url` against the outbound policy and pins it: the scheme, the `net.connect` \
            grant and the resolved address are all decided here, and the answer carries the \
            address that was approved.",
    params: &[ParamDoc {
        name: "url",
        desc: "The URL to approve; `tainted` is accepted here and nowhere else outbound.",
        shape: &[],
    }],
    ret: "A `Core\\Http\\Target` bound to one address, which is what the client connects to — so a \
          second name lookup cannot answer differently.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The text is not a URL, its scheme is neither `http` nor `https`, it names no host, \
               `net.connect` does not grant that host, the host resolves to no address, or it \
               resolves to a loopback, private, link-local or unspecified address that \
               `net.internal` does not name.",
    }],
};

/// [`TARGET`]'s name, written once — see [`NAME`].
pub(crate) const TARGET_NAME: &str = r"Core\Http\Target";

/// `rule:http-server/allow-url-pins-the-address`'s pinned target: a URL and the one address it was approved at.
///
/// No members, for the reason this module's own docs give — a program names it
/// and hands it on, and reading the address back out is the operation that
/// would make pinning decorative.
pub(crate) const TARGET: CoreClass = CoreClass {
    name: TARGET_NAME,
    methods: &[],
    instance: &[],
    slots: &["url", "address"],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_http_allow_url" => (nvs_core_http_allow_url as *const ()).cast(),
        "nvs_core_http_client_get" => (nvs_core_http_client_get as *const ()).cast(),
        "nvs_core_http_client_post" => (nvs_core_http_client_post as *const ()).cast(),
        "nvs_core_http_client_put" => (nvs_core_http_client_put as *const ()).cast(),
        "nvs_core_http_client_patch" => (nvs_core_http_client_patch as *const ()).cast(),
        "nvs_core_http_client_delete" => (nvs_core_http_client_delete as *const ()).cast(),
        "nvs_core_http_client_head" => (nvs_core_http_client_head as *const ()).cast(),
        "nvs_core_http_client_request" => (nvs_core_http_client_request as *const ()).cast(),
        "nvs_core_http_part_file" => (nvs_core_http_part_file as *const ()).cast(),
        "nvs_core_http_part_bytes" => (nvs_core_http_part_bytes as *const ()).cast(),
        "nvs_core_http_response_status" => (nvs_core_http_response_status as *const ()).cast(),
        "nvs_core_http_response_text" => (nvs_core_http_response_text as *const ()).cast(),
        "nvs_core_http_response_bytes" => (nvs_core_http_response_bytes as *const ()).cast(),
        "nvs_core_http_response_json_as" => (nvs_core_http_response_json_as as *const ()).cast(),
        "nvs_core_http_response_header" => (nvs_core_http_response_header as *const ()).cast(),
        "nvs_core_http_response_headers" => (nvs_core_http_response_headers as *const ()).cast(),
        // The streamed reply's own, beside its class rather than here: they are
        // this module's symbols, and [`stream`] is a module of this one for
        // [`transport`]'s reason.
        other => return stream::address(other),
    })
}

/// The `string` in slot 0.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: slot 0 is a `string` in the one row,
/// so another tag is a compiled-code bug rather than anything a program can
/// write — `E0401` refuses the call first.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `string`, got tag {}",
            args[0].tag_byte()
        ))
    })
}

/// The address `text` resolves to, once the URL and the deployment have both
/// approved it — the whole of `rule:http-server/allow-url-pins-the-address` and `rule:security/net-address-policy` as this module holds it, written
/// under `member` so a refusal names the row the caller wrote.
///
/// The order of the four questions is deliberate. The text is parsed and its
/// scheme judged first, because both are statements about the argument and
/// neither tells a caller anything about the deployment. Then
/// `nvs_runtime::capability::pin_host` asks the capability about the host and
/// § 3's table about the address it resolves to — in that order, so an
/// ungranted program cannot use this member as a resolver for names it was
/// never allowed to reach.
///
/// One function rather than one per caller: [`nvs_core_http_allow_url`] and
/// every row of [`CLIENT`] that is handed a plain `string` ask exactly this,
/// and a second copy of it would be the second writer that agrees until it
/// does not.
///
/// # Errors
///
/// A thrown `RuntimeError` for any of the four: the text is not a URL, its
/// scheme is outside the roster, it names no host, or the capability refuses
/// the host or the address it resolves to.
fn pin(ctx: &mut Ctx, text: &str, member: &str) -> Result<std::net::IpAddr, Fault> {
    let host = judged_host(text, member)?;
    nvs_runtime::capability::pin_host(ctx, &host, member)
}

/// [`pin`]'s first two questions — the ones a URL answers by itself — and the
/// host they leave: the text parses, and its scheme is one of the two this
/// class speaks.
///
/// Split out because `rule:testing/an-outbound-call-is-answered-from-a-table`
/// asks exactly these of a faked call and none of the ones below them: a call
/// answered from a test's table is still refused for a scheme nothing here
/// speaks, because that is a statement about what the program wrote, while
/// resolution and § 3's address table have nothing to judge where no
/// connection is made.
///
/// # Errors
///
/// A thrown `RuntimeError` for a text that is not a URL, a scheme outside the
/// roster, or a URL that names no host.
fn judged_host(text: &str, member: &str) -> Result<String, Fault> {
    let reference = UriRef::parse(text).map_err(|_| {
        Fault::thrown(format!(
            "{member}: this text is not a URL, so there is no host in it to approve"
        ))
    })?;

    // Case-insensitively, because a scheme is: `HTTP://` is the same URL, and a
    // roster compared byte for byte would be one a caller can step around by
    // shouting.
    let scheme = reference.scheme().map(Scheme::as_str).unwrap_or_default();
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err(Fault::thrown(format!(
            "{member}: the scheme must be `http` or `https`, and this URL names `{scheme}`"
        )));
    }

    let authority = reference.authority().ok_or_else(|| {
        Fault::thrown(format!(
            "{member}: the URL names no host, so there is nothing to resolve and pin"
        ))
    })?;

    Ok(Authority::host(&authority).to_owned())
}

nvs_runtime::nvs_helper! {
    /// `Core\Http::allowUrl(tainted string $url): Core\Http\Target` — `rule:http-server/allow-url-pins-the-address`
    /// , and the only spelling that removes `tainted` from an outbound URL.
    ///
    /// The four questions it asks, and the order it asks them in, are [`pin`]'s.
    /// What is here is the answer: a value carrying both the URL and the address
    /// that was approved, for the reason this module's own docs give.
    fn nvs_core_http_allow_url(ctx, args: [1]) {
        let text = text_of(args, "allowUrl")?;
        let pinned = pin(ctx, text, MEMBER)?;

        Ok(crate::instance::build(
            &TARGET,
            [
                Value::str(NvsStr::new(text.as_bytes())),
                Value::str(NvsStr::new(pinned.to_string().as_bytes())),
            ],
        ))
    }
}

// ------------------------------------------------------------------- the client

/// [`CLIENT`]'s name, written once — see [`NAME`].
pub(crate) const CLIENT_NAME: &str = r"Core\Http\Client";

/// [`RESPONSE`]'s name, written once — see [`NAME`].
pub(crate) const RESPONSE_NAME: &str = r"Core\Http\Response";

/// The `Duration` every time bound in [`OPTIONS`] is spelled as, once.
const DURATION: CoreTy = CoreTy::Instance(crate::time::DURATION_NAME);

/// The two [`OPTIONS`] names `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`'s refusal is written over, spelled
/// once so the rule and the rows cannot drift apart —
/// [`crate::registry::idempotent_retry_rule`] hands these to the checker rather
/// than the checker holding its own copy of them.
pub(crate) const RETRY_ATTEMPTS_OPTION: &str = "retryAttempts";
/// See [`RETRY_ATTEMPTS_OPTION`].
pub(crate) const RETRY_KEY_OPTION: &str = "retryIdempotencyKey";

/// `rule:security/outbound-url-is-a-sink`'s outbound sink, as the one parameter every request member
/// takes: a URL the program itself authored, or a [`TARGET`] the launderer
/// already approved.
///
/// **Both halves are unqualified**, which is what refuses a `tainted` operand:
/// the union's own qualifier cell is empty (`nvs_types::core_lib`'s `qual_of`
/// reads none through a union), and `tainted string` is not assignable to
/// `string`, so the diagnostic comes from the type rather than from a check a
/// member could forget. The [`Qual::Sink`] mark on the text half is the
/// statement of *why* — this parameter's content becomes a request something
/// makes — and it is what a reader of the row sees.
const URL: CoreTy = CoreTy::Union(&[CoreTy::Text(Qual::Sink), CoreTy::Instance(TARGET_NAME)]);

/// `rule:http-server/no-spelling-for-an-unbounded-wait`'s `Core\Http\Options`, as the one trailing bag every request
/// member carries.
///
/// **The absence is the decision.** Every bound here is a `Duration`, which has
/// no infinite value; none is nullable, so `{deadline: null}` does not compile;
/// and an omitted option passes `Const::Null`, which the body reads as *not
/// given* and the transport will answer from `[http.client]` — § 5's rule that
/// leaving a field out inherits the bound rather than removing it. There is no
/// spelling left over for "wait forever".
///
/// The three `retry*` options are § 6's one shape, flattened for the reason
/// this module's own docs give.
///
/// A macro rather than a slice, for [`request_params`]' reason one axis over:
/// `stream` carries these keys and the two bounds a body read as it arrives has
/// of its own, so one row's bag is the other's with a group after it. A `const`
/// slice cannot be extended, and a second copy of the shared keys would say the
/// same thing until it did not.
// The keys every row carries, at the indentation they had as a slice because
// rustfmt does not reach inside a macro's body, then the group a row adds after
// them.
macro_rules! request_options {
    ($($trailing:expr),* $(,)?) => { &[
    CoreOption {
        name: "deadline",
        ty: DURATION,
        default: Const::Null,
    },
    CoreOption {
        name: "connectTimeout",
        ty: DURATION,
        default: Const::Null,
    },
    // `secret` on one axis and unqualified on the other, which is the type
    // saying what this position is: `rule:security/secret-sinks-refuse` names an
    // outbound request's headers as one of the three places a credential has to
    // be able to reach, and an `Authorization` header is why. A `tainted` value
    // is still refused, by assignability rather than by a check — a header value
    // is copied verbatim into the request, and one from outside chooses what is
    // sent alongside it.
    CoreOption {
        name: "headers",
        ty: CoreTy::Array(&CoreTy::SecretText(Qual::Neutral)),
        default: Const::EmptyArray,
    },
    // A count rather than a `bool`: `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned` turns redirects off by default,
    // and a program that wants them owes a number, since "follow them" with no
    // bound is the unbounded spelling one hop up.
    CoreOption {
        name: "followRedirects",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: RETRY_ATTEMPTS_OPTION,
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "retryBackoff",
        ty: DURATION,
        default: Const::Null,
    },
    CoreOption {
        name: RETRY_KEY_OPTION,
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    // `rule:http-server/an-outbound-request-carries-one-body`'s four body keys, and the fifth that types a raw
    // one. Which key the body was written under is what says how it is sent, so
    // there is no mode string beside one value (`rule:core-api/no-mode-strings`) and no
    // sniffing an `array<string>` for whether it meant a form or an object.
    //
    // `mixed` rather than a structured type: what a JSON body may hold is what
    // JSON may hold, and the walk that encodes it is `crate::json`'s with the
    // outbound `secret` exemption applied. That admits `null`, so the omission
    // is `Const::NeverWritten` and not `Const::Null`
    // (`rule:core-api/a-nullable-field-omits-as-the-never-written-marker`): the
    // document `null` is a body a program may mean, and it arrives under
    // `Tag::Null` where writing no `json` at all arrives under `Tag::Unset`.
    CoreOption {
        name: JSON_OPTION,
        ty: CoreTy::Mixed,
        default: Const::NeverWritten,
    },
    // Every body position admits both qualifiers, and the *type* is what says
    // so: `nvs_types::core_lib`'s `qual_of` reads no mark through an options
    // bag, so a `Qual` here would be documentation the checker never consults.
    // `secret tainted string` is what `nvs_types::expr::assign` widens a plain,
    // a `tainted` and a `secret` argument onto alike, which is the admission
    // `rule:security/secret-sinks-refuse` grants an outbound request and `rule:http-server/an-outbound-request-carries-one-body` grants a body
    // written out of what a user sent.
    CoreOption {
        name: FORM_OPTION,
        ty: CoreTy::Array(&CoreTy::SecretTaintedStr),
        default: Const::Null,
    },
    // The third arm is the one that does not have to be held: a `Core\Http\Part`
    // names a file the framing reads while it sends, so an upload larger than
    // this process's own ceiling is still a body a program can write.
    CoreOption {
        name: BODY_OPTION,
        ty: CoreTy::Union(&[
            CoreTy::SecretTaintedStr,
            CoreTy::SecretTaintedBytes,
            CoreTy::Instance(PART_NAME),
        ]),
        default: Const::Null,
    },
    // Unqualified, as a header value is and for the same reason: this text is
    // copied verbatim into a header line, and it names a media type the program
    // itself knows rather than anything a reply or a user supplied.
    CoreOption {
        name: CONTENT_TYPE_OPTION,
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    // Every field is a value or a part, which is the same union `body` takes
    // one element of: a form that uploads a file is the case multipart exists
    // for, and a map admitting only text could not express one.
    CoreOption {
        name: MULTIPART_OPTION,
        ty: CoreTy::Array(&MULTIPART_FIELD),
        default: Const::Null,
    },
    $($trailing,)*
] };
}

/// Every row but `stream`'s bag: what a buffered call may write, and nothing
/// else — see [`request_options`].
const OPTIONS: &[CoreOption] = request_options!();

/// `stream`'s bag: the shared keys, then
/// `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`'s two
/// further bounds.
///
/// The two are here rather than in [`OPTIONS`] because that rule makes either
/// of them on a buffered member a compile-time diagnostic, and a bag that does
/// not declare a key is where that diagnostic already comes from: the checker
/// refuses an option no bag names and lists the ones this member has. A shared
/// bag plus a check in each buffered body would have refused the same call at
/// run time, one deployment later.
const STREAM_OPTIONS: &[CoreOption] = request_options!(IDLE_OPTION, MAX_DURATION_OPTION);

/// The longest silence a streamed body may go through — see [`STREAM_OPTIONS`].
const IDLE_OPTION: CoreOption = CoreOption {
    name: "idle",
    ty: DURATION,
    default: Const::Null,
};

/// The longest a streamed body may take at all — see [`STREAM_OPTIONS`]. Two
/// bounds and not one because a server dribbling a byte a second passes every
/// idle check ever written, and only a lifetime ends it.
const MAX_DURATION_OPTION: CoreOption = CoreOption {
    name: "maxDuration",
    ty: DURATION,
    default: Const::Null,
};

/// What one key of [`MULTIPART_OPTION`]'s map holds: a value, or a part sent
/// under its own name and type.
const MULTIPART_FIELD: CoreTy =
    CoreTy::Union(&[CoreTy::SecretTaintedStr, CoreTy::Instance(PART_NAME)]);

/// `rule:http-server/an-outbound-request-carries-one-body`'s body keys, named once: [`OPTIONS`] declares them,
/// [`crate::registry::request_body_rule`] hands the same spellings to the
/// checker, and the transport reads them out of the slots below. A renamed key
/// therefore cannot leave the refusal looking for a name no row writes.
pub(crate) const JSON_OPTION: &str = "json";
/// See [`JSON_OPTION`].
pub(crate) const FORM_OPTION: &str = "form";
/// See [`JSON_OPTION`].
pub(crate) const BODY_OPTION: &str = "body";
/// See [`JSON_OPTION`].
pub(crate) const MULTIPART_OPTION: &str = "multipart";
/// The key that types [`BODY_OPTION`]'s octets and means nothing without them —
/// see [`JSON_OPTION`].
pub(crate) const CONTENT_TYPE_OPTION: &str = "contentType";
/// The header the framing writes that key into, lower-cased as a record's names
/// are — see [`faked`].
const CONTENT_TYPE_HEADER: &str = "content-type";

/// The four keys of [`JSON_OPTION`]'s family, in [`OPTIONS`]' own order: at most
/// one of them may be written at a call, and a member whose verb carries no body
/// admits none.
pub(crate) const BODY_OPTIONS: &[&str] = &[JSON_OPTION, FORM_OPTION, BODY_OPTION, MULTIPART_OPTION];

// --------------------------------------------------------------- a part of a body

/// [`PART`]'s name, written once — see [`NAME`].
pub(crate) const PART_NAME: &str = r"Core\Http\Part";

/// The octets a raw body carries, as the two arms every such position admits.
///
/// One spelling for [`BODY_OPTION`]'s text half and for `Core\Http\Part::bytes`'
/// first parameter, because the two are the same question — what may be sent
/// verbatim — and a second copy of the union is the one that stops matching.
const OCTETS: CoreTy = CoreTy::Union(&[CoreTy::SecretTaintedStr, CoreTy::SecretTaintedBytes]);

/// `Core\Http\Part::file`'s bag: the two things a part says about itself that
/// its path does not already say.
const PART_FILE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "filename",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: CONTENT_TYPE_OPTION,
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
];

/// `Core\Http\Part::bytes`' bag, which is [`PART_FILE_OPTIONS`] without the
/// key that row takes as a required argument.
const PART_BYTES_OPTIONS: &[CoreOption] = &[CoreOption {
    name: CONTENT_TYPE_OPTION,
    ty: CoreTy::Text(Qual::Neutral),
    default: Const::Null,
}];

/// `rule:http-server/an-outbound-request-carries-one-body`'s part: a body, or one field of a
/// multipart body, described rather than held.
///
/// **Two constructors and no member**, which is [`TARGET`]'s shape for a
/// different reason. A part is what a request is *told to send*; handing the
/// octets back out would make `file` hold the file it exists to avoid holding,
/// and the whole reason a large upload is written as a part is that nothing
/// between the disk and the socket ever has all of it.
///
/// `filename` is required on `bytes` and optional on `file` because a path
/// already carries one: an omitted `filename` is the path's last component,
/// while octets a program composed have no name until it writes one.
pub(crate) const PART: CoreClass = CoreClass {
    name: PART_NAME,
    methods: &[
        CoreMethod {
            name: "file",
            names: &["path"],
            // A sink in the path, as every path in `Core\IO` is and for that
            // class's reason: `..` and the separators direct the resolver, so a
            // `tainted` path is refused where it is written.
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Options(PART_FILE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(PART_NAME),
            symbol: "nvs_core_http_part_file",
            doc: Some(&PART_FILE_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &["data", "filename"],
            params: &[
                OCTETS,
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(PART_BYTES_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(PART_NAME),
            symbol: "nvs_core_http_part_bytes",
            doc: Some(&PART_BYTES_DOC),
        },
    ],
    instance: &[],
    slots: &["path", "data", "filename", "contentType"],
    constants: &[],
};

/// `Core\Http\Part::file`'s reference card — `rule:core-api/reference-card`.
const PART_FILE_DOC: MethodDoc = MethodDoc {
    short: "A body, or one field of a multipart body, read from the file at `$path` while the \
            request is being sent rather than held in memory first. Needs the `fs.read` \
            capability, which is checked here so that an ungranted path is refused where it is \
            written.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The file to send, absolute or relative to the working directory.",
            shape: &[],
        },
        ParamDoc {
            name: "filename",
            desc: "The name the other end is told, defaulting to the path's last component.",
            shape: &[],
        },
        ParamDoc {
            name: "contentType",
            desc: "The media type this part is sent under. Omitted, a multipart field is sent \
                   as `application/octet-stream` and a whole body under the call's own \
                   `contentType`.",
            shape: &[],
        },
    ],
    ret: "A part, to be written to `body` or to one key of `multipart`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `fs.read` for this path.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "There is nothing at `$path`, or the operating system refused to describe it.",
        },
    ],
};

/// `Core\Http\Part::bytes`' reference card — `rule:core-api/reference-card`.
const PART_BYTES_DOC: MethodDoc = MethodDoc {
    short: "A body, or one field of a multipart body, sent from octets the program is already \
            holding, under the name it gives them.",
    params: &[
        ParamDoc {
            name: "data",
            desc: "The octets to send, as a `string` or a `bytes`.",
            shape: &[],
        },
        ParamDoc {
            name: "filename",
            desc: "The name the other end is told. Required, because octets carry none.",
            shape: &[],
        },
        ParamDoc {
            name: "contentType",
            desc: "The media type this part is sent under, defaulting as `file`'s does.",
            shape: &[],
        },
    ],
    ret: "A part, to be written to `body` or to one key of `multipart`.",
    errors: &[],
};

/// [`PART`]'s slots, by index — see [`TARGET_URL_SLOT`].
///
/// A part is one of two kinds and the empty slot is which: a `file` part holds
/// its path and no octets, a `bytes` part its octets and no path. One class
/// rather than two, because every position that takes a part takes both kinds
/// and a union of two instance types at each of them would say the same thing
/// twice.
const PART_PATH_SLOT: usize = 0;
/// See [`PART_PATH_SLOT`].
const PART_DATA_SLOT: usize = 1;
/// See [`PART_PATH_SLOT`].
const PART_FILENAME_SLOT: usize = 2;
/// See [`PART_PATH_SLOT`].
const PART_CONTENT_TYPE_SLOT: usize = 3;

/// How many arguments each [`PART`] row takes: its own parameters plus one per
/// option of its bag, which is three for both of them.
const PART_ARITY: usize = 3;

nvs_runtime::nvs_helper! {
    /// `Core\Http\Part::file(string $path, Core\Http\FilePartOptions): Core\Http\Part`
    /// — `rule:http-server/an-outbound-request-carries-one-body`.
    ///
    /// The capability is asked **here**, where the program named the path, and
    /// asked again where the file is opened. Neither is redundant: a part is
    /// built from a name a program wrote and sent from a call somewhere else, so
    /// a grant that covers neither should fail at the line that wrote the path
    /// rather than inside a request that has already been composed. The `stat`
    /// this door performs is also the answer to whether there is a file there at
    /// all, which is the other thing worth learning before a socket is opened.
    ///
    /// The size is deliberately not kept: `Content-Length` is read when the
    /// body is framed, so a part built early and sent late reports what is on
    /// disk rather than what was there when it was named.
    fn nvs_core_http_part_file(ctx, args: [PART_ARITY]) {
        const MEMBER: &str = r"Core\Http\Part::file";

        let path = text_of(args, "file")?.to_owned();
        nvs_runtime::capability::metadata(ctx, std::path::Path::new(&path), MEMBER)?;

        let filename = args[1].as_text().map(str::to_owned).or_else(|| {
            std::path::Path::new(&path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
        Ok(crate::instance::build(
            &PART,
            [
                Value::str(NvsStr::new(path.as_bytes())),
                Value::null(),
                text_slot(filename.as_deref()),
                text_slot(args[2].as_text()),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Part::bytes(string|bytes $data, string $filename, Core\Http\BytesPartOptions): Core\Http\Part`
    /// — `rule:http-server/an-outbound-request-carries-one-body`.
    ///
    /// The octets are copied into the part under `Tag::Bytes` whichever arm
    /// carried them, because what a body is made of is octets and the
    /// distinction the two arms draw — is this text — has no bearing on what
    /// goes onto the wire.
    fn nvs_core_http_part_bytes(_ctx, args: [PART_ARITY]) {
        // Both refusals below are unreachable from source: the row's own
        // parameters are `string|bytes` and `string`, so `E0401` refuses the
        // call before a compiled one can arrive here with another tag.
        let data = args[0]
            .as_str_bytes()
            .or_else(|| args[0].as_bytes())
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Http\\Part::bytes expected a `string` or a `bytes`, got tag {}",
                    args[0].tag_byte()
                ))
            })?;
        // Unreachable from source for the reason above: `E0401` refuses a
        // `filename` that is not a `string` at the row's second parameter.
        let filename = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Http\\Part::bytes expected a `string` for `filename`, got tag {}",
                args[1].tag_byte()
            ))
        })?;

        Ok(crate::instance::build(
            &PART,
            [
                Value::null(),
                Value::bytes(NvsStr::new(data)),
                Value::str(NvsStr::new(filename.as_bytes())),
                text_slot(args[2].as_text()),
            ],
        ))
    }
}

/// One optional slot of a [`PART`]: the text, or the `null` that says the
/// program wrote none.
fn text_slot(text: Option<&str>) -> Value {
    text.map_or_else(Value::null, |text| Value::str(NvsStr::new(text.as_bytes())))
}

/// The ABI slot each of [`OPTIONS`]'s bounds flattens into — the bag expands to
/// one argument per option, in declaration order, after the URL at slot 0.
const DEADLINE: usize = 1;
const CONNECT_TIMEOUT: usize = 2;
const HEADERS: usize = 3;
const FOLLOW_REDIRECTS: usize = 4;
const RETRY_ATTEMPTS: usize = 5;
const RETRY_BACKOFF: usize = 6;
const RETRY_KEY: usize = 7;
/// The body keys' own slots, in [`OPTIONS`]' order — see [`DEADLINE`].
const JSON: usize = 8;
/// See [`JSON`].
const FORM: usize = 9;
/// See [`JSON`].
const BODY: usize = 10;
/// See [`JSON`].
const CONTENT_TYPE: usize = 11;
/// See [`JSON`].
const MULTIPART: usize = 12;
/// [`STREAM_OPTIONS`]' own two slots, after every shared key's, and reachable
/// only from the one row that declares them — see [`DEADLINE`].
const IDLE: usize = 13;
/// See [`IDLE`].
const MAX_DURATION: usize = 14;

/// How many arguments a request member takes: the URL plus one per option, which
/// is what every `nvs_helper!` row below writes as its arity. Derived rather
/// than written, so a key added to [`OPTIONS`] cannot leave a helper declaring
/// an arity the call site does not pass. The body keys hold the slots after
/// [`RETRY_KEY`] and are named where they are read.
const REQUEST_ARITY: usize = OPTIONS.len() + 1;

/// How many arguments `stream` takes after the verb it is handed: the URL plus
/// one per key of [`STREAM_OPTIONS`], which is [`REQUEST_ARITY`] and the two
/// bounds that bag adds. Derived for that constant's reason.
const STREAM_ARITY: usize = STREAM_OPTIONS.len() + 1;

/// [`TARGET`]'s two slots, by index — see [`STATUS_SLOT`].
const TARGET_URL_SLOT: usize = 0;
/// See [`TARGET_URL_SLOT`].
const TARGET_ADDRESS_SLOT: usize = 1;

/// `rule:http-server/no-spelling-for-an-unbounded-wait`'s `[http.client]` block, as the answers an omitted option
/// inherits when the deployment configured nothing.
///
/// The section's own TOML is the home of these three numbers; they are repeated
/// here because a default no code holds is a default nothing applies, and
/// [`bound_of`] reads the directive first in every case.
const DEFAULT_DEADLINE: Duration = Duration::from_secs(30);
/// See [`DEFAULT_DEADLINE`].
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// See [`DEFAULT_DEADLINE`]. `[http.client] idle`'s shipped value.
const DEFAULT_IDLE: Duration = Duration::from_secs(30);
/// See [`DEFAULT_DEADLINE`]. `[http.client] max_duration`'s shipped value.
const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(300);
/// § 6's base delay, which the same section leaves out of the block because it
/// is only reachable once a program has opted into retrying at all.
const DEFAULT_BACKOFF: Duration = Duration::from_millis(100);
/// See [`DEFAULT_DEADLINE`]. `[http.client] pool_idle`'s shipped value: enough
/// idle connections for a handful of upstream APIs, per core
/// (`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`).
const DEFAULT_POOL_IDLE: usize = 16;
/// See [`DEFAULT_DEADLINE`]. `[http.client] pool_idle_timeout`'s shipped value,
/// which sits under the idle timeout of every common proxy so that this client
/// is normally the side that closes — the side that cannot lose a request to
/// the race.
const DEFAULT_POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// `rule:http-server/no-spelling-for-an-unbounded-wait`'s request members, over `rule:security/outbound-url-is-a-sink`'s sink.
///
/// One shape, and one row per verb: the verb is the member's own name, which
/// is what makes § 7's idempotency question answerable while compiling.
/// `request` is the row for a verb that is not known until it runs — it takes a
/// `Core\Http\Method` case at slot 0, which is also how `Options` and `Trace`
/// are sent, and pays for it by answering that question at the call instead.
/// There is no request object anywhere in this: the bag is the whole of what a
/// call says. Every buffered row takes the same one, and `stream` takes it with
/// the two bounds a body read as it arrives has of its own
/// ([`STREAM_OPTIONS`]).
pub(crate) const CLIENT: CoreClass = CoreClass {
    name: CLIENT_NAME,
    methods: &[
        CoreMethod {
            name: "get",
            names: &["url"],
            params: &[URL, CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_http_client_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "post",
            names: &["url"],
            params: &[URL, CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_http_client_post",
            doc: Some(&POST_DOC),
        },
        CoreMethod {
            name: "put",
            names: &["url"],
            params: &[URL, CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_http_client_put",
            doc: Some(&PUT_DOC),
        },
        CoreMethod {
            name: "patch",
            names: &["url"],
            params: &[URL, CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_http_client_patch",
            doc: Some(&PATCH_DOC),
        },
        CoreMethod {
            name: "delete",
            names: &["url"],
            params: &[URL, CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_http_client_delete",
            doc: Some(&DELETE_DOC),
        },
        CoreMethod {
            name: "head",
            names: &["url"],
            params: &[URL, CoreTy::Options(OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_http_client_head",
            doc: Some(&HEAD_DOC),
        },
        CoreMethod {
            name: "request",
            names: &["method", "url"],
            params: &[
                CoreTy::Enum(crate::router::METHOD_NAME),
                URL,
                CoreTy::Options(OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_http_client_request",
            doc: Some(&REQUEST_DOC),
        },
        // The one row that answers something other than a `Core\Http\Response`:
        // a reply read as it arrives rather than as a value, whose class and
        // whose four readers are [`stream`]'s. The verb is a parameter here for
        // `request`'s reason and one of its own — a streamed reply is usually a
        // `POST` — and the bag is the only one on this class that is not
        // [`OPTIONS`], because `idle` and `maxDuration` bound a body no other
        // row has.
        CoreMethod {
            name: "stream",
            names: &["method", "url"],
            params: &[
                CoreTy::Enum(crate::router::METHOD_NAME),
                URL,
                CoreTy::Options(STREAM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(stream::STREAM_NAME),
            symbol: "nvs_core_http_client_stream",
            doc: Some(&STREAM_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// What every request member answers with — `rule:http-server/no-spelling-for-an-unbounded-wait` and `rule:http-server/retry-is-opt-in-jittered-and-closed`'s reply, as the
/// slots a transport fills and the members that read them back.
///
/// `status` is a member rather than a property and `text` answers a `tainted`
/// string; this module's own docs are the home of both decisions. **The body
/// slot holds octets and `text` is what asks whether they are UTF-8**, so a
/// reply that is not text is a reply `bytes` reads and `text` refuses, rather
/// than a call that failed before either was written. Each slot arrives with
/// the member that reads it —
/// `a_class_with_slots_has_instance_members_and_the_reverse` is that rule — so
/// the header map is a slot because `header` and `headers` are members.
///
/// **One slot carries three readers**, because what a program does with a body
/// is not a property of the body: `bytes` asks nothing of the octets, `text`
/// asks whether they are UTF-8, and `jsonAs<T>` reads them as one JSON
/// document and hands back the `T` it spells. None of the three keeps anything,
/// so any of them may follow any other over the one slot the transport filled.
///
/// **The header pair is two members and not one**, because a field the origin
/// sent twice is two values: `header` joins them the way RFC 9110 § 5.3 makes
/// them equivalent, and `headers` is the reading that answer cannot be
/// recovered from. `Core\Request` splits the same question the same way, and
/// `Set-Cookie` is the field where the join is wrong rather than lossy, so
/// `header` refuses it by name.
pub(crate) const RESPONSE: CoreClass = CoreClass {
    name: RESPONSE_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "status",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_http_response_status",
            doc: Some(&STATUS_DOC),
        },
        CoreMethod {
            name: "text",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_http_response_text",
            doc: Some(&TEXT_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedBytes,
            symbol: "nvs_core_http_response_bytes",
            doc: Some(&BYTES_DOC),
        },
        CoreMethod {
            name: "jsonAs",
            names: &[],
            params: &[CoreTy::Options(crate::json::DECODE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Written("T"),
            symbol: "nvs_core_http_response_json_as",
            doc: Some(&JSON_AS_DOC),
        },
        CoreMethod {
            name: "header",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_http_response_header",
            doc: Some(&HEADER_DOC),
        },
        CoreMethod {
            name: "headers",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::TaintedStr),
            symbol: "nvs_core_http_response_headers",
            doc: Some(&HEADERS_DOC),
        },
    ],
    slots: &["status", "body", "headers"],
    constants: &[],
};

/// [`RESPONSE`]'s status slot, by index — the layout its `slots` names, which
/// `CoreClass::slot` is the check on.
const STATUS_SLOT: usize = 0;
/// [`RESPONSE`]'s body slot. See [`STATUS_SLOT`].
const BODY_SLOT: usize = 1;
/// [`RESPONSE`]'s header slot, holding [`header_map`]'s array. See
/// [`STATUS_SLOT`].
const HEADERS_SLOT: usize = 2;

/// The field `header` will not join, and the member it names instead.
///
/// A `Set-Cookie` line is not a comma-separated list: RFC 6265 § 3 exempts it
/// from RFC 9110 § 5.3's equivalence, and an `Expires` attribute writes a comma
/// of its own, so joining two of them produces a string that parses as neither
/// cookie. Every other field either does not repeat or repeats as a list.
const UNJOINABLE_FIELD: &str = "set-cookie";

/// `Core\Http\Response::status`'s reference card — `rule:core-api/reference-card`.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "The reply's HTTP status code, as the origin sent it and with nothing read into it — \
            replacing `curl_getinfo`'s `CURLINFO_RESPONSE_CODE` key.",
    params: &[],
    ret: "The status line's three-digit code. A `404` and a `500` are answers, so they arrive \
          here rather than as a throw; only a request that got no reply at all throws.",
    errors: &[],
};

/// `Core\Http\Response::text`'s reference card — `rule:core-api/reference-card`.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "The reply's body as text, replacing `curl_exec`'s return value and the \
            `CURLOPT_RETURNTRANSFER` flag that decided whether there was one.",
    params: &[],
    ret: "The body, `tainted`: it is bytes another host chose, and a pinned address settles where \
          they came from rather than what is in them. A sink's own launderer is the way out of \
          it, and there is no generic one.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The body is not valid UTF-8, so it is not a `string`. It is not repaired: a \
               replacement byte would hand a program a body that is not what the origin sent and \
               no way to tell. `bytes` reads the same body without asking.",
    }],
};

/// `Core\Http\Response::bytes`'s reference card — `rule:core-api/reference-card`.
const BYTES_DOC: MethodDoc = MethodDoc {
    short: "The reply's body as the octets that arrived, for the replies that are not text at all \
            — an image, an archive, a signature.",
    params: &[],
    ret: "Every byte of the body, `tainted` for `text`'s reason and asking nothing of them: a \
          reply that is not UTF-8 is read here and refused there, so which of the two a program \
          calls is what decides whether the question is asked.",
    errors: &[],
};

/// `Core\Http\Response::jsonAs`'s reference card — `rule:core-api/reference-card`.
const JSON_AS_DOC: MethodDoc = MethodDoc {
    short: "The reply's body hydrated into an instance of `T` — `Core\\Json::decodeAs` over the \
            octets `bytes` answers, carrying the same `{maxDepth?}` bag; write `array<T>` to read \
            a JSON array as one instance per element.",
    params: &[ParamDoc {
        name: "maxDepth",
        desc: "How deep the document may nest before it is refused, counted PHP's way: a scalar \
               document is depth 1.",
        shape: &[],
    }],
    ret: "A new `T` built from the document's fields, or one `T` per element for an `array<T>`. \
          Every text field of `T` has to declare `tainted`, because these are octets another host \
          chose and the field is where they land.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`T` carries no `#[Json\\Derive]` codec to decode into, or a `maxDepth` outside \
                   1..=1024 was asked for.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "The body is not one whole JSON document at that depth — which includes a reply \
                   with no body at all and one that is not UTF-8 — or it is not the object `T` \
                   decodes from, or its fields are missing or of the wrong type. Every failed \
                   field is one issue on the error, at its own path. What the reply declared as \
                   its `Content-Type` is not consulted either way.",
        },
    ],
};

/// `Core\Http\Response::header`'s reference card — `rule:core-api/reference-card`.
const HEADER_DOC: MethodDoc = MethodDoc {
    short: "One reply header, read by a name that matches however the origin capitalised it — \
            replacing `curl_getinfo`'s header string and the hand-written parse under it.",
    params: &[ParamDoc {
        name: "name",
        desc: "The field name, matched case-insensitively as RFC 9110 § 5.1 defines it.",
        shape: &[],
    }],
    ret: "The field's value, `tainted` as every byte another host chose is, with a field the \
          origin sent more than once joined by `, ` in arrival order; `null` where the reply \
          carried no such field.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`Set-Cookie`, which is the one field that does not join — two cookies read as one \
               are neither, so `headers` is what reads it.",
    }],
};

/// `Core\Http\Response::headers`'s reference card — `rule:core-api/reference-card`.
const HEADERS_DOC: MethodDoc = MethodDoc {
    short: "Every line the reply carried under one name, kept apart — the reading a joined value \
            cannot be recovered from, and the only way to read `Set-Cookie`.",
    params: &[ParamDoc {
        name: "name",
        desc: "The field name, matched case-insensitively exactly as `header` matches it.",
        shape: &[],
    }],
    ret: "One `tainted` entry per line, in arrival order, and an empty array where the reply \
          carried no such field — so a field that arrived once answers a list of one rather than \
          anything a caller has to tell apart.",
    errors: &[],
};

/// The parameters every row of [`CLIENT`] documents — one bag, so the options
/// are described once rather than once per verb, and a reader comparing two
/// members finds no difference because there is none.
///
/// A macro rather than a slice because a card's `params` begins with its own
/// row's `names` — `a_documented_rows_param_docs_agree_with_its_names` holds
/// it there — and `request` names a verb ahead of the URL, so one card's list
/// is the other's with an entry in front. A `const` slice cannot be extended,
/// and a second copy of the options would say the same thing until it did not.
// The entries a card names ahead of the shared ones — `request`'s verb, or
// nothing at all — then the list itself, at the indentation it had as a slice,
// because rustfmt does not reach inside a macro's body, then the entries a card
// names after them, which is [`STREAM_OPTIONS`]' two bounds.
macro_rules! request_params {
    ($($leading:expr),* $(,)?) => { request_params!($($leading),* ; ) };
    ($($leading:expr),* ; $($trailing:expr),* $(,)?) => { &[
    $($leading,)*
    ParamDoc {
        name: "url",
        desc: "Where the request goes: a URL the program itself authored, or the \
               `Core\\Http\\Target` that `Core\\Http::allowUrl` pinned. A `tainted` value is \
               refused here and accepted only at that launderer.",
        shape: &[],
    },
    ParamDoc {
        name: "deadline",
        desc: "The whole call's budget, covering the connection, every redirect hop, every retry \
               attempt and every backoff between them. Omitted, the runtime's `[http.client] \
               deadline` applies; there is no spelling for no deadline at all.",
        shape: &[],
    },
    ParamDoc {
        name: "connectTimeout",
        desc: "How long the connection alone may take, inside `deadline` rather than beside it.",
        shape: &[],
    },
    ParamDoc {
        name: "headers",
        desc: "Extra request headers, by name. A `secret` is admitted here — a credential has to \
               reach the API it authenticates to — and a `tainted` value is not. The runtime's \
               own headers are added around these.",
        shape: &[],
    },
    ParamDoc {
        name: "followRedirects",
        desc: "How many redirect hops to follow. Omitted, the runtime's `[http.client] \
               max_redirects` applies, and that is `0` with nothing configured: each hop is \
               re-checked and re-pinned against the outbound policy.",
        shape: &[],
    },
    ParamDoc {
        name: "retryAttempts",
        desc: "The total number of attempts including the first, so `1` is the default behaviour \
               written out and `0` is refused. Retries are jittered and share the one deadline.",
        shape: &[],
    },
    ParamDoc {
        name: "retryBackoff",
        desc: "The base delay retries grow from, exponentially and with full jitter. Omitted, it \
               is `100ms`; the jitter is not configurable.",
        shape: &[],
    },
    ParamDoc {
        name: "retryIdempotencyKey",
        desc: "Sent as `Idempotency-Key`, identical across attempts. Required for `post` and \
               `patch` when `retryAttempts` is given, and accepted by every other member.",
        shape: &[],
    },
    ParamDoc {
        name: "json",
        desc: "The value to send as `application/json`, encoded once for the whole call. A \
               `secret` inside it is sent, as it is at a header, and a written `null` is the \
               document `null` rather than no body.",
        shape: &[],
    },
    ParamDoc {
        name: "form",
        desc: "The fields to send as `application/x-www-form-urlencoded`, by name.",
        shape: &[],
    },
    ParamDoc {
        name: "body",
        desc: "The octets to send exactly as given, under `contentType`. At most one of `json`, \
               `form`, `body` and `multipart` may be written, and none of them on `get` or \
               `head`; both refusals are made while compiling.",
        shape: &[],
    },
    ParamDoc {
        name: "contentType",
        desc: "The media type `body`'s octets are sent under. It means nothing without `body`, \
               and writing it alone is refused while compiling.",
        shape: &[],
    },
    ParamDoc {
        name: "multipart",
        desc: "The parts to send as `multipart/form-data`, by name.",
        shape: &[],
    },
    $($trailing,)*
    ] };
}

/// Every row but `request`'s: the URL, then the bag — see [`request_params`].
const REQUEST_PARAMS: &[ParamDoc] = request_params!();

/// `request`'s: the verb it is handed, ahead of what every other row documents.
const DYNAMIC_PARAMS: &[ParamDoc] = request_params!(METHOD_PARAM);

/// `stream`'s: `request`'s list, and after it the two bounds that row's own bag
/// declares — see [`STREAM_OPTIONS`].
const STREAM_PARAMS: &[ParamDoc] = request_params!(METHOD_PARAM ; IDLE_PARAM, MAX_DURATION_PARAM);

/// The silence bound `stream` carries — see [`STREAM_PARAMS`].
const IDLE_PARAM: ParamDoc = ParamDoc {
    name: "idle",
    desc: "The longest the body may go silent for once the head has arrived. Omitted, the \
           runtime's `[http.client] idle` applies; there is no spelling for no bound at all.",
    shape: &[],
};

/// The lifetime bound `stream` carries — see [`STREAM_PARAMS`].
const MAX_DURATION_PARAM: ParamDoc = ParamDoc {
    name: "maxDuration",
    desc: "The longest the body may take altogether, which is what ends an origin dribbling a \
           byte at a time under every idle check. Omitted, the runtime's `[http.client] \
           max_duration` applies. `deadline` covers the connection and the head and stops there.",
    shape: &[],
};

/// The verb `request` carries — see [`DYNAMIC_PARAMS`].
const METHOD_PARAM: ParamDoc = ParamDoc {
    name: "method",
    desc: "The verb to send, as a `Core\\Http\\Method` case. A `Post` or a `Patch` retried without \
           `retryIdempotencyKey` throws before the first attempt, which the member whose verb is \
           its own name refuses while compiling instead.",
    shape: &[],
};

/// What every request member answers, once — see [`REQUEST_PARAMS`].
const REQUEST_RET: &str = "A `Core\\Http\\Response` carrying the status and the body of the reply. \
                           A `404` and a `500` are answers and arrive here; only a request that \
                           got no reply at all throws. An `https` URL is fetched over TLS, with \
                           the certificate verified against the authorities Novis carries.";

/// What every request member throws, once — see [`REQUEST_PARAMS`].
const REQUEST_ERRORS: &[ErrorDoc] = &[
    ErrorDoc {
        error: "RuntimeError",
        desc: "The URL is refused: it is not a URL, its scheme is neither `http` nor `https`, it \
               names no host, `net.connect` does not grant that host, or it resolves to a \
               loopback, private, link-local or unspecified address that `net.internal` does not \
               name. An option is outside its \
               bounds: a `deadline`, `connectTimeout` or `retryBackoff` — or, on the row that \
               declares them, an `idle` or `maxDuration` — that is not a positive \
               duration, or a `retryAttempts` of zero. A header name or value carries a control \
               byte, which would end the line early. An `https` host presented a certificate that \
               does not verify against the authorities Novis carries, or one that is not valid for \
               that name. Or the reply is not HTTP or is larger than one request may hold — \
               a body that is not text is not one of these, and is refused at \
               `Core\\Http\\Response::text` rather than here. Where the verb is handed in rather than \
               named — `request` — the two questions the other rows answer while compiling are \
               asked before the first attempt instead: a `Get` or a `Head` given a body key, and \
               a `Post` or a `Patch` asking for retries without `retryIdempotencyKey`.",
    },
    ErrorDoc {
        error: "TimeoutError",
        desc: "The `deadline` passed before there was an answer. It covers the connection, every \
               redirect hop, every retry attempt and every backoff between them, and a backoff \
               that would end past it throws at once rather than sleeping first.",
    },
    ErrorDoc {
        error: "IOError",
        desc: "The last attempt could not reach the pinned address, or the connection failed while \
               the request was being sent or the reply read.",
    },
];

/// `Core\Http\Client::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Fetches `$url` under a finite budget, over the address the outbound policy pinned.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::post`'s reference card — `rule:core-api/reference-card`.
const POST_DOC: MethodDoc = MethodDoc {
    short: "Sends a `POST` to `$url` under a finite budget. Its retries need \
            `retryIdempotencyKey`, because a repeated `POST` is a second effect rather than a \
            second question.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::put`'s reference card — `rule:core-api/reference-card`.
const PUT_DOC: MethodDoc = MethodDoc {
    short: "Sends a `PUT` to `$url` under a finite budget. Idempotent by definition, so its \
            retries need no key.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::patch`'s reference card — `rule:core-api/reference-card`.
const PATCH_DOC: MethodDoc = MethodDoc {
    short: "Sends a `PATCH` to `$url` under a finite budget. A partial update repeated is a second \
            effect, so its retries need `retryIdempotencyKey` exactly as `post`'s do.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::delete`'s reference card — `rule:core-api/reference-card`.
const DELETE_DOC: MethodDoc = MethodDoc {
    short: "Sends a `DELETE` to `$url` under a finite budget.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::head`'s reference card — `rule:core-api/reference-card`.
const HEAD_DOC: MethodDoc = MethodDoc {
    short: "Asks `$url` for its headers alone, under the same budget a `get` would have.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::request`'s reference card — `rule:core-api/reference-card`.
const REQUEST_DOC: MethodDoc = MethodDoc {
    short: "Sends `$method` to `$url` under a finite budget — the row for a verb chosen at run \
            time, where every other one is a verb of its own. There is no request object: the \
            same bag the named rows take is the whole of what a call says.",
    params: DYNAMIC_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::stream`'s reference card — `rule:core-api/reference-card`.
const STREAM_DOC: MethodDoc = MethodDoc {
    short: "Sends `$method` to `$url` and answers once the head has arrived, leaving the body to \
            be read as it comes — the row for a reply a program works through rather than holds, \
            such as a server-sent event stream or a result set a line at a time.",
    params: STREAM_PARAMS,
    ret: "A `Core\\Http\\Stream` whose `status()`, `header()` and `headers()` answer the head, and \
          whose body is read by exactly one of `events()`, `lines()`, `chunks()` and `saveTo()`.",
    errors: REQUEST_ERRORS,
};

/// One of § 5's time bounds, judged: present and positive, or left out.
///
/// # Errors
///
/// A thrown `RuntimeError` naming the option for a bound that is not positive —
/// `rule:http-server/no-spelling-for-an-unbounded-wait` has no spelling for an unbounded wait, and a zero one is that
/// spelling said quietly. A [`Fault::fatal`] for a slot that is neither a
/// `Duration` nor `Tag::Null`, which the row's own type rules out.
fn judge_bound(args: &[Value], at: usize, option: &str, member: &str) -> Result<(), Fault> {
    if matches!(args[at].tag(), Some(Tag::Null)) {
        return Ok(());
    }
    let nanos = crate::time::nanos_of(args, at, option)?;
    if nanos <= 0 {
        return Err(Fault::thrown(format!(
            "{member}: `{option}` must be a positive duration, and this one is {nanos}ns"
        )));
    }
    Ok(())
}

/// The two questions the verb decides, asked of the verb a call actually
/// carries: a body on a verb that carries none, and a retried effect with no
/// key to recognise it by.
///
/// Dead on every row whose verb is its own name, because the checker answers
/// both while compiling from `registry::request_body_rule` and
/// `registry::idempotent_retry_rule` — the same two spellings, read once. It is
/// `request` that reaches here, where the verb arrives as an argument and
/// neither question can be put to a call site: asked before the first attempt,
/// so a test run finds it rather than production finding it on the one retry
/// that matters (`rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`).
///
/// # Errors
///
/// A thrown `RuntimeError` naming the key for a body written under a `GET` or a
/// `HEAD`, and one naming `retryIdempotencyKey` for a `POST` or a `PATCH` that
/// asked for retries without it.
fn judge_verb(args: &[Value], verb: &str, member: &str) -> Result<(), Fault> {
    if matches!(verb, "GET" | "HEAD")
        && let Some(key) = written_body_key(args)
    {
        return Err(Fault::thrown(format!(
            "{member}: a `{verb}` sends no body, and `{key}` is one — put the value in the URL's \
             query, or send it under a verb that carries a body"
        )));
    }
    if matches!(verb, "POST" | "PATCH")
        && !matches!(args[RETRY_ATTEMPTS].tag(), Some(Tag::Null))
        && args[RETRY_KEY].as_text().is_none()
    {
        return Err(Fault::thrown(format!(
            "{member}: a repeated `{verb}` is a second effect rather than a second question, so \
             retrying one needs `retryIdempotencyKey` in the same bag, sent as `Idempotency-Key` \
             and identical across attempts"
        )));
    }
    Ok(())
}

/// Which of [`BODY_OPTIONS`] this call wrote, in [`body_of`]'s own order, or
/// `None` for a call that wrote none.
///
/// The order is that function's because the two answer one question — which key
/// frames this body — and a second reading free to pick a different one would
/// report a key that is not the key being sent.
fn written_body_key(args: &[Value]) -> Option<&'static str> {
    if !matches!(args[JSON].tag(), Some(Tag::Unset)) {
        return Some(JSON_OPTION);
    }
    if args[FORM].array_ptr().is_some() {
        return Some(FORM_OPTION);
    }
    if args[MULTIPART].array_ptr().is_some() {
        return Some(MULTIPART_OPTION);
    }
    if matches!(args[BODY].tag(), Some(Tag::Object)) || octets_of(&args[BODY]).is_some() {
        return Some(BODY_OPTION);
    }
    None
}

/// § 6's attempt count, judged: at least one, or left out.
///
/// # Errors
///
/// A thrown `RuntimeError` for a count of zero, and a [`Fault::fatal`] for a
/// slot that is neither a `uint` nor `Tag::Null`.
fn judge_attempts(args: &[Value], member: &str) -> Result<(), Fault> {
    if matches!(args[RETRY_ATTEMPTS].tag(), Some(Tag::Null)) {
        return Ok(());
    }
    let attempts = args[RETRY_ATTEMPTS].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `uint` for `retryAttempts`, got tag {}",
            args[RETRY_ATTEMPTS].tag_byte()
        ))
    })?;
    if attempts == 0 {
        return Err(Fault::thrown(format!(
            "{member}: `retryAttempts` counts the first attempt too, so the smallest it can be \
             is 1"
        )));
    }
    Ok(())
}

/// One of § 5's time bounds as the transport wants it: the option if it was
/// given, then the `[http.client]` directive, then `fallback`.
///
/// The three-step order is § 5's "the absence is the decision" read forwards —
/// leaving a field out inherits the deployment's bound rather than removing it,
/// and a deployment that configured nothing inherits the shipped one.
///
/// A bag that is **narrower** than `at` reads as a bag that left the field out,
/// which is what [`IDLE`] and [`MAX_DURATION`] need: they sit past the last
/// slot of [`OPTIONS`], so every buffered row asks for them off the end and
/// gets the directive's value, while the one row that declares them reads what
/// the call wrote.
///
/// # Errors
///
/// [`judge_bound`]'s, for an option that is not a positive duration. A
/// *directive* that will not parse is not an error here: `nvs.toml` is
/// validated where it is loaded, and a second refusal at the call site would
/// fail a request over a key the operator can no longer see.
fn bound_of(
    ctx: &Ctx,
    args: &[Value],
    at: usize,
    option: &str,
    directive: &str,
    fallback: Duration,
) -> Result<Duration, Fault> {
    if args
        .get(at)
        .is_some_and(|held| !matches!(held.tag(), Some(Tag::Null)))
    {
        let nanos = crate::time::nanos_of(args, at, option)?;
        return Ok(Duration::from_nanos(nanos.unsigned_abs()));
    }
    let configured = ctx
        .config()
        .and_then(|config| config.get(directive))
        .and_then(|text| duration::parse(&text).ok())
        .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()));
    Ok(configured.unwrap_or(fallback))
}

/// The `headers` bag at slot `at`, copied out as the lines the request will
/// carry, with every name as the caller wrote it.
///
/// Copied rather than borrowed for `crate::str`'s reason: a key arrives as its
/// own reference, and holding one per entry across the exchange would owe a
/// release on every early return under it.
///
/// The slot is a parameter because two bags have this shape and one walk reads
/// both: this class's request headers, and the reply headers
/// `Core\Test::answerHttp` registers (`rule:testing/an-outbound-call-is-answered-from-a-table`).
/// A caller that wants them lower-cased says so at its own call site, since a
/// request carries the names the program wrote.
///
/// # Errors
///
/// A [`Fault::fatal`] for an argument or an element that is not text, both
/// ruled out by the row's `array<string>` and so unreachable from source.
pub(crate) fn headers_of(
    args: &[Value],
    at: usize,
    member: &str,
) -> Result<Vec<(String, String)>, Fault> {
    let Some(array) = args[at].array_ptr() else {
        return Err(Fault::fatal(format!(
            "{member} expected {:?} for `headers`, got tag {}",
            Tag::Array,
            args[at].tag_byte()
        )));
    };
    let mut headers = Vec::new();
    let mut from = 0_usize;
    loop {
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live allocation, \
                      so it is live for the length of this call, and `from` only \
                      ever advances past a slot this same cursor reported"
        )]
        let (slot, name, value) = unsafe {
            let slot = nvs_runtime::nvs_array_next_slot(array, from);
            let Ok(slot) = usize::try_from(slot) else {
                break;
            };
            let name = NvsStr::from_raw(nvs_runtime::nvs_array_key_at(array, slot));
            let mut value = Value::null();
            nvs_runtime::nvs_array_value_at(array, slot, &raw mut value);
            (slot, name, value)
        };
        from = slot + 1;

        // An array key is `int|string` and neither can be invalid UTF-8, for
        // the reasons `Core\Str::replaceAll`'s own cursor states in full.
        let name = std::str::from_utf8(name.as_bytes())
            .map_err(|_| Fault::fatal(format!("{member} found a header name that is not text")))?;
        let text = value.as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `string` header value, got tag {}",
                value.tag_byte()
            ))
        })?;
        headers.push((name.to_owned(), text.to_owned()));
    }
    Ok(headers)
}

/// The URL to send to and the address it was approved at.
///
/// A `Target` argument was pinned by the launderer that built it, and asking
/// again would be the second resolution `rule:http-server/allow-url-pins-the-address` exists to remove — so its
/// two slots are read back here and no name is looked up. A plain `string` is
/// the form § 1 keeps for a URL the program authored, and it goes through the
/// same door.
///
/// # Errors
///
/// [`pin`]'s four for a `string`. A [`Fault::fatal`] for an argument of another
/// shape or a target whose slots this crate did not write, both unreachable
/// from source.
fn approved(ctx: &mut Ctx, args: &[Value], member: &str) -> Result<(String, IpAddr), Fault> {
    let url = given_url(args, member)?;
    if !matches!(args[0].tag(), Some(Tag::Object)) {
        let address = pin(ctx, &url, member)?;
        return Ok((url, address));
    }

    let target = crate::instance::receiver(args[0], &TARGET, member)?;
    let address = crate::instance::slot(target, TARGET_ADDRESS_SLOT);
    let address = address
        .as_text()
        .and_then(|text| text.parse::<IpAddr>().ok())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} found a `Core\\Http\\Target` it cannot read"
            ))
        })?;
    Ok((url, address))
}

/// The URL this call names, whichever arm of the union carried it.
///
/// Separate from [`approved`] because the answer table asks only this half:
/// a faked call compares the *text* against its rows and never learns an
/// address, so the two questions cannot be one function without the faked path
/// resolving a name it will not connect to.
///
/// # Errors
///
/// A [`Fault::fatal`] for an argument of another shape or a target whose slots
/// this crate did not write, both unreachable from source.
fn given_url(args: &[Value], member: &str) -> Result<String, Fault> {
    if !matches!(args[0].tag(), Some(Tag::Object)) {
        return Ok(args[0]
            .as_text()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{member} expected a `string` or a `Core\\Http\\Target`, got tag {}",
                    args[0].tag_byte()
                ))
            })?
            .to_owned());
    }
    let target = crate::instance::receiver(args[0], &TARGET, member)?;
    let url = crate::instance::slot(target, TARGET_URL_SLOT);
    Ok(url
        .as_text()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} found a `Core\\Http\\Target` it cannot read"
            ))
        })?
        .to_owned())
}

/// `rule:http-server/an-outbound-request-carries-one-body`'s body, framed: which
/// of the four keys the call wrote is what says how the octets are composed and
/// what type they are sent under.
///
/// **At most one of them is written**, and that is a diagnostic rather than a
/// check here — [`crate::registry::request_body_rule`] hands the same spellings
/// to `nvs_types::expr::args`, which refuses two keys at one call and any of
/// them on a verb that carries no body. So this reads them in [`OPTIONS`]' own
/// order and takes the first it finds, and there is no "and also" branch for a
/// combination the compiler has already refused.
///
/// The whole body is built **once per call**, before the first connection: a
/// retry resends these pieces rather than encoding the document again, and a
/// file is opened once here — where a `Ctx` exists to ask the capability door —
/// and rewound per attempt.
///
/// # Errors
///
/// [`crate::json::written`]'s `LogicError` for a `json` value this encoder
/// refuses, [`nvs_runtime::capability::open_read`]'s refusal and `IOError` for a
/// file part, and a thrown `RuntimeError` for a field name or filename that
/// could end a header line early.
fn body_of(ctx: &Ctx, args: &[Value], member: &str) -> Result<Option<transport::Body>, Fault> {
    if !matches!(args[JSON].tag(), Some(Tag::Unset)) {
        // `Tag::Unset` and not `Tag::Null`: the document `null` is a body a
        // program may mean, which is what [`OPTIONS`]' `json` row states.
        let document = crate::json::written(args[JSON], member)?;
        return Ok(Some(held(
            Some("application/json".to_owned()),
            document.into_bytes(),
        )));
    }
    if args[FORM].array_ptr().is_some() {
        let mut encoded = String::new();
        for (name, value) in fields_of(args, FORM, "form", member)? {
            let text = value.as_text().ok_or_else(|| {
                Fault::fatal(format!(
                    "{member} expected a `string` in `form`, got tag {}",
                    value.tag_byte()
                ))
            })?;
            if !encoded.is_empty() {
                encoded.push('&');
            }
            encoded.push_str(&crate::uri::encode(
                name.as_bytes(),
                crate::uri::Form::FormValue,
            ));
            encoded.push('=');
            encoded.push_str(&crate::uri::encode(
                text.as_bytes(),
                crate::uri::Form::FormValue,
            ));
        }
        return Ok(Some(held(
            Some("application/x-www-form-urlencoded".to_owned()),
            encoded.into_bytes(),
        )));
    }
    if args[MULTIPART].array_ptr().is_some() {
        return multipart(ctx, args, member).map(Some);
    }
    if matches!(args[BODY].tag(), Some(Tag::Object)) {
        let part = part_of(ctx, args[BODY], member)?;
        // The call's own `contentType` wins over the part's: a part carries one
        // so that a multipart field can be typed, and a body written beside an
        // explicit key is the program saying what it means this time.
        let content_type = args[CONTENT_TYPE]
            .as_text()
            .map(str::to_owned)
            .or(part.content_type);
        return Ok(Some(transport::Body {
            content_type,
            length: part.length,
            pieces: vec![part.piece],
        }));
    }
    if let Some(octets) = octets_of(&args[BODY]) {
        return Ok(Some(held(
            args[CONTENT_TYPE].as_text().map(str::to_owned),
            octets.to_vec(),
        )));
    }
    Ok(None)
}

/// A body that is one run of octets the framing is holding.
fn held(content_type: Option<String>, octets: Vec<u8>) -> transport::Body {
    transport::Body {
        content_type,
        length: octets.len() as u64,
        pieces: vec![transport::Piece::Held(octets)],
    }
}

/// `multipart/form-data` over the map at [`MULTIPART`]: one segment per field,
/// a part sent under its own name and type, and a boundary no field carries.
///
/// The boundary is drawn at random per call rather than derived from what is
/// being sent. A predictable one is a body an attacker can write a second part
/// into — the field content is theirs whenever an upload is — and checking that
/// a derived boundary does not occur in the content would mean reading the file
/// this framing exists in order not to read.
///
/// # Errors
///
/// [`part_of`]'s, and a thrown `RuntimeError` for a field name or filename
/// holding a quote or a control byte.
fn multipart(ctx: &Ctx, args: &[Value], member: &str) -> Result<transport::Body, Fault> {
    let boundary = format!(
        "----NovisBoundary{:016x}{:016x}",
        rand::rng().random_range(0..=u64::MAX),
        rand::rng().random_range(0..=u64::MAX)
    );
    let mut pieces: Vec<transport::Piece> = Vec::new();
    let mut length = 0_u64;

    for (name, value) in fields_of(args, MULTIPART, "multipart", member)? {
        quotable(&name, "field name", member)?;
        let mut head = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"");
        if matches!(value.tag(), Some(Tag::Object)) {
            let part = part_of(ctx, value, member)?;
            let filename = part.filename.unwrap_or_else(|| name.clone());
            let content_type = part
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned());
            quotable(&filename, "filename", member)?;
            quotable(&content_type, "media type", member)?;
            head.push_str(&format!(
                "; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            ));
            length += head.len() as u64 + part.length;
            pieces.push(transport::Piece::Held(head.into_bytes()));
            pieces.push(part.piece);
        } else {
            let text = value.as_text().ok_or_else(|| {
                Fault::fatal(format!(
                    "{member} expected a `string` or a `Core\\Http\\Part` in `multipart`, got \
                     tag {}",
                    value.tag_byte()
                ))
            })?;
            head.push_str("\r\n\r\n");
            head.push_str(text);
            length += head.len() as u64;
            pieces.push(transport::Piece::Held(head.into_bytes()));
        }
        length += 2;
        pieces.push(transport::Piece::Held(b"\r\n".to_vec()));
    }

    let close = format!("--{boundary}--\r\n").into_bytes();
    length += close.len() as u64;
    pieces.push(transport::Piece::Held(close));

    Ok(transport::Body {
        content_type: Some(format!("multipart/form-data; boundary={boundary}")),
        length,
        pieces,
    })
}

/// `text` refused where it could end a multipart part's own header early.
///
/// [`transport::field`]'s refusal one level down, and a priority-1 one for the
/// same reason: a quote closes the `name="…"` a segment is identified by, and a
/// control byte ends the line, so either is a second part the caller did not
/// write. There is no escaping that makes one safe here either — RFC 7578 gives
/// none — only a rejection that makes it visible.
///
/// # Errors
///
/// A thrown `RuntimeError` naming the text and what it was being used as.
fn quotable(text: &str, what: &str, member: &str) -> Result<(), Fault> {
    if text
        .bytes()
        .any(|byte| byte < 0x20 || byte == 0x7f || byte == b'"')
    {
        return Err(Fault::thrown(format!(
            "{member}: `{text}` is not a {what} this request can carry — a quote or a control \
             byte in it would end the part's own header early, which is a second part the caller \
             did not write"
        )));
    }
    Ok(())
}

/// One [`PART`] read back out: what it sends, how long it is, and the two
/// things it says about itself.
struct Framed {
    /// The octets, or the file they will be read from.
    piece: transport::Piece,
    /// How many of them, which is what the head promises.
    length: u64,
    /// The name the other end is told, where the part carries one.
    filename: Option<String>,
    /// The media type it is sent under, where the part names one.
    content_type: Option<String>,
}

/// The [`PART`] in `value`, opened if it names a file.
///
/// # Errors
///
/// [`nvs_runtime::capability::open_read`]'s refusal and `IOError`, and a
/// [`Fault::fatal`] for a value that is not a part or a part whose slots this
/// crate did not write — both unreachable from source.
fn part_of(ctx: &Ctx, value: Value, member: &str) -> Result<Framed, Fault> {
    let part = crate::instance::receiver(value, &PART, member)?;
    let filename = crate::instance::slot(part, PART_FILENAME_SLOT)
        .as_text()
        .map(str::to_owned);
    let content_type = crate::instance::slot(part, PART_CONTENT_TYPE_SLOT)
        .as_text()
        .map(str::to_owned);

    let path = crate::instance::slot(part, PART_PATH_SLOT);
    if let Some(path) = path.as_text() {
        // The door again, and at the open rather than at the `stat`: the
        // descriptor this returns is the one every attempt writes from, so the
        // octets sent are the ones the grant was checked against.
        let file = nvs_runtime::capability::open_read(ctx, std::path::Path::new(path), member)?;
        let length = file
            .metadata()
            .map_err(|err| {
                Fault::thrown(format!(
                    "{member}: the size of `{path}` could not be read, and it is the \
                     `Content-Length` this request has to send — {err}"
                ))
            })?
            .len();
        return Ok(Framed {
            piece: transport::Piece::File {
                handle: std::cell::RefCell::new(Box::new(file)),
                length,
            },
            length,
            filename,
            content_type,
        });
    }

    let data = crate::instance::slot(part, PART_DATA_SLOT);
    let octets = octets_of(&data)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} found a `Core\\Http\\Part` it cannot read"
            ))
        })?
        .to_vec();
    Ok(Framed {
        length: octets.len() as u64,
        piece: transport::Piece::Held(octets),
        filename,
        content_type,
    })
}

/// The octets in `value`, whichever of the two arms of [`OCTETS`] carried them,
/// or `None` for a value that is neither.
fn octets_of(value: &Value) -> Option<&[u8]> {
    value.as_str_bytes().or_else(|| value.as_bytes())
}

/// The `array<string, …>` at `at`, as its keys and its values in written order.
///
/// [`headers_of`]'s walk over a map whose values are not all text: a multipart
/// field is a `string` or a `Core\Http\Part`, so the reading is left to the
/// caller and only the key is text here.
///
/// # Errors
///
/// A [`Fault::fatal`] for an argument that is not an array or a key that is not
/// text, both ruled out by the row's own type.
fn fields_of(
    args: &[Value],
    at: usize,
    key: &str,
    member: &str,
) -> Result<Vec<(String, Value)>, Fault> {
    let Some(array) = args[at].array_ptr() else {
        return Err(Fault::fatal(format!(
            "{member} expected {:?} for `{key}`, got tag {}",
            Tag::Array,
            args[at].tag_byte()
        )));
    };
    let mut fields = Vec::new();
    let mut from = 0_usize;
    loop {
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live allocation, \
                      so it is live for the length of this call, and `from` only \
                      ever advances past a slot this same cursor reported"
        )]
        let (slot, name, value) = unsafe {
            let slot = nvs_runtime::nvs_array_next_slot(array, from);
            let Ok(slot) = usize::try_from(slot) else {
                break;
            };
            let name = NvsStr::from_raw(nvs_runtime::nvs_array_key_at(array, slot));
            let mut value = Value::null();
            nvs_runtime::nvs_array_value_at(array, slot, &raw mut value);
            (slot, name, value)
        };
        from = slot + 1;

        // An array key is `int|string` and neither can be invalid UTF-8, for
        // the reasons `Core\Str::replaceAll`'s own cursor states in full.
        let name = std::str::from_utf8(name.as_bytes())
            .map_err(|_| Fault::fatal(format!("{member} found a `{key}` key that is not text")))?;
        fields.push((name.to_owned(), value));
    }
    Ok(fields)
}

/// Every request member's body: the URL through the outbound policy, the
/// options through § 5's bounds, and then the exchange itself.
///
/// The order is the contract. Everything a caller can get wrong is decided
/// before anything leaves the process, so a program's own tests find a refused
/// URL or an impossible deadline without a network — and the clock the deadline
/// is measured from starts once all of it has passed, since a budget spent
/// judging arguments is not a budget the other end was given.
///
/// # Errors
///
/// [`pin`]'s four, [`judge_bound`]'s, [`judge_attempts`]' and [`judge_verb`]',
/// and then [`transport::send`]'s.
fn request(ctx: &mut Ctx, args: &[Value], member: &str, verb: &str) -> Result<Value, Fault> {
    let (status, body, headers) = exchanged(ctx, args, member, verb, false)?;
    Ok(crate::instance::build(
        &RESPONSE,
        [Value::int(status), body, headers],
    ))
}

/// One exchange, as its status, the value its answer's body slot holds and
/// [`header_map`]'s array — every request member's whole body, ahead of the
/// class that reads the answer back.
///
/// The split is [`stream::STREAM`]'s: a buffered reply and a streamed one are
/// the same call, checked the same way and sent over the same transport, and
/// differ only in which class the answer is built into. A second copy of the
/// reading would be a second place for a bound to be judged, which is the one
/// mistake here that still connects.
///
/// The body slot is where the two classes part: a `Core\Http\Response` holds
/// the octets, which are all here by the time this returns, and a
/// `Core\Http\Stream` holds [`filed`]'s key to a reader the rest of the reply
/// is still arriving on.
///
/// # Errors
///
/// [`approved`]'s refusals, [`judge_bound`]'s, [`judge_attempts`]',
/// [`judge_verb`]'s, [`body_of`]'s and whatever [`transport::send`] raised — or,
/// where a test has armed the answer table, [`faked`]'s.
fn exchanged(
    ctx: &mut Ctx,
    args: &[Value],
    member: &str,
    verb: &str,
    streamed: bool,
) -> Result<(i64, Value, Value), Fault> {
    let named = format!("{CLIENT_NAME}::{member}");
    if ctx.faked_http().is_armed() {
        return faked(ctx, args, &named, verb, streamed);
    }
    let (url, address) = approved(ctx, args, &named)?;

    judge_bound(args, DEADLINE, "deadline", &named)?;
    judge_bound(args, CONNECT_TIMEOUT, "connectTimeout", &named)?;
    judge_bound(args, RETRY_BACKOFF, "retryBackoff", &named)?;
    judge_attempts(args, &named)?;
    judge_verb(args, verb, &named)?;

    // Framed before the clock below starts: the encode and the open are this
    // end's work, and a budget spent on them is not a budget the other end was
    // given.
    let body = body_of(ctx, args, &named)?;

    let call = transport::Call {
        member: &named,
        verb,
        url,
        address,
        deadline: Instant::now()
            + bound_of(
                ctx,
                args,
                DEADLINE,
                "deadline",
                "http.client.deadline",
                DEFAULT_DEADLINE,
            )?,
        connect_timeout: bound_of(
            ctx,
            args,
            CONNECT_TIMEOUT,
            "connectTimeout",
            "http.client.connect_timeout",
            DEFAULT_CONNECT_TIMEOUT,
        )?,
        idle: bound_of(ctx, args, IDLE, "idle", "http.client.idle", DEFAULT_IDLE)?,
        max_duration: bound_of(
            ctx,
            args,
            MAX_DURATION,
            "maxDuration",
            "http.client.max_duration",
            DEFAULT_MAX_DURATION,
        )?,
        headers: headers_of(args, HEADERS, &named)?,
        redirects: redirects_of(ctx, args),
        attempts: args[RETRY_ATTEMPTS]
            .as_uint()
            .unwrap_or(1)
            .try_into()
            .unwrap_or(u32::MAX),
        backoff: bound_of(
            ctx,
            args,
            RETRY_BACKOFF,
            "retryBackoff",
            "http.client.retry_backoff",
            DEFAULT_BACKOFF,
        )?,
        idempotency_key: args[RETRY_KEY].as_text().map(str::to_owned),
        body,
        pool: pool_of(ctx),
        compress: crate::compress::Bound::ceiling(ctx),
        traceparent: traceparent_of(ctx),
    };

    // The one difference between the two members, and it is which bounds the
    // body runs under rather than a second reading of it: a buffered call is
    // one deadline over the whole exchange under `REPLY_CEILING`, and a
    // streamed one stops at the head and leaves the body on the socket under
    // `idle` and `maxDuration`
    // (`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`).
    if streamed {
        let answer = transport::send_streamed(&call, &mut |hop| pin(ctx, hop, &named))?;
        let headers = header_map(&answer.headers);
        return Ok((answer.status, filed(ctx, answer.body), headers));
    }
    let reply = transport::send(&call, &mut |hop| pin(ctx, hop, &named))?;
    let headers = header_map(&reply.headers);
    Ok((
        reply.status,
        Value::bytes(NvsStr::new(&reply.body)),
        headers,
    ))
}

/// A streamed reply's body, filed against this request, as the key its
/// `Core\Http\Stream`'s body slot holds.
///
/// The reader is a Rust value and a slot holds a [`Value`], so what the class
/// carries is a key into the request's own table — `Core\IO\File`'s design, and
/// [`Ctx::hold_open_reader`](nvs_runtime::Ctx::hold_open_reader) is where why a
/// `Core` handle is a key is argued.
fn filed(ctx: &mut Ctx, body: transport::Incoming) -> Value {
    Value::uint(ctx.hold_open_reader(Box::new(body)))
}

/// The field lines of a reply, as a header slot holds them: one entry per
/// lower-cased name, each an array of that name's lines in arrival order.
///
/// `at` names the slot rather than this reading assuming one, because a
/// buffered reply and a streamed one both carry the map and neither's layout is
/// the other's.
///
/// Grouped here rather than read back out of a flat list at every call, because
/// both readers of the slot ask the same question of it and a program asking
/// twice would pay for the walk twice. The names arrive lower-cased from the
/// parse and from the answer table alike, so the case rule RFC 9110 § 5.1
/// states is one `to_ascii_lowercase` at the lookup and none here.
///
/// A linear scan over the groups already seen rather than a map, for
/// `Core\Request`'s reason: one reply carries few distinct names, and a map
/// would cost an allocation per group to save a comparison per line.
fn header_map(lines: &[(String, String)]) -> Value {
    let mut grouped: Vec<(&str, NvsArray)> = Vec::new();
    for (name, value) in lines {
        let held = Value::str(NvsStr::new(value.as_bytes()));
        match grouped.iter_mut().find(|(seen, _)| *seen == name) {
            Some((_, values)) => values.append(held),
            None => {
                let mut values = NvsArray::new();
                values.append(held);
                grouped.push((name, values));
            }
        }
    }
    let mut out = NvsArray::new();
    for (name, values) in grouped {
        out.set(NvsStr::new(name.as_bytes()), Value::array(values));
    }
    Value::array(out)
}

/// Every line the reply carried under `name`, as the two readers of
/// [`HEADERS_SLOT`] both see them, or `None` where it carried no such field.
///
/// The lookup lower-cases what the caller wrote and nothing else: the slot's
/// keys were lower-cased where they were parsed, so the comparison RFC 9110
/// § 5.1 asks for is one allocation at the call rather than a walk that
/// compares case-insensitively at every entry.
fn field_lines(object: *mut nvs_runtime::ObjHeader, at: usize, name: &str) -> Option<Vec<Vec<u8>>> {
    let map = crate::instance::slot(object, at).array_ptr()?;
    let map = crate::arr::borrowed(map);
    let values = map.get(name.to_ascii_lowercase().as_bytes())?.array_ptr()?;
    let values = crate::arr::borrowed(values);
    let mut lines = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = values.next_slot(from) {
        from = slot + 1;
        let held = values
            .value_at(slot)
            .expect("next_slot only names live entries");
        lines.push(held.as_text().unwrap_or_default().as_bytes().to_vec());
    }
    Some(lines)
}

/// The `$name` argument of a header reading.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, unreachable from source: the row's own
/// `CoreTy::Text` is what `E0401` refuses anything else against.
fn field_name<'a>(value: &'a Value, class: &str, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{class}::{member} expected a `string` name, got tag {}",
            value.tag_byte()
        ))
    })
}

/// One field of a reply, joined as RFC 9110 § 5.3 makes repeated lines
/// equivalent — `header()`'s answer, for whichever class asked.
///
/// The join is the reading a program almost always wants: a `content-type` the
/// origin sent twice means what the two lines say together. `Set-Cookie` is the
/// field the equivalence does not cover ([`UNJOINABLE_FIELD`]), so it throws
/// rather than answering a string that parses as neither cookie — and it throws
/// whether or not the reply carried one, because a rule that depended on what
/// arrived is a rule no program could be written against.
///
/// # Errors
///
/// A `LogicError` naming `headers` for `Set-Cookie`.
fn joined_field(
    object: *mut nvs_runtime::ObjHeader,
    at: usize,
    name: &str,
    class: &str,
) -> Result<Value, Fault> {
    if name.eq_ignore_ascii_case(UNJOINABLE_FIELD) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{class}::header(): a `Set-Cookie` line is not part of RFC 9110 § 5.3's comma \
                 equivalence and two of them joined parse as neither cookie — \
                 `headers(\"set-cookie\")` answers them one line each"
            ),
        ));
    }
    Ok(match field_lines(object, at, name) {
        None => Value::null(),
        Some(lines) => Value::str(NvsStr::new(&lines.join(&b", "[..]))),
    })
}

/// Every line one field carried, kept apart where [`joined_field`] joins them —
/// `headers()`'s answer, for whichever class asked.
///
/// A field that arrived once answers a list of one and a field that never
/// arrived an empty list, so nothing here is nullable: the absence a caller
/// asks about is `Core\Arr::count`'s zero, and a `?array` would add a second
/// spelling of the same emptiness (`rule:core-api/shape-rules` R5).
fn listed_field(object: *mut nvs_runtime::ObjHeader, at: usize, name: &str) -> Value {
    let mut out = NvsArray::new();
    for line in field_lines(object, at, name).unwrap_or_default() {
        out.append(Value::str(NvsStr::new(&line)));
    }
    Value::array(out)
}

/// One call answered from a test's table —
/// `rule:testing/an-outbound-call-is-answered-from-a-table`, as the branch
/// taken before anything is resolved.
///
/// **Above [`approved`] rather than inside [`transport::send`]**, which is the
/// whole of what the rule buys: a faked call asks no capability, looks up no
/// name and judges no address, because there is no connection for an address
/// to be pinned to — so a client's own conformance cases are writable with no
/// listener and no outbound grant. The door it steps around is one that only
/// ever *narrows* what a program may reach, and nothing here reaches anything.
///
/// **Everything decided from the arguments is still decided**, and in the same
/// order: the scheme roster, the demand for a host, and § 5's bounds. A test
/// that takes its subject off the network keeps every refusal its subject
/// would have met on it, so a wrong URL fails the same way faked and real.
///
/// **The body is framed here too**, by the one function a real call frames with,
/// because what a test asks its subject is what it *sent* — and a body
/// recomposed from the options at the point of asking would be a second writer
/// agreeing with the first until it did not. The `Content-Type` that framing
/// chose joins the record beside the program's own headers, since the program
/// never wrote it and a case asserting a JSON body would otherwise have no way
/// to see that one was sent. The rest of what a request grows on the wire — the
/// `Host`, the `User-Agent`, the `Content-Length` — is the transport's, and
/// nothing connects here.
///
/// # Errors
///
/// [`judged_host`]'s three, [`judge_bound`]'s, [`judge_attempts`]' and
/// [`judge_verb`]', and a `LogicError` naming a URL the table does not answer.
fn faked(
    ctx: &mut Ctx,
    args: &[Value],
    named: &str,
    verb: &str,
    streamed: bool,
) -> Result<(i64, Value, Value), Fault> {
    let url = given_url(args, named)?;
    judged_host(&url, named)?;
    judge_bound(args, DEADLINE, "deadline", named)?;
    judge_bound(args, CONNECT_TIMEOUT, "connectTimeout", named)?;
    judge_bound(args, RETRY_BACKOFF, "retryBackoff", named)?;
    judge_attempts(args, named)?;
    judge_verb(args, verb, named)?;

    let mut headers: Vec<(String, String)> = headers_of(args, HEADERS, named)?
        .into_iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value))
        .collect();
    // The same framing a real call sends, collected instead of written: a case
    // asserting on a multipart body is asserting on what left the program, and
    // a faked path that recomposed it from the options would be a second writer
    // agreeing with the first until it did not. The `Content-Type` it chose
    // joins the record for the same reason — it is the framing's answer, and
    // the program never wrote it.
    let body = body_of(ctx, args, named)?;
    let octets = match &body {
        Some(body) => {
            if let Some(content_type) = &body.content_type
                && !headers.iter().any(|(name, _)| name == CONTENT_TYPE_HEADER)
            {
                headers.push((CONTENT_TYPE_HEADER.to_owned(), content_type.clone()));
            }
            body.collected(named)?
        }
        None => Vec::new(),
    };
    ctx.faked_http_mut().record(nvs_runtime::HttpSent {
        verb: verb.to_owned(),
        url: url.clone(),
        headers,
        body: octets,
    });

    let Some(answer) = ctx.faked_http().answer_for(&url) else {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{named}: this test answers outbound calls from a table and no answer is \
                 registered for {url} — `Core\\Test::answerHttp` registers one, exactly or as a \
                 prefix ending in `*`"
            ),
        ));
    };
    let status = i64::from(answer.status);
    let octets = answer.body.clone();
    let headers = header_map(&answer.headers);
    // A streamed call is answered through a reader here too, over octets that
    // are all present already ([`transport::Incoming::already`]): the walks
    // have one way to frame a body, and a test that armed the table is walking
    // the same code a call to an origin does.
    let body = if streamed {
        filed(ctx, transport::Incoming::already(octets, named))
    } else {
        Value::bytes(NvsStr::new(&octets))
    };
    Ok((status, body, headers))
}

/// The `traceparent` this call carries: the request's own trace, when
/// `[trace] propagate` is on.
///
/// `rule:observability/an-outbound-call-propagates-traceparent` — propagating
/// it is what makes a trace cross a service boundary at all — and § 6 ships the
/// directive **on**, so a deployment that configured nothing propagates. There
/// is no per-call option beside it: which traces leave this process is a
/// deployment decision, which is why § 6 makes the whole `[trace]` block
/// `System`.
///
/// Off is the word `false` and nothing else. A value that is not a boolean is
/// the shipped default rather than a refusal, for [`bound_of`]'s reason:
/// `nvs.toml` is validated where it is loaded, and failing a request over a key
/// the operator can no longer see is the wrong direction.
///
/// The id itself is the runtime's and not this class's — `Ctx` holds one for
/// every request whatever the sampling decision, per
/// [`nvs_runtime::trace_context`].
fn traceparent_of(ctx: &Ctx) -> Option<String> {
    ctx.config()
        .and_then(|config| config.get("trace.propagate"))
        .is_none_or(|text| text.trim() != "false")
        .then(|| ctx.trace_context().traceparent())
}

/// What this core's connection store is bounded by: `[http.client] pool_idle`
/// and `pool_idle_timeout`, then the pair
/// `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`
/// ships.
///
/// No option slot and no `Result`, unlike [`bound_of`]: both directives are
/// `System`-class (`rule:config/three-changeability-classes`), so the bag has
/// no key for a request to widen a bound on memory it shares with every
/// co-resident request, and a value that does not parse was refused where the
/// configuration was loaded rather than being a failed request here.
fn pool_of(ctx: &Ctx) -> pool::Caps {
    let written = |directive| ctx.config().and_then(|config| config.get(directive));
    pool::Caps {
        idle: written("http.client.pool_idle")
            .and_then(|text| text.trim().parse::<usize>().ok())
            .unwrap_or(DEFAULT_POOL_IDLE),
        timeout: written("http.client.pool_idle_timeout")
            .and_then(|text| duration::parse(&text).ok())
            .map_or(DEFAULT_POOL_IDLE_TIMEOUT, |nanos| {
                Duration::from_nanos(nanos.unsigned_abs())
            }),
    }
}

/// How many redirect hops this call may follow: the option, then
/// `[http.client] max_redirects`, then `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`'s zero.
///
/// No `Result`, unlike [`bound_of`]: a `uint` has no invalid value to judge and
/// zero is the default rather than a mistake, so there is nothing here that can
/// refuse.
fn redirects_of(ctx: &Ctx, args: &[Value]) -> u32 {
    args[FOLLOW_REDIRECTS]
        .as_uint()
        .or_else(|| {
            ctx.config()
                .and_then(|config| config.get("http.client.max_redirects"))
                .and_then(|text| text.trim().parse::<u64>().ok())
        })
        .unwrap_or(0)
        .try_into()
        .unwrap_or(u32::MAX)
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::get(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — `rule:http-server/no-spelling-for-an-unbounded-wait`. [`request`] is the body; the verb is this row's own name.
    fn nvs_core_http_client_get(ctx, args: [REQUEST_ARITY]) {
        request(ctx, args, "get", "GET")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::post(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — `rule:http-server/no-spelling-for-an-unbounded-wait` and `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_post(ctx, args: [REQUEST_ARITY]) {
        request(ctx, args, "post", "POST")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::put(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — `rule:http-server/no-spelling-for-an-unbounded-wait`. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_put(ctx, args: [REQUEST_ARITY]) {
        request(ctx, args, "put", "PUT")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::patch(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — `rule:http-server/no-spelling-for-an-unbounded-wait` and `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_patch(ctx, args: [REQUEST_ARITY]) {
        request(ctx, args, "patch", "PATCH")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::delete(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — `rule:http-server/no-spelling-for-an-unbounded-wait`. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_delete(ctx, args: [REQUEST_ARITY]) {
        request(ctx, args, "delete", "DELETE")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::head(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — `rule:http-server/no-spelling-for-an-unbounded-wait`. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_head(ctx, args: [REQUEST_ARITY]) {
        request(ctx, args, "head", "HEAD")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::request(Core\Http\Method $method, string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — `rule:http-server/no-spelling-for-an-unbounded-wait`, for a verb chosen at run time.
    ///
    /// The verb arrives at slot 0 and every option sits one slot further on, so
    /// the rest of the row is [`request`]'s under a slice of its own arguments
    /// rather than a second copy of the same reading. What the named rows
    /// answer while compiling, [`judge_verb`] answers here.
    fn nvs_core_http_client_request(ctx, args: [REQUEST_ARITY + 1]) {
        let verb = crate::router::method_verb(&args[0], "Core\\Http\\Client::request")?
            .to_ascii_uppercase();
        request(ctx, &args[1..], "request", &verb)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::status(): int` — `rule:http-server/retry-is-opt-in-jittered-and-closed`.
    ///
    /// A call rather than a property access, for the reason this module's own
    /// docs give: `$response->status` is `E0405` and always will be.
    ///
    /// # Errors
    ///
    /// A [`Fault::fatal`] naming the member if the receiver is not a
    /// `Core\Http\Response` or its `status` slot holds no `int`. Both are
    /// unreachable from source — `E0401` refuses a receiver of another type
    /// before any of this runs, and the slot is written by the transport in
    /// this crate and by nothing else.
    fn nvs_core_http_response_status(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &RESPONSE, "status")?;
        crate::instance::slot(object, STATUS_SLOT)
            .as_int()
            .map(Value::int)
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{RESPONSE_NAME}::status found a non-`int` `status` slot"
                ))
            })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::text(): tainted string` — `rule:http-server/retry-is-opt-in-jittered-and-closed`, `rule:security/tainted-qualifier`
    /// .
    ///
    /// **The UTF-8 question is asked here and not by the transport**, because
    /// it is this member's question: a reply that is not text still has octets
    /// [`nvs_core_http_response_bytes`] can hand back, and a demand made while
    /// reading the socket would refuse the whole call over a body the program
    /// never meant to read as a string. Bytes that are not text are refused
    /// rather than repaired (`rule:errors/ambiguous-input-refused`) — a
    /// replacement byte is a body the origin did not send, with nothing to tell
    /// a program so.
    ///
    /// The answer is the slot's own allocation retagged rather than copied: a
    /// `string` and a `bytes` share one heap shape, the grapheme count is
    /// computed on the first ask rather than at construction, and the tag is
    /// the UTF-8 promise this member has just discharged.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` for a body that is not UTF-8. A [`Fault::fatal`] naming
    /// the member if the receiver is not a `Core\Http\Response` or its `body`
    /// slot holds no `bytes` — both unreachable from source, exactly as in
    /// [`nvs_core_http_response_status`].
    fn nvs_core_http_response_text(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &RESPONSE, "text")?;
        let body = crate::instance::slot(object, BODY_SLOT);
        let (octets, payload) = body
            .as_bytes()
            .zip(body.buffer_ptr())
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{RESPONSE_NAME}::text found a non-`bytes` `body` slot"
                ))
            })?;
        if let Err(err) = std::str::from_utf8(octets) {
            return Err(Fault::thrown(format!(
                "{RESPONSE_NAME}::text: the reply's body is not valid UTF-8, so it is not a \
                 `string` — byte {} is where it stops being text, and `bytes()` reads it whole",
                err.valid_up_to()
            )));
        }
        #[expect(
            unsafe_code,
            reason = "the receiver owns a reference for the length of the call, so the \
                      slot it holds is live, which is `Value::retain`'s whole obligation, \
                      and the payload the fresh reference belongs to was just checked to \
                      be well-formed UTF-8, which is `NvsStr`'s own invariant for a `Tag::Str`"
        )]
        unsafe {
            body.retain();
            Ok(Value::str(NvsStr::from_raw(payload)))
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::bytes(): tainted bytes` — the body's octets, and
    /// the reader that asks nothing of them.
    ///
    /// The slot already holds them: [`nvs_core_http_response_text`] is where
    /// the UTF-8 demand lives, so this is the tag the transport wrote handed
    /// back with a reference taken. `crate::instance::slot` borrows, and a
    /// value returned to Novis code owes the retain.
    ///
    /// # Errors
    ///
    /// A [`Fault::fatal`] naming the member if the receiver is not a
    /// `Core\Http\Response` or its `body` slot holds no `bytes` — both
    /// unreachable from source, exactly as in
    /// [`nvs_core_http_response_status`].
    fn nvs_core_http_response_bytes(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &RESPONSE, "bytes")?;
        let body = crate::instance::slot(object, BODY_SLOT);
        if body.as_bytes().is_none() {
            return Err(Fault::fatal(format!(
                "{RESPONSE_NAME}::bytes found a non-`bytes` `body` slot"
            )));
        }
        #[expect(
            unsafe_code,
            reason = "the receiver owns a reference for the length of the call, so the \
                      slot it holds is live, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            body.retain();
        }
        Ok(body)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::jsonAs<T>({maxDepth?: uint}): T` — the reply's body
    /// hydrated into `T`, which is `Core\Json::decodeAs` over the octets
    /// [`nvs_core_http_response_bytes`] answers, carrying that member's
    /// `{maxDepth?}` bag and its default.
    ///
    /// **Arguments 0 to 2 are what the call site wrote as its type argument**,
    /// not values: `crate::registry::WRITTEN_CLASS_MEMBERS` puts this member on
    /// the roster whose helper is handed a `nvs_runtime::ClassDesc`, the
    /// `array<...>` flag and an inline shape's wire contract ahead of
    /// *everything*, receiver included, and that roster's docs own why. So the
    /// receiver is argument 3 and the arity here is three more than the
    /// registry row's.
    ///
    /// The octets are read where they lie. The slot holds the whole body and
    /// this member keeps nothing of what it built, so `bytes`, `text` and a
    /// second `jsonAs<T>` may each follow it and read the same reply — which is
    /// what makes the qualifier the only thing separating the three readings.
    ///
    /// **What it spends:** one parse of the body and the instances it hands
    /// back, both released with the call and neither kept on the reply, so both
    /// are O(in-flight).
    ///
    /// # Errors
    ///
    /// `LogicError` for a `T` carrying no codec and for a `maxDepth` outside the
    /// bag's range, `ParseError` for a body that is not the document `T` decodes
    /// from. A [`Fault::fatal`] naming the member if the receiver is not a
    /// `Core\Http\Response` or its `body` slot holds no `bytes` — both
    /// unreachable from source, exactly as in
    /// [`nvs_core_http_response_status`].
    fn nvs_core_http_response_json_as(ctx, args: [5]) {
        // Unreachable from source, because arguments 0 to 2 are not a program's
        // values: `nvs_ir::lower` writes all three out of the type argument at
        // the call site, and a call naming none is `E0442` — `takes 1 type
        // argument(s)` — before any of this runs.
        let class = args[0].as_class_desc().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Http\\Response::jsonAs` was called with no class in argument 0",
        ))?;
        // Unreachable from source for the same reason and refused by the same
        // `E0442`: slot 1 is the `ConstBool` the lowering emits beside the
        // descriptor, so a call that has one has the other.
        let list = args[1].as_bool().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Http\\Response::jsonAs` was called with no list flag in \
             argument 1",
        ))?;
        // Slot 2 is an inline shape's wire contract, and a written *class* gets
        // the zero word there — `crate::json::decode_as`'s own reading, since
        // this member is that member over the reply's octets.
        let shape = args[2].as_shape_codec();
        let object = crate::instance::receiver(args[3], &RESPONSE, "jsonAs")?;
        // Before the body is touched, on `Core\Request::jsonAs`'s reasoning: a
        // `maxDepth` this member will refuse is a defect in the program, and
        // saying so must not spend a reading of the reply.
        let max = crate::json::max_depth(&args[4], "Core\\Http\\Response::jsonAs")?;
        let body = crate::instance::slot(object, BODY_SLOT);
        let octets = body.as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{RESPONSE_NAME}::jsonAs found a non-`bytes` `body` slot"
            ))
        })?;
        // The UTF-8 question is `text`'s to throw a `RuntimeError` over and this
        // member's to call a malformed document: what arrived is being read as
        // JSON, and JSON is text, so bytes that are not are one more way for the
        // body not to be the document it was asked for.
        let text = std::str::from_utf8(octets).map_err(|_| {
            let message = format!(
                "{RESPONSE_NAME}::jsonAs(): the reply's body is not UTF-8, so it is not a JSON \
                 document"
            );
            let issues = crate::issue::list([("", message.as_str())]);
            Fault::thrown_with_issues(ThrownClass::Parse, message, issues)
        })?;
        #[expect(
            unsafe_code,
            reason = "the descriptor and the contract came out of the constants a \
                      compiled unit owns, so both outlive this call and every \
                      object made from it"
        )]
        unsafe {
            crate::json::decode_as(
                ctx,
                class,
                shape,
                text,
                max,
                list,
                "Core\\Http\\Response::jsonAs",
            )
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::header(string $name): ?tainted string` —
    /// `rule:security/tainted-qualifier`'s reply half, read one field at a time.
    ///
    /// [`joined_field`] is the reading and the home of what it decides, because
    /// a streamed reply answers the same question over a map of its own.
    ///
    /// # Errors
    ///
    /// That function's `LogicError` for `Set-Cookie`. A [`Fault::fatal`] naming
    /// the member for a receiver that is not a `Core\Http\Response` or a
    /// `$name` that is not text, both unreachable from source exactly as in
    /// [`nvs_core_http_response_status`].
    fn nvs_core_http_response_header(_ctx, args: [2]) {
        let object = crate::instance::receiver(args[0], &RESPONSE, "header")?;
        let name = field_name(&args[1], RESPONSE_NAME, "header")?;
        joined_field(object, HEADERS_SLOT, name, RESPONSE_NAME)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::headers(string $name): array<tainted string>` —
    /// [`nvs_core_http_response_header`]'s other half, keeping apart what that
    /// one joins. [`listed_field`] is the reading.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_response_header`]'s two fatals, and no throw: `headers`
    /// is what that member's `Set-Cookie` refusal names.
    fn nvs_core_http_response_headers(_ctx, args: [2]) {
        let object = crate::instance::receiver(args[0], &RESPONSE, "headers")?;
        let name = field_name(&args[1], RESPONSE_NAME, "headers")?;
        Ok(listed_field(object, HEADERS_SLOT, name))
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::net::{IpAddr, Ipv4Addr, TcpListener};

    use nvs_runtime::{Ctx, Fault, NvsStr, ThrownClass, Value};

    use crate::tests::granting;

    use super::{
        BODY_OPTION, BODY_OPTIONS, BODY_SLOT, CONNECT_TIMEOUT, CONTENT_TYPE_OPTION, DEADLINE,
        FOLLOW_REDIRECTS, FORM_OPTION, HEADERS, HEADERS_SLOT, IDLE, JSON_OPTION, MAX_DURATION,
        MULTIPART_OPTION, OPTIONS, REQUEST_ARITY, RESPONSE, RETRY_ATTEMPTS, RETRY_ATTEMPTS_OPTION,
        RETRY_BACKOFF, RETRY_KEY, RETRY_KEY_OPTION, STATUS_SLOT, STREAM_ARITY, STREAM_OPTIONS,
        TARGET, TARGET_ADDRESS_SLOT, TARGET_URL_SLOT,
    };

    /// The grant every case here starts from: the host is reachable and no
    /// address is excepted, which is § 3's table exactly as it ships.
    const GRANTED: &str = "[capabilities.net]\nconnect = [\"127.0.0.1\"]\n";

    /// [`TARGET`]'s layout, asserted for [`RESPONSE`]'s reason and one more:
    /// these two slots are written by the launderer and read back by the
    /// client, so a swapped pair would connect to a URL and pin an address,
    /// which is the one mistake here that still runs.
    #[test]
    fn a_targets_slot_constants_are_the_names_it_declares() {
        assert_eq!(TARGET_URL_SLOT, TARGET.slot("url"));
        assert_eq!(TARGET_ADDRESS_SLOT, TARGET.slot("address"));
    }

    /// The two halves of the layout agree: the index a body reads by and the
    /// name the registry declares are one decision written twice, which is the
    /// pairing [`crate::registry::CoreClass::slots`] exists to keep honest.
    #[test]
    fn a_responses_slot_constants_are_the_names_it_declares() {
        assert_eq!(STATUS_SLOT, RESPONSE.slot("status"));
        assert_eq!(BODY_SLOT, RESPONSE.slot("body"));
        assert_eq!(HEADERS_SLOT, RESPONSE.slot("headers"));
    }

    /// Every option's ABI slot is its own position in the bag — the two tests
    /// above's pairing, over the one layout no member can see it get wrong. An
    /// option inserted anywhere but the end renumbers every slot after it, and a
    /// body then reads a `followRedirects` count out of the `headers` argument,
    /// which is a mismatch the ABI has no tag to catch.
    #[test]
    fn every_option_slot_is_its_position_in_the_bag() {
        for (slot, name) in [
            (DEADLINE, "deadline"),
            (CONNECT_TIMEOUT, "connectTimeout"),
            (HEADERS, "headers"),
            (FOLLOW_REDIRECTS, "followRedirects"),
            (RETRY_ATTEMPTS, RETRY_ATTEMPTS_OPTION),
            (RETRY_BACKOFF, "retryBackoff"),
            (RETRY_KEY, RETRY_KEY_OPTION),
        ] {
            assert_eq!(OPTIONS[slot - 1].name, name, "slot {slot}");
        }
        assert_eq!(REQUEST_ARITY, OPTIONS.len() + 1);

        // `stream`'s bag is the same layout with a group after it, so its two
        // slots are asserted against the bag that declares them — the one row
        // whose arguments the shared reading walks past the end of.
        for (position, shared) in OPTIONS.iter().enumerate() {
            assert_eq!(STREAM_OPTIONS[position].name, shared.name, "at {position}");
        }
        for (slot, name) in [(IDLE, "idle"), (MAX_DURATION, "maxDuration")] {
            assert_eq!(STREAM_OPTIONS[slot - 1].name, name, "slot {slot}");
        }
        assert_eq!(STREAM_ARITY, STREAM_OPTIONS.len() + 1);
    }

    /// Every body key is an option of the bag, and `contentType` with them —
    /// the refusal in `nvs_types::expr::args` is written over
    /// [`BODY_OPTIONS`]'s spellings alone, so a key that is not also declared
    /// here would be refused at a call site that could never have written it.
    #[test]
    fn every_body_key_the_refusal_reads_is_an_option_of_the_bag() {
        assert_eq!(
            BODY_OPTIONS,
            [JSON_OPTION, FORM_OPTION, BODY_OPTION, MULTIPART_OPTION]
        );
        for name in BODY_OPTIONS.iter().chain(&[CONTENT_TYPE_OPTION]) {
            assert!(
                OPTIONS.iter().any(|option| option.name == *name),
                "`{name}` is not an option of the bag"
            );
        }
    }

    /// `rule:security/the-policy-lives-in-the-capability`, asserted as **agreement** rather than as a value: the
    /// address policy is the capability's, and this module holds no copy of it.
    /// The launderer and `nvs_runtime::capability::pin_host` are asked about the
    /// same host on the same two deployments, and what is pinned is that they
    /// answer the *same thing* — a client that had grown a table of its own
    /// would read plausibly on either line alone and disagree only here.
    ///
    /// Both of § 3's keys, because both are the operator's: `net.connect`
    /// decides the host and `net.internal` decides the address, and nothing a
    /// caller writes reaches either one.
    #[test]
    fn the_address_policy_is_read_from_the_capability_and_not_from_the_client() {
        const MEMBER: &str = "Core\\Http::allowUrl";
        const URL: &str = "http://127.0.0.1:8099/ok";
        const HOST: &str = "127.0.0.1";

        // The deployment grants the host and excepts no address, so § 3's table
        // refuses what the name resolves to.
        let mut denied = Ctx::buffered();
        denied.set_config(granting(GRANTED));
        let by_client = super::pin(&mut denied, URL, MEMBER)
            .expect_err("loopback is the first range § 3 denies");
        let by_door = nvs_runtime::capability::pin_host(&denied, HOST, MEMBER)
            .expect_err("and the door is where that refusal is written");
        assert_eq!(
            format!("{by_client:?}"),
            format!("{by_door:?}"),
            "the refusal a caller reads is the door's own sentence, not one `Core\\Http` composed"
        );
        assert!(
            format!("{by_client:?}").contains("net.internal"),
            "and it names the key an operator would have to write: {by_client:?}"
        );

        // One line added to the *configuration*, nothing changed in this crate,
        // and both answers move together.
        let mut excepted = Ctx::buffered();
        excepted.set_config(granting(
            "[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n",
        ));
        let pinned =
            super::pin(&mut excepted, URL, MEMBER).expect("an address the deployment bought back");
        assert_eq!(pinned, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(
            nvs_runtime::capability::pin_host(&excepted, HOST, MEMBER)
                .expect("the door approves it too, or the two had drifted"),
            pinned
        );

        // The grant side is the capability's as well, and asked first: a context
        // that configures nothing refuses on `net.connect` without ever looking
        // at an address, so this member is not a resolver for names it may not
        // reach.
        let mut ungranted = Ctx::buffered();
        let refused = super::pin(&mut ungranted, URL, MEMBER)
            .expect_err("a context with no configuration grants nothing");
        let Fault::Thrown(class, message) = refused else {
            panic!("a capability refusal is catchable — `rule:security/denial-is-a-runtime-error`");
        };
        assert_eq!(class, ThrownClass::Runtime);
        assert!(
            message.contains("net.connect") && !message.contains("net.internal"),
            "the host is refused before the address is judged: {message}"
        );
    }

    /// `rule:security/net-address-policy`'s other half, which a thrown `Fault` on its own does not
    /// pin: the refusal lands **before the socket**. A listener is bound on the
    /// address the URL names, and the assertion is that it was never accepted —
    /// a client that connected first and asked the policy afterwards throws the
    /// very same error, having already announced this process to the address the
    /// deployment denied.
    ///
    /// Driven through [`super::request`], the body every client row shares, so
    /// what is pinned is the order the *member* runs in and not the order
    /// [`super::pin`] alone does.
    #[test]
    fn a_denied_address_range_fails_before_a_connection_is_made() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        listener
            .set_nonblocking(true)
            .expect("a listener that answers now rather than waiting");
        let at = listener.local_addr().expect("its own address");

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(GRANTED));

        let url = Value::str(NvsStr::new(format!("http://{at}/ok").as_bytes()));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = url;
        let refused = super::request(&mut ctx, &args, "get", "GET")
            .expect_err("loopback, which `net.internal` does not except");
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the reference `NvsStr::new` just \
                      produced, and a native member never releases an argument \
                      its caller still owns"
        )]
        unsafe {
            url.release();
        }
        assert!(
            format!("{refused:?}").contains("net.internal"),
            "{refused:?}"
        );

        match listener.accept() {
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Ok(_) => {
                panic!("the door refused the address and a socket was opened to it anyway")
            }
            Err(err) => panic!("the listener failed for a reason that is not the point: {err}"),
        }
    }

    /// `rule:observability/an-outbound-call-propagates-traceparent` and `rule:observability/metrics-and-trace-blocks-are-system`: `[trace] propagate` decides whether this request's
    /// trace leaves the process, it ships **on**, and what leaves is the
    /// runtime's own id — one per request, not one per call.
    ///
    /// The last assertion is the one that fails if this class ever mints an id
    /// of its own: a per-call id would still print a well-formed header on every
    /// line above it.
    #[test]
    fn traceparent_is_the_requests_own_id_and_only_while_propagate_is_on() {
        let mut on = Ctx::buffered();
        on.set_config(granting(GRANTED));
        let sent = super::traceparent_of(&on).expect("§ 6 ships `propagate` on");
        assert_eq!(sent, on.trace_context().traceparent());
        assert_eq!(
            super::traceparent_of(&on).as_deref(),
            Some(sent.as_str()),
            "a second call in one request is the same trace"
        );

        let mut off = Ctx::buffered();
        off.set_config(granting("[trace]\npropagate = false\n"));
        assert_eq!(
            super::traceparent_of(&off),
            None,
            "`propagate = false` is the whole of how a trace stops here"
        );

        // A context nobody configured is § 6's shipped defaults and not a
        // refusal — the same reading `bound_of` gives a directive nothing set.
        let unconfigured = Ctx::buffered();
        let other = super::traceparent_of(&unconfigured).expect("nothing configured propagates");
        assert_ne!(other, sent, "two requests are two traces");
    }
}
