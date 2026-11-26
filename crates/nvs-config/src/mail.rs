//! The `[mail.<name>]` blocks of `rule:programs/framework-core-half`, judged once the merge is done:
//! the `tls` and `auth` words, the credential they go with, the `oauth_*` keys an `xoauth2` block
//! fetches its token with, the `tls_ca_file` and the pool.
//!
//! **A credential never travels in the clear.** `tls = "none"` beside a `user` or a password is
//! refused here, naming the file, so the send that would have refused it later never runs.
//! `nvs_stdlib::mail`'s module doc is the home of why TLS is asked for by a credential, and why
//! `starttls` with no credential is still required and verified.
//!
//! **A word that would be read and never used is refused too:** an `auth` with no credential, and a
//! `tls_ca_file` beside `tls = "none"`.
//!
//! The `tls_ca_file` is resolved and trust-checked by [`canonicalize`] on `[db.<name>]
//! tls_ca_file`'s footing, and the password arrives from `password_file` through
//! [`mod@crate::secret`] before [`validate`] runs. Cost: one pass per tree at boot and at reload,
//! and nothing per request.

use std::collections::BTreeMap;
use std::path::Path;

use nvs_diagnostics::{Diagnostic, code};

use crate::db::PoolBounds;
use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, MailEndpoint};

/// The port that speaks TLS from its first byte (RFC 8314), and so the one an unset `tls` reads as
/// `implicit` on.
pub const IMPLICIT_PORT: u16 = 465;

/// How a block reaches its relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tls {
    /// TLS from the first byte, before the greeting.
    Implicit,
    /// The greeting in the clear, then `STARTTLS` before anything else. A relay that does not
    /// offer it is refused.
    StartTls,
    /// No TLS at all, for a block with no credential.
    None,
}

/// The mechanism a credential is sent with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auth {
    /// RFC 4616's `PLAIN`.
    Plain,
    /// The `LOGIN` mechanism: the user and the password, each in its own base64 line.
    Login,
    /// Google's and Microsoft's `XOAUTH2`: the user and an OAuth bearer token the host fetched
    /// from the block's `oauth_token_url`.
    XOAuth2,
}

impl Auth {
    /// The mechanism's name on the `AUTH` line.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Plain => "PLAIN",
            Self::Login => "LOGIN",
            Self::XOAuth2 => "XOAUTH2",
        }
    }
}

/// Where an `auth = "xoauth2"` block's token comes from: the parts of its `oauth_token_url`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenUrl {
    /// The host the certificate is checked against, without brackets for an IPv6 literal.
    pub host: String,
    /// The port, 443 where the URL names none.
    pub port: u16,
    /// The path and query the request names, `/` where the URL has none.
    pub target: String,
}

/// The `oauth_token_url` `written`, or `None` when it is not an `https://` URL with a host and no
/// user, fragment, space or control character.
#[must_use]
pub fn token_url(written: &str) -> Option<TokenUrl> {
    let rest = written.strip_prefix("https://")?;
    if rest.bytes().any(|byte| byte <= b' ' || byte == 0x7f) || rest.contains('#') {
        return None;
    }
    let (authority, target) = match rest.find(['/', '?']) {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if authority.contains('@') {
        return None;
    }
    let (host, port) = if let Some(literal) = authority.strip_prefix('[') {
        let (host, after) = literal.split_once(']')?;
        let port = match after {
            "" => None,
            _ => Some(after.strip_prefix(':')?),
        };
        (host, port)
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    let port = match port {
        Some(port) => port.parse::<u16>().ok().filter(|port| *port != 0)?,
        None => 443,
    };
    if host.is_empty() {
        return None;
    }
    let target = if target.starts_with('?') {
        format!("/{target}")
    } else {
        target.to_owned()
    };
    Some(TokenUrl {
        host: host.to_owned(),
        port,
        target,
    })
}

/// The first `oauth_*` key `block` sets, by its name.
fn oauth_written(block: &MailEndpoint) -> Option<&'static str> {
    [
        ("oauth_token_url", block.oauth_token_url.is_some()),
        ("oauth_client_id", block.oauth_client_id.is_some()),
        ("oauth_client_secret", block.oauth_client_secret.is_some()),
        (
            "oauth_client_secret_file",
            block.oauth_client_secret_file.is_some(),
        ),
        ("oauth_scope", block.oauth_scope.is_some()),
        ("oauth_refresh_token", block.oauth_refresh_token.is_some()),
        (
            "oauth_refresh_token_file",
            block.oauth_refresh_token_file.is_some(),
        ),
    ]
    .into_iter()
    .find_map(|(key, set)| set.then_some(key))
}

/// The bounds an unwritten `[mail.<name>.pool]` key takes: `max = 4`, `idle = 2`,
/// `lifetime = "5m"` and `acquire = "5s"`, from [`crate::defaults`].
pub const POOL_DEFAULTS: PoolBounds = PoolBounds {
    enabled: true,
    max: crate::defaults::MAIL_POOL_MAX,
    idle: crate::defaults::MAIL_POOL_IDLE,
    lifetime: crate::defaults::MAIL_POOL_LIFETIME,
    acquire: crate::defaults::MAIL_POOL_ACQUIRE,
};

/// The block's pool bounds, read as a `[db.<name>]` block's are, over [`POOL_DEFAULTS`].
///
/// # Errors
///
/// Exactly [`crate::db::pool_of`]'s, naming `mail.<name>.pool`.
pub fn pool_for(
    name: &str,
    block: &MailEndpoint,
    origins: &BTreeMap<String, Origin>,
) -> Result<PoolBounds, Diagnostic> {
    crate::db::pool_over(
        &format!("mail.{name}"),
        block.pool.as_ref(),
        POOL_DEFAULTS,
        origins,
    )
}

/// The `tls` a block means: `written` when it is a word, else the default for `port` and whether a
/// credential is set. `None` for a word that is not a mode, which [`validate`] refuses at boot.
#[must_use]
pub fn tls_from(written: Option<&str>, port: u16, credential: bool) -> Option<Tls> {
    match written {
        Some("implicit") => Some(Tls::Implicit),
        Some("starttls") => Some(Tls::StartTls),
        Some("none") => Some(Tls::None),
        Some(_) => None,
        None if port == IMPLICIT_PORT => Some(Tls::Implicit),
        None if credential => Some(Tls::StartTls),
        None => Some(Tls::None),
    }
}

/// The `auth` a block names, `Ok(None)` where it names none.
///
/// # Errors
///
/// The word itself, for any word but `plain`, `login` and `xoauth2`. [`validate`] refuses it at
/// boot.
pub fn auth_from(written: Option<&str>) -> Result<Option<Auth>, &str> {
    match written {
        None => Ok(None),
        Some("plain") => Ok(Some(Auth::Plain)),
        Some("login") => Ok(Some(Auth::Login)),
        Some("xoauth2") => Ok(Some(Auth::XOAuth2)),
        Some(other) => Err(other),
    }
}

/// Makes every `[mail.<name>] tls_ca_file` absolute against the file that wrote it, and proves it is
/// inside the trust boundary — [`crate::db::canonicalize`]'s pass, for this family.
///
/// # Errors
///
/// `E0607` for a bundle outside the trust boundary and `E0605` for one that cannot be read.
pub fn canonicalize(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    for (name, block) in &mut config.mail {
        let Some(written) = block.tls_ca_file.as_deref() else {
            continue;
        };
        let base = crate::db::written_in(origins, &format!("mail.{name}.tls_ca_file"));
        let path = crate::resolve::absolute(base, Path::new(written));
        let trusted = files.trust(&path).map_err(|why| {
            crate::resolve::untrusted(
                &path,
                &why,
                "a `[mail]` block's `tls_ca_file` names it, and whoever can write it chooses which \
                 relay the mail may be sent to",
            )
        })?;
        let trusted = trusted.to_string_lossy().into_owned();
        crate::db::rewrite_in(table, "mail", name, "tls_ca_file", &trusted);
        block.tls_ca_file = Some(trusted);
    }
    Ok(())
}

/// Every `[mail.<name>]` block, asked of the merged tree.
///
/// # Errors
///
/// `E0601` for the first block whose `tls` is not a mode, whose `tls = "none"` is beside a
/// credential or a `tls_ca_file`, whose `auth` is not a mechanism or has no credential to send,
/// whose `oauth_*` keys [`oauth`] refuses, or whose `[mail.<name>.pool]` bounds
/// [`crate::db::pool_of`] refuses.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    for (name, block) in &config.mail {
        judge(name, block, origins)?;
        pool_for(name, block, origins)?;
    }
    Ok(())
}

fn judge(
    name: &str,
    block: &MailEndpoint,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let credential =
        block.user.is_some() || block.password.is_some() || block.password_file.is_some();
    let port = block.port.unwrap_or(crate::defaults::MAIL_PORT);
    let tls_key = format!("mail.{name}.tls");
    match tls_from(block.tls.as_deref(), port, credential) {
        None => {
            return Err(refusal(
                &tls_key,
                format!(
                    "`[mail.{name}] tls = \"{}\"` is not a TLS mode",
                    block.tls.as_deref().unwrap_or_default()
                ),
                "`tls` is `\"implicit\"`, `\"starttls\"` or `\"none\"`.",
                "write one of the three, or leave the key out for the default",
                origins,
            ));
        }
        Some(Tls::None) if credential => {
            return Err(refusal(
                &tls_key,
                format!("`[mail.{name}]` has `tls = \"none\"` and a credential"),
                "A password is never sent without TLS.",
                "write `tls = \"starttls\"` or `tls = \"implicit\"`, or remove the credential",
                origins,
            ));
        }
        Some(Tls::None) if block.tls_ca_file.is_some() => {
            return Err(refusal(
                &format!("mail.{name}.tls_ca_file"),
                format!("`[mail.{name}]` has a `tls_ca_file` and no TLS"),
                "The block sends in the clear, so the certificates would never be read.",
                "write `tls = \"starttls\"` or `tls = \"implicit\"`, or remove `tls_ca_file`",
                origins,
            ));
        }
        Some(_) => {}
    }
    let auth_key = format!("mail.{name}.auth");
    match auth_from(block.auth.as_deref()) {
        Err(word) => Err(refusal(
            &auth_key,
            format!("`[mail.{name}] auth = \"{word}\"` is not a mechanism"),
            "`auth` is `\"plain\"`, `\"login\"` or `\"xoauth2\"`.",
            "write one of the three, or leave the key out for the default",
            origins,
        )),
        Ok(Some(Auth::XOAuth2)) => oauth(name, block, origins),
        Ok(_) if oauth_written(block).is_some() => {
            let key = oauth_written(block).unwrap_or_default();
            Err(refusal(
                &format!("mail.{name}.{key}"),
                format!("`[mail.{name}]` sets `{key}` and its `auth` is not `\"xoauth2\"`"),
                "Only an `xoauth2` block fetches a token, so the key would never be read.",
                "write `auth = \"xoauth2\"`, or remove the `oauth_*` keys",
                origins,
            ))
        }
        Ok(Some(_)) if !credential => Err(refusal(
            &auth_key,
            format!("`[mail.{name}]` has an `auth` and no credential"),
            "`auth` says how the user and password are sent, and the block has none.",
            "write `user` and `password`, or remove `auth`",
            origins,
        )),
        Ok(_) => Ok(()),
    }
}

/// An `auth = "xoauth2"` block: a `user` and no password, an `https://` token URL, a client id
/// and a client secret.
fn oauth(
    name: &str,
    block: &MailEndpoint,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let key = |key: &str| format!("mail.{name}.{key}");
    if block.user.is_none() {
        return Err(refusal(
            &key("auth"),
            format!("`[mail.{name}]` has `auth = \"xoauth2\"` and no `user`"),
            "The token signs in one mailbox, and `user` names it.",
            "write `user`, the address the token signs in as",
            origins,
        ));
    }
    if block.password.is_some() || block.password_file.is_some() {
        return Err(refusal(
            &key("password"),
            format!("`[mail.{name}]` has `auth = \"xoauth2\"` and a password"),
            "An `xoauth2` block signs in with a token, so the password would never be read.",
            "remove `password` or `password_file`",
            origins,
        ));
    }
    match block.oauth_token_url.as_deref() {
        None => {
            return Err(refusal(
                &key("auth"),
                format!("`[mail.{name}]` has `auth = \"xoauth2\"` and no `oauth_token_url`"),
                "The token is fetched from that URL.",
                "write `oauth_token_url`, the `https://` URL of the token server",
                origins,
            ));
        }
        Some(url) if token_url(url).is_none() => {
            return Err(refusal(
                &key("oauth_token_url"),
                format!("`[mail.{name}] oauth_token_url = \"{url}\"` is not an `https://` URL"),
                "The token request carries the client secret, and it is only sent over TLS.",
                "write the token server's `https://` URL",
                origins,
            ));
        }
        Some(_) => {}
    }
    for (written, wanted) in [
        (block.oauth_client_id.is_some(), "oauth_client_id"),
        (block.oauth_client_secret.is_some(), "oauth_client_secret"),
    ] {
        if !written {
            return Err(refusal(
                &key("auth"),
                format!("`[mail.{name}]` has `auth = \"xoauth2\"` and no `{wanted}`"),
                "The token server needs the client's id and secret.",
                &format!("write `{wanted}`"),
                origins,
            ));
        }
    }
    Ok(())
}

fn refusal(
    key: &str,
    message: String,
    why: &str,
    help: &str,
    origins: &BTreeMap<String, Origin>,
) -> Diagnostic {
    Diagnostic::error(code::E_BAD_DIRECTIVE, message)
        .with_note(format!("{why}{}", origin_note(origins.get(key))))
        .with_help(help.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The defaults `tls` reads as, and the two blocks a boot refuses: a credential in the clear
    /// and a mode that is not one of the three words.
    #[test]
    fn a_mail_block_s_tls_defaults_and_refusals() {
        assert_eq!(tls_from(None, 465, false), Some(Tls::Implicit));
        assert_eq!(tls_from(None, 587, true), Some(Tls::StartTls));
        assert_eq!(tls_from(None, 25, false), Some(Tls::None));
        assert_eq!(tls_from(Some("starttls"), 25, false), Some(Tls::StartTls));
        assert_eq!(tls_from(Some("ssl"), 25, false), None);

        let origins = BTreeMap::new();
        let block = |tls: Option<&str>, user: bool, auth: Option<&str>| MailEndpoint {
            tls: tls.map(str::to_owned),
            user: user.then(|| "app".to_owned()),
            password: user.then(|| "s3cret".to_owned()),
            auth: auth.map(str::to_owned),
            ..MailEndpoint::default()
        };
        assert!(judge("relay", &block(Some("none"), true, None), &origins).is_err());
        assert!(judge("relay", &block(Some("ssl"), false, None), &origins).is_err());
        assert!(judge("relay", &block(None, false, Some("login")), &origins).is_err());
        assert!(judge("relay", &block(None, true, Some("cram-md5")), &origins).is_err());
        assert!(judge("relay", &block(None, true, Some("login")), &origins).is_ok());
        assert!(judge("relay", &block(Some("starttls"), false, None), &origins).is_ok());
    }

    /// An `xoauth2` block whose `oauth_token_url` is not `https://` does not boot, and the error
    /// names the key. So do a password beside `xoauth2`, a missing client secret, and an `oauth_*`
    /// key on a block that does not use `xoauth2`.
    #[test]
    fn a_token_url_that_is_not_https_is_a_settings_error() {
        let origins = BTreeMap::new();
        let block = |url: &str| MailEndpoint {
            user: Some("noreply@example.com".to_owned()),
            auth: Some("xoauth2".to_owned()),
            oauth_token_url: Some(url.to_owned()),
            oauth_client_id: Some("app".to_owned()),
            oauth_client_secret: Some("s3cret".to_owned()),
            ..MailEndpoint::default()
        };
        assert!(judge("relay", &block("https://login.example.com/token"), &origins).is_ok());
        for url in [
            "http://login.example.com/token",
            "https://",
            "https://user@login.example.com/token",
            "https://login.example.com:0/token",
            "login.example.com/token",
        ] {
            let Err(refused) = judge("relay", &block(url), &origins) else {
                panic!("{url} was taken as a token URL");
            };
            assert!(
                format!("{refused:?}").contains("oauth_token_url"),
                "{refused:?}"
            );
        }

        let mut with_password = block("https://login.example.com/token");
        with_password.password = Some("s3cret".to_owned());
        assert!(judge("relay", &with_password, &origins).is_err());
        let mut without_secret = block("https://login.example.com/token");
        without_secret.oauth_client_secret = None;
        assert!(judge("relay", &without_secret, &origins).is_err());
        let mut plain = block("https://login.example.com/token");
        plain.auth = Some("plain".to_owned());
        plain.password = Some("s3cret".to_owned());
        assert!(judge("relay", &plain, &origins).is_err());

        assert_eq!(
            token_url("https://[::1]:8443?a=b"),
            Some(TokenUrl {
                host: "::1".to_owned(),
                port: 8443,
                target: "/?a=b".to_owned(),
            })
        );
    }
}
