//! `nvs debug`'s listener: the viewer's page, its two files and its API, served to that page and
//! to nothing else (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! **One thread per connection, and every read and write blocks it.** `nvs debug` has no core and
//! no reactor, so `hyper` is driven by [`nvs_host::block_on`], which parks the thread off a core,
//! over a transport whose every poll is a blocking call. `half_close` is on, so `hyper` never reads
//! while it writes a response: an event stream waits on the watermark and never on the browser.
//! A read waits at most [`IDLE`] and a write at most [`STALLED`], so a tab that went away ends its
//! thread. Past [`MAX_CONNECTIONS`] a new connection is closed at once.
//!
//! **Who is served.** On loopback, a `Host` must name a loopback address or `localhost` with the
//! bound port, which turns away a page that rebinds its own name to this address. Off loopback the
//! name a browser uses is not known here, because a published port and a container's name change
//! it, so the token decides instead: [`super::secure`] makes the TLS configuration and the token,
//! the printed link carries the token once, and from then on an `HttpOnly`, `Secure`,
//! `SameSite=Strict` cookie does. Everywhere, an `Origin` that is sent must be the page's own, a
//! request that changes something needs that `Origin` and a JSON body, and every response carries
//! [`CSP`], `nosniff`, `no-referrer` and `no-store`.
//!
//! **The watermark is pushed, never the records**
//! (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). The ingest loop publishes the last committed batch into a [`Watermark`],
//! and each `/api/events` stream sends the newest number it has not sent yet. A tab that does not
//! read blocks its own thread's write and nothing else, and a number it missed is replaced by the
//! next one, so nothing buffers.
//!
//! **A body is read only once the checks pass.** `PUT /api/settings` is the one route with a body,
//! and it is read up to [`settings::BODY_MAX`] after the host, the origin and the token are
//! admitted. A longer body is a `413`.
//!
//! **What it spends:** a thread per open connection, at most [`MAX_CONNECTIONS`], each with its
//! connection's buffers and the stack `std` gives a thread, and while a settings body is read, that
//! body. One [`Watermark`] and one [`Api`] for the service. The two embedded files are sent from
//! the binary's read-only data.

use std::convert::Infallible;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError, mpsc};
use std::task::{Context, Poll};
use std::time::Duration;

use hyper::body::{Bytes, Frame, Incoming, SizeHint};
use hyper::header::{self, HeaderMap, HeaderValue};
use hyper::rt::ReadBufCursor;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};

use super::Signal;
use super::api::Api;
use super::secure::Secure;
use super::settings::{self, Store};
use super::{search, stream};
use crate::viewer;

/// The most connections served at once. A browser opens at most six to one address, so this covers
/// several tabs, and a thread each is a bound the service can afford.
pub(crate) const MAX_CONNECTIONS: usize = 64;

/// How long a read waits: for a request's head, its body, or the next request on an idle
/// connection.
const IDLE: Duration = Duration::from_secs(60);

/// How long a write waits on a browser that does not read.
const STALLED: Duration = Duration::from_secs(30);

/// How often an event stream with nothing new sends a comment, which is how a closed tab is found.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// The cookie that carries the token off loopback. `__Host-` makes the browser keep it only when it
/// is `Secure`, has `Path=/` and names no domain.
const COOKIE: &str = "__Host-nvs-debug";

/// The page, the same for every view path: the app mounts into `#app` and routes on the path.
pub(crate) const PAGE: &str = "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Novis debug</title>\
<link rel=\"stylesheet\" href=\"/viewer.css\"></head>\n<body><div id=\"app\"></div>\
<script type=\"module\" src=\"/viewer.js\"></script></body>\n</html>\n";

/// The policy every response carries: no inline script, no inline style, no `eval` and no other
/// origin.
pub(crate) const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; \
frame-ancestors 'none'";

/// The address and files `[debug]` names, with `--listen` already applied.
pub(crate) struct Listen {
    /// `[debug] listen`, as written.
    pub(crate) addr: String,
    /// `[debug] tls_cert_file`.
    pub(crate) cert: Option<std::path::PathBuf>,
    /// `[debug] tls_key_file`.
    pub(crate) key: Option<std::path::PathBuf>,
    /// `[debug] token_file`.
    pub(crate) token: Option<std::path::PathBuf>,
}

/// Binds `listen`, starts the thread that accepts on it, and answers the link to print. The error
/// is the message to print. `signal` reaches the ingest loop, for `POST /api/index/discard`.
pub(crate) fn start(
    listen: &Listen,
    folder: &nvs_config::data::Folder,
    watermark: Arc<Watermark>,
    signal: mpsc::Sender<Signal>,
) -> Result<String, String> {
    let wanted: SocketAddr = listen.addr.parse().map_err(|_| {
        format!(
            "`[debug] listen` is `{}`, and it must be an IP address and a port, \
             such as `127.0.0.1:8137`",
            listen.addr
        )
    })?;
    let socket = TcpListener::bind(wanted).map_err(|err| {
        format!(
            "the debug viewer cannot listen on `{wanted}`: {err}. \
             Set another address with `[debug] listen` or `--listen`."
        )
    })?;
    let addr = socket.local_addr().map_err(|err| err.to_string())?;
    let index = folder.debug_index();
    let api = Api::open(&index, signal)
        .map_err(|err| format!("the debug viewer cannot read `{}`: {err}", index.display()))?;
    let secure = if addr.ip().is_loopback() {
        None
    } else {
        Some(Secure::load(listen, folder, addr.ip())?)
    };
    let link = match &secure {
        None => format!("http://{addr}/"),
        Some(secure) => {
            let host = if addr.ip().is_unspecified() {
                format!("localhost:{}", addr.port())
            } else {
                addr.to_string()
            };
            format!("https://{host}/?token={}", secure.token())
        }
    };
    let site = Arc::new(Site {
        addr,
        secure,
        watermark,
        api,
        settings: Store::new(folder.viewer_settings()),
    });
    std::thread::Builder::new()
        .name("nvs-debug-accept".to_owned())
        .spawn(move || accept(&socket, &site))
        .map_err(|err| format!("the debug viewer cannot start: {err}"))?;
    Ok(link)
}

/// Accepts until the process ends, a thread per connection.
fn accept(socket: &TcpListener, site: &Arc<Site>) {
    let open = Arc::new(AtomicUsize::new(0));
    for stream in socket.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(err) => {
                // A reset before the accept is the peer's. Running out of descriptors is the
                // process's, and the pause keeps this loop from spinning until some are free.
                if err.kind() != io::ErrorKind::ConnectionAborted {
                    std::thread::sleep(Duration::from_millis(100));
                }
                continue;
            }
        };
        if open.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            open.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let slot = Slot(Arc::clone(&open));
        let site = Arc::clone(site);
        // A thread that cannot start drops the closure, and with it the slot.
        let _ = std::thread::Builder::new()
            .name("nvs-debug-viewer".to_owned())
            .spawn(move || {
                let _slot = slot;
                connection(stream, &site);
            });
    }
}

/// One of [`MAX_CONNECTIONS`], given back when the connection's thread ends.
struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Serves one connection until the browser closes it, a wait runs out, or `hyper` ends it.
fn connection(stream: TcpStream, site: &Site) {
    if stream.set_read_timeout(Some(IDLE)).is_err()
        || stream.set_write_timeout(Some(STALLED)).is_err()
    {
        return;
    }
    let _ = stream.set_nodelay(true);
    let transport = match &site.secure {
        None => Transport::Plain(stream),
        Some(secure) => match rustls::ServerConnection::new(secure.tls()) {
            Ok(tls) => Transport::Tls(Box::new(rustls::StreamOwned::new(tls, stream))),
            Err(_) => return,
        },
    };
    let service = service_fn(|request: Request<Incoming>| async move {
        Ok::<_, Infallible>(site.answer(request).await)
    });
    let _ = nvs_host::block_on(
        http1::Builder::new()
            .half_close(true)
            .serve_connection(Blocking(transport), service),
    );
}

/// What every connection shares: the bound address, the off-loopback secrets, the watermark, the
/// index's read connection and the settings file.
struct Site {
    addr: SocketAddr,
    secure: Option<Secure>,
    watermark: Arc<Watermark>,
    api: Api,
    settings: Store,
}

impl Site {
    /// The response to `request`, with the headers every response carries.
    async fn answer(&self, request: Request<Incoming>) -> Response<Body> {
        let (parts, incoming) = request.into_parts();
        let request = Request::from_parts(parts, ());
        let mut response = match self.refused(&request) {
            Some(refused) => refused,
            None if *request.method() == Method::PUT => {
                match read_body(incoming, settings::BODY_MAX).await {
                    Some(body) => self.route(&request, &body),
                    None => text(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "The body is too large, or it ended early.",
                    ),
                }
            }
            None => self.route(&request, &[]),
        };
        let headers = response.headers_mut();
        for (name, value) in [
            (header::CONTENT_SECURITY_POLICY, CSP),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
            (header::CACHE_CONTROL, "no-store"),
            (
                header::HeaderName::from_static("cross-origin-resource-policy"),
                "same-origin",
            ),
        ] {
            headers.insert(name, HeaderValue::from_static(value));
        }
        response
    }

    /// The checks in the module doc's order: the response that turns `request` away, or `None`
    /// when it may be routed.
    fn refused(&self, request: &Request<()>) -> Option<Response<Body>> {
        let headers = request.headers();
        let Some(host) = single(headers, &header::HOST) else {
            return Some(text(
                StatusCode::BAD_REQUEST,
                "The request needs one `Host` header.",
            ));
        };
        if !self.admits_host(host) {
            return Some(text(
                StatusCode::MISDIRECTED_REQUEST,
                "This host name is not allowed. Open the address that `nvs debug` printed.",
            ));
        }
        let changing = !matches!(*request.method(), Method::GET | Method::HEAD);
        match single(headers, &header::ORIGIN) {
            Some(origin) if !self.is_own_origin(origin, host) => {
                return Some(text(StatusCode::FORBIDDEN, "This origin is not allowed."));
            }
            None if changing || headers.contains_key(header::ORIGIN) => {
                return Some(text(
                    StatusCode::FORBIDDEN,
                    "A request that changes something needs one `Origin` header.",
                ));
            }
            _ => {}
        }
        if changing && !is_json(headers) {
            return Some(text(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "A request that changes something needs a JSON body.",
            ));
        }
        if let Some(secure) = &self.secure {
            if let Some(token) = query_token(request.uri().query()) {
                return Some(if secure.admits(token) {
                    admitted(request.uri().path(), token)
                } else {
                    text(StatusCode::UNAUTHORIZED, "This token is not valid.")
                });
            }
            if !cookie_token(headers).is_some_and(|token| secure.admits(token)) {
                return Some(text(
                    StatusCode::UNAUTHORIZED,
                    "Open the link that `nvs debug` printed. It contains the token.",
                ));
            }
        }
        None
    }

    /// The route of an admitted `request`, with its `body`.
    fn route(&self, request: &Request<()>, body: &[u8]) -> Response<Body> {
        let path = request.uri().path();
        let query = request.uri().query();
        let reads = matches!(*request.method(), Method::GET | Method::HEAD);
        let method = request.method();
        match path {
            "/viewer.js" if reads => file(viewer::JS, "text/javascript; charset=utf-8"),
            "/viewer.css" if reads => file(viewer::CSS, "text/css; charset=utf-8"),
            "/api/events" if *method == Method::GET => events(Arc::clone(&self.watermark)),
            "/api/requests" if reads => api(self.api.requests(query)),
            "/api/request" if reads => api(self.api.request(query)),
            "/api/stream" if reads => api(stream::stream(&self.api, query)),
            "/api/quarantine" if reads => api(stream::quarantine(&self.api)),
            "/api/search" if reads => api(search::search(&self.api, query)),
            "/api/settings" if reads => api(self.settings.get()),
            "/api/settings" if *method == Method::PUT => api(self.settings.put(body)),
            "/api/index/discard" if *method == Method::POST => api(self.api.discard()),
            _ if path.starts_with("/api/") => json(
                StatusCode::NOT_FOUND,
                "{\"error\":\"There is no such API path.\"}",
            ),
            _ if reads => file(PAGE.as_bytes(), "text/html; charset=utf-8"),
            _ => {
                let mut response = text(
                    StatusCode::METHOD_NOT_ALLOWED,
                    "This path takes `GET` and `HEAD`.",
                );
                response
                    .headers_mut()
                    .insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
                response
            }
        }
    }

    /// Whether `host` may name this listener: anything well formed off loopback, where the token
    /// decides, and on loopback a loopback name with the bound port.
    fn admits_host(&self, host: &str) -> bool {
        let Some((name, port)) = split_host(host) else {
            return false;
        };
        if self.secure.is_some() {
            return true;
        }
        let port = port.unwrap_or(80);
        port == self.addr.port()
            && (name.eq_ignore_ascii_case("localhost")
                || name.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()))
    }

    /// Whether `origin` is the page's own: this listener's scheme and the `Host` already admitted.
    fn is_own_origin(&self, origin: &str, host: &str) -> bool {
        let scheme = if self.secure.is_some() {
            "https://"
        } else {
            "http://"
        };
        origin
            .strip_prefix(scheme)
            .is_some_and(|rest| rest.eq_ignore_ascii_case(host))
    }
}

/// The one value of `name` as text, or `None` when it is absent, repeated or not text.
fn single<'a>(headers: &'a HeaderMap, name: &header::HeaderName) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    value.to_str().ok()
}

/// A `Host` value's name and port. The name of an IPv6 address keeps no brackets.
fn split_host(host: &str) -> Option<(&str, Option<u16>)> {
    let (name, port) = if let Some(rest) = host.strip_prefix('[') {
        let (name, after) = rest.split_once(']')?;
        match after {
            "" => (name, None),
            _ => (name, Some(after.strip_prefix(':')?)),
        }
    } else {
        match host.split_once(':') {
            Some((name, port)) => (name, Some(port)),
            None => (host, None),
        }
    };
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
    {
        return None;
    }
    let port = match port {
        Some(port) => Some(port.parse::<u16>().ok()?),
        None => None,
    };
    Some((name, port))
}

/// Whether the body is declared JSON.
fn is_json(headers: &HeaderMap) -> bool {
    single(headers, &header::CONTENT_TYPE).is_some_and(|value| {
        value
            .split(';')
            .next()
            .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
    })
}

/// The `token` parameter of a query.
fn query_token(query: Option<&str>) -> Option<&str> {
    query?
        .split('&')
        .find_map(|pair| pair.strip_prefix("token="))
}

/// The token the [`COOKIE`] cookie carries.
fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| pair.trim().strip_prefix(COOKIE)?.strip_prefix('='))
}

/// The link's token accepted: the cookie set, and the browser sent to the same path without it.
fn admitted(path: &str, token: &str) -> Response<Body> {
    let cookie = format!("{COOKIE}={token}; Path=/; Secure; HttpOnly; SameSite=Strict");
    let mut response = text(StatusCode::SEE_OTHER, "The token is accepted.");
    let headers = response.headers_mut();
    if let (Ok(location), Ok(cookie)) =
        (HeaderValue::from_str(path), HeaderValue::from_str(&cookie))
    {
        headers.insert(header::LOCATION, location);
        headers.insert(header::SET_COOKIE, cookie);
    }
    response
}

/// An API answer as a JSON response.
fn api((status, body): (StatusCode, Vec<u8>)) -> Response<Body> {
    with(
        status,
        "application/json",
        Body::Bytes(Some(Bytes::from(body))),
    )
}

/// The body of a request, or `None` when it is longer than `max` bytes or fails partway.
async fn read_body(mut incoming: Incoming, max: usize) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(frame) =
        std::future::poll_fn(|cx| hyper::body::Body::poll_frame(Pin::new(&mut incoming), cx)).await
    {
        if let Ok(data) = frame.ok()?.into_data() {
            if bytes.len() + data.len() > max {
                return None;
            }
            bytes.extend_from_slice(&data);
        }
    }
    Some(bytes)
}

fn file(bytes: &'static [u8], content_type: &'static str) -> Response<Body> {
    with(
        StatusCode::OK,
        content_type,
        Body::Bytes(Some(Bytes::from_static(bytes))),
    )
}

fn text(status: StatusCode, message: &'static str) -> Response<Body> {
    with(
        status,
        "text/plain; charset=utf-8",
        Body::Bytes(Some(Bytes::from_static(message.as_bytes()))),
    )
}

fn json(status: StatusCode, body: &'static str) -> Response<Body> {
    with(
        status,
        "application/json",
        Body::Bytes(Some(Bytes::from_static(body.as_bytes()))),
    )
}

/// The event stream of the watermark.
fn events(watermark: Arc<Watermark>) -> Response<Body> {
    with(
        StatusCode::OK,
        "text/event-stream",
        Body::Events(Events {
            watermark,
            seen: None,
            sent: false,
        }),
    )
}

fn with(status: StatusCode, content_type: &'static str, body: Body) -> Response<Body> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}

/// The number of the last committed batch, published by the ingest loop and read by every event
/// stream.
pub(crate) struct Watermark {
    batch: Mutex<i64>,
    raised: Condvar,
}

impl Watermark {
    pub(crate) fn new(batch: i64) -> Arc<Self> {
        Arc::new(Self {
            batch: Mutex::new(batch),
            raised: Condvar::new(),
        })
    }

    /// Publishes `batch` and wakes every stream when it changed. A new index starts again at `0`,
    /// so any change counts, not only a rise.
    pub(crate) fn publish(&self, batch: i64) {
        let mut current = self.batch.lock().unwrap_or_else(PoisonError::into_inner);
        if *current != batch {
            *current = batch;
            self.raised.notify_all();
        }
    }

    /// The batch once it is not `seen`, waiting at most `wait`. `None` when the wait ran out.
    fn after(&self, seen: Option<i64>, wait: Duration) -> Option<i64> {
        let current = self.batch.lock().unwrap_or_else(PoisonError::into_inner);
        let (current, _) = self
            .raised
            .wait_timeout_while(current, wait, |batch| Some(*batch) == seen)
            .unwrap_or_else(PoisonError::into_inner);
        (Some(*current) != seen).then_some(*current)
    }
}

/// A response body: bytes sent once, or the watermark's event stream.
enum Body {
    Bytes(Option<Bytes>),
    Events(Events),
}

/// One tab's event stream: the last number it was sent.
struct Events {
    watermark: Arc<Watermark>,
    seen: Option<i64>,
    /// Whether the last poll gave an event. The poll after it returns `Pending` once, so `hyper`
    /// flushes the event before the stream waits for the next one.
    sent: bool,
}

impl hyper::body::Body for Body {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        match self.get_mut() {
            Body::Bytes(bytes) => Poll::Ready(bytes.take().map(|bytes| Ok(Frame::data(bytes)))),
            Body::Events(events) => {
                if std::mem::take(&mut events.sent) {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                events.sent = true;
                // The thread is this connection's alone, so the wait holds up nothing else.
                let event = match events.watermark.after(events.seen, KEEP_ALIVE) {
                    Some(batch) => {
                        events.seen = Some(batch);
                        Bytes::from(format!("data: {batch}\n\n"))
                    }
                    None => Bytes::from_static(b": keep-alive\n\n"),
                };
                Poll::Ready(Some(Ok(Frame::data(event))))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(self, Body::Bytes(None))
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Body::Bytes(bytes) => {
                SizeHint::with_exact(bytes.as_ref().map_or(0, |b| b.len() as u64))
            }
            Body::Events(_) => SizeHint::default(),
        }
    }
}

/// An accepted connection, in the clear on loopback and over TLS off it.
enum Transport {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ServerConnection, TcpStream>>),
}

impl Read for Transport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Transport::Plain(stream) => stream.read(buf),
            Transport::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for Transport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Transport::Plain(stream) => stream.write(buf),
            Transport::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Transport::Plain(stream) => stream.flush(),
            Transport::Tls(stream) => stream.flush(),
        }
    }
}

/// [`Transport`] as the two traits `hyper` drives, where every poll is the blocking call and is
/// therefore never `Pending`. The read is a copy through zeroed scratch, because this crate forbids
/// the `unsafe` that filling `hyper`'s cursor directly would take.
struct Blocking(Transport);

impl hyper::rt::Read for Blocking {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        let mut scratch = [0u8; 8 * 1024];
        let room = buf.remaining().min(scratch.len());
        Poll::Ready(self.get_mut().0.read(&mut scratch[..room]).map(|read| {
            buf.put_slice(&scratch[..read]);
        }))
    }
}

impl hyper::rt::Write for Blocking {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.get_mut().0.write(buf))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.get_mut().0.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(match &mut self.get_mut().0 {
            Transport::Plain(stream) => stream.shutdown(Shutdown::Write),
            Transport::Tls(stream) => {
                stream.conn.send_close_notify();
                stream.flush()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_splits_into_its_name_and_port() {
        assert_eq!(
            split_host("127.0.0.1:8137"),
            Some(("127.0.0.1", Some(8137)))
        );
        assert_eq!(split_host("[::1]:8137"), Some(("::1", Some(8137))));
        assert_eq!(split_host("localhost"), Some(("localhost", None)));
        assert_eq!(split_host("evil.example/x:1"), None);
        assert_eq!(split_host("localhost:99999"), None);
        assert_eq!(split_host(""), None);
    }

    #[test]
    fn the_token_is_read_from_the_query_and_the_cookie() {
        assert_eq!(query_token(Some("a=1&token=abc")), Some("abc"));
        assert_eq!(query_token(None), None);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("x=1; __Host-nvs-debug=abc"),
        );
        assert_eq!(cookie_token(&headers), Some("abc"));
    }
}
