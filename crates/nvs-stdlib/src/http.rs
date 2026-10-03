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
//! URL the program itself authored — goes through the same questions [`pin`] asks for
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
//! [`pin`], so `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`'s rule is enforced by the same questions the first URL passed and
//! there is no second copy of the policy under the socket. What that module's own doc owns is the
//! rest — one connection per attempt, how `https` reaches `nvs-host`'s TLS client and which host
//! name its certificate is checked against, and what a reply is allowed to make this process hold.
//!
//! # What it spends
//!
//! One `Core\Http\Target` allocation per laundered URL — the URL and the
//! approved set, at most eight addresses as text — charged to the request that laundered it, and
//! one resolution per call, which `pin_host_addresses`' own docs own. A request member allocates
//! nothing of its own before the transport:
//! a `Target` argument is borrowed, and a `string` one is pinned without building a target, since
//! nothing downstream of the check would read it. A call that writes a body spends that body once,
//! charged to the request and held until the last attempt is done with it — except a
//! `Core\Http\Part::file`, which is a descriptor and a chunk rather than the file
//! ([`transport::Piece`]). What the exchange itself spends is [`transport`]'s to state.
//!
//! # Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-stdlib/src/http.rs` lists them.

mod pool;
pub(crate) mod socket;
mod span;
pub(crate) mod stream;
mod transport;

use std::net::IpAddr;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use fluent_uri::UriRef;
use fluent_uri::component::{Authority, Scheme};
use nvs_config::capability::{Cap, Scope};
use nvs_host::tls::CallPolicy;
use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};
use nvs_syntax::duration;
use rand::RngExt;

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\Http";

/// What a refusal from this module and from the door below it is written under.
const MEMBER: &str = r"Core\Http::allowUrl";

/// `Core\Http`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Makes HTTP calls to other servers. `Core\\Http::allowUrl` checks a URL that came from \
            user input, and the classes under `Core\\Http` send the requests. \
            `Core\\Http::methodName` gives the name of an HTTP method as text.",
};

/// `Core\Http\Target`'s class card — `rule:core-api/reference-card`.
const TARGET_CARD: ClassDoc = ClassDoc {
    short: "A URL that `Core\\Http::allowUrl` checked, with the addresses it was approved for. \
            A call connects only to those addresses, so a second name lookup cannot send it \
            somewhere else.",
};

/// `Core\Http\Identity`'s class card — `rule:core-api/reference-card`.
const IDENTITY_CARD: ClassDoc = ClassDoc {
    short: "A client certificate and its key. A request sends it when the server asks the client \
            to prove who it is.",
};

/// `Core\Http\Part`'s class card — `rule:core-api/reference-card`.
const PART_CARD: ClassDoc = ClassDoc {
    short: "Describes a request body, or one field of a multipart body. A file part is read from \
            disk while it is sent, so the whole file is never in memory.",
};

/// `Core\Http\Client`'s class card — `rule:core-api/reference-card`.
const CLIENT_CARD: ClassDoc = ClassDoc {
    short: "Sends HTTP requests to other servers, with one method for each HTTP verb. Each call \
            returns the reply as a `Core\\Http\\Response`.",
};

/// `Core\Http`: `rule:http-server/allow-url-pins-the-address`'s launderer, and
/// the wire text of a `Core\Http\Method` case.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
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
        },
        // A static member here and not one on the enum, because an enum keeps
        // no members of its own (`rule:enums/no-class-machinery`).
        CoreMethod {
            name: "methodName",
            names: &["method"],
            params: &[CoreTy::Enum(crate::router::METHOD_NAME)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_http_method_name",
            doc: Some(&METHOD_NAME_DOC),
        },
    ],
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
    ret: "A `Core\\Http\\Target` bound to every address the host resolved to that the policy \
          approved — at most eight, in the resolver's order — which is the set the client connects \
          across, so a second name lookup cannot answer differently.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The text is not a URL, its scheme is neither `http` nor `https`, it names no host, \
               `net.connect` does not grant that host, the host resolves to no address, or it \
               resolves to a loopback, private, link-local or unspecified address that \
               `net.internal` does not name.",
    }],
};

/// `Core\Http::methodName`'s reference card — `rule:core-api/reference-card`.
const METHOD_NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of an HTTP method as it is written in a request and in an `Allow` \
            header. `Core\\Http\\Method::Get` gives `GET`.",
    params: &[ParamDoc {
        name: "method",
        desc: "The method, as a case of `Core\\Http\\Method`.",
        shape: &[],
    }],
    ret: "The name in capital letters: `GET`, `HEAD`, `OPTIONS`, `TRACE`, `POST`, `PUT`, \
          `PATCH` or `DELETE`.",
    errors: &[],
};

/// [`TARGET`]'s name, written once — see [`NAME`].
pub(crate) const TARGET_NAME: &str = r"Core\Http\Target";

/// `rule:http-server/allow-url-pins-the-address`'s pinned target: a URL and every address it was approved at.
///
/// The set and not one of it, per
/// `rule:http-server/an-outbound-call-tries-every-approved-address`: a call
/// falls back across it, and a retry reuses it rather than resolving a second
/// time.
///
/// No members, for the reason this module's own docs give — a program names it
/// and hands it on, and reading the set back out is the operation that would
/// make pinning decorative.
pub(crate) const TARGET: CoreClass = CoreClass {
    name: TARGET_NAME,
    doc: Some(&TARGET_CARD),
    methods: &[],
    instance: &[],
    slots: &["url", "addresses"],
    constants: &[],
};

/// [`IDENTITY`]'s name, written once — see [`NAME`].
pub(crate) const IDENTITY_NAME: &str = r"Core\Http\Identity";

/// [`IDENTITY`]'s first slot: the certificate chain as the PEM it was read
/// from, leaf first.
pub(crate) const IDENTITY_CHAIN_SLOT: usize = 0;

/// [`IDENTITY`]'s second slot: the key's PKCS#8, copied out of the
/// `Core\Crypto\KeyPair` the read was given.
pub(crate) const IDENTITY_PKCS8_SLOT: usize = 1;

/// [`IDENTITY`]'s third slot: the leaf certificate's SHA-256, lower-case hex —
/// the identity's public name, and what a pool key carries.
pub(crate) const IDENTITY_FINGERPRINT_SLOT: usize = 2;

/// A client identity: the certificate chain a call presents when a server asks
/// the client for one, over the key pair that proves the leaf is this client's.
///
/// **No members**, for [`TARGET`]'s reason and a second one of its own. A
/// program reads an identity and names it in a request's bag; reading the key
/// back out is not an operation an identity is for, and there is no answer a
/// member could give about the chain that the server's own verdict does not
/// give better.
///
/// **The chain and the key are what the slots hold, not the session state.**
/// [`nvs_host::tls::NvsIdentity`] is the built `ClientConfig`, and it is built
/// where a connection is opened rather than kept in a table here: a
/// process-lifetime map keyed by private key material is priority 1 spent to
/// buy priority 3, which is the trade
/// [`crate::crypto::KEY_PAIR`]'s own doc already turned down for the same
/// material. The pool is what makes that cheap — a reused connection has its
/// identity in its key and handshakes not at all.
///
/// The read still builds one and throws it away, exactly as
/// `Core\Crypto\KeyPair::read` parses a key to refuse it rather than to keep
/// it: a mismatched chain and key are refused where the program can still act
/// on it, not during a handshake against a live server.
///
/// **What it spends:** the chain's PEM, the key's PKCS#8 and 64 octets of hex
/// per identity, held as long as the value is and released with it.
pub(crate) const IDENTITY: CoreClass = CoreClass {
    name: IDENTITY_NAME,
    doc: Some(&IDENTITY_CARD),
    methods: &[CoreMethod {
        name: "read",
        // The chain is `bytes` and unqualified: a certificate is public, and
        // it is deployment material the program was handed rather than
        // anything off a wire. The key arrives as the object that already
        // refused everything a private key can be refused for, so this row
        // asks no question `Core\Crypto\KeyPair::read` has answered.
        names: &["chainPem", "key"],
        params: &[
            CoreTy::Blob(Qual::Neutral),
            CoreTy::Instance(crate::crypto::KEY_PAIR_NAME),
        ],
        defaults: &[],
        return_ty: CoreTy::Instance(IDENTITY_NAME),
        symbol: "nvs_core_http_identity_read",
        doc: Some(&IDENTITY_READ_DOC),
    }],
    instance: &[],
    slots: &["chain", "pkcs8", "fingerprint"],
    constants: &[],
};

/// `Core\Http\Identity::read`'s reference card — `rule:core-api/reference-card`.
const IDENTITY_READ_DOC: MethodDoc = MethodDoc {
    short: "Reads the certificate chain a request presents when a server asks the client for \
            one, over the key pair that goes with it. The leaf is checked against the key here, \
            so a chain and a key that are not each other's are refused before any connection is \
            opened.",
    params: &[
        ParamDoc {
            name: "chainPem",
            desc: "The chain as PEM, the leaf first and then whatever intermediates the server \
                   needs to build a path — the order every other client takes one in.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The leaf's private key. Its kind is what the handshake is signed with, so a \
                   key that only agrees — `X25519` — is refused here.",
            shape: &[],
        },
    ],
    ret: "The identity, to name as a request's `identity` option. It is presented only when the \
          server asks for it, and two identities never share a pooled connection.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$chainPem` is not a PEM certificate chain or holds no certificate, `$key` is not \
               a key a TLS handshake can be signed with, or the chain's leaf carries a different \
               public key than `$key`. A client identity is the program's own deployment \
               material, so every refusal is a mistake in what it was deployed with rather than \
               a verdict on anyone.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_http_allow_url" => (nvs_core_http_allow_url as *const ()).cast(),
        "nvs_core_http_method_name" => (nvs_core_http_method_name as *const ()).cast(),
        "nvs_core_http_identity_read" => (nvs_core_http_identity_read as *const ()).cast(),
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
        "nvs_core_http_response_tls" => (nvs_core_http_response_tls as *const ()).cast(),
        "nvs_core_http_tls_info_version" => (nvs_core_http_tls_info_version as *const ()).cast(),
        "nvs_core_http_tls_info_cipher" => (nvs_core_http_tls_info_cipher as *const ()).cast(),
        "nvs_core_http_tls_info_verified" => (nvs_core_http_tls_info_verified as *const ()).cast(),
        "nvs_core_http_tls_info_peer_chain" => {
            (nvs_core_http_tls_info_peer_chain as *const ()).cast()
        }
        "nvs_core_http_tls_info_subject" => (nvs_core_http_tls_info_subject as *const ()).cast(),
        "nvs_core_http_tls_info_issuer" => (nvs_core_http_tls_info_issuer as *const ()).cast(),
        "nvs_core_http_tls_info_expiry" => (nvs_core_http_tls_info_expiry as *const ()).cast(),
        // The streamed reply's own and the outbound socket's, beside their
        // classes rather than here: they are this module's symbols, and
        // [`stream`] and [`socket`] are modules of this one for [`transport`]'s
        // reason.
        other => return stream::address(other).or_else(|| socket::address(other)),
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

/// Every address `text` resolves to, once the URL and the deployment have both
/// approved it — the whole of `rule:http-server/allow-url-pins-the-address` and `rule:security/net-address-policy` as this module holds it, written
/// under `member` so a refusal names the row the caller wrote.
///
/// The order of the questions is deliberate. The text is parsed and its
/// scheme judged first, because both are statements about the argument and
/// neither tells a caller anything about the deployment. Then
/// `nvs_runtime::capability::pin_host_addresses` asks the capability about the
/// host and § 3's table about every address it resolves to — in that order, so
/// an ungranted program cannot use this member as a resolver for names it was
/// never allowed to reach.
///
/// The address question is the one a deployment writing `[http.client.proxy]
/// resolve = "proxy"` has said this process cannot ask: there the proxy
/// resolves the destination and no address for it is ever learned here, so the
/// text, the scheme and the `net.connect` grant's host list are asked exactly
/// as they are for a direct call, and `rule:security/net-address-policy`'s
/// table is the proxy's to enforce
/// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
/// The empty set answered there is what carries the narrowing downstream:
/// `CONNECT` names the host rather than an address, and the connection is
/// pooled under the same text. A host the operator listed in `bypass` is not
/// this case — it is dialled directly and under the full policy — so the fork
/// is asked of the host and not of the block alone.
///
/// One function rather than one per caller: [`nvs_core_http_allow_url`] and
/// every row of [`CLIENT`] that is handed a plain `string` ask exactly this,
/// and a second copy of it would be the second writer that agrees until it
/// does not. The launderer is inside that *and* rather than beside it: a
/// `Core\Http::allowUrl` that still resolved under the word would refuse every
/// URL in the one network the word exists for, since the resolver it reaches
/// there has no route outward.
///
/// # Errors
///
/// A thrown `RuntimeError` for any of them: the text is not a URL, its
/// scheme is outside the roster, it names no host, or the capability refuses
/// the host or one of the addresses it resolves to — one denied address refuses
/// the host whole.
fn pin(
    ctx: &mut Ctx,
    text: &str,
    member: &str,
    roster: Roster,
) -> Result<Vec<std::net::IpAddr>, Fault> {
    let host = judged_host(text, member, roster)?;
    if proxy_of(ctx).is_some_and(|proxy| proxy.resolves(&host)) {
        nvs_runtime::capability::require(ctx, Cap::NetConnect, Scope::Host(&host), member)?;
        return Ok(Vec::new());
    }
    nvs_runtime::capability::pin_host_addresses(ctx, &host, member)
}

/// Which of `rule:http-server/allow-url-pins-the-address`'s four schemes the
/// caller speaks.
///
/// The launderer admits all four and pins them identically, because what it
/// approves is a host and its addresses rather than an intention; which of them
/// a given row serves is the row's own refusal, so a URL laundered for one use
/// cannot be spent on the other. That is why this is a parameter of
/// [`judged_host`] rather than a constant in it.
#[derive(Clone, Copy)]
pub(crate) enum Roster {
    /// What a request member speaks: `http` and `https`.
    Request,
    /// What the row that opens a socket speaks: `ws` and `wss`
    /// (`rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`).
    Socket,
    /// What `Core\Http::allowUrl` admits: all four.
    Laundered,
}

impl Roster {
    /// Whether `scheme` — already lower-cased by the caller — is one this
    /// roster holds.
    fn admits(self, scheme: &str) -> bool {
        match self {
            Self::Request => matches!(scheme, "http" | "https"),
            Self::Socket => matches!(scheme, "ws" | "wss"),
            Self::Laundered => matches!(scheme, "http" | "https" | "ws" | "wss"),
        }
    }
}

/// [`pin`]'s first two questions — the ones a URL answers by itself — and the
/// host they leave: the text parses, and its scheme is one this caller speaks.
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
/// roster, a URL that names no host, or a port past `65535`.
fn judged_host(text: &str, member: &str, roster: Roster) -> Result<String, Fault> {
    let reference = UriRef::parse(text).map_err(|_| {
        Fault::thrown(format!(
            "{member}: this text is not a URL, so there is no host in it to approve"
        ))
    })?;

    // Case-insensitively, because a scheme is: `HTTP://` is the same URL, and a
    // roster compared byte for byte would be one a caller can step around by
    // shouting.
    let scheme = reference.scheme().map(Scheme::as_str).unwrap_or_default();
    let folded = scheme.to_ascii_lowercase();
    if !roster.admits(&folded) {
        // One whole sentence per arm rather than a roster spelled into a shared
        // one: a refusal's text is what a case freezes, and a message assembled
        // from a fragment is one no case can be grepped back to.
        return Err(Fault::thrown(match roster {
            Roster::Request => format!(
                "{member}: the scheme must be `http` or `https`, and this URL names `{scheme}`"
            ),
            Roster::Socket => {
                format!("{member}: the scheme must be `ws` or `wss`, and this URL names `{scheme}`")
            }
            Roster::Laundered => format!(
                "{member}: the scheme must be `http`, `https`, `ws` or `wss`, and this URL names \
                 `{scheme}`"
            ),
        }));
    }

    // An authority with nothing in its host part — `http:///path`, `http://:80/` —
    // names no host just as a URL with no authority does, and gets the same
    // sentence: handed on, the empty text would reach the capability as a host
    // called ``, and the refusal would name a grant nobody could write.
    let no_host = || {
        Fault::thrown(format!(
            "{member}: the URL names no host, so there is nothing to resolve and pin"
        ))
    };
    let authority = reference.authority().ok_or_else(no_host)?;
    let host = Authority::host(&authority);
    if host.is_empty() {
        return Err(no_host());
    }
    // A port past `65535` is refused here as well as by the connection's own
    // reading, so a call a test answers from its table fails as a real one does,
    // and `allowUrl` never approves a URL nothing could connect to.
    if authority.port_to_u16().is_err() {
        return Err(Fault::thrown(format!(
            "{member}: the URL's port is past 65535, so it names no TCP port to connect to"
        )));
    }
    Ok(host.to_owned())
}

nvs_runtime::nvs_helper! {
    /// `Core\Http::allowUrl(tainted string $url): Core\Http\Target` — `rule:http-server/allow-url-pins-the-address`
    /// , and the only spelling that removes `tainted` from an outbound URL.
    ///
    /// The questions it asks, and the order it asks them in, are [`pin`]'s.
    /// What is here is the answer: a value carrying both the URL and every
    /// address that was approved, for the reason this module's own docs give.
    ///
    /// The set is written as one text per address, which is the form the slot
    /// holds and the form [`addresses_of`] reads back — an address is a value
    /// with no representation of its own in the language, and a target carrying
    /// eight of them carries eight strings.
    ///
    /// Under `[http.client.proxy] resolve = "proxy"` it is empty, because the
    /// door approved a host it never resolved. The `Target` then carries the
    /// approval and nothing else, which is what a call reads back as *ask the
    /// proxy for this one by name*.
    fn nvs_core_http_allow_url(ctx, args: [1]) {
        let text = text_of(args, "allowUrl")?;
        let approved = pin(ctx, text, MEMBER, Roster::Laundered)?;

        let mut addresses = NvsArray::new();
        for address in approved {
            addresses.append(Value::str(NvsStr::new(address.to_string().as_bytes())));
        }
        Ok(crate::instance::build(
            &TARGET,
            [
                Value::str(NvsStr::new(text.as_bytes())),
                Value::array(addresses),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http::methodName(Core\Http\Method $method): string` — the case's
    /// wire token, `GET` for `Get`.
    ///
    /// The token is the case's own name upper-cased, read off
    /// [`crate::router::METHOD`]'s roster through
    /// [`crate::router::method_verb`]: the same derivation
    /// `Core\Http\Client::request` sends a verb with, so the text a program
    /// writes into an `Allow` header is the text a request goes out with, and
    /// there is no second table of the eight tokens to disagree with the first.
    /// One allocation of at most seven bytes, written once.
    fn nvs_core_http_method_name(_ctx, args: [1]) {
        let verb = crate::router::method_verb(&args[0], r"Core\Http::methodName")?;
        Ok(Value::str(NvsStr::build(verb.len(), |out| {
            for byte in verb.bytes() {
                out.push(&[byte.to_ascii_uppercase()]);
            }
        })))
    }
}

/// The identity a call's `identity` option names, built out of the chain and
/// the key that option's [`IDENTITY`] holds — or `None` where it named none.
///
/// The session is built here, once per call and released with it, rather than
/// being kept behind the identity value: [`IDENTITY`]'s own doc argues why, and
/// the pool is what makes it cheap, since a reused connection handshakes not at
/// all.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver or a slot of the wrong tag, and for a chain
/// and a key [`nvs_host::tls::NvsIdentity::read`] refuses. Neither is reachable
/// from source: the option's declared type is checked while compiling, and
/// those slots were written by a read of the same two octet strings that had
/// already been accepted.
fn identity_option(
    args: &[Value],
    member: &str,
    bag: Bag,
) -> Result<Option<transport::Identity>, Fault> {
    if matches!(args[bag.identity].tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(None);
    }

    let receiver = crate::instance::receiver(args[bag.identity], &IDENTITY, member)?;
    let chain_slot = crate::instance::slot(receiver, IDENTITY_CHAIN_SLOT);
    let pkcs8_slot = crate::instance::slot(receiver, IDENTITY_PKCS8_SLOT);
    let print_slot = crate::instance::slot(receiver, IDENTITY_FINGERPRINT_SLOT);
    let chain = identity_octets(&chain_slot, IDENTITY_CHAIN_SLOT, member)?;
    let pkcs8 = identity_octets(&pkcs8_slot, IDENTITY_PKCS8_SLOT, member)?;
    let fingerprint = identity_octets(&print_slot, IDENTITY_FINGERPRINT_SLOT, member)?;

    let session = nvs_host::tls::NvsIdentity::read(chain, pkcs8).map_err(|why| {
        Fault::fatal(format!(
            "{member}: an identity that had already been read was refused the second time: {why}"
        ))
    })?;
    Ok(Some(transport::Identity {
        session,
        fingerprint: String::from_utf8_lossy(fingerprint).into_owned(),
    }))
}

/// The octets an [`IDENTITY`] slot holds.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the slot: every one of them is written by
/// [`nvs_core_http_identity_read`] and by nothing else, so another tag is a
/// compiled-code bug rather than anything a program can write.
fn identity_octets<'a>(held: &'a Value, at: usize, member: &str) -> Result<&'a [u8], Fault> {
    held.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{IDENTITY_NAME}::{member} expected a `bytes` in its `{}` slot",
            IDENTITY.slots[at]
        ))
    })
}

/// The refusal both halves of a client identity share —
/// [`crate::crypto::KEY_PAIR`]'s reading of the same question, over a
/// certificate as well as a key.
///
/// A `LogicError` and not a `RuntimeError`: a client identity is what the
/// deployment handed this program to prove *itself* with, so there is nobody a
/// verdict could be about and every way of failing is a mistake in the files.
/// `why` is [`nvs_host::tls::NvsIdentity::read`]'s own sentence, which names
/// which half did not hold.
fn identity_refused(why: &std::io::Error) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!("{IDENTITY_NAME}::read(): {why}."),
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\Identity::read(bytes $chainPem, Crypto\KeyPair $key): Core\Http\Identity`
    /// — the chain a request presents when a server asks the client for one.
    ///
    /// The chain and the key are parsed here to be refused, not to be kept, for
    /// `Core\Crypto\KeyPair::read`'s reason: what the object holds is the octets
    /// that were validated, and [`identity_of`] reads them back where a
    /// connection is opened. The fingerprint is the third slot because it is the
    /// only one a pool key may carry — a leaf certificate is public, and the two
    /// beside it are not.
    fn nvs_core_http_identity_read(_ctx, args: [2]) {
        let chain = args[0].as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{IDENTITY_NAME}::read expected a `bytes`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let (held, _) = crate::crypto::stored_key(args, 1, &crate::crypto::KEY_PAIR, "read")?;
        let pkcs8 = held.as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{IDENTITY_NAME}::read expected a `bytes` in the key pair's first slot"
            ))
        })?;

        let identity =
            nvs_host::tls::NvsIdentity::read(chain, pkcs8).map_err(|why| identity_refused(&why))?;
        let fingerprint = fingerprint_of(identity.leaf());
        nvs_runtime::affordable(Some(chain.len() + pkcs8.len()), "Core\\Http\\Identity::read")?;

        Ok(crate::instance::build(
            &IDENTITY,
            [
                Value::bytes(NvsStr::new(chain)),
                Value::bytes(NvsStr::new(pkcs8)),
                Value::str(NvsStr::new(fingerprint.as_bytes())),
            ],
        ))
    }
}

/// A leaf certificate's SHA-256 as lower-case hex, which is what names an
/// identity everywhere one is named at all.
///
/// The leaf rather than the whole chain: the key matched *it*, so two identities
/// with the same leaf are the same client however their intermediates were
/// written. The digest rather than the certificate: a pool key is a string that
/// is compared on every draw, and 64 octets compare faster than a kilobyte of
/// DER while saying exactly as much.
fn fingerprint_of(leaf: &[u8]) -> String {
    use sha2::Digest as _;
    data_encoding::HEXLOWER.encode(&sha2::Sha256::digest(leaf))
}

// ------------------------------------------------------------------- the client

/// [`CLIENT`]'s name, written once — see [`NAME`].
pub(crate) const CLIENT_NAME: &str = r"Core\Http\Client";

/// [`RESPONSE`]'s name, written once — see [`NAME`].
pub(crate) const RESPONSE_NAME: &str = r"Core\Http\Response";

/// [`TLS_INFO`]'s name, written once — see [`NAME`].
pub(crate) const TLS_INFO_NAME: &str = r"Core\Http\TlsInfo";

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
/// The keys every outbound call carries, whatever it opens: the two waits, the
/// headers, who this end is when the server asks, the address the connect is
/// made to, and `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s
/// policy over the connection.
///
/// **A bag is a closed set of keys** (`rule:core-api/shape-rules` R2), and these
/// are the ones that mean something to a request and to a socket alike. What a
/// request-and-reply exchange adds — the redirect count, the retry trio and the
/// body keys — [`request_options`] writes into the `$exchange` position here,
/// so `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`'s
/// bag can take the connection half without the keys that could never do
/// anything on a socket, and the shared half still has one home rather than two
/// copies that would say the same thing until they did not.
// The keys at the indentation they had as a slice because rustfmt does not
// reach inside a macro's body: what opening a connection says, the exchange
// keys a row inserts into the middle of it, and the group a row adds at the end.
macro_rules! connection_options {
    ($($exchange:expr),* $(,)? ; $($trailing:expr),* $(,)?) => { &[
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
    $($exchange,)*
    // The first of the keys that are about the connection rather than about
    // what goes over it: who this end is when the server asks. An object rather
    // than two `bytes` keys, because a chain and a key that have not been
    // checked against each other are exactly what [`IDENTITY`]'s read exists to
    // refuse — and because the pool is keyed on the identity, which needs one
    // value to name.
    CoreOption {
        name: IDENTITY_OPTION,
        ty: CoreTy::Instance(IDENTITY_NAME),
        default: Const::Null,
    },
    // `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s keys: the
    // four a `capabilities.tls` grant unlocks for the URL's host, and the fifth
    // that needs no grant. Separate keys rather than one policy object, because
    // each asks a different question — whose certificates, which key, which
    // name, whether to look at all — and a bag nested inside a bag has nothing
    // to flatten into (`rule:core-api/shape-rules` R2).
    //
    // Unqualified, which is what refuses a `tainted` blob here: an options bag's
    // member carries no mark for `nvs_types::core_lib`'s `qual_of` to read, and
    // what has no mark refuses a qualified argument. Trust anchors chosen by
    // whoever is being verified are the whole of what this key must not admit.
    CoreOption {
        name: TLS_CA_OPTION,
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    // One pin or a list of them: a key being rotated is two live pins for as
    // long as the rotation takes, and a program that could name only one would
    // have to stop pinning to get through it.
    CoreOption {
        name: TLS_PIN_OPTION,
        ty: CoreTy::Union(&[
            CoreTy::Text(Qual::Neutral),
            CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
        ]),
        default: Const::Null,
    },
    // `true` unless the call writes `false`, rather than an omission standing
    // for strictness: the default a call inherits is the safe answer, and the
    // relaxation is a word at the call site rather than the absence of one.
    CoreOption {
        name: TLS_VERIFY_HOST_OPTION,
        ty: CoreTy::Bool,
        default: Const::Bool(true),
    },
    CoreOption {
        name: TLS_VERIFY_OPTION,
        ty: CoreTy::Bool,
        default: Const::Bool(true),
    },
    // The key of the group that needs no grant, because it can only tighten: a
    // floor under `[http.client.tls] min_version` throws rather than lowering
    // what the deployment set.
    CoreOption {
        name: TLS_MIN_VERSION_OPTION,
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    // `rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`'s
    // key: the address this call connects to, instead of the one the URL's host
    // resolves to. Last of the shared keys, and unqualified like the URL beside
    // it for `rule:security/outbound-url-is-a-sink`'s reason — the address a
    // connect is made to is a sink, and an address that came from outside is
    // the whole of what it must not admit.
    CoreOption {
        name: CONNECT_TO_OPTION,
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    $($trailing,)*
] };
}

/// A macro rather than a slice, for [`request_params`]' reason one axis over:
/// `stream` carries these keys and the two bounds a body read as it arrives has
/// of its own, so one row's bag is the other's with a group after it. A `const`
/// slice cannot be extended, and a second copy of the shared keys would say the
/// same thing until it did not.
// The exchange keys, written into [`connection_options`]' middle: what a
// request-and-reply sends and how many times, and the one relaxation that only
// a redirect can reach.
macro_rules! request_options {
    ($($trailing:expr),* $(,)?) => { connection_options!(
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
    ;
    // `rule:http-server/an-https-redirect-never-becomes-plaintext`'s half of
    // the pair the deployment does not hold. `false` unless the call writes
    // otherwise, like the two relaxing bools in the shared half and for the same
    // reason: the default a call inherits is the safe answer, and a hop out of
    // TLS is a word at the call site rather than the absence of one. It is a
    // trailing key rather than a shared one because a socket follows no
    // redirect, so there is no hop for it to permit.
    CoreOption {
        name: REDIRECT_TO_HTTP_OPTION,
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    $($trailing),*
) };
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

/// `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`'s bag:
/// the keys every outbound call carries, and
/// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`'s
/// group after them.
///
/// **The exchange half is absent rather than ignored.** There is no `body`, no
/// `followRedirects` and no retry trio, because a bag is a closed set of keys
/// and a key that could never do anything on a socket is one a program would
/// write and then wait for. `idle` and `maxDuration` are the two bounds a
/// streamed reply already carries ([`STREAM_OPTIONS`]) and mean the same two
/// things one level down: an idle check alone never ends a peer that dribbles,
/// and a lifetime alone lets a dead connection sit until it expires.
const SOCKET_OPTIONS: &[CoreOption] = connection_options!(
    ;
    PROTOCOLS_OPTION,
    IDLE_OPTION,
    MAX_DURATION_OPTION,
    MAX_MESSAGE_OPTION,
    SEND_TIMEOUT_OPTION,
    PING_OPTION,
);

/// The subprotocols the opening request offers as `Sec-WebSocket-Protocol` —
/// see [`SOCKET_OPTIONS`].
///
/// An empty array rather than a `null` default, because offering none and
/// offering an empty list are one thing: there is nothing a program could mean
/// by the difference, so there is no spelling for it.
const PROTOCOLS_OPTION: CoreOption = CoreOption {
    name: "protocols",
    ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
    default: Const::EmptyArray,
};

/// The largest message a socket reassembles, past which it is closed with
/// `1009` — see [`SOCKET_OPTIONS`]. A `uint` of bytes rather than a `Duration`,
/// and neither type has an infinite value.
const MAX_MESSAGE_OPTION: CoreOption = CoreOption {
    name: "maxMessage",
    ty: CoreTy::Uint,
    default: Const::Null,
};

/// How long a frame may wait to be written — see [`SOCKET_OPTIONS`].
const SEND_TIMEOUT_OPTION: CoreOption = CoreOption {
    name: "sendTimeout",
    ty: DURATION,
    default: Const::Null,
};

/// The silence after which a ping is sent — see [`SOCKET_OPTIONS`]. The one
/// knob here with an off position, and it is off by default, because a ping is
/// traffic the peer did not ask for: a program that sets it is choosing to have
/// `idle` end a *dead* peer rather than a quiet one.
const PING_OPTION: CoreOption = CoreOption {
    name: "ping",
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

/// The key a call names an [`IDENTITY`] under, spelled once.
pub(crate) const IDENTITY_OPTION: &str = "identity";

/// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s keys, spelled
/// once: [`OPTIONS`] declares them, and the check that asks each key's grant of
/// the request's own snapshot names the same spelling in its refusal, so a
/// renamed key cannot leave the refusal talking about a key no row declares.
pub(crate) const TLS_CA_OPTION: &str = "tlsCa";
/// See [`TLS_CA_OPTION`].
pub(crate) const TLS_PIN_OPTION: &str = "tlsPin";
/// See [`TLS_CA_OPTION`].
pub(crate) const TLS_VERIFY_HOST_OPTION: &str = "tlsVerifyHost";
/// See [`TLS_CA_OPTION`].
pub(crate) const TLS_VERIFY_OPTION: &str = "tlsVerify";
/// The floor a call may raise without a grant, spelled beside the four that
/// need one — see [`TLS_CA_OPTION`].
pub(crate) const TLS_MIN_VERSION_OPTION: &str = "tlsMinVersion";
/// `rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`'s
/// key, spelled once for the row that declares it, the grant question that
/// unlocks it and the refusal a `Core\Http\Target` beside it raises.
pub(crate) const CONNECT_TO_OPTION: &str = "connectTo";
/// `rule:http-server/an-https-redirect-never-becomes-plaintext`'s key, spelled
/// once: the row declares it and the refusal for a hop the call did not expect
/// names the same spelling.
pub(crate) const REDIRECT_TO_HTTP_OPTION: &str = "redirectToHttp";
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
    doc: Some(&PART_CARD),
    methods: &[
        CoreMethod {
            name: "file",
            names: &["path"],
            // A sink in the path, as every path in `Core\IO` is and for that
            // class's reason: `..` and the separators direct the resolver, so a
            // `tainted` path is refused where it is written.
            params: &[CoreTy::Path(Qual::Sink), CoreTy::Options(PART_FILE_OPTIONS)],
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
            desc: "The file to send. A relative path must be a string literal, and is joined to \
                   the folder of the file that contains it.",
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
/// The client identity's slot, last of the shared keys — see [`DEADLINE`].
const IDENTITY_AT: usize = 13;
/// The first of the relaxing keys, whose group [`connection_options`] emits as
/// one run: the other four are counted off this one by [`Bag`], so a bag's
/// flattening of the whole group is this number. See [`DEADLINE`].
const TLS_CA: usize = 14;
/// The address a call names for itself, last of the shared keys — see
/// [`DEADLINE`].
const CONNECT_TO: usize = 19;
/// The one hop a call has to expect before it is taken, last of the shared keys
/// — see [`DEADLINE`].
const REDIRECT_TO_HTTP: usize = 20;
/// [`STREAM_OPTIONS`]' own two slots, after every shared key's, and reachable
/// only from the one row that declares them — see [`DEADLINE`].
const IDLE: usize = 21;
/// See [`IDLE`].
const MAX_DURATION: usize = 22;

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

/// The ABI slot each of [`SOCKET_OPTIONS`]' keys flattens into. The shared
/// connection keys come first and in their own order, so these are not
/// [`DEADLINE`]'s numbers: a bag without the exchange half is a different
/// flattening of a different closed set, and reusing one row's indices for
/// another's bag is how a body comes to read the key beside the one it meant.
const SOCKET_DEADLINE: usize = 1;
/// See [`SOCKET_DEADLINE`].
const SOCKET_CONNECT_TIMEOUT: usize = 2;
/// See [`SOCKET_DEADLINE`].
const SOCKET_HEADERS: usize = 3;
/// The client identity's slot, first of the keys after the exchange half's hole
/// — see [`SOCKET_DEADLINE`].
const SOCKET_IDENTITY: usize = 4;
/// The first of the five relaxing TLS keys, which [`connection_options`] emits
/// as one run — see [`SOCKET_DEADLINE`] and [`Bag::relaxing`].
const SOCKET_TLS_CA: usize = 5;
/// See [`SOCKET_DEADLINE`].
const SOCKET_CONNECT_TO: usize = 10;
/// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`'s
/// group, after every connection key's slot — see [`SOCKET_DEADLINE`].
const SOCKET_PROTOCOLS: usize = 11;
/// See [`SOCKET_PROTOCOLS`].
const SOCKET_IDLE: usize = 12;
/// See [`SOCKET_PROTOCOLS`].
const SOCKET_MAX_DURATION: usize = 13;
/// See [`SOCKET_PROTOCOLS`].
const SOCKET_MAX_MESSAGE: usize = 14;
/// See [`SOCKET_PROTOCOLS`].
const SOCKET_SEND_TIMEOUT: usize = 15;
/// See [`SOCKET_PROTOCOLS`].
const SOCKET_PING: usize = 16;

/// How many arguments `openSocket` takes: the URL plus one per key of
/// [`SOCKET_OPTIONS`]. Derived for [`REQUEST_ARITY`]'s reason.
pub(crate) const SOCKET_ARITY: usize = SOCKET_OPTIONS.len() + 1;

/// Where one row's bag put the keys every row shares, and which schemes its URL
/// may name.
///
/// The readings below — the pin, the trust grants, the policy and the identity
/// — are one question asked by every row of `Core\Http\Client`, and the answer
/// is in a different ABI slot for each bag: a bag without the exchange half
/// flattens the same keys onto lower numbers. Handing the flattening in is what
/// keeps those readings one implementation rather than one per bag, which is
/// [`connection_options`]' argument one layer down.
#[derive(Clone, Copy)]
struct Bag {
    /// The first of the five relaxing TLS keys. The other four are the next
    /// four slots, because [`connection_options`] emits the group as one run
    /// and this is the offset that run starts at.
    relaxing: usize,
    /// Where `identity` landed.
    identity: usize,
    /// Where `connectTo` landed.
    connect_to: usize,
    /// The schemes this row's URL may name — see [`Roster`].
    roster: Roster,
}

/// The relaxing group's five slots, counted off [`Bag::relaxing`] in the order
/// [`connection_options`] writes them. The order lives there and is read here,
/// so a key inserted into that run moves every slot at once rather than in as
/// many places as there are bags.
impl Bag {
    /// `tlsCa`'s slot.
    const fn ca(self) -> usize {
        self.relaxing
    }

    /// `tlsPin`'s slot.
    const fn pin(self) -> usize {
        self.relaxing + 1
    }

    /// `tlsVerifyHost`'s slot.
    const fn verify_host(self) -> usize {
        self.relaxing + 2
    }

    /// `tlsVerify`'s slot.
    const fn verify(self) -> usize {
        self.relaxing + 3
    }

    /// `tlsMinVersion`'s slot.
    const fn min_version(self) -> usize {
        self.relaxing + 4
    }
}

/// [`OPTIONS`]' flattening, which every request row and `stream` share.
const REQUEST_BAG: Bag = Bag {
    relaxing: TLS_CA,
    identity: IDENTITY_AT,
    connect_to: CONNECT_TO,
    roster: Roster::Request,
};

/// [`SOCKET_OPTIONS`]' flattening, which the one row that opens a socket has to
/// itself.
const SOCKET_BAG: Bag = Bag {
    relaxing: SOCKET_TLS_CA,
    identity: SOCKET_IDENTITY,
    connect_to: SOCKET_CONNECT_TO,
    roster: Roster::Socket,
};

/// [`TARGET`]'s two slots, by index — see [`STATUS_SLOT`].
const TARGET_URL_SLOT: usize = 0;
/// See [`TARGET_URL_SLOT`].
const TARGET_ADDRESSES_SLOT: usize = 1;

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
/// See [`DEFAULT_DEADLINE`]. `[http.client.socket] max_message`'s shipped
/// value, in octets: what `nvs_server::bounds` already applies to the inbound
/// half of RFC 6455, because one process holding two opinions about the size of
/// one message is how a program comes to work in one direction and not the
/// other.
const DEFAULT_MAX_MESSAGE: u64 = 4 << 20;
/// See [`DEFAULT_DEADLINE`]. `[http.client.socket] send_timeout`'s shipped
/// value: how long one frame may wait to be written before the peer that
/// stopped reading is a peer this end stops writing to.
const DEFAULT_SEND_TIMEOUT: Duration = Duration::from_secs(30);

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
    doc: Some(&CLIENT_CARD),
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
        // The only row that opens something the program then holds, and the
        // only one that answers no reply. It is a row here rather than a door of
        // its own because the opening handshake **is** an outbound call
        // (`rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`):
        // the pin, the grants, the TLS policy, an identity and the proxy apply
        // by the row being on this class rather than by being told to, and a
        // second door would be the copy that comes to be missing a check. Its
        // bag is the connection half of every other row's ([`SOCKET_OPTIONS`]).
        CoreMethod {
            name: "openSocket",
            names: &["url"],
            params: &[URL, CoreTy::Options(SOCKET_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(socket::SOCKET_NAME),
            symbol: socket::OPEN_SYMBOL,
            doc: Some(&OPEN_SOCKET_DOC),
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
///
/// **`tls` is the one reader that answers `null`**, and that answer is the
/// reply's own rather than a value gone missing: a plain `http` exchange and one
/// a test's table served had no session to report
/// (`rule:http-server/a-reply-reports-its-tls-session`).
pub(crate) const RESPONSE: CoreClass = CoreClass {
    name: RESPONSE_NAME,
    doc: Some(&RESPONSE_CARD),
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
        CoreMethod {
            name: "tls",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(TLS_INFO_NAME)),
            symbol: "nvs_core_http_response_tls",
            doc: Some(&TLS_DOC),
        },
    ],
    slots: &["status", "body", "headers", "tls"],
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
/// [`RESPONSE`]'s session slot, holding a [`TLS_INFO`] instance or `null`. See
/// [`STATUS_SLOT`].
const TLS_SLOT: usize = 3;

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

/// `Core\Http\Response::tls`'s reference card — `rule:core-api/reference-card`.
const TLS_DOC: MethodDoc = MethodDoc {
    short: "The TLS session this reply arrived over: what the handshake settled, and whether it \
            checked the peer — replacing the scattered `CURLINFO_SSL_*` keys of `curl_getinfo`.",
    params: &[],
    ret: "The session, or `null` where the reply arrived over none at all — a plain `http` \
          exchange, or a call a test's answer table served.",
    errors: &[],
};

/// `Core\Http\Response`'s own card — `rule:core-api/reference-card`.
const RESPONSE_CARD: ClassDoc = ClassDoc {
    short: "The reply to an outbound HTTP call: its status code, its headers, its body, and the \
            TLS connection it came over. `Core\\Http\\Client::get()` returns one.",
};

// ---------------------------------------------------------- the session reported

/// `Core\Http\TlsInfo` — what one reply's session settled, and whether it
/// checked anything (`rule:http-server/a-reply-reports-its-tls-session`).
///
/// **`verified` is the member this class exists for.** A deployment that relaxed
/// verification for one partner host has no other way to assert, in a test and
/// in production telemetry, that every *other* call still verified — the grant
/// is otherwise unobservable from inside the language.
///
/// **Four slots answer seven members**, because three of them are one reading of
/// one certificate: `subject`, `issuer` and `expiry` parse the leaf of the chain
/// the slot already holds, where `nvs_host::tls::leaf` is the parser. Filling
/// three more slots at the framing would charge every reply that arrived over
/// TLS for a parse that the call auditing a partner's certificate is the only
/// one to want, and a reply's certificate is not read by the ordinary call.
///
/// The chain is `tainted` and no timing is here: where a call's time went is the
/// `http` trace event's (`rule:observability/trace-events-carry-a-kind`).
pub(crate) const TLS_INFO: CoreClass = CoreClass {
    name: TLS_INFO_NAME,
    doc: Some(&TLS_INFO_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "version",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_http_tls_info_version",
            doc: Some(&VERSION_DOC),
        },
        CoreMethod {
            name: "cipher",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_http_tls_info_cipher",
            doc: Some(&CIPHER_DOC),
        },
        CoreMethod {
            name: "verified",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_http_tls_info_verified",
            doc: Some(&VERIFIED_DOC),
        },
        CoreMethod {
            name: "peerChain",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::TaintedStr),
            symbol: "nvs_core_http_tls_info_peer_chain",
            doc: Some(&PEER_CHAIN_DOC),
        },
        CoreMethod {
            name: "subject",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_http_tls_info_subject",
            doc: Some(&SUBJECT_DOC),
        },
        CoreMethod {
            name: "issuer",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_http_tls_info_issuer",
            doc: Some(&ISSUER_DOC),
        },
        CoreMethod {
            name: "expiry",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(crate::time::INSTANT_NAME),
            symbol: "nvs_core_http_tls_info_expiry",
            doc: Some(&EXPIRY_DOC),
        },
    ],
    slots: &["version", "cipher", "verified", "chain"],
    constants: &[],
};

/// `Core\Http\TlsInfo`'s own card — `rule:core-api/reference-card`.
const TLS_INFO_CARD: ClassDoc = ClassDoc {
    short: "The details of the TLS connection a reply came over: the TLS version, the cipher, the \
            server's certificates, and whether they were checked. `Core\\Http\\Response::tls()` \
            returns one.",
};

/// [`TLS_INFO`]'s version slot, by index — the layout its `slots` names.
const TLS_VERSION_SLOT: usize = 0;
/// [`TLS_INFO`]'s cipher slot. See [`TLS_VERSION_SLOT`].
const TLS_CIPHER_SLOT: usize = 1;
/// [`TLS_INFO`]'s verification slot, holding `CallPolicy::verifies`. See
/// [`TLS_VERSION_SLOT`].
const TLS_VERIFIED_SLOT: usize = 2;
/// [`TLS_INFO`]'s chain slot, holding the peer's certificates as DER, leaf
/// first. See [`TLS_VERSION_SLOT`].
const TLS_CHAIN_SLOT: usize = 3;

/// `Core\Http\TlsInfo::version`'s reference card — `rule:core-api/reference-card`.
const VERSION_DOC: MethodDoc = MethodDoc {
    short: "The protocol version the handshake settled on.",
    params: &[],
    ret: "`TLSv1.3` or `TLSv1.2`, spelled as every TLS tool prints it, or the registry's own \
          `0x` code for a version this build negotiated and cannot name.",
    errors: &[],
};

/// `Core\Http\TlsInfo::cipher`'s reference card — `rule:core-api/reference-card`.
const CIPHER_DOC: MethodDoc = MethodDoc {
    short: "The cipher suite the handshake settled on, under its IANA registry name.",
    params: &[],
    ret: "A name such as `TLS_AES_128_GCM_SHA256`, or the suite's `0x` code where the registry \
          has none for it.",
    errors: &[],
};

/// `Core\Http\TlsInfo::verified`'s reference card — `rule:core-api/reference-card`.
const VERIFIED_DOC: MethodDoc = MethodDoc {
    short: "Whether this session checked both the peer's certificate chain and the name on it.",
    params: &[],
    ret: "`true` where the chain was built to a trust anchor — the configured `roots` or the \
          call's own `tlsCa` — and the URL's host was matched against the certificate. `false` \
          where the call dropped either check, which only a `[capabilities.tls]` grant naming \
          the host makes possible.",
    errors: &[],
};

/// `Core\Http\TlsInfo::peerChain`'s reference card — `rule:core-api/reference-card`.
const PEER_CHAIN_DOC: MethodDoc = MethodDoc {
    short: "The certificate chain the peer presented, as PEM, leaf first.",
    params: &[],
    ret: "One PEM block per certificate, leaf first, and an empty array where the peer presented \
          none. Every entry is `tainted`: a certificate is bytes the other end chose.",
    errors: &[],
};

/// `Core\Http\TlsInfo::subject`'s reference card — `rule:core-api/reference-card`.
const SUBJECT_DOC: MethodDoc = MethodDoc {
    short: "The leaf certificate's subject, as RFC 4514 writes a distinguished name.",
    params: &[],
    ret: "The spelling `openssl x509 -subject` prints, such as `CN=api.example.com`. `tainted`, \
          for `peerChain`'s reason.",
    errors: &[LEAF_ERROR],
};

/// `Core\Http\TlsInfo::issuer`'s reference card — `rule:core-api/reference-card`.
const ISSUER_DOC: MethodDoc = MethodDoc {
    short: "The leaf certificate's issuer, as RFC 4514 writes a distinguished name.",
    params: &[],
    ret: "Who signed the leaf, in `subject`'s spelling and `tainted` for its reason.",
    errors: &[LEAF_ERROR],
};

/// `Core\Http\TlsInfo::expiry`'s reference card — `rule:core-api/reference-card`.
const EXPIRY_DOC: MethodDoc = MethodDoc {
    short: "When the leaf certificate stops being valid — its `notAfter` field.",
    params: &[],
    ret: "The instant the peer's certificate expires, which is the check an operator most wants \
          a program to make about a partner it calls.",
    errors: &[LEAF_ERROR],
};

/// The refusal the three leaf readers share, written once because it is one
/// reading and they differ only in which field they take from it.
const LEAF_ERROR: ErrorDoc = ErrorDoc {
    error: "RuntimeError",
    desc: "The peer presented no certificate, or a leaf that is not X.509 this can read. \
           `version`, `cipher`, `verified` and `peerChain` answer either way, since none of \
           them reads inside a certificate.",
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
macro_rules! connection_params {
    ($($leading:expr),* ; $($exchange:expr),* $(,)? ; $($trailing:expr),* $(,)?) => { &[
    $($leading,)*
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
    $($exchange,)*
    ParamDoc {
        name: "identity",
        desc: "The client certificate to present when the server asks for one, read by \
               `Core\\Http\\Identity::read`. Nothing is presented to a server that does not ask. \
               Two identities never share a pooled connection, and neither does a call that \
               names none.",
        shape: &[],
    },
    ParamDoc {
        name: "tlsCa",
        desc: "The PEM certificates to trust for this call, in place of the runtime's own roots. \
               Needs the URL's host in the `capabilities.tls` `anchors` grant, and a call whose \
               host is not in it throws before connecting.",
        shape: &[],
    },
    ParamDoc {
        name: "tlsPin",
        desc: "One `sha256//<base64>` public-key pin, or several: the peer is accepted when its \
               SubjectPublicKeyInfo hashes to one of them and no chain is built, which is how a \
               self-signed origin is reached. Needs the host in the `pin` grant.",
        shape: &[],
    },
    ParamDoc {
        name: "tlsVerifyHost",
        desc: "Written as `false`, the chain is still built and checked and only the name is \
               skipped. Needs the host in the `any_name` grant.",
        shape: &[],
    },
    ParamDoc {
        name: "tlsVerify",
        desc: "Written as `false`, neither the chain nor the name is checked — the handshake \
               signature still is, so the peer holds the key it presented, but nothing says whose \
               key it is. Needs the host in the `insecure` grant.",
        shape: &[],
    },
    ParamDoc {
        name: "tlsMinVersion",
        desc: "The version floor this call speaks over, `\"1.2\"` or `\"1.3\"`. It needs no grant \
               because it can only tighten, and a value below the runtime's `[http.client.tls] \
               min_version` throws.",
        shape: &[],
    },
    ParamDoc {
        name: "connectTo",
        desc: "The IP address to connect to, instead of resolving the URL's host — which is still \
               the name the certificate is checked against. Needs that host in the \
               `net.connect_to` grant, and the address is judged by the deployment's address \
               policy exactly as a resolved one is. Beside a `Core\\Http\\Target`, which already \
               carries the address its laundering approved, it throws.",
        shape: &[],
    },
    $($trailing,)*
    ] };
}

/// A card's entries for a request row: [`connection_params`]' list with the
/// exchange half written into it — see [`request_options`], whose keys these
/// describe one for one.
macro_rules! request_params {
    ($($leading:expr),* $(,)?) => { request_params!($($leading),* ; ) };
    ($($leading:expr),* ; $($trailing:expr),* $(,)?) => { connection_params!(
    $($leading,)* URL_PARAM, DEADLINE_PARAM ;
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
    }
    ;
    ParamDoc {
        name: "redirectToHttp",
        desc: "Written as `true`, a redirect out of `https` into plaintext may be followed — and \
               only where the hop's host is in the `net.downgrade` grant as well. Left out, such \
               a hop throws, because one `Location` header is otherwise all it takes for an \
               origin to strip a call's TLS. A plain `http` URL asked for directly is untouched \
               by this.",
        shape: &[],
    },
    $($trailing),*
) };
}

/// Every row but `request`'s: the URL, then the bag — see [`request_params`].
const REQUEST_PARAMS: &[ParamDoc] = request_params!();

/// `request`'s: the verb it is handed, ahead of what every other row documents.
const DYNAMIC_PARAMS: &[ParamDoc] = request_params!(METHOD_PARAM);

/// `stream`'s: `request`'s list, and after it the two bounds that row's own bag
/// declares — see [`STREAM_OPTIONS`].
const STREAM_PARAMS: &[ParamDoc] = request_params!(METHOD_PARAM ; IDLE_PARAM, MAX_DURATION_PARAM);

/// `openSocket`'s: the connection keys, then the group
/// `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`
/// adds — one entry per key of [`SOCKET_OPTIONS`], in its order.
const SOCKET_PARAMS: &[ParamDoc] = connection_params!(
    SOCKET_URL_PARAM, SOCKET_DEADLINE_PARAM ; ;
    PROTOCOLS_PARAM,
    SOCKET_IDLE_PARAM,
    SOCKET_MAX_DURATION_PARAM,
    MAX_MESSAGE_PARAM,
    SEND_TIMEOUT_PARAM,
    PING_PARAM,
);

/// Where a request goes — see [`REQUEST_PARAMS`].
const URL_PARAM: ParamDoc = ParamDoc {
    name: "url",
    desc: "Where the request goes: a URL the program itself authored, or the \
           `Core\\Http\\Target` that `Core\\Http::allowUrl` pinned. A `tainted` value is \
           refused here and accepted only at that launderer.",
    shape: &[],
};

/// The budget a whole request runs under — see [`REQUEST_PARAMS`].
const DEADLINE_PARAM: ParamDoc = ParamDoc {
    name: "deadline",
    desc: "The whole call's budget, covering the connection, every redirect hop, every retry \
           attempt and every backoff between them. Omitted, the runtime's `[http.client] \
           deadline` applies; there is no spelling for no deadline at all.",
    shape: &[],
};

/// Where a socket goes — see [`SOCKET_PARAMS`].
const SOCKET_URL_PARAM: ParamDoc = ParamDoc {
    name: "url",
    desc: "The peer to open the socket to, as a `ws` or `wss` URL the program itself authored, or \
           the `Core\\Http\\Target` that `Core\\Http::allowUrl` pinned. An `http` or `https` URL \
           is refused here and a `tainted` one is accepted only at that launderer.",
    shape: &[],
};

/// The budget the opening handshake runs under — see [`SOCKET_PARAMS`].
const SOCKET_DEADLINE_PARAM: ParamDoc = ParamDoc {
    name: "deadline",
    desc: "The opening handshake's budget, covering the connection and every address tried, and \
           ending at the `101`. What bounds the conversation after it is `idle` and \
           `maxDuration`. Omitted, the runtime's `[http.client] deadline` applies; there is no \
           spelling for no deadline at all.",
    shape: &[],
};

/// The subprotocols a socket offers — see [`SOCKET_PARAMS`].
const PROTOCOLS_PARAM: ParamDoc = ParamDoc {
    name: "protocols",
    desc: "The subprotocols to offer as `Sec-WebSocket-Protocol`, most preferred first. Each \
           name is one word of letters, digits and the punctuation an HTTP token allows, such as \
           `.`, `-` and `_`. A space, a comma or a control character throws an error. A peer that chooses one of them is reported by \
           `Core\\Http\\Socket::protocol`, and one that chooses a name that was never offered is \
           refused.",
    shape: &[],
};

/// The silence bound a socket carries — see [`SOCKET_PARAMS`].
const SOCKET_IDLE_PARAM: ParamDoc = ParamDoc {
    name: "idle",
    desc: "The longest the socket may go silent for in either direction. Omitted, the runtime's \
           `[http.client] idle` applies; there is no spelling for no bound at all. A `ping` is \
           what makes this end a dead peer rather than a quiet one.",
    shape: &[],
};

/// The lifetime bound a socket carries — see [`SOCKET_PARAMS`].
const SOCKET_MAX_DURATION_PARAM: ParamDoc = ParamDoc {
    name: "maxDuration",
    desc: "The longest the socket may live altogether, which is what ends a peer dribbling a \
           frame at a time under every idle check. Omitted, the runtime's `[http.client] \
           max_duration` applies.",
    shape: &[],
};

/// The message cap a socket carries — see [`SOCKET_PARAMS`].
const MAX_MESSAGE_PARAM: ParamDoc = ParamDoc {
    name: "maxMessage",
    desc: "The largest message, in bytes, this end reassembles a peer's fragments into. A message \
           past it closes the socket with `1009` and the waiting `receive` throws naming the cap. \
           Omitted, the runtime's `[http.client.socket] max_message` applies.",
    shape: &[],
};

/// The send bound a socket carries — see [`SOCKET_PARAMS`].
const SEND_TIMEOUT_PARAM: ParamDoc = ParamDoc {
    name: "sendTimeout",
    desc: "The longest a frame may wait to be written to a peer that is not reading, and the \
           wait `close` gives the peer's own close frame. Omitted, the runtime's \
           `[http.client.socket] send_timeout` applies.",
    shape: &[],
};

/// The ping knob — see [`SOCKET_PARAMS`].
const PING_PARAM: ParamDoc = ParamDoc {
    name: "ping",
    desc: "The silence after which this end sends a ping, so that `idle` ends a dead peer rather \
           than a quiet one. Left out, none is sent — a ping is traffic the peer did not ask for. \
           A peer's own ping is answered either way, which is the protocol rather than a policy.",
    shape: &[],
};

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

/// `Core\Http\Client::openSocket`'s reference card — `rule:core-api/reference-card`.
const OPEN_SOCKET_DOC: MethodDoc = MethodDoc {
    short: "Opens a WebSocket to `$url` and answers it once the peer's `101` has arrived — the \
            row for a realtime API a program talks with, rather than a reply it reads and is \
            done with.",
    params: SOCKET_PARAMS,
    ret: "A `Core\\Http\\Socket` carrying the conversation: `receive()` answers the peer's next \
          message as the `Core\\Socket\\Message` a server-side connection already answers with, \
          `send()` and `sendBytes()` write the two payload kinds, and `close()` ends it. It \
          belongs to the task that opened it and is closed with `1001` when that task ends, so a \
          socket never outlives the request, command or job that holds it.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The URL is refused: it is not a URL, its scheme is neither `ws` nor `wss` — this \
               row refuses `http` and `https` exactly as every other row refuses `ws` and `wss` — \
               it names no host, `net.connect` does not grant that host, or it resolves to an \
               address the deployment's policy denies. An option is outside its bounds: a \
               `deadline`, `connectTimeout`, `idle`, `maxDuration`, `sendTimeout` or `ping` that \
               is not a positive duration, or a `protocols` entry that is not one word. Or the \
               peer did not open a socket: it answered a \
               redirect, which is never followed and names its `Location`; it answered any other \
               status than `101`; or it chose a subprotocol that `protocols` never offered.",
    }],
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

/// Every relaxing option this call wrote, against the grant that unlocks it,
/// and the floor it asks for against the one the deployment set.
///
/// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s two halves:
/// the deployment says which hosts may be reached with verification relaxed and
/// the call says it wants that here, so a grant relaxes nothing on its own and
/// an option outside its grant never opens a socket. The question goes to the
/// request's own snapshot through the door every other capability uses
/// (`rule:security/capability-question-is-grant-and-scope`), and the class is
/// the one difference: a `LogicError`, because a program asking for its own
/// guarantee to be weakened at a host nobody named has made a mistake rather
/// than met a refusal it can degrade around
/// ([`nvs_runtime::capability::require_as`]).
///
/// Asked here beside the other judges rather than in the transport, which holds
/// no `Ctx` and by then holds a socket: what a relaxed call must not do is
/// announce this process to the host before the deployment has been asked about
/// it. The scheme is not consulted — the ask is the ask, and a `tlsCa` written
/// beside an `http` URL is the same request for trust nobody granted.
///
/// # Errors
///
/// A thrown `LogicError` naming the grant, for an option whose host the grant
/// does not list, and [`judge_floor`]'s. [`judged_host`]'s three cannot fire: a
/// URL that reaches here has already been approved.
fn judge_trust(ctx: &Ctx, args: &[Value], url: &str, member: &str, bag: Bag) -> Result<(), Fault> {
    let host = judged_host(url, member, bag.roster)?;
    for (asked, cap, option) in [
        (
            !matches!(args[bag.ca()].tag(), Some(Tag::Null)),
            Cap::TlsAnchors,
            TLS_CA_OPTION,
        ),
        (
            !matches!(args[bag.pin()].tag(), Some(Tag::Null)),
            Cap::TlsPin,
            TLS_PIN_OPTION,
        ),
        (
            args[bag.verify_host()].as_bool() == Some(false),
            Cap::TlsAnyName,
            TLS_VERIFY_HOST_OPTION,
        ),
        (
            args[bag.verify()].as_bool() == Some(false),
            Cap::TlsInsecure,
            TLS_VERIFY_OPTION,
        ),
    ] {
        if asked {
            nvs_runtime::capability::require_as(
                ctx,
                cap,
                Scope::Host(&host),
                ThrownClass::Logic,
                &format!("{member}'s `{option}`"),
            )?;
        }
    }
    judge_floor(ctx, args, member, bag)
}

/// `tlsMinVersion` against `[http.client.tls] min_version`: the one key of the
/// group that needs no grant, because a call may only raise the floor.
///
/// One direction, checked here rather than taken as the larger of the two
/// silently, so a program that believes it is speaking 1.3 over a deployment
/// that allows 1.2 learns which of them is wrong. The two spellings are the
/// versions this client implements, which is what makes anything else a
/// mistake rather than a version it might grow into.
///
/// # Errors
///
/// A thrown `LogicError` naming `tlsMinVersion`, for a floor under the
/// deployment's or outside the two versions. A directive that will not parse is
/// not an error here, for [`bound_of`]'s reason.
fn judge_floor(ctx: &Ctx, args: &[Value], member: &str, bag: Bag) -> Result<(), Fault> {
    let Some(asked) = args[bag.min_version()].as_text() else {
        return Ok(());
    };
    let Some(rank) = tls_version_rank(asked) else {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{member}: `tlsMinVersion` is `\"1.2\"` or `\"1.3\"` — this client implements \
                 neither TLS 1.0 nor 1.1 — and this call asked for `\"{asked}\"`"
            ),
        ));
    };
    let floor = ctx
        .config()
        .and_then(|config| config.get("http.client.tls.min_version"));
    let floor = floor.as_deref().unwrap_or(DEFAULT_MIN_VERSION);
    if rank < tls_version_rank(floor).unwrap_or(0) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{member}: `tlsMinVersion` can only raise the floor, and `\"{asked}\"` is under \
                 the `\"{floor}\"` this deployment set in `[http.client.tls] min_version`"
            ),
        ));
    }
    Ok(())
}

/// Where a TLS version spelling sits against the others, or `None` for a text
/// that is not one this client speaks.
///
/// An order and not a parse: the only question either caller has is which of
/// two floors is higher, and a number that is not a version keeps the answer
/// from being read as one.
fn tls_version_rank(version: &str) -> Option<u8> {
    match version.trim() {
        "1.2" => Some(2),
        "1.3" => Some(3),
        _ => None,
    }
}

/// The floor a deployment that configured none speaks over — `[http.client.tls]
/// min_version`'s shipped value, which is this client's lowest.
const DEFAULT_MIN_VERSION: &str = "1.2";

/// The five relaxing keys as one value, for the session the handshake builds.
///
/// Read here rather than in the transport for [`judge_trust`]'s reason, one
/// step further on: every field of this has just been proved against the grant
/// that unlocks it, so a policy that reaches a socket is one the deployment
/// already answered for, and the transport carries a decision rather than a
/// question it holds no `Ctx` to ask. A call that wrote none of the keys builds
/// the default, which asks for nothing and handshakes under the process's own
/// configuration — no second `ClientConfig` and nothing to release
/// ([`nvs_host::tls::NvsTls::over_policy`]).
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot whose tag is not what its key declares, which
/// the rows' `string`, `bool` and `string|array<string>` rule out from source.
fn policy_of(args: &[Value], member: &str, bag: Bag) -> Result<CallPolicy, Fault> {
    Ok(CallPolicy {
        anchors: relaxing_text(args, bag.ca(), TLS_CA_OPTION, member)?,
        pins: pins_of(args, member, bag)?,
        any_name: args[bag.verify_host()].as_bool() == Some(false),
        insecure: args[bag.verify()].as_bool() == Some(false),
        min_version: relaxing_text(args, bag.min_version(), TLS_MIN_VERSION_OPTION, member)?,
    })
}

/// One text key of the relaxing group, or `None` where the call left it out.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is neither text nor absent.
fn relaxing_text(
    args: &[Value],
    at: usize,
    option: &str,
    member: &str,
) -> Result<Option<String>, Fault> {
    if matches!(args[at].tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(None);
    }
    let text = args[at].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `string` for `{option}`, got tag {}",
            args[at].tag_byte()
        ))
    })?;
    Ok(Some(text.to_owned()))
}

/// `tlsPin`'s one pin or list of them, flattened to the list a policy holds.
///
/// One key of two shapes because a key being rotated is two live pins, and the
/// single spelling is the list of one: which of the two a call wrote is decided
/// here, so nothing downstream of this asks the question a second time.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot or an element that is neither text nor an
/// array of it, which the row's `string|array<string>` rules out from source.
fn pins_of(args: &[Value], member: &str, bag: Bag) -> Result<Vec<String>, Fault> {
    let at = bag.pin();
    if matches!(args[at].tag(), Some(Tag::Null | Tag::Unset)) {
        return Ok(Vec::new());
    }
    if let Some(one) = args[at].as_text() {
        return Ok(vec![one.to_owned()]);
    }
    let Some(array) = args[at].array_ptr() else {
        return Err(Fault::fatal(format!(
            "{member} expected a `string` or an `array<string>` for `{TLS_PIN_OPTION}`, got tag {}",
            args[at].tag_byte()
        )));
    };
    let mut pins = Vec::new();
    let mut from = 0_usize;
    loop {
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live allocation, \
                      so it is live for the length of this call, and `from` only \
                      ever advances past a slot this same cursor reported"
        )]
        let (slot, value) = unsafe {
            let slot = nvs_runtime::nvs_array_next_slot(array, from);
            let Ok(slot) = usize::try_from(slot) else {
                break;
            };
            let mut value = Value::null();
            nvs_runtime::nvs_array_value_at(array, slot, &raw mut value);
            (slot, value)
        };
        from = slot + 1;

        let pin = value.as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `string` pin in `{TLS_PIN_OPTION}`, got tag {}",
                value.tag_byte()
            ))
        })?;
        pins.push(pin.to_owned());
    }
    Ok(pins)
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
    if let Some(written) = wait_of(args, at, option)? {
        return Ok(written);
    }
    let configured = ctx
        .config()
        .and_then(|config| config.get(directive))
        .and_then(|text| duration::parse(&text).ok())
        .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()));
    Ok(configured.unwrap_or(fallback))
}

/// The wait the call itself wrote at `at`, or `None` where it wrote none.
///
/// [`bound_of`]'s first step alone, which is the whole of a key that has no
/// directive behind it and no default: a socket's `ping` is off until a program
/// asks for one
/// (`rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`),
/// so an omitted one has nothing to fall through to. Every *bound* still has a
/// value, which is what that rule requires — this is the one key that is a
/// request rather than a bound.
///
/// # Errors
///
/// [`crate::time::nanos_of`]'s fatal for a slot the row's own type rules out.
fn wait_of(args: &[Value], at: usize, option: &str) -> Result<Option<Duration>, Fault> {
    if args
        .get(at)
        .is_some_and(|held| !matches!(held.tag(), Some(Tag::Null)))
    {
        let nanos = crate::time::nanos_of(args, at, option)?;
        return Ok(Some(Duration::from_nanos(nanos.unsigned_abs())));
    }
    Ok(None)
}

/// The octet cap the call wrote at `at`, then the directive, then `fallback` —
/// [`bound_of`]'s three steps for a count rather than for a wait.
///
/// There is nothing to judge on the way through, which is the difference: a
/// `uint` has no negative value and no infinite one, so what a call writes is
/// the cap it gets and zero is a cap of zero rather than a spelling for
/// unbounded — the same word `Core\Codec`'s `maxBytes` is one class over. A
/// directive that will not read as a size falls through to `fallback` for
/// [`bound_of`]'s reason: `nvs.toml` is refused where it is loaded, and a
/// second refusal here would fail a request over a key the operator can no
/// longer see.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is neither a `uint` nor `Tag::Null`,
/// which the row's own type rules out.
fn cap_of(
    ctx: &Ctx,
    args: &[Value],
    at: usize,
    option: &str,
    directive: &str,
    fallback: u64,
) -> Result<u64, Fault> {
    if let Some(written) = args
        .get(at)
        .filter(|held| !matches!(held.tag(), Some(Tag::Null)))
    {
        return written.as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "`{option}` expected {:?}, got tag {}",
                Tag::Int,
                written.tag_byte()
            ))
        });
    }
    let configured = ctx
        .config()
        .and_then(|config| config.get(directive))
        .and_then(|text| {
            let written = nvs_config::Setting::Text(text);
            match nvs_config::Quantity::parse(directive, nvs_config::Unit::Bytes, &written) {
                Ok(nvs_config::Quantity::Bytes(octets)) => Some(octets),
                _ => None,
            }
        });
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

/// The address a call named with `connectTo`, once both grants and the address
/// policy have approved it —
/// `rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`.
///
/// Three questions, and the first is the one the option does not remove: naming
/// an address does not make a host reachable, so `net.connect` is still asked
/// about the host the URL wrote, and the handshake still checks the certificate
/// against that same name. `net.connect_to` is asked about it too, so a
/// deployment says which hosts a call may steer itself at rather than granting
/// the steering everywhere at once. The address then goes through
/// [`nvs_runtime::capability::pinned_addresses`], which is the one home of
/// `rule:security/net-address-policy` and of `net.internal`'s exceptions — the
/// option therefore chooses among addresses the deployment already allows and
/// widens nothing.
///
/// **An IP literal, and a name refused rather than resolved.** A lookup here
/// would be a second resolution reached through the option instead of through
/// the URL, which is exactly what pinning an address exists to remove
/// (`rule:http-server/allow-url-pins-the-address`). A literal reaches no
/// resolver, so the answer is the set of one
/// `rule:http-server/an-outbound-call-tries-every-approved-address` says a
/// named address is.
///
/// # Errors
///
/// [`judged_host`]'s three, a thrown `RuntimeError` naming whichever grant this
/// deployment did not write, a thrown `LogicError` for a value that is not an
/// IP literal, and `pinned_addresses`' refusal for an address the policy denies.
fn named_address(ctx: &Ctx, url: &str, named: &str, member: &str) -> Result<Vec<IpAddr>, Fault> {
    let host = judged_host(url, member, Roster::Request)?;
    nvs_runtime::capability::require(ctx, Cap::NetConnect, Scope::Host(&host), member)?;
    nvs_runtime::capability::require(
        ctx,
        Cap::NetConnectTo,
        Scope::Host(&host),
        &format!("{member}'s `{CONNECT_TO_OPTION}`"),
    )?;
    if named.parse::<IpAddr>().is_err() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{member}: `{CONNECT_TO_OPTION}` is the IP address to connect to, and `{named}` \
                 is not one — a name written here would be resolved a second time, which is what \
                 pinning an address exists to remove"
            ),
        ));
    }
    nvs_runtime::capability::pinned_addresses(ctx, named, member)
}

/// One redirect hop's re-pin, in the shape [`transport::send`] asks for it: the
/// downgrade question first, where the deployment can be asked about it, and
/// then the same approval the first URL passed.
///
/// The transport reports whether this hop leaves TLS behind and decides
/// nothing, because only there are both schemes known, while the `Ctx` and the
/// call's own bag — the two halves
/// `rule:http-server/an-https-redirect-never-becomes-plaintext` needs — are
/// only here.
///
/// # Errors
///
/// [`judge_downgrade`]'s two for a hop down into plaintext, and [`pin`]'s.
fn repinned(
    ctx: &mut Ctx,
    args: &[Value],
    hop: &str,
    downgrade: bool,
    member: &str,
) -> Result<Vec<IpAddr>, Fault> {
    if downgrade {
        judge_downgrade(ctx, args, hop, member)?;
    }
    pin(ctx, hop, member, Roster::Request)
}

/// A hop out of `https` and into `http`, against the call's own word and the
/// deployment's — `rule:http-server/an-https-redirect-never-becomes-plaintext`.
///
/// The call is asked first, and one that wrote nothing is refused without the
/// deployment being consulted at all: the key ships off, so an ordinary program
/// never reaches the grant question, and a deployment that granted a downgrade
/// somewhere still does not hand one to a call that was not expecting it.
///
/// A `RuntimeError` and not [`judge_trust`]'s `LogicError`, because what has
/// happened is a reply this origin chose to send rather than something the
/// program asked for, and a program that degrades instead of insisting is a
/// reasonable program (`rule:security/denial-is-a-runtime-error`).
///
/// # Errors
///
/// [`judged_host`]'s three, a thrown `RuntimeError` for a call that did not
/// write the key, and `require`'s for a hop host outside the grant.
fn judge_downgrade(ctx: &Ctx, args: &[Value], hop: &str, member: &str) -> Result<(), Fault> {
    let host = judged_host(hop, member, Roster::Request)?;
    if args[REDIRECT_TO_HTTP].as_bool() != Some(true) {
        return Err(Fault::thrown(format!(
            "{member}: this `https` call was redirected to `{hop}`, which is plaintext. A \
             downgrade takes both halves — the `net.downgrade` grant for `{host}`, and \
             `{REDIRECT_TO_HTTP_OPTION}: true` at the call, which this one did not write"
        )));
    }
    nvs_runtime::capability::require(
        ctx,
        Cap::NetDowngrade,
        Scope::Host(&host),
        &format!("{member}'s `{REDIRECT_TO_HTTP_OPTION}`"),
    )
}

/// The URL to send to and the set of addresses it was approved at.
///
/// A `Target` argument was pinned by the launderer that built it, and asking
/// again would be the second resolution `rule:http-server/allow-url-pins-the-address` exists to remove — so its
/// two slots are read back here and no name is looked up. A plain `string` is
/// the form § 1 keeps for a URL the program authored, and it goes through the
/// same door — or through [`named_address`], where the call named the address
/// itself.
///
/// # Errors
///
/// [`pin`]'s for a `string`, [`named_address`]'s for
/// a call that wrote `connectTo`, and a thrown `LogicError` where that key sits
/// beside a `Target`, which already carries the address its laundering
/// approved. A
/// [`Fault::fatal`] for an argument of another shape or a target whose slots
/// this crate did not write, both unreachable from source.
fn approved(
    ctx: &mut Ctx,
    args: &[Value],
    member: &str,
    bag: Bag,
) -> Result<(String, Vec<IpAddr>), Fault> {
    let url = given_url(args, member)?;
    let named = args[bag.connect_to].as_text();
    if !matches!(args[0].tag(), Some(Tag::Object)) {
        let addresses = match named {
            Some(named) => named_address(ctx, &url, named, member)?,
            None => pin(ctx, &url, member, bag.roster)?,
        };
        return Ok((url, addresses));
    }

    if let Some(named) = named {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{member}: a `Core\\Http\\Target` already carries the address its laundering \
                 approved, and this call wrote `{CONNECT_TO_OPTION}: \"{named}\"` beside it — the \
                 two are answers to one question, so the call has not said which it meant"
            ),
        ));
    }

    let target = crate::instance::receiver(args[0], &TARGET, member)?;
    let addresses = addresses_of(target).ok_or_else(|| {
        Fault::fatal(format!(
            "{member} found a `Core\\Http\\Target` it cannot read"
        ))
    })?;
    Ok((url, addresses))
}

/// The approved set as [`TARGET_ADDRESSES_SLOT`] holds it: one text per
/// address, in the order the resolver answered them, and none at all for a
/// `Target` laundered where the proxy resolves the destination.
///
/// `None` rather than a shorter set for anything the slot holds that is not an
/// address: this crate is what writes that slot, so a value of another kind
/// would be a mistake in this file, and answering with the entries that did
/// parse would be a call connecting to a set the door never approved. An empty
/// set is not that mistake and is not folded in with it — it is what [`pin`]
/// answers under `resolve = "proxy"`, and telling the two apart is what lets a
/// call refuse a target no proxy will tunnel instead of reading it as
/// unwritable.
fn addresses_of(object: *mut nvs_runtime::ObjHeader) -> Option<Vec<IpAddr>> {
    let held = crate::instance::slot(object, TARGET_ADDRESSES_SLOT).array_ptr()?;
    let held = crate::arr::borrowed(held);
    let mut out = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = held.next_slot(from) {
        from = slot + 1;
        let address = held
            .value_at(slot)
            .expect("next_slot only names live entries");
        out.push(address.as_text()?.parse::<IpAddr>().ok()?);
    }
    Some(out)
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
fn body_of(ctx: &mut Ctx, args: &[Value], member: &str) -> Result<Option<transport::Body>, Fault> {
    if !matches!(args[JSON].tag(), Some(Tag::Unset)) {
        // `Tag::Unset` and not `Tag::Null`: the document `null` is a body a
        // program may mean, which is what [`OPTIONS`]' `json` row states.
        let document = crate::json::written(ctx, args[JSON], member)?;
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
    let fields = fields_of(args, MULTIPART, "multipart", member)?;

    // Every name is checked before any file is opened, so a form that is
    // refused holds no descriptor and reads nothing from the disk.
    for (name, value) in &fields {
        quotable(name, "field name", member)?;
        if matches!(value.tag(), Some(Tag::Object)) {
            let part = crate::instance::receiver(*value, &PART, member)?;
            let filename = crate::instance::slot(part, PART_FILENAME_SLOT);
            let content_type = crate::instance::slot(part, PART_CONTENT_TYPE_SLOT);
            quotable(filename.as_text().unwrap_or(name), "filename", member)?;
            if let Some(content_type) = content_type.as_text() {
                quotable(content_type, "media type", member)?;
            }
        }
    }

    for (name, value) in fields {
        let mut head = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"");
        if matches!(value.tag(), Some(Tag::Object)) {
            let part = part_of(ctx, value, member)?;
            let filename = part.filename.unwrap_or_else(|| name.clone());
            let content_type = part
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned());
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
/// [`pin`]'s, [`judge_bound`]'s, [`judge_attempts`]', [`judge_verb`]'s and
/// [`judge_trust`]'s, and then [`transport::send`]'s.
fn request(ctx: &mut Ctx, args: &[Value], member: &str, verb: &str) -> Result<Value, Fault> {
    let (status, body, headers, tls) = exchanged(ctx, args, member, verb, false)?;
    Ok(crate::instance::build(
        &RESPONSE,
        [Value::int(status), body, headers, tls_info(tls)],
    ))
}

/// The `Core\Http\TlsInfo` a reply reports, or `null` where it arrived over no
/// session — `rule:http-server/a-reply-reports-its-tls-session`.
///
/// The chain is copied into the instance as the DER it already is, and PEM is
/// written at `peerChain` rather than here: a reply that nobody asks about pays
/// for neither encoding, and the slot stays the one thing the leaf readers parse.
fn tls_info(tls: Option<transport::Tls>) -> Value {
    match tls {
        Some(tls) => tls_info_of(&tls.session, tls.verified),
        None => Value::null(),
    }
}

/// A `Core\Http\TlsInfo` over `session`, reporting `verified` — the one place
/// the class's four slots are filled, for a handshake's session and for the
/// one `Core\Test::tlsSession` describes. The caller owns it.
pub(crate) fn tls_info_of(session: &nvs_host::tls::Session, verified: bool) -> Value {
    let mut chain = NvsArray::new();
    for der in session.chain() {
        chain.append(Value::bytes(NvsStr::new(der)));
    }
    crate::instance::build(
        &TLS_INFO,
        [
            Value::str(NvsStr::new(session.version().as_bytes())),
            Value::str(NvsStr::new(session.cipher().as_bytes())),
            Value::bool(verified),
            Value::array(chain),
        ],
    )
}

/// The session a `Core\Http\TlsInfo` holds, copied out as the plain data a
/// test's answer table keeps — `Core\Test::answerHttp`'s `tls` option.
///
/// # Errors
///
/// The fatal for a receiver that is not a `Core\Http\TlsInfo`, which the
/// option's declared type makes unreachable from source.
pub(crate) fn answered_tls(value: Value, member: &str) -> Result<nvs_runtime::AnsweredTls, Fault> {
    let object = crate::instance::receiver(value, &TLS_INFO, member)?;
    let text = |slot: usize| {
        crate::instance::slot(object, slot)
            .as_text()
            .unwrap_or_default()
            .to_owned()
    };
    Ok(nvs_runtime::AnsweredTls {
        version: text(TLS_VERSION_SLOT),
        cipher: text(TLS_CIPHER_SLOT),
        verified: crate::instance::slot(object, TLS_VERIFIED_SLOT)
            .as_bool()
            .unwrap_or(false),
        chain: chain_of(object),
    })
}

/// One certificate as PEM: the base64 of its DER, wrapped at 64 characters
/// between the two `CERTIFICATE` lines.
///
/// RFC 7468 § 2's encoding, which is what `openssl x509` reads and writes and
/// therefore the one form of a certificate a program can hand to anything else.
fn pem_of(der: &[u8]) -> String {
    let mut out = String::from("-----BEGIN CERTIFICATE-----\n");
    let encoded = STANDARD.encode(der);
    for line in encoded.as_bytes().chunks(64) {
        out.push_str(&String::from_utf8_lossy(line));
        out.push('\n');
    }
    out.push_str("-----END CERTIFICATE-----\n");
    out
}

/// The peer's chain as [`TLS_CHAIN_SLOT`] holds it: DER, leaf first, one entry
/// per certificate.
fn chain_of(object: *mut nvs_runtime::ObjHeader) -> Vec<Vec<u8>> {
    let Some(held) = crate::instance::slot(object, TLS_CHAIN_SLOT).array_ptr() else {
        return Vec::new();
    };
    let held = crate::arr::borrowed(held);
    let mut out = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = held.next_slot(from) {
        from = slot + 1;
        let der = held
            .value_at(slot)
            .expect("next_slot only names live entries");
        out.push(der.as_bytes().unwrap_or_default().to_vec());
    }
    out
}

/// What the leaf of `chain` says, for the three members that report a field of
/// it — `nvs_host::tls::leaf` is the parser, and [`TLS_INFO`]'s own doc is why
/// it runs here rather than at the framing.
///
/// # Errors
///
/// A `RuntimeError` where the peer presented no certificate, or a leaf that does
/// not parse as X.509.
fn leaf_of(chain: &[Vec<u8>], member: &str) -> Result<nvs_host::tls::Leaf, Fault> {
    let why = match chain.first() {
        Some(der) => match nvs_host::tls::leaf(der) {
            Some(read) => return Ok(read),
            None => "presented a leaf certificate this cannot read",
        },
        None => "presented no certificate",
    };
    // no case can reach this: every `Core\Http\TlsInfo` a program can hold has
    // a readable leaf, because a handshake's peer presented one and
    // `Core\Test::tlsSession` issues one. `a_leaf_that_is_absent_or_unreadable_is_refused`
    // is what asserts it instead.
    Err(Fault::thrown(format!(
        "{TLS_INFO_NAME}::{member}: the peer {why}, so there is nothing to read this from"
    )))
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
/// The session comes back as the transport's own value rather than as a built
/// instance, because only the buffered half reports one: a streamed reply has no
/// `tls` member, and building a `Core\Http\TlsInfo` for it would allocate an
/// answer nothing can ask for.
///
/// # Errors
///
/// [`approved`]'s refusals, [`judge_bound`]'s, [`judge_attempts`]',
/// [`judge_verb`]'s, [`judge_trust`]'s, [`body_of`]'s and whatever
/// [`transport::send`] raised — or,
/// where a test has armed the answer table, [`faked`]'s.
fn exchanged(
    ctx: &mut Ctx,
    args: &[Value],
    member: &str,
    verb: &str,
    streamed: bool,
) -> Result<(i64, Value, Value, Option<transport::Tls>), Fault> {
    let named = format!("{CLIENT_NAME}::{member}");
    if ctx.faked_http().is_armed() {
        return faked(ctx, args, &named, verb, streamed);
    }
    // The resolve time the event reports is this call's own, and a call handed
    // an already-pinned `Core\Http\Target` spent none of it: that lookup was
    // `allowUrl`'s, on an earlier call, and charging it here would be the
    // second measurement that disagrees with the first.
    let pinned_already = matches!(args[0].tag(), Some(Tag::Object));
    let began = Instant::now();
    let (url, addresses) = approved(ctx, args, &named, REQUEST_BAG)?;
    let resolve = if pinned_already {
        Duration::ZERO
    } else {
        began.elapsed()
    };

    judge_bound(args, DEADLINE, "deadline", &named)?;
    judge_bound(args, CONNECT_TIMEOUT, "connectTimeout", &named)?;
    judge_bound(args, RETRY_BACKOFF, "retryBackoff", &named)?;
    judge_attempts(args, &named)?;
    judge_verb(args, verb, &named)?;
    judge_trust(ctx, args, &url, &named, REQUEST_BAG)?;

    // Framed before the clock below starts: the encode and the open are this
    // end's work, and a budget spent on them is not a budget the other end was
    // given.
    let body = body_of(ctx, args, &named)?;

    let call = transport::Call {
        member: &named,
        verb,
        url,
        addresses,
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
        identity: identity_option(args, &named, REQUEST_BAG)?,
        policy: policy_of(args, &named, REQUEST_BAG)?,
        traceparent: traceparent_of(ctx),
        span: std::cell::RefCell::new(span::HttpSpan::opened(verb, resolve)),
        proxy: proxy_of(ctx),
    };

    // The one difference between the two members, and it is which bounds the
    // body runs under rather than a second reading of it: a buffered call is
    // one deadline over the whole exchange under `REPLY_CEILING`, and a
    // streamed one stops at the head and leaves the body on the socket under
    // `idle` and `maxDuration`
    // (`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`).
    if streamed {
        let answer = transport::send_streamed(&call, &mut |hop, downgrade| {
            repinned(ctx, args, hop, downgrade, &named)
        })?;
        let headers = header_map(&answer.headers);
        let body = filed(ctx, answer.body);
        traced(ctx, &call);
        return Ok((answer.status, body, headers, answer.tls));
    }
    let reply = transport::send(&call, &mut |hop, downgrade| {
        repinned(ctx, args, hop, downgrade, &named)
    })?;
    let headers = header_map(&reply.headers);
    traced(ctx, &call);
    Ok((
        reply.status,
        Value::bytes(NvsStr::new(&reply.body)),
        headers,
        reply.tls,
    ))
}

/// Files this call's `http` trace event, now that there is an answer to file —
/// `rule:observability/trace-events-carry-a-kind`, once per call whatever its
/// attempts and hops.
///
/// **Here and not in [`transport`]**, which is that module's whole shape: it is
/// handed addresses rather than a `Ctx`, and a trace belongs to the request.
/// [`Ctx::records_spans`](nvs_runtime::Ctx::records_spans) is asked here for the
/// same reason, so a request nothing records pays only for the clock reads the
/// span already took — [`span`]'s module doc prices those against a round trip.
///
/// A call the answer table served never reaches this: [`exchanged`] answers from
/// [`faked`] before a span exists, so nothing crossed a network and there is
/// nothing to report. On a streamed reply the event is filed with the head,
/// which is where the call ends and the program's own reading begins.
fn traced(ctx: &mut Ctx, call: &transport::Call<'_>) {
    if !ctx.records_spans() {
        return;
    }
    let line = {
        let mut span = call.span.borrow_mut();
        span.finished();
        span.to_string()
    };
    ctx.record_http(&line);
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
/// `Core\Test\Response` keeps its headers in a slot of this shape and reads
/// them through here too.
///
/// The answer is the list of `string` values the slot already holds, borrowed
/// rather than copied: the reply owns a reference to it for as long as the
/// receiver is live, so a reader that hands a line back retains that line and
/// a reader that only looks at one pays nothing.
///
/// The lookup lower-cases what the caller wrote and nothing else: the slot's
/// keys were lower-cased where they were parsed, so the comparison RFC 9110
/// § 5.1 asks for is one lookup rather than a walk that compares
/// case-insensitively at every entry, and a name written in lower case costs
/// no allocation at all.
pub(crate) fn field_lines(
    object: *mut nvs_runtime::ObjHeader,
    at: usize,
    name: &str,
) -> Option<std::mem::ManuallyDrop<NvsArray>> {
    let map = crate::arr::borrowed(crate::instance::slot(object, at).array_ptr()?);
    let key: std::borrow::Cow<'_, str> = if name.bytes().any(|byte| byte.is_ascii_uppercase()) {
        std::borrow::Cow::Owned(name.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(name)
    };
    Some(crate::arr::borrowed(map.get(key.as_bytes())?.array_ptr()?))
}

/// The lines of `values` in arrival order, each borrowed from the list.
pub(crate) fn each_line(values: &NvsArray) -> impl Iterator<Item = Value> + '_ {
    let mut from = 0_usize;
    std::iter::from_fn(move || {
        let slot = values.next_slot(from)?;
        from = slot + 1;
        Some(
            values
                .value_at(slot)
                .expect("next_slot only names live entries"),
        )
    })
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
pub(crate) fn joined_field(
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
    let Some(values) = field_lines(object, at, name) else {
        return Ok(Value::null());
    };
    // A field that arrived once is the whole answer, and by far the common
    // one, so it is handed back shared rather than copied.
    if values.count() == 1 {
        let line = each_line(&values).next().expect("a list of one has a line");
        #[expect(
            unsafe_code,
            reason = "the receiver owns a reference for the length of the call, so the \
                      line its header map holds is live, which is `Value::retain`'s whole \
                      obligation"
        )]
        unsafe {
            line.retain();
        }
        return Ok(line);
    }
    let mut joined = Vec::new();
    for (at, line) in each_line(&values).enumerate() {
        if at > 0 {
            joined.extend_from_slice(b", ");
        }
        joined.extend_from_slice(line.as_text().unwrap_or_default().as_bytes());
    }
    Ok(Value::str(NvsStr::new(&joined)))
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
    if let Some(values) = field_lines(object, at, name) {
        for line in each_line(&values) {
            // The list handed back owns one reference to each line, and the
            // reply keeps its own.
            #[expect(
                unsafe_code,
                reason = "the receiver owns a reference for the length of the call, so \
                          the line its header map holds is live, which is \
                          `Value::retain`'s whole obligation"
            )]
            unsafe {
                line.retain();
            }
            out.append(line);
        }
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
) -> Result<(i64, Value, Value, Option<transport::Tls>), Fault> {
    let url = given_url(args, named)?;
    judged_host(&url, named, Roster::Request)?;
    judge_bound(args, DEADLINE, "deadline", named)?;
    judge_bound(args, CONNECT_TIMEOUT, "connectTimeout", named)?;
    judge_bound(args, RETRY_BACKOFF, "retryBackoff", named)?;
    judge_attempts(args, named)?;
    judge_verb(args, verb, named)?;

    let mut headers: Vec<(String, String)> = headers_of(args, HEADERS, named)?
        .into_iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value))
        .collect();
    // A header a real call refuses before it connects is refused here before
    // it is recorded, so a test sees the same error and no record of a call
    // production never makes.
    for (name, value) in &headers {
        transport::judged_field(name, value, named)?;
    }
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
    // The session the row was registered with, if any, as a fresh copy per
    // reply: `Core\Test::tlsSession` described it and nothing negotiated it
    // (`rule:testing/an-outbound-call-is-answered-from-a-table`).
    let tls = answer.tls.as_ref().map(|held| transport::Tls {
        session: nvs_host::tls::Session::recorded(
            held.version.clone(),
            held.cipher.clone(),
            held.chain.clone(),
        ),
        verified: held.verified,
    });
    // A streamed call is answered through a reader here too, over octets that
    // are all present already ([`transport::Incoming::already`]): the walks
    // have one way to frame a body, and a test that armed the table is walking
    // the same code a call to an origin does.
    let body = if streamed {
        filed(ctx, transport::Incoming::already(octets, named))
    } else {
        Value::bytes(NvsStr::new(&octets))
    };
    Ok((status, body, headers, tls))
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

/// The forward proxy this call leaves through, or `None` where the operator
/// wrote no `[http.client.proxy]` block — which is how a deployment asks for no
/// proxy (`rule:http-server/an-outbound-proxy-is-operator-configured`).
///
/// No option slot and no `Result`, for [`pool_of`]'s two reasons: the block is
/// `System`-class, so a request has no key to move where its bytes go, and a
/// `url` this client cannot dial was refused where the configuration was loaded.
/// A block that reached here with one anyway is read as no proxy rather than as
/// a failed call, which is the safe direction only because the boot cannot let
/// one through: `nvs_config::http::proxy_endpoint` is the same grammar both
/// sides ask.
fn proxy_of(ctx: &Ctx) -> Option<transport::Proxy> {
    let written = nvs_config::http::written_proxy(&ctx.config()?.snapshot().config)?;
    let url = written.url.as_deref()?;
    let (host, port) = nvs_config::http::proxy_endpoint(url).ok()?;
    Some(transport::Proxy {
        url: url.to_owned(),
        host: host.to_owned(),
        port,
        by_name: written.resolve.as_deref() == Some(nvs_config::http::RESOLVE_AT_THE_PROXY),
        bypass: written.bypass.clone().unwrap_or_default(),
        authorization: written.username.as_deref().map(|user| {
            // The password arrives already materialized, whether the operator
            // wrote it inline or named the file whose content it is
            // (`rule:config/a-secret-is-a-file-whose-content-is-the-value`), and
            // a block with a user and no password is the empty half of the pair
            // rather than a refusal — `Basic` has a spelling for it and the
            // proxy is the end that decides.
            let password = written.password.as_deref().unwrap_or_default();
            format!("Basic {}", STANDARD.encode(format!("{user}:{password}")))
        }),
    })
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

nvs_runtime::nvs_helper! {
    /// `Core\Http\Response::tls(): ?Core\Http\TlsInfo` — `rule:http-server/a-reply-reports-its-tls-session`.
    ///
    /// The slot holds the whole answer, built where the reply was framed: the
    /// connection it was negotiated on has gone back to the pool by now and may
    /// be serving somebody else's request, so nothing here asks the transport
    /// anything.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_response_status`]'s fatal for a receiver that is not a
    /// `Core\Http\Response`, unreachable from source for that member's reason.
    fn nvs_core_http_response_tls(_ctx, args: [1]) {
        crate::instance::read_slot(args, &RESPONSE, TLS_SLOT, "tls")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\TlsInfo::version(): string` — `rule:http-server/a-reply-reports-its-tls-session`.
    ///
    /// Not `tainted`, and neither is [`nvs_core_http_tls_info_cipher`]: what a
    /// handshake settled on is a value out of a closed set this end agreed to,
    /// not bytes the other end wrote — `rule:security/tainted-sources` draws
    /// that line at a fixed enum-shaped field.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_response_status`]'s fatal for a wrongly-tagged receiver.
    fn nvs_core_http_tls_info_version(_ctx, args: [1]) {
        crate::instance::read_slot(args, &TLS_INFO, TLS_VERSION_SLOT, "version")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\TlsInfo::cipher(): string` — the suite, under the IANA name
    /// `nvs_host::tls` writes it in.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_tls_info_version`]'s.
    fn nvs_core_http_tls_info_cipher(_ctx, args: [1]) {
        crate::instance::read_slot(args, &TLS_INFO, TLS_CIPHER_SLOT, "cipher")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\TlsInfo::verified(): bool` — whether the chain *and* the name
    /// were both checked, which is the member
    /// `rule:http-server/a-reply-reports-its-tls-session` exists for.
    ///
    /// The slot was filled from `CallPolicy::verifies`, which owns what each of
    /// the relaxing options means; reading those five fields a second time here
    /// is the copy that comes to disagree with it.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_tls_info_version`]'s.
    fn nvs_core_http_tls_info_verified(_ctx, args: [1]) {
        crate::instance::read_slot(args, &TLS_INFO, TLS_VERIFIED_SLOT, "verified")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\TlsInfo::peerChain(): array<tainted string>` — the peer's
    /// certificates as PEM, leaf first.
    ///
    /// The slot holds DER and this writes PEM, per ADR 0180 § 13: DER is what
    /// `rustls` hands over, and PEM is the form a program can paste into
    /// anything else.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_tls_info_version`]'s. A peer that presented no
    /// certificate answers an empty array rather than throwing — the three
    /// readers that have to open one are where that becomes a refusal.
    fn nvs_core_http_tls_info_peer_chain(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &TLS_INFO, "peerChain")?;
        let mut out = NvsArray::new();
        for der in chain_of(object) {
            out.append(Value::str(NvsStr::new(pem_of(&der).as_bytes())));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\TlsInfo::subject(): tainted string` — who the leaf says it is.
    ///
    /// `tainted` where [`nvs_core_http_tls_info_version`] is not: a name inside
    /// a certificate is text the other end chose, and a session that verified
    /// the chain settled which host answered rather than what is written in it.
    ///
    /// # Errors
    ///
    /// [`leaf_of`]'s `RuntimeError`, and the fatal for a wrongly-tagged
    /// receiver.
    fn nvs_core_http_tls_info_subject(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &TLS_INFO, "subject")?;
        let read = leaf_of(&chain_of(object), "subject")?;
        Ok(Value::str(NvsStr::new(read.subject().as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\TlsInfo::issuer(): tainted string` — who signed the leaf.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_tls_info_subject`]'s two.
    fn nvs_core_http_tls_info_issuer(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &TLS_INFO, "issuer")?;
        let read = leaf_of(&chain_of(object), "issuer")?;
        Ok(Value::str(NvsStr::new(read.issuer().as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Http\TlsInfo::expiry(): Core\Time\Instant` — the leaf's `notAfter`.
    ///
    /// An `Instant` and not the text the certificate holds, because the question
    /// a program asks of this is how long is left, which is arithmetic
    /// `Core\Time` already owns.
    ///
    /// # Errors
    ///
    /// [`nvs_core_http_tls_info_subject`]'s two.
    fn nvs_core_http_tls_info_expiry(_ctx, args: [1]) {
        let object = crate::instance::receiver(args[0], &TLS_INFO, "expiry")?;
        let read = leaf_of(&chain_of(object), "expiry")?;
        // A `notAfter` past `Core\Time\Instant`'s last instant, such as RFC
        // 5280's `99991231235959Z` for a certificate with no end, is this fatal:
        // the module doc's first known gap.
        crate::time::instant_at_system_time(read.expiry()).ok_or_else(|| {
            Fault::fatal(format!(
                "{TLS_INFO_NAME}::expiry read a `notAfter` no `Core\\Time\\Instant` names"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::ErrorKind;
    use std::net::{IpAddr, Ipv4Addr, TcpListener};
    use std::rc::Rc;

    use nvs_host::blocking::pool_size;
    use nvs_host::reactor::install;
    use nvs_host::{Reactor, Scheduler, run_until_idle};
    use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, TaskRoot, ThrownClass, Value};

    use crate::tests::granting;

    use super::{
        BODY_OPTION, BODY_OPTIONS, BODY_SLOT, CONNECT_TIMEOUT, CONNECT_TO, CONNECT_TO_OPTION,
        CONTENT_TYPE_OPTION, DEADLINE, FOLLOW_REDIRECTS, FORM_OPTION, HEADERS, HEADERS_SLOT,
        IDENTITY_AT, IDENTITY_OPTION, IDLE, JSON, JSON_OPTION, MAX_DURATION, MULTIPART_OPTION,
        OPTIONS, REDIRECT_TO_HTTP, REDIRECT_TO_HTTP_OPTION, REQUEST_ARITY, REQUEST_BAG, RESPONSE,
        RETRY_ATTEMPTS, RETRY_ATTEMPTS_OPTION, RETRY_BACKOFF, RETRY_KEY, RETRY_KEY_OPTION,
        SOCKET_ARITY, SOCKET_BAG, SOCKET_CONNECT_TIMEOUT, SOCKET_DEADLINE, SOCKET_HEADERS,
        SOCKET_IDLE, SOCKET_MAX_DURATION, SOCKET_MAX_MESSAGE, SOCKET_OPTIONS, SOCKET_PING,
        SOCKET_PROTOCOLS, SOCKET_SEND_TIMEOUT, STATUS_SLOT, STREAM_ARITY, STREAM_OPTIONS, TARGET,
        TARGET_ADDRESSES_SLOT, TARGET_URL_SLOT, TLS_CA_OPTION, TLS_MIN_VERSION_OPTION,
        TLS_PIN_OPTION, TLS_VERIFY_HOST_OPTION, TLS_VERIFY_OPTION,
    };

    /// The grant every case here starts from: the host is reachable and no
    /// address is excepted, which is § 3's table exactly as it ships.
    const GRANTED: &str = "[capabilities.net]\nconnect = [\"127.0.0.1\"]\n";

    /// A loopback origin that answers `replies` in order, one connection each,
    /// and hands back how many it got through.
    ///
    /// Every reply a case writes here closes its connection, which is what makes
    /// one thread enough: a pooled connection would leave this loop waiting on
    /// an accept the client is never going to make, and a case asserting on a
    /// count would hang rather than fail.
    ///
    /// A `std` listener for the transport's own origin's reason: what these
    /// cases drive is the member above it, and the address it hands down has
    /// already been through the door.
    fn origin(
        replies: Vec<&'static str>,
    ) -> (std::net::SocketAddr, std::thread::JoinHandle<usize>) {
        use std::io::{Read, Write};

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let served = std::thread::spawn(move || {
            let mut answered = 0;
            let mut buffer = [0_u8; 4096];
            for reply in replies {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut head = Vec::new();
                while !head.windows(4).any(|end| end == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => head.extend_from_slice(&buffer[..read]),
                    }
                }
                if head.is_empty() || stream.write_all(reply.as_bytes()).is_err() {
                    break;
                }
                stream.flush().ok();
                answered += 1;
                // Whatever the client is still sending is read before this end
                // closes: a request body that arrives after the reply went out
                // is a reset in the client's face on Windows if it is left in
                // the receive buffer, and the case would fail as an `IOError`
                // on the read rather than on what it asserts.
                while matches!(stream.read(&mut buffer), Ok(read) if read > 0) {}
            }
            answered
        });
        (at, served)
    }

    /// The request every case below writes: the URL, an empty header array —
    /// which is what the compiler passes for a bag key nobody wrote — and the
    /// `json` slot said to have been left out rather than written as the
    /// document `null`.
    ///
    /// The array is the caller's to release, along with the URL it was handed.
    fn asking(url: Value) -> [Value; REQUEST_ARITY] {
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = url;
        args[HEADERS] = Value::array(NvsArray::new());
        args[JSON] = Value::unset();
        args
    }

    /// The one event a call files, and § 15's field set inside it: what was
    /// asked, of whom, what came back, how many tries it took and where the
    /// time went.
    ///
    /// Asserted field by field rather than against a whole line, because the
    /// durations are the one part no case can predict — what is pinned is that
    /// every field is there and says what this exchange did.
    #[test]
    fn an_outbound_call_files_one_http_trace_event_with_its_timings() {
        let (at, served) = origin(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        ]);

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        ctx.set_debug_flags(nvs_runtime::DebugFlags::TRACE);

        let url = Value::str(NvsStr::new(format!("http://{at}/ok").as_bytes()));
        let args = asking(url);
        let answer = super::request(&mut ctx, &args, "get", "GET").expect("the origin's answer");
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced and the one \
                      the member answered with, and neither is its caller's"
        )]
        unsafe {
            url.release();
            args[HEADERS].release();
            answer.release();
        }
        assert_eq!(served.join().expect("the origin thread"), 1);

        let trace = ctx.trace();
        assert_eq!(trace.len(), 1, "one event per call: {trace:?}");
        assert_eq!(trace[0].kind, nvs_runtime::TraceKind::Http);
        let line = &trace[0].callee;
        let host = format!("host={}", at.ip());
        let port = format!("port={}", at.port());
        let address = format!("address={at}");
        for field in [
            "http ",
            "method=GET",
            "scheme=http",
            host.as_str(),
            port.as_str(),
            "path=/ok",
            "status=200",
            "attempts=1",
            "hops=0",
            address.as_str(),
            "resolve=",
            "connect=",
            "tls=",
            "first_byte=",
            "took=",
        ] {
            assert!(line.contains(field), "`{field}` is missing from `{line}`");
        }
    }

    /// The rule's *one event carrying its attempt count*: a call the origin
    /// refused once is one line saying it took two tries, and not two lines.
    ///
    /// Both halves matter. Two events would make a retried call look like two
    /// outbound calls to every consumer downstream, and an event reporting
    /// `attempts=1` would hide the retry altogether.
    #[test]
    fn a_retried_call_is_one_http_event_carrying_its_attempt_count() {
        let (at, served) = origin(vec![
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        ]);

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        ctx.set_debug_flags(nvs_runtime::DebugFlags::TRACE);

        let url = Value::str(NvsStr::new(format!("http://{at}/ok").as_bytes()));
        let mut args = asking(url);
        args[RETRY_ATTEMPTS] = Value::uint(2);
        let answer =
            super::request(&mut ctx, &args, "get", "GET").expect("the second attempt's answer");
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced and the one \
                      the member answered with, and neither is its caller's"
        )]
        unsafe {
            url.release();
            args[HEADERS].release();
            answer.release();
        }
        assert_eq!(served.join().expect("the origin thread"), 2);

        let trace = ctx.trace();
        assert_eq!(trace.len(), 1, "a retry is one event, not one per attempt");
        let line = &trace[0].callee;
        assert!(line.contains("attempts=2"), "{line}");
        assert!(line.contains("status=200"), "{line}");
    }

    /// § 15's exclusion, over a call carrying one of each: a query string, a
    /// header value and a body. None of the three reaches the event, and the
    /// path does — a line that dropped the path with the query would pass an
    /// assertion written only on the secrets.
    ///
    /// The structural half is `super::span`'s: the span is never handed the
    /// call, so the header and the body are not in scope for it, and the query
    /// is cut by the type that renders rather than by this caller.
    #[test]
    fn the_http_trace_event_carries_no_query_string_header_value_or_body() {
        let (at, served) = origin(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        ]);

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        ctx.set_debug_flags(nvs_runtime::DebugFlags::TRACE);

        let url = Value::str(NvsStr::new(
            format!("http://{at}/things?token=sekrit").as_bytes(),
        ));
        let mut written = NvsArray::new();
        written.set(
            NvsStr::new(b"x-api-key"),
            Value::str(NvsStr::new(b"hunter2")),
        );
        let headers = Value::array(written);
        let body = Value::str(NvsStr::new(b"parcel-of-secrets"));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = url;
        args[HEADERS] = headers;
        args[JSON] = body;
        let answer = super::request(&mut ctx, &args, "post", "POST").expect("the origin's answer");
        #[expect(
            unsafe_code,
            reason = "this frame owns the four references it just produced, and a \
                      native member never releases an argument its caller still owns"
        )]
        unsafe {
            url.release();
            headers.release();
            body.release();
            answer.release();
        }
        assert_eq!(served.join().expect("the origin thread"), 1);

        let line = &ctx.trace()[0].callee;
        assert!(line.contains("path=/things"), "{line}");
        for secret in [
            "sekrit",
            "token",
            "x-api-key",
            "hunter2",
            "parcel-of-secrets",
        ] {
            assert!(
                !line.contains(secret),
                "`{secret}` reached the trace: {line}"
            );
        }
    }

    /// § 15's last sentence: a call the table answered files no event, because
    /// nothing crossed a network — there is no address to name and no time to
    /// report, and an event of zeroes would read as a call that was made.
    #[test]
    fn a_call_the_table_answered_files_no_http_event() {
        const URL: &str = "http://127.0.0.1:8099/ok";

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(GRANTED));
        ctx.set_debug_flags(nvs_runtime::DebugFlags::TRACE);
        ctx.faked_http_mut().answer(nvs_runtime::HttpAnswer {
            url: URL.to_owned(),
            status: 200,
            headers: Vec::new(),
            body: b"ok".to_vec(),
            tls: None,
        });

        let url = Value::str(NvsStr::new(URL.as_bytes()));
        let args = asking(url);
        let answer = super::request(&mut ctx, &args, "get", "GET").expect("the table's answer");
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced and the one \
                      the member answered with, and neither is its caller's"
        )]
        unsafe {
            url.release();
            args[HEADERS].release();
            answer.release();
        }

        assert!(
            ctx.trace().is_empty(),
            "a call that never left the process filed one: {:?}",
            ctx.trace()
        );
    }

    /// [`TARGET`]'s layout, asserted for [`RESPONSE`]'s reason and one more:
    /// these two slots are written by the launderer and read back by the
    /// client, so a swapped pair would connect to a URL and pin an address,
    /// which is the one mistake here that still runs.
    #[test]
    fn a_targets_slot_constants_are_the_names_it_declares() {
        assert_eq!(TARGET_URL_SLOT, TARGET.slot("url"));
        assert_eq!(TARGET_ADDRESSES_SLOT, TARGET.slot("addresses"));
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
            (IDENTITY_AT, IDENTITY_OPTION),
            (REQUEST_BAG.ca(), TLS_CA_OPTION),
            (REQUEST_BAG.pin(), TLS_PIN_OPTION),
            (REQUEST_BAG.verify_host(), TLS_VERIFY_HOST_OPTION),
            (REQUEST_BAG.verify(), TLS_VERIFY_OPTION),
            (REQUEST_BAG.min_version(), TLS_MIN_VERSION_OPTION),
            (CONNECT_TO, CONNECT_TO_OPTION),
            (REDIRECT_TO_HTTP, REDIRECT_TO_HTTP_OPTION),
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

    /// The socket bag's flattening, asserted the same way and separately: it
    /// shares every key here with the request bag and puts none of them at the
    /// same number, so a reading handed the wrong [`Bag`] reads the key beside
    /// the one it meant and this is what says so.
    #[test]
    fn every_socket_slot_is_its_position_in_the_socket_bag() {
        for (slot, name) in [
            (SOCKET_DEADLINE, "deadline"),
            (SOCKET_CONNECT_TIMEOUT, "connectTimeout"),
            (SOCKET_HEADERS, "headers"),
            (SOCKET_BAG.identity, IDENTITY_OPTION),
            (SOCKET_BAG.ca(), TLS_CA_OPTION),
            (SOCKET_BAG.pin(), TLS_PIN_OPTION),
            (SOCKET_BAG.verify_host(), TLS_VERIFY_HOST_OPTION),
            (SOCKET_BAG.verify(), TLS_VERIFY_OPTION),
            (SOCKET_BAG.min_version(), TLS_MIN_VERSION_OPTION),
            (SOCKET_BAG.connect_to, CONNECT_TO_OPTION),
            (SOCKET_PROTOCOLS, "protocols"),
            (SOCKET_IDLE, "idle"),
            (SOCKET_MAX_DURATION, "maxDuration"),
            (SOCKET_MAX_MESSAGE, "maxMessage"),
            (SOCKET_SEND_TIMEOUT, "sendTimeout"),
            (SOCKET_PING, "ping"),
        ] {
            assert_eq!(SOCKET_OPTIONS[slot - 1].name, name, "slot {slot}");
        }
        assert_eq!(SOCKET_ARITY, SOCKET_OPTIONS.len() + 1);
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
    /// The launderer and `nvs_runtime::capability::pin_host_addresses` are asked
    /// about the same host on the same two deployments, and what is pinned is that they
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
        let by_client = super::pin(&mut denied, URL, MEMBER, super::Roster::Request)
            .expect_err("loopback is the first range § 3 denies");
        let by_door = nvs_runtime::capability::pin_host_addresses(&denied, HOST, MEMBER)
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
        let pinned = super::pin(&mut excepted, URL, MEMBER, super::Roster::Request)
            .expect("an address the deployment bought back");
        assert_eq!(pinned, vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]);
        assert_eq!(
            nvs_runtime::capability::pin_host_addresses(&excepted, HOST, MEMBER)
                .expect("the door approves it too, or the two had drifted"),
            pinned
        );

        // The grant side is the capability's as well, and asked first: a context
        // that configures nothing refuses on `net.connect` without ever looking
        // at an address, so this member is not a resolver for names it may not
        // reach.
        let mut ungranted = Ctx::buffered();
        let refused = super::pin(&mut ungranted, URL, MEMBER, super::Roster::Request)
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

    /// `Core\Http::methodName` driven the way compiled code drives it: every
    /// case of `Core\Http\Method` crosses as its ordinal and comes back as the
    /// wire token written out here, so the test is the second spelling the
    /// member itself does not keep.
    // covers: Core\Http::methodName
    #[test]
    fn method_name_spells_every_case_as_its_wire_token() {
        const TOKENS: [(&str, &str); 8] = [
            ("Get", "GET"),
            ("Head", "HEAD"),
            ("Options", "OPTIONS"),
            ("Trace", "TRACE"),
            ("Post", "POST"),
            ("Put", "PUT"),
            ("Patch", "PATCH"),
            ("Delete", "DELETE"),
        ];
        assert_eq!(
            crate::router::METHOD.cases.len(),
            TOKENS.len(),
            "a case added to the roster is a token this test has to name"
        );
        let mut ctx = Ctx::buffered();
        for (case, token) in TOKENS {
            let ordinal = crate::router::method_case(case).expect("a case the roster names");
            let name = nvs_runtime::call(
                super::nvs_core_http_method_name,
                &mut ctx,
                &[Value::int(ordinal)],
            )
            .expect("every case has a name");
            assert_eq!(name.as_text(), Some(token), "`{case}`");
            #[expect(
                unsafe_code,
                reason = "this frame owns the one reference the member answered with"
            )]
            unsafe {
                name.release();
            }
        }
    }

    /// `Core\Http::allowUrl` driven the way compiled code drives it. A granted
    /// public address answers a `Core\Http\Target` holding the URL exactly as
    /// written and the one address it was approved at. A URL whose authority
    /// has an empty host part is thrown with the no-host sentence before the
    /// capability is asked, so the refusal never names a grant for a host
    /// called ``.
    // covers: Core\Http::allowUrl
    #[test]
    fn allow_url_pins_a_granted_address_and_refuses_an_empty_host() {
        const URL: &str = "https://203.0.113.10/hook";
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(
            "[capabilities.net]\nconnect = [\"203.0.113.10\"]\n",
        ));

        let url = Value::str(NvsStr::new(URL.as_bytes()));
        let target = nvs_runtime::call(super::nvs_core_http_allow_url, &mut ctx, &[url])
            .expect("a granted public address is approved");
        let object = target.obj_ptr().expect("the answer is an instance");
        assert_eq!(
            crate::instance::slot(object, TARGET_URL_SLOT).as_text(),
            Some(URL),
            "the URL is kept as the program wrote it"
        );
        assert_eq!(
            super::addresses_of(object),
            Some(vec![IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))]),
            "and the address it was approved at is the one it carries"
        );
        #[expect(
            unsafe_code,
            reason = "this frame owns the reference `NvsStr::new` produced and the \
                      target the member answered with, and neither is its caller's"
        )]
        unsafe {
            url.release();
            target.release();
        }

        for empty in ["http:///hook", "http://:8080/hook", "https://@/hook"] {
            let url = Value::str(NvsStr::new(empty.as_bytes()));
            let refused = nvs_runtime::call(super::nvs_core_http_allow_url, &mut ctx, &[url]);
            #[expect(
                unsafe_code,
                reason = "this frame owns exactly the reference `NvsStr::new` just \
                          produced, and a native member never releases an argument \
                          its caller still owns"
            )]
            unsafe {
                url.release();
            }
            assert!(refused.is_err(), "`{empty}` was approved");
            let message = ctx.take_pending().expect("the refusal is a thrown error");
            assert!(
                message.contains("names no host") && !message.contains("net.connect"),
                "`{empty}` is refused for its missing host, not by the capability: {message}"
            );
        }
    }

    /// The address [`unhurried`] answers with: TEST-NET-3, which § 3's table
    /// does not deny and which no machine a case can reach answers on.
    const UNHURRIED: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9));

    /// A resolver that takes its time, and takes it **off the core**: the sleep
    /// sits inside `nvs_host::blocking::run`, which is where a worker's own
    /// `resolve_off_core` puts the lookup it wraps.
    ///
    /// The interval is what the case is named for and not what it asserts — the
    /// order below holds for any of it, and a longer one would only make a
    /// passing run slower.
    fn unhurried(_host: &str) -> std::io::Result<Vec<IpAddr>> {
        nvs_host::blocking::run(|| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(vec![UNHURRIED])
        })
    }

    /// `rule:http-server/a-core-is-never-blocked-on-a-syscall` over the one call
    /// the launderer makes that can wait on the network without a readiness to
    /// park on: the name is resolved on the blocking pool, so the core is handed
    /// back for the interval the resolver spends there.
    ///
    /// The neighbour is what makes that an assertion rather than a hope. A
    /// lookup that held the core would pin the same address and finish both
    /// tasks — it would only be slower, and nothing about the pin's own answer
    /// can tell the two apart. What can is the *order*: the neighbour is spawned
    /// second and must run first, which happens only if the pin suspended.
    /// `pool_size` is the other half, read on a thread that has started no pool
    /// thread until this case makes it — a run where it is still zero is a run
    /// where the lookup never left this thread.
    ///
    /// The resolver reaches `super::pin` through the per-thread seam
    /// `nvs_runtime::capability::install_resolver` owns, which is the same one
    /// `nvs_host::Worker::spawn` installs the real lookup into.
    #[test]
    fn a_slow_lookup_leaves_the_core_free_for_another_task() {
        const HOST: &str = "slow.test";
        const MEMBER: &str = "Core\\Http::allowUrl";

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        assert_eq!(
            pool_size().0,
            0,
            "this thread's pool had already started threads, so the count below proves nothing"
        );
        nvs_runtime::capability::install_resolver(unhurried);

        // The outcome is carried out as text rather than asserted inside the
        // task: a panic at a task root is contained by the scheduler and
        // reported through it, so an `expect` in there would fail quietly.
        let order = Rc::new(RefCell::new(Vec::new()));
        let looking = Rc::clone(&order);
        let mut granted = Ctx::buffered();
        granted.set_config(granting(&format!(
            "[capabilities.net]\nconnect = [\"{HOST}\"]\n"
        )));
        sched.spawn(granted, TaskRoot::Worker, move |ctx| {
            let answer = match super::pin(
                ctx,
                &format!("https://{HOST}/ok"),
                MEMBER,
                super::Roster::Request,
            ) {
                Ok(approved) => format!("pinned {approved:?}"),
                Err(fault) => format!("{fault:?}"),
            };
            looking.borrow_mut().push(answer);
        });
        let neighbour = Rc::clone(&order);
        sched.spawn(Ctx::buffered(), TaskRoot::Worker, move |_ctx| {
            neighbour.borrow_mut().push("neighbour".to_owned());
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 2, "a task never came back off the pool");
        assert_eq!(
            *order.borrow(),
            vec![
                "neighbour".to_owned(),
                format!("pinned {:?}", vec![UNHURRIED])
            ],
            "the core was held for the whole lookup, or the pin refused the host"
        );
        assert!(
            pool_size().0 > 0,
            "the lookup ran on this thread instead of on the pool"
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

    /// What a deployment that may reach loopback grants, and nothing about
    /// trust: the address side of every relaxing test below is already settled,
    /// so what those tests move is the `[capabilities.tls]` half alone.
    const REACHABLE: &str =
        "[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n";

    /// One exchange with a loopback origin that answers `reply`: the request
    /// head the origin read, the status and the body the member answered with.
    ///
    /// The head is read off the wire, so a test asserts the verb that was sent
    /// rather than the name of the member that sent it.
    fn exchanged_once(
        member: &str,
        verb: &str,
        path: &str,
        reply: &'static [u8],
    ) -> (String, Option<i64>, Vec<u8>) {
        let (head, answer) = answered_once(member, verb, path, reply);
        let object = answer.obj_ptr().expect("the answer is an instance");
        let status = crate::instance::slot(object, STATUS_SLOT).as_int();
        let body = crate::instance::slot(object, BODY_SLOT)
            .as_bytes()
            .expect("the body slot is `bytes`")
            .to_vec();
        #[expect(
            unsafe_code,
            reason = "this frame owns the reference the member answered with, and it is \
                      not its caller's"
        )]
        unsafe {
            answer.release();
        }
        (head, status, body)
    }

    /// One exchange with a loopback origin that answers `reply`: the request
    /// head the origin read, and the `Core\Http\Response` the member answered
    /// with, which the caller owns and releases.
    fn answered_once(
        member: &str,
        verb: &str,
        path: &str,
        reply: &'static [u8],
    ) -> (String, Value) {
        use std::io::{Read, Write};

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client's connection");
            let mut head = Vec::new();
            let mut buffer = [0_u8; 4096];
            while !head.windows(4).any(|end| end == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => head.extend_from_slice(&buffer[..read]),
                }
            }
            stream.write_all(reply).expect("the reply is written");
            stream.flush().ok();
            String::from_utf8_lossy(&head).into_owned()
        });

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        let url = Value::str(NvsStr::new(format!("http://{at}{path}").as_bytes()));
        let args = asking(url);
        let answer = super::request(&mut ctx, &args, member, verb).expect("the origin's answer");
        let head = served.join().expect("the origin thread");
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced, and neither is its \
                      caller's"
        )]
        unsafe {
            url.release();
            args[HEADERS].release();
        }
        (head, answer)
    }

    /// `Core\Http\Client::delete` writes `DELETE` on the request line with the
    /// URL's own path, and a `204` with no body answers a response whose status
    /// is that code and whose body is empty rather than absent.
    // covers: Core\Http\Client::delete
    #[test]
    fn delete_sends_the_delete_verb_and_reads_a_204_with_no_body() {
        let (head, status, body) = exchanged_once(
            "delete",
            "DELETE",
            "/orders/3",
            b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n",
        );
        assert!(
            head.starts_with("DELETE /orders/3 HTTP/1.1\r\n"),
            "the request line carries the verb and the path: {head:?}"
        );
        assert_eq!(status, Some(204));
        assert!(body.is_empty(), "a reply with no body is an empty body");
    }

    /// `Core\Http\Client::get` writes `GET` with the path and the query string
    /// the URL carries, and reads the body the origin framed with a length.
    // covers: Core\Http\Client::get
    #[test]
    fn get_sends_the_path_and_query_and_reads_the_whole_body() {
        let (head, status, body) = exchanged_once(
            "get",
            "GET",
            "/rates?base=EUR",
            b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"USD\":1.1}",
        );
        assert!(
            head.starts_with("GET /rates?base=EUR HTTP/1.1\r\n"),
            "the request line carries the verb, the path and the query: {head:?}"
        );
        assert_eq!(status, Some(200));
        assert_eq!(body, b"{\"USD\":1.1}");
    }

    /// `Core\Http\Client::head` writes `HEAD`, and a `Content-Length` on the
    /// reply describes a body that is never sent: the response's body is
    /// empty, and reading it does not wait for bytes that will not come.
    // covers: Core\Http\Client::head
    #[test]
    fn head_sends_the_head_verb_and_reads_no_body_whatever_the_length_says() {
        let (head, status, body) = exchanged_once(
            "head",
            "HEAD",
            "/report.pdf",
            b"HTTP/1.1 200 OK\r\nContent-Length: 52000\r\nConnection: close\r\n\r\n",
        );
        assert!(
            head.starts_with("HEAD /report.pdf HTTP/1.1\r\n"),
            "the request line carries the verb and the path: {head:?}"
        );
        assert_eq!(status, Some(200));
        assert!(body.is_empty(), "a `HEAD` reply has no body");
    }

    /// A loopback origin that answers `replies` in order, one connection each,
    /// and hands back every request it read, head and body, as text.
    ///
    /// The body is read to the length the head declares before the reply goes
    /// out, so a case asserts on what was sent rather than on how much of it
    /// had arrived.
    fn recorded(
        replies: Vec<&'static [u8]>,
    ) -> (std::net::SocketAddr, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let served = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for reply in replies {
                let (mut stream, _) = listener.accept().expect("the client's connection");
                let mut read = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    if let Some(end) = read.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&read[..end]).to_ascii_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .and_then(|value| value.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if read.len() >= end + 4 + length {
                            break;
                        }
                    }
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(got) => read.extend_from_slice(&buffer[..got]),
                    }
                }
                stream.write_all(reply).expect("the reply is written");
                stream.flush().ok();
                requests.push(String::from_utf8_lossy(&read).into_owned());
            }
            requests
        });
        (at, served)
    }

    /// `Core\Http\Client::post` retried after a `503` sends the same
    /// `Idempotency-Key` and the same JSON body on both attempts, which is what
    /// lets the origin read the second `POST` as the first one again rather
    /// than as a second order.
    // covers: Core\Http\Client::post
    #[test]
    fn post_retried_after_a_503_sends_one_idempotency_key_and_one_body_on_every_attempt() {
        let (at, served) = recorded(vec![
            b"HTTP/1.1 503 Service Unavailable\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ]);
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        let url = Value::str(NvsStr::new(format!("http://{at}/orders").as_bytes()));
        let mut order = NvsArray::new();
        order.set(NvsStr::new(b"id"), Value::int(7));
        let json = Value::array(order);
        let key = Value::str(NvsStr::new(b"order-7"));
        let mut args = asking(url);
        args[JSON] = json;
        args[RETRY_ATTEMPTS] = Value::uint(2);
        args[RETRY_KEY] = key;
        let answer =
            super::request(&mut ctx, &args, "post", "POST").expect("the second attempt's answer");
        let object = answer.obj_ptr().expect("the answer is an instance");
        let status = crate::instance::slot(object, STATUS_SLOT).as_int();
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced and the one \
                      the member answered with, and neither is its caller's"
        )]
        unsafe {
            url.release();
            args[HEADERS].release();
            json.release();
            key.release();
            answer.release();
        }
        let requests = served.join().expect("the origin thread");

        assert_eq!(
            status,
            Some(201),
            "the second attempt's answer is the one returned"
        );
        assert_eq!(
            requests.len(),
            2,
            "a `503` is retried once under two attempts"
        );
        for sent in &requests {
            assert!(sent.starts_with("POST /orders HTTP/1.1\r\n"), "{sent:?}");
            assert!(
                sent.contains("\r\nIdempotency-Key: order-7\r\n"),
                "every attempt carries the one key: {sent:?}"
            );
            assert!(
                sent.contains("\r\nContent-Type: application/json\r\n"),
                "{sent:?}"
            );
            assert!(
                sent.ends_with("\r\n\r\n{\"id\":7}"),
                "every attempt carries the whole body: {sent:?}"
            );
        }
    }

    /// `Core\Http\Client::put` writes `PUT` with its body, and retries a `503`
    /// with no `retryIdempotencyKey`: the second attempt is the same request
    /// again, body included, and neither attempt carries an `Idempotency-Key`.
    // covers: Core\Http\Client::put
    #[test]
    fn put_retries_without_a_key_and_sends_the_same_body_on_every_attempt() {
        let (at, served) = recorded(vec![
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        ]);
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        let url = Value::str(NvsStr::new(format!("http://{at}/stock/7").as_bytes()));
        let body = Value::str(NvsStr::new(b"ready"));
        let mut args = asking(url);
        args[RETRY_ATTEMPTS] = Value::uint(2);
        args[JSON] = body;
        let answer =
            super::request(&mut ctx, &args, "put", "PUT").expect("the second attempt's answer");
        let object = answer.obj_ptr().expect("the answer is an instance");
        let status = crate::instance::slot(object, STATUS_SLOT).as_int();
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced and the one \
                      the member answered with, and neither is its caller's"
        )]
        unsafe {
            url.release();
            args[HEADERS].release();
            body.release();
            answer.release();
        }
        let requests = served.join().expect("the origin thread");

        assert_eq!(
            status,
            Some(200),
            "the second attempt's answer is the one returned"
        );
        assert_eq!(requests.len(), 2, "a `503` is tried again without a key");
        for request in &requests {
            assert!(
                request.starts_with("PUT /stock/7 HTTP/1.1\r\n"),
                "the request line carries the verb and the path: {request:?}"
            );
            assert!(
                request.ends_with("\r\n\r\n\"ready\""),
                "every attempt carries the whole body: {request:?}"
            );
            assert!(
                !request.to_ascii_lowercase().contains("idempotency-key"),
                "a `PUT` retry sends no key: {request:?}"
            );
        }
    }

    /// `Core\Http\Client::patch` writes `PATCH` on the request line, frames a
    /// `json` body under its own `Content-Type` and length, and carries a
    /// retried call's `retryIdempotencyKey` as `Idempotency-Key`. The same
    /// retry with no key is refused before any socket is dialled, so the one
    /// connection the origin accepts is the keyed call's.
    // covers: Core\Http\Client::patch
    #[test]
    fn patch_sends_its_verb_json_body_and_idempotency_key_and_refuses_a_keyless_retry() {
        let (at, served) = recorded(vec![
            b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n",
        ]);
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        let url = Value::str(NvsStr::new(format!("http://{at}/orders/7").as_bytes()));
        let status = Value::str(NvsStr::new(b"paid"));
        let key = Value::str(NvsStr::new(b"order-7-paid"));

        let mut keyless = asking(url);
        keyless[JSON] = status;
        keyless[RETRY_ATTEMPTS] = Value::uint(3);
        let refused = super::request(&mut ctx, &keyless, "patch", "PATCH")
            .expect_err("a retried `PATCH` with no key");
        assert!(
            format!("{refused:?}").contains("retryIdempotencyKey"),
            "the refusal names the key it is missing: {refused:?}"
        );

        let mut keyed = asking(url);
        keyed[JSON] = status;
        keyed[RETRY_ATTEMPTS] = Value::uint(3);
        keyed[RETRY_KEY] = key;
        let answer =
            super::request(&mut ctx, &keyed, "patch", "PATCH").expect("the origin's answer");
        let requests = served.join().expect("the origin thread");
        let object = answer.obj_ptr().expect("the answer is an instance");
        let answered = crate::instance::slot(object, STATUS_SLOT).as_int();
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced and the one \
                      the member answered with, and neither is its caller's"
        )]
        unsafe {
            url.release();
            status.release();
            key.release();
            keyless[HEADERS].release();
            keyed[HEADERS].release();
            answer.release();
        }

        let wire = &requests[0];
        assert!(
            wire.starts_with("PATCH /orders/7 HTTP/1.1\r\n"),
            "the request line carries the verb and the path: {wire:?}"
        );
        assert!(
            wire.contains("\r\nIdempotency-Key: order-7-paid\r\n"),
            "a retried `PATCH` sends its key: {wire:?}"
        );
        assert!(
            wire.contains("\r\nContent-Type: application/json\r\n"),
            "{wire:?}"
        );
        assert!(wire.contains("\r\nContent-Length: 6\r\n"), "{wire:?}");
        assert!(
            wire.ends_with("\r\n\r\n\"paid\""),
            "the body is the encoded document: {wire:?}"
        );
        assert_eq!(answered, Some(204));
    }

    /// `Core\Http\Client::request` puts the verb it is handed on the request
    /// line, including one that no named row sends: `Core\Http\Method::Options`
    /// crosses as its ordinal, reads back as `Options`, and goes out as
    /// `OPTIONS` with the URL's own path.
    // covers: Core\Http\Client::request
    #[test]
    fn request_sends_a_verb_no_named_row_sends_on_the_request_line() {
        let verb = crate::router::method_verb(&Value::int(2), "Core\\Http\\Client::request")
            .expect("ordinal 2 is a `Core\\Http\\Method` case")
            .to_ascii_uppercase();
        assert_eq!(verb, "OPTIONS");
        let (head, status, body) = exchanged_once(
            "request",
            &verb,
            "/orders/7",
            b"HTTP/1.1 204 No Content\r\nAllow: GET, PUT, DELETE\r\nConnection: close\r\n\r\n",
        );
        assert!(
            head.starts_with("OPTIONS /orders/7 HTTP/1.1\r\n"),
            "the request line carries the handed verb and the path: {head:?}"
        );
        assert_eq!(status, Some(204));
        assert!(body.is_empty(), "a reply with no body is an empty body");
    }

    /// `Core\Http\Client::stream` returns once the head has arrived: the origin
    /// below writes its status line and headers and holds the body back until
    /// the call has returned. The verb on the wire is the one the call was
    /// given, and the body slot holds the key of a reader rather than octets.
    // covers: Core\Http\Client::stream
    #[test]
    fn stream_answers_at_the_head_while_the_origin_still_holds_the_body() {
        use std::io::{Read, Write};
        use std::sync::mpsc;
        use std::time::Duration;

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let (release, released) = mpsc::channel::<()>();
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client's connection");
            let mut head = Vec::new();
            let mut buffer = [0_u8; 4096];
            while !head.windows(4).any(|end| end == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => head.extend_from_slice(&buffer[..read]),
                }
            }
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                      Content-Length: 11\r\nConnection: close\r\n\r\n",
                )
                .expect("the head is written");
            stream.flush().ok();
            // A client that waited for the body gets it only after this
            // timeout, and `held_back` is then `false`.
            let held_back = released.recv_timeout(Duration::from_secs(5)).is_ok();
            stream.write_all(b"data: one\n\n").ok();
            stream.flush().ok();
            (String::from_utf8_lossy(&head).into_owned(), held_back)
        });

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));
        let url = Value::str(NvsStr::new(format!("http://{at}/v1/chat").as_bytes()));
        let mut args = [Value::null(); STREAM_ARITY];
        args[0] = url;
        args[HEADERS] = Value::array(NvsArray::new());
        args[JSON] = Value::unset();
        let (status, body, headers, _) =
            super::exchanged(&mut ctx, &args, "stream", "POST", true).expect("the head arrived");
        release.send(()).ok();
        let (head, held_back) = served.join().expect("the origin thread");

        assert!(
            held_back,
            "the call returned only once the body had been sent"
        );
        assert!(
            head.starts_with("POST /v1/chat HTTP/1.1\r\n"),
            "the request line carries the verb the call was given: {head:?}"
        );
        assert_eq!(status, 200);
        assert!(
            body.as_uint().is_some(),
            "a streamed body slot holds the key of a reader, not the octets"
        );
        #[expect(
            unsafe_code,
            reason = "this frame owns the references it just produced and the header \
                      map the exchange answered with, and none of them is its caller's"
        )]
        unsafe {
            url.release();
            args[HEADERS].release();
            headers.release();
        }
        drop(ctx);
    }

    /// `Core\Http\Client::openSocket` judges every bound it was handed before
    /// it asks the grant. A context that grants nothing refuses a zero `idle`
    /// by naming the option, the same URL with no bound is refused by
    /// `net.connect`, and the listener the URL names never sees a connection.
    // covers: Core\Http\Client::openSocket
    #[test]
    fn open_socket_judges_its_bounds_before_the_grant_and_never_connects() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        listener
            .set_nonblocking(true)
            .expect("a listener that answers now rather than waiting");
        let at = listener.local_addr().expect("its own address");

        // No configuration at all, so `net.connect` grants nothing.
        let mut ctx = Ctx::buffered();
        let url = Value::str(NvsStr::new(format!("ws://{at}/chat").as_bytes()));
        let idle = crate::instance::build(&crate::time::DURATION, [Value::int(0)]);
        // The empty list a call site fills in for `headers` when none are written.
        let headers = Value::array(NvsArray::new());

        for (bound, named, absent) in [
            (idle, "`idle`", "net.connect"),
            (Value::null(), "net.connect", "`idle`"),
        ] {
            let mut args = [Value::null(); SOCKET_ARITY];
            args[0] = url;
            args[SOCKET_HEADERS] = headers;
            args[SOCKET_IDLE] = bound;
            let refused = nvs_runtime::call(
                super::socket::nvs_core_http_client_open_socket,
                &mut ctx,
                &args,
            );
            assert!(refused.is_err(), "a call {named} refuses opened a socket");
            let message = ctx.take_pending().expect("the refusal is a thrown error");
            assert!(
                message.contains(named) && !message.contains(absent),
                "the refusal names {named} and not {absent}: {message}"
            );
        }
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the references `NvsStr::new` and \
                      `instance::build` produced, and a native member never releases \
                      an argument its caller still owns"
        )]
        unsafe {
            url.release();
            idle.release();
            headers.release();
        }

        match listener.accept() {
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Ok(_) => panic!("a refused call opened a connection to the host anyway"),
            Err(err) => panic!("the listener failed for a reason that is not the point: {err}"),
        }
    }

    /// `Core\Http\Response::bytes` answers the body exactly as the origin sent
    /// it, as a `bytes` value, for a body that is not UTF-8 — the one `text`
    /// refuses — and a second call reads the same octets, because reading
    /// takes a reference and consumes nothing.
    // covers: Core\Http\Response::bytes
    #[test]
    fn response_bytes_reads_a_body_text_refuses_and_reads_it_again() {
        const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\xff\x00";
        let (_, answer) = answered_once(
            "get",
            "GET",
            "/logo.png",
            b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 10\r\n\
              Connection: close\r\n\r\n\x89PNG\r\n\x1a\n\xff\x00",
        );
        let mut ctx = Ctx::buffered();

        let first = nvs_runtime::call(super::nvs_core_http_response_bytes, &mut ctx, &[answer])
            .expect("`bytes` reads any body");
        let second = nvs_runtime::call(super::nvs_core_http_response_bytes, &mut ctx, &[answer])
            .expect("a second read finds the same body");
        assert!(
            matches!(first.tag(), Some(nvs_runtime::Tag::Bytes)),
            "the answer is `bytes`"
        );
        assert_eq!(first.as_bytes(), Some(PNG), "every octet, as it arrived");
        assert_eq!(
            second.as_bytes(),
            Some(PNG),
            "reading the body does not consume it"
        );

        let refused = nvs_runtime::call(super::nvs_core_http_response_text, &mut ctx, &[answer]);
        assert!(
            refused.is_err(),
            "`text` refuses the body `bytes` just read"
        );
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message.contains("UTF-8"),
            "the refusal names the reason: {message}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the response and the two references `bytes` \
                      answered with, and none is its caller's"
        )]
        unsafe {
            first.release();
            second.release();
            answer.release();
        }
    }

    /// `Core\Http\Response::text` answers a UTF-8 body as a `string` carrying
    /// every octet the origin sent, a second call answers the same text, and a
    /// body that stops being UTF-8 part-way through a character is refused with
    /// the offset of the first octet that is not text.
    // covers: Core\Http\Response::text
    #[test]
    fn response_text_reads_a_utf8_body_twice_and_names_where_a_broken_one_stops() {
        const GREETING: &str = "Grüße, 世界\n";
        let (_, answer) = answered_once(
            "get",
            "GET",
            "/hello.txt",
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 16\r\n\
              Connection: close\r\n\r\nGr\xc3\xbc\xc3\x9fe, \xe4\xb8\x96\xe7\x95\x8c\n",
        );
        let mut ctx = Ctx::buffered();

        let first = nvs_runtime::call(super::nvs_core_http_response_text, &mut ctx, &[answer])
            .expect("`text` reads a UTF-8 body");
        let second = nvs_runtime::call(super::nvs_core_http_response_text, &mut ctx, &[answer])
            .expect("a second read finds the same body");
        assert!(
            matches!(first.tag(), Some(nvs_runtime::Tag::Str)),
            "the answer is a `string`"
        );
        assert_eq!(
            first.as_text(),
            Some(GREETING),
            "every octet, as it arrived"
        );
        assert_eq!(
            second.as_text(),
            Some(GREETING),
            "reading the body does not consume it"
        );

        let (_, broken) = answered_once(
            "get",
            "GET",
            "/latin1.txt",
            b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nGr\xc3\xbc\xe4\xb8e",
        );
        let refused = nvs_runtime::call(super::nvs_core_http_response_text, &mut ctx, &[broken]);
        assert!(refused.is_err(), "a truncated character is not text");
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message.contains("byte 4 is where it stops being text"),
            "the refusal names the first octet that is not text: {message}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns both responses and the two references `text` \
                      answered with, and none is its caller's"
        )]
        unsafe {
            first.release();
            second.release();
            answer.release();
            broken.release();
        }
    }

    /// `Core\Http\Response::status` answers the code on the status line as the
    /// origin wrote it — a `404` and a `500` are answers the call returns, not
    /// throws — and ignores the reason phrase beside it, however it is spelled.
    // covers: Core\Http\Response::status
    #[test]
    fn response_status_answers_the_code_the_origin_wrote_for_an_error_too() {
        let replies: [(&'static [u8], i64); 3] = [
            (
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                404,
            ),
            (
                b"HTTP/1.1 500 Everything Is Fine\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                500,
            ),
            (
                b"HTTP/1.1 299 \r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                299,
            ),
        ];
        let mut ctx = Ctx::buffered();
        for (reply, code) in replies {
            let (_, answer) = answered_once("get", "GET", "/", reply);
            let status =
                nvs_runtime::call(super::nvs_core_http_response_status, &mut ctx, &[answer])
                    .expect("an error status is an answer, not a throw");
            assert_eq!(status.as_int(), Some(code), "the code on the status line");
            #[expect(
                unsafe_code,
                reason = "this frame owns the response, and `status` answered an int"
            )]
            unsafe {
                answer.release();
            }
        }
    }

    /// The one-field shape `{name: string}` at the ABI a compiled call site
    /// hands `jsonAs<T>()`: a descriptor, the `array<...>` flag and the wire
    /// contract. The table is leaked because a descriptor's address is its
    /// identity and it must outlive every instance made from it.
    fn named_shape() -> (
        *const nvs_runtime::ClassDesc,
        *const nvs_runtime::ShapeCodec,
    ) {
        use nvs_runtime::{ClassTable, CodecField, CodecTy};

        let mut table = ClassTable::new();
        // `$` cannot start a Novis identifier, so no declared class collides.
        let id = table.define("$shape{name}".to_owned(), &["name"], &[]);
        let codec = vec![CodecField {
            key: "name".to_owned(),
            slot: 0,
            param: 0,
            ty: CodecTy::Str,
            element: None,
            class: None,
            cases: None,
            shape: None,
            nullable: false,
            required: true,
            default: None,
        }];
        let shape = table.define_shape_codec(codec, vec![std::ptr::null()], vec![std::ptr::null()]);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        (table.desc(id), shape)
    }

    /// `Core\Http\Response::jsonAs` hydrates a UTF-8 body into the shape it was
    /// handed, and a second call reads the same body again; a `maxDepth` outside
    /// its range is refused before the body is read, and a body that is not
    /// UTF-8 is refused as a document that is not JSON.
    // covers: Core\Http\Response::jsonAs
    #[test]
    fn response_json_as_hydrates_a_shape_twice_and_refuses_a_bad_depth_and_a_non_text_body() {
        let (class, codec) = named_shape();
        let asked = |ctx: &mut Ctx, answer: Value, depth: u64| {
            nvs_runtime::call(
                super::nvs_core_http_response_json_as,
                ctx,
                &[
                    Value::class_desc(class),
                    Value::bool(false),
                    Value::shape_codec(codec),
                    answer,
                    Value::uint(depth),
                ],
            )
        };
        let name_of = |instance: Value| -> String {
            #[expect(
                unsafe_code,
                reason = "the value is an instance this frame holds a reference to, so \
                          the field borrowed from it cannot outlive the allocation"
            )]
            let object = std::mem::ManuallyDrop::new(unsafe {
                nvs_runtime::NvsObj::from_raw(instance.obj_ptr().expect("an instance"))
            });
            object
                .field(0)
                .as_text()
                .expect("`name` is declared `string`")
                .to_owned()
        };
        let (_, answer) = answered_once(
            "get",
            "GET",
            "/me",
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 18\r\n\
              Connection: close\r\n\r\n{\"name\":\"Gr\xc3\xbc\xc3\x9fe\"}",
        );
        let mut ctx = Ctx::buffered();

        let first = asked(&mut ctx, answer, 64).expect("the body is the document");
        let second = asked(&mut ctx, answer, 64).expect("a second read finds the same body");
        assert_eq!(name_of(first), "Grüße", "the field, decoded");
        assert_eq!(
            name_of(second),
            "Grüße",
            "reading the body does not consume it"
        );

        assert!(
            asked(&mut ctx, answer, 0).is_err(),
            "a depth of 0 is outside the bag's range"
        );
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message.contains("maxDepth"),
            "the refusal names the option: {message}"
        );

        let (_, broken) = answered_once(
            "get",
            "GET",
            "/blob",
            b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\n{\"name\":\"\xff\"}",
        );
        // `take_pending` answers an empty sentence for a `ParseError` thrown with
        // an issue list under a bare `Ctx`, so the sentence is pinned from Novis by
        // `docs/examples/core/Http-Response/jsonAs/02-a-reply-that-is-not-json.nvs`.
        assert!(
            asked(&mut ctx, broken, 64).is_err(),
            "a body that is not UTF-8 is not a JSON document"
        );
        assert!(ctx.take_pending().is_some(), "the refusal is pending");

        #[expect(
            unsafe_code,
            reason = "this frame owns both responses and the two instances `jsonAs` \
                      answered with, and none is its caller's"
        )]
        unsafe {
            first.release();
            second.release();
            answer.release();
            broken.release();
        }
    }

    /// A reply carrying `Vary` on two lines under two spellings and
    /// `Set-Cookie` on two lines, one of them with a comma of its own — the
    /// fixture both field readers below are asked about.
    const FIELDS_REPLY: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nVary: Accept\r\n\
        vary: Accept-Encoding\r\nSet-Cookie: a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT\r\n\
        Set-Cookie: b=2\r\nConnection: close\r\n\r\n";

    /// `name` as the text a program passes a field reader.
    fn field(name: &str) -> Value {
        Value::str(NvsStr::new(name.as_bytes()))
    }

    /// `Core\Http\Response::header` matches a name however either side
    /// capitalised it, joins a field that arrived on two lines with `, ` in
    /// arrival order, answers `null` for a field that never arrived, and throws
    /// for `Set-Cookie` in any capitalisation, naming `headers` as the reader.
    // covers: Core\Http\Response::header
    #[test]
    fn response_header_joins_repeats_answers_null_and_refuses_set_cookie() {
        let (_, answer) = answered_once("get", "GET", "/", FIELDS_REPLY);
        let mut ctx = Ctx::buffered();
        let (upper, missing, cookie) = (field("VARY"), field("x-missing"), field("set-COOKIE"));

        let joined = nvs_runtime::call(
            super::nvs_core_http_response_header,
            &mut ctx,
            &[answer, upper],
        )
        .expect("a field that arrived is read");
        assert_eq!(
            joined.as_text(),
            Some("Accept, Accept-Encoding"),
            "joined in arrival order"
        );
        let absent = nvs_runtime::call(
            super::nvs_core_http_response_header,
            &mut ctx,
            &[answer, missing],
        )
        .expect("a field that never arrived is an answer too");
        assert!(
            matches!(absent.tag(), Some(nvs_runtime::Tag::Null)),
            "no such field is `null`"
        );

        let refused = nvs_runtime::call(
            super::nvs_core_http_response_header,
            &mut ctx,
            &[answer, cookie],
        );
        assert!(refused.is_err(), "`Set-Cookie` is never joined");
        let message = ctx.take_pending().expect("the refusal says why");
        assert!(
            message.contains("headers(\"set-cookie\")"),
            "it names the reader: {message}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the response, the three names and the value \
                      `header` answered with, and none is its caller's"
        )]
        unsafe {
            joined.release();
            upper.release();
            missing.release();
            cookie.release();
            answer.release();
        }
    }

    /// `Core\Http\Response::headers` keeps every line of a field apart and in
    /// arrival order — two cookies stay two, the comma inside an `Expires`
    /// included — and answers an empty list rather than `null` for a field
    /// that never arrived.
    // covers: Core\Http\Response::headers
    #[test]
    fn response_headers_keeps_each_line_apart_and_answers_empty_for_none() {
        let (_, answer) = answered_once("get", "GET", "/", FIELDS_REPLY);
        let mut ctx = Ctx::buffered();
        let (cookie, vary, missing) = (field("Set-Cookie"), field("vary"), field("x-missing"));

        for (name, lines) in [
            (
                cookie,
                &["a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT", "b=2"][..],
            ),
            (vary, &["Accept", "Accept-Encoding"][..]),
            (missing, &[][..]),
        ] {
            let listed = nvs_runtime::call(
                super::nvs_core_http_response_headers,
                &mut ctx,
                &[answer, name],
            )
            .expect("`headers` never throws");
            let array = crate::arr::borrowed(listed.array_ptr().expect("the answer is an array"));
            let read: Vec<String> = (0..array.count())
                .map(|at| {
                    let line = array
                        .get_index(i64::try_from(at).expect("an index"))
                        .expect("a line");
                    line.as_text().expect("each line is text").to_owned()
                })
                .collect();
            assert_eq!(read, lines, "one entry per line, in arrival order");
            #[expect(
                unsafe_code,
                reason = "this frame owns the list `headers` answered with, and it is not \
                          its caller's"
            )]
            unsafe {
                listed.release();
            }
        }

        #[expect(
            unsafe_code,
            reason = "this frame owns the response and the three names, and none is its \
                      caller's"
        )]
        unsafe {
            cookie.release();
            vary.release();
            missing.release();
            answer.release();
        }
    }

    /// A `Core\Http\TlsInfo` holding one freshly generated certificate for
    /// `api.example.com`, filled the way `tls_info` fills one from a handshake:
    /// the version, the suite, whether the call verified, and the chain as DER.
    /// The caller owns it.
    fn tls_info_for_example_host(verified: bool) -> Value {
        let issued = rcgen::generate_simple_self_signed(vec!["api.example.com".to_owned()])
            .expect("the certificate could not be generated");
        let mut chain = NvsArray::new();
        chain.append(Value::bytes(NvsStr::new(issued.cert.der())));
        crate::instance::build(
            &super::TLS_INFO,
            [
                Value::str(NvsStr::new(b"TLSv1.3")),
                Value::str(NvsStr::new(b"TLS13_AES_128_GCM_SHA256")),
                Value::bool(verified),
                Value::array(chain),
            ],
        )
    }

    /// `Core\Http\Response::tls` answers `null` for a reply a real origin sent
    /// over plain `http`, and for a reply holding a session answers that same
    /// `Core\Http\TlsInfo` with a reference of the caller's own: reading it twice
    /// and releasing each read leaves the response's own reference standing.
    // covers: Core\Http\Response::tls
    #[test]
    fn response_tls_is_null_over_plain_http_and_the_held_session_otherwise() {
        let mut ctx = Ctx::buffered();
        let (_, plain) = answered_once(
            "get",
            "GET",
            "/",
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        );
        let none = nvs_runtime::call(super::nvs_core_http_response_tls, &mut ctx, &[plain])
            .expect("`tls` never throws");
        assert_eq!(
            none.tag(),
            Some(nvs_runtime::Tag::Null),
            "a reply over plain http reports no session"
        );

        let session = tls_info_for_example_host(true);
        let object = session.obj_ptr().expect("the session is an instance");
        let secure = crate::instance::build(
            &RESPONSE,
            [Value::int(200), Value::null(), Value::null(), session],
        );
        for _ in 0..2 {
            let read = nvs_runtime::call(super::nvs_core_http_response_tls, &mut ctx, &[secure])
                .expect("`tls` never throws");
            assert_eq!(read.obj_ptr(), Some(object), "the session the reply holds");
            #[expect(
                unsafe_code,
                reason = "the object is live: the response holds a reference to it"
            )]
            let (held, verified) = unsafe {
                (
                    nvs_runtime::NvsObj::refcount_of(object),
                    crate::instance::slot(object, super::TLS_VERIFIED_SLOT).as_bool(),
                )
            };
            assert_eq!(held, 2, "the response's reference and this read's");
            assert_eq!(
                verified,
                Some(true),
                "the session is the one the reply was built with"
            );
            #[expect(
                unsafe_code,
                reason = "this frame owns the reference `tls` answered with"
            )]
            unsafe {
                read.release();
            }
        }
        #[expect(
            unsafe_code,
            reason = "the object is live: the response still holds its reference"
        )]
        let held = unsafe { nvs_runtime::NvsObj::refcount_of(object) };
        assert_eq!(held, 1, "every read released what it took");

        #[expect(
            unsafe_code,
            reason = "this frame owns both responses, and neither is its caller's"
        )]
        unsafe {
            plain.release();
            secure.release();
        }
    }

    /// `Core\Http\TlsInfo::version` and `::cipher` read back what the session
    /// settled on, spelled as `nvs_host::tls` writes it: every suite this build
    /// negotiates, under both versions, reads back unchanged and paired with
    /// its own version. A receiver that is not an object is a fatal, not a
    /// string.
    // covers: Core\Http\TlsInfo::version
    // covers: Core\Http\TlsInfo::cipher
    #[test]
    fn tls_info_version_and_cipher_read_back_every_suite_under_its_own_version() {
        let mut ctx = Ctx::buffered();
        let now = std::time::SystemTime::now();
        let read = |ctx: &mut Ctx, member, info: Value| {
            let text = nvs_runtime::call(member, ctx, &[info]).expect("a reader never throws");
            let owned = text.as_text().expect("the reading is text").to_owned();
            #[expect(unsafe_code, reason = "this frame owns the reading")]
            unsafe {
                text.release();
            }
            owned
        };
        for (version, cipher) in [
            ("TLSv1.3", "TLS_AES_128_GCM_SHA256"),
            ("TLSv1.3", "TLS_AES_256_GCM_SHA384"),
            ("TLSv1.3", "TLS_CHACHA20_POLY1305_SHA256"),
            ("TLSv1.2", "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256"),
            ("TLSv1.2", "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384"),
            ("TLSv1.2", "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256"),
            ("TLSv1.2", "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256"),
        ] {
            let session = nvs_host::tls::described(&nvs_host::tls::Description {
                version: Some(version),
                cipher: Some(cipher),
                subject: "CN=api.example.com",
                issuer: "CN=Example Test CA",
                expiry: now + std::time::Duration::from_secs(86_400),
                now,
            })
            .expect("a suite this build negotiates");
            let info = super::tls_info_of(&session, true);
            assert_eq!(
                read(&mut ctx, super::nvs_core_http_tls_info_version, info),
                version
            );
            assert_eq!(
                read(&mut ctx, super::nvs_core_http_tls_info_cipher, info),
                cipher
            );
            #[expect(unsafe_code, reason = "this frame owns the session")]
            unsafe {
                info.release();
            }
        }

        for member in [
            super::nvs_core_http_tls_info_version,
            super::nvs_core_http_tls_info_cipher,
        ] {
            assert!(
                nvs_runtime::call(member, &mut ctx, &[Value::null()]).is_err(),
                "`null` is not a session"
            );
        }
    }

    /// `Core\Http\TlsInfo::verified` reads back the flag the session was filled
    /// with, from a handshake's slot and from a described session alike, and
    /// the flag is each session's own: one described chain read under both
    /// flags answers both. A receiver that is not an object is a fatal, not a
    /// `bool`.
    // covers: Core\Http\TlsInfo::verified
    #[test]
    fn tls_info_verified_reads_the_flag_each_session_was_filled_with() {
        let mut ctx = Ctx::buffered();
        let now = std::time::SystemTime::now();
        let described = nvs_host::tls::described(&nvs_host::tls::Description {
            version: None,
            cipher: None,
            subject: "CN=api.example.com",
            issuer: "CN=Example Test CA",
            expiry: now + std::time::Duration::from_secs(86_400),
            now,
        })
        .expect("the default session");
        for verified in [true, false] {
            for info in [
                tls_info_for_example_host(verified),
                super::tls_info_of(&described, verified),
            ] {
                let read =
                    nvs_runtime::call(super::nvs_core_http_tls_info_verified, &mut ctx, &[info])
                        .expect("`verified` never throws");
                assert_eq!(read.as_bool(), Some(verified));
                #[expect(unsafe_code, reason = "this frame owns the session")]
                unsafe {
                    info.release();
                }
            }
        }
        assert!(
            nvs_runtime::call(
                super::nvs_core_http_tls_info_verified,
                &mut ctx,
                &[Value::null()]
            )
            .is_err(),
            "`null` is not a session"
        );
    }

    /// `Core\Http\TlsInfo::subject`, `::issuer` and `::expiry` parse the leaf of
    /// the chain: a described session reads back the names it was written with
    /// and its expiry in whole seconds, and a self-signed leaf names itself as
    /// its own issuer. A receiver that is not an object is a fatal for each.
    // covers: Core\Http\TlsInfo::subject
    // covers: Core\Http\TlsInfo::issuer
    // covers: Core\Http\TlsInfo::expiry
    #[test]
    fn tls_info_subject_issuer_and_expiry_read_the_leaf_of_the_chain() {
        fn text_of(read: Value) -> String {
            let text = read.as_text().expect("a name is text").to_owned();
            #[expect(unsafe_code, reason = "this frame owns the read")]
            unsafe {
                read.release();
            }
            text
        }

        let mut ctx = Ctx::buffered();
        let now = std::time::SystemTime::now();
        let expiry = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_938_254_400);
        let described = nvs_host::tls::described(&nvs_host::tls::Description {
            version: None,
            cipher: None,
            subject: "CN=api.example.com, O=Shop",
            issuer: "CN=Example Test CA",
            expiry,
            now,
        })
        .expect("a described session");
        let info = super::tls_info_of(&described, true);
        let subject = nvs_runtime::call(super::nvs_core_http_tls_info_subject, &mut ctx, &[info])
            .expect("a described leaf parses");
        assert_eq!(text_of(subject), "CN=api.example.com, O=Shop");
        let issuer = nvs_runtime::call(super::nvs_core_http_tls_info_issuer, &mut ctx, &[info])
            .expect("a described leaf parses");
        assert_eq!(text_of(issuer), "CN=Example Test CA");
        let instant = nvs_runtime::call(super::nvs_core_http_tls_info_expiry, &mut ctx, &[info])
            .expect("a described leaf parses");
        let seconds = nvs_runtime::call(
            crate::time::nvs_core_time_instant_to_epoch_seconds,
            &mut ctx,
            &[instant],
        )
        .expect("an instant has epoch seconds");
        assert_eq!(seconds.as_int(), Some(1_938_254_400));
        #[expect(unsafe_code, reason = "this frame owns the session and the instant")]
        unsafe {
            instant.release();
            info.release();
        }

        let self_signed = tls_info_for_example_host(true);
        let subject = nvs_runtime::call(
            super::nvs_core_http_tls_info_subject,
            &mut ctx,
            &[self_signed],
        )
        .expect("an rcgen leaf parses");
        let issuer = nvs_runtime::call(
            super::nvs_core_http_tls_info_issuer,
            &mut ctx,
            &[self_signed],
        )
        .expect("an rcgen leaf parses");
        assert_eq!(text_of(subject), text_of(issuer));
        #[expect(unsafe_code, reason = "this frame owns the session")]
        unsafe {
            self_signed.release();
        }

        for reader in [
            super::nvs_core_http_tls_info_subject,
            super::nvs_core_http_tls_info_issuer,
            super::nvs_core_http_tls_info_expiry,
        ] {
            assert!(
                nvs_runtime::call(reader, &mut ctx, &[Value::null()]).is_err(),
                "`null` is not a session"
            );
        }
    }

    /// `Core\Http\TlsInfo::peerChain` writes each certificate the session holds
    /// as one PEM block, leaf first: a described session's two blocks decode
    /// back to its own leaf and CA in that order, no body line is wider than 64
    /// characters, and a session holding no chain answers an empty array. A
    /// receiver that is not an object is a fatal.
    // covers: Core\Http\TlsInfo::peerChain
    #[test]
    fn tls_info_peer_chain_writes_each_certificate_as_pem_leaf_first() {
        use base64::Engine as _;

        fn blocks_of(read: Value) -> Vec<String> {
            let mut out = Vec::new();
            {
                let held = crate::arr::borrowed(read.array_ptr().expect("the chain is an array"));
                let mut from = 0_usize;
                while let Some(slot) = held.next_slot(from) {
                    from = slot + 1;
                    let block = held
                        .value_at(slot)
                        .expect("next_slot only names live entries");
                    out.push(block.as_text().expect("a PEM block is text").to_owned());
                }
            }
            #[expect(unsafe_code, reason = "this frame owns the read")]
            unsafe {
                read.release();
            }
            out
        }

        let mut ctx = Ctx::buffered();
        let now = std::time::SystemTime::now();
        let described = nvs_host::tls::described(&nvs_host::tls::Description {
            version: None,
            cipher: None,
            subject: "CN=api.example.com",
            issuer: "CN=Example Test CA",
            expiry: now + std::time::Duration::from_secs(86_400),
            now,
        })
        .expect("a described session");
        assert_eq!(
            described.chain().len(),
            2,
            "a leaf and the CA that signed it"
        );
        let info = super::tls_info_of(&described, true);
        let read = nvs_runtime::call(super::nvs_core_http_tls_info_peer_chain, &mut ctx, &[info])
            .expect("`peerChain` never throws for a session");
        let blocks = blocks_of(read);
        assert_eq!(blocks.len(), 2);
        for (block, der) in blocks.iter().zip(described.chain()) {
            let body = block
                .strip_prefix("-----BEGIN CERTIFICATE-----\n")
                .and_then(|rest| rest.strip_suffix("-----END CERTIFICATE-----\n"))
                .expect("a block is framed by the two `CERTIFICATE` lines");
            assert!(body.lines().all(|line| line.len() <= 64), "{block}");
            let decoded = super::STANDARD
                .decode(body.replace('\n', ""))
                .expect("a block's body is base64");
            assert_eq!(&decoded, der, "the blocks keep the chain's order");
        }
        #[expect(unsafe_code, reason = "this frame owns the session")]
        unsafe {
            info.release();
        }

        let bare = crate::instance::build(
            &super::TLS_INFO,
            [
                Value::str(NvsStr::new(b"TLSv1.3")),
                Value::str(NvsStr::new(b"TLS13_AES_128_GCM_SHA256")),
                Value::bool(false),
                Value::array(NvsArray::new()),
            ],
        );
        let read = nvs_runtime::call(super::nvs_core_http_tls_info_peer_chain, &mut ctx, &[bare])
            .expect("an empty chain is not an error");
        assert!(blocks_of(read).is_empty());
        #[expect(unsafe_code, reason = "this frame owns the session")]
        unsafe {
            bare.release();
        }

        assert!(
            nvs_runtime::call(
                super::nvs_core_http_tls_info_peer_chain,
                &mut ctx,
                &[Value::null()]
            )
            .is_err(),
            "`null` is not a session"
        );
    }

    /// `Core\Http\Part::file` keeps the path and never the octets. The part it
    /// builds for a granted file holds the path, an empty data slot and the
    /// path's last component as its `filename`, and a written `filename` and
    /// `contentType` replace those defaults. A file that exists outside the
    /// grant is refused by `fs.read` before it is looked at.
    // covers: Core\Http\Part::file
    #[test]
    fn part_file_holds_the_path_and_no_octets_and_asks_the_grant_first() {
        let dir = std::env::temp_dir().join("nvs-http-part-file");
        let granted = dir.join("granted");
        std::fs::create_dir_all(&granted).expect("a temporary directory the tests own");
        let inside = granted.join("report.csv");
        std::fs::write(&inside, b"day,total\n").expect("the file the part names");
        let outside = dir.join("outside.csv");
        std::fs::write(&outside, b"not granted\n").expect("a file beside the grant");

        // A TOML literal string, because a Windows path in a basic string is escapes.
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(&format!(
            "[capabilities.fs]\nread = ['{}']\n",
            granted.display()
        )));

        let written = inside.to_string_lossy().into_owned();
        let path = Value::str(NvsStr::new(written.as_bytes()));
        let named = Value::str(NvsStr::new(b"renamed.csv"));
        let typed = Value::str(NvsStr::new(b"text/csv"));
        for (filename, content_type, want_name, want_type) in [
            (Value::null(), Value::null(), "report.csv", None),
            (named, typed, "renamed.csv", Some("text/csv")),
        ] {
            let part = nvs_runtime::call(
                super::nvs_core_http_part_file,
                &mut ctx,
                &[path, filename, content_type],
            )
            .expect("a granted file that exists");
            let object = part.obj_ptr().expect("the answer is an instance");
            assert_eq!(
                crate::instance::slot(object, super::PART_PATH_SLOT).as_text(),
                Some(written.as_str()),
                "the part holds the path as written"
            );
            assert!(
                matches!(
                    crate::instance::slot(object, super::PART_DATA_SLOT).tag(),
                    Some(nvs_runtime::Tag::Null)
                ),
                "a file part holds no octets"
            );
            assert_eq!(
                crate::instance::slot(object, super::PART_FILENAME_SLOT).as_text(),
                Some(want_name)
            );
            assert_eq!(
                crate::instance::slot(object, super::PART_CONTENT_TYPE_SLOT).as_text(),
                want_type
            );
            #[expect(
                unsafe_code,
                reason = "this frame owns the part the member answered with"
            )]
            unsafe {
                part.release();
            }
        }

        let beside = Value::str(NvsStr::new(outside.to_string_lossy().as_bytes()));
        let refused = nvs_runtime::call(
            super::nvs_core_http_part_file,
            &mut ctx,
            &[beside, Value::null(), Value::null()],
        );
        assert!(refused.is_err(), "a file outside the grant became a part");
        let message = ctx.take_pending().expect("the refusal is a thrown error");
        assert!(
            message.contains("fs.read"),
            "an existing file outside the grant is refused by the capability: {message}"
        );
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the references `NvsStr::new` produced, \
                      and a native member never releases an argument its caller still owns"
        )]
        unsafe {
            path.release();
            named.release();
            typed.release();
            beside.release();
        }
    }

    /// A form with a field name that cannot be sent is refused before any of
    /// its files is opened. The part below names a file the context may read
    /// when the part is built and may not read when the form is framed, so the
    /// error being the name's rather than `fs.read`'s shows that no file was
    /// opened first.
    #[test]
    fn a_multipart_form_checks_every_name_before_it_opens_a_file() {
        let dir = std::env::temp_dir().join("nvs-http-multipart-order");
        std::fs::create_dir_all(&dir).expect("a temporary directory the tests own");
        let inside = dir.join("note.txt");
        std::fs::write(&inside, b"note\n").expect("the file the part names");

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(&format!(
            "[capabilities.fs]\nread = ['{}']\n",
            dir.display()
        )));
        let path = Value::str(NvsStr::new(inside.to_string_lossy().as_bytes()));
        let part = nvs_runtime::call(
            super::nvs_core_http_part_file,
            &mut ctx,
            &[path, Value::null(), Value::null()],
        )
        .expect("a granted file that exists");
        ctx.set_config(granting(REACHABLE));

        let mut fields = NvsArray::new();
        fields.set(NvsStr::new(b"file"), part);
        fields.set(NvsStr::new(b"bad\"name"), Value::str(NvsStr::new(b"x")));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[super::MULTIPART] = Value::array(fields);
        let Err(refused) = super::multipart(&ctx, &args, r"Core\Http\Client::post") else {
            panic!("a field name with a quote was framed");
        };
        let refused = format!("{refused:?}");
        assert!(
            refused.contains("field name"),
            "the name is what is refused: {refused}"
        );
        assert!(
            !refused.contains("fs.read"),
            "no file was opened first: {refused}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the path it produced and the form array, which \
                      holds the only reference to the part"
        )]
        unsafe {
            path.release();
            args[super::MULTIPART].release();
        }
    }

    /// `Core\Http\Part::bytes` stores its octets as `bytes` whichever arm
    /// carried them and stores no path. A `string` and a `bytes` of the same
    /// octets read back as the same held piece, with the name as written and
    /// an omitted `contentType` read back as none.
    // covers: Core\Http\Part::bytes
    #[test]
    fn part_bytes_holds_either_arm_as_the_same_octets_and_no_path() {
        const OCTETS: &[u8] = b"name,total\n";
        let mut ctx = Ctx::buffered();
        let text = Value::str(NvsStr::new(OCTETS));
        let raw = Value::bytes(NvsStr::new(OCTETS));
        let filename = Value::str(NvsStr::new(b"orders.csv"));
        let typed = Value::str(NvsStr::new(b"text/csv"));

        let from_text = nvs_runtime::call(
            super::nvs_core_http_part_bytes,
            &mut ctx,
            &[text, filename, typed],
        )
        .expect("a `string` arm builds a part");
        let from_bytes = nvs_runtime::call(
            super::nvs_core_http_part_bytes,
            &mut ctx,
            &[raw, filename, Value::null()],
        )
        .expect("a `bytes` arm builds a part");

        for (part, content_type) in [(from_text, Some("text/csv")), (from_bytes, None)] {
            let object = part.obj_ptr().expect("the answer is an instance");
            assert!(
                matches!(
                    crate::instance::slot(object, super::PART_PATH_SLOT).tag(),
                    Some(nvs_runtime::Tag::Null)
                ),
                "a part built from octets holds no path"
            );
            assert_eq!(
                crate::instance::slot(object, super::PART_DATA_SLOT).as_bytes(),
                Some(OCTETS),
                "the octets are held as `bytes` whichever arm carried them"
            );
            let framed = super::part_of(&ctx, part, r"Core\Http\Client::post")
                .expect("a part this member built reads back");
            assert_eq!(framed.length, OCTETS.len() as u64);
            assert!(
                matches!(&framed.piece, super::transport::Piece::Held(held) if held.as_slice() == OCTETS),
                "the piece is the octets themselves, held"
            );
            assert_eq!(framed.filename.as_deref(), Some("orders.csv"));
            assert_eq!(framed.content_type.as_deref(), content_type);
        }

        #[expect(
            unsafe_code,
            reason = "this frame owns the references `NvsStr::new` produced and the \
                      two parts the member answered with, and none is its caller's"
        )]
        unsafe {
            text.release();
            raw.release();
            filename.release();
            typed.release();
            from_text.release();
            from_bytes.release();
        }
    }

    /// `Core\Http\Identity::read` keeps what it checked: the chain exactly as
    /// the program passed it, the pair's own PKCS#8 DER, and the leaf's SHA-256
    /// as lower-case hex. The fingerprint is the leaf's alone, so the same leaf
    /// with its issuer after it names the same identity, while the issuer's key
    /// over that chain is thrown as a `LogicError` naming the mismatch.
    // covers: Core\Http\Identity::read
    #[test]
    fn identity_read_keeps_the_chain_and_names_the_identity_by_its_leaf() {
        use sha2::Digest as _;

        const LEAF: &str = "-----BEGIN CERTIFICATE-----\nMIIBZzCCAQ6gAwIBAgIBAjAKBggqhkjOPQQDAjAYMRYwFAYDVQQDDA1ub3ZpcyB0\nZXN0IGNhMCAXDTI2MDkxMjIzNDMyMloYDzIxMjYwODE5MjM0MzIyWjAdMRswGQYD\nVQQDDBJsZWFmLm5vdmlzLmV4YW1wbGUwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNC\nAARsn8jTlvalvY27GIy5MEAhedk0hTduuei+6w3oC+YGT6Oo/R0MxoMtmUvhJLqW\nwY841Jo0D0ncfN+RfqQumQgWo0IwQDAdBgNVHQ4EFgQU4YjxNI3Zm79IaJTjEAsd\nZ33g5NgwHwYDVR0jBBgwFoAUYXHdOcSvHEpU6eRt6uCGrm/kckEwCgYIKoZIzj0E\nAwIDRwAwRAIgU7u+n0wo8NXFiu4+xaVKNc3nkN18o3GFgjuLmJkvLRQCICGQuFac\ngtWeQVkGYL2V+BnVrdR9QaThMcoIojXmgY6Q\n-----END CERTIFICATE-----\n";
        const ISSUER: &str = "-----BEGIN CERTIFICATE-----\nMIIBhjCCAS2gAwIBAgIUV0uykyi92cy1XbXyftDdAZt6jL0wCgYIKoZIzj0EAwIw\nGDEWMBQGA1UEAwwNbm92aXMgdGVzdCBjYTAgFw0yNjA5MTIyMzQzMjJaGA8yMTI2\nMDgxOTIzNDMyMlowGDEWMBQGA1UEAwwNbm92aXMgdGVzdCBjYTBZMBMGByqGSM49\nAgEGCCqGSM49AwEHA0IABBbbsFmglrXdizZZ5GVhBN1uQOBg93SUc/deqOO5ZPMn\nTVDw2JDzW4uCnCeBlRTvu+xJG2srdROAdmchqKfz12ijUzBRMB0GA1UdDgQWBBRh\ncd05xK8cSlTp5G3q4Iaub+RyQTAfBgNVHSMEGDAWgBRhcd05xK8cSlTp5G3q4Iau\nb+RyQTAPBgNVHRMBAf8EBTADAQH/MAoGCCqGSM49BAMCA0cAMEQCIDXK5Gwryalq\ntQsX3awNpXAZJ8e0tYVGlMF1WQf14CK6AiALVkvZIJbooDOaBEhlpJC26/S+ArGm\njO0pIW15iNfuGQ==\n-----END CERTIFICATE-----\n";
        const LEAF_KEY: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgNfM2JpNIZ3eTXvRd\n5pkBv32iz2fY5Nto/tlQ5IS9n36hRANCAARsn8jTlvalvY27GIy5MEAhedk0hTdu\nuei+6w3oC+YGT6Oo/R0MxoMtmUvhJLqWwY841Jo0D0ncfN+RfqQumQgW\n-----END PRIVATE KEY-----\n";
        const ISSUER_KEY: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQge4bIsc4PmELo2CVS\ntvkEVY7KDqvUT529mhxe+FepsvahRANCAAQW27BZoJa13Ys2WeRlYQTdbkDgYPd0\nlHP3XqjjuWTzJ01Q8NiQ81uLgpwngZUU77vsSRtrK3UTgHZnIain89do\n-----END PRIVATE KEY-----\n";

        // The leaf's DER, decoded here independently of the member, and the
        // name an identity over it must carry.
        let body: String = LEAF
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect();
        let der = data_encoding::BASE64
            .decode(body.as_bytes())
            .expect("the leaf's PEM body is base64");
        let expected = data_encoding::HEXLOWER.encode(&sha2::Sha256::digest(&der));

        let mut ctx = Ctx::buffered();
        let p256 = Value::int(crate::crypto::KeyKind::P256.tag());
        let leaf_key = Value::bytes(NvsStr::new(LEAF_KEY.as_bytes()));
        let issuer_key = Value::bytes(NvsStr::new(ISSUER_KEY.as_bytes()));
        let mine = nvs_runtime::call(
            crate::crypto::nvs_core_crypto_key_pair_read,
            &mut ctx,
            &[leaf_key, p256],
        )
        .expect("the leaf's key reads as a P-256 pair");
        let theirs = nvs_runtime::call(
            crate::crypto::nvs_core_crypto_key_pair_read,
            &mut ctx,
            &[issuer_key, p256],
        )
        .expect("the issuer's key reads as a P-256 pair");
        let (held, _) = crate::crypto::stored_key(&[mine], 0, &crate::crypto::KEY_PAIR, "read")
            .expect("a pair fills its key slot");
        let pkcs8 = crate::crypto::stored_octets(&held, &crate::crypto::KEY_PAIR, "read")
            .expect("the pair's key slot holds its DER")
            .to_vec();

        let chained_text = format!("{LEAF}{ISSUER}");
        let alone = Value::bytes(NvsStr::new(LEAF.as_bytes()));
        let chained = Value::bytes(NvsStr::new(chained_text.as_bytes()));
        for (chain, written) in [(alone, LEAF), (chained, chained_text.as_str())] {
            let identity =
                nvs_runtime::call(super::nvs_core_http_identity_read, &mut ctx, &[chain, mine])
                    .expect("the leaf's own key is its identity");
            let object = identity.obj_ptr().expect("the answer is an instance");
            assert_eq!(
                crate::instance::slot(object, super::IDENTITY_CHAIN_SLOT).as_bytes(),
                Some(written.as_bytes()),
                "the chain is kept exactly as the program passed it"
            );
            assert_eq!(
                crate::instance::slot(object, super::IDENTITY_PKCS8_SLOT).as_bytes(),
                Some(pkcs8.as_slice()),
                "the key is the pair's own DER"
            );
            assert_eq!(
                crate::instance::slot(object, super::IDENTITY_FINGERPRINT_SLOT).as_text(),
                Some(expected.as_str()),
                "the name is the leaf's SHA-256, whatever certificates follow it"
            );
            #[expect(
                unsafe_code,
                reason = "the call answered this reference and nothing else holds it"
            )]
            unsafe {
                identity.release();
            }
        }

        // The issuer's key matches the chain's second certificate and not its
        // first, so it is refused as a mistake in the deployment's own files.
        let refused = nvs_runtime::call(
            super::nvs_core_http_identity_read,
            &mut ctx,
            &[chained, theirs],
        );
        assert!(
            refused.is_err(),
            "the issuer's key was taken for the leaf's"
        );
        assert_eq!(
            ctx.pending_class().as_deref(),
            Some("LogicError"),
            "a mismatched chain and key are the program's own mistake"
        );
        let message = ctx.take_pending().expect("the refusal is a thrown error");
        assert!(
            message.contains("different public key"),
            "the refusal names the mismatch: {message}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns every reference `NvsStr::new` and the two reads \
                      produced, and a native member never releases an argument its \
                      caller still owns"
        )]
        unsafe {
            alone.release();
            chained.release();
            leaf_key.release();
            issuer_key.release();
            mine.release();
            theirs.release();
        }
    }

    /// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s refusal,
    /// where it has to land: before the socket. A listener is bound on the
    /// address the URL names, the deployment grants that address and no
    /// relaxation, and the assertion is that nothing was ever accepted — a
    /// client that asked the grant after connecting throws the very same error,
    /// having already announced this process to the host it may not relax for.
    ///
    /// A `LogicError` and not the door's `RuntimeError`: the program asked for
    /// its own guarantee to be weakened somewhere nobody said it could be
    /// (`nvs_runtime::capability::require_as`).
    #[test]
    fn a_tls_option_without_its_host_grant_throws_before_connecting() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        listener
            .set_nonblocking(true)
            .expect("a listener that answers now rather than waiting");
        let at = listener.local_addr().expect("its own address");

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(REACHABLE));

        let url = Value::str(NvsStr::new(format!("http://{at}/ok").as_bytes()));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = url;
        // A written `null` under `json` is the document `null`, which is a body,
        // and this call sends none — the bag's own `Const::NeverWritten`.
        args[JSON] = Value::unset();
        args[REQUEST_BAG.verify()] = Value::bool(false);
        let refused = super::request(&mut ctx, &args, "get", "GET")
            .expect_err("`tlsVerify: false` under a deployment that granted no `tls.insecure`");
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the reference `NvsStr::new` just \
                      produced, and a native member never releases an argument \
                      its caller still owns"
        )]
        unsafe {
            url.release();
        }

        let Fault::Thrown(class, message) = refused else {
            panic!("a relaxing option nobody granted is catchable, and names the grant")
        };
        assert_eq!(class, ThrownClass::Logic, "{message}");
        assert!(
            message.contains("tls.insecure") && message.contains(TLS_VERIFY_OPTION),
            "the refusal names the grant an operator would write and the option that wanted it: \
             {message}"
        );

        match listener.accept() {
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Ok(_) => panic!("the grant was missing and a socket was opened to the host anyway"),
            Err(err) => panic!("the listener failed for a reason that is not the point: {err}"),
        }
    }

    /// The scope half of the same rule: a grant is a list of hosts, so one that
    /// names another host relaxes nothing here. Each option is asked against
    /// its own grant, both ways round — the grant naming somewhere else
    /// refuses, and the same grant naming this host lets the call through — so
    /// a pairing that had drifted would show up as one option unlocked by
    /// another's grant.
    #[test]
    fn a_tls_grant_for_another_host_relaxes_nothing() {
        const MEMBER: &str = "Core\\Http\\Client::get";
        const URL: &str = "https://127.0.0.1:8443/ok";

        let anchors = Value::str(NvsStr::new(b"-----BEGIN CERTIFICATE-----"));
        let pins = Value::str(NvsStr::new(b"sha256//ZDk="));
        for (slot, value, grant, key) in [
            (REQUEST_BAG.ca(), anchors, "anchors", "tls.anchors"),
            (REQUEST_BAG.pin(), pins, "pin", "tls.pin"),
            (
                REQUEST_BAG.verify_host(),
                Value::bool(false),
                "any_name",
                "tls.any_name",
            ),
            (
                REQUEST_BAG.verify(),
                Value::bool(false),
                "insecure",
                "tls.insecure",
            ),
        ] {
            let mut args = [Value::null(); REQUEST_ARITY];
            args[slot] = value;

            let mut elsewhere = Ctx::buffered();
            elsewhere.set_config(granting(&format!(
                "{REACHABLE}[capabilities.tls]\n{grant} = [\"api.example.com\"]\n"
            )));
            let refused = super::judge_trust(&elsewhere, &args, URL, MEMBER, super::REQUEST_BAG)
                .expect_err("the grant names a host this call is not made to");
            assert!(
                format!("{refused:?}").contains(key),
                "the refusal names the grant that would have unlocked it: {refused:?}"
            );

            let mut named = Ctx::buffered();
            named.set_config(granting(&format!(
                "{REACHABLE}[capabilities.tls]\n{grant} = [\"127.0.0.1\"]\n"
            )));
            super::judge_trust(&named, &args, URL, MEMBER, super::REQUEST_BAG)
                .expect("the deployment named this host under this option's own grant");
        }
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the two references `NvsStr::new` \
                      produced, and `judge_trust` reads an argument without \
                      taking one"
        )]
        unsafe {
            anchors.release();
            pins.release();
        }
    }

    /// The key of the group that needs no grant, and the one direction it may
    /// move: a call may raise the floor and never lower it. The unconfigured
    /// deployment is `1.2`, so the same `tlsMinVersion` that is fine there is
    /// refused under an operator who raised it — and a version this client does
    /// not implement is refused wherever it is written.
    #[test]
    fn tls_min_version_below_the_floor_throws() {
        const MEMBER: &str = "Core\\Http\\Client::get";
        const URL: &str = "https://127.0.0.1:8443/ok";

        let asked = Value::str(NvsStr::new(b"1.2"));
        let ancient = Value::str(NvsStr::new(b"1.0"));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[REQUEST_BAG.min_version()] = asked;

        let mut shipped = Ctx::buffered();
        shipped.set_config(granting(REACHABLE));
        super::judge_trust(&shipped, &args, URL, MEMBER, super::REQUEST_BAG)
            .expect("`1.2` is the floor a deployment that set none speaks over");

        let mut raised = Ctx::buffered();
        raised.set_config(granting(&format!(
            "{REACHABLE}[http.client.tls]\nmin_version = \"1.3\"\n"
        )));
        let refused = super::judge_trust(&raised, &args, URL, MEMBER, super::REQUEST_BAG)
            .expect_err("`1.2` is under the floor this operator set");
        let Fault::Thrown(class, message) = refused else {
            panic!("a floor the program asked for and cannot have is catchable")
        };
        assert_eq!(class, ThrownClass::Logic, "{message}");
        assert!(
            message.contains(TLS_MIN_VERSION_OPTION) && message.contains("1.3"),
            "the refusal names the key and the floor it is under: {message}"
        );

        args[REQUEST_BAG.min_version()] = ancient;
        assert!(
            super::judge_trust(&shipped, &args, URL, MEMBER, super::REQUEST_BAG).is_err(),
            "this client implements neither TLS 1.0 nor 1.1, so `1.0` is not a floor to ask for"
        );
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the two references `NvsStr::new` \
                      produced, and `judge_trust` reads an argument without \
                      taking one"
        )]
        unsafe {
            asked.release();
            ancient.release();
        }
    }

    /// What a deployment writes to let a call steer itself: the URL's host in
    /// both `net.connect` and `net.connect_to`, and the address it will be
    /// steered to excepted from the denied ranges.
    const STEERABLE: &str = "[capabilities.net]\nconnect = [\"api.example.invalid\"]\nconnect_to = \
                             [\"api.example.invalid\"]\ninternal = [\"127.0.0.1\"]\n";

    /// `rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`'s
    /// two halves in one call: the address connected to is the one the option
    /// named, and the URL is handed on untouched, so the certificate is still
    /// checked against the host it wrote. That host resolves nowhere, which is
    /// what the first half rests on — an approval that had gone through the
    /// resolver rather than through the option could not have answered at all.
    ///
    /// The second half is the same call under a deployment that granted another
    /// host. Naming an address is not a way around `net.connect`, so a program
    /// cannot reach a host nobody granted by writing its address down.
    #[test]
    fn connect_to_connects_to_the_named_address_and_checks_the_urls_host() {
        const MEMBER: &str = "Core\\Http\\Client::get";
        const URL: &str = "http://api.example.invalid:8080/ok";

        let url = Value::str(NvsStr::new(URL.as_bytes()));
        let named = Value::str(NvsStr::new(b"127.0.0.1"));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = url;
        args[CONNECT_TO] = named;

        let mut granted = Ctx::buffered();
        granted.set_config(granting(STEERABLE));
        let (sent, addresses) = super::approved(&mut granted, &args, MEMBER, super::REQUEST_BAG)
            .expect("the deployment named this host under both grants and excepted the address");
        assert_eq!(
            sent, URL,
            "the URL is unchanged, so the handshake checks the name it wrote"
        );
        assert_eq!(
            addresses,
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
            "an address the call named is the set of one it reaches no resolver to grow"
        );

        let mut elsewhere = Ctx::buffered();
        elsewhere.set_config(granting(
            "[capabilities.net]\nconnect = [\"other.example.invalid\"]\nconnect_to = \
             [\"other.example.invalid\"]\ninternal = [\"127.0.0.1\"]\n",
        ));
        let refused = super::approved(&mut elsewhere, &args, MEMBER, super::REQUEST_BAG)
            .expect_err("`connectTo` is not a way around the grant over the URL's own host");
        assert!(
            format!("{refused:?}").contains("api.example.invalid"),
            "the refusal names the host the URL wrote: {refused:?}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the two references `NvsStr::new` \
                      produced, and `approved` reads an argument without taking \
                      one"
        )]
        unsafe {
            url.release();
            named.release();
        }
    }

    /// What a deployment writes when only its proxy can resolve a destination:
    /// the URL's host granted, and `resolve = "proxy"` beside the proxy's own
    /// address.
    const AT_THE_PROXY: &str = "[capabilities.net]\nconnect = [\"api.example.invalid\"]\n\n\
                                [http.client.proxy]\nurl = \"http://proxy.internal:3128\"\n\
                                resolve = \"proxy\"\n";

    /// `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`'s
    /// narrowing, and its edges: under `resolve = "proxy"` the door approves a
    /// host it never looked up, and everything the URL's own text can be judged
    /// on is judged exactly as it is for a direct call.
    ///
    /// `api.example.invalid` resolves nowhere, which is what carries the first
    /// half: an approval that had reached the resolver could not have answered
    /// at all, and the empty set it answers with is what sends the host name to
    /// the proxy. The same URL under `local` is the control — there the address
    /// question is asked, and this host cannot answer it.
    #[test]
    fn resolve_proxy_sends_the_host_name_and_skips_only_the_address_check() {
        const MEMBER: &str = "Core\\Http\\Client::get";
        const URL: &str = "https://api.example.invalid/ok";

        let url = Value::str(NvsStr::new(URL.as_bytes()));
        let elsewhere = Value::str(NvsStr::new(b"ftp://api.example.invalid/ok"));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = url;

        let mut proxied = Ctx::buffered();
        proxied.set_config(granting(AT_THE_PROXY));
        let (sent, addresses) = super::approved(&mut proxied, &args, MEMBER, super::REQUEST_BAG)
            .expect("the proxy resolves this destination, so nothing here had to");
        assert_eq!(
            sent, URL,
            "the URL is handed on untouched, and its host is what `CONNECT` carries"
        );
        assert!(
            addresses.is_empty(),
            "no address was learned for a host this deployment does not resolve: {addresses:?}"
        );

        let mut ungranted = Ctx::buffered();
        ungranted.set_config(granting(
            "[capabilities.net]\nconnect = [\"other.example.invalid\"]\n\n\
             [http.client.proxy]\nurl = \"http://proxy.internal:3128\"\nresolve = \"proxy\"\n",
        ));
        let refused = super::approved(&mut ungranted, &args, MEMBER, super::REQUEST_BAG)
            .expect_err("the grant's host list is asked of a proxied call like any other");
        assert!(
            format!("{refused:?}").contains("api.example.invalid"),
            "the refusal names the host the URL wrote: {refused:?}"
        );

        args[0] = elsewhere;
        let outside = super::approved(&mut proxied, &args, MEMBER, super::REQUEST_BAG)
            .expect_err("the scheme roster is a statement about the text, not about the network");
        assert!(
            format!("{outside:?}").contains("scheme"),
            "a scheme outside the roster is refused under a proxy too: {outside:?}"
        );

        args[0] = url;
        let mut locally = Ctx::buffered();
        locally.set_config(granting(
            "[capabilities.net]\nconnect = [\"api.example.invalid\"]\n\n\
             [http.client.proxy]\nurl = \"http://proxy.internal:3128\"\nresolve = \"local\"\n",
        ));
        super::approved(&mut locally, &args, MEMBER, super::REQUEST_BAG)
            .expect_err("`local` keeps the pin, and this host resolves to no address to pin");

        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the two references `NvsStr::new` \
                      produced, and `approved` reads an argument without taking \
                      one"
        )]
        unsafe {
            url.release();
            elsewhere.release();
        }
    }

    /// The launderer under the same word: `Core\Http::allowUrl` asks the door
    /// every call door asks, so `resolve = "proxy"` does not leave `tainted` on
    /// every URL in the one network the word exists for
    /// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    ///
    /// What it answers with is read back through [`super::approved`], because an
    /// empty set is what a call depends on telling apart from a slot this crate
    /// did not write: one is the approval of a host nothing here resolved, and
    /// the other is unreadable and fatal.
    #[test]
    fn a_target_laundered_under_resolve_proxy_carries_the_approval_and_no_address() {
        const LAUNDERER: &str = "Core\\Http::allowUrl";
        const MEMBER: &str = "Core\\Http\\Client::get";
        const URL: &str = "https://api.example.invalid/ok";

        let mut proxied = Ctx::buffered();
        proxied.set_config(granting(AT_THE_PROXY));
        let pinned = super::pin(&mut proxied, URL, LAUNDERER, super::Roster::Laundered)
            .expect("the proxy resolves this destination, so the launderer had nothing to look up");
        assert!(
            pinned.is_empty(),
            "an address was learned for a host this deployment cannot resolve: {pinned:?}"
        );

        // The value the launderer builds out of that answer, read back by the
        // door a call carrying it goes through.
        let mut empty = NvsArray::new();
        for address in &pinned {
            empty.append(Value::str(NvsStr::new(address.to_string().as_bytes())));
        }
        let target = crate::instance::build(
            &TARGET,
            [Value::str(NvsStr::new(URL.as_bytes())), Value::array(empty)],
        );
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = target;
        let (sent, addresses) = super::approved(&mut proxied, &args, MEMBER, super::REQUEST_BAG)
            .expect("a target with no address in it is an approval, not an unreadable slot");
        assert_eq!(
            sent, URL,
            "the URL is read back out of the target it was laundered into"
        );
        assert!(
            addresses.is_empty(),
            "the door answered with an address the launderer never had: {addresses:?}"
        );

        // A slot another writer mangled is still the fatal it was: the empty set
        // is the one thing that stopped being folded in with it.
        let mut mangled = NvsArray::new();
        mangled.append(Value::int(443));
        let wrong = crate::instance::build(
            &TARGET,
            [
                Value::str(NvsStr::new(URL.as_bytes())),
                Value::array(mangled),
            ],
        );
        args[0] = wrong;
        let unreadable = super::approved(&mut proxied, &args, MEMBER, super::REQUEST_BAG)
            .expect_err("an entry that is not an address is a target this crate did not write");
        assert!(
            matches!(unreadable, Fault::Fatal(_)),
            "a mangled slot is a fatal and not something a program catches: {unreadable:?}"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the two objects `instance::build` \
                      produced, and `approved` reads an argument without taking \
                      one"
        )]
        unsafe {
            target.release();
            wrong.release();
        }
    }

    /// No spelling of the ambient proxy variables is read anywhere on the
    /// outbound path: the block an operator wrote is the only way a call is
    /// proxied (`rule:http-server/an-outbound-proxy-is-operator-configured`).
    ///
    /// Asserted over the source rather than by setting one, because the failure
    /// this guards against is a future reader being *added* — and an
    /// environment a case sets is process-wide state every other case on the
    /// core would then share (`rule:security/no-cross-request-state`). Each
    /// file is scanned down to its first `#[cfg(test)]` and with its comments
    /// dropped, so this case's own prose is not what it reads.
    #[test]
    fn no_proxy_environment_variable_is_ever_read() {
        let shipped = |source: &'static str| {
            source
                .lines()
                .take_while(|line| line.trim_start() != "#[cfg(test)]")
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        for (module, source) in [
            ("http.rs", shipped(include_str!("http.rs"))),
            (
                "http/transport.rs",
                shipped(include_str!("http/transport.rs")),
            ),
        ] {
            for spelling in ["HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "ALL_PROXY"] {
                assert!(
                    !source.contains(spelling),
                    "`{module}` names `{spelling}`; where a call goes is the operator's \
                     `[http.client.proxy]` block and nothing else"
                );
            }
            assert!(
                !source.contains("env::var"),
                "`{module}` reads the environment, which is ambient state shared by every \
                 request in this process"
            );
        }
    }

    /// The address policy is asked of a named address exactly as of a resolved
    /// one, and the refusal lands where it has to: before the socket. A
    /// listener is bound on the address the option names, the deployment grants
    /// the host under both keys and excepts nothing, and the assertion is that
    /// nothing was ever accepted.
    #[test]
    fn connect_to_an_address_the_policy_denies_throws_before_connecting() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        listener
            .set_nonblocking(true)
            .expect("a listener that answers now rather than waiting");
        let at = listener.local_addr().expect("its own address");

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(
            "[capabilities.net]\nconnect = [\"api.example.invalid\"]\nconnect_to = \
             [\"api.example.invalid\"]\n",
        ));

        let url = Value::str(NvsStr::new(
            format!("http://api.example.invalid:{}/ok", at.port()).as_bytes(),
        ));
        let named = Value::str(NvsStr::new(b"127.0.0.1"));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = url;
        // A written `null` under `json` is the document `null`, which is a body,
        // and this call sends none — the bag's own `Const::NeverWritten`.
        args[JSON] = Value::unset();
        args[CONNECT_TO] = named;
        let refused = super::request(&mut ctx, &args, "get", "GET")
            .expect_err("loopback, which this deployment's `net.internal` does not except");
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the two references `NvsStr::new` \
                      just produced, and a native member never releases an \
                      argument its caller still owns"
        )]
        unsafe {
            url.release();
            named.release();
        }
        assert!(
            format!("{refused:?}").contains("net.internal"),
            "the refusal names the exception an operator would have to write: {refused:?}"
        );

        match listener.accept() {
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Ok(_) => panic!("the policy denied the address and a socket was opened to it anyway"),
            Err(err) => panic!("the listener failed for a reason that is not the point: {err}"),
        }
    }

    /// A `Core\Http\Target` carries the address its laundering approved, so a
    /// call that also names one has written two answers to one question. A
    /// `LogicError` rather than a silent override in either direction: the
    /// program has not said which of them it meant, and picking one for it is
    /// how a pin gets stepped around.
    #[test]
    fn connect_to_beside_a_target_is_a_logic_error() {
        const MEMBER: &str = "Core\\Http\\Client::get";

        let mut addresses = NvsArray::new();
        addresses.append(Value::str(NvsStr::new(b"127.0.0.1")));
        let target = crate::instance::build(
            &TARGET,
            [
                Value::str(NvsStr::new(b"http://api.example.invalid:8080/ok")),
                Value::array(addresses),
            ],
        );
        let named = Value::str(NvsStr::new(b"127.0.0.2"));
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = target;
        args[CONNECT_TO] = named;

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(STEERABLE));
        let refused = super::approved(&mut ctx, &args, MEMBER, super::REQUEST_BAG)
            .expect_err("a pinned target and a named address are two answers to one question");
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the target it built and the \
                      reference `NvsStr::new` produced, and `approved` reads an \
                      argument without taking one"
        )]
        unsafe {
            target.release();
            named.release();
        }

        let Fault::Thrown(class, message) = refused else {
            panic!("a call that wrote both is a mistake in the program, and catchable")
        };
        assert_eq!(class, ThrownClass::Logic, "{message}");
        assert!(
            message.contains(CONNECT_TO_OPTION) && message.contains("Target"),
            "the refusal names the option and the value it cannot sit beside: {message}"
        );
    }

    /// `rule:http-server/an-outbound-call-tries-every-approved-address`: what
    /// the launderer approved is a **set**, and what reaches the transport is
    /// that set whole, in the order the resolver answered it. A target read
    /// back as its first address alone would leave the fallback nothing to fall
    /// back across, and the order is the rule's as well — RFC 8305 interleaves
    /// the families from the resolver's first answer onwards.
    ///
    /// Read back rather than resolved, which is the other half: the addresses
    /// below answer no name, so a call reaching this target through anything
    /// but its slots would refuse or connect elsewhere.
    #[test]
    fn a_target_hands_the_whole_approved_set_to_the_call() {
        const MEMBER: &str = "Core\\Http\\Client::get";
        const URL: &str = "http://api.example.invalid/ok";

        let mut addresses = NvsArray::new();
        for text in [
            b"198.51.100.7".as_slice(),
            b"203.0.113.9".as_slice(),
            b"192.0.2.4".as_slice(),
        ] {
            addresses.append(Value::str(NvsStr::new(text)));
        }
        let target = crate::instance::build(
            &TARGET,
            [
                Value::str(NvsStr::new(URL.as_bytes())),
                Value::array(addresses),
            ],
        );
        let mut args = [Value::null(); REQUEST_ARITY];
        args[0] = target;

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(STEERABLE));
        let read = super::approved(&mut ctx, &args, MEMBER, super::REQUEST_BAG);
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the target it built, and \
                      `approved` reads an argument without taking one"
        )]
        unsafe {
            target.release();
        }

        let (url, approved) = read.expect("a target the launderer built is read back");
        assert_eq!(url, URL);
        assert_eq!(
            approved,
            vec![
                IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7)),
                IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9)),
                IpAddr::V4(Ipv4Addr::new(192, 0, 2, 4)),
            ],
            "every approved address reaches the call, in the resolver's order"
        );
    }

    /// [`super::leaf_of`]'s two refusals, which no conformance case can reach:
    /// a `.nvst` case answers every outbound call from a table
    /// (`rule:testing/an-outbound-call-is-answered-from-a-table`) and a table
    /// has no session, so no case ever holds a `Core\Http\TlsInfo` to ask.
    ///
    /// Both spellings of "nothing to read", because they are two different
    /// mistakes at the other end — a peer that sent no certificate at all, and
    /// one whose leaf is not something an X.509 parser accepts — and a member
    /// that collapsed them would leave an operator reading the wrong log.
    #[test]
    fn a_leaf_that_is_absent_or_unreadable_is_refused() {
        let absent = super::leaf_of(&[], "subject").expect_err("a chain with no leaf in it");
        let unreadable = super::leaf_of(&[vec![0x30, 0x00]], "issuer")
            .expect_err("two bytes of DER are not a certificate");

        for (refused, expected) in [
            (absent, "presented no certificate"),
            (unreadable, "cannot read"),
        ] {
            let Fault::Thrown(class, message) = refused else {
                panic!("what the peer sent is not the program's mistake, and is catchable")
            };
            assert_eq!(class, ThrownClass::Runtime, "{message}");
            assert!(
                message.contains(expected),
                "the refusal says which of the two happened: {message}"
            );
        }
    }
}
