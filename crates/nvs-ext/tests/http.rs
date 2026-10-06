//! `rule:security/extension-grants-are-an-intersection`'s outbound half, through a running guest: a
//! guest's `wasi:http` request reaches a host only when its entry, its manifest and its caller all
//! grant it, and it is sent by `Core\Http\Client`'s transport, so the address policy, TLS trust and
//! the configured proxy apply to it as they do to Novis code
//! (`rule:packaging/a-guest-has-no-ambient-authority`).
//!
//! The guest is a core module written against the canonical ABI, made a component by
//! `wit-component`, so no wasm toolchain is needed. The budget queues each request the guest sends,
//! and the loop that polls the call sends it through `nvs_stdlib::extension_send` with a context
//! built from an `nvs.toml` text, as `nvs serve` does between two polls. Every origin and proxy is
//! a loopback listener on a thread of its own.

use std::future::Future;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

use nvs_config::extension::Granted;
use nvs_ext::call::{Answer, Budget, Host, Incoming, Level, Outbound, Outgoing, Unsent};
use nvs_ext::grants::Caller;
use nvs_ext::load::{Extension, pin as pin_of};
use nvs_ext::manifest::Manifest;
use nvs_ext::source::{FORMAT, Source};
use nvs_runtime::{Ctx, Fault, ThrownClass};
use wasmtime::component::{Component, Val};
use wit_component::{ComponentEncoder, StringEncoding, embed_component_metadata};
use wit_parser::Resolve;

const WIT: &str = "package shop:web;

interface api {
    get: func(https: bool, authority: string) -> s64;
}

world guest {
    import wasi:io/poll@0.2.12;
    import wasi:http/types@0.2.12;
    import wasi:http/outgoing-handler@0.2.12;
    export api;
}
";

/// The guest's core module. `get` sends a `GET` for `/` at `authority`, over `https` or `http`,
/// waits on the response's pollable and returns its status. It returns `-1 - code` when `handle`
/// fails and `-101 - code` when the response does, `code` being the `error-code` case's index,
/// and `-1000` and below for a step the host should never fail.
const CORE: &str = r#"(module
  (import "wasi:io/poll@0.2.12" "poll" (func $poll (param i32 i32 i32)))
  (import "wasi:http/types@0.2.12" "[constructor]fields" (func $fields (result i32)))
  (import "wasi:http/types@0.2.12" "[constructor]outgoing-request"
    (func $request (param i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]outgoing-request.set-scheme"
    (func $scheme (param i32 i32 i32 i32 i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]outgoing-request.set-authority"
    (func $authority (param i32 i32 i32 i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]future-incoming-response.subscribe"
    (func $subscribe (param i32) (result i32)))
  (import "wasi:http/types@0.2.12" "[method]future-incoming-response.get"
    (func $get (param i32 i32)))
  (import "wasi:http/types@0.2.12" "[method]incoming-response.status"
    (func $status (param i32) (result i32)))
  (import "wasi:http/outgoing-handler@0.2.12" "handle"
    (func $handle (param i32 i32 i32 i32)))
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 4096))
  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $heap) (i32.sub (local.get 2) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get 2))))
    (global.set $heap (i32.add (local.get $at) (local.get 3)))
    (local.get $at))
  (func (export "shop:web/api#get") (param $https i32) (param $ptr i32) (param $len i32)
    (result i64)
    (local $req i32) (local $future i32)
    (local.set $req (call $request (call $fields)))
    (if (call $scheme (local.get $req) (i32.const 1) (local.get $https) (i32.const 0)
          (i32.const 0))
      (then (return (i64.const -1000))))
    (if (call $authority (local.get $req) (i32.const 1) (local.get $ptr) (local.get $len))
      (then (return (i64.const -1001))))
    (call $handle (local.get $req) (i32.const 0) (i32.const 0) (i32.const 32))
    (if (i32.load8_u (i32.const 32))
      (then (return (i64.sub (i64.const -1) (i64.extend_i32_u (i32.load8_u (i32.const 40)))))))
    (local.set $future (i32.load (i32.const 40)))
    (i32.store (i32.const 64) (call $subscribe (local.get $future)))
    (call $poll (i32.const 64) (i32.const 1) (i32.const 72))
    (call $get (local.get $future) (i32.const 128))
    (if (i32.eqz (i32.load8_u (i32.const 128))) (then (return (i64.const -1002))))
    (if (i32.load8_u (i32.const 136)) (then (return (i64.const -1003))))
    (if (i32.load8_u (i32.const 144))
      (then (return (i64.sub (i64.const -101) (i64.extend_i32_u (i32.load8_u (i32.const 152)))))))
    (i64.extend_i32_u (call $status (i32.load (i32.const 152))))))"#;

/// What `get` returns when `handle` answers `HTTP-request-denied`, the 16th case of `error-code`.
const DENIED_AT_HANDLE: i64 = -16;
/// What it returns when the response is `HTTP-request-denied`.
const DENIED: i64 = -116;

/// The guest, componentized against its world.
fn guest_bytes() -> Vec<u8> {
    let mut resolve = Resolve::default();
    resolve
        .push_dir(nvs_repo::path("wit/nvs-ext"))
        .expect("the world's WIT parses");
    let package = resolve
        .push_str("guest.wit", WIT)
        .expect("the guest's world parses");
    let world = resolve
        .select_world(&[package], Some("guest"))
        .expect("the guest's world exists");
    let mut module = wat::parse_str(CORE).expect("the guest's core module compiles");
    embed_component_metadata(&mut module, &resolve, world, StringEncoding::UTF8)
        .expect("the guest's metadata embeds");
    ComponentEncoder::default()
        .module(&module)
        .expect("the guest's core module reads")
        .validate(true)
        .encode()
        .expect("the guest componentizes")
}

/// A host with a slot for each of a case's requests, and the guest on it, its entry granting
/// `entry` and its manifest requesting `requests`.
fn host_with(entry: &[&str], requests: &[&str]) -> (Host, Extension) {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let bytes = guest_bytes();
    let manifest = format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "Shop\\Web", "interface": "shop:web/api",
        "requests": {{"connect": {}}},
        "methods": [
          {{"name": "get", "params": [{{"name": "https", "type": "bool"}},
            {{"name": "authority", "type": "string"}}], "returns": "int"}}
        ]}}"#,
        serde_json::to_string(requests).expect("a list of hosts is JSON")
    );
    let extension = Extension {
        path: PathBuf::from("web.nvsx"),
        sha256: pin_of(&bytes),
        memory: None,
        grants: Granted {
            connect: entry.iter().map(|&host| host.to_owned()).collect(),
            ..Granted::default()
        },
        component: Component::new(host.engine(), &bytes).expect("the guest compiles"),
        manifest: Manifest::parse(manifest.as_bytes()).expect("the manifest reads"),
        source: Source {
            source: FORMAT,
            files: Vec::new(),
        },
    };
    (host, extension)
}

/// A request budget with no limits, whose caller holds the `[capabilities]` of `snapshot`, and
/// which queues each outbound request for the loop that polls the call, as `nvs serve`'s does.
/// It keeps every line logged to it.
struct Sending {
    snapshot: Arc<nvs_config::Snapshot>,
    queued: Mutex<Vec<(Outgoing, Answer)>>,
    sent: AtomicUsize,
    lines: Mutex<Vec<String>>,
}

impl Sending {
    fn new(snapshot: Arc<nvs_config::Snapshot>) -> Arc<Self> {
        Arc::new(Self {
            snapshot,
            queued: Mutex::new(Vec::new()),
            sent: AtomicUsize::new(0),
            lines: Mutex::new(Vec::new()),
        })
    }

    /// The lines logged so far, joined.
    fn logged(&self) -> String {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .join("\n")
    }

    fn take(&self) -> Vec<(Outgoing, Answer)> {
        std::mem::take(&mut *self.queued.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// How many requests the guest has handed the host to send.
    fn sent(&self) -> usize {
        self.sent.load(Ordering::Relaxed)
    }
}

impl Budget for Sending {
    fn cpu_spent(&self) -> bool {
        false
    }

    fn charge(&self, _bytes: i64) -> bool {
        true
    }

    fn log(&self, _level: Level, _channel: &str, message: &str) {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(message.to_owned());
    }

    fn wall_clock(&self) -> Duration {
        Duration::ZERO
    }

    fn monotonic_clock(&self) -> u64 {
        0
    }

    fn random(&self, out: &mut [u8]) {
        out.fill(0);
    }

    fn setting(&self, _block: &str, _key: &str) -> Option<serde_json::Value> {
        None
    }

    fn caller(&self) -> Caller<'_> {
        Caller {
            capabilities: self.snapshot.config.capabilities.as_ref(),
            narrowed: None,
        }
    }

    fn send(&self, request: Outgoing) -> Outbound {
        self.sent.fetch_add(1, Ordering::Relaxed);
        let (outbound, answer) = Outbound::pair();
        self.queued
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((request, answer));
        outbound
    }
}

/// The snapshot an `nvs.toml` holding `written` gives a request.
fn snapshot(written: &str) -> Arc<nvs_config::Snapshot> {
    let table: toml::Table = written.parse().expect("the case writes valid TOML");
    let config: nvs_config::Config = table
        .clone()
        .try_into()
        .expect("the case writes a block this tree has");
    Arc::new(nvs_config::Snapshot {
        config,
        table,
        ..nvs_config::Snapshot::default()
    })
}

/// `request` sent through `Core\Http\Client`'s transport under `ctx`, as `nvs serve` sends it.
fn sent(ctx: &mut Ctx, request: Outgoing) -> Result<Incoming, Unsent> {
    let Outgoing {
        method,
        url,
        headers,
        body,
    } = request;
    match nvs_stdlib::extension_send(ctx, &method, &url, headers, body) {
        Ok(reply) => Ok(Incoming {
            status: u16::try_from(reply.status).unwrap_or(u16::MAX),
            headers: reply.headers,
            body: reply.body,
        }),
        Err(
            Fault::Thrown(ThrownClass::Timeout, message)
            | Fault::ThrownWithSlots(ThrownClass::Timeout, message, _),
        ) => Err(Unsent::Timeout(message.into_owned())),
        Err(
            Fault::Thrown(ThrownClass::Io, message)
            | Fault::ThrownWithSlots(ThrownClass::Io, message, _),
        ) => Err(Unsent::Connection(message.into_owned())),
        Err(
            Fault::Thrown(_, message)
            | Fault::ThrownWithSlots(_, message, _)
            | Fault::Fatal(message),
        ) => Err(Unsent::Refused(message.into_owned())),
        Err(other) => Err(Unsent::Refused(format!("{other:?}"))),
    }
}

/// What the guest's `get` returns for `authority`, its requests sent under `nvs.toml` text
/// `written` between two polls of the call.
fn get(
    host: &Host,
    extension: &Extension,
    written: &str,
    https: bool,
    authority: &str,
) -> (i64, Arc<Sending>) {
    let snapshot = snapshot(written);
    let budget = Sending::new(Arc::clone(&snapshot));
    let request = host.request(Arc::clone(&budget) as Arc<dyn Budget>);
    let mut ctx = Ctx::buffered();
    ctx.set_config(snapshot);
    let args = [Val::Bool(https), Val::String(authority.to_owned())];
    let mut call = pin!(request.call(extension, "get", &args));
    let mut cx = Context::from_waker(Waker::noop());
    let out = loop {
        if let Poll::Ready(out) = call.as_mut().poll(&mut cx) {
            break out.expect("the call returns");
        }
        for (outgoing, answer) in budget.take() {
            answer.fill(sent(&mut ctx, outgoing));
        }
    };
    (returned(&out), budget)
}

fn returned(out: &[Val]) -> i64 {
    match out {
        [Val::S64(value)] => *value,
        other => panic!("`get` returned {other:?}"),
    }
}

/// Polls `future` on this thread until it is ready.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
    }
}

/// How long a loopback peer waits on a client before it gives up.
const BOUND: Duration = Duration::from_secs(10);

/// A loopback listener that answers every connection with `handle` on a thread of its own, and
/// counts the connections it accepts.
fn listening(handle: fn(TcpStream)) -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is free");
    let at = listener
        .local_addr()
        .expect("a bound socket has an address");
    let accepted = Arc::new(AtomicUsize::new(0));
    let counting = Arc::clone(&accepted);
    thread::spawn(move || {
        for stream in listener.incoming().map_while(Result::ok) {
            counting.fetch_add(1, Ordering::Relaxed);
            thread::spawn(move || handle(stream));
        }
    });
    (at, accepted)
}

/// Reads one request head from `stream`, or `None` when it ends first.
fn head(stream: &mut impl Read) -> Option<String> {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return None,
        }
    }
    String::from_utf8(head).ok()
}

const OK: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";

/// An origin that answers one request with `200`.
fn origin(mut stream: TcpStream) {
    drop(stream.set_read_timeout(Some(BOUND)));
    if head(&mut stream).is_some() {
        drop(stream.write_all(OK));
    }
}

/// A forward proxy that answers a `CONNECT` with `200`, then answers the request in the tunnel
/// itself with `200` and a header naming the destination the `CONNECT` asked for.
fn proxy(mut stream: TcpStream) {
    drop(stream.set_read_timeout(Some(BOUND)));
    let Some(connect) = head(&mut stream) else {
        return;
    };
    let Some(target) = connect.strip_prefix("CONNECT ") else {
        return;
    };
    let target = target.split(' ').next().unwrap_or_default().to_owned();
    if stream
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .is_err()
        || head(&mut stream).is_none()
    {
        return;
    }
    let status = if target == "origin.example.com:80" {
        200
    } else {
        502
    };
    drop(
        stream.write_all(
            format!(
                "HTTP/1.1 {status} Tunnelled\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        ),
    );
}

/// An `https` origin whose certificate is self-signed, which no root the process trusts signs.
fn untrusted(stream: TcpStream) {
    let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()])
        .expect("the certificate is issued");
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
        issued.signing_key.serialize_der(),
    ));
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("the provider speaks the default versions")
    .with_no_client_auth()
    .with_single_cert(vec![issued.cert.der().clone()], key)
    .expect("the certificate and its key match");
    let Ok(conn) = rustls::ServerConnection::new(Arc::new(config)) else {
        return;
    };
    drop(stream.set_read_timeout(Some(BOUND)));
    let mut tls = rustls::StreamOwned::new(conn, stream);
    if head(&mut tls).is_some() {
        drop(tls.write_all(OK));
    }
}

/// A caller granted `connect`, with `127.0.0.1` excepted from the address policy.
fn granted_loopback(connect: &str) -> String {
    format!("[capabilities.net]\nconnect = [\"{connect}\"]\ninternal = [\"127.0.0.1\"]\n")
}

const LOOPBACK: &[&str] = &["127.0.0.1"];

// covers: tools:config/extension-grants-what-a-component-may-reach
#[test]
fn a_guest_http_call_to_a_host_all_three_grant_succeeds() {
    let (at, accepted) = listening(origin);
    let (host, extension) = host_with(LOOPBACK, LOOPBACK);
    let (status, budget) = get(
        &host,
        &extension,
        &granted_loopback("127.0.0.1"),
        false,
        &at.to_string(),
    );
    assert_eq!(status, 200);
    assert_eq!(budget.sent(), 1);
    assert_eq!(accepted.load(Ordering::Relaxed), 1);
}

#[test]
fn a_guest_http_call_to_a_host_the_entry_does_not_grant_fails() {
    let (at, accepted) = listening(origin);
    let (host, extension) = host_with(&["tiles.example.com"], LOOPBACK);
    let (status, budget) = get(
        &host,
        &extension,
        &granted_loopback("127.0.0.1"),
        false,
        &at.to_string(),
    );
    assert_eq!(status, DENIED_AT_HANDLE);
    assert_eq!(budget.sent(), 0);
    assert_eq!(accepted.load(Ordering::Relaxed), 0);
}

#[test]
fn a_guest_http_call_to_a_host_the_manifest_does_not_request_fails() {
    let (at, accepted) = listening(origin);
    let (host, extension) = host_with(LOOPBACK, &["tiles.example.com"]);
    let (status, budget) = get(
        &host,
        &extension,
        &granted_loopback("127.0.0.1"),
        false,
        &at.to_string(),
    );
    assert_eq!(status, DENIED_AT_HANDLE);
    assert_eq!(budget.sent(), 0);
    assert_eq!(accepted.load(Ordering::Relaxed), 0);
}

#[test]
fn a_guest_http_call_to_a_host_the_caller_does_not_hold_fails() {
    let (at, accepted) = listening(origin);
    let (host, extension) = host_with(LOOPBACK, LOOPBACK);
    let (status, budget) = get(
        &host,
        &extension,
        &granted_loopback("tiles.example.com"),
        false,
        &at.to_string(),
    );
    assert_eq!(status, DENIED_AT_HANDLE);
    assert_eq!(budget.sent(), 0);
    assert_eq!(accepted.load(Ordering::Relaxed), 0);
}

/// All three parties name the host, but the caller's grant writes no exception for the loopback
/// address, so the client refuses the address it resolves to and connects to nothing.
#[test]
fn a_guest_http_call_to_a_private_address_fails_under_the_address_policy() {
    let (at, accepted) = listening(origin);
    let (host, extension) = host_with(LOOPBACK, LOOPBACK);
    let (status, budget) = get(
        &host,
        &extension,
        "[capabilities.net]\nconnect = [\"127.0.0.1\"]\n",
        false,
        &at.to_string(),
    );
    assert_eq!(status, DENIED);
    assert_eq!(budget.sent(), 1);
    assert_eq!(accepted.load(Ordering::Relaxed), 0);
    assert!(budget.logged().contains("loopback"), "{}", budget.logged());
}

/// The proxy answers `200` only to a tunnel to the destination the guest named, so the status
/// says the request went through it.
#[test]
fn a_guest_http_call_goes_through_the_configured_proxy() {
    let (at, accepted) = listening(proxy);
    let destination = &["origin.example.com"];
    let (host, extension) = host_with(destination, destination);
    let written = format!(
        "[capabilities.net]\nconnect = [\"origin.example.com\"]\n\n\
         [http.client.proxy]\nurl = \"http://{at}\"\nresolve = \"proxy\"\n"
    );
    let (status, _) = get(&host, &extension, &written, false, "origin.example.com");
    assert_eq!(status, 200);
    assert_eq!(accepted.load(Ordering::Relaxed), 1);
}

/// TLS is verified strictly, and nothing the process trusts signs the origin's certificate.
#[test]
fn a_guest_https_call_to_an_untrusted_certificate_fails() {
    let (at, accepted) = listening(untrusted);
    let (host, extension) = host_with(LOOPBACK, LOOPBACK);
    let (status, budget) = get(
        &host,
        &extension,
        &granted_loopback("127.0.0.1"),
        true,
        &at.to_string(),
    );
    assert_eq!(status, DENIED);
    assert_eq!(budget.sent(), 1);
    assert_eq!(accepted.load(Ordering::Relaxed), 1);
    assert!(
        budget.logged().contains("invalid peer certificate"),
        "{}",
        budget.logged()
    );
}

/// While one call waits on its response, a second call on the same thread runs to its end. The
/// first call then returns the status it is given.
#[test]
fn a_guest_waiting_on_an_http_response_lets_another_task_on_its_core_run() {
    let (host, extension) = host_with(LOOPBACK, LOOPBACK);
    let budget = Sending::new(snapshot(&granted_loopback("127.0.0.1")));
    let waiting = host.request(Arc::clone(&budget) as Arc<dyn Budget>);
    let args = [Val::Bool(false), Val::String("127.0.0.1:9".to_owned())];
    let mut call = pin!(waiting.call(&extension, "get", &args));
    let mut cx = Context::from_waker(Waker::noop());
    let mut queued = Vec::new();
    for _ in 0..10_000 {
        assert!(call.as_mut().poll(&mut cx).is_pending());
        queued = budget.take();
        if !queued.is_empty() {
            break;
        }
    }
    let [(outgoing, answer)]: [(Outgoing, Answer); 1] = queued
        .try_into()
        .expect("the guest sends one request and waits");
    assert_eq!(outgoing.url, "http://127.0.0.1:9/");
    for _ in 0..100 {
        assert!(call.as_mut().poll(&mut cx).is_pending());
    }

    let other_budget = Sending::new(snapshot(""));
    let other = host.request(Arc::clone(&other_budget) as Arc<dyn Budget>);
    let out = block_on(other.call(&extension, "get", &args)).expect("the other call returns");
    assert_eq!(returned(&out), DENIED_AT_HANDLE);
    assert!(call.as_mut().poll(&mut cx).is_pending());

    answer.fill(Ok(Incoming {
        status: 204,
        headers: Vec::new(),
        body: Vec::new(),
    }));
    let out = block_on(call).expect("the waiting call returns");
    assert_eq!(returned(&out), 204);
}
