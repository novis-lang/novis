//! What a failed TLS handshake tells the person who can fix it, worded once for every outbound
//! client: `Core\Http\Client`, `Core\Mail`, `Core\Ldap`, `Core\Db` and the shared cache.
//!
//! Two failures have a fix a setting makes, and [`advice`] names that setting:
//!
//! - **The certificate is from a CA nobody here trusts** ([`nvs_host::tls::unknown_issuer`]). An
//!   operator's block names its own `tls_ca_file` and `[tls.client] roots`, and an address the
//!   program wrote names `[tls.client] roots` alone, because the program has no block.
//! - **The certificate is for another name** ([`nvs_host::tls::names_presented`]). The message
//!   lists the names it is for, and the fix is the setting that holds the address. It never names
//!   a CA setting, because no CA fixes a name.
//!
//! Every other handshake failure keeps the error `rustls` gave, and [`advice`] returns `None`.
//! The advice is built here and not in `nvs-ldap` or `nvs-db`, because only this layer knows the
//! block's name. It costs one check of the error, on a handshake that already failed.

use std::io;

use nvs_host::tls::Refusal;

/// Where the address and the trusted CAs of a failed connection were set.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Settings<'a> {
    /// An operator's block. `block` is its dotted name, such as `ldap.corp`, and `address` is the
    /// key in it that holds the address, such as `url` or `host`.
    Block { block: &'a str, address: &'a str },
    /// An operator's block with no `tls_ca_file` key, such as `[cache.shared]`. Only
    /// `[tls.client] roots` adds a CA for it.
    BlockWithoutCaFile { block: &'a str, address: &'a str },
    /// An address the program wrote, such as `Core\Ldap::open`'s `url`.
    Program,
}

/// The sentences that say why the handshake with `host` failed and which setting fixes it, or
/// `None` for a failure no setting fixes.
pub(crate) fn advice(why: &io::Error, host: &str, settings: Settings<'_>) -> Option<String> {
    Some(fix_for(&nvs_host::tls::refusal(why)?, host, settings))
}

/// [`advice`] for a refusal a driver kept beside its own error.
pub(crate) fn fix_for(refusal: &Refusal, host: &str, settings: Settings<'_>) -> String {
    let names = match refusal {
        Refusal::OtherName(names) => names,
        Refusal::UnknownIssuer => {
            return unknown_issuer(settings);
        }
    };
    let fix = match settings {
        Settings::Block { block, address } | Settings::BlockWithoutCaFile { block, address } => {
            format!("Set `[{block}] {address}` to")
        }
        Settings::Program => "Connect to".to_owned(),
    };
    if names.is_empty() {
        format!("Its certificate is not for `{host}`. {fix} a name the certificate is for.")
    } else {
        let listed: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
        format!(
            "Its certificate is for {}, and not for `{host}`. {fix} one of those names.",
            listed.join(", ")
        )
    }
}

/// The sentences for a certificate from a CA nobody here trusts.
fn unknown_issuer(settings: Settings<'_>) -> String {
    let fix = match settings {
        Settings::Block { block, .. } => format!(
            "set `[{block}] tls_ca_file` to a PEM file of that CA, or add it to `[tls.client] \
             roots`"
        ),
        Settings::BlockWithoutCaFile { .. } | Settings::Program => {
            "add it to `[tls.client] roots`".to_owned()
        }
    };
    format!(
        "Its certificate is signed by a CA this host does not trust. If the server uses a \
         private or company CA, {fix}."
    )
}

/// `"<member>: the TLS handshake with `<host>` did not complete."`, then [`advice`], or the
/// error itself where no setting fixes it.
pub(crate) fn handshake_failed(
    member: &str,
    host: &str,
    why: &io::Error,
    settings: Settings<'_>,
) -> String {
    match advice(why, host, settings) {
        Some(fix) => format!("{member}: the TLS handshake with `{host}` did not complete. {fix}"),
        None => format!("{member}: the TLS handshake with `{host}` did not complete — {why}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An error `rustls` gave, as a handshake returns it.
    fn failed(error: rustls::CertificateError) -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidData,
            rustls::Error::InvalidCertificate(error),
        )
    }

    /// An unknown issuer names the block's `tls_ca_file` and `[tls.client] roots`, and only the
    /// roots for an address the program wrote.
    #[test]
    fn an_unknown_issuer_names_the_ca_settings() {
        let why = failed(rustls::CertificateError::UnknownIssuer);
        let block = Settings::Block {
            block: "ldap.corp",
            address: "url",
        };
        let said = advice(&why, "dc.example.com", block).expect("a fix is named");
        assert!(
            said.contains("`[ldap.corp] tls_ca_file`") && said.contains("`[tls.client] roots`"),
            "{said}"
        );
        let said = advice(&why, "dc.example.com", Settings::Program).expect("a fix is named");
        assert!(
            said.contains("`[tls.client] roots`") && !said.contains("tls_ca_file"),
            "{said}"
        );
    }

    /// A certificate for another name, reported without its names, names the address setting and
    /// no CA setting.
    #[test]
    fn a_certificate_for_another_name_names_the_address_setting() {
        let why = failed(rustls::CertificateError::NotValidForName);
        let block = Settings::Block {
            block: "db.main",
            address: "host",
        };
        let said = advice(&why, "10.0.0.5", block).expect("a fix is named");
        assert!(
            said.contains("not for `10.0.0.5`") && said.contains("`[db.main] host`"),
            "{said}"
        );
        assert!(!said.contains("roots") && !said.contains("tls_ca_file"), "{said}");
    }

    /// Any other failure has no advice, and the message keeps the error.
    #[test]
    fn another_failure_keeps_its_error() {
        let why = failed(rustls::CertificateError::Expired);
        assert_eq!(advice(&why, "a.example", Settings::Program), None);
        let said = handshake_failed("Core\\Ldap::open", "a.example", &why, Settings::Program);
        assert!(said.contains("Expired"), "{said}");
    }
}
