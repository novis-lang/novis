//! What `nvs debug` serves the viewer with off loopback: a TLS configuration and a token
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! `[debug] tls_cert_file`, `tls_key_file` and `token_file` name files somebody brought. Without
//! them, or when a named file does not exist, the service uses its own, generated once into the
//! data folder's private `debug/` directory ([`nvs_config::data::Folder::viewer_secrets`]) and kept,
//! so a restart serves the same link and a browser keeps the exception it made for the
//! certificate. The certificate is self-signed with `rcgen` for `localhost` and the bound address,
//! and the token is 32 bytes from the thread's CSPRNG, written as hex.
//!
//! **What it spends:** one certificate generation the first time the service runs off loopback, and
//! the configuration and token for the service's life.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rand::RngExt;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use super::listen::Listen;

/// The TLS configuration and the token one service serves with.
pub(crate) struct Secure {
    tls: Arc<rustls::ServerConfig>,
    token: String,
}

impl Secure {
    /// The overrides `listen` names where their files exist, and the generated files otherwise.
    /// The error is the message to print.
    pub(crate) fn load(
        listen: &Listen,
        folder: &nvs_config::data::Folder,
        ip: IpAddr,
    ) -> Result<Self, String> {
        let [own_cert, own_key, own_token] = folder.viewer_secrets();
        let (cert, key) = match (given(listen.cert.as_deref()), given(listen.key.as_deref())) {
            (Some(cert), Some(key)) => (cert, key),
            _ => {
                if !own_cert.exists() || !own_key.exists() {
                    generate(&own_cert, &own_key, ip)?;
                }
                (own_cert, own_key)
            }
        };
        let token = match given(listen.token.as_deref()) {
            Some(token) => token,
            None => {
                if !own_token.exists() {
                    let bytes: [u8; 32] = rand::rng().random();
                    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                    write_private(&own_token, hex.as_bytes())?;
                }
                own_token
            }
        };
        let token = std::fs::read_to_string(&token)
            .map_err(|err| format!("cannot read `{}`: {err}", token.display()))?
            .trim()
            .to_owned();
        // These characters go into a link and a cookie as they are.
        if token.len() < 16
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~'))
        {
            return Err(
                "the debug viewer's token must be at least 16 characters, each a letter, \
                 a digit, `-`, `_`, `.` or `~`"
                    .to_owned(),
            );
        }
        Ok(Self {
            tls: Arc::new(config(&cert, &key)?),
            token,
        })
    }

    pub(crate) fn tls(&self) -> Arc<rustls::ServerConfig> {
        Arc::clone(&self.tls)
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

/// Writes `bytes` to `path`, readable by this account alone on Unix. On Windows the file inherits
/// `debug/`'s private access list.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|err| format!("cannot write `{}`: {err}", path.display()))
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
