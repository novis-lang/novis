//! `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s forwarded walk: which address the request came from, and
//! whether anything it carried is allowed to say otherwise.
//!
//! Two facts leave this module — the client address and the effective scheme —
//! and both have the same shape of answer: **the socket peer, unless a trusted
//! proxy asserted otherwise**. `[server] trusted_proxies` is the whole of who
//! may assert, it defaults to empty, and empty means the headers are not read
//! at all rather than read and disbelieved. That difference is observable:
//! `Core\Request::header` still returns `X-Forwarded-For` `tainted` to an
//! application that wants to decide for itself, and this module having ignored
//! it is what leaves that decision unmade.
//!
//! # Decision: right-to-left, and the refusal is only where a value would be used
//!
//! The walk starts at the peer and moves **left**, because every entry to the
//! right of the first untrusted one was written by something this deployment
//! vouches for and everything to its left is the untrusted client's own text.
//! Leftmost-wins is fully attacker-controlled and is the version most
//! frameworks shipped first; § 6 names it as the thing this is not.
//!
//! A token the walk **lands on** and cannot read as an IP address is a `400`
//! ([`Unusable`]), because that value was about to become the client address.
//! A token further left is never examined and so never refuses — and an
//! `X-Forwarded-For` from an *untrusted* peer is ignored silently, never
//! refused, which § 6 calls load-bearing: any client can set that header, so
//! refusing on its mere presence would let anyone deny service by sending one.
//!
//! # Decision: a port is not part of the address, and a withheld hop is not a defect
//!
//! Two spellings a *conforming* proxy writes are read rather than refused, and
//! neither is a repair. A token may carry the **port** the hop connected from —
//! `203.0.113.9:54321`, and `[2001:db8::1]:443` for the one form in which a v6
//! address may, its own colons being the separator otherwise — which Azure's
//! front ends and IIS emit by default. Stripping it is § 6's `Host` rule
//! applied to the same kind of value: a port names no different address, so
//! this is `rule:errors/ambiguous-input-refused`'s *accept verbatim* branch. A port that is not a number
//! is **not** accepted, because then the token means nothing definite and
//! reading past it would be inventing the answer.
//!
//! And a hop may say it is **withholding** the address — RFC 7239 § 6.3's
//! `unknown`, which Squid emits with `forwarded_for` off, and its obfuscated
//! `_hidden` identifiers. Landing on one answers `None`: the same answer a
//! Unix-domain peer that forwarded nothing gives, because it is the same fact,
//! and refusing would take down every deployment behind such a proxy over a
//! value it wrote on purpose. Only a *trusted* hop can put one where the walk
//! lands — a client's own `unknown` sits to the left of the address its proxy
//! appended, and is never reached.
//!
//! # Decision: an entry `trusted_proxies` cannot parse is dropped here, and the boot refusal is a backlog slice
//!
//! [`Trusted::of`] hands back the entries it could not read rather than
//! swallowing them, and this crate's callers have nowhere to raise them: a
//! diagnostic naming the line is `nvs-config`'s, this crate has no
//! `nvs-diagnostics` dependency, and [`crate::secure`]'s module doc already
//! settled the same question the same way for `[http.headers]`. Dropping is the
//! fail-safe direction on its own terms — an entry that names no network
//! **shrinks** the trusted set, so a typo makes a proxy untrusted and its
//! headers ignored, never the reverse.
//!
//! **What it spends:** one `Trusted` per server — a `Vec` of one [`Net`], an
//! address and a prefix length, per written entry, resolved at boot and shared
//! by every core — and, per request, one borrowed slice per `X-Forwarded-For`
//! token with no allocation at all. A request from an unproxied deployment
//! reads no header and touches neither.
//!

use std::net::IpAddr;

use hyper::HeaderMap;
use hyper::header::HeaderName;

use crate::secure::Scheme;

/// § 6's only forwarded-address header. RFC 7239's `Forwarded` is deliberately
/// not read: supporting both is what *creates* an ambiguity that would then
/// have to be refused.
const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

/// § 6's scheme assertion, read from a trusted peer and from nobody else.
const X_FORWARDED_PROTO: HeaderName = HeaderName::from_static("x-forwarded-proto");

/// The client-address headers § 6 answers with a `Warn` rather than a parse:
/// RFC 7239's `Forwarded`, nginx's `X-Real-IP`, and the vendor spellings a CDN
/// puts in front of a deployment.
///
/// Reading any of them would make what a proxy may assert depend on which
/// header it happened to write, which is the ambiguity `X-Forwarded-For` being
/// the only one avoids. **Reporting them is the point.** `X-Real-IP` is the
/// common one — `proxy_set_header X-Real-IP $remote_addr;` with no
/// `X-Forwarded-For` beside it is one of the most-copied nginx recipes there
/// is, and under it every client is logged as the proxy with nothing said.
const OTHER_ADDRESS_HEADERS: [HeaderName; 6] = [
    HeaderName::from_static("forwarded"),
    HeaderName::from_static("x-real-ip"),
    HeaderName::from_static("cf-connecting-ip"),
    HeaderName::from_static("true-client-ip"),
    HeaderName::from_static("fastly-client-ip"),
    HeaderName::from_static("x-client-ip"),
];

/// Stands in for a field line whose bytes are not text, so that such a line is
/// one ordinary token that does not parse rather than a second code path. It
/// therefore refuses exactly where any other unreadable token would — when the
/// walk lands on it — and is invisible otherwise.
const NOT_TEXT: &str = "\u{fffd}";

/// How the request reached this process — the half of "who is speaking" that
/// the operating system answered rather than the peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrival {
    /// A TCP connection from this address.
    Tcp(IpAddr),
    /// A Unix-domain socket, which § 6 makes **implicitly trusted**: the OS
    /// enforces who may connect to it. The operator warning that belongs beside
    /// that is in the ADR — a `0660` socket is trusted by *group membership*.
    ///
    /// Carries no address, because a Unix-domain socket's is a path or nothing
    /// and never something a forwarded header could be checked against: what
    /// decided who may connect was the mode on the socket
    /// (`[server] socket_mode`), which is why the connection is trusted at all.
    Unix,
}

impl Arrival {
    /// The peer's address, and `None` for a socket that has no such thing.
    #[must_use]
    pub fn address(self) -> Option<IpAddr> {
        match self {
            Self::Tcp(ip) => Some(canonical(ip)),
            Self::Unix => None,
        }
    }

    /// Whether this peer may assert anything — § 6's first question, asked
    /// before a header is looked at.
    #[must_use]
    fn may_speak(self, trusted: &Trusted) -> bool {
        match self {
            Self::Tcp(ip) => trusted.holds(ip),
            Self::Unix => true,
        }
    }
}

/// One written `trusted_proxies` entry, as an address and a prefix length.
///
/// A bare address is its own full-width prefix, so `10.0.0.7` and
/// `10.0.0.7/32` are the same row and the walk has one comparison rather than
/// two. The base is **not** required to be already masked: `10.1.2.3/8` holds
/// exactly what `10.0.0.0/8` holds, because [`Net::holds`] masks both sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Net {
    base: IpAddr,
    prefix: u8,
}

impl Net {
    /// One written entry, or `None` for text that names no network.
    fn of(written: &str) -> Option<Self> {
        let written = written.trim();
        let (addr, len) = match written.split_once('/') {
            Some((addr, len)) => (addr, Some(len)),
            None => (written, None),
        };
        let base = canonical(addr.parse::<IpAddr>().ok()?);
        let width = if base.is_ipv4() { 32 } else { 128 };
        let prefix = match len {
            None => width,
            Some(len) => match len.parse::<u8>() {
                Ok(prefix) if prefix <= width => prefix,
                _ => return None,
            },
        };
        Some(Self { base, prefix })
    }

    /// Whether `ip` falls inside this network.
    ///
    /// A v4-mapped v6 address has already been folded to v4 on both sides, so
    /// a proxy reaching a dual-stack listener as `::ffff:10.0.0.7` matches the
    /// `10.0.0.0/8` the operator wrote — the two spellings are one address and
    /// making the operator write both would be the repair
    /// `rule:errors/ambiguous-input-refused` forbids in the other direction.
    ///
    fn holds(self, ip: IpAddr) -> bool {
        match (self.base, canonical(ip)) {
            (IpAddr::V4(base), IpAddr::V4(ip)) => {
                masked(u128::from(u32::from(base)), 32 - self.prefix)
                    == masked(u128::from(u32::from(ip)), 32 - self.prefix)
            }
            (IpAddr::V6(base), IpAddr::V6(ip)) => {
                masked(u128::from(base), 128 - self.prefix)
                    == masked(u128::from(ip), 128 - self.prefix)
            }
            _ => false,
        }
    }
}

/// `bits` with its low `host` bits cleared, and `host == 128` clearing all of
/// them — which a bare shift would not, being undefined for the full width.
fn masked(bits: u128, host: u8) -> u128 {
    if host >= 128 {
        0
    } else {
        bits & (u128::MAX << host)
    }
}

/// An address as its one canonical spelling: a v4-mapped v6 address is the v4
/// address it maps, and everything else is itself.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        IpAddr::V4(v4) => IpAddr::V4(v4),
    }
}

/// `[server] trusted_proxies`, resolved: the set of addresses whose forwarded
/// headers this server reads.
///
/// **Empty is the default and it is not "trust nobody" but "read nothing"** —
/// § 6's first line, and the whole difference between a deployment that forgot
/// the directive and one that has no proxy.
#[derive(Clone, Debug, Default)]
pub struct Trusted {
    nets: Vec<Net>,
}

impl Trusted {
    /// The written entries, resolved, and the ones that named no network.
    ///
    /// The rejected entries are returned rather than dropped here so that a
    /// caller with somewhere to raise them can; this crate has none, which the
    /// module doc owns.
    #[must_use]
    pub fn of(written: &[String]) -> (Self, Vec<String>) {
        let mut nets = Vec::with_capacity(written.len());
        let mut rejected = Vec::new();
        for entry in written {
            match Net::of(entry) {
                Some(net) => nets.push(net),
                None => rejected.push(entry.clone()),
            }
        }
        (Self { nets }, rejected)
    }

    /// Nothing written — the default, under which no forwarded header is read.
    #[must_use]
    pub fn none() -> Self {
        Self { nets: Vec::new() }
    }

    /// Whether `[server] trusted_proxies` named anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nets.is_empty()
    }

    /// Whether this address is one this deployment vouches for.
    #[must_use]
    fn holds(&self, ip: IpAddr) -> bool {
        self.nets.iter().any(|net| net.holds(ip))
    }
}

/// What the walk answers: the facts a request arrives with once § 6 has
/// decided who was allowed to state them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Origin {
    client: Option<IpAddr>,
    scheme: Scheme,
    ignored_address_header: bool,
}

impl Origin {
    /// The client address, and `None` for the ways a request can have none:
    /// a Unix-domain peer that forwarded nothing, and a trusted hop that
    /// withheld it.
    #[must_use]
    pub fn client(self) -> Option<IpAddr> {
        self.client
    }

    /// The effective scheme, which feeds HSTS emission ([`crate::secure`]) and
    /// `Core\Request::scheme()`, and nothing else. § 6 states the rest of
    /// that list as absences — it does not feed redirects, and it does not feed
    /// the cookie `Secure` flag.
    #[must_use]
    pub fn scheme(self) -> Scheme {
        self.scheme
    }

    /// A trusted peer sent one of [`OTHER_ADDRESS_HEADERS`] and no
    /// `X-Forwarded-For`: § 6's one `Warn`, reported by the caller because this
    /// module has no log.
    ///
    /// The request is served either way, with the peer as the client. Refusing
    /// would take a site down over a header Novis chose not to support — and
    /// staying *quiet* is the failure this answer exists to close, because a
    /// deployment whose proxy writes only `X-Real-IP` is otherwise wrong about
    /// every client and told nothing.
    ///
    /// An untrusted peer's headers are not read at all, so they are never
    /// reported: any client can send these, and a `Warn` per request would be a
    /// log flood anyone could turn on.
    #[must_use]
    pub fn ignored_address_header(self) -> bool {
        self.ignored_address_header
    }
}

/// The walk landed on a token that is not an IP address — § 6's `400`, and the
/// only refusal in this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unusable;

/// § 6's walk: the client address and the effective scheme for one request.
///
/// # Errors
///
/// [`Unusable`] when the trusted walk stops on an `X-Forwarded-For` token that
/// does not parse as an IP address — the value was about to be used and cannot
/// be, which is a `400`.
pub fn walk(arrival: Arrival, trusted: &Trusted, headers: &HeaderMap) -> Result<Origin, Unusable> {
    let peer = arrival.address();
    // Both of these answer without reading a header: an empty directive means
    // the headers are not parsed at all, and an untrusted peer's are ignored
    // silently. The two arms differ in the ADR and not in the code, and that is
    // the point — the second must not become a refusal.
    if trusted.is_empty() || !arrival.may_speak(trusted) {
        return Ok(Origin {
            client: peer,
            scheme: Scheme::Http,
            ignored_address_header: false,
        });
    }
    let chain = chain(headers);
    let client = if chain.is_empty() {
        peer
    } else {
        untrusted_in(&chain, trusted)?
    };
    Ok(Origin {
        client,
        scheme: asserted(headers),
        ignored_address_header: chain.is_empty() && named_elsewhere(headers),
    })
}

/// Whether a header this module does not read carries a client address.
fn named_elsewhere(headers: &HeaderMap) -> bool {
    OTHER_ADDRESS_HEADERS
        .iter()
        .any(|name| headers.contains_key(name))
}

/// Every `X-Forwarded-For` token, in the order the peer wrote them.
///
/// Several field lines are **joined** rather than refused: RFC 7230 § 3.2.2
/// permits combining a list-valued field, so every conforming reader agrees on
/// what this means and § 6 calls it *unusual* rather than *ambiguous*. Empty
/// tokens are dropped, because a trailing comma is punctuation and not a hop.
fn chain(headers: &HeaderMap) -> Vec<&str> {
    let mut tokens = Vec::new();
    for line in headers.get_all(X_FORWARDED_FOR) {
        let line = line.to_str().unwrap_or(NOT_TEXT);
        tokens.extend(line.split(',').map(str::trim).filter(|hop| !hop.is_empty()));
    }
    tokens
}

/// The rightmost entry that is not itself trusted, walking left from the peer,
/// and `None` where that walk lands on a hop that withheld the address.
///
/// A chain whose every entry is trusted answers with its **leftmost** one:
/// there is no untrusted hop to find, and the address the chain says the
/// request started at is the honest remaining answer. Falling back to the peer
/// instead would report the nearest proxy as the client, which is the failure
/// § 6's `Warn` exists to catch elsewhere.
fn untrusted_in(chain: &[&str], trusted: &Trusted) -> Result<Option<IpAddr>, Unusable> {
    let mut leftmost = None;
    for token in chain.iter().rev() {
        match hop(token) {
            Hop::At(ip) if trusted.holds(ip) => leftmost = Some(ip),
            Hop::At(ip) => return Ok(Some(ip)),
            Hop::Withheld => return Ok(None),
            Hop::Unreadable => return Err(Unusable),
        }
    }
    // Unreachable with a non-empty chain, which is the only thing `walk` calls
    // this with: the first token either answered or set this.
    Ok(leftmost)
}

/// What one `X-Forwarded-For` token names.
enum Hop {
    /// An address, canonical and with any port already off it.
    At(IpAddr),
    /// A hop that says it is naming none.
    Withheld,
    /// Text that is neither, and so a `400` where the walk lands on it.
    Unreadable,
}

/// One token, read. The module doc owns why the middle case is not a refusal.
fn hop(token: &str) -> Hop {
    // RFC 7239 § 6.3's two ways of saying "not disclosed". An obfuscated
    // identifier is `_` and then a name only the proxy that wrote it can
    // resolve, so there is nothing here to compare against either way.
    if token.eq_ignore_ascii_case("unknown") || token.starts_with('_') {
        return Hop::Withheld;
    }
    match address(token) {
        Some(ip) => Hop::At(canonical(ip)),
        None => Hop::Unreadable,
    }
}

/// A token's address, with the port and the brackets that carry one removed.
fn address(token: &str) -> Option<IpAddr> {
    // `[v6]` or `[v6]:port` — RFC 3986's spelling, and the only one in which a
    // v6 address can carry a port at all.
    if let Some(rest) = token.strip_prefix('[') {
        let (inside, after) = rest.split_once(']')?;
        if !(after.is_empty() || after.strip_prefix(':').is_some_and(is_port)) {
            return None;
        }
        return inside.parse().ok();
    }
    // A bare address is itself, and this is asked *before* a port is looked
    // for: a v6 address is mostly colons, and `::1:80` is one address rather
    // than a port on `::1`. Reading the whole token first is the conforming
    // way round, and it is also the one that cannot silently truncate.
    if let Ok(ip) = token.parse::<IpAddr>() {
        return Some(ip);
    }
    // What is left can only be `v4:port`.
    let (addr, port) = token.split_once(':')?;
    if !is_port(port) {
        return None;
    }
    addr.parse().ok()
}

/// Whether this is a port number — what separates an address carrying one from
/// a token that means nothing definite.
fn is_port(text: &str) -> bool {
    text.parse::<u16>().is_ok()
}

/// The scheme a trusted peer asserted, defaulting to `http`.
///
/// The **last** token wins, which is the one written by the proxy nearest this
/// server: a chain of proxies appends, so the rightmost value is the one whose
/// author is the peer that is allowed to speak. Anything that is not `https`
/// is `http` — a value the header cannot mean is a downgrade rather than a
/// refusal, because it costs a request nothing but an HSTS header it was not
/// going to be able to honour anyway.
fn asserted(headers: &HeaderMap) -> Scheme {
    let mut scheme = Scheme::Http;
    for line in headers.get_all(X_FORWARDED_PROTO) {
        let Ok(line) = line.to_str() else { continue };
        for token in line.split(',').map(str::trim).filter(|it| !it.is_empty()) {
            scheme = if token.eq_ignore_ascii_case("https") {
                Scheme::Https
            } else {
                Scheme::Http
            };
        }
    }
    scheme
}

#[cfg(test)]
mod tests {
    use super::{Arrival, Origin, Scheme, Trusted, Unusable, walk};
    use hyper::HeaderMap;
    use hyper::header::{HeaderName, HeaderValue};
    use std::net::IpAddr;

    /// A header map from field lines, in order and with repeats kept.
    fn head(lines: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in lines {
            headers.append(
                HeaderName::try_from(*name).expect("a header name"),
                HeaderValue::from_str(value).expect("a header value"),
            );
        }
        headers
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("an address")
    }

    fn from(text: &str) -> Arrival {
        Arrival::Tcp(ip(text))
    }

    fn proxies(entries: &[&str]) -> Trusted {
        let written: Vec<String> = entries.iter().map(|it| (*it).to_owned()).collect();
        let (trusted, rejected) = Trusted::of(&written);
        assert!(rejected.is_empty(), "test entries must all parse");
        trusted
    }

    /// The whole of `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s first rule, asked as a partition rather than
    /// as one example: for every combination of a written `trusted_proxies` and
    /// a peer, the client address and the effective scheme are the peer's own
    /// **unless** the peer is trusted — and then they are what it asserted.
    // covers: tools:server/behind-a-proxy-trusted-proxies
    #[test]
    fn client_ip_and_scheme_come_from_the_peer_unless_a_trusted_proxy_asserted() {
        let asserting = head(&[
            ("X-Forwarded-For", "203.0.113.9, 10.0.0.7"),
            ("X-Forwarded-Proto", "https"),
        ]);

        // Empty ⇒ the headers are not read at all. The peer is the client and
        // the scheme is plaintext however loudly the request says otherwise,
        // and this holds for a peer that *would* have been a proxy.
        for peer in ["10.0.0.7", "198.51.100.4"] {
            let untouched = walk(from(peer), &Trusted::none(), &asserting).expect("no refusal");
            assert_eq!(
                untouched,
                Origin {
                    client: Some(ip(peer)),
                    scheme: Scheme::Http,
                    ignored_address_header: false,
                },
                "an empty trusted_proxies read a header from {peer}"
            );
        }

        // A Unix-domain peer is no exception to empty: nothing is read, and
        // with no address of its own the client is nobody.
        let unread = walk(Arrival::Unix, &Trusted::none(), &asserting).expect("no refusal");
        assert_eq!(
            unread,
            Origin {
                client: None,
                scheme: Scheme::Http,
                ignored_address_header: false,
            },
            "an empty trusted_proxies read a header from a socket"
        );

        let trusted = proxies(&["10.0.0.0/8", "2001:db8::/32"]);

        // Non-empty, peer untrusted ⇒ ignored silently, never refused. Same
        // answer as above, and reached by the other of § 6's two arms.
        let ignored = walk(from("198.51.100.4"), &trusted, &asserting).expect("never a refusal");
        assert_eq!(
            ignored,
            Origin {
                client: Some(ip("198.51.100.4")),
                scheme: Scheme::Http,
                ignored_address_header: false,
            }
        );

        // Non-empty, peer trusted ⇒ both facts come from what it asserted. The
        // client is the rightmost entry that is not itself trusted, so the
        // trusted hop to its right is walked past and the attacker-controlled
        // text to its left is never reached.
        let spoofed = head(&[
            ("X-Forwarded-For", "9.9.9.9, 203.0.113.9, 10.0.0.7"),
            ("X-Forwarded-Proto", "https"),
        ]);
        for peer in ["10.0.0.7", "2001:db8::1"] {
            let asserted = walk(from(peer), &trusted, &spoofed).expect("no refusal");
            assert_eq!(
                asserted,
                Origin {
                    client: Some(ip("203.0.113.9")),
                    scheme: Scheme::Https,
                    ignored_address_header: false,
                },
                "a trusted {peer} was not believed"
            );
        }

        // A trusted peer that asserted nothing is still the client itself:
        // "unless" is about what was asserted, not about who connected.
        let silent = walk(from("10.0.0.7"), &trusted, &HeaderMap::new()).expect("no refusal");
        assert_eq!(
            silent,
            Origin {
                client: Some(ip("10.0.0.7")),
                scheme: Scheme::Http,
                ignored_address_header: false,
            }
        );

        // And a Unix-domain peer is trusted by the OS, so it may assert both —
        // with no address of its own to fall back to when it does not.
        let over_a_socket = walk(Arrival::Unix, &trusted, &asserting).expect("no refusal");
        assert_eq!(
            over_a_socket,
            Origin {
                client: Some(ip("203.0.113.9")),
                scheme: Scheme::Https,
                ignored_address_header: false,
            }
        );
        let quiet = walk(Arrival::Unix, &trusted, &HeaderMap::new()).expect("no refusal");
        assert_eq!(quiet.client(), None);
    }

    /// § 6's four defect rows, which do not resolve alike.
    #[test]
    fn the_four_defects_resolve_four_ways() {
        let trusted = proxies(&["10.0.0.0/8"]);

        // Several field lines are joined with commas and then walked.
        let split = head(&[
            ("X-Forwarded-For", "203.0.113.9"),
            ("X-Forwarded-For", "10.0.0.7"),
        ]);
        let joined = walk(from("10.0.0.7"), &trusted, &split).expect("no refusal");
        assert_eq!(joined.client(), Some(ip("203.0.113.9")));

        // `Forwarded` with no `X-Forwarded-For` from a trusted peer: ignored,
        // request served, one Warn for the caller to raise.
        let rfc7239 = head(&[("Forwarded", "for=203.0.113.9;proto=https")]);
        let served = walk(from("10.0.0.7"), &trusted, &rfc7239).expect("never a refusal");
        assert_eq!(served.client(), Some(ip("10.0.0.7")));
        assert!(served.ignored_address_header(), "the Warn was not reported");

        // The token the walk lands on does not parse: 400.
        let junk = head(&[("X-Forwarded-For", "not-an-address, 10.0.0.7")]);
        assert_eq!(walk(from("10.0.0.7"), &trusted, &junk), Err(Unusable));

        // But a token further left is never examined, so the same junk behind
        // an untrusted hop is not a refusal.
        let unreached = head(&[("X-Forwarded-For", "not-an-address, 203.0.113.9, 10.0.0.7")]);
        let reached = walk(from("10.0.0.7"), &trusted, &unreached).expect("no refusal");
        assert_eq!(reached.client(), Some(ip("203.0.113.9")));

        // And an `X-Forwarded-For` from an untrusted peer is ignored silently
        // however malformed it is — refusal is reserved for a value that was
        // about to be used, or anyone could deny service by sending one.
        let hostile = walk(from("198.51.100.4"), &trusted, &junk).expect("never a refusal");
        assert_eq!(hostile.client(), Some(ip("198.51.100.4")));
    }

    /// A `trusted_proxies` entry is an address or a CIDR block, a v4-mapped v6
    /// peer is the v4 address it maps, and an entry that names no network is
    /// handed back rather than silently widening or narrowing the set.
    // covers: tools:server/behind-a-proxy-trusted-proxies
    #[test]
    fn a_trusted_proxies_entry_is_an_address_or_a_block() {
        let written: Vec<String> = [
            "10.0.0.0/8",
            "192.0.2.4",
            "2001:db8::/32",
            "10.0.0.0/33",
            "nope",
        ]
        .iter()
        .map(|it| (*it).to_owned())
        .collect();
        let (trusted, rejected) = Trusted::of(&written);
        assert_eq!(rejected, vec!["10.0.0.0/33".to_owned(), "nope".to_owned()]);
        assert!(!trusted.is_empty());

        let chain = head(&[("X-Forwarded-For", "203.0.113.9")]);
        for peer in [
            "10.255.255.254",
            "192.0.2.4",
            "2001:db8:1::9",
            "::ffff:10.1.2.3",
        ] {
            let inside = walk(from(peer), &trusted, &chain).expect("no refusal");
            assert_eq!(
                inside.client(),
                Some(ip("203.0.113.9")),
                "{peer} should have been trusted"
            );
        }
        for peer in ["11.0.0.1", "192.0.2.5", "2001:db9::1"] {
            let outside = walk(from(peer), &trusted, &chain).expect("no refusal");
            assert_eq!(
                outside.client(),
                Some(ip(peer)),
                "{peer} should not have been trusted"
            );
        }
    }

    /// The nearest proxy's `X-Forwarded-Proto` is the one that counts, and
    /// anything that is not `https` leaves the scheme plaintext.
    #[test]
    fn the_nearest_proxy_asserts_the_scheme_and_only_https_is_believed() {
        let trusted = proxies(&["10.0.0.0/8"]);
        let cases = [
            (vec!["https"], Scheme::Https),
            (vec!["HTTPS"], Scheme::Https),
            (vec!["http"], Scheme::Http),
            (vec!["https", "http"], Scheme::Http),
            (vec!["http", "https"], Scheme::Https),
            (vec!["gopher"], Scheme::Http),
        ];
        for (lines, expected) in cases {
            let written: Vec<(&str, &str)> = lines
                .iter()
                .map(|value| ("X-Forwarded-Proto", *value))
                .collect();
            let answered = walk(from("10.0.0.7"), &trusted, &head(&written)).expect("no refusal");
            assert_eq!(answered.scheme(), expected, "for {lines:?}");
        }
    }

    /// § 6's two conforming spellings that are not defects: a port on the
    /// token, and a hop that withheld the address.
    #[test]
    fn a_port_is_not_part_of_the_address_and_a_withheld_hop_answers_none() {
        let trusted = proxies(&["10.0.0.0/8"]);

        // The port comes off in both families, and off the *trusted* hop too —
        // it is read before the trust test, or a proxy that writes its own port
        // would stop being recognised as itself.
        for (written, expected) in [
            ("203.0.113.9:54321, 10.0.0.7:443", "203.0.113.9"),
            ("[2001:db8::9]:443, 10.0.0.7", "2001:db8::9"),
            ("[2001:db8::9], 10.0.0.7", "2001:db8::9"),
            ("::ffff:203.0.113.9, 10.0.0.7", "203.0.113.9"),
        ] {
            let carried = head(&[("X-Forwarded-For", written)]);
            let answered = walk(from("10.0.0.7"), &trusted, &carried).expect("no refusal");
            assert_eq!(answered.client(), Some(ip(expected)), "for {written:?}");
        }

        // What is not a port is not read past: the token means nothing
        // definite, and the walk had landed on it.
        for written in [
            "203.0.113.9:notaport, 10.0.0.7",
            "203.0.113.9:99999, 10.0.0.7",
            "203.0.113.9:, 10.0.0.7",
            "[2001:db8::9, 10.0.0.7",
        ] {
            let carried = head(&[("X-Forwarded-For", written)]);
            assert_eq!(
                walk(from("10.0.0.7"), &trusted, &carried),
                Err(Unusable),
                "for {written:?}"
            );
        }

        // A hop that withheld the address answers `None` rather than refusing.
        for written in ["unknown, 10.0.0.7", "UNKNOWN", "_hidden, 10.0.0.7"] {
            let carried = head(&[("X-Forwarded-For", written)]);
            let withheld = walk(from("10.0.0.7"), &trusted, &carried).expect("never a refusal");
            assert_eq!(withheld.client(), None, "for {written:?}");
        }

        // And a client's own `unknown` is never reached, because its proxy
        // appended the address it actually came from to the right of it.
        let spoofed = head(&[("X-Forwarded-For", "unknown, 203.0.113.9, 10.0.0.7")]);
        let walked = walk(from("10.0.0.7"), &trusted, &spoofed).expect("no refusal");
        assert_eq!(walked.client(), Some(ip("203.0.113.9")));
    }

    /// A client-address header Novis does not read is reported rather than
    /// parsed — and reported only where it could have been believed.
    #[test]
    fn a_client_address_header_this_module_does_not_read_is_reported() {
        let trusted = proxies(&["10.0.0.0/8"]);

        // None of these is parsed, so the value's shape never matters.
        for name in [
            "Forwarded",
            "X-Real-IP",
            "CF-Connecting-IP",
            "True-Client-IP",
            "Fastly-Client-IP",
            "X-Client-IP",
        ] {
            let alone = head(&[(name, "203.0.113.9")]);
            let served = walk(from("10.0.0.7"), &trusted, &alone).expect("never a refusal");
            assert_eq!(served.client(), Some(ip("10.0.0.7")), "{name} was parsed");
            assert!(
                served.ignored_address_header(),
                "{name} was ignored quietly"
            );
        }

        // An untrusted peer's are not read at all, so there is nothing to
        // report: anyone can send one, and a Warn per request would be a log
        // flood a stranger could turn on.
        let hostile = head(&[("X-Real-IP", "203.0.113.9")]);
        let ignored = walk(from("198.51.100.4"), &trusted, &hostile).expect("never a refusal");
        assert!(!ignored.ignored_address_header());

        // And an `X-Forwarded-For` beside one has answered the question, so
        // nothing was ignored.
        let both = head(&[("X-Real-IP", "9.9.9.9"), ("X-Forwarded-For", "203.0.113.9")]);
        let answered = walk(from("10.0.0.7"), &trusted, &both).expect("no refusal");
        assert_eq!(answered.client(), Some(ip("203.0.113.9")));
        assert!(!answered.ignored_address_header());
    }
}
