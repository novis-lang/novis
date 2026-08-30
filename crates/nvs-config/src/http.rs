//! [ADR 0074] §§ 2-3's two meaningless combinations, in the one implementation the boot and
//! `Core\Config::set` both ask.
//!
//! **The rule is written once and read from two places, and that asymmetry is the module.** § 2
//! refuses `origins = ["*"]` with `credentials = true` and § 3 refuses `same_site = "None"` with
//! `secure = false`, "at boot with the line named and at runtime by `Core\Config::set` returning
//! `false`". Two refusals in two shapes is a standing invitation to write the *condition* twice and
//! have them drift, which is why [`Inbound`] holds the four values the two questions are decided
//! from and [`Inbound::meaningless`] is the only place either question is answered.
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
//! Cost: four `bool`s built at boot, at reload, and once per `Core\Config::set` naming a key under
//! `[http.cors]` or `[http.cookies]`. Every other `set` returns before this module is reached.
//!
//! [ADR 0074]: ../../../docs/adr/0074-http-defaults-safe-and-finite.md

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Http};

/// The four values §§ 2-3's two refusals are decided from, resolved to what is in force.
///
/// Booleans and not the blocks themselves, because the callers read from different places — a typed
/// tree at boot, a snapshot plus an overlay plus a proposed assignment inside a request — and the
/// only thing they have to agree on is the answer to these four questions.
///
/// Built by [`Inbound::of`] so the defaults are stated once; there is deliberately no `Default`
/// impl, because `bool`'s own default is `false` and two of these four default to `true`.
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
    /// `origins` is absent on purpose: it is a list, and § 5 of ADR 0064 crosses values as text, so
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

/// Whether a `same_site` value is § 3's `None`.
///
/// Case-insensitive, because the attribute a browser parses is, so a tree writing `none` has
/// configured the same cookie and must reach the same refusal.
fn is_none_same_site(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("none")
}

/// A boolean as `Core\Config::set` crosses it: ADR 0064 § 5 sends values as text, and a directive
/// the file wrote as a TOML boolean reads back as `true` or `false`.
fn is_true(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("true")
}

/// §§ 2-3's two refusals, asked of the merged tree at boot.
///
/// # Errors
///
/// One [`Diagnostic`], `E0612`, for the first pair with no correct meaning: `origins = ["*"]` with
/// `credentials = true`, or `same_site = "None"` with `secure = false`.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
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
