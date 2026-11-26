//! The `[imap.<name>]` blocks of `rule:core-classes/imap`, judged once the merge is done: the `tls`
//! and `auth` words, the credential, the `oauth_*` keys of an `xoauth2` block, the `tls_ca_file`
//! and the `directory` a block reads in place of a server.
//!
//! **Every session is TLS.** `tls` is `implicit` (the default, port 993) or `starttls`, and
//! there is no word for the clear: a mailbox session signs in, and a password is never sent
//! without TLS. `[mail.<name>]` has `none` because an internal relay takes mail with no
//! credential. An IMAP server always needs one, so the word would only send a password in the
//! clear.
//!
//! **A word that would be read and never used is refused**, as a mail block's is: an `oauth_*` key
//! on a block whose `auth` is not `xoauth2`, a password beside `xoauth2`, and any key only a
//! connection reads beside `directory`.
//!
//! The `tls_ca_file` is resolved and trust-checked by [`canonicalize`] on `[db.<name>]
//! tls_ca_file`'s footing, and the `directory` is made absolute against the file that wrote it, as
//! a `[storage.<name>] root` is. Cost: one pass per tree at boot and at reload, and nothing per
//! request.

use std::collections::BTreeMap;
use std::path::Path;

use nvs_diagnostics::{Diagnostic, code};

use crate::mail::{Auth, Tls, auth_from, token_url};
use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, ImapMailbox};

/// The `tls` a block means, `None` for a word that is not `implicit` or `starttls`.
#[must_use]
pub fn tls_from(written: Option<&str>) -> Option<Tls> {
    match written {
        None | Some("implicit") => Some(Tls::Implicit),
        Some("starttls") => Some(Tls::StartTls),
        Some(_) => None,
    }
}

/// Makes every `[imap.<name>] tls_ca_file` absolute and trust-checked, and every `directory`
/// absolute, against the file that wrote it.
///
/// # Errors
///
/// `E0607` for a bundle outside the trust boundary.
pub fn canonicalize(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    for (name, block) in &mut config.imap {
        if let Some(written) = block
            .directory
            .as_deref()
            .filter(|written| !written.is_empty())
        {
            let base = crate::db::written_in(origins, &format!("imap.{name}.directory"));
            let path = crate::resolve::absolute(base, Path::new(written))
                .to_string_lossy()
                .into_owned();
            crate::db::rewrite_in(table, "imap", name, "directory", &path);
            block.directory = Some(path);
        }
        let Some(written) = block.tls_ca_file.as_deref() else {
            continue;
        };
        let base = crate::db::written_in(origins, &format!("imap.{name}.tls_ca_file"));
        let path = crate::resolve::absolute(base, Path::new(written));
        let trusted = files.trust(&path).map_err(|why| {
            crate::resolve::untrusted(
                &path,
                &why,
                "an `[imap]` block's `tls_ca_file` names it, and whoever can write it chooses which \
                 server the mailbox password is sent to",
            )
        })?;
        let trusted = trusted.to_string_lossy().into_owned();
        crate::db::rewrite_in(table, "imap", name, "tls_ca_file", &trusted);
        block.tls_ca_file = Some(trusted);
    }
    Ok(())
}

/// Every `[imap.<name>]` block, asked of the merged tree.
///
/// # Errors
///
/// `E0601` for the first block whose `tls` or `auth` is not a word it takes, whose `directory`
/// is beside a key only a connection reads, whose `oauth_*` keys are not an `xoauth2` block's, or
/// whose credential is half written.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    for (name, block) in &config.imap {
        judge(name, block, origins)?;
    }
    Ok(())
}

fn judge(
    name: &str,
    block: &ImapMailbox,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let key = |key: &str| format!("imap.{name}.{key}");
    if block.directory.is_some() {
        return directory(name, block, origins);
    }
    if tls_from(block.tls.as_deref()).is_none() {
        return Err(refusal(
            &key("tls"),
            format!(
                "`[imap.{name}] tls = \"{}\"` is not a TLS mode",
                block.tls.as_deref().unwrap_or_default()
            ),
            "`tls` is `\"implicit\"` or `\"starttls\"`. A mailbox password is never sent \
             without TLS, so there is no `\"none\"`.",
            "write one of the two, or leave the key out for `implicit`",
            origins,
        ));
    }
    let password = block.password.is_some() || block.password_file.is_some();
    match auth_from(block.auth.as_deref()) {
        Err(word) => Err(refusal(
            &key("auth"),
            format!("`[imap.{name}] auth = \"{word}\"` is not a mechanism"),
            "`auth` is `\"plain\"`, `\"login\"` or `\"xoauth2\"`.",
            "write one of the three, or leave the key out for the default",
            origins,
        )),
        Ok(Some(Auth::XOAuth2)) => oauth(name, block, origins),
        Ok(_) if oauth_written(block).is_some() => {
            let written = oauth_written(block).unwrap_or_default();
            Err(refusal(
                &key(written),
                format!("`[imap.{name}]` sets `{written}` and its `auth` is not `\"xoauth2\"`"),
                "Only an `xoauth2` block fetches a token, so the key would never be read.",
                "write `auth = \"xoauth2\"`, or remove the `oauth_*` keys",
                origins,
            ))
        }
        Ok(_) if block.user.is_some() != password => {
            let (written, missing) = if password {
                ("password", "user")
            } else {
                ("user", "password")
            };
            Err(refusal(
                &key(written),
                format!("`[imap.{name}]` sets `{written}` and no `{missing}`"),
                "The server signs in with the user and the password together.",
                &format!("write `{missing}`, or remove `{written}`"),
                origins,
            ))
        }
        Ok(_) => Ok(()),
    }
}

/// A `directory` block: the folders are subdirectories of that path, and every key only a
/// server session reads would be read and never used.
fn directory(
    name: &str,
    block: &ImapMailbox,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let connection = [
        ("host", block.host.is_some()),
        ("port", block.port.is_some()),
        ("tls", block.tls.is_some()),
        ("tls_ca_file", block.tls_ca_file.is_some()),
        ("user", block.user.is_some()),
        ("password", block.password.is_some()),
        ("password_file", block.password_file.is_some()),
        ("auth", block.auth.is_some()),
        ("timeout", block.timeout.is_some()),
    ];
    let written = connection
        .iter()
        .find(|(_, set)| *set)
        .map(|(key, _)| *key)
        .or_else(|| oauth_written(block));
    match written {
        Some(key) => Err(refusal(
            &format!("imap.{name}.{key}"),
            format!("`[imap.{name}]` has a `directory` and sets `{key}`"),
            "A directory block reads its folders from files and opens no connection, so the key \
             would never be read.",
            &format!("remove `{key}`, or remove `directory` to read a server"),
            origins,
        )),
        None => Ok(()),
    }
}

/// An `auth = "xoauth2"` block: a `user` and no password, an `https://` token URL, a client id
/// and a client secret, as [`crate::mail`] asks of a mail block.
fn oauth(
    name: &str,
    block: &ImapMailbox,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let key = |key: &str| format!("imap.{name}.{key}");
    if block.user.is_none() {
        return Err(refusal(
            &key("auth"),
            format!("`[imap.{name}]` has `auth = \"xoauth2\"` and no `user`"),
            "The token signs in one mailbox, and `user` names it.",
            "write `user`, the address the token signs in as",
            origins,
        ));
    }
    if block.password.is_some() || block.password_file.is_some() {
        return Err(refusal(
            &key("password"),
            format!("`[imap.{name}]` has `auth = \"xoauth2\"` and a password"),
            "An `xoauth2` block signs in with a token, so the password would never be read.",
            "remove `password` or `password_file`",
            origins,
        ));
    }
    match block.oauth_token_url.as_deref() {
        None => {
            return Err(refusal(
                &key("auth"),
                format!("`[imap.{name}]` has `auth = \"xoauth2\"` and no `oauth_token_url`"),
                "The token is fetched from that URL.",
                "write `oauth_token_url`, the `https://` URL of the token server",
                origins,
            ));
        }
        Some(url) if token_url(url).is_none() => {
            return Err(refusal(
                &key("oauth_token_url"),
                format!("`[imap.{name}] oauth_token_url = \"{url}\"` is not an `https://` URL"),
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
                format!("`[imap.{name}]` has `auth = \"xoauth2\"` and no `{wanted}`"),
                "The token server needs the client's id and secret.",
                &format!("write `{wanted}`"),
                origins,
            ));
        }
    }
    Ok(())
}

/// The first `oauth_*` key `block` sets, by its name.
fn oauth_written(block: &ImapMailbox) -> Option<&'static str> {
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

    /// `tls` is `implicit` unset, `starttls` when written, and no other word boots, `none`
    /// included. Half a credential does not boot, and a user with a password does.
    #[test]
    fn an_imap_block_s_tls_and_credential_refusals() {
        assert_eq!(tls_from(None), Some(Tls::Implicit));
        assert_eq!(tls_from(Some("starttls")), Some(Tls::StartTls));
        assert_eq!(tls_from(Some("none")), None);

        let origins = BTreeMap::new();
        let block = |tls: Option<&str>, user: bool, password: bool| ImapMailbox {
            host: Some("imap.example.com".to_owned()),
            tls: tls.map(str::to_owned),
            user: user.then(|| "app".to_owned()),
            password: password.then(|| "s3cret".to_owned()),
            ..ImapMailbox::default()
        };
        assert!(judge("inbox", &block(None, true, true), &origins).is_ok());
        assert!(judge("inbox", &block(Some("starttls"), true, true), &origins).is_ok());
        let refused = judge("inbox", &block(Some("none"), true, true), &origins).unwrap_err();
        assert!(
            format!("{refused:?}").contains("not a TLS mode"),
            "{refused:?}"
        );
        assert!(judge("inbox", &block(None, true, false), &origins).is_err());
        assert!(judge("inbox", &block(None, false, true), &origins).is_err());

        let mut cram = block(None, true, true);
        cram.auth = Some("cram-md5".to_owned());
        assert!(judge("inbox", &cram, &origins).is_err());
        let mut stray = block(None, true, true);
        stray.oauth_scope = Some("mail".to_owned());
        let refused = judge("inbox", &stray, &origins).unwrap_err();
        assert!(
            format!("{refused:?}").contains("oauth_scope"),
            "{refused:?}"
        );
    }

    /// An `xoauth2` block needs a user, an `https://` token URL, a client id and a secret, and no
    /// password.
    #[test]
    fn an_xoauth2_imap_block_needs_its_token_keys() {
        let origins = BTreeMap::new();
        let block = ImapMailbox {
            host: Some("imap.example.com".to_owned()),
            user: Some("inbox@example.com".to_owned()),
            auth: Some("xoauth2".to_owned()),
            oauth_token_url: Some("https://login.example.com/token".to_owned()),
            oauth_client_id: Some("app".to_owned()),
            oauth_client_secret: Some("s3cret".to_owned()),
            ..ImapMailbox::default()
        };
        assert!(judge("inbox", &block, &origins).is_ok());
        let mut http = block.clone();
        http.oauth_token_url = Some("http://login.example.com/token".to_owned());
        assert!(judge("inbox", &http, &origins).is_err());
        let mut password = block.clone();
        password.password = Some("s3cret".to_owned());
        assert!(judge("inbox", &password, &origins).is_err());
        let mut no_secret = block;
        no_secret.oauth_client_secret = None;
        assert!(judge("inbox", &no_secret, &origins).is_err());
    }

    /// A directory block boots alone, and a key only a connection reads beside it does not.
    #[test]
    fn a_directory_block_refuses_every_connection_key() {
        let origins = BTreeMap::new();
        let block = ImapMailbox {
            directory: Some("mail/inbox".to_owned()),
            ..ImapMailbox::default()
        };
        assert!(judge("local", &block, &origins).is_ok());
        let with_host = ImapMailbox {
            host: Some("imap.example.com".to_owned()),
            ..block.clone()
        };
        let refused = judge("local", &with_host, &origins).unwrap_err();
        assert!(format!("{refused:?}").contains("`host`"), "{refused:?}");
        let with_oauth = ImapMailbox {
            oauth_client_id: Some("app".to_owned()),
            ..block
        };
        assert!(judge("local", &with_oauth, &origins).is_err());
    }
}
