//! The `[mail.<name>]` blocks of `rule:programs/framework-core-half`, judged once the merge is done:
//! the `tls` and `auth` words, the credential they go with, the `oauth_*` keys an `xoauth2` block
//! fetches its token with, the `dkim_*` keys a block signs with, the `tls_ca_file` and the pool.
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
//! tls_ca_file`'s footing. So is the `dkim_key_file`, which [`canonicalize`] also reads once so
//! [`dkim_key`] refuses an RSA key under 2048 bits at boot. The password arrives from `password_file` through
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

/// The kind of key a `dkim_key_file` holds, and so the `a=` the signature names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DkimAlgorithm {
    /// `rsa-sha256`, RFC 6376's.
    Rsa,
    /// `ed25519-sha256`, RFC 8463's.
    Ed25519,
}

/// A `dkim_key_file`'s key, read and judged: its kind and its PKCS#8 DER.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DkimKey {
    /// Which signature the key makes.
    pub algorithm: DkimAlgorithm,
    /// The PKCS#8 `PrivateKeyInfo`, as the PEM carried it.
    pub pkcs8: Vec<u8>,
}

/// The narrowest RSA key a block may sign with. RFC 8301 makes 1024 bits the floor a verifier
/// accepts, and every large receiver asks for 2048.
pub const DKIM_RSA_MIN_BITS: usize = 2048;
/// The widest RSA key a block may sign with, the widest `ring` signs with.
pub const DKIM_RSA_MAX_BITS: usize = 4096;

/// The DER of `1.2.840.113549.1.1.1`, `rsaEncryption`.
const RSA_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
/// The DER of `1.3.101.112`, `id-Ed25519`.
const ED25519_OID: &[u8] = &[0x2b, 0x65, 0x70];

/// The key a `dkim_key_file` holds, from its PEM text.
///
/// The width of an RSA key is read from the modulus here, at boot, so a key a receiver would
/// refuse never signs a message. The rest of the key is checked by `ring` when it is first used.
///
/// # Errors
///
/// A sentence for anything but one PKCS#8 `PRIVATE KEY` block of an RSA key of
/// [`DKIM_RSA_MIN_BITS`] to [`DKIM_RSA_MAX_BITS`] or an Ed25519 key.
pub fn dkim_key(pem: &str) -> Result<DkimKey, String> {
    use base64::Engine as _;
    const BEGIN: &str = "-----BEGIN PRIVATE KEY-----";
    const END: &str = "-----END PRIVATE KEY-----";
    let not_pkcs8 = || {
        "it is not a PKCS#8 `PRIVATE KEY` in PEM. Convert a key with \
         `openssl pkcs8 -topk8 -nocrypt -in old.pem -out dkim.pem`"
            .to_owned()
    };
    let start = pem.find(BEGIN).ok_or_else(not_pkcs8)? + BEGIN.len();
    let end = start + pem[start..].find(END).ok_or_else(not_pkcs8)?;
    let base64: String = pem[start..end]
        .chars()
        .filter(|each| !each.is_ascii_whitespace())
        .collect();
    let der = base64::engine::general_purpose::STANDARD
        .decode(base64)
        .map_err(|_| not_pkcs8())?;

    let malformed = || "its PKCS#8 structure cannot be read".to_owned();
    let (tag, info, _) = der_element(&der).ok_or_else(malformed)?;
    if tag != 0x30 {
        return Err(malformed());
    }
    let (_, _, rest) = der_element(info)
        .filter(|(tag, ..)| *tag == 0x02)
        .ok_or_else(malformed)?;
    let (tag, algorithm, rest) = der_element(rest).ok_or_else(malformed)?;
    let (_, oid, _) = der_element(algorithm)
        .filter(|(tag, ..)| *tag == 0x06)
        .ok_or_else(malformed)?;
    if tag != 0x30 {
        return Err(malformed());
    }
    let (tag, private, _) = der_element(rest).ok_or_else(malformed)?;
    if tag != 0x04 {
        return Err(malformed());
    }
    let algorithm = if oid == RSA_OID {
        let bits = rsa_bits(private).ok_or_else(malformed)?;
        if !(DKIM_RSA_MIN_BITS..=DKIM_RSA_MAX_BITS).contains(&bits) {
            return Err(format!(
                "it is an RSA key of {bits} bits, and a DKIM key is an RSA key of \
                 {DKIM_RSA_MIN_BITS} to {DKIM_RSA_MAX_BITS} bits or an Ed25519 key"
            ));
        }
        DkimAlgorithm::Rsa
    } else if oid == ED25519_OID {
        DkimAlgorithm::Ed25519
    } else {
        return Err(
            "it is a key of another kind, and a DKIM key is an RSA key or an Ed25519 key"
                .to_owned(),
        );
    };
    Ok(DkimKey {
        algorithm,
        pkcs8: der,
    })
}

/// One DER element at the start of `octets`: its tag, its content and what follows it.
fn der_element(octets: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = octets.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (length, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let (bytes, rest) = rest.split_at(count);
        let length = bytes
            .iter()
            .fold(0_usize, |sum, byte| (sum << 8) | usize::from(*byte));
        (length, rest)
    };
    (rest.len() >= length).then(|| (tag, &rest[..length], &rest[length..]))
}

/// The width of the modulus of a PKCS#1 `RSAPrivateKey`, in bits.
fn rsa_bits(key: &[u8]) -> Option<usize> {
    let (tag, fields, _) = der_element(key)?;
    let (version, _, rest) = (tag == 0x30).then(|| der_element(fields)).flatten()?;
    let (tag, modulus, _) = (version == 0x02).then(|| der_element(rest)).flatten()?;
    if tag != 0x02 {
        return None;
    }
    let at = modulus.iter().position(|byte| *byte != 0)?;
    let lead = modulus[at];
    Some((modulus.len() - at - 1) * 8 + (8 - lead.leading_zeros() as usize))
}

/// Whether `written` is a domain name in ASCII: dot-separated labels of letters, digits and `-`,
/// each 1 to 63 bytes, none starting or ending with `-`.
fn is_ascii_domain(written: &str) -> bool {
    !written.is_empty()
        && written.len() <= 253
        && written.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
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
/// `E0607` for a bundle or a DKIM key outside the trust boundary, `E0605` for one that cannot be
/// read, and `E0601` for a DKIM key [`dkim_key`] refuses.
pub fn canonicalize(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    for (name, block) in &mut config.mail {
        if let Some(written) = block.dkim_key_file.clone() {
            let key = format!("mail.{name}.dkim_key_file");
            let base = crate::db::written_in(origins, &key);
            let path = crate::resolve::absolute(base, Path::new(&written));
            let trusted = files.trust(&path).map_err(|why| {
                crate::resolve::untrusted(
                    &path,
                    &why,
                    "a `[mail]` block's `dkim_key_file` names it, and whoever can write it can sign \
                     mail as the domain",
                )
            })?;
            let text = files.read(&trusted).map_err(|why| {
                crate::resolve::unreadable(
                    &trusted,
                    &why,
                    "a `[mail]` block's `dkim_key_file` names it",
                )
            })?;
            dkim_key(&text).map_err(|why| {
                refusal(
                    &key,
                    format!("`[mail.{name}] dkim_key_file` cannot sign mail: {why}"),
                    "Every message the block sends is signed with this key.",
                    "write the path of a PKCS#8 PEM file with an RSA key of 2048 bits or more, \
                     or an Ed25519 key",
                    origins,
                )
            })?;
            let trusted = trusted.to_string_lossy().into_owned();
            crate::db::rewrite_in(table, "mail", name, "dkim_key_file", &trusted);
            block.dkim_key_file = Some(trusted);
        }
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
    if block.capture == Some(true) {
        return capture(name, block, origins);
    }
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
    dkim(name, block, origins)?;
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

/// A `capture = true` block: a `from`, and none of the keys only a connection reads. Each of those
/// would be read and never used, and a `host` beside `capture` reads as a block that sends.
fn capture(
    name: &str,
    block: &MailEndpoint,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    if block.from.is_none() {
        return Err(refusal(
            &format!("mail.{name}.capture"),
            format!("`[mail.{name}]` has `capture = true` and no `from`"),
            "A kept message has the `From:` header a sent one has.",
            "write `from`, the address the mail is sent from",
            origins,
        ));
    }
    let connection = [
        ("host", block.host.is_some()),
        ("port", block.port.is_some()),
        ("user", block.user.is_some()),
        ("password", block.password.is_some()),
        ("password_file", block.password_file.is_some()),
        ("timeout", block.timeout.is_some()),
        ("tls", block.tls.is_some()),
        ("tls_ca_file", block.tls_ca_file.is_some()),
        ("auth", block.auth.is_some()),
        ("dkim_selector", block.dkim_selector.is_some()),
        ("dkim_key_file", block.dkim_key_file.is_some()),
        ("dkim_domain", block.dkim_domain.is_some()),
        ("pool", block.pool.is_some()),
    ];
    let written = connection
        .iter()
        .find(|(_, set)| *set)
        .map(|(key, _)| *key)
        .or_else(|| oauth_written(block));
    match written {
        Some(key) => Err(refusal(
            &format!("mail.{name}.{key}"),
            format!("`[mail.{name}]` has `capture = true` and sets `{key}`"),
            "A capture block keeps its mail and opens no connection, so the key would never be \
             read.",
            &format!("remove `{key}`, or remove `capture` to send the mail"),
            origins,
        )),
        None => Ok(()),
    }
}

/// A block's `dkim_*` keys: `dkim_selector` and `dkim_key_file` both or neither, `dkim_domain` only
/// beside them, and the selector and the domain each a domain name in ASCII. The key itself is
/// read by [`canonicalize`].
fn dkim(
    name: &str,
    block: &MailEndpoint,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let key = |key: &str| format!("mail.{name}.{key}");
    match (&block.dkim_selector, &block.dkim_key_file) {
        (Some(_), Some(_)) => {}
        (None, None) if block.dkim_domain.is_some() => {
            return Err(refusal(
                &key("dkim_domain"),
                format!("`[mail.{name}]` sets `dkim_domain` and no DKIM key"),
                "The domain is only written into a signature, and the block signs nothing.",
                "write `dkim_selector` and `dkim_key_file`, or remove `dkim_domain`",
                origins,
            ));
        }
        (None, None) => return Ok(()),
        (selector, _) => {
            let (written, missing) = if selector.is_some() {
                ("dkim_selector", "dkim_key_file")
            } else {
                ("dkim_key_file", "dkim_selector")
            };
            return Err(refusal(
                &key(written),
                format!("`[mail.{name}]` sets `{written}` and no `{missing}`"),
                "A signature needs the key and the selector a receiver finds its public half by.",
                &format!("write `{missing}`, or remove `{written}`"),
                origins,
            ));
        }
    }
    for (wanted, written) in [
        ("dkim_selector", block.dkim_selector.as_deref()),
        ("dkim_domain", block.dkim_domain.as_deref()),
    ] {
        if let Some(written) = written.filter(|written| !is_ascii_domain(written)) {
            return Err(refusal(
                &key(wanted),
                format!("`[mail.{name}] {wanted} = \"{written}\"` is not a domain name"),
                "A receiver looks the public key up in DNS under this name.",
                "write labels of letters, digits and `-`, separated by dots",
                origins,
            ));
        }
    }
    Ok(())
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

    /// A capture block boots with a `from` alone. With no `from`, or with a key only a connection
    /// reads, it does not, and the error names the key.
    #[test]
    fn a_capture_block_needs_from_and_no_host() {
        let origins = BTreeMap::new();
        let capture = MailEndpoint {
            capture: Some(true),
            from: Some("noreply@example.com".to_owned()),
            from_name: Some("Shop".to_owned()),
            return_path: Some("bounces@example.com".to_owned()),
            ..MailEndpoint::default()
        };
        assert!(judge("dev", &capture, &origins).is_ok());

        let without_from = MailEndpoint {
            from: None,
            ..capture.clone()
        };
        let refused = judge("dev", &without_from, &origins).unwrap_err();
        assert!(format!("{refused:?}").contains("no `from`"), "{refused:?}");

        let with_host = MailEndpoint {
            host: Some("smtp.example.com".to_owned()),
            ..capture.clone()
        };
        let refused = judge("dev", &with_host, &origins).unwrap_err();
        assert!(format!("{refused:?}").contains("`host`"), "{refused:?}");

        let with_oauth = MailEndpoint {
            oauth_client_id: Some("app".to_owned()),
            ..capture
        };
        let refused = judge("dev", &with_oauth, &origins).unwrap_err();
        assert!(
            format!("{refused:?}").contains("oauth_client_id"),
            "{refused:?}"
        );
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

    /// A DER element with a length of up to two bytes.
    fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
        let length = u16::try_from(content.len()).unwrap();
        let [high, low] = length.to_be_bytes();
        let mut out = vec![tag];
        match length {
            0..0x80 => out.push(low),
            0x80..0x100 => out.extend([0x81, low]),
            _ => out.extend([0x82, high, low]),
        }
        out.extend_from_slice(content);
        out
    }

    /// The PEM of a PKCS#8 RSA key whose modulus is `bits` wide. The other values are placeholders,
    /// because the width is all the boot reads.
    fn rsa_pem(bits: usize) -> String {
        use base64::Engine as _;
        let mut modulus = vec![0_u8; bits / 8 + 1];
        modulus[1] = 0x80;
        let pkcs1 = tlv(
            0x30,
            &[tlv(0x02, &[0]), tlv(0x02, &modulus), tlv(0x02, &[1, 0, 1])].concat(),
        );
        let der = tlv(
            0x30,
            &[
                tlv(0x02, &[0]),
                tlv(0x30, &[tlv(0x06, RSA_OID), vec![0x05, 0x00]].concat()),
                tlv(0x04, &pkcs1),
            ]
            .concat(),
        );
        let body = base64::engine::general_purpose::STANDARD.encode(der);
        format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n")
    }

    /// An RSA key under 2048 bits does not boot, and the error names its width. A 2048-bit key
    /// and an Ed25519 key are read, and a PKCS#1 key gets the command that converts it. The
    /// selector needs the key file, and both must be domain names.
    #[test]
    fn a_key_shorter_than_2048_bits_is_a_settings_error() {
        let short = dkim_key(&rsa_pem(1024)).unwrap_err();
        assert!(short.contains("1024 bits"), "{short}");
        assert!(dkim_key(&rsa_pem(2047)).is_err());
        assert_eq!(
            dkim_key(&rsa_pem(2048)).unwrap().algorithm,
            DkimAlgorithm::Rsa
        );
        assert!(dkim_key(&rsa_pem(4096)).is_ok());
        assert!(dkim_key(&rsa_pem(8192)).is_err());
        let ed25519 = "-----BEGIN PRIVATE KEY-----\n\
                       MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g\n\
                       -----END PRIVATE KEY-----\n";
        assert_eq!(dkim_key(ed25519).unwrap().algorithm, DkimAlgorithm::Ed25519);
        let pkcs1 =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAK==\n-----END RSA PRIVATE KEY-----\n";
        assert!(dkim_key(pkcs1).unwrap_err().contains("openssl pkcs8"));

        let origins = BTreeMap::new();
        let block = |selector: Option<&str>, file: bool, domain: Option<&str>| MailEndpoint {
            dkim_selector: selector.map(str::to_owned),
            dkim_key_file: file.then(|| "dkim.pem".to_owned()),
            dkim_domain: domain.map(str::to_owned),
            ..MailEndpoint::default()
        };
        assert!(judge("relay", &block(Some("mail2026"), true, None), &origins).is_ok());
        assert!(
            judge(
                "relay",
                &block(Some("a.b"), true, Some("example.com")),
                &origins
            )
            .is_ok()
        );
        assert!(judge("relay", &block(Some("mail2026"), false, None), &origins).is_err());
        assert!(judge("relay", &block(None, true, None), &origins).is_err());
        assert!(judge("relay", &block(None, false, Some("example.com")), &origins).is_err());
        assert!(judge("relay", &block(Some("bad selector"), true, None), &origins).is_err());
        assert!(judge("relay", &block(Some("s"), true, Some("-x.com")), &origins).is_err());
    }
}
