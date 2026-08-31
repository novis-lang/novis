//! `Core\Http` — [ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md)'s outbound door,
//! which is one member: the launderer every outbound URL has to pass through, and the pinned
//! `Core\Http\Target` it answers with.
//!
//! ADR 0058 places the class and § 2 gives the signature; what belongs here is why the answer is a
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
//! ([ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 6). ADR 0058 § 2 calls
//! this the first ADR 0024 launderer whose output is a value rather than a plain string; it is the
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
//! it is `nvs_runtime::capability::pin_host` — ADR 0058 § 5 puts it in the capability rather than
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
//! [ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 5 gives every request
//! member the same two parameters: the URL, and one trailing [`OPTIONS`] bag. The URL is ADR 0058
//! § 1's sink — `string | Core\Http\Target`, **both unqualified**, so a `tainted` operand is a
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
//! site — the verb is the member's own name, the bag is an ADR 0063 R2 literal — so
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
//! [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md) § 1 means — its
//! roster names these readers for that reason. `status()` is an `int` and carries no qualifier,
//! because there is nothing in three digits for a sink to misread.
//!
//! The `body` slot holds a `string` rather than the bytes that arrived. Decoding is one question,
//! answered by the transport where the charset is known; a reader that decoded on every call would
//! answer it again, and a second answer is the one that will disagree.
//!
//! # What is not here yet, and why each is deliberate rather than forgotten
//!
//! **The transport.** Every row resolves, pins and judges its options exactly as it will, and then
//! throws: stage 5's socket over the runtime's own reactor is the missing half, and a member that
//! answered a fabricated `Core\Http\Response` would be worse than one that says so. That refusal is
//! the one message in this module that a later slice deletes.
//!
//! **A request body.** `post` and `put` take a URL and options and nothing else, because a body's
//! `tainted` behaviour is a decision the spec's own `send(Core\Http\Request)` row owns and the
//! transport is what makes it testable — posting user-supplied data is ordinary, so the answer is
//! not the URL's answer, and guessing it here would pin the wrong one in a signature.
//!
//! **The reply's headers.** [`RESPONSE`] answers `status()` and `text()` and nothing else: a slot
//! and the member that reads it are one decision, and a `header()` over a map nothing fills would
//! be a surface with no behaviour under it. It arrives with the transport that writes the map.
//!
//! **What it spends:** one `Core\Http\Target` allocation per laundered URL, two slots wide, charged
//! to the request that laundered it — and one synchronous resolution per call, which
//! `pin_host`'s own docs own. A request member allocates nothing of its own before the transport:
//! a `Target` argument is borrowed, and a `string` one is pinned without building a target, since
//! nothing downstream of the check would read it.

use fluent_uri::UriRef;
use fluent_uri::component::{Authority, Scheme};
use nvs_runtime::{Ctx, Fault, NvsStr, Tag, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Http";

/// What a refusal from this module and from the door below it is written under.
const MEMBER: &str = r"Core\Http::allowUrl";

/// ADR 0058 § 2's launderer, as the one row `Core\Http` has today.
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

/// `Core\Http::allowUrl`'s reference card — ADR 0117.
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
               resolves to a loopback, private, link-local or unspecified address, which no grant \
               reaches.",
    }],
};

/// [`TARGET`]'s name, written once — see [`NAME`].
pub(crate) const TARGET_NAME: &str = r"Core\Http\Target";

/// ADR 0058 § 2's pinned target: a URL and the one address it was approved at.
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
        "nvs_core_http_client_delete" => (nvs_core_http_client_delete as *const ()).cast(),
        "nvs_core_http_client_head" => (nvs_core_http_client_head as *const ()).cast(),
        "nvs_core_http_response_status" => (nvs_core_http_response_status as *const ()).cast(),
        "nvs_core_http_response_text" => (nvs_core_http_response_text as *const ()).cast(),
        _ => return None,
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
/// approved it — the whole of ADR 0058 §§ 2-3 as this module holds it, written
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

    nvs_runtime::capability::pin_host(ctx, Authority::host(&authority), member)
}

nvs_runtime::nvs_helper! {
    /// `Core\Http::allowUrl(tainted string $url): Core\Http\Target` — ADR 0058
    /// § 2, and the only spelling that removes `tainted` from an outbound URL.
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

/// The two [`OPTIONS`] names ADR 0074 § 7's refusal is written over, spelled
/// once so the rule and the rows cannot drift apart —
/// [`crate::registry::idempotent_retry_rule`] hands these to the checker rather
/// than the checker holding its own copy of them.
pub(crate) const RETRY_ATTEMPTS_OPTION: &str = "retryAttempts";
/// See [`RETRY_ATTEMPTS_OPTION`].
pub(crate) const RETRY_KEY_OPTION: &str = "retryIdempotencyKey";

/// ADR 0058 § 1's outbound sink, as the one parameter every request member
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

/// ADR 0074 § 5's `Core\Http\Options`, as the one trailing bag every request
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
const OPTIONS: &[CoreOption] = &[
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
    // Unqualified for the URL's reason and by the same mechanism: a header value
    // is copied verbatim into the request this member makes, and until ADR 0088's
    // classification is read at the call it is assignability that refuses
    // `array<tainted string>` here.
    CoreOption {
        name: "headers",
        ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
        default: Const::EmptyArray,
    },
    // A count rather than a `bool`: ADR 0058 § 4 turns redirects off by default,
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
];

/// The ABI slot each of [`OPTIONS`]'s bounds flattens into — the bag expands to
/// one argument per option, in declaration order, after the URL at slot 0.
const DEADLINE: usize = 1;
const CONNECT_TIMEOUT: usize = 2;
const RETRY_ATTEMPTS: usize = 5;
const RETRY_BACKOFF: usize = 6;

/// ADR 0074 § 5's request members, over ADR 0058 § 1's sink.
///
/// Five rows and one shape: the verb is the member's own name, which is what
/// makes § 7's idempotency question answerable while compiling. `patch`,
/// `options` and `trace` are the verbs `Core\Http\Method` has that this class
/// does not — the spec's own `send(Core\Http\Request)` row is where a method
/// chosen at run time belongs, and it lands with the transport.
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
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// What every request member answers with — ADR 0074 §§ 5-6's reply, as the
/// two slots a transport fills and the two members that read them back.
///
/// `status` is a member rather than a property and `text` answers a `tainted`
/// string; this module's own docs are the home of both decisions. The roster
/// stops at two because a slot and the member that reads it are one decision
/// — `a_class_with_slots_has_instance_members_and_the_reverse` is that rule —
/// so the header map arrives with the transport that fills it.
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
    ],
    slots: &["status", "body"],
    constants: &[],
};

/// [`RESPONSE`]'s status slot, by index — the layout its `slots` names, which
/// `CoreClass::slot` is the check on.
const STATUS_SLOT: usize = 0;
/// [`RESPONSE`]'s body slot. See [`STATUS_SLOT`].
const BODY_SLOT: usize = 1;

/// `Core\Http\Response::status`'s reference card — ADR 0117.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "The reply's HTTP status code, as the origin sent it and with nothing read into it — \
            replacing `curl_getinfo`'s `CURLINFO_RESPONSE_CODE` key.",
    params: &[],
    ret: "The status line's three-digit code. A `404` and a `500` are answers, so they arrive \
          here rather than as a throw; only a request that got no reply at all throws.",
    errors: &[],
};

/// `Core\Http\Response::text`'s reference card — ADR 0117.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "The reply's body as text, replacing `curl_exec`'s return value and the \
            `CURLOPT_RETURNTRANSFER` flag that decided whether there was one.",
    params: &[],
    ret: "The body, `tainted`: it is bytes another host chose, and a pinned address settles where \
          they came from rather than what is in them. A sink's own launderer is the way out of \
          it, and there is no generic one.",
    errors: &[],
};

/// The parameters every row of [`CLIENT`] documents — one bag, so the seven
/// options are described once rather than five times, and a reader comparing
/// two members finds no difference because there is none.
const REQUEST_PARAMS: &[ParamDoc] = &[
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
        desc: "Extra request headers, by name. The runtime's own headers are added around these.",
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
        desc: "Sent as `Idempotency-Key`, identical across attempts. Required for `post` when \
               `retryAttempts` is given, and accepted by every other member.",
        shape: &[],
    },
];

/// What every request member answers, once — see [`REQUEST_PARAMS`].
const REQUEST_RET: &str = "A `Core\\Http\\Response` carrying the status, the headers and the body \
                           of the reply. **The transport behind this member is not built yet**, so \
                           today it throws instead of answering.";

/// What every request member throws, once — see [`REQUEST_PARAMS`].
const REQUEST_ERRORS: &[ErrorDoc] = &[ErrorDoc {
    error: "RuntimeError",
    desc: "The URL is refused: it is not a URL, its scheme is neither `http` nor `https`, it names \
           no host, `net.connect` does not grant that host, or it resolves to a loopback, private, \
           link-local or unspecified address. An option is outside its bounds: a `deadline`, \
           `connectTimeout` or `retryBackoff` that is not a positive duration, or a \
           `retryAttempts` of zero. And, while the transport is unbuilt, the send itself.",
}];

/// `Core\Http\Client::get`'s reference card — ADR 0117.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Fetches `$url` under a finite budget, over the address the outbound policy pinned.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::post`'s reference card — ADR 0117.
const POST_DOC: MethodDoc = MethodDoc {
    short: "Sends a `POST` to `$url` under a finite budget. The one member whose retries need \
            `retryIdempotencyKey`, because a repeated `POST` is a second effect rather than a \
            second question.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::put`'s reference card — ADR 0117.
const PUT_DOC: MethodDoc = MethodDoc {
    short: "Sends a `PUT` to `$url` under a finite budget. Idempotent by definition, so its \
            retries need no key.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::delete`'s reference card — ADR 0117.
const DELETE_DOC: MethodDoc = MethodDoc {
    short: "Sends a `DELETE` to `$url` under a finite budget.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// `Core\Http\Client::head`'s reference card — ADR 0117.
const HEAD_DOC: MethodDoc = MethodDoc {
    short: "Asks `$url` for its headers alone, under the same budget a `get` would have.",
    params: REQUEST_PARAMS,
    ret: REQUEST_RET,
    errors: REQUEST_ERRORS,
};

/// One of § 5's time bounds, judged: present and positive, or left out.
///
/// # Errors
///
/// A thrown `RuntimeError` naming the option for a bound that is not positive —
/// ADR 0074 § 5 has no spelling for an unbounded wait, and a zero one is that
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

/// Every request member's body: the URL through the outbound policy, the
/// options through § 5's bounds, and then the transport that does not exist.
///
/// The order is the contract. Everything a caller can get wrong is decided
/// before anything leaves the process, so a program's own tests find a refused
/// URL or an impossible deadline without a network — which is also why the
/// missing transport is the *last* thing this reaches rather than the first.
///
/// # Errors
///
/// [`pin`]'s four, [`judge_bound`]'s and [`judge_attempts`]', and then the
/// unbuilt transport's own refusal.
fn request(ctx: &mut Ctx, args: &[Value], member: &str) -> Result<Value, Fault> {
    let named = format!("{CLIENT_NAME}::{member}");

    // A `Target` argument was pinned by the launderer that built it, and asking
    // again would be the second resolution ADR 0058 § 2 exists to remove. A
    // plain `string` is the form § 1 keeps for a URL the program authored, and
    // it goes through the same door.
    if !matches!(args[0].tag(), Some(Tag::Object)) {
        let text = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{named} expected a `string` or a `Core\\Http\\Target`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        pin(ctx, text, &named)?;
    }

    judge_bound(args, DEADLINE, "deadline", &named)?;
    judge_bound(args, CONNECT_TIMEOUT, "connectTimeout", &named)?;
    judge_bound(args, RETRY_BACKOFF, "retryBackoff", &named)?;
    judge_attempts(args, &named)?;

    Err(Fault::thrown(format!(
        "{named}: the request is approved and there is no transport behind it yet, so nothing \
         was sent"
    )))
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::get(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — ADR 0074 § 5. [`request`] is the body; the verb is this row's own name.
    fn nvs_core_http_client_get(ctx, args: [8]) {
        request(ctx, args, "get")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::post(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — ADR 0074 §§ 5, 7. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_post(ctx, args: [8]) {
        request(ctx, args, "post")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::put(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — ADR 0074 § 5. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_put(ctx, args: [8]) {
        request(ctx, args, "put")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::delete(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — ADR 0074 § 5. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_delete(ctx, args: [8]) {
        request(ctx, args, "delete")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Client::head(string|Core\Http\Target $url, Core\Http\Options): Core\Http\Response`
    /// — ADR 0074 § 5. See [`nvs_core_http_client_get`].
    fn nvs_core_http_client_head(ctx, args: [8]) {
        request(ctx, args, "head")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::status(): int` — ADR 0074 § 6.
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
    /// `Core\Http\Response::text(): tainted string` — ADR 0074 § 6, ADR 0024
    /// § 1.
    ///
    /// The slot is handed straight back with a reference taken, since the
    /// transport decoded once already; `crate::instance::slot` borrows, and a
    /// value returned to Novis code owes the retain.
    ///
    /// # Errors
    ///
    /// A [`Fault::fatal`] naming the member if the receiver is not a
    /// `Core\Http\Response` or its `body` slot holds no `string` — both
    /// unreachable from source, exactly as in
    /// [`nvs_core_http_response_status`].
    fn nvs_core_http_response_text(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &RESPONSE, "text")?;
        let body = crate::instance::slot(object, BODY_SLOT);
        if body.as_text().is_none() {
            return Err(Fault::fatal(format!(
                "{RESPONSE_NAME}::text found a non-`string` `body` slot"
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

#[cfg(test)]
mod tests {
    use super::{BODY_SLOT, RESPONSE, STATUS_SLOT};

    /// The two halves of the layout agree: the index a body reads by and the
    /// name the registry declares are one decision written twice, which is the
    /// pairing [`crate::registry::CoreClass::slots`] exists to keep honest.
    #[test]
    fn a_responses_slot_constants_are_the_names_it_declares() {
        assert_eq!(STATUS_SLOT, RESPONSE.slot("status"));
        assert_eq!(BODY_SLOT, RESPONSE.slot("body"));
    }
}
