//! `rule:packaging/a-guest-crash-throws` and `rule:packaging/a-guest-runs-under-the-requests-budget`:
//! an export's `err` throws the class the world maps it to, a trap throws `ExtensionError` and the
//! next call gets a fresh instance, and a CPU or memory limit is a resource-limit `FATAL`.
//!
//! What the program sees is [`Failure::outcome`]. The runtime is linked here only to pin the class
//! and limit names that outcome carries to the spellings the host raises them with.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

use nvs_ext::call::{Class, Crash, Error, Failure, Host, Limit, Meter, Outcome, Request};
use nvs_ext::load::{Extension, pin};
use nvs_ext::manifest::Manifest;
use nvs_ext::source::{FORMAT, Source};
use nvs_runtime::ThrownClass;
use wasmtime::component::{Component, Val};

const PAGE: u64 = 65536;

/// A component exporting `shop:orders/api`, its `error` imported from `nvs:ext/types`. `invalid`,
/// `parse` and `runtime` return that `err` case; `name` returns a string that is not UTF-8;
/// `crash` reaches `unreachable`; `bump` adds one to a global and returns it; `forever` loops;
/// `grow` grows the memory by its argument in pages.
///
/// A `result<s64, error>` is returned through memory: its case at 0, then at 8 either the `s64` or
/// the `error`, whose case is at 8 and whose string is at 12 and 16.
const GUEST: &str = r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
  (core module $m
    (memory (export "memory") 1)
    (global $n (mut i64) (i64.const 0))
    (data (i32.const 0x100) "\01\00\00\00\00\00\00\00\00\00\00\00\00\02\00\00\08\00\00\00")
    (data (i32.const 0x120) "\01\00\00\00\00\00\00\00\01\00\00\00\10\02\00\00\0a\00\00\00")
    (data (i32.const 0x140) "\01\00\00\00\00\00\00\00\02\00\00\00\20\02\00\00\06\00\00\00")
    (data (i32.const 0x180) "\00\00\00\00\30\02\00\00\02\00\00\00")
    (data (i32.const 0x200) "no lines")
    (data (i32.const 0x210) "not a date")
    (data (i32.const 0x220) "closed")
    (data (i32.const 0x230) "\ff\fe")
    (func (export "invalid") (result i32) i32.const 0x100)
    (func (export "parse") (result i32) i32.const 0x120)
    (func (export "runtime") (result i32) i32.const 0x140)
    (func (export "name") (result i32) i32.const 0x180)
    (func (export "crash") (result i32) unreachable)
    (func (export "bump") (result i32)
      (global.set $n (i64.add (global.get $n) (i64.const 1)))
      (i64.store (i32.const 0x1a8) (global.get $n))
      i32.const 0x1a0)
    (func (export "forever") (result i32)
      (loop $l (br $l))
      unreachable)
    (func (export "grow") (param i32) (result i32)
      (drop (memory.grow (local.get 0)))
      i32.const 0x1a0))
  (core instance $i (instantiate $m))
  (alias core export $i "memory" (core memory $mem))
  (type $r (result s64 (error $error)))
  (func $invalid (result $r) (canon lift (core func $i "invalid") (memory $mem)))
  (func $parse (result $r) (canon lift (core func $i "parse") (memory $mem)))
  (func $runtime (result $r) (canon lift (core func $i "runtime") (memory $mem)))
  (func $name (result (result string (error $error)))
    (canon lift (core func $i "name") (memory $mem)))
  (func $crash (result $r) (canon lift (core func $i "crash") (memory $mem)))
  (func $bump (result $r) (canon lift (core func $i "bump") (memory $mem)))
  (func $forever (result $r) (canon lift (core func $i "forever") (memory $mem)))
  (func $grow (param "pages" u32) (result $r) (canon lift (core func $i "grow") (memory $mem)))
  (instance $api
    (export "invalid" (func $invalid))
    (export "parse" (func $parse))
    (export "runtime" (func $runtime))
    (export "name" (func $name))
    (export "crash" (func $crash))
    (export "bump" (func $bump))
    (export "forever" (func $forever))
    (export "grow" (func $grow)))
  (export "shop:orders/api" (instance $api)))"#;

const MANIFEST: &str = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Orders", "interface": "shop:orders/api", "methods": [
  {"name": "invalid", "returns": "int"},
  {"name": "parse", "returns": "int"},
  {"name": "runtime", "returns": "int"},
  {"name": "name", "returns": "string"},
  {"name": "crash", "returns": "int"},
  {"name": "bump", "returns": "int"},
  {"name": "forever", "returns": "int"},
  {"name": "grow", "params": [{"name": "pages", "type": "int"}], "returns": "int"}
]}"#;

/// A host linking `nvs:ext/types`, which offers types and no function, and the guest under it.
fn fixture() -> (Host, Extension) {
    let host = Host::new(8, |linker| {
        linker.instance("nvs:ext/types@1.0.0")?;
        Ok(())
    })
    .expect("the host starts");
    let bytes = wat::parse_str(GUEST).expect("the test component compiles");
    let extension = Extension {
        path: PathBuf::from("orders.nvsx"),
        sha256: pin(&bytes),
        memory: None,
        grants: nvs_config::extension::Granted::default(),
        component: Component::new(host.engine(), &bytes).expect("the component compiles"),
        manifest: Manifest::parse(MANIFEST.as_bytes()).expect("the manifest reads"),
        source: Source {
            source: FORMAT,
            files: Vec::new(),
        },
    };
    (host, extension)
}

fn request(host: &Host, cpu: Duration, memory: Option<u64>) -> Request {
    host.request(Arc::new(Meter::new(cpu, memory)))
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

fn call(
    request: &Request,
    extension: &Extension,
    method: &str,
    args: &[Val],
) -> Result<Vec<Val>, Failure> {
    block_on(request.call(extension, method, args))
}

fn bump(request: &Request, extension: &Extension) -> i64 {
    match call(request, extension, "bump", &[]).as_deref() {
        Ok([Val::S64(n)]) => *n,
        other => panic!("`bump` returns one `s64`, not {other:?}"),
    }
}

/// The failure of `method`, and what the program sees of it.
fn failure(request: &Request, extension: &Extension, method: &str) -> (Failure, Outcome) {
    let failure = call(request, extension, method, &[]).expect_err("the call fails");
    let outcome = failure.outcome();
    (failure, outcome)
}

/// The crash `failure` is, which names the extension and `export`.
fn crash(failure: &Failure, export: &str) -> Crash {
    let Failure::Trap(crash) = failure else {
        panic!("a trap was expected, not {failure:?}");
    };
    assert_eq!(crash.extension, "Shop\\Orders");
    assert_eq!(crash.export, export);
    crash.clone()
}

fn assert_throws(outcome: &Outcome, class: Class, thrown: ThrownClass, message: &str) {
    assert_eq!(outcome, &Outcome::Throw(class, message.to_owned()));
    assert_eq!(class.name(), thrown.name());
}

#[test]
fn an_err_invalid_throws_logic_error_with_its_message() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_secs(30), None);
    assert_eq!(bump(&request, &extension), 1);
    let (failure, outcome) = failure(&request, &extension, "invalid");
    assert_eq!(
        failure,
        Failure::Error(Error::Invalid("no lines".to_owned()))
    );
    assert_throws(&outcome, Class::Logic, ThrownClass::Logic, "no lines");
    assert_eq!(
        bump(&request, &extension),
        2,
        "an `err` is a return, so the instance is kept"
    );
}

#[test]
fn an_err_parse_throws_parse_error_with_its_message() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_secs(30), None);
    let (failure, outcome) = failure(&request, &extension, "parse");
    assert_eq!(
        failure,
        Failure::Error(Error::Parse("not a date".to_owned()))
    );
    assert_throws(&outcome, Class::Parse, ThrownClass::Parse, "not a date");
}

#[test]
fn an_err_runtime_throws_runtime_error_with_its_message() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_secs(30), None);
    let (failure, outcome) = failure(&request, &extension, "runtime");
    assert_eq!(failure, Failure::Error(Error::Runtime("closed".to_owned())));
    assert_throws(&outcome, Class::Runtime, ThrownClass::Runtime, "closed");
}

#[test]
fn a_guest_trap_throws_extension_error_naming_the_extension_the_export_and_the_trap() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_secs(30), None);
    let (failure, outcome) = failure(&request, &extension, "crash");
    let crash = crash(&failure, "crash");
    assert!(crash.reason.contains("unreachable"), "{}", crash.reason);
    let Outcome::Throw(class, message) = &outcome else {
        panic!("a trap throws, and {outcome:?} does not");
    };
    assert_throws(&outcome, *class, ThrownClass::Extension, message);
    assert_eq!(*class, Class::Extension);
    for part in ["Shop\\Orders", "`crash`", crash.reason.as_str()] {
        assert!(message.contains(part), "{message} names {part}");
    }
}

#[test]
fn a_guest_returning_invalid_utf8_throws_extension_error() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_secs(30), None);
    let (failure, outcome) = failure(&request, &extension, "name");
    let crash = crash(&failure, "name");
    assert!(
        crash.reason.to_lowercase().contains("utf-8"),
        "{}",
        crash.reason
    );
    assert!(
        matches!(outcome, Outcome::Throw(Class::Extension, _)),
        "{outcome:?}"
    );
}

#[test]
fn the_next_call_after_a_trap_gets_a_fresh_instance_and_succeeds() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_secs(30), None);
    assert_eq!(bump(&request, &extension), 1);
    assert_eq!(bump(&request, &extension), 2);
    let (failure, _) = failure(&request, &extension, "crash");
    crash(&failure, "crash");
    assert_eq!(request.instances(), 0, "the trapped instance is dropped");
    assert_eq!(
        bump(&request, &extension),
        1,
        "the next call starts from a fresh instance"
    );
    assert_eq!(request.instances(), 1);
}

#[test]
fn a_guest_past_the_request_cpu_limit_ends_the_request_with_a_resource_limit_fatal() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_millis(50), None);
    let (failure, outcome) = failure(&request, &extension, "forever");
    assert_eq!(failure, Failure::Limit(Limit::Cpu));
    assert!(
        matches!(outcome, Outcome::Fatal(Limit::Cpu, _)),
        "{outcome:?}"
    );
}

#[test]
fn a_guest_past_the_request_memory_limit_ends_the_request_with_a_resource_limit_fatal() {
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_secs(30), Some(4 * PAGE));
    let out = call(&request, &extension, "grow", &[Val::U32(100)]);
    let failure = out.expect_err("the growth is past the limit");
    assert_eq!(failure, Failure::Limit(Limit::Memory));
    assert!(
        matches!(failure.outcome(), Outcome::Fatal(Limit::Memory, _)),
        "{:?}",
        failure.outcome()
    );
}

#[test]
fn a_guest_resource_limit_reaches_on_limit_and_no_catch() {
    // A `FATAL` is what reaches `onLimit`, under the limit's `[limits]` name. A throw is what a
    // `catch` sees, and a limit is never one.
    for (limit, runtime) in [
        (Limit::Cpu, nvs_runtime::Limit::CpuTime),
        (Limit::Memory, nvs_runtime::Limit::Memory),
    ] {
        let outcome = Failure::Limit(limit).outcome();
        assert!(
            matches!(outcome, Outcome::Fatal(reached, _) if reached == limit),
            "{outcome:?}"
        );
        assert_eq!(limit.name(), runtime.name());
    }
    let (host, extension) = fixture();
    let request = request(&host, Duration::from_millis(50), None);
    let (_, outcome) = failure(&request, &extension, "forever");
    assert!(
        !matches!(outcome, Outcome::Throw(..)),
        "a limit reached inside a guest is not a throw: {outcome:?}"
    );
}
