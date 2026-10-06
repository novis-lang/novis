//! `wasi:http`'s outgoing handler over [`Budget::send`]: a guest's request leaves through
//! `Core\Http\Client`'s transport, so the caller's `net.connect`, the address policy at every hop,
//! TLS trust, `[http.client]`'s bounds and the proxy apply as they do to Novis code
//! (`rule:packaging/a-guest-has-no-ambient-authority`).
//!
//! **The host is checked by two parties.** `handle` answers `HTTP-request-denied` before anything
//! is sent when the URL's host is not in the instance's effective `connect` set, which is the
//! entry's grant, the manifest's request and the caller's own grant intersected
//! ([`crate::grants::effective`]), compared case-insensitively as `net.connect` compares. The host
//! behind [`Budget::send`] then asks the caller's `net.connect` again at every hop, so a redirect
//! stays inside the caller's grant (`rule:security/extension-grants-are-an-intersection`).
//!
//! Decisions ADR 0246 § 7 left to this module, under the priority ordering:
//!
//! - **Bodies cross whole, both ways.** A request is sent once its body is finished, or as soon as
//!   it is polled when the guest never asked for a body, and the response's body is read whole
//!   before the guest sees the response. While the body is still open, the response is not ready
//!   and its pollable is: a guest polling it in a loop without finishing the body spins until its
//!   CPU limit. A body dropped without `finish`, or finished with trailers, fails the request.
//! - **Every byte the host keeps for a guest is charged to its request** ([`Budget::charge`]) until
//!   the guest drops what holds it: header lists, the request's body and the response's body. A
//!   guest cannot grow host memory past its request's cap by cloning a header list in a loop.
//! - **The authority is strict.** It is a host of letters, digits, `-` and `.`, or a bracketed IPv6
//!   address, then an optional port: no user information, and nothing a second URL parser could
//!   read as another host. The scheme defaults to `https`, and the path to `/`.
//! - **The transport writes the framing.** `host`, `content-length`, `transfer-encoding`,
//!   `connection` and the other hop-by-hop headers ([`FRAMING`]) a guest sets are dropped when the
//!   request is sent. A header name is a token, and a value is UTF-8 without CR, LF or NUL.
//! - **`request-options` sets nothing.** Each setter returns an error, as the interface allows, and
//!   `[http.client]`'s one deadline covers the call.
//! - **A failure is logged.** The guest gets an `error-code` by the failure's class, and the
//!   message `Core\Http\Client` gave goes to the request's log at `warn` under the extension's
//!   channel.
//! - **A `headers()` getter returns an immutable copy**, not a child resource, so dropping the
//!   parent first is not an error.
//! - **No guest holds an `incoming-request` or a `response-outparam`**: nothing links
//!   `wasi:http/incoming-handler`, so those resources have no values and their methods are never
//!   reached.
//!
//! Cost: a call costs what `Core\Http\Client` costs, plus one copy of each body across the
//! boundary, charged to the request and freed when the guest drops it or the instance goes.

#![allow(
    unreachable_pub,
    reason = "`bindgen!` re-exports each resource type here as its `wasi:http/types` resource, which a type only the crate sees cannot be"
)]

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Poll, Waker};

use wasmtime::component::Resource;
use wasmtime_wasi_io::async_trait;
use wasmtime_wasi_io::bytes::Bytes;
use wasmtime_wasi_io::poll::{DynPollable, Pollable, subscribe};
use wasmtime_wasi_io::streams::{
    DynInputStream, DynOutputStream, InputStream, OutputStream, StreamError, StreamResult,
};

use super::bindings::wasi::http::{outgoing_handler, types};
use super::{Context, PERMIT};
use crate::call::{Budget, Incoming, Level, Outbound, Outgoing, Unsent};

use types::{ErrorCode, HeaderError, IncomingRequest, Method, ResponseOutparam, Scheme};

/// The headers the transport writes itself, dropped from a guest's request when it is sent.
pub(crate) const FRAMING: &[&str] = &[
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "proxy-connection",
    "te",
    "trailer",
    "upgrade",
];

/// Bytes the host keeps for a guest, charged to its request until this is dropped.
struct Charge {
    budget: Arc<dyn Budget>,
    bytes: i64,
}

impl Charge {
    fn new(budget: &Arc<dyn Budget>) -> Self {
        Self {
            budget: Arc::clone(budget),
            bytes: 0,
        }
    }

    /// Charges `bytes` more, or returns `false` and charges nothing when the request cannot
    /// afford them.
    fn add(&mut self, bytes: usize) -> bool {
        let Ok(bytes) = i64::try_from(bytes) else {
            return false;
        };
        if !self.budget.charge(bytes) {
            return false;
        }
        self.bytes += bytes;
        true
    }
}

impl Drop for Charge {
    fn drop(&mut self) {
        if self.bytes != 0 {
            self.budget.charge(-self.bytes);
        }
    }
}

/// A header list: names lower-case, values checked, in the order they were written.
pub struct Fields {
    entries: Vec<(String, Vec<u8>)>,
    mutable: bool,
    charge: Charge,
}

impl Fields {
    /// Appends `value` under `name`.
    fn append(&mut self, name: &str, value: Vec<u8>) -> wasmtime::Result<Result<(), HeaderError>> {
        if !self.mutable {
            return Ok(Err(HeaderError::Immutable));
        }
        if !token(name) || !field_value(&value) {
            return Ok(Err(HeaderError::InvalidSyntax));
        }
        self.charged(name.len() + value.len())?;
        self.entries.push((name.to_ascii_lowercase(), value));
        Ok(Ok(()))
    }

    /// Replaces every value under `name` with `values`.
    fn set(
        &mut self,
        name: &str,
        values: Vec<Vec<u8>>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        if !self.mutable {
            return Ok(Err(HeaderError::Immutable));
        }
        if !token(name) || !values.iter().all(|value| field_value(value)) {
            return Ok(Err(HeaderError::InvalidSyntax));
        }
        self.charged(values.iter().map(|value| name.len() + value.len()).sum())?;
        let name = name.to_ascii_lowercase();
        self.entries.retain(|(held, _)| *held != name);
        self.entries
            .extend(values.into_iter().map(|value| (name.clone(), value)));
        Ok(Ok(()))
    }

    fn charged(&mut self, bytes: usize) -> wasmtime::Result<()> {
        if self.charge.add(bytes) {
            Ok(())
        } else {
            Err(over_memory())
        }
    }
}

/// The trap a guest gets when what the host would keep for it is past its request's memory.
fn over_memory() -> wasmtime::Error {
    wasmtime::Error::msg("the request's memory limit is reached")
}

/// Whether `name` is an HTTP token.
fn token(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

/// Whether `value` is a header value the transport can write: UTF-8, with no CR, LF or NUL.
fn field_value(value: &[u8]) -> bool {
    std::str::from_utf8(value).is_ok() && !value.iter().any(|b| matches!(b, b'\r' | b'\n' | 0))
}

/// The host `authority` names, without brackets, or `None` when it is not strictly a host and an
/// optional port.
fn host(authority: &str) -> Option<&str> {
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (host, port) = rest.split_once(']')?;
        let ok = |b: u8| b.is_ascii_hexdigit() || b == b':' || b == b'.';
        (!host.is_empty() && host.bytes().all(ok)).then_some((host, port))?
    } else {
        let end = authority.find(':').unwrap_or(authority.len());
        let (host, port) = authority.split_at(end);
        let ok = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'.';
        (!host.is_empty() && host.bytes().all(ok)).then_some((host, port))?
    };
    if port.is_empty() {
        return Some(host);
    }
    let digits = port.strip_prefix(':')?;
    (!digits.is_empty() && digits.len() <= 5 && digits.bytes().all(|b| b.is_ascii_digit()))
        .then_some(host)
}

/// Whether `path` is an origin-form path and query: printable ASCII after a `/`, with no fragment.
fn path_with_query(path: &str) -> bool {
    path.starts_with('/') && path.bytes().all(|b| (0x21..0x7f).contains(&b) && b != b'#')
}

/// The name `method` is sent under.
fn method_name(method: &Method) -> &str {
    match method {
        Method::Get => "GET",
        Method::Head => "HEAD",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
        Method::Connect => "CONNECT",
        Method::Options => "OPTIONS",
        Method::Trace => "TRACE",
        Method::Patch => "PATCH",
        Method::Other(name) => name,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A request a guest is building.
pub struct OutgoingRequest {
    method: Method,
    scheme: Option<Scheme>,
    authority: Option<String>,
    path: Option<String>,
    headers: Fields,
    body: Option<Arc<Mutex<Body>>>,
}

/// A response a guest is building. Nothing sends one, because no guest is handed a
/// `response-outparam`.
pub struct OutgoingResponse {
    status: u16,
    headers: Fields,
    body: Option<Arc<Mutex<Body>>>,
}

/// The body a guest writes, shared by its `outgoing-body`, its stream and the response that waits
/// on it.
struct Body {
    bytes: Vec<u8>,
    state: Sending,
    charge: Charge,
}

/// Where a [`Body`] stands.
enum Sending {
    Open,
    Finished,
    Failed(ErrorCode),
}

/// `outgoing-body`: dropped without `finish`, it fails the request it belongs to.
pub struct OutgoingBody {
    body: Arc<Mutex<Body>>,
    written: bool,
}

impl Drop for OutgoingBody {
    fn drop(&mut self) {
        let mut body = lock(&self.body);
        if matches!(body.state, Sending::Open) {
            body.state = Sending::Failed(ErrorCode::InternalError(Some(
                "the request body was dropped before `finish`".to_owned(),
            )));
        }
    }
}

/// The output stream of an `outgoing-body`.
struct BodyWriter(Arc<Mutex<Body>>);

#[async_trait]
impl Pollable for BodyWriter {
    async fn ready(&mut self) {}
}

impl OutputStream for BodyWriter {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        let mut body = lock(&self.0);
        if !matches!(body.state, Sending::Open) {
            return Err(StreamError::Closed);
        }
        if !body.charge.add(bytes.len()) {
            return Err(StreamError::LastOperationFailed(over_memory()));
        }
        body.bytes.extend_from_slice(&bytes);
        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(PERMIT)
    }
}

/// `request-options`, which holds nothing.
pub struct RequestOptions;

/// The response to a request `handle` took.
pub struct FutureIncomingResponse {
    state: Response,
    body: Option<Arc<Mutex<Body>>>,
    budget: Arc<dyn Budget>,
    channel: Arc<str>,
}

/// Where a [`FutureIncomingResponse`] stands.
enum Response {
    /// Its body is not finished yet.
    Waiting(Outgoing),
    Sent(Outbound),
    /// The response, until the guest takes it.
    Done(Option<Result<IncomingResponse, ErrorCode>>),
}

impl FutureIncomingResponse {
    /// Sends the request once its body is finished.
    fn advance(&mut self) {
        if !matches!(self.state, Response::Waiting(_)) {
            return;
        }
        let bytes = match &self.body {
            None => None,
            Some(body) => {
                let mut body = lock(body);
                match &body.state {
                    Sending::Open => return,
                    Sending::Failed(code) => {
                        self.state = Response::Done(Some(Err(code.clone())));
                        return;
                    }
                    Sending::Finished => {
                        Some(std::mem::take(&mut body.bytes)).filter(|bytes| !bytes.is_empty())
                    }
                }
            }
        };
        if let Response::Waiting(mut request) =
            std::mem::replace(&mut self.state, Response::Done(None))
        {
            request.body = bytes;
            self.state = Response::Sent(self.budget.send(request));
        }
    }

    /// The response the guest is given for `result`.
    fn received(&self, result: Result<Incoming, Unsent>) -> Result<IncomingResponse, ErrorCode> {
        let (code, message) = match result {
            Ok(incoming) => {
                let mut charge = Charge::new(&self.budget);
                if !charge.add(incoming.body.len()) {
                    return Err(ErrorCode::HttpResponseBodySize(Some(
                        incoming.body.len() as u64
                    )));
                }
                return Ok(IncomingResponse {
                    status: incoming.status,
                    headers: incoming
                        .headers
                        .into_iter()
                        .map(|(name, value)| (name, value.into_bytes()))
                        .collect(),
                    body: Some(Held {
                        bytes: Bytes::from(incoming.body),
                        _charge: charge,
                    }),
                });
            }
            Err(Unsent::Timeout(message)) => (ErrorCode::ConnectionTimeout, message),
            Err(Unsent::Connection(message)) => (ErrorCode::ConnectionRefused, message),
            Err(Unsent::Refused(message)) => (ErrorCode::HttpRequestDenied, message),
        };
        self.budget.log(Level::Warn, &self.channel, &message);
        Err(code)
    }
}

#[async_trait]
impl Pollable for FutureIncomingResponse {
    async fn ready(&mut self) {
        self.advance();
        if let Response::Sent(outbound) = &mut self.state {
            let result = outbound.await;
            let response = self.received(result);
            self.state = Response::Done(Some(response));
        }
    }
}

/// A response, its body not consumed yet.
pub struct IncomingResponse {
    status: u16,
    headers: Vec<(String, Vec<u8>)>,
    body: Option<Held>,
}

/// A response body, charged to the request while the guest holds it.
struct Held {
    bytes: Bytes,
    _charge: Charge,
}

/// `incoming-body`, its stream not taken yet.
pub struct IncomingBody(Option<Held>);

/// The input stream of an `incoming-body`.
struct BodyReader(Held);

#[async_trait]
impl Pollable for BodyReader {
    async fn ready(&mut self) {}
}

impl InputStream for BodyReader {
    fn read(&mut self, size: usize) -> StreamResult<Bytes> {
        let bytes = &mut self.0.bytes;
        if bytes.is_empty() && size > 0 {
            return Err(StreamError::Closed);
        }
        Ok(bytes.split_to(size.min(bytes.len()).min(PERMIT)))
    }
}

/// `future-trailers`: a response has none, and that is known at once.
pub struct FutureTrailers {
    taken: bool,
}

#[async_trait]
impl Pollable for FutureTrailers {
    async fn ready(&mut self) {}
}

impl Context {
    /// A header list holding `entries`, charged to the request.
    fn fields(
        &mut self,
        entries: Vec<(String, Vec<u8>)>,
        mutable: bool,
    ) -> wasmtime::Result<Resource<Fields>> {
        let mut charge = Charge::new(&self.budget);
        if !charge.add(
            entries
                .iter()
                .map(|(name, value)| name.len() + value.len())
                .sum(),
        ) {
            return Err(over_memory());
        }
        Ok(self.table.push(Fields {
            entries,
            mutable,
            charge,
        })?)
    }

    /// A new body for a request or a response, or `None` when `taken` says it already has one.
    fn new_body(&self, taken: bool) -> Option<OutgoingBody> {
        if taken {
            return None;
        }
        Some(OutgoingBody {
            body: Arc::new(Mutex::new(Body {
                bytes: Vec::new(),
                state: Sending::Open,
                charge: Charge::new(&self.budget),
            })),
            written: false,
        })
    }

    /// The request `request` is sent as, or the code `handle` returns when it may not be sent.
    fn outgoing(&self, request: &OutgoingRequest) -> Result<Outgoing, ErrorCode> {
        let scheme = match &request.scheme {
            None | Some(Scheme::Https) => "https",
            Some(Scheme::Http) => "http",
            Some(Scheme::Other(_)) => return Err(ErrorCode::HttpRequestUriInvalid),
        };
        let authority = request
            .authority
            .as_deref()
            .ok_or(ErrorCode::HttpRequestUriInvalid)?;
        let host = host(authority).ok_or(ErrorCode::HttpRequestUriInvalid)?;
        if !self
            .connect
            .iter()
            .any(|granted| granted.eq_ignore_ascii_case(host))
        {
            return Err(ErrorCode::HttpRequestDenied);
        }
        let path = request.path.as_deref().unwrap_or("/");
        let headers = request
            .headers
            .entries
            .iter()
            .filter(|(name, _)| !FRAMING.contains(&name.as_str()))
            .map(|(name, value)| (name.clone(), String::from_utf8_lossy(value).into_owned()))
            .collect();
        Ok(Outgoing {
            method: method_name(&request.method).to_owned(),
            url: format!("{scheme}://{authority}{path}"),
            headers,
            body: None,
        })
    }
}

impl types::Host for Context {
    fn http_error_code(
        &mut self,
        err: Resource<wasmtime_wasi_io::streams::Error>,
    ) -> wasmtime::Result<Option<ErrorCode>> {
        self.table.get(&err)?;
        Ok(None)
    }
}

impl types::HostFields for Context {
    fn new(&mut self) -> wasmtime::Result<Resource<Fields>> {
        self.fields(Vec::new(), true)
    }

    fn from_list(
        &mut self,
        entries: Vec<(String, Vec<u8>)>,
    ) -> wasmtime::Result<Result<Resource<Fields>, HeaderError>> {
        if !entries
            .iter()
            .all(|(name, value)| token(name) && field_value(value))
        {
            return Ok(Err(HeaderError::InvalidSyntax));
        }
        let entries = entries
            .into_iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value))
            .collect();
        Ok(Ok(self.fields(entries, true)?))
    }

    fn get(&mut self, fields: Resource<Fields>, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
        let name = name.to_ascii_lowercase();
        Ok(self
            .table
            .get(&fields)?
            .entries
            .iter()
            .filter(|(held, _)| *held == name)
            .map(|(_, value)| value.clone())
            .collect())
    }

    fn has(&mut self, fields: Resource<Fields>, name: String) -> wasmtime::Result<bool> {
        let name = name.to_ascii_lowercase();
        Ok(self
            .table
            .get(&fields)?
            .entries
            .iter()
            .any(|(held, _)| *held == name))
    }

    fn set(
        &mut self,
        fields: Resource<Fields>,
        name: String,
        values: Vec<Vec<u8>>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        self.table.get_mut(&fields)?.set(&name, values)
    }

    fn delete(
        &mut self,
        fields: Resource<Fields>,
        name: String,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        self.table.get_mut(&fields)?.set(&name, Vec::new())
    }

    fn append(
        &mut self,
        fields: Resource<Fields>,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        self.table.get_mut(&fields)?.append(&name, value)
    }

    fn entries(&mut self, fields: Resource<Fields>) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        Ok(self.table.get(&fields)?.entries.clone())
    }

    fn clone(&mut self, fields: Resource<Fields>) -> wasmtime::Result<Resource<Fields>> {
        let entries = self.table.get(&fields)?.entries.clone();
        self.fields(entries, true)
    }

    fn drop(&mut self, fields: Resource<Fields>) -> wasmtime::Result<()> {
        self.table.delete(fields)?;
        Ok(())
    }
}

impl types::HostIncomingRequest for Context {
    fn method(&mut self, request: Resource<IncomingRequest>) -> wasmtime::Result<Method> {
        match *self.table.get(&request)? {}
    }

    fn path_with_query(
        &mut self,
        request: Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        match *self.table.get(&request)? {}
    }

    fn scheme(&mut self, request: Resource<IncomingRequest>) -> wasmtime::Result<Option<Scheme>> {
        match *self.table.get(&request)? {}
    }

    fn authority(
        &mut self,
        request: Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        match *self.table.get(&request)? {}
    }

    fn headers(
        &mut self,
        request: Resource<IncomingRequest>,
    ) -> wasmtime::Result<Resource<Fields>> {
        match *self.table.get(&request)? {}
    }

    fn consume(
        &mut self,
        request: Resource<IncomingRequest>,
    ) -> wasmtime::Result<Result<Resource<IncomingBody>, ()>> {
        match *self.table.get(&request)? {}
    }

    fn drop(&mut self, request: Resource<IncomingRequest>) -> wasmtime::Result<()> {
        match *self.table.get(&request)? {}
    }
}

impl types::HostOutgoingRequest for Context {
    fn new(&mut self, headers: Resource<Fields>) -> wasmtime::Result<Resource<OutgoingRequest>> {
        let headers = self.table.delete(headers)?;
        Ok(self.table.push(OutgoingRequest {
            method: Method::Get,
            scheme: None,
            authority: None,
            path: None,
            headers,
            body: None,
        })?)
    }

    fn body(
        &mut self,
        request: Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Result<Resource<OutgoingBody>, ()>> {
        let taken = self.table.get(&request)?.body.is_some();
        let Some(body) = self.new_body(taken) else {
            return Ok(Err(()));
        };
        self.table.get_mut(&request)?.body = Some(Arc::clone(&body.body));
        Ok(Ok(self.table.push(body)?))
    }

    fn method(&mut self, request: Resource<OutgoingRequest>) -> wasmtime::Result<Method> {
        Ok(self.table.get(&request)?.method.clone())
    }

    fn set_method(
        &mut self,
        request: Resource<OutgoingRequest>,
        method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        if let Method::Other(name) = &method
            && !token(name)
        {
            return Ok(Err(()));
        }
        self.table.get_mut(&request)?.method = method;
        Ok(Ok(()))
    }

    fn path_with_query(
        &mut self,
        request: Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        Ok(self.table.get(&request)?.path.clone())
    }

    fn set_path_with_query(
        &mut self,
        request: Resource<OutgoingRequest>,
        path: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        if path.as_deref().is_some_and(|path| !path_with_query(path)) {
            return Ok(Err(()));
        }
        self.table.get_mut(&request)?.path = path;
        Ok(Ok(()))
    }

    fn scheme(&mut self, request: Resource<OutgoingRequest>) -> wasmtime::Result<Option<Scheme>> {
        Ok(self.table.get(&request)?.scheme.clone())
    }

    fn set_scheme(
        &mut self,
        request: Resource<OutgoingRequest>,
        scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        if matches!(scheme, Some(Scheme::Other(_))) {
            return Ok(Err(()));
        }
        self.table.get_mut(&request)?.scheme = scheme;
        Ok(Ok(()))
    }

    fn authority(
        &mut self,
        request: Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        Ok(self.table.get(&request)?.authority.clone())
    }

    fn set_authority(
        &mut self,
        request: Resource<OutgoingRequest>,
        authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        if authority
            .as_deref()
            .is_some_and(|authority| host(authority).is_none())
        {
            return Ok(Err(()));
        }
        self.table.get_mut(&request)?.authority = authority;
        Ok(Ok(()))
    }

    fn headers(
        &mut self,
        request: Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Resource<Fields>> {
        let entries = self.table.get(&request)?.headers.entries.clone();
        self.fields(entries, false)
    }

    fn drop(&mut self, request: Resource<OutgoingRequest>) -> wasmtime::Result<()> {
        self.table.delete(request)?;
        Ok(())
    }
}

impl types::HostRequestOptions for Context {
    fn new(&mut self) -> wasmtime::Result<Resource<RequestOptions>> {
        Ok(self.table.push(RequestOptions)?)
    }

    fn connect_timeout(
        &mut self,
        options: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<u64>> {
        self.table.get(&options)?;
        Ok(None)
    }

    fn set_connect_timeout(
        &mut self,
        options: Resource<RequestOptions>,
        _duration: Option<u64>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table.get(&options)?;
        Ok(Err(()))
    }

    fn first_byte_timeout(
        &mut self,
        options: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<u64>> {
        self.table.get(&options)?;
        Ok(None)
    }

    fn set_first_byte_timeout(
        &mut self,
        options: Resource<RequestOptions>,
        _duration: Option<u64>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table.get(&options)?;
        Ok(Err(()))
    }

    fn between_bytes_timeout(
        &mut self,
        options: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<u64>> {
        self.table.get(&options)?;
        Ok(None)
    }

    fn set_between_bytes_timeout(
        &mut self,
        options: Resource<RequestOptions>,
        _duration: Option<u64>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.table.get(&options)?;
        Ok(Err(()))
    }

    fn drop(&mut self, options: Resource<RequestOptions>) -> wasmtime::Result<()> {
        self.table.delete(options)?;
        Ok(())
    }
}

impl types::HostResponseOutparam for Context {
    fn send_informational(
        &mut self,
        param: Resource<ResponseOutparam>,
        _status: u16,
        _headers: Resource<Fields>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        match *self.table.get(&param)? {}
    }

    fn set(
        &mut self,
        param: Resource<ResponseOutparam>,
        _response: Result<Resource<OutgoingResponse>, ErrorCode>,
    ) -> wasmtime::Result<()> {
        match *self.table.get(&param)? {}
    }

    fn drop(&mut self, param: Resource<ResponseOutparam>) -> wasmtime::Result<()> {
        match *self.table.get(&param)? {}
    }
}

impl types::HostIncomingResponse for Context {
    fn status(&mut self, response: Resource<IncomingResponse>) -> wasmtime::Result<u16> {
        Ok(self.table.get(&response)?.status)
    }

    fn headers(
        &mut self,
        response: Resource<IncomingResponse>,
    ) -> wasmtime::Result<Resource<Fields>> {
        let entries = self.table.get(&response)?.headers.clone();
        self.fields(entries, false)
    }

    fn consume(
        &mut self,
        response: Resource<IncomingResponse>,
    ) -> wasmtime::Result<Result<Resource<IncomingBody>, ()>> {
        let Some(body) = self.table.get_mut(&response)?.body.take() else {
            return Ok(Err(()));
        };
        Ok(Ok(self.table.push(IncomingBody(Some(body)))?))
    }

    fn drop(&mut self, response: Resource<IncomingResponse>) -> wasmtime::Result<()> {
        self.table.delete(response)?;
        Ok(())
    }
}

impl types::HostIncomingBody for Context {
    fn stream(
        &mut self,
        body: Resource<IncomingBody>,
    ) -> wasmtime::Result<Result<Resource<DynInputStream>, ()>> {
        let Some(held) = self.table.get_mut(&body)?.0.take() else {
            return Ok(Err(()));
        };
        let stream: DynInputStream = Box::new(BodyReader(held));
        Ok(Ok(self.table.push(stream)?))
    }

    fn finish(
        &mut self,
        body: Resource<IncomingBody>,
    ) -> wasmtime::Result<Resource<FutureTrailers>> {
        self.table.delete(body)?;
        Ok(self.table.push(FutureTrailers { taken: false })?)
    }

    fn drop(&mut self, body: Resource<IncomingBody>) -> wasmtime::Result<()> {
        self.table.delete(body)?;
        Ok(())
    }
}

impl types::HostFutureTrailers for Context {
    fn subscribe(
        &mut self,
        trailers: Resource<FutureTrailers>,
    ) -> wasmtime::Result<Resource<DynPollable>> {
        subscribe(&mut self.table, trailers)
    }

    fn get(
        &mut self,
        trailers: Resource<FutureTrailers>,
    ) -> wasmtime::Result<Option<Result<Result<Option<Resource<Fields>>, ErrorCode>, ()>>> {
        let trailers = self.table.get_mut(&trailers)?;
        if std::mem::replace(&mut trailers.taken, true) {
            return Ok(Some(Err(())));
        }
        Ok(Some(Ok(Ok(None))))
    }

    fn drop(&mut self, trailers: Resource<FutureTrailers>) -> wasmtime::Result<()> {
        self.table.delete(trailers)?;
        Ok(())
    }
}

impl types::HostOutgoingResponse for Context {
    fn new(&mut self, headers: Resource<Fields>) -> wasmtime::Result<Resource<OutgoingResponse>> {
        let headers = self.table.delete(headers)?;
        Ok(self.table.push(OutgoingResponse {
            status: 200,
            headers,
            body: None,
        })?)
    }

    fn status_code(&mut self, response: Resource<OutgoingResponse>) -> wasmtime::Result<u16> {
        Ok(self.table.get(&response)?.status)
    }

    fn set_status_code(
        &mut self,
        response: Resource<OutgoingResponse>,
        status: u16,
    ) -> wasmtime::Result<Result<(), ()>> {
        if !(100..=599).contains(&status) {
            return Ok(Err(()));
        }
        self.table.get_mut(&response)?.status = status;
        Ok(Ok(()))
    }

    fn headers(
        &mut self,
        response: Resource<OutgoingResponse>,
    ) -> wasmtime::Result<Resource<Fields>> {
        let entries = self.table.get(&response)?.headers.entries.clone();
        self.fields(entries, false)
    }

    fn body(
        &mut self,
        response: Resource<OutgoingResponse>,
    ) -> wasmtime::Result<Result<Resource<OutgoingBody>, ()>> {
        let taken = self.table.get(&response)?.body.is_some();
        let Some(body) = self.new_body(taken) else {
            return Ok(Err(()));
        };
        self.table.get_mut(&response)?.body = Some(Arc::clone(&body.body));
        Ok(Ok(self.table.push(body)?))
    }

    fn drop(&mut self, response: Resource<OutgoingResponse>) -> wasmtime::Result<()> {
        self.table.delete(response)?;
        Ok(())
    }
}

impl types::HostOutgoingBody for Context {
    fn write(
        &mut self,
        body: Resource<OutgoingBody>,
    ) -> wasmtime::Result<Result<Resource<DynOutputStream>, ()>> {
        let body = self.table.get_mut(&body)?;
        if std::mem::replace(&mut body.written, true) {
            return Ok(Err(()));
        }
        let stream: DynOutputStream = Box::new(BodyWriter(Arc::clone(&body.body)));
        Ok(Ok(self.table.push(stream)?))
    }

    fn finish(
        &mut self,
        body: Resource<OutgoingBody>,
        trailers: Option<Resource<Fields>>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let body = self.table.delete(body)?;
        let mut shared = lock(&body.body);
        if let Some(trailers) = trailers {
            self.table.delete(trailers)?;
            let code = ErrorCode::InternalError(Some("trailers are not sent".to_owned()));
            shared.state = Sending::Failed(code.clone());
            return Ok(Err(code));
        }
        if let Sending::Failed(code) = &shared.state {
            return Ok(Err(code.clone()));
        }
        shared.state = Sending::Finished;
        Ok(Ok(()))
    }

    fn drop(&mut self, body: Resource<OutgoingBody>) -> wasmtime::Result<()> {
        self.table.delete(body)?;
        Ok(())
    }
}

impl types::HostFutureIncomingResponse for Context {
    fn subscribe(
        &mut self,
        response: Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<Resource<DynPollable>> {
        subscribe(&mut self.table, response)
    }

    fn get(
        &mut self,
        response: Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<Option<Result<Result<Resource<IncomingResponse>, ErrorCode>, ()>>> {
        let future = self.table.get_mut(&response)?;
        future.advance();
        if let Response::Sent(outbound) = &mut future.state {
            let mut cx = std::task::Context::from_waker(Waker::noop());
            let Poll::Ready(result) = Pin::new(outbound).poll(&mut cx) else {
                return Ok(None);
            };
            future.state = Response::Done(Some(future.received(result)));
        }
        let Response::Done(done) = &mut future.state else {
            return Ok(None);
        };
        match done.take() {
            None => Ok(Some(Err(()))),
            Some(Err(code)) => Ok(Some(Ok(Err(code)))),
            Some(Ok(incoming)) => Ok(Some(Ok(Ok(self.table.push(incoming)?)))),
        }
    }

    fn drop(&mut self, response: Resource<FutureIncomingResponse>) -> wasmtime::Result<()> {
        self.table.delete(response)?;
        Ok(())
    }
}

impl outgoing_handler::Host for Context {
    fn handle(
        &mut self,
        request: Resource<OutgoingRequest>,
        options: Option<Resource<RequestOptions>>,
    ) -> wasmtime::Result<Result<Resource<FutureIncomingResponse>, ErrorCode>> {
        let request = self.table.delete(request)?;
        if let Some(options) = options {
            self.table.delete(options)?;
        }
        let outgoing = match self.outgoing(&request) {
            Ok(outgoing) => outgoing,
            Err(code) => return Ok(Err(code)),
        };
        Ok(Ok(self.table.push(FutureIncomingResponse {
            state: Response::Waiting(outgoing),
            body: request.body.clone(),
            budget: Arc::clone(&self.budget),
            channel: Arc::clone(&self.channel),
        })?))
    }
}
