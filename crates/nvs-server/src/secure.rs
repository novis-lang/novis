//! `rule:http-server/secure-headers-with-nothing-written`'s secure header set: what every response this server writes carries
//! when nothing is configured.
//!
//! § 1 is a table of shipped defaults rather than a feature — `nosniff`, `frame-ancestors
//! 'none'`, `strict-origin-when-cross-origin`, and HSTS on a `https` effective scheme — and
//! the whole of the decision is that **an operator who wrote nothing gets all of them**.
//! [`Secure::of`] reads `[http.headers]` once at boot and renders the set into header lines;
//! [`Secure::fill`] is what one response costs, which is four name lookups and four
//! `HeaderValue` clones — a refcount bump each, no allocation and no re-parsing.
//!
//! # Decision: the set *fills*, it does not overwrite, and that is what makes `setHeader` an override
//!
//! Spec § 15 gives a program an override of a policy-owned header on one response and
//! `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins` is why the policy is not narrowing-only. Written as an insert, that would be
//! an ordering rule — every policy header applied before the program's, on every path a
//! response can leave by — and a path that got the order wrong would silently take the
//! member's effect away on exactly the headers it exists to change.
//!
//! So [`Secure::fill`] fills a name the response does not already carry and leaves one it
//! does. Ordering then stops being load-bearing: the policy is applied once, at the single
//! point in [`crate::serve::serve_connection`] every response passes through — a program's, a
//! mount table's `404`, a static file's, [`crate::admit::over_capacity`]'s `503` — and what
//! `Core\Response::setHeader` wrote is what the peer reads because it is already there.
//! `Content-Type` is untouched for the same reason and is never in this set: a body member
//! owns its own media type (`rule:security/response-body-is-one-typed-member`).
//!
//! # Decision: the three details § 1 calls decisions rather than transcription
//!
//! **HSTS is emitted on an `https` effective scheme and not otherwise**, which is [`Scheme`]:
//! a parameter of [`Secure::fill`] rather than a fact read off the socket, because Novis never
//! terminates TLS (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`) and the only thing that can assert `https` is a *trusted*
//! proxy's `X-Forwarded-Proto` (0097 § 6). That walk is [`crate::forwarded`] and
//! [`crate::serve::serve_connection`] is the one call site that may pass [`Scheme::Https`] —
//! it does so for a request whose peer is in `[server] trusted_proxies` and that said so.
//! With that directive empty, which is its default, no forwarded header is read at all and
//! every request this server serves is [`Scheme::Http`] with no HSTS sent — § 1's own answer
//! for an unconfigured tree, and not a gap.
//!
//! **`hsts_subdomains` defaults to `false`**, so `includeSubDomains` is absent unless it is
//! written. § 1's reason is that it is the HSTS setting that has taken deployments down, for a
//! year, with no way to revoke it from a client.
//!
//! **There is no default `default-src`**, so with nothing written the `Content-Security-Policy`
//! is `frame-ancestors 'none'` alone. A written `content_security_policy` and the resolved
//! `frame_ancestors` are **one header**, the written policy first and `frame-ancestors` last:
//! they are directives of the same header and two of them would have the second ignored. A
//! policy that writes its own `frame-ancestors` therefore sets `frame_ancestors = false`, which
//! is the spelling that drops this crate's directive.
//!
//! # Decision: a written value the wire cannot carry is read as if it had not been written
//!
//! Every value below `[http.headers]` but two is operator-written text, and a header value is
//! printable ASCII: a `\r\n` in one is a response-splitting attempt, whoever wrote it. Such a
//! value is dropped and the shipped default stands in its place — the whole rule, one sentence,
//! applied to every field. Emitting *nothing* for that header was rejected because it turns a
//! typo in a `Referrer-Policy` into a silently weaker response, which is the direction § 1
//! exists to close.
//!
//! Refusing the **boot** is better still and is not what this module does: a diagnostic
//! naming the line is `nvs-config`'s to raise, this crate has no `nvs-diagnostics` dependency,
//! and the fail-safe reading above is correct with or without it. The backlog owns that slice.
//!

use hyper::HeaderMap;
use hyper::header::{self, HeaderName, HeaderValue};
use nvs_config::tree::{Http, HttpHeaders};
use nvs_config::{Quantity, Setting, Unit};

/// § 1's `referrer_policy` with nothing written.
const REFERRER: &str = "strict-origin-when-cross-origin";

/// § 1's `hsts` with nothing written — `365d`, in the seconds the header is spelled in.
const MAX_AGE: u64 = 365 * 24 * 60 * 60;

/// § 1's `frame_ancestors` with nothing written, as the CSP source it renders to.
const NO_ANCESTOR: &str = "'none'";

/// `Permissions-Policy`, which the `http` crate has no constant for.
const PERMISSIONS_POLICY: HeaderName = HeaderName::from_static("permissions-policy");

/// The scheme a request effectively arrived over — § 1's condition on HSTS, and nothing else
/// in this module reads it.
///
/// **Declared one crate down**, in `nvs_runtime`, and re-exported here under the name every
/// call site in this crate already spells. `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s walk decides it once per request and
/// two things then read that answer: this module, which sends HSTS only over `Https`, and
/// `nvs_runtime::Inbound`, which carries it to `Core\Request::scheme()`. A second enum here
/// would be a second home for one fact and a conversion at the seam between them.
pub use nvs_runtime::Scheme;

/// `rule:http-server/secure-headers-with-nothing-written`'s header set, resolved from `[http.headers]` and rendered once.
///
/// One per published snapshot: `[http.headers]` reloads, so [`crate::serve::Serving`] derives
/// a new set the first time a request is answered under a new snapshot, and every core shares it.
///
#[derive(Clone, Debug)]
pub struct Secure {
    /// Every header that does not depend on the request, in the order § 1 lists them.
    always: Vec<(HeaderName, HeaderValue)>,
    /// `Strict-Transport-Security`, held apart because [`Scheme`] decides whether it is sent.
    hsts: Option<HeaderValue>,
}

/// § 1's shipped set, which is what a tree writing no `[http.headers]` resolves to.
impl Default for Secure {
    fn default() -> Self {
        Self::of(None)
    }
}

impl Secure {
    /// The set in force for a tree, with every absent key at § 1's shipped default.
    ///
    /// Takes the whole `[http]` block rather than `[http.headers]` so that a caller holding a
    /// [`nvs_config::Config`] passes `config.http.as_ref()` and never has to know which of the
    /// five sub-blocks this reads.
    #[must_use]
    pub fn of(http: Option<&Http>) -> Self {
        let written = http.and_then(|http| http.headers.as_ref());
        let mut always = Vec::with_capacity(4);
        // A `bool`, so there is nothing here that can fail to spell.
        if written
            .and_then(|headers| headers.content_type_options)
            .unwrap_or(true)
        {
            always.push((
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ));
        }
        if let Some(policy) = content_security_policy(written) {
            always.push((header::CONTENT_SECURITY_POLICY, policy));
        }
        if let Some(policy) = referrer_policy(written) {
            always.push((header::REFERRER_POLICY, policy));
        }
        if let Some(policy) = permissions_policy(written) {
            always.push((PERMISSIONS_POLICY, policy));
        }
        Self {
            always,
            hsts: hsts(written),
        }
    }

    /// Writes every header of the set that `headers` does not already carry.
    ///
    /// A name already present is left exactly as it is, which is the module doc's first
    /// decision: that is how `Core\Response::setHeader` overrides a policy-owned header, and it
    /// is why this may be applied at one point per response rather than in a fixed order
    /// against every other thing that writes one.
    pub fn fill(&self, headers: &mut HeaderMap, scheme: Scheme) {
        for (name, value) in &self.always {
            headers.entry(name).or_insert_with(|| value.clone());
        }
        // § 1: on a plaintext connection a browser ignores it, so sending it there is noise —
        // and it is a header that cannot be revoked from a client, so the unasserted scheme
        // is the one that must not send it.
        if let (Scheme::Https, Some(hsts)) = (scheme, self.hsts.as_ref()) {
            headers
                .entry(header::STRICT_TRANSPORT_SECURITY)
                .or_insert_with(|| hsts.clone());
        }
    }
}

/// `written` as a header value, falling back to `shipped` where the wire cannot carry it —
/// the module doc's third decision, in the one place it is applied.
fn spell(written: &str, shipped: Option<&str>) -> Option<HeaderValue> {
    HeaderValue::from_str(written)
        .ok()
        .or_else(|| shipped.and_then(|shipped| HeaderValue::from_str(shipped).ok()))
}

/// The written policy and the resolved `frame-ancestors`, as the one header they are.
fn content_security_policy(written: Option<&HttpHeaders>) -> Option<HeaderValue> {
    let ancestors = frame_ancestors(written);
    let policy = written
        .and_then(|headers| headers.content_security_policy.as_deref())
        .unwrap_or_default()
        .trim();
    let whole = match (policy.is_empty(), ancestors.as_deref()) {
        // § 1's "empty: nothing emitted beyond frame-ancestors", with the frame-ancestors
        // directive itself turned off: there is no policy left to send.
        (true, None) => return None,
        (true, Some(ancestors)) => ancestors.to_string(),
        (false, None) => policy.to_string(),
        (false, Some(ancestors)) => format!("{policy}; {ancestors}"),
    };
    spell(&whole, ancestors.as_deref())
}

/// § 1's `frame_ancestors` as a CSP directive, and [`None`] for the `false` that drops it.
fn frame_ancestors(written: Option<&HttpHeaders>) -> Option<String> {
    let sources = match written.and_then(|headers| headers.frame_ancestors.as_ref()) {
        None => NO_ANCESTOR.to_string(),
        Some(Setting::Bool(false)) => return None,
        Some(Setting::Text(text)) => source(text),
        Some(Setting::List(origins)) if !origins.is_empty() => origins
            .iter()
            .map(|origin| source(origin))
            .collect::<Vec<_>>()
            .join(" "),
        // A number, a `true`, an empty list: none of them names a source, so the shipped
        // default stands rather than a policy nobody wrote.
        Some(_) => NO_ANCESTOR.to_string(),
    };
    Some(format!("frame-ancestors {sources}"))
}

/// One `frame_ancestors` entry as CSP spells it: § 1's two keywords quoted, an origin as
/// written.
///
/// Case-insensitive, because a browser reads the directive that way and a tree writing `None`
/// has configured the same policy.
fn source(written: &str) -> String {
    let written = written.trim();
    if written.is_empty() || written.eq_ignore_ascii_case("none") {
        return NO_ANCESTOR.to_string();
    }
    if written.eq_ignore_ascii_case("self") {
        return "'self'".to_string();
    }
    written.to_string()
}

/// § 1's `referrer_policy`, and [`None`] for the empty string that turns it off.
fn referrer_policy(written: Option<&HttpHeaders>) -> Option<HeaderValue> {
    match written.and_then(|headers| headers.referrer_policy.as_deref()) {
        None => Some(HeaderValue::from_static(REFERRER)),
        Some(policy) if policy.trim().is_empty() => None,
        Some(policy) => spell(policy.trim(), Some(REFERRER)),
    }
}

/// § 1's `permissions_policy`, whose default is empty — "empty: nothing emitted".
fn permissions_policy(written: Option<&HttpHeaders>) -> Option<HeaderValue> {
    let policy = written
        .and_then(|headers| headers.permissions_policy.as_deref())?
        .trim();
    if policy.is_empty() {
        return None;
    }
    spell(policy, None)
}

/// § 1's `hsts` and `hsts_subdomains` as the one header they spell, and [`None`] for the
/// `false` that disables it.
fn hsts(written: Option<&HttpHeaders>) -> Option<HeaderValue> {
    let seconds = match written.and_then(|headers| headers.hsts.as_ref()) {
        None => MAX_AGE,
        Some(Setting::Bool(false)) => return None,
        // The same reader every other duration in the tree goes through, so `365d`, `8760h`
        // and a bare number of seconds are one value here as they are everywhere else. A
        // magnitude it refuses falls back with the rest.
        Some(setting) => match Quantity::parse("http.headers.hsts", Unit::Duration, setting) {
            Ok(Quantity::Nanos(nanos)) => nanos / 1_000_000_000,
            _ => MAX_AGE,
        },
    };
    let subdomains = written
        .and_then(|headers| headers.hsts_subdomains)
        .unwrap_or(false);
    let value = if subdomains {
        format!("max-age={seconds}; includeSubDomains")
    } else {
        format!("max-age={seconds}")
    };
    HeaderValue::from_str(&value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The set as one response would carry it, so a case reads as the wire does.
    fn filled(secure: &Secure, scheme: Scheme) -> HeaderMap {
        let mut headers = HeaderMap::new();
        secure.fill(&mut headers, scheme);
        headers
    }

    /// One header of a filled map, or `"<absent>"` — a missing header and a wrong one then
    /// fail the same assertion instead of one of them panicking.
    fn line(headers: &HeaderMap, name: &str) -> String {
        headers.get(name).map_or_else(
            || "<absent>".to_string(),
            |value| String::from_utf8_lossy(value.as_bytes()).into_owned(),
        )
    }

    /// `rule:http-server/secure-headers-with-nothing-written`'s whole point: a tree that wrote no `[http.headers]` gets the set anyway.
    #[test]
    fn nothing_configured_is_section_ones_shipped_set() {
        let headers = filled(&Secure::default(), Scheme::Http);
        assert_eq!(line(&headers, "x-content-type-options"), "nosniff");
        assert_eq!(
            line(&headers, "content-security-policy"),
            "frame-ancestors 'none'",
            "§ 1's CSP is frame-ancestors alone — there is no default `default-src`"
        );
        assert_eq!(
            line(&headers, "referrer-policy"),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            line(&headers, "permissions-policy"),
            "<absent>",
            "§ 1's `permissions_policy` defaults to empty, which emits nothing"
        );
    }

    /// § 1's first decision, both sides of it: the same set answers differently for the two
    /// schemes, and the plaintext one — every request this server sees today — sends nothing.
    #[test]
    fn hsts_is_sent_on_an_https_effective_scheme_and_not_on_a_plaintext_one() {
        let secure = Secure::default();
        assert_eq!(
            line(&filled(&secure, Scheme::Http), "strict-transport-security"),
            "<absent>"
        );
        assert_eq!(
            line(&filled(&secure, Scheme::Https), "strict-transport-security"),
            "max-age=31536000",
            "§ 1 ships `365d` and defaults `hsts_subdomains` to false"
        );
    }

    /// The module's first decision, which is what makes `Core\\Response::setHeader` an
    /// override: a header the response already carries is not the policy's to rewrite.
    #[test]
    fn a_header_the_response_already_carries_is_left_alone() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        );
        Secure::default().fill(&mut headers, Scheme::Https);
        assert_eq!(line(&headers, "referrer-policy"), "no-referrer");
        assert_eq!(
            line(&headers, "x-content-type-options"),
            "nosniff",
            "one overridden header does not take the rest of the set with it"
        );
    }

    /// Every field of the block, written: each one is read, and the two whose spelling is a
    /// keyword rather than free text render as CSP spells them.
    #[test]
    fn a_written_block_is_read_field_by_field() {
        let http = Http {
            headers: Some(HttpHeaders {
                content_type_options: Some(false),
                frame_ancestors: Some(Setting::List(vec![
                    "self".to_string(),
                    "https://embed.example".to_string(),
                ])),
                referrer_policy: Some("no-referrer".to_string()),
                hsts: Some(Setting::Text("30d".to_string())),
                hsts_subdomains: Some(true),
                content_security_policy: Some("default-src 'self'".to_string()),
                permissions_policy: Some("geolocation=()".to_string()),
            }),
            ..Http::default()
        };
        let headers = filled(&Secure::of(Some(&http)), Scheme::Https);
        assert_eq!(line(&headers, "x-content-type-options"), "<absent>");
        assert_eq!(
            line(&headers, "content-security-policy"),
            "default-src 'self'; frame-ancestors 'self' https://embed.example",
            "a written policy and the frame-ancestors directive are one header, in that order"
        );
        assert_eq!(line(&headers, "referrer-policy"), "no-referrer");
        assert_eq!(
            line(&headers, "strict-transport-security"),
            "max-age=2592000; includeSubDomains"
        );
        assert_eq!(line(&headers, "permissions-policy"), "geolocation=()");
    }

    /// The two spellings that turn a header off, which are not the same spelling — `false` for
    /// a setting, an empty string for a free-text policy.
    #[test]
    fn false_and_the_empty_string_are_what_drop_a_header() {
        let http = Http {
            headers: Some(HttpHeaders {
                frame_ancestors: Some(Setting::Bool(false)),
                referrer_policy: Some(String::new()),
                hsts: Some(Setting::Bool(false)),
                ..HttpHeaders::default()
            }),
            ..Http::default()
        };
        let headers = filled(&Secure::of(Some(&http)), Scheme::Https);
        assert_eq!(
            line(&headers, "content-security-policy"),
            "<absent>",
            "with no policy written, dropping frame-ancestors leaves no CSP to send"
        );
        assert_eq!(line(&headers, "referrer-policy"), "<absent>");
        assert_eq!(line(&headers, "strict-transport-security"), "<absent>");
        assert_eq!(
            line(&headers, "x-content-type-options"),
            "nosniff",
            "the field nobody wrote is still § 1's"
        );
    }

    /// The module's third decision, on the value most likely to carry one: a `\r\n` in a
    /// written policy is a response-splitting attempt, and what answers is the shipped default
    /// rather than a header the operator's typo silently removed.
    #[test]
    fn a_value_the_wire_cannot_carry_falls_back_to_the_shipped_default() {
        let http = Http {
            headers: Some(HttpHeaders {
                referrer_policy: Some("no-referrer\r\nX-Injected: yes".to_string()),
                content_security_policy: Some("default-src\r\nX-Injected: yes".to_string()),
                ..HttpHeaders::default()
            }),
            ..Http::default()
        };
        let headers = filled(&Secure::of(Some(&http)), Scheme::Http);
        assert_eq!(
            line(&headers, "referrer-policy"),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            line(&headers, "content-security-policy"),
            "frame-ancestors 'none'",
            "the written policy is dropped whole; what is left is what nothing written means"
        );
        assert_eq!(
            headers.get("x-injected"),
            None,
            "nothing a written value spelled reached the map as a header of its own"
        );
    }
}
