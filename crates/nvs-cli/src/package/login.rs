//! `nvs pkg login` and `nvs pkg whoami`: the publish token a registry gave its owner, stored and
//! checked (`rule:packaging/a-publish-token-is-scoped-and-expires`,
//! `rule:config/ownership-is-the-trust-boundary`).
//!
//! **The token comes from standard input, never from an argument**, because an argument shows in
//! the process list and the shell's history. Text that is not `nvsreg_` and 64 lowercase hex digits
//! is refused before anything connects ([`well_formed`]).
//!
//! **The registry checks the token before it is stored.** Both commands send it to the registry
//! service's `GET /api/whoami` as a bearer token ([`ask`]), which answers the account, the scope
//! and the expiry, or `401` for a token that is unknown, expired or revoked. The registry is
//! `--registry`, the same URL a `[registries]` line names, and [`DEFAULT_REGISTRY`] without it.
//! The service's routes sit under that URL beside `registry.json`. An `https:` URL connects
//! anywhere, as [`super::index`] reads a registry; `http:` is allowed only for a loopback host,
//! and then every address it resolves to must be loopback, which is how a developer reaches a
//! registry running on their own machine. The transport drops the `authorization` header on a
//! redirect to another origin and refuses a redirect from `https:` to `http:`.
//!
//! **The token is stored under the data folder's `tokens/`**, one file per registry, named by a
//! hash of the registry's URL ([`file_of`]). The folder is created privately with
//! [`nvs_config::data::create_private_root`], an existing data folder is checked before anything is
//! created in it, and the file is written to a fresh temporary name, `0600` on Unix, and renamed
//! into place. A stored token is read only once [`nvs_config::trust::check`] passes on the file and
//! its folder, the check the artifact cache takes, so a token file another account can write is
//! refused and never sent.
//!
//! Cost: one small file per registry, and one request of at most [`REPLY_LIMIT`] bytes per command.
//! Nothing here runs on the request path.

use std::fs;
use std::io::{self, BufRead as _, IsTerminal as _, Read as _, Write as _};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use rand::RngExt as _;

use super::fetch::{self, Failed};
use super::index::{Fault, fault};
use super::manifest::DEFAULT_REGISTRY;

/// The name a refused connection names.
const MEMBER: &str = "nvs pkg";

/// What every publish token starts with.
const PREFIX: &str = "nvsreg_";

/// The number of hex digits after [`PREFIX`].
const DIGITS: usize = 64;

/// The most bytes read from standard input: a token and a line ending, with room to spare.
const INPUT_LIMIT: u64 = 1024;

/// The most bytes the registry's reply may have.
const REPLY_LIMIT: u64 = 64 * 1024;

/// How long the request to the registry may take.
const WITHIN: Duration = Duration::from_secs(30);

/// What the registry said about a token.
struct Holder {
    /// The account that owns the token, as 32 hex digits.
    account: String,
    /// The vendor or the package the token may publish.
    scope: String,
    /// When the token expires, as a Unix time.
    expires_at: i64,
}

/// `nvs pkg login`: the token on standard input, checked with `registry` and stored.
pub(crate) fn login(registry: Option<&str>) -> ExitCode {
    fetch::exit(logging_in(registry).map_err(Failed::from))
}

/// `nvs pkg whoami`: the stored token for `registry`, checked with it, and what it says.
pub(crate) fn whoami(registry: Option<&str>) -> ExitCode {
    fetch::exit(asking(registry).map_err(Failed::from))
}

/// [`login`], returning what stopped it.
fn logging_in(registry: Option<&str>) -> Result<(), Fault> {
    let origin = origin(registry)?;
    let token = read_input()?;
    let holder = match ask(&origin, &token)? {
        Some(holder) => holder,
        None => {
            return Err(fault(format!(
                "The registry `{origin}` does not accept this token. The token may be expired or \
                 revoked. Nothing was stored."
            )));
        }
    };
    let file = store(&origin, &token)?;
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "Logged in to `{origin}`.");
    print_holder(&mut out, &holder);
    let _ = writeln!(out, "The token is stored in `{}`.", file.display());
    Ok(())
}

/// [`whoami`], returning what stopped it.
fn asking(registry: Option<&str>) -> Result<(), Fault> {
    let origin = origin(registry)?;
    let token = stored(&origin)?;
    let Some(holder) = ask(&origin, &token)? else {
        return Err(fault(format!(
            "The registry `{origin}` does not accept your stored token. The token is expired or \
             revoked. Run `nvs pkg login` again with a new token."
        )));
    };
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "Registry: {origin}");
    print_holder(&mut out, &holder);
    Ok(())
}

/// The account, the scope and the expiry date, one line each.
fn print_holder(out: &mut impl io::Write, holder: &Holder) {
    let expires = jiff::Timestamp::from_second(holder.expires_at).map_or_else(
        |_| holder.expires_at.to_string(),
        |at| at.strftime("%Y-%m-%d").to_string(),
    );
    let _ = writeln!(out, "Account: {}", holder.account);
    let _ = writeln!(out, "Scope: {}", holder.scope);
    let _ = writeln!(out, "Expires: {expires} (UTC)");
}

/// The registry's URL without a trailing `/`: `given`, or [`DEFAULT_REGISTRY`].
///
/// # Errors
///
/// A URL that is not `https:` with a host, or `http:` with a loopback host.
pub(crate) fn origin(given: Option<&str>) -> Result<String, Fault> {
    let url = given.unwrap_or(DEFAULT_REGISTRY).trim_end_matches('/');
    let wrong = || {
        fault(format!(
            "`{url}` is not a registry URL. It must start with `https://`. Use `http://` only \
             for a registry on this computer, such as `http://localhost:8000`."
        ))
    };
    if let Some(rest) = url.strip_prefix("https://") {
        let host = host_of(rest).ok_or_else(wrong)?;
        if host.is_empty() {
            return Err(wrong());
        }
        return Ok(url.to_owned());
    }
    let rest = url.strip_prefix("http://").ok_or_else(wrong)?;
    let host = host_of(rest).ok_or_else(wrong)?;
    if !is_loopback_host(host) {
        return Err(wrong());
    }
    Ok(url.to_owned())
}

/// The host part of `rest`, a URL after its `scheme://`, without brackets or a port. `None` for an
/// authority with user information, which a registry URL never has.
fn host_of(rest: &str) -> Option<&str> {
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return None;
    }
    if let Some(bracketed) = authority.strip_prefix('[') {
        return bracketed.split_once(']').map(|(host, _)| host);
    }
    Some(authority.split(':').next().unwrap_or_default())
}

/// Whether `host` names this computer: `localhost` or a loopback address.
fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// The token on standard input, checked to be one.
///
/// # Errors
///
/// Input that cannot be read, and text that is not a token.
fn read_input() -> Result<String, Fault> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        eprintln!("Paste your publish token, then press Enter:");
    }
    let mut text = String::new();
    if stdin.lock().take(INPUT_LIMIT).read_line(&mut text).is_err() {
        return Err(fault("The token cannot be read from standard input."));
    }
    let token = text.trim();
    if !well_formed(token) {
        return Err(fault(format!(
            "This is not a publish token. A token starts with `{PREFIX}`, followed by {DIGITS} \
             characters from 0 to 9 and a to f."
        )));
    }
    Ok(token.to_owned())
}

/// Whether `text` is [`PREFIX`] followed by [`DIGITS`] lowercase hex digits.
fn well_formed(text: &str) -> bool {
    text.strip_prefix(PREFIX).is_some_and(|hex| {
        hex.len() == DIGITS && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

/// What the registry at `origin` says about `token`, or `None` when it answers `401`.
///
/// # Errors
///
/// A failed request, another status, and a reply that is not the expected JSON.
fn ask(origin: &str, token: &str) -> Result<Option<Holder>, Fault> {
    let url = format!("{origin}/api/whoami");
    let (status, body) = send(origin, token, "/api/whoami", None)?;
    match status {
        200 => {}
        401 => return Ok(None),
        status => {
            return Err(fault(format!(
                "The registry `{origin}` returned status {status} for `/api/whoami`."
            )));
        }
    }
    let bad = || fault(format!("The reply of `{url}` is not what a registry returns."));
    let json: serde_json::Value = serde_json::from_slice(&body).map_err(|_| bad())?;
    let text = |key: &str| json.get(key).and_then(serde_json::Value::as_str).map(str::to_owned);
    Ok(Some(Holder {
        account: text("account").ok_or_else(bad)?,
        scope: text("scope").ok_or_else(bad)?,
        expires_at: json
            .get("expiresAt")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(bad)?,
    }))
}

/// One request to the registry at `origin`'s `path` with `token` as its bearer token: a `GET`, or
/// a `POST` of the JSON `body`. The reply's status and its body, of at most [`REPLY_LIMIT`]
/// bytes.
///
/// # Errors
///
/// A registry that cannot be reached, and a reply that cannot be read.
pub(crate) fn send(
    origin: &str,
    token: &str,
    path: &str,
    body: Option<Vec<u8>>,
) -> Result<(i64, Vec<u8>), Fault> {
    let url = format!("{origin}{path}");
    let headers = vec![
        (
            "User-Agent".to_owned(),
            format!("nvs/{}", env!("CARGO_PKG_VERSION")),
        ),
        ("Authorization".to_owned(), format!("Bearer {token}")),
        ("Accept".to_owned(), "application/json".to_owned()),
    ];
    let bounds = nvs_stdlib::ToolBounds {
        within: WITHIN,
        bytes: REPLY_LIMIT,
    };
    let plain = url.starts_with("http://");
    let refused = |address: IpAddr| {
        (plain && !address.is_loopback()).then(|| {
            format!("resolves to `{address}`, which is not this computer. Use `https://` for it.")
        })
    };
    let body = body.map(|bytes| ("application/json", bytes));
    let mut reply = nvs_stdlib::tool_stream(MEMBER, &url, headers, body, bounds, &refused)
        .map_err(|why| fault(format!("The registry `{origin}` cannot be reached: {why}")))?;
    let mut read = Vec::new();
    reply
        .read_to_end(&mut read)
        .map_err(|why| fault(format!("The reply of `{url}` cannot be read: {why}")))?;
    Ok((reply.status, read))
}

/// The data folder's `tokens/`.
///
/// # Errors
///
/// No data folder.
fn tokens_dir() -> Result<PathBuf, Fault> {
    nvs_config::data::current()
        .map(nvs_config::data::Folder::tokens)
        .ok_or_else(|| fault("There is no data folder to store the token in. Use `--data`."))
}

/// The file the token for `origin` is stored in: a hash of the URL, because a URL is not a file
/// name.
fn file_of(dir: &Path, origin: &str) -> PathBuf {
    let hex = blake3::hash(origin.as_bytes()).to_hex();
    dir.join(&hex[..32])
}

/// The fault for a folder or a file the ownership check refused.
fn untrusted(path: &Path, why: &nvs_config::trust::Untrusted) -> Fault {
    let help = match why {
        nvs_config::trust::Untrusted::Breach { help, .. } => format!(" {help}"),
        nvs_config::trust::Untrusted::Unreadable(_) => String::new(),
    };
    fault(format!(
        "`{}` cannot be used for a token: {}.{help}",
        path.display(),
        why.message()
    ))
}

/// `token` written to the file for `origin`, privately, and the file's path.
///
/// # Errors
///
/// No data folder, a folder the ownership check refuses, and a failed write.
fn store(origin: &str, token: &str) -> Result<PathBuf, Fault> {
    let dir = tokens_dir()?;
    if let Some(root) = dir.parent().filter(|root| root.exists()) {
        nvs_config::data::check(root).map_err(|why| untrusted(root, &why))?;
    }
    let failed = |err: io::Error| {
        fault(format!(
            "The token cannot be stored in `{}`: {err}",
            dir.display()
        ))
    };
    nvs_config::data::create_private_root(&dir).map_err(failed)?;
    nvs_config::trust::check(&dir).map_err(|why| untrusted(&dir, &why))?;
    let file = file_of(&dir, origin);
    let temp = dir.join(format!(".tmp-{:032x}", rand::rng().random::<u128>()));
    let written = write_private(&temp, token).and_then(|()| fs::rename(&temp, &file));
    if written.is_err() {
        // The write's error is the one to report; a temporary that will not go is a stray file.
        let _ = fs::remove_file(&temp);
    }
    written.map_err(failed)?;
    Ok(file)
}

/// `token` and a line ending in the new file `path`, readable by this account alone on Unix. On
/// Windows the file inherits the private access list of `tokens/`.
fn write_private(path: &Path, token: &str) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    file.write_all(token.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()
}

/// The token stored for `origin`, once the ownership check passes on its file.
///
/// # Errors
///
/// No stored token, a file the check refuses, and a file that does not contain a token.
pub(crate) fn stored(origin: &str) -> Result<String, Fault> {
    let file = file_of(&tokens_dir()?, origin);
    if !file.exists() {
        return Err(fault(format!(
            "You are not logged in to `{origin}`. Run `nvs pkg login` and paste your token."
        )));
    }
    nvs_config::trust::check(&file).map_err(|why| untrusted(&file, &why))?;
    let mut text = String::new();
    fs::File::open(&file)
        .and_then(|opened| opened.take(INPUT_LIMIT).read_to_string(&mut text))
        .map_err(|err| fault(format!("`{}` cannot be read: {err}", file.display())))?;
    let token = text.trim();
    if !well_formed(token) {
        return Err(fault(format!(
            "`{}` does not contain a publish token. Run `nvs pkg login` again.",
            file.display()
        )));
    }
    Ok(token.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{host_of, origin, well_formed};

    #[test]
    fn only_a_prefixed_64_digit_hex_token_is_well_formed() {
        let hex = "0123456789abcdef".repeat(4);
        assert!(well_formed(&format!("nvsreg_{hex}")));
        assert!(!well_formed(&hex));
        assert!(!well_formed(&format!("nvsreg_{}", hex.to_uppercase())));
        assert!(!well_formed(&format!("nvsreg_{hex}0")));
        assert!(!well_formed(&format!("nvsreg_{}", &hex[1..])));
    }

    #[test]
    fn http_is_a_registry_url_on_loopback_only() {
        assert_eq!(
            origin(None).as_deref(),
            Ok("https://registry.novis-lang.org")
        );
        assert!(origin(Some("https://r.example.com/")).is_ok());
        assert!(origin(Some("http://localhost:8000")).is_ok());
        assert!(origin(Some("http://127.0.0.1:8000/")).is_ok());
        assert!(origin(Some("http://[::1]:8000")).is_ok());
        assert!(origin(Some("http://r.example.com")).is_err());
        assert!(origin(Some("http://localhost@r.example.com")).is_err());
        assert!(origin(Some("https://")).is_err());
        assert!(origin(Some("ftp://r.example.com")).is_err());
        assert_eq!(host_of("[::1]:80/x"), Some("::1"));
    }
}
