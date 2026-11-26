//! The debug viewer's link, its token and whether a service runs: what `nvs debug`, `nvs debug link`
//! and the language server's `nvs/debugViewer` each read
//! (`rule:testing/the-viewer-is-framed-only-by-the-editor-under-a-token-path`).
//!
//! It is here because this crate owns the data folder ([`crate::data::Folder`]) and both `nvs-cli`
//! and `nvs-lsp` link it, so the token rule has one copy and the link an editor shows is the one
//! the next service admits.
//!
//! **The token** is the file `[debug] token_file` names when it exists, or the folder's own
//! `debug/viewer.token`, created here the first time as 32 bytes from the thread's CSPRNG, written
//! as hex. A caller that warns about a named file that does not exist checks it before it calls
//! [`token`], because this crate prints nothing.
//!
//! **Whether a service runs** is whether `debug/service.lock` is held. [`probe`] takes the lock and
//! lets go of it at once, so a service that starts in that instant tries its lock again.
//!
//! **What it spends:** one read of the token file, its creation the first time, and one lock probe.

use std::fs::{OpenOptions, TryLockError};
use std::net::SocketAddr;
use std::path::Path;

use rand::RngExt;

/// The path prefix an editor frames the viewer under. The token is the segment after it.
pub const EMBED: &str = "/embed/";

/// The link `nvs debug` prints for `addr` and `token`, and the link an editor frames. Off loopback
/// the scheme is `https`, and an unspecified address is named `localhost`.
#[must_use]
pub fn links(addr: SocketAddr, token: &str) -> (String, String) {
    let origin = if addr.ip().is_loopback() {
        format!("http://{addr}")
    } else if addr.ip().is_unspecified() {
        format!("https://localhost:{}", addr.port())
    } else {
        format!("https://{addr}")
    };
    (
        format!("{origin}/?token={token}"),
        format!("{origin}{EMBED}{token}/"),
    )
}

/// The token in `named` when that file exists, or in the generated file `own`, created here the
/// first time. The error is the message to print.
///
/// # Errors
///
/// When the file cannot be read or written, or its token is shorter than 16 characters or has a
/// character a link, a cookie or a path segment cannot carry as it is.
pub fn token(named: Option<&Path>, own: &Path) -> Result<String, String> {
    let path = match named.filter(|path| path.exists()) {
        Some(path) => path,
        None => {
            if !own.exists() {
                let bytes: [u8; 32] = rand::rng().random();
                let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                write_private(own, hex.as_bytes())?;
            }
            own
        }
    };
    let token = std::fs::read_to_string(path)
        .map_err(|err| format!("cannot read `{}`: {err}", path.display()))?
        .trim()
        .to_owned();
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
    Ok(token)
}

/// Writes `bytes` to `path`, readable by this account alone on Unix. On Windows the file inherits
/// `debug/`'s private access list.
///
/// # Errors
///
/// The message to print when the file cannot be opened or written.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write as _;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|err| format!("cannot write `{}`: {err}", path.display()))
}

/// Whether a service holds the lock at `path`. The probe takes the lock and lets it go at once.
/// No lock file means no service ever ran there.
///
/// # Errors
///
/// When the lock file exists and cannot be opened or locked.
pub fn probe(path: &Path) -> std::io::Result<bool> {
    let file = match OpenOptions::new().write(true).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };
    match file.try_lock() {
        Ok(()) => Ok(false),
        Err(TryLockError::WouldBlock) => Ok(true),
        Err(TryLockError::Error(err)) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loopback_link_is_http_and_the_embed_link_carries_the_token_as_a_segment() {
        let addr: SocketAddr = "127.0.0.1:8137".parse().unwrap();
        let (url, embed) = links(addr, "abcdefghijklmnop");
        assert_eq!(url, "http://127.0.0.1:8137/?token=abcdefghijklmnop");
        assert_eq!(embed, "http://127.0.0.1:8137/embed/abcdefghijklmnop/");
        let any: SocketAddr = "0.0.0.0:9000".parse().unwrap();
        assert_eq!(links(any, "t").1, "https://localhost:9000/embed/t/");
    }
}
