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
//! **What it spends:** one `Core\Http\Target` allocation per laundered URL, two slots wide, charged
//! to the request that laundered it — and one synchronous resolution per call, which
//! `pin_host`'s own docs own.

use fluent_uri::UriRef;
use fluent_uri::component::{Authority, Scheme};
use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

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

nvs_runtime::nvs_helper! {
    /// `Core\Http::allowUrl(tainted string $url): Core\Http\Target` — ADR 0058
    /// § 2, and the only spelling that removes `tainted` from an outbound URL.
    ///
    /// The order of the four questions is deliberate. The text is parsed and
    /// its scheme judged first, because both are statements about the argument
    /// and neither tells a caller anything about the deployment. Then
    /// `nvs_runtime::capability::pin_host` asks the capability about the host
    /// and § 3's table about the address it resolves to — in that order, so an
    /// ungranted program cannot use this member as a resolver for names it was
    /// never allowed to reach.
    fn nvs_core_http_allow_url(ctx, args: [1]) {
        let text = text_of(args, "allowUrl")?;

        let reference = UriRef::parse(text).map_err(|_| {
            Fault::thrown(format!(
                "{MEMBER}: this text is not a URL, so there is no host in it to approve"
            ))
        })?;

        // Case-insensitively, because a scheme is: `HTTP://` is the same URL,
        // and a roster compared byte for byte would be one a caller can step
        // around by shouting.
        let scheme = reference.scheme().map(Scheme::as_str).unwrap_or_default();
        if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
            return Err(Fault::thrown(format!(
                "{MEMBER}: the scheme must be `http` or `https`, and this URL names `{scheme}`"
            )));
        }

        let authority = reference.authority().ok_or_else(|| {
            Fault::thrown(format!(
                "{MEMBER}: the URL names no host, so there is nothing to resolve and pin"
            ))
        })?;
        let host = Authority::host(&authority);

        let pinned = nvs_runtime::capability::pin_host(ctx, host, MEMBER)?;

        Ok(crate::instance::build(
            &TARGET,
            [
                Value::str(NvsStr::new(text.as_bytes())),
                Value::str(NvsStr::new(pinned.to_string().as_bytes())),
            ],
        ))
    }
}
