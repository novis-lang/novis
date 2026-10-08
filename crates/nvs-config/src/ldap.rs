//! ADR 0278 § 2's `[ldap.<name>]` blocks, judged once the merge is done: the URL list, the two `tls`
//! words, the credential, the timeout and the pool.
//!
//! **An empty password is refused when the block is read** (`rule:security/ldap-empty-password-is-refused`).
//! A block with a `user` and no password, or a password of `""`, would make every pooled connection an
//! unauthenticated bind, which Active Directory answers with success and the rights of nobody. The
//! client in `nvs_ldap` refuses the same bind before it writes a byte; this is the half that names the
//! file. A password with no `user` is refused too, because an anonymous bind sends none and the value
//! would be read and never used.
//!
//! **`tls = "none"` is half of a cleartext bind, never all of it**
//! (`rule:security/ldap-cleartext-bind-is-granted-per-host`). The other half is the host on
//! `[capabilities.ldap] cleartext`, which `Core\Ldap` asks at connect time, because a grant is asked of a
//! host and the grants are a request's own. What this module refuses is the word that contradicts its
//! URL: `tls = "none"` beside an `ldaps://` URL, which handshakes before anything else.
//!
//! **The URL is judged by its scheme here and parsed in full by `nvs_ldap::Url::parse`** when it is
//! dialled. That crate reads no configuration and this one depends on no protocol crate, so the one
//! parser stays below both and a host this module cannot read is the connect's own refusal.
//!
//! The `tls_ca_file` is resolved and trust-checked by [`canonicalize`] on `[db.<name>] tls_ca_file`'s
//! footing, and the password arrives from `password_file` through [`mod@crate::secret`] before
//! [`validate`] runs. Cost: one pass per tree at boot and at reload, and nothing per request.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use nvs_diagnostics::{Diagnostic, code};

use crate::db::PoolBounds;
use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, LdapDirectory, Setting};

/// How long one operation may take where the block names no `timeout`, connect and bind included.
/// There is no spelling for unlimited (ADR 0278 § 2).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// What a block's `tls` says: the encrypted default, or plain `ldap://` for a granted host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tls {
    /// `"required"`, or the key left out: `ldaps://` handshakes first and `ldap://` runs StartTLS
    /// before the bind.
    Required,
    /// `"none"`: no TLS on `ldap://`, for a host on `[capabilities.ldap] cleartext` alone.
    None,
}

/// The URLs a block names, in the order they are tried. A block [`validate`] accepted names at
/// least one.
#[must_use]
pub fn urls_of(block: &LdapDirectory) -> Vec<&str> {
    match block.url.as_ref() {
        Some(Setting::Text(url)) => vec![url.as_str()],
        Some(Setting::List(urls)) => urls.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    }
}

/// The block's `tls`. A word [`validate`] refuses never reaches a reader, so anything but `"none"`
/// reads as the encrypted default.
#[must_use]
pub fn tls_of(block: &LdapDirectory) -> Tls {
    if block.tls.as_deref() == Some("none") {
        Tls::None
    } else {
        Tls::Required
    }
}

/// The block's `timeout`, or [`DEFAULT_TIMEOUT`] where it names none.
///
/// # Errors
///
/// `E0601` for a value that is not a duration, for `false` and for zero: a timeout of nothing fails
/// every operation, and `false` would be the unlimited timeout the block has no spelling for.
pub fn timeout_for(
    name: &str,
    block: &LdapDirectory,
    origins: &BTreeMap<String, Origin>,
) -> Result<Duration, Diagnostic> {
    let key = format!("ldap.{name}.timeout");
    let help =
        "write how long one operation may take, as `30s`, or leave the key out for the default";
    let Some(written) = block.timeout.as_ref() else {
        return Ok(DEFAULT_TIMEOUT);
    };
    match crate::db::duration(&key, Some(written), origins, help)? {
        Some(zero) if zero.is_zero() => Err(crate::db::refuse(
            &key,
            &crate::value::as_written(written),
            "a timeout of zero stops every operation before it starts",
            help,
            origins,
        )),
        Some(timeout) => Ok(timeout),
        None => Ok(DEFAULT_TIMEOUT),
    }
}

/// The block's pool bounds, read as a `[db.<name>]` block's are.
///
/// # Errors
///
/// Exactly [`crate::db::pool_of`]'s, naming `ldap.<name>.pool`.
pub fn pool_for(
    name: &str,
    block: &LdapDirectory,
    origins: &BTreeMap<String, Origin>,
) -> Result<PoolBounds, Diagnostic> {
    crate::db::pool_of(&format!("ldap.{name}"), block.pool.as_ref(), origins)
}

/// Makes every `[ldap.<name>] tls_ca_file` absolute against the file that wrote it, and proves it is
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
    for (name, block) in &mut config.ldap {
        let Some(written) = block.tls_ca_file.as_deref() else {
            continue;
        };
        let base = crate::db::written_in(origins, &format!("ldap.{name}.tls_ca_file"));
        let path = crate::resolve::absolute(base, Path::new(written));
        let trusted = files.trust(&path).map_err(|why| {
            crate::resolve::untrusted(
                &path,
                &why,
                "an `[ldap]` block's `tls_ca_file` names it, and whoever can write it chooses which \
                 server the connection may be talking to",
            )
        })?;
        let trusted = trusted.to_string_lossy().into_owned();
        crate::db::rewrite_in(table, "ldap", name, "tls_ca_file", &trusted);
        block.tls_ca_file = Some(trusted);
    }
    Ok(())
}

/// Every `[ldap.<name>]` block, asked of the merged tree.
///
/// # Errors
///
/// `E0601` for the first block that names no URL, a URL that is not `ldap://` or `ldaps://`, a `tls`
/// that is neither word or is `"none"` beside an `ldaps://` URL, a `user` with an empty password, a
/// password with no `user`, or a timeout or pool [`timeout_for`] and [`pool_for`] refuse.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    for (name, block) in &config.ldap {
        urls(name, block, origins)?;
        tls(name, block, origins)?;
        credential(name, block, origins)?;
        timeout_for(name, block, origins)?;
        pool_for(name, block, origins)?;
    }
    Ok(())
}

fn urls(
    name: &str,
    block: &LdapDirectory,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let key = format!("ldap.{name}.url");
    let written = match block.url.as_ref() {
        Some(Setting::Text(_) | Setting::List(_)) => urls_of(block),
        Some(_) | None => Vec::new(),
    };
    if written.is_empty() {
        return Err(refusal(
            &key,
            format!("`[ldap.{name}]` has no `url`"),
            "The block needs the address of at least one directory server.",
            "write `url = \"ldaps://dc1.example.test\"`, or a list of URLs to try in order",
            origins,
        ));
    }
    for url in written {
        let rest = url
            .strip_prefix("ldaps://")
            .or_else(|| url.strip_prefix("ldap://"));
        if rest.is_none_or(|rest| rest.trim_end_matches('/').is_empty()) {
            return Err(refusal(
                &key,
                format!("`{url}` in `[ldap.{name}] url` is not an LDAP URL"),
                "Each URL starts with `ldaps://` or `ldap://`, followed by a host.",
                "write it as `ldaps://dc1.example.test` or `ldaps://dc1.example.test:636`",
                origins,
            ));
        }
    }
    Ok(())
}

fn tls(
    name: &str,
    block: &LdapDirectory,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let key = format!("ldap.{name}.tls");
    match block.tls.as_deref() {
        None | Some("required") => Ok(()),
        Some("none") => match urls_of(block)
            .into_iter()
            .find(|url| url.starts_with("ldaps://"))
        {
            Some(url) => Err(refusal(
                &key,
                format!("`[ldap.{name}]` has `tls = \"none\"` and the URL `{url}`"),
                "An `ldaps://` URL always uses TLS. `tls = \"none\"` is only for `ldap://` URLs.",
                "remove `tls = \"none\"`, or change the URL to `ldap://`",
                origins,
            )),
            None => Ok(()),
        },
        Some(other) => Err(refusal(
            &key,
            format!("`[ldap.{name}] tls = \"{other}\"` is not a TLS mode"),
            "`tls` is `\"required\"`, the default, or `\"none\"`.",
            "write `tls = \"required\"`, or leave the key out",
            origins,
        )),
    }
}

fn credential(
    name: &str,
    block: &LdapDirectory,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let password = block
        .password
        .as_deref()
        .filter(|password| !password.is_empty());
    match (block.user.as_deref(), password) {
        (Some(""), _) => Err(refusal(
            &format!("ldap.{name}.user"),
            format!("`[ldap.{name}] user` is empty"),
            "A block with no `user` binds anonymously. An empty `user` is not allowed.",
            "write the account to bind as, or remove `user` for an anonymous bind",
            origins,
        )),
        (Some(_), None) => {
            let key = if block.password_file.is_some() {
                format!("ldap.{name}.password_file")
            } else {
                format!("ldap.{name}.user")
            };
            Err(refusal(
                &key,
                format!("`[ldap.{name}]` has a `user` and an empty password"),
                "A bind with a user and an empty password logs in as nobody, and many servers \
                 report success. So an empty password is not allowed.",
                "write `password_file` with the account's password, or remove `user` for an \
                 anonymous bind",
                origins,
            ))
        }
        (None, _) if block.password.is_some() || block.password_file.is_some() => Err(refusal(
            &format!("ldap.{name}.password"),
            format!("`[ldap.{name}]` has a password and no `user`"),
            "An anonymous bind sends no password, so this password would never be used.",
            "write the `user` this password belongs to, or remove the password",
            origins,
        )),
        _ => Ok(()),
    }
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
