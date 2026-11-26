//! What `nvs debug` serves the viewer with: a token on every bind, and a TLS configuration off
//! loopback (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! The token is needed on loopback too, because every local account and every page that reaches a
//! loopback port can otherwise read the spool's request data through the viewer's API.
//!
//! `[debug] tls_cert_file`, `tls_key_file` and `token_file` name files somebody brought. Without
//! them, or when a named file does not exist, the service uses its own, generated once into the
//! data folder's private `debug/` directory ([`nvs_config::data::Folder::viewer_secrets`]) and kept,
//! so a restart serves the same link and a browser keeps the exception it made for the
//! certificate. The certificate is self-signed with `rcgen` for `localhost` and the bound address,
//! and the token is [`nvs_config::viewer::token`]'s.
//!
//! **What it spends:** one token file the first time the service runs, one certificate generation
//! the first time it runs off loopback, and the configuration and token for the service's life.
//! Each viewer request costs one comparison of the token, in constant time.

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nvs_config::viewer::write_private;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use super::listen::Listen;

/// The token one service serves with, the cookie that carries it, and off loopback the TLS
/// configuration.
pub(crate) struct Secure {
    tls: Option<Arc<rustls::ServerConfig>>,
    token: String,
    cookie: String,
}

impl Secure {
    /// The overrides `listen` names where their files exist, and the generated files otherwise.
    /// The TLS files are read only when `addr` is off loopback. The error is the message to print.
    pub(crate) fn load(
        listen: &Listen,
        folder: &nvs_config::data::Folder,
        addr: SocketAddr,
    ) -> Result<Self, String> {
        let [own_cert, own_key, own_token] = folder.viewer_secrets();
        let tls = if addr.ip().is_loopback() {
            None
        } else {
            let (cert, key) = match (given(listen.cert.as_deref()), given(listen.key.as_deref())) {
                (Some(cert), Some(key)) => (cert, key),
                _ => {
                    if !own_cert.exists() || !own_key.exists() {
                        generate(&own_cert, &own_key, addr.ip())?;
                    }
                    (own_cert, own_key)
                }
            };
            Some(Arc::new(config(&cert, &key)?))
        };
        let token = token(listen, &own_token)?;
        // `__Host-` makes the browser keep the cookie only when it is `Secure`, so it needs TLS.
        // A browser sends a loopback cookie to every port of the address, so the port is in its
        // name, and two services on one address do not replace each other's cookie.
        let cookie = match tls {
            Some(_) => "__Host-nvs-debug".to_owned(),
            None => format!("nvs-debug-{}", addr.port()),
        };
        Ok(Self { tls, token, cookie })
    }

    /// The TLS configuration, off loopback.
    pub(crate) fn tls(&self) -> Option<Arc<rustls::ServerConfig>> {
        self.tls.clone()
    }

    /// The name of the cookie that carries the token.
    pub(crate) fn cookie(&self) -> &str {
        &self.cookie
    }

    /// The `Set-Cookie` value that stores `token` in the browser. `Secure` goes with TLS alone.
    pub(crate) fn set_cookie(&self, token: &str) -> String {
        let secure = if self.tls.is_some() { " Secure;" } else { "" };
        format!(
            "{}={token}; Path=/;{secure} HttpOnly; SameSite=Strict",
            self.cookie
        )
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }

    /// Whether `presented` is the token, compared in time that does not depend on where they differ.
    pub(crate) fn admits(&self, presented: &str) -> bool {
        let (a, b) = (self.token.as_bytes(), presented.as_bytes());
        a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
    }
}

/// The token `[debug] token_file` names, or the generated file `own`, created the first time
/// ([`nvs_config::viewer::token`]). A named file that does not exist is a warning here.
/// `nvs debug link` and the language server read it the same way, so every link is the one the
/// service admits.
pub(crate) fn token(listen: &Listen, own: &Path) -> Result<String, String> {
    nvs_config::viewer::token(given(listen.token.as_deref()).as_deref(), own)
}

/// `path` when it is set and its file exists. A set path whose file does not exist is a warning,
/// and the generated file is used.
fn given(path: Option<&Path>) -> Option<PathBuf> {
    let path = path?;
    if path.exists() {
        return Some(path.to_owned());
    }
    eprintln!(
        "warning: `{}` does not exist, so `nvs debug` uses the file it created",
        path.display()
    );
    None
}

/// A self-signed certificate for `localhost` and `ip`, and its key, written to `cert` and `key`.
fn generate(cert: &Path, key: &Path, ip: IpAddr) -> Result<(), String> {
    let mut names = vec!["localhost".to_owned()];
    if !ip.is_unspecified() {
        names.push(ip.to_string());
    }
    let made = rcgen::generate_simple_self_signed(names)
        .map_err(|err| format!("cannot create the debug viewer's certificate: {err}"))?;
    write_private(key, made.signing_key.serialize_pem().as_bytes())?;
    write_private(cert, made.cert.pem().as_bytes())
}

/// The server configuration for the certificate chain in `cert` and the key in `key`.
fn config(cert: &Path, key: &Path) -> Result<rustls::ServerConfig, String> {
    let chain = CertificateDer::pem_file_iter(cert)
        .and_then(|certs| certs.collect::<Result<Vec<_>, _>>())
        .map_err(|err| format!("cannot read the certificate `{}`: {err}", cert.display()))?;
    let key = PrivateKeyDer::from_pem_file(key)
        .map_err(|err| format!("cannot read the key `{}`: {err}", key.display()))?;
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|err| err.to_string())?
    .with_no_client_auth()
    .with_single_cert(chain, key)
    .map_err(|err| format!("the debug viewer's certificate and key do not pair: {err}"))?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}
