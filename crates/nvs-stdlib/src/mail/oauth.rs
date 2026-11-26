//! An `auth = "xoauth2"` block's token: fetched by the host from the block's `oauth_token_url`,
//! and kept for the next send until a minute before it expires.
//!
//! **Two grants, chosen by one key.** A block with no `oauth_refresh_token` asks with the
//! client-credentials grant, the shape Microsoft 365 uses for an application that sends as a
//! mailbox. A block with one sends it with the refresh-token grant, the shape Google uses. Both
//! send the client's id and secret, and the scope when the block names one (RFC 6749 § 4.4 and
//! RFC 6749 § 6).
//!
//! **The host fetches the token itself**, over [`nvs_host::tls`] with the compiled-in anchors,
//! never through `Core\Http`. The URL came from root-owned configuration, so it is not put
//! through the address policy, for the reason [`super::resolve`] is not. A block's
//! `tls_ca_file` is the relay's, and the token server is never checked against it.
//!
//! **The cache.** One token per block per process, under [`CACHE`], keyed by the block's name and
//! replaced when any `oauth_*` value of the block changes, so a reload that changes the client
//! fetches a new token. A token is reused until [`MARGIN`] before its `expires_in`. A token the
//! relay refuses is dropped by [`forget`], so the next send fetches again. Two sends that find no
//! token at once both fetch, and the later one is kept: the lock is never held over the network.
//! Memory: one token, a few kilobytes at most, per configured `xoauth2` block, bounded by the
//! configuration and never by traffic.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use nvs_config::mail::TokenUrl;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Ctx, Fault, ThrownClass};

use super::configured_in;

/// How long before its `expires_in` a token is fetched again, so a token never expires between
/// the fetch and the relay's check.
const MARGIN: Duration = Duration::from_secs(60);

/// The most bytes a token server's reply may have. A token is a few kilobytes, and this cap is
/// what stops a server from choosing how much a send holds.
const REPLY_CEILING: usize = 64 * 1024;

/// What an `auth = "xoauth2"` block wrote: the mailbox and every `oauth_*` value.
pub(crate) struct Grant {
    /// `user`, the mailbox the token signs in.
    pub(crate) user: String,
    /// `oauth_token_url`, already read by [`nvs_config::mail::token_url`].
    url: TokenUrl,
    /// `oauth_client_id`.
    client_id: String,
    /// `oauth_client_secret`, from the tree or its `_file`.
    client_secret: String,
    /// `oauth_scope`, sent only when written.
    scope: Option<String>,
    /// `oauth_refresh_token`, which chooses the refresh-token grant.
    refresh_token: Option<String>,
}

/// One kept token.
struct Kept {
    /// [`fingerprint`] of the grant that fetched it.
    print: [u8; 32],
    /// The bearer token.
    token: String,
    /// When it is fetched again.
    renew_at: Instant,
}

/// Every kept token, by block: `mail.<name>` or `imap.<name>`.
type Cache = Mutex<BTreeMap<String, Kept>>;

/// The process's tokens. [`super`]'s module doc and this module's say why it is one per block.
static CACHE: Cache = Mutex::new(BTreeMap::new());

/// The grant of the block `[<family>.<endpoint>]`, where `family` is `mail` or `imap`. Both
/// families write the same `user` and `oauth_*` keys.
///
/// # Errors
///
/// A catchable `RuntimeError` for a block the boot would have refused: no `user`, a password, a
/// token URL that is not `https://`, or no client id or secret.
pub(crate) fn grant_of(
    ctx: &Ctx,
    family: &str,
    endpoint: &str,
    member: &str,
) -> Result<Grant, Fault> {
    let read = |key: &str| configured_in(ctx, family, endpoint, key);
    let missing = |key: &str| {
        Fault::thrown(format!(
            "{member}: `[{family}.{endpoint}]` has `auth = \"xoauth2\"` and no `{key}`"
        ))
    };
    let user = read("user").ok_or_else(|| missing("user"))?;
    if read("password").is_some() {
        return Err(Fault::thrown(format!(
            "{member}: `[{family}.{endpoint}]` has `auth = \"xoauth2\"` and a password, which \
             it never sends"
        )));
    }
    let written = read("oauth_token_url").ok_or_else(|| missing("oauth_token_url"))?;
    let url = nvs_config::mail::token_url(&written).ok_or_else(|| {
        Fault::thrown(format!(
            "{member}: `[{family}.{endpoint}] oauth_token_url` is not an `https://` URL"
        ))
    })?;
    Ok(Grant {
        user,
        url,
        client_id: read("oauth_client_id").ok_or_else(|| missing("oauth_client_id"))?,
        client_secret: read("oauth_client_secret").ok_or_else(|| missing("oauth_client_secret"))?,
        scope: read("oauth_scope"),
        refresh_token: read("oauth_refresh_token"),
    })
}

/// The bearer token for `block`, `mail.<name>` or `imap.<name>`: the kept one while it is fresh,
/// else a new one from the token server. Every wait ends at `until`.
///
/// # Errors
///
/// [`fetch`]'s.
pub(crate) fn token(
    grant: &Grant,
    block: &str,
    until: Instant,
    member: &str,
) -> Result<String, Fault> {
    token_at(&CACHE, grant, block, Instant::now(), until, member)
}

/// Drops `block`'s kept token, after the server refused it.
pub(crate) fn forget(block: &str) {
    CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(block);
}

/// [`token`] over `cache`, as of `now`.
fn token_at(
    cache: &Cache,
    grant: &Grant,
    block: &str,
    now: Instant,
    until: Instant,
    member: &str,
) -> Result<String, Fault> {
    let print = fingerprint(grant);
    if let Some(kept) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(block)
        .filter(|kept| kept.print == print && now < kept.renew_at)
    {
        return Ok(kept.token.clone());
    }
    let (token, lasts) = fetch(grant, block, until, member)?;
    let mut kept = cache.lock().unwrap_or_else(PoisonError::into_inner);
    match lasts
        .and_then(|lasts| lasts.checked_sub(MARGIN))
        .and_then(|fresh| now.checked_add(fresh))
    {
        Some(renew_at) => {
            kept.insert(
                block.to_owned(),
                Kept {
                    print,
                    token: token.clone(),
                    renew_at,
                },
            );
        }
        // A token that lasts no longer than the margin, or names no lifetime, is used once.
        None => {
            kept.remove(block);
        }
    }
    Ok(token)
}

/// A SHA-256 of every value the token request is made of, so a changed value fetches anew.
fn fingerprint(grant: &Grant) -> [u8; 32] {
    let mut digest = ring::digest::Context::new(&ring::digest::SHA256);
    for part in [
        grant.user.as_str(),
        grant.url.host.as_str(),
        &grant.url.port.to_string(),
        grant.url.target.as_str(),
        grant.client_id.as_str(),
        grant.client_secret.as_str(),
        grant.scope.as_deref().unwrap_or("\u{0}none"),
        grant.refresh_token.as_deref().unwrap_or("\u{0}none"),
    ] {
        digest.update(&part.len().to_be_bytes());
        digest.update(part.as_bytes());
    }
    let mut print = [0_u8; 32];
    print.copy_from_slice(digest.finish().as_ref());
    print
}

/// `value` as `application/x-www-form-urlencoded`: every byte but RFC 3986's unreserved ones as
/// `%XX`.
fn form(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The token request's body: the grant, the client, and the scope when written.
fn body_of(grant: &Grant) -> String {
    let mut fields = match &grant.refresh_token {
        Some(refresh) => vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
        ],
        None => vec![("grant_type", "client_credentials")],
    };
    fields.push(("client_id", &grant.client_id));
    fields.push(("client_secret", &grant.client_secret));
    if let Some(scope) = &grant.scope {
        fields.push(("scope", scope));
    }
    fields
        .iter()
        .map(|(key, value)| format!("{key}={}", form(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// The whole HTTP/1.1 request, closed after its reply.
fn request_of(grant: &Grant) -> String {
    let url = &grant.url;
    let host = if url.host.contains(':') {
        format!("[{}]", url.host)
    } else {
        url.host.clone()
    };
    let authority = if url.port == 443 {
        host
    } else {
        format!("{host}:{}", url.port)
    };
    let body = body_of(grant);
    format!(
        "POST {} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: \
         application/x-www-form-urlencoded\r\nAccept: application/json\r\nContent-Length: \
         {}\r\nConnection: close\r\n\r\n{body}",
        url.target,
        body.len()
    )
}

/// The anchors the token server's certificate is checked against: the compiled-in ones.
#[cfg(not(test))]
fn anchors() -> Option<std::path::PathBuf> {
    None
}

#[cfg(test)]
thread_local! {
    /// The test's own CA, for the fake token server on this thread.
    static ANCHORS: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// In a test, the CA the test set for its thread.
#[cfg(test)]
fn anchors() -> Option<std::path::PathBuf> {
    ANCHORS.with(|anchors| anchors.borrow().clone())
}

/// A new token and how long it lasts, from the token server. Every wait ends at `until`.
///
/// # Errors
///
/// A catchable `IOError` naming the block when the server cannot be reached, its certificate does
/// not verify, it answers a status other than 200 (named, with the OAuth `error` code when it
/// sends one), or its reply has no usable `access_token`. Nothing has been sent to the relay.
fn fetch(
    grant: &Grant,
    block: &str,
    until: Instant,
    member: &str,
) -> Result<(String, Option<Duration>), Fault> {
    let server = &grant.url.host;
    let failed = |why: &str| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: `[{block}]` could not fetch its token from {server}: {why}. \
                 Nothing was sent."
            ),
        )
    };
    let addresses: Vec<SocketAddr> =
        nvs_runtime::capability::resolve_addresses(server, member, Some(until))
            .map_err(|_| failed("the name resolves to no address"))?
            .into_iter()
            .map(|ip| SocketAddr::new(ip, grant.url.port))
            .collect();
    let budget = until.saturating_duration_since(Instant::now());
    let mut stream =
        NvsTcp::connect_racing(&addresses, budget).map_err(|why| failed(&why.to_string()))?;
    stream.set_deadline(Some(until));
    let mut tls = match anchors() {
        Some(bundle) => NvsTls::over_bundle(stream, server, &bundle),
        None => NvsTls::over(stream, server),
    }
    .map_err(|why| failed(&format!("the TLS handshake did not complete ({why})")))?;
    tls.write_all(request_of(grant).as_bytes())
        .and_then(|()| tls.flush())
        .map_err(|why| failed(&why.to_string()))?;
    let raw = read_reply(&mut tls).map_err(|why| failed(&why))?;
    let (status, body) = parse_reply(&raw).ok_or_else(|| failed("its reply is not HTTP"))?;
    let json: Option<serde_json::Value> = serde_json::from_slice(&body).ok();
    if status != 200 {
        let code = json
            .as_ref()
            .and_then(|json| json.get("error"))
            .and_then(serde_json::Value::as_str)
            .filter(|code| {
                !code.is_empty()
                    && code.len() <= 64
                    && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            .map(|code| format!(" (`{code}`)"))
            .unwrap_or_default();
        return Err(Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: the token server of `[{block}]` answered {status}{code}. Nothing \
                 was sent."
            ),
        ));
    }
    let json = json.ok_or_else(|| failed("its reply is not JSON"))?;
    let token = json
        .get("access_token")
        .and_then(serde_json::Value::as_str)
        .filter(|token| !token.is_empty() && token.bytes().all(|b| (0x21..=0x7e).contains(&b)))
        .ok_or_else(|| failed("its reply has no `access_token`"))?;
    if json
        .get("token_type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| !kind.eq_ignore_ascii_case("bearer"))
    {
        return Err(failed("its token is not a bearer token"));
    }
    let lasts = json.get("expires_in").and_then(|lasts| {
        lasts
            .as_u64()
            .or_else(|| lasts.as_str().and_then(|text| text.parse().ok()))
            .map(Duration::from_secs)
    });
    Ok((token.to_owned(), lasts))
}

/// The reply's bytes, read until it is complete or the server closes, and at most
/// [`REPLY_CEILING`].
fn read_reply(stream: &mut impl Read) -> Result<Vec<u8>, String> {
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => return Ok(raw),
            Ok(read) => raw.extend_from_slice(&chunk[..read]),
            // A server that closes without TLS's `close_notify` has still sent a whole reply
            // when its length says so, which `parse_reply` checks.
            Err(why) if why.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(raw),
            Err(why) => return Err(why.to_string()),
        }
        if raw.len() > REPLY_CEILING {
            return Err(format!("its reply is over {REPLY_CEILING} bytes"));
        }
        if parse_reply(&raw).is_some() {
            return Ok(raw);
        }
    }
}

/// The status and the body of a complete reply, or `None` while it is incomplete or is not HTTP.
///
/// A body is complete at its `Content-Length` or at the last chunk of a chunked one. A reply with
/// neither is not read: every token server answers HTTP/1.1 with one of the two.
fn parse_reply(raw: &[u8]) -> Option<(u16, Vec<u8>)> {
    let end = raw.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..end]).ok()?;
    let rest = &raw[end + 4..];
    let mut lines = head.split("\r\n");
    let status = lines
        .next()?
        .strip_prefix("HTTP/1.")?
        .get(2..5)?
        .parse::<u16>()
        .ok()?;
    let mut length = None;
    let mut chunked = false;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            length = Some(value.parse::<usize>().ok()?);
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            chunked = value.eq_ignore_ascii_case("chunked");
        }
    }
    if chunked {
        return dechunk(rest).map(|body| (status, body));
    }
    let length = length?;
    rest.get(..length).map(|body| (status, body.to_vec()))
}

/// A chunked body's content, or `None` until its last chunk has arrived.
fn dechunk(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line = rest.windows(2).position(|window| window == b"\r\n")?;
        let size = std::str::from_utf8(&rest[..line]).ok()?;
        let size = size.split(';').next()?.trim();
        let size = usize::from_str_radix(size, 16).ok()?;
        rest = &rest[line + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use nvs_config::mail::Tls;

    use super::super::smtp::tests::{Fake, MESSAGE, ada, block, localhost, send_to, serve};
    use super::*;

    /// A fake token server on a loopback port: one connection per reply in `replies`, each
    /// answered with that status and JSON body. The thread returns each request it read.
    fn token_server(
        certificate: Arc<rustls::ServerConfig>,
        replies: Vec<(u16, &'static str)>,
    ) -> (u16, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("its address").port();
        let thread = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for (status, body) in replies {
                let Ok((socket, _)) = listener.accept() else {
                    break;
                };
                drop(socket.set_read_timeout(Some(Duration::from_secs(10))));
                let Ok(session) = rustls::ServerConnection::new(Arc::clone(&certificate)) else {
                    break;
                };
                let mut wire = rustls::StreamOwned::new(session, socket);
                let mut raw = Vec::new();
                let mut byte = [0_u8; 1];
                while !raw.ends_with(b"\r\n\r\n") {
                    match wire.read(&mut byte) {
                        Ok(1) => raw.push(byte[0]),
                        _ => break,
                    }
                }
                let head = String::from_utf8_lossy(&raw).into_owned();
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .and_then(|length| length.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                let mut request = vec![0_u8; length];
                drop(wire.read_exact(&mut request));
                seen.push(format!("{head}{}", String::from_utf8_lossy(&request)));
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: \
                     {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                drop(wire.write_all(reply.as_bytes()));
                wire.conn.send_close_notify();
                drop(wire.flush());
            }
            seen
        });
        (port, thread)
    }

    /// A grant for `https://localhost:<port>/token`, with a refresh token when one is given.
    fn grant(port: u16, refresh_token: Option<&str>) -> Grant {
        Grant {
            user: "noreply@example.com".to_owned(),
            url: nvs_config::mail::token_url(&format!("https://localhost:{port}/token"))
                .expect("a token URL"),
            client_id: "app".to_owned(),
            client_secret: "s3cret".to_owned(),
            scope: Some("https://mail.example.com/.default".to_owned()),
            refresh_token: refresh_token.map(str::to_owned),
        }
    }

    fn trust(ca: &std::path::Path) {
        ANCHORS.with(|anchors| *anchors.borrow_mut() = Some(ca.to_owned()));
    }

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }

    /// Two sends through a client-credentials block fetch one token and sign in with it twice.
    /// The token request carries the grant, the client and the scope.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`ring` runs C and assembly, which Miri cannot interpret"
    )]
    #[test]
    fn a_client_credentials_token_is_fetched_once_and_reused() {
        let (certificate, _dir, ca) = localhost();
        trust(&ca);
        let (port, tokens) = token_server(
            Arc::clone(&certificate),
            vec![(
                200,
                r#"{"access_token":"tok-1","token_type":"Bearer","expires_in":3600}"#,
            )],
        );
        let mut endpoint = block(Tls::StartTls);
        endpoint.name = "oauth-once".to_owned();
        endpoint.ca_file = Some(ca);
        endpoint.oauth = Some(grant(port, None));
        let mut auth_lines = Vec::new();
        for _ in 0..2 {
            let (address, server) = serve(Fake {
                starttls: true,
                extensions: &["AUTH PLAIN XOAUTH2"],
                certificate: Some(Arc::clone(&certificate)),
                ..Fake::default()
            });
            send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE)
                .expect("the message is sent with the token");
            let seen = server.join().expect("the server");
            assert!(seen.contains(&"<body>".to_owned()), "{seen:?}");
            auth_lines.extend(seen.into_iter().filter(|line| line.starts_with("AUTH")));
        }
        let requests = tokens.join().expect("the token server");
        assert_eq!(requests.len(), 1, "{requests:?}");
        assert!(
            requests[0].starts_with("POST /token HTTP/1.1\r\n"),
            "{requests:?}"
        );
        assert!(
            requests[0].ends_with(
                "grant_type=client_credentials&client_id=app&client_secret=s3cret\
                 &scope=https%3A%2F%2Fmail.example.com%2F.default"
            ),
            "{requests:?}"
        );
        let expected = format!(
            "AUTH XOAUTH2 {}",
            STANDARD.encode("user=noreply@example.com\u{1}auth=Bearer tok-1\u{1}\u{1}")
        );
        assert_eq!(auth_lines, vec![expected.clone(), expected]);
    }

    /// A kept token is reused until a minute before it expires, then fetched again. A token that
    /// lasts no longer than that minute is never kept.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`ring` runs C and assembly, which Miri cannot interpret"
    )]
    #[test]
    fn a_token_is_fetched_again_before_it_expires() {
        let (certificate, _dir, ca) = localhost();
        trust(&ca);
        let (port, tokens) = token_server(
            certificate,
            vec![
                (200, r#"{"access_token":"tok-1","expires_in":120}"#),
                (200, r#"{"access_token":"tok-2","expires_in":"30"}"#),
                (200, r#"{"access_token":"tok-3","expires_in":120}"#),
            ],
        );
        let cache = Cache::default();
        let grant = grant(port, None);
        let at = |now: Instant| {
            token_at(
                &cache,
                &grant,
                "oauth-expiry",
                now,
                soon(),
                "Core\\Mail::send",
            )
            .expect("a token")
        };
        let start = Instant::now();
        assert_eq!(at(start), "tok-1");
        assert_eq!(at(start + Duration::from_secs(59)), "tok-1");
        assert_eq!(at(start + Duration::from_secs(61)), "tok-2");
        assert_eq!(at(start + Duration::from_secs(62)), "tok-3");
        assert_eq!(tokens.join().expect("the token server").len(), 3);
    }

    /// A block with a refresh token sends it with the refresh-token grant.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`ring` runs C and assembly, which Miri cannot interpret"
    )]
    #[test]
    fn a_refresh_token_block_sends_its_refresh_token() {
        let (certificate, _dir, ca) = localhost();
        trust(&ca);
        let (port, tokens) = token_server(
            certificate,
            vec![(200, r#"{"access_token":"tok-1","expires_in":3600}"#)],
        );
        let cache = Cache::default();
        let token = token_at(
            &cache,
            &grant(port, Some("1//r e")),
            "oauth-refresh",
            Instant::now(),
            soon(),
            "Core\\Mail::send",
        )
        .expect("a token");
        assert_eq!(token, "tok-1");
        let requests = tokens.join().expect("the token server");
        assert!(
            requests[0].ends_with(
                "grant_type=refresh_token&refresh_token=1%2F%2Fr%20e&client_id=app\
                 &client_secret=s3cret&scope=https%3A%2F%2Fmail.example.com%2F.default"
            ),
            "{requests:?}"
        );
    }

    /// A token server that refuses the client gives an `IOError` naming the block, the status and
    /// the OAuth error code. The relay is never reached.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`ring` runs C and assembly, which Miri cannot interpret"
    )]
    #[test]
    fn a_refused_token_names_the_block_and_sends_nothing() {
        let (certificate, _dir, ca) = localhost();
        trust(&ca);
        let (port, tokens) = token_server(
            certificate,
            vec![(
                400,
                r#"{"error":"invalid_client","error_description":"no"}"#,
            )],
        );
        let relay = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let mut endpoint = block(Tls::StartTls);
        endpoint.name = "oauth-refused".to_owned();
        endpoint.oauth = Some(grant(port, None));
        let address = relay.local_addr().expect("its address");
        let Err(fault) = send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE) else {
            panic!("a send went on without a token");
        };
        let said = format!("{fault:?}");
        assert!(
            said.contains("`[mail.oauth-refused]`")
                && said.contains("400")
                && said.contains("invalid_client")
                && said.contains("Nothing was sent"),
            "{said}"
        );
        assert_eq!(tokens.join().expect("the token server").len(), 1);
        relay
            .set_nonblocking(true)
            .expect("a non-blocking listener");
        assert!(
            relay.accept().is_err(),
            "the relay was reached after the token was refused"
        );
    }
}
