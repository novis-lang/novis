//! `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`'s two meaningless combinations, in the one implementation the boot and
//! `Core\Config::set` both ask — and, beside them, the values under `[http.*]` that are refused
//! before either question is worth asking.
//!
//! **The rule is written once and read from two places, and that asymmetry is the module.** § 2
//! refuses `origins = ["*"]` with `credentials = true` and § 3 refuses `same_site = "None"` with
//! `secure = false`, "at boot with the line named and at runtime by `Core\Config::set` returning
//! `false`". Two refusals in two shapes is a standing invitation to write the *condition* twice and
//! have them drift, which is why [`Inbound`] holds the values the two questions are decided from
//! and [`Inbound::meaningless`] is the only place either question is answered.
//! [`validate`] reads those values off the merged tree; [`Request::set`](crate::request::Request)
//! reads them off its own snapshot and overlay and folds the proposed assignment in before asking.
//! What differs between the two callers is where the values come from, never what makes them wrong.
//!
//! **Refused rather than warned**, because neither combination is a weak policy — every browser
//! rejects both, so a deployment that wrote one has an access-control rule that does not do what it
//! says and no signal that it does not. § 2 says so for CORS and § 3 says the cookie pair is refused
//! "by the same mechanism, for the same reason".
//!
//! **An absent key is its shipped default, not `false`.** § 2's CORS block is closed with nothing
//! written and § 3's cookies are `Secure; HttpOnly; SameSite=Lax`, so a tree that never mentions
//! `secure` has `secure = true` in force and a `same_site = "None"` written beside it is meaningful.
//! Reading an absent boolean as `false` would refuse that tree, which is the one direction a
//! security check must not fail in: it would teach operators to write the pair out to get a boot.
//!
//! **[`Cookies`] is the other half of § 3, and it is a resolver rather than a refusal.** The pair
//! above says which trees are wrong; this says what a `Core\Response::addCookie` inherits for an
//! option its call site left out — `Secure; HttpOnly; SameSite=Lax; Path=/` with nothing written.
//! Both read the same block, and `secure` is the one value both need, read once here so § 3's
//! default for it is stated in one place. [`validate`] additionally refuses a `same_site` that is
//! none of the three spellings, as `E0624`: that is what lets [`Cookies::of`] resolve the key with
//! no fourth arm, and so without ever repairing one.
//!
//! **[`validate`] additionally refuses what the wire cannot carry, which is a different question
//! from what a policy means.** § 1's free-text values — `referrer_policy`,
//! `content_security_policy` and `permissions_policy` — go onto every response verbatim, so a
//! `\r\n` in one is a response split against every request the server will answer. `nvs_server`'s
//! `secure` already declines to spell such a value and emits the shipped default instead, which is
//! the right answer for a request in flight and the wrong one for a boot: the deployment believes
//! its policy is in force and nothing says otherwise. `E0625` at boot is what makes that fallback
//! unreachable from a server that started, and both are kept.
//!
//! **§ 2's own values are refused on the same two terms**, because `nvs_server::cors` resolves them
//! into header lines at boot and so has the same fallbacks to make unreachable: an entry of
//! `methods`, `headers` or `expose` that a header line cannot carry is `E0625`, and a `max_age`
//! that is not a duration is `E0601` — the code a directive whose value is not what its unit takes
//! has however it arrived, so [`mod@crate::value`] writes that sentence rather than this module.
//! `origins` is deliberately not checked for either: nothing writes it onto a response, and an
//! entry a header line could not carry is one no `Origin` can equal.
//!
//! **[`validate`] carries the outbound block's refusals too, and they are asked first.** They are
//! not about a response at all: `[http.client.tls]`'s three and then `[http.client.proxy]`'s four,
//! because a tree that writes its own TLS secrets to a file or hands its destinations to a proxy on
//! terms it never stated is a security question, and what a header means is not. Inside the proxy block the
//! mandatory `resolve` is asked before the `url` it qualifies, since that word is what decides
//! whether `rule:security/net-address-policy` is still answered about an address
//! (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
//! `resolve = "proxy"` is a valid tree and the narrowing is never silent: [`advise`] is what says
//! so at every boot, beside the key log's own announcement.
//!
//! Cost: [`Inbound`]'s booleans built at boot, at reload, and once per `Core\Config::set` naming a
//! key under `[http.cors]` or `[http.cookies]`. Every other `set` returns before this module is
//! reached. [`Cookies::of`] is another read of the same block plus one `String` clone, once per
//! `addCookie` call; the byte scans are boot and reload only, over hand-written values.
//!

use std::collections::BTreeMap;
use std::path::Path;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, Http, HttpClientProxy, HttpClientTls, Setting};
use crate::value::{Quantity, Unit};

/// The `[http.client.tls] roots` entry naming the compiled-in Mozilla set rather than a file.
///
/// A constant because two crates spell it: this one skips it while resolving paths, and
/// `nvs_host::tls` reads it as "extend with the bundled anchors". A literal in each would be one
/// typo away from a `roots` list that silently trusted a file named `bundled`.
pub const BUNDLED: &str = "bundled";

/// `[http.client.proxy] resolve = "local"`: Novis resolves the destination, checks every address
/// against `rule:security/net-address-policy` and asks the proxy to `CONNECT` to one it approved.
///
/// A constant for [`BUNDLED`]'s reason — the boot refuses a third word here and
/// `nvs_stdlib::http::transport` decides what to put in the `CONNECT` line from the same two
/// spellings, and a literal in each place is one typo away from a tunnel that gave up the pin
/// without saying so.
pub const RESOLVE_LOCALLY: &str = "local";

/// `[http.client.proxy] resolve = "proxy"`: `CONNECT` carries the host name, and the address
/// question is the proxy's — see [`RESOLVE_LOCALLY`], and
/// `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise` for what the
/// word costs and what every boot says about it.
pub const RESOLVE_AT_THE_PROXY: &str = "proxy";

/// The values §§ 2-3's two refusals are decided from, resolved to what is in force.
///
/// Booleans and not the blocks themselves, because the callers read from different places — a typed
/// tree at boot, a snapshot plus an overlay plus a proposed assignment inside a request — and the
/// only thing they have to agree on is the answer to these questions.
///
/// Built by [`Inbound::of`] so the defaults are stated once; there is deliberately no `Default`
/// impl, because `bool`'s own default is `false` and some of these default to `true`.
#[derive(Clone, Copy, Debug)]
pub struct Inbound {
    /// Whether `[http.cors] origins` contains `*`.
    pub star_origin: bool,
    /// `[http.cors] credentials`; § 2's default is `false`.
    pub credentials: bool,
    /// Whether `[http.cookies] same_site` is `None`; § 3's default is `Lax`.
    pub same_site_none: bool,
    /// `[http.cookies] secure`; § 3's default is `true`.
    pub secure: bool,
}

/// A combination with no correct meaning — one variant per pair §§ 2-3 refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meaningless {
    /// § 2: `origins = ["*"]` together with `credentials = true`.
    CorsStarWithCredentials,
    /// § 3: `same_site = "None"` together with `secure = false`.
    SameSiteNoneWithoutSecure,
}

impl Inbound {
    /// What is in force for a tree, with every absent key at §§ 2-3's shipped default.
    #[must_use]
    pub fn of(http: Option<&Http>) -> Self {
        let cors = http.and_then(|http| http.cors.as_ref());
        let cookies = http.and_then(|http| http.cookies.as_ref());
        Self {
            star_origin: cors
                .and_then(|cors| cors.origins.as_deref())
                .is_some_and(|origins| origins.iter().any(|origin| origin == "*")),
            credentials: cors.and_then(|cors| cors.credentials).unwrap_or(false),
            same_site_none: cookies
                .and_then(|cookies| cookies.same_site.as_deref())
                .is_some_and(is_none_same_site),
            secure: cookies.and_then(|cookies| cookies.secure).unwrap_or(true),
        }
    }

    /// Folds one `key = value` assignment in — a request's overlay entry, or the value
    /// `Core\Config::set` is proposing. A key deciding neither pair changes nothing.
    ///
    /// `origins` is absent on purpose: it is a list, and § 5 of `rule:config/the-file-is-nvs-toml-and-it-is-toml` crosses values as text, so
    /// no request can set one. Which star origins are in force is therefore always the snapshot's
    /// answer, and [`of`](Self::of) has already read it.
    pub fn assign(&mut self, key: &str, value: &str) {
        match key {
            "http.cors.credentials" => self.credentials = is_true(value),
            "http.cookies.same_site" => self.same_site_none = is_none_same_site(value),
            "http.cookies.secure" => self.secure = is_true(value),
            _ => {}
        }
    }

    /// Whether either pair is one of §§ 2-3's, and which — [`None`] for a meaningful configuration.
    ///
    /// The one place the condition itself is written. Both refusals check the *pair*: a star origin
    /// alone is a deliberate public API and `same_site = "None"` alone is an ordinary cross-site
    /// cookie, so neither half is wrong by itself and neither is refused by itself.
    #[must_use]
    pub fn meaningless(self) -> Option<Meaningless> {
        if self.star_origin && self.credentials {
            return Some(Meaningless::CorsStarWithCredentials);
        }
        if self.same_site_none && !self.secure {
            return Some(Meaningless::SameSiteNoneWithoutSecure);
        }
        None
    }
}

impl Meaningless {
    /// The key whose origin names the line a boot refusal points at — the half an operator is most
    /// likely to have written last, and the one that is a scalar in both pairs.
    fn key(self) -> &'static str {
        match self {
            Self::CorsStarWithCredentials => "http.cors.credentials",
            Self::SameSiteNoneWithoutSecure => "http.cookies.same_site",
        }
    }

    /// The refusal's own sentence.
    fn what(self) -> &'static str {
        match self {
            Self::CorsStarWithCredentials => {
                "`[http.cors]` names `*` as an origin and sets `credentials = true`"
            }
            Self::SameSiteNoneWithoutSecure => {
                "`[http.cookies]` sets `same_site = \"None\"` and `secure = false`"
            }
        }
    }

    /// Why it is refused rather than warned about.
    fn note(self) -> &'static str {
        match self {
            Self::CorsStarWithCredentials => {
                "§ 2 permits `*` only with `credentials = false`: every browser rejects a wildcard \
                 origin on a credentialed response, so the pair is not a risky access-control \
                 policy but one that does not run at all, and nothing in the deployment would say so"
            }
            Self::SameSiteNoneWithoutSecure => {
                "§ 3 refuses this pair by the same mechanism as § 2's and for the same reason: \
                 browsers drop a `SameSite=None` cookie that is not `Secure`, so the configuration \
                 describes a cookie that is never stored"
            }
        }
    }

    /// The two ways out, both of which are a decision about what the deployment meant.
    fn help(self) -> &'static str {
        match self {
            Self::CorsStarWithCredentials => {
                "set `credentials = false`, or name the exact origins that may send credentials"
            }
            Self::SameSiteNoneWithoutSecure => {
                "set `secure = true`, or use `same_site = \"Lax\"` for a cookie that is not sent \
                 cross-site"
            }
        }
    }
}

/// § 3's `same_site`, as the closed set the attribute actually has.
///
/// A `String` in the tree and an enum from here on, because everything downstream of the parse asks
/// which of three this is rather than what it was spelled as — [`validate`] refuses a fourth
/// spelling at boot, so no later reader has to carry an "or something else" arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SameSite {
    /// Sent with a top-level navigation and not with a cross-site subrequest — § 3's default.
    Lax,
    /// Never sent cross-site at all.
    Strict,
    /// Sent cross-site, which is why § 3 pairs it with `Secure`.
    None,
}

impl SameSite {
    /// The three spellings, case-insensitively; [`None`] for anything else.
    ///
    /// Case-insensitive because the attribute a browser parses is, so a tree writing `none` has
    /// configured the same cookie and must reach § 3's refusal rather than a fourth-spelling one.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        for (name, case) in [
            ("Lax", Self::Lax),
            ("Strict", Self::Strict),
            ("None", Self::None),
        ] {
            if value.eq_ignore_ascii_case(name) {
                return Some(case);
            }
        }
        Option::None
    }

    /// The attribute as it is written on the wire.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lax => "Lax",
            Self::Strict => "Strict",
            Self::None => "None",
        }
    }
}

/// § 3's defaults, resolved to what is in force — what every `Core\Response::addCookie`
/// inherits for an option its call site left out.
///
/// Beside [`Inbound`] rather than folded into it, because the two answer different questions off
/// the same block: `Inbound` holds the booleans *two refusals* are decided from, and this holds
/// the values *a cookie is written with*. They overlap in `secure` alone, and that one
/// value is read here through the same `unwrap_or(true)` on purpose — § 3 states one default for
/// it, so a second statement of it would be the drift this module exists to prevent.
///
/// Cost: one `String` clone per `addCookie` call, for `path`. The alternative is borrowing the
/// snapshot across the member's own writes, which `crates/nvs-stdlib`'s queue path already declined
/// for the same reason: the tree, the values read out of it and the `ctx` being written are live at
/// once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cookies {
    /// The `Secure` attribute; § 3's default is `true`.
    pub secure: bool,
    /// The `HttpOnly` attribute; § 3's default is `true`.
    pub http_only: bool,
    /// The `SameSite` attribute; § 3's default is [`SameSite::Lax`].
    pub same_site: SameSite,
    /// The `Path` attribute; § 3's default is `/`.
    pub path: String,
}

impl Cookies {
    /// What is in force for a tree, with every absent key at § 3's shipped default.
    ///
    /// An unparseable `same_site` resolves to the default rather than throwing, and that is not a
    /// repair: [`validate`] has already refused such a tree at boot, so this arm is reachable only
    /// from a snapshot that never started a server.
    #[must_use]
    pub fn of(http: Option<&Http>) -> Self {
        let cookies = http.and_then(|http| http.cookies.as_ref());
        Self {
            secure: cookies.and_then(|cookies| cookies.secure).unwrap_or(true),
            http_only: cookies
                .and_then(|cookies| cookies.http_only)
                .unwrap_or(true),
            same_site: cookies
                .and_then(|cookies| cookies.same_site.as_deref())
                .and_then(SameSite::parse)
                .unwrap_or(SameSite::Lax),
            path: cookies
                .and_then(|cookies| cookies.path.as_deref())
                .unwrap_or("/")
                .to_owned(),
        }
    }
}

/// Whether a `same_site` value is § 3's `None`.
fn is_none_same_site(value: &str) -> bool {
    SameSite::parse(value) == Some(SameSite::None)
}

/// Whether a value is bytes a header field value can carry — RFC 9110's rule, which
/// `Core\Response::setHeader` applies to a value a *program* wrote and this applies to one the
/// *configuration* did.
///
/// Written twice on purpose, in the two crates that each own one of those two moments:
/// `nvs-stdlib` has no business in a `nvs.toml` and this crate has none in a member's arguments.
/// What they share is one byte-range test, and a dependency between them to save it would be
/// the more expensive of the two.
fn carriable(value: &str) -> bool {
    value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

/// A boolean as `Core\Config::set` crosses it: `rule:config/ini-set-is-core-config-set` sends values as text, and a directive
/// the file wrote as a TOML boolean reads back as `true` or `false`.
fn is_true(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("true")
}

/// Makes every PEM file under `[http.client.tls] roots` absolute — `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` — and proves each
/// one is inside the trust boundary.
///
/// [`crate::db::canonicalize`]'s pass asked of the outbound client's anchors instead of a `[db]`
/// block's, and a trust-boundary file for the same reason
/// (`rule:config/ownership-is-the-trust-boundary`): whoever can rewrite one of these chooses which
/// servers **every** outbound call in the process may be talking to, which is a wider authority
/// than the per-block bundle and never a narrower one. Over the merged tree, because which list is
/// in force is a question only the merge has answered.
///
/// The [`BUNDLED`] entry is passed over — it names no file — and nothing here reads a bundle it
/// resolves. `nvs_host::tls` parses them, which is what keeps one answer to whose certificates this
/// process believes (`rule:security/one-tls-client`).
///
/// # Errors
///
/// `E0607` for an entry outside the trust boundary and `E0605` for one that cannot be read at all,
/// which is [`crate::resolve::untrusted`]'s split.
pub fn canonicalize(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    let Some(roots) = config
        .http
        .as_mut()
        .and_then(|http| http.client.as_mut())
        .and_then(|client| client.tls.as_mut())
        .and_then(|tls| tls.roots.as_mut())
    else {
        return Ok(());
    };
    let base = crate::db::written_in(origins, "http.client.tls.roots");
    for entry in roots.iter_mut() {
        if entry == BUNDLED {
            continue;
        }
        let path = crate::resolve::absolute(base, Path::new(entry));
        let trusted = files.trust(&path).map_err(|why| {
            crate::resolve::untrusted(
                &path,
                &why,
                "`[http.client.tls] roots` names it, and whoever can write it chooses which servers \
                 every outbound call this process makes may be talking to",
            )
        })?;
        *entry = trusted.to_string_lossy().into_owned();
    }
    rewrite_roots(table, roots);
    Ok(())
}

/// Puts the resolved `roots` back into the merged table as well as onto the typed tree.
///
/// [`crate::db::canonicalize`]'s reason exactly: `Snapshot::retype` deserializes the tree back out
/// of the table, so a pass that rewrote only the typed side would prove one set of files safe and
/// hand `nvs_host::tls` a different set to parse. A tree the table does not hold is not an error —
/// the typed side is what says the block exists.
fn rewrite_roots(table: &mut toml::value::Table, roots: &[String]) {
    let Some(tls) = table
        .get_mut("http")
        .and_then(toml::Value::as_table_mut)
        .and_then(|http| http.get_mut("client"))
        .and_then(toml::Value::as_table_mut)
        .and_then(|client| client.get_mut("tls"))
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };
    let entries = roots
        .iter()
        .map(|entry| toml::Value::String(entry.clone()))
        .collect();
    tls.insert("roots".to_string(), toml::Value::Array(entries));
}

/// `[http.client.tls]`'s three values, asked of the merged tree at boot.
///
/// Here rather than beside the anchors in `nvs_host::tls` because each of the three is wrong in a
/// way the file can be shown for: the parse that would otherwise discover it happens on the first
/// outbound call, inside a request, where nothing can name the line an operator wrote.
fn tls(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(tls) = written_tls(config) else {
        return Ok(());
    };
    if tls.roots.as_deref().is_some_and(<[String]>::is_empty) {
        return Err(Diagnostic::error(
            code::E_TLS_ROOTS_EMPTY,
            "`[http.client.tls] roots` is empty, so no certificate could vouch for any origin"
                .to_string(),
        )
        .with_note(format!(
            "the list is the whole answer to whose certificates an outbound `https` call believes, \
             and an empty one answers nobody's: every such call would fail at the handshake with an \
             unknown issuer{}",
            origin_note(origins.get("http.client.tls.roots"))
        ))
        .with_help(format!(
            "leave the key out for the compiled-in set, or name what this deployment trusts — \
             `[\"{BUNDLED}\"]`, a PEM file, or both"
        )));
    }
    if let Some(written) = tls.min_version.as_deref()
        && !matches!(written, "1.2" | "1.3")
    {
        return Err(Diagnostic::error(
            code::E_TLS_MIN_VERSION,
            format!("`[http.client.tls] min_version` is `{written}`, which this client cannot speak"),
        )
        .with_note(format!(
            "the client implements TLS 1.2 and 1.3 and nothing beneath them, so a lower floor would \
             leave the real one at 1.2 while the file said otherwise{}",
            origin_note(origins.get("http.client.tls.min_version"))
        ))
        .with_help("write `1.2` or `1.3` — or leave the key out, which is `1.2`".to_string()));
    }
    let Some(written) = tls.keylog.as_deref() else {
        return Ok(());
    };
    if mode(config) != crate::mode::PRODUCTION {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_KEYLOG_IN_PRODUCTION,
        format!(
            "`[http.client.tls] keylog` writes `{written}`, and this host runs in `production`"
        ),
    )
    .with_note(format!(
        "the file collects every TLS session's secrets, which decrypts everything this deployment \
         sends — the credentials in it included — for whoever can read it{}",
        origin_note(origins.get("http.client.tls.keylog"))
    ))
    .with_help(
        "remove the key, or write it only in a tree whose `[mode] default` is `development`"
            .to_string(),
    ))
}

/// What `[mode] default` says, which is `production` with nothing written
/// (`rule:config/two-modes-and-the-default-is-production`).
fn mode(config: &Config) -> &str {
    config
        .mode
        .as_ref()
        .and_then(|mode| mode.default.as_deref())
        .unwrap_or(crate::mode::PRODUCTION)
}

/// The `[http.client.tls]` block as the merge left it.
fn written_tls(config: &Config) -> Option<&HttpClientTls> {
    config
        .http
        .as_ref()
        .and_then(|http| http.client.as_ref())
        .and_then(|client| client.tls.as_ref())
}

/// The `[http.client.proxy]` block, where a tree wrote one. An absent block is no proxy.
///
/// The one spelling of this chain: [`mod@crate::secret`]'s roster holds the block's credential and
/// `nvs_stdlib::http` builds a call's tunnel from it, and either of them walking the tree itself
/// would be a second reader that agrees until a block moves.
#[must_use]
pub fn written_proxy(config: &Config) -> Option<&HttpClientProxy> {
    config
        .http
        .as_ref()
        .and_then(|http| http.client.as_ref())
        .and_then(|client| client.proxy.as_ref())
}

/// The octets `[http] csrf_key` decodes to, which is what the construction
/// behind `Core\Csrf` takes.
///
/// The number is `nvs_runtime::csrf::KEY_LEN`'s, repeated here because this
/// crate sits below that one and a boot refusal cannot call the thing that
/// enforces it. That module holds a compile-time assertion against this
/// constant, so the two cannot drift.
pub const CSRF_KEY_BYTES: usize = 32;

/// The token key `[http] csrf_key` names, decoded — `None` where the tree names
/// none.
///
/// Written as unpadded URL-safe base64, which is the alphabet every other key
/// and token in this tree travels in and the one a `Core\Crypto::generateKey()`
/// value reaches a mounted file as. Standard base64's two extra characters are
/// accepted on the way in, because a key generated by some other tool is the
/// ordinary case and the two alphabets cannot be confused for one another.
///
/// **`None` also for a value that is not a key**, which is not a fallback: a
/// tree holding one is refused by [`validate`] at boot, so no started server
/// reaches this with an unreadable key and the silent-disarm it would otherwise
/// mean is unreachable rather than merely unlikely.
#[must_use]
pub fn csrf_key(config: &Config) -> Option<Vec<u8>> {
    let written = config
        .http
        .as_ref()
        .and_then(|http| http.csrf_key.as_deref())?;
    decoded_key(written)
}

/// [`csrf_key`]'s decode, as the one function both it and [`validate`] ask, so
/// that what the boot accepted and what the door reads cannot be two answers.
fn decoded_key(written: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;

    let trimmed = written.trim_end_matches('=');
    let key = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(trimmed)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(trimmed))
        .ok()?;
    (key.len() == CSRF_KEY_BYTES).then_some(key)
}

/// `[http.client.proxy]`'s four refusals, asked of the merged tree at boot.
///
/// The mandatory word comes first, because it is the one that decides whether
/// `rule:security/net-address-policy` still sees an address: a block that never answered it is a
/// deployment whose egress posture nobody has stated, and reading a `url` before that would be
/// checking the address of a proxy whose terms are unknown.
fn proxy(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(proxy) = written_proxy(config) else {
        return Ok(());
    };
    match proxy.resolve.as_deref() {
        Some(RESOLVE_LOCALLY | RESOLVE_AT_THE_PROXY) => {}
        None => {
            return Err(Diagnostic::error(
                code::E_PROXY_RESOLVE_MISSING,
                "`[http.client.proxy]` writes no `resolve`, and there is no default for it"
                    .to_string(),
            )
            .with_note(format!(
                "the word says who resolves the destination, which is who the address policy is \
                 asked of: `{RESOLVE_LOCALLY}` resolves here and tunnels to an address \
                 `rule:security/net-address-policy` approved, `{RESOLVE_AT_THE_PROXY}` sends the \
                 host name and moves that question to the proxy{}",
                origin_note(origins.get("http.client.proxy.url"))
            ))
            .with_help(format!(
                "write `resolve = \"{RESOLVE_LOCALLY}\"` unless only the proxy can resolve this \
                 network's destinations, in which case write `\"{RESOLVE_AT_THE_PROXY}\"` and read \
                 the warning it prints at every boot"
            )));
        }
        Some(third) => {
            return Err(Diagnostic::error(
                code::E_PROXY_RESOLVE_UNKNOWN,
                format!(
                    "`[http.client.proxy] resolve` is `{third}`, which is neither \
                     `{RESOLVE_LOCALLY}` nor `{RESOLVE_AT_THE_PROXY}`"
                ),
            )
            .with_note(format!(
                "those two are the whole roster, and the choice between them is a security \
                 posture rather than a spelling — reading a third word as either would pick one \
                 out of a typo{}",
                origin_note(origins.get("http.client.proxy.resolve"))
            ))
            .with_help(format!(
                "`{RESOLVE_LOCALLY}` keeps every call pinned to an address this deployment \
                 approved; `{RESOLVE_AT_THE_PROXY}` hands the address question to the proxy"
            )));
        }
    }
    if let Some(problem) = url_problem(proxy.url.as_deref()) {
        return Err(Diagnostic::error(code::E_PROXY_URL_UNDIALABLE, problem)
            .with_note(format!(
                "every destination is reached by `CONNECT` over plain TCP to that address, so the \
                 scheme is `http` and nothing else — TLS to the proxy is a second trust decision \
                 with no spelling here — and a credential is read from `username` and `password`, \
                 never from the URL{}",
                origin_note(origins.get("http.client.proxy.url"))
            ))
            .with_help(
                "write `url = \"http://proxy.internal:3128\"`, or leave the whole block out, \
                 which is how a deployment asks for no proxy"
                    .to_string(),
            ));
    }
    for entry in proxy.bypass.iter().flatten() {
        let Some(problem) = bypass_problem(entry) else {
            continue;
        };
        return Err(Diagnostic::error(
            code::E_PROXY_BYPASS_ENTRY,
            format!("`[http.client.proxy] bypass` has the entry `{entry}`, which {problem}"),
        )
        .with_note(format!(
            "the list is host names matched against the URL's own text before anything is \
             resolved — each entry exact, or with a leading `.` for a suffix — so an entry that \
             cannot equal a host is a bypass an operator believes is in force, and a range is more \
             traffic left unproxied than they can see they are asking for{}",
            origin_note(origins.get("http.client.proxy.bypass"))
        ))
        .with_help(
            "write the host alone — `internal.example.com`, or `.example.com` for every host \
             under it"
                .to_string(),
        ));
    }
    Ok(())
}

/// Why a `[http.client.proxy] url` is one this client cannot dial, as the whole first line of the
/// refusal, or `None` when it can.
///
/// An absent `url` is one of the answers rather than a `None`: leaving the block out is how a
/// deployment asks for no proxy, so a block that writes one and names no address is a deployment
/// believing its egress is tunnelled when nothing tunnels it.
fn url_problem(written: Option<&str>) -> Option<String> {
    let Some(written) = written else {
        return Some("`[http.client.proxy]` is written and names no `url`".to_string());
    };
    proxy_endpoint(written)
        .err()
        .map(|why| format!("`[http.client.proxy] url` is `{written}`, which {why}"))
}

/// Where a `[http.client.proxy] url` that names no port is dialled — an `http://` URL's own
/// default, since the proxy is spoken to over plain TCP like any other origin.
pub const DEFAULT_PROXY_PORT: u16 = 80;

/// The host and port a `[http.client.proxy] url` names, so that the boot's refusal and the
/// transport's `CONNECT` take one text apart the same way: a URL that passed [`validate`] is one
/// `nvs_stdlib::http` can dial, and there is no second grammar for it to disagree with.
///
/// # Errors
///
/// Why this client cannot dial the text, as the clause [`url_problem`] writes after `which`.
pub fn proxy_endpoint(written: &str) -> Result<(&str, u16), &'static str> {
    let rest = written
        .strip_prefix("http://")
        .ok_or("is not an `http://` URL")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if authority.contains('@') {
        return Err("carries a credential");
    }
    let (host, port) = if let Some(inside) = authority.strip_prefix('[') {
        let (host, tail) = inside
            .split_once(']')
            .ok_or("has no closing `]` on its IPv6 address")?;
        match tail {
            "" => (host, None),
            _ => (
                host,
                Some(
                    tail.strip_prefix(':')
                        .ok_or("has text after its IPv6 address")?,
                ),
            ),
        }
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    if host.is_empty() {
        return Err("names no host");
    }
    let port = match port {
        Some(port) => port
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or("names no port a connection can be opened to")?,
        None => DEFAULT_PROXY_PORT,
    };
    Ok((host, port))
}

/// Why a `[http.client.proxy] bypass` entry is not a host name, or `None` when it is one.
///
/// The scheme is asked before the port so that `http://host` is reported as the scheme it is
/// rather than as the `:` inside it.
fn bypass_problem(entry: &str) -> Option<&'static str> {
    if entry.is_empty() {
        return Some("is empty");
    }
    if entry.contains("://") {
        return Some("carries a scheme");
    }
    if entry.contains(':') {
        return Some("carries a port");
    }
    if entry.contains('*') {
        return Some("is a wildcard");
    }
    if entry.contains('/') {
        return Some("carries a path or a CIDR range");
    }
    None
}

/// This module's boot announcements, one per start each: a key log a host accepted, and an address
/// question a `resolve` moved to the proxy.
///
/// Warnings and not lines in the boot log's ordinary body, because each describes a state somebody
/// switched on and nobody reports having switched off. Neither is a refusal: `production` never
/// reaches the first — [`tls`] refused that tree — and the second is a tree the operator meant,
/// where what the log owes an auditor is the sentence saying so.
#[must_use]
pub fn advise(config: &Config, origins: &BTreeMap<String, Origin>) -> Vec<Diagnostic> {
    let mut announced = Vec::new();
    if let Some(written) = written_tls(config).and_then(|tls| tls.keylog.as_deref()) {
        announced.push(
            Diagnostic::warning(
                code::W_TLS_KEYLOG_ON,
                format!("every TLS session's secrets are being appended to `{written}`"),
            )
            .with_note(format!(
                "`[http.client.tls] keylog` is set and this host runs in `{}`, so the file \
                 decrypts this deployment's outbound traffic for anyone who can read it{}",
                mode(config),
                origin_note(origins.get("http.client.tls.keylog"))
            ))
            .with_help("remove the key once the capture you wanted is taken".to_string()),
        );
    }
    if written_proxy(config).and_then(|proxy| proxy.resolve.as_deref())
        == Some(RESOLVE_AT_THE_PROXY)
    {
        announced.push(
            Diagnostic::warning(
                code::W_PROXY_RESOLVES_THE_DESTINATION,
                format!(
                    "`[http.client.proxy] resolve` is `{RESOLVE_AT_THE_PROXY}`, so every outbound \
                     destination is resolved by the proxy and not by this deployment"
                ),
            )
            .with_note(format!(
                "`rule:security/net-address-policy` is answered about an address, and there is \
                 none here: `CONNECT` carries the host name, so what is judged is the URL's \
                 scheme, the `net.connect` grant's host list and the tainted-URL check{}",
                origin_note(origins.get("http.client.proxy.resolve"))
            ))
            .with_help(format!(
                "write `{RESOLVE_LOCALLY}` where this network can resolve its own destinations, \
                 which keeps every call pinned to an address Novis approved"
            )),
        );
    }
    announced
}

/// `[http.client.socket]`'s two bounds, refused where either is written with no bound in it.
///
/// # Errors
///
/// [`socket_bound`]'s `E0647`, and `E0601` for a value that is not a quantity at all.
fn socket(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(block) = config
        .http
        .as_ref()
        .and_then(|http| http.client.as_ref())
        .and_then(|client| client.socket.as_ref())
    else {
        return Ok(());
    };
    socket_bound(
        "http.client.socket.max_message",
        Unit::Bytes,
        block.max_message.as_ref(),
        origins,
    )?;
    socket_bound(
        "http.client.socket.send_timeout",
        Unit::Duration,
        block.send_timeout.as_ref(),
        origins,
    )
}

/// One of those bounds, and `Ok(())` for one the block left out.
///
/// [`mod@crate::value`] is the parser, so `"4MB"`, `4194304` and `"30s"` all read as what they
/// spell and a suffix it does not know is refused in its own words. What is left for this function
/// is the pair of values that parse and are not bounds: `false`, which removes a ceiling everywhere
/// else in this file, and zero, which is the same value written the other way round — a cap of
/// nothing admits no message and a wait of nothing writes no frame. An outbound socket has no
/// spelling for either (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`).
fn socket_bound(
    key: &str,
    unit: Unit,
    written: Option<&Setting>,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let Some(setting) = written else {
        return Ok(());
    };
    let quantity = Quantity::parse(key, unit, setting)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    let removed = match quantity {
        Quantity::Unbounded => true,
        Quantity::Bytes(magnitude) | Quantity::Nanos(magnitude) | Quantity::Count(magnitude) => {
            magnitude == 0
        }
        // Unreachable in either unit above, and answered rather than left out: a ratio is neither a
        // size nor a wait, so a value that spelled one has already been refused by the parser.
        Quantity::Ratio(_) => false,
    };
    if !removed {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_SOCKET_BOUND_REMOVED,
        format!(
            "`{key}` is `{}`, which is not a bound",
            crate::value::as_written(setting)
        ),
    )
    .with_note(format!(
        "a socket this host holds open reassembles a message into the opening task's memory and \
         waits to write one, and neither has a spelling for \"no limit\": `false` removes a \
         ceiling, and zero admits no message and writes no frame{}",
        origin_note(origins.get(key))
    ))
    .with_help(
        "write the bound this deployment wants, or leave the key out for the shipped one — \
         `max_message = 4194304` and `send_timeout = \"30s\"`"
            .to_string(),
    ))
}

/// §§ 2-3's two refusals, asked of the merged tree at boot.
///
/// # Errors
///
/// One [`Diagnostic`], `E0612`, for the first pair with no correct meaning: `origins = ["*"]` with
/// `credentials = true`, or `same_site = "None"` with `secure = false`. `E0624` first, for a
/// `same_site` that is none of the three spellings — a value neither pair can be decided from.
/// Before either, `E0625` for a value under `[http.headers]` or a list entry under `[http.cors]`
/// that a header line cannot carry, and `E0601` for a `[http.cors] max_age` that is not a duration.
/// Before all of them, the outbound block's: `[http.client.tls]`'s three — `E0638` for an empty
/// `roots`, `E0639` for a `min_version` this client cannot speak, `E0640` for a `keylog` on a
/// `production` host — and then `[http.client.proxy]`'s four, `E0643` for a block with no
/// `resolve`, `E0644` for a third word, `E0645` for a `url` this client cannot dial and `E0646` for
/// a `bypass` entry that is not a host name. After those, `[http.client.socket]`'s `E0647` for a
/// bound written as `false` or as zero, which is a bound that is not one.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    // The outbound block first, and its `keylog` is why: a tree that leaks its own TLS secrets is
    // a security hole, so it is refused before any question about what a response header means.
    tls(config, origins)?;
    // The proxy beside it, for the same ordering reason: where every outbound byte goes, and who
    // gets to resolve the destination, outranks what a response header means.
    proxy(config, origins)?;
    // The socket block last of the outbound three and still ahead of everything inbound: what it
    // refuses is a bound taken off a conversation this host holds open, which outranks what a
    // response header means and sits under where the bytes go.
    socket(config, origins)?;
    // The first inbound question, and ahead of every other one: whether a state-changing request
    // is verified at all outranks what a response header means, and a key the door cannot read
    // would leave `rule:security/csrf-is-on-by-default`'s token half unarmed under a tree that
    // says it is on.
    if let Some(written) = config
        .http
        .as_ref()
        .and_then(|http| http.csrf_key.as_deref())
        && decoded_key(written).is_none()
    {
        return Err(Diagnostic::error(
            code::E_BAD_CSRF_KEY,
            "`[http] csrf_key` does not hold a key".to_string(),
        )
        .with_note(format!(
            "the door verifies every unsafe verb's token against this value, so it is \
             {CSRF_KEY_BYTES} octets written as base64 — `Core\\Crypto::generateKey()` answers one \
             of the right length. The value is not quoted here, because a key does not belong in a \
             log{}",
            origin_note(origins.get("http.csrf_key"))
        ))
        .with_help(
            "write the key's base64, or leave the directive out and let the door refuse only the \
             cross-origin half"
                .to_string(),
        ));
    }
    // § 1's free-text policies, before anything about meaning: a value the wire cannot carry
    // is not a policy that is wrong, it is a policy that never reaches a peer at all.
    let headers = config.http.as_ref().and_then(|http| http.headers.as_ref());
    if let Some(headers) = headers {
        for (key, written) in [
            ("referrer_policy", headers.referrer_policy.as_deref()),
            (
                "content_security_policy",
                headers.content_security_policy.as_deref(),
            ),
            ("permissions_policy", headers.permissions_policy.as_deref()),
        ] {
            let Some(value) = written else { continue };
            if carriable(value) {
                continue;
            }
            return Err(Diagnostic::error(
                code::E_UNCARRIABLE_HEADER,
                format!("`[http.headers] {key}` holds a byte a header line cannot carry"),
            )
            .with_note(format!(
                "§ 1 writes this value onto every response verbatim, so a field value is printable \
                 ASCII and nothing else — a carriage return or a newline in it would end the \
                 header and begin one nobody wrote{}",
                origin_note(origins.get(&format!("http.headers.{key}")))
            ))
            .with_help(
                "write the policy on one line, or leave the key out for § 1's shipped default"
                    .to_string(),
            ));
        }
    }
    let cors = config.http.as_ref().and_then(|http| http.cors.as_ref());
    if let Some(cors) = cors {
        // § 2's lists reach a preflight's header lines exactly as § 1's policies reach an
        // ordinary answer's, so they are refused here on the same terms and under the same code.
        // `origins` is not among them: it is never written onto a response, only compared byte for
        // byte against an `Origin` a peer sent, and a value the wire delivered is carriable by
        // construction — so an uncarriable one there is unmatchable rather than unsendable.
        for (key, written) in [
            ("methods", cors.methods.as_deref()),
            ("headers", cors.headers.as_deref()),
            ("expose", cors.expose.as_deref()),
        ] {
            let Some(values) = written else { continue };
            if values.iter().all(|value| carriable(value)) {
                continue;
            }
            return Err(Diagnostic::error(
                code::E_UNCARRIABLE_HEADER,
                format!("`[http.cors] {key}` holds a byte a header line cannot carry"),
            )
            .with_note(format!(
                "§ 2 sends this list to a browser as one header line, so each entry is printable \
                 ASCII and nothing else — a carriage return or a newline in one would end the \
                 header and begin one nobody wrote{}",
                origin_note(origins.get(&format!("http.cors.{key}")))
            ))
            .with_help(
                "write one method or header name per entry, or leave the key out for § 2's \
                 shipped default"
                    .to_string(),
            ));
        }
        // `max_age` is written as a duration and sent as a number of seconds, and this is the only
        // place the crossing between them can be refused: `nvs_server::cors` resolves it at boot
        // with no arm for a value that is not a duration, which is what keeps a preflight from ever
        // being answered with a repaired number.
        if let Some(written) = cors.max_age.as_deref() {
            let key = "http.cors.max_age";
            let value = Setting::Text(written.to_owned());
            Quantity::parse(key, Unit::Duration, &value)
                .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
        }
    }
    // Before the pairs, because an unreadable `same_site` leaves § 3's question unanswerable: a
    // fourth spelling is not `None`, so the pair check would pass it and `Cookies::of` would then
    // have to choose between repairing it and failing inside a request. Refusing here is the only
    // arrangement where neither happens.
    let written = config
        .http
        .as_ref()
        .and_then(|http| http.cookies.as_ref())
        .and_then(|cookies| cookies.same_site.as_deref());
    if let Some(value) = written
        && SameSite::parse(value).is_none()
    {
        return Err(Diagnostic::error(
            code::E_BAD_SAME_SITE,
            format!("`[http.cookies] same_site` is `{value}`, which is not a `SameSite` attribute"),
        )
        .with_note(format!(
            "§ 3 names three and a browser parses three: `Lax`, `Strict` and `None`. A fourth \
             spelling is dropped by the browser, which leaves the cookie at that browser's own \
             default rather than at the one this block was written to state{}",
            origin_note(origins.get("http.cookies.same_site"))
        ))
        .with_help(
            "write `Lax`, `Strict` or `None` — or leave the key out, which is `Lax`".to_string(),
        ));
    }
    let Some(found) = Inbound::of(config.http.as_ref()).meaningless() else {
        return Ok(());
    };
    Err(
        Diagnostic::error(code::E_MEANINGLESS_HTTP_PAIR, found.what().to_string())
            .with_note(format!(
                "{}{}",
                found.note(),
                origin_note(origins.get(found.key()))
            ))
            .with_help(found.help().to_string()),
    )
}
