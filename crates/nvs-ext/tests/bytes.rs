//! `rule:packaging/a-value-crosses-as-its-wit-type`: a third-party export's `bytes` crosses as the
//! host's `bytes` resource of `nvs:ext/types`, and a `Core` record's byte field stays `list<u8>`.
//!
//! The guest takes each `bytes` argument as a `borrow<bytes>`, reads it with `read` or `size`, and
//! drops the borrow before it returns. It returns a `bytes` as an `own<bytes>` it made with the
//! constructor and filled with `append`. Every export returns `result<T, error>` through memory at
//! 16: its case at 16, then the `T` at 20, or at 24 for a `u64`. `realloc` is a bump allocator from
//! 40960 that grows the memory, so a 16 MiB `read` fits.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

use nvs_ext::call::{Failure, Host, Limit, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Entry, Extension, Loader, Refused, pin};
use nvs_ext::pack::append_section;
use nvs_ext::section::MANIFEST;

const GUEST: &str = r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))
    (type $pk (record (field "spki" (list u8))))
    (export "public-key" (type (eq $pk)))
    (export "bytes" (type $bytes (sub resource)))
    (export "[constructor]bytes" (func (result (own $bytes))))
    (export "[method]bytes.size" (func (param "self" (borrow $bytes)) (result u64)))
    (export "[method]bytes.read" (func (param "self" (borrow $bytes)) (result (list u8))))
    (export "[method]bytes.append"
      (func (param "self" (borrow $bytes)) (param "data" (list u8))))))
  (alias export $types "error" (type $error))
  (alias export $types "public-key" (type $public-key))
  (alias export $types "bytes" (type $bytes))
  (type $labelled-def (record (field "label" string) (field "data" (own $bytes))))
  (export $labelled "labelled" (type $labelled-def))
  (core module $memory
    (memory (export "memory") 1)
    (global $bump (mut i32) (i32.const 40960))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $at i32) (local $end i32) (local $have i32)
      (local.set $at
        (i32.and
          (i32.add (global.get $bump) (i32.sub (local.get 2) (i32.const 1)))
          (i32.sub (i32.const 0) (local.get 2))))
      (local.set $end (i32.add (local.get $at) (local.get 3)))
      (local.set $have (i32.shl (memory.size) (i32.const 16)))
      (if (i32.gt_u (local.get $end) (local.get $have))
        (then
          (if (i32.eq
                (memory.grow
                  (i32.shr_u
                    (i32.add (i32.sub (local.get $end) (local.get $have)) (i32.const 65535))
                    (i32.const 16)))
                (i32.const -1))
            (then unreachable))))
      (global.set $bump (local.get $end))
      (local.get $at)))
  (core instance $mi (instantiate $memory))
  (alias core export $mi "memory" (core memory $mem))
  (alias core export $mi "realloc" (core func $realloc))
  (core func $new (canon lower (func $types "[constructor]bytes")))
  (core func $size (canon lower (func $types "[method]bytes.size")))
  (core func $read
    (canon lower (func $types "[method]bytes.read") (memory $mem) (realloc $realloc)))
  (core func $append (canon lower (func $types "[method]bytes.append") (memory $mem)))
  (core func $drop (canon resource.drop $bytes))
  (core module $m
    (import "host" "memory" (memory 1))
    (import "host" "new" (func $new (result i32)))
    (import "host" "size" (func $size (param i32) (result i64)))
    (import "host" "read" (func $read (param i32 i32)))
    (import "host" "append" (func $append (param i32 i32 i32)))
    (import "host" "drop" (func $drop (param i32)))
    (data (i32.const 512) "abcd")
    (func $own (param i32) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store (i32.const 20) (local.get 0))
      (i32.const 16))
    (func $copy (param $v i32) (result i32) (local $out i32)
      (call $read (local.get $v) (i32.const 64))
      (local.set $out (call $new))
      (call $append (local.get $out) (i32.load (i32.const 64)) (i32.load (i32.const 68)))
      (local.get $out))
    (func (export "echo") (param $v i32) (result i32) (local $out i32)
      (local.set $out (call $copy (local.get $v)))
      (call $drop (local.get $v))
      (call $own (local.get $out)))
    (func (export "parts") (result i32) (local $out i32)
      (local.set $out (call $new))
      (call $append (local.get $out) (i32.const 512) (i32.const 2))
      (call $append (local.get $out) (i32.const 514) (i32.const 2))
      (call $own (local.get $out)))
    (func (export "repeat") (param $times i64) (result i32) (local $out i32)
      (local.set $out (call $new))
      (block $done
        (loop $next
          (br_if $done (i64.eqz (local.get $times)))
          (call $append (local.get $out) (i32.const 4096) (i32.const 32768))
          (local.set $times (i64.sub (local.get $times) (i64.const 1)))
          (br $next)))
      (call $own (local.get $out)))
    (func (export "total") (param $at i32) (param $n i32) (result i32)
      (local $sum i64) (local $v i32)
      (block $done
        (loop $next
          (br_if $done (i32.eqz (local.get $n)))
          (local.set $v (i32.load (local.get $at)))
          (local.set $sum (i64.add (local.get $sum) (call $size (local.get $v))))
          (call $drop (local.get $v))
          (local.set $at (i32.add (local.get $at) (i32.const 4)))
          (local.set $n (i32.sub (local.get $n) (i32.const 1)))
          (br $next)))
      (i32.store8 (i32.const 16) (i32.const 0))
      (i64.store (i32.const 24) (local.get $sum))
      (i32.const 16))
    (func (export "labelled") (param $v i32) (result i32) (local $out i32)
      (local.set $out (call $copy (local.get $v)))
      (call $drop (local.get $v))
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store (i32.const 20) (i32.const 512))
      (i32.store (i32.const 24) (i32.const 4))
      (i32.store (i32.const 28) (local.get $out))
      (i32.const 16))
    (func (export "key") (param i32 i32) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store (i32.const 20) (local.get 0))
      (i32.store (i32.const 24) (local.get 1))
      (i32.const 16)))
  (core instance $i (instantiate $m
    (with "host" (instance
      (export "memory" (memory $mem))
      (export "new" (func $new))
      (export "size" (func $size))
      (export "read" (func $read))
      (export "append" (func $append))
      (export "drop" (func $drop))))))
  (func $echo (param "value" (borrow $bytes)) (result (result (own $bytes) (error $error)))
    (canon lift (core func $i "echo") (memory $mem) (realloc $realloc)))
  (func $parts (result (result (own $bytes) (error $error)))
    (canon lift (core func $i "parts") (memory $mem) (realloc $realloc)))
  (func $repeat (param "times" s64) (result (result (own $bytes) (error $error)))
    (canon lift (core func $i "repeat") (memory $mem) (realloc $realloc)))
  (func $total (param "values" (list (borrow $bytes))) (result (result s64 (error $error)))
    (canon lift (core func $i "total") (memory $mem) (realloc $realloc)))
  (func $labelled (param "value" (borrow $bytes)) (result (result $labelled (error $error)))
    (canon lift (core func $i "labelled") (memory $mem) (realloc $realloc)))
  (func $key (param "value" $public-key) (result (result $public-key (error $error)))
    (canon lift (core func $i "key") (memory $mem) (realloc $realloc)))
  (instance $api
    (export "echo" (func $echo))
    (export "parts" (func $parts))
    (export "repeat" (func $repeat))
    (export "total" (func $total))
    (export "labelled" (func $labelled))
    (export "key" (func $key)))
  (export "shop:files/api" (instance $api)))"#;

const GUEST_MANIFEST: &str = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Files", "interface": "shop:files/api",
  "methods": [
    {"name": "echo", "params": [{"name": "value", "type": "bytes"}], "returns": "bytes"},
    {"name": "parts", "returns": "bytes"},
    {"name": "repeat", "params": [{"name": "times", "type": "int"}], "returns": "bytes"},
    {"name": "total", "params": [{"name": "values", "type": "array<bytes>"}], "returns": "int"},
    {"name": "labelled", "params": [{"name": "value", "type": "bytes"}], "returns": "{label: string, data: bytes}"},
    {"name": "key", "params": [{"name": "value", "type": "Core\\Crypto\\PublicKey"}], "returns": "Core\\Crypto\\PublicKey"}]}"#;

/// A guest whose export takes and returns a `bytes` as `list<u8>`, as a built-in's does.
const LIST_GUEST: &str = r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
  (core module $m
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) (i32.const 1024))
    (func (export "echo") (param i32 i32) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store (i32.const 20) (local.get 0))
      (i32.store (i32.const 24) (local.get 1))
      (i32.const 16)))
  (core instance $i (instantiate $m))
  (alias core export $i "memory" (core memory $mem))
  (alias core export $i "realloc" (core func $realloc))
  (func $echo (param "value" (list u8)) (result (result (list u8) (error $error)))
    (canon lift (core func $i "echo") (memory $mem) (realloc $realloc)))
  (instance $api (export "echo" (func $echo)))
  (export "shop:files/api" (instance $api)))"#;

const LIST_MANIFEST: &str = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Files", "interface": "shop:files/api",
  "methods": [{"name": "echo", "params": [{"name": "value", "type": "bytes"}], "returns": "bytes"}]}"#;

fn host() -> Host {
    Host::new(4, |_| Ok(())).expect("the host starts")
}

/// The component `wat` under `manifest`, as the loader loads it or refuses it.
fn load(host: &Host, wat: &str, manifest: &str) -> Result<Extension, Refused> {
    let component = wat::parse_str(wat).expect("the test component compiles");
    let bytes = append_section(component, MANIFEST, manifest.as_bytes());
    let entry = Entry {
        path: PathBuf::from("files.nvsx"),
        sha256: pin(&bytes),
        memory: None,
        grants: nvs_config::extension::Granted::default(),
    };
    Loader::new(host.engine()).load_bytes(&entry, &bytes)
}

fn fixture() -> (Host, Extension) {
    let host = host();
    let extension = load(&host, GUEST, GUEST_MANIFEST).expect("the guest loads");
    (host, extension)
}

struct Unpark(std::thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Runs `call` to the end on this thread.
fn block_on<T>(call: impl Future<Output = T>) -> T {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut call = std::pin::pin!(call);
    loop {
        if let Poll::Ready(out) = call.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::park_timeout(Duration::from_millis(100));
    }
}

/// `method` called with `args` on a request under `meter`.
fn call(
    host: &Host,
    meter: &Arc<Meter>,
    extension: &Extension,
    method: &str,
    args: Vec<Value>,
) -> Result<Option<Value>, Failure> {
    let request: Request = host.request(Arc::clone(meter) as _);
    block_on(request.call_values(extension, method, args))
}

fn unlimited() -> Arc<Meter> {
    Arc::new(Meter::new(Duration::from_secs(30), None))
}

#[test]
fn a_bytes_argument_crosses_as_a_borrowed_resource() {
    let (host, extension) = fixture();
    let sent = Value::Bytes(b"a byte string".to_vec());
    let out = call(&host, &unlimited(), &extension, "echo", vec![sent.clone()]);
    assert_eq!(out.expect("the echo succeeds"), Some(sent));
    let empty = Value::Bytes(Vec::new());
    let out = call(&host, &unlimited(), &extension, "echo", vec![empty.clone()]);
    assert_eq!(out.expect("an empty echo succeeds"), Some(empty));
}

#[test]
fn a_bytes_result_is_built_in_parts() {
    let (host, extension) = fixture();
    let out = call(&host, &unlimited(), &extension, "parts", Vec::new());
    assert_eq!(
        out.expect("the call succeeds"),
        Some(Value::Bytes(b"abcd".to_vec()))
    );
    let out = call(
        &host,
        &unlimited(),
        &extension,
        "repeat",
        vec![Value::Int(3)],
    );
    match out.expect("the call succeeds") {
        Some(Value::Bytes(bytes)) => assert_eq!(bytes.len(), 3 * 32768),
        other => panic!("a byte string was expected, not {other:?}"),
    }
}

#[test]
fn a_bytes_inside_a_list_or_shape_crosses_as_a_resource() {
    let (host, extension) = fixture();
    let list = Value::Array(vec![
        (Key::Int(0), Value::Bytes(vec![1; 3])),
        (Key::Int(1), Value::Bytes(Vec::new())),
        (Key::Int(2), Value::Bytes(vec![2; 5])),
    ]);
    let out = call(&host, &unlimited(), &extension, "total", vec![list]);
    assert_eq!(out.expect("the call succeeds"), Some(Value::Int(8)));
    let out = call(
        &host,
        &unlimited(),
        &extension,
        "labelled",
        vec![Value::Bytes(b"payload".to_vec())],
    );
    assert_eq!(
        out.expect("the call succeeds"),
        Some(Value::Array(vec![
            (
                Key::String("label".to_owned()),
                Value::String("abcd".to_owned())
            ),
            (
                Key::String("data".to_owned()),
                Value::Bytes(b"payload".to_vec())
            ),
        ]))
    );
}

#[test]
fn a_core_record_keeps_its_byte_list() {
    let (host, extension) = fixture();
    let key = Value::Core {
        class: "Core\\Crypto\\PublicKey".to_owned(),
        fields: vec![(
            "spki".to_owned(),
            Value::Bytes(vec![0x30, 0x2a, 0x30, 0x05]),
        )],
    };
    let out = call(&host, &unlimited(), &extension, "key", vec![key.clone()]);
    assert_eq!(out.expect("the call succeeds"), Some(key));
}

#[test]
fn a_result_past_the_budget_stops_at_the_memory_limit() {
    let (host, extension) = fixture();
    let meter = Arc::new(Meter::new(Duration::from_secs(30), Some(1 << 20)));
    // 64 parts of 32 KiB are 2 MiB, twice what the request may hold.
    let out = call(&host, &meter, &extension, "repeat", vec![Value::Int(64)]);
    assert!(
        matches!(out, Err(Failure::Limit(Limit::Memory))),
        "the call should stop at the memory limit, and it returned {out:?}"
    );
    assert_eq!(meter.charged(), 0, "every charge is given back");
}

/// One `Val` per byte would charge at least 32 bytes per byte, so a 16 MiB echo would need over
/// 512 MiB. Each byte is copied once each way: into the guest's memory by `read`, and into the
/// result's buffer by `append`. So the echo fits in 40 MiB.
#[test]
fn a_16_mib_echo_copies_each_byte_once_each_way() {
    const SIZE: usize = 16 << 20;
    let (host, extension) = fixture();
    let meter = Arc::new(Meter::new(Duration::from_secs(60), Some(40 << 20)));
    let sent: Vec<u8> = (0..SIZE).map(|at| at.to_le_bytes()[0]).collect();
    let out = call(
        &host,
        &meter,
        &extension,
        "echo",
        vec![Value::Bytes(sent.clone())],
    );
    match out.expect("the echo fits in the budget") {
        Some(Value::Bytes(bytes)) => assert!(bytes == sent, "the echo returns the same bytes"),
        other => panic!("a byte string was expected, not {other:?}"),
    }
}

#[test]
fn a_list_of_u8_in_a_third_party_signature_does_not_load() {
    let host = host();
    let Err(err) = load(&host, LIST_GUEST, LIST_MANIFEST) else {
        panic!("a `bytes` as `list<u8>` is refused");
    };
    assert!(
        err.reason.contains("`borrow<bytes>` of `nvs:ext/types`"),
        "the refusal names the resource: {}",
        err.reason
    );
}
