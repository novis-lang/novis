//! `rule:packaging/a-value-crosses-as-its-wit-type`: every row of the table converts a Novis value to
//! its WIT value, crosses into a guest and comes back as the same value.
//!
//! The guest echoes its argument from one export per row. Every export returns
//! `result<T, error>` through memory at 16: its case at 16, then the `T` at 20 when `T` aligns to 4
//! or less, and at 24 when it aligns to 8. A string or a list is a pointer and a length. `realloc` is
//! a bump allocator from 1024, which is where the host writes a string or a list it passes in.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

use nvs_ext::call::{Failure, Host, Meter, Request};
use nvs_ext::convert::{Key, Value, from_wit, to_wit};
use nvs_ext::load::{Extension, pin};
use nvs_ext::manifest::Manifest;
use nvs_ext::source::{FORMAT, Source};
use nvs_ext::types::NovisType;
use wasmtime::component::{Component, Val};

const GUEST: &str = r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
  (import "shop:shapes/types" (instance $shapes
    (type $item (record (field "item-name" string) (field "count" (option s64))))
    (export "item" (type (eq $item)))))
  (alias export $shapes "item" (type $item))
  (core module $m
    (memory (export "memory") 1)
    (global $bump (mut i32) (i32.const 1024))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
      (local.set $at
        (i32.and
          (i32.add (global.get $bump) (i32.sub (local.get 2) (i32.const 1)))
          (i32.sub (i32.const 0) (local.get 2))))
      (global.set $bump (i32.add (local.get $at) (local.get 3)))
      (local.get $at))
    (func (export "echo-bool") (param i32) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store8 (i32.const 20) (local.get 0))
      (i32.const 16))
    (func (export "echo-i64") (param i64) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i64.store (i32.const 24) (local.get 0))
      (i32.const 16))
    (func (export "echo-f64") (param f64) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (f64.store (i32.const 24) (local.get 0))
      (i32.const 16))
    (func (export "echo-span") (param i32 i32) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store (i32.const 20) (local.get 0))
      (i32.store (i32.const 24) (local.get 1))
      (i32.const 16))
    (func (export "echo-option") (param i32 i32 i32) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store8 (i32.const 20) (local.get 0))
      (i32.store (i32.const 24) (local.get 1))
      (i32.store (i32.const 28) (local.get 2))
      (i32.const 16))
    (func (export "echo-record") (param i32 i32 i32 i64) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store (i32.const 24) (local.get 0))
      (i32.store (i32.const 28) (local.get 1))
      (i32.store8 (i32.const 32) (local.get 2))
      (i64.store (i32.const 40) (local.get 3))
      (i32.const 16)))
  (core instance $i (instantiate $m))
  (alias core export $i "memory" (core memory $mem))
  (alias core export $i "realloc" (core func $realloc))
  (func $bool (param "value" bool) (result (result bool (error $error)))
    (canon lift (core func $i "echo-bool") (memory $mem) (realloc $realloc)))
  (func $int (param "value" s64) (result (result s64 (error $error)))
    (canon lift (core func $i "echo-i64") (memory $mem) (realloc $realloc)))
  (func $uint (param "value" u64) (result (result u64 (error $error)))
    (canon lift (core func $i "echo-i64") (memory $mem) (realloc $realloc)))
  (func $float (param "value" f64) (result (result f64 (error $error)))
    (canon lift (core func $i "echo-f64") (memory $mem) (realloc $realloc)))
  (func $string (param "value" string) (result (result string (error $error)))
    (canon lift (core func $i "echo-span") (memory $mem) (realloc $realloc)))
  (func $bytes (param "value" (list u8)) (result (result (list u8) (error $error)))
    (canon lift (core func $i "echo-span") (memory $mem) (realloc $realloc)))
  (func $list (param "value" (list string)) (result (result (list string) (error $error)))
    (canon lift (core func $i "echo-span") (memory $mem) (realloc $realloc)))
  (func $keyed (param "value" (list (tuple string s64)))
    (result (result (list (tuple string s64)) (error $error)))
    (canon lift (core func $i "echo-span") (memory $mem) (realloc $realloc)))
  (func $optional (param "value" (option string)) (result (result (option string) (error $error)))
    (canon lift (core func $i "echo-option") (memory $mem) (realloc $realloc)))
  (func $shape (param "value" $item) (result (result $item (error $error)))
    (canon lift (core func $i "echo-record") (memory $mem) (realloc $realloc)))
  (instance $api
    (export "echo-bool" (func $bool))
    (export "echo-int" (func $int))
    (export "echo-uint" (func $uint))
    (export "echo-float" (func $float))
    (export "echo-string" (func $string))
    (export "echo-bytes" (func $bytes))
    (export "echo-list" (func $list))
    (export "echo-keyed" (func $keyed))
    (export "echo-optional" (func $optional))
    (export "echo-shape" (func $shape)))
  (export "shop:shapes/api" (instance $api)))"#;

/// Each export's method, and the Novis type of its one parameter and its return.
const ROWS: [(&str, &str); 10] = [
    ("echoBool", "bool"),
    ("echoInt", "int"),
    ("echoUint", "uint"),
    ("echoFloat", "float"),
    ("echoString", "string"),
    ("echoBytes", "bytes"),
    ("echoList", "array<string>"),
    ("echoKeyed", "array<string, int>"),
    ("echoOptional", "?string"),
    ("echoShape", "{itemName: string, count?: int}"),
];

/// A host linking the two type instances the guest imports, which offer types and no function, and
/// the guest under it.
fn fixture() -> (Host, Extension) {
    let host = Host::new(8, |linker| {
        linker.instance("nvs:ext/types@1.0.0")?;
        linker.instance("shop:shapes/types")?;
        Ok(())
    })
    .expect("the host starts");
    let methods: Vec<String> = ROWS
        .iter()
        .map(|(name, ty)| {
            format!(
                r#"{{"name": "{name}", "params": [{{"name": "value", "type": "{ty}"}}], "returns": "{ty}"}}"#
            )
        })
        .collect();
    let manifest = format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "Shop\\Shapes", "interface": "shop:shapes/api", "methods": [{}]}}"#,
        methods.join(", ")
    );
    let bytes = wat::parse_str(GUEST).expect("the test component compiles");
    let extension = Extension {
        path: PathBuf::from("shapes.nvsx"),
        sha256: pin(&bytes),
        memory: None,
        component: Component::new(host.engine(), &bytes).expect("the component compiles"),
        manifest: Manifest::parse(manifest.as_bytes()).expect("the manifest reads"),
        source: Source {
            source: FORMAT,
            files: Vec::new(),
        },
    };
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

fn request(host: &Host) -> Request {
    host.request(Arc::new(Meter::new(Duration::from_secs(30), None)))
}

fn ty(text: &str) -> NovisType {
    NovisType::parse(text).expect("the type parses")
}

/// `value` sent to the guest's `method`, whose type is `text`, and what came back.
fn echo(
    request: &Request,
    extension: &Extension,
    method: &str,
    text: &str,
    value: &Value,
) -> Value {
    let ty = ty(text);
    let arg = to_wit(&ty, value.clone()).expect("the value converts");
    let out: Result<Vec<Val>, Failure> = block_on(request.call(extension, method, &[arg]));
    match <[Val; 1]>::try_from(out.expect("the call succeeds")) {
        Ok([val]) => from_wit(&ty, val).expect("the result converts"),
        Err(other) => panic!("one result was expected, not {other:?}"),
    }
}

fn s(text: &str) -> Value {
    Value::String(text.to_owned())
}

fn k(text: &str) -> Key {
    Key::String(text.to_owned())
}

fn list(values: Vec<Value>) -> Value {
    Value::Array((0..).map(Key::Int).zip(values).collect())
}

/// The value of each row, `?T` and a shape twice: with the optional part and without it.
fn samples() -> Vec<(&'static str, &'static str, Value)> {
    let row = |method: &str| {
        ROWS.iter()
            .find(|(m, _)| *m == method)
            .copied()
            .expect("a row")
    };
    let mut out = Vec::new();
    let mut add = |method: &'static str, value: Value| {
        let (method, ty) = row(method);
        out.push((method, ty, value));
    };
    add("echoBool", Value::Bool(true));
    add("echoInt", Value::Int(-42));
    add("echoUint", Value::Uint(7));
    add("echoFloat", Value::Float(2.5));
    add("echoString", s("grüße, 世界"));
    add("echoBytes", Value::Bytes(vec![0, 1, 0x7f, 0xff]));
    add("echoList", list(vec![s("a"), s(""), s("c")]));
    add(
        "echoKeyed",
        Value::Array(vec![(k("b"), Value::Int(2)), (k("a"), Value::Int(1))]),
    );
    add("echoOptional", Value::Null);
    add("echoOptional", s("set"));
    add(
        "echoShape",
        Value::Array(vec![
            (k("itemName"), s("lamp")),
            (k("count"), Value::Int(3)),
        ]),
    );
    add("echoShape", Value::Array(vec![(k("itemName"), s("lamp"))]));
    out
}

#[test]
fn every_row_of_the_value_table_round_trips_through_a_guest() {
    let (host, extension) = fixture();
    let request = request(&host);
    let samples = samples();
    for (_, ty) in ROWS {
        assert!(
            samples.iter().any(|(_, t, _)| *t == ty),
            "the row `{ty}` has no sample"
        );
    }
    for (method, ty, value) in &samples {
        assert_eq!(
            &echo(&request, &extension, method, ty, value),
            value,
            "the row `{ty}` came back changed"
        );
    }
}

#[test]
fn a_uint_crosses_as_u64_with_no_conversion() {
    let uint = ty("uint");
    assert_eq!(
        to_wit(&uint, Value::Uint(u64::MAX)).expect("a uint converts"),
        Val::U64(u64::MAX)
    );
    assert!(
        to_wit(&uint, Value::Int(1)).is_err(),
        "an int is not a uint"
    );
    let (host, extension) = fixture();
    let request = request(&host);
    let max = Value::Uint(u64::MAX);
    assert_eq!(echo(&request, &extension, "echoUint", "uint", &max), max);
}

#[test]
fn an_optional_shape_field_crosses_as_an_option() {
    let shape = ty("{itemName: string, count?: int}");
    let record = |count: Option<Val>| {
        Val::Record(vec![
            ("item-name".to_owned(), Val::String("lamp".to_owned())),
            ("count".to_owned(), Val::Option(count.map(Box::new))),
        ])
    };
    let with = Value::Array(vec![
        (k("count"), Value::Int(3)),
        (k("itemName"), s("lamp")),
    ]);
    assert_eq!(
        to_wit(&shape, with).expect("the shape converts"),
        record(Some(Val::S64(3)))
    );
    let without = Value::Array(vec![(k("itemName"), s("lamp"))]);
    assert_eq!(
        to_wit(&shape, without.clone()).expect("the shape converts"),
        record(None)
    );
    assert_eq!(
        from_wit(&shape, record(None)).expect("the record converts"),
        without,
        "a `None` field is absent from the shape"
    );
    let missing = Value::Array(vec![(k("count"), Value::Int(3))]);
    assert!(
        to_wit(&shape, missing).is_err(),
        "a required field that is absent does not convert"
    );
}

#[test]
fn a_keyed_array_crosses_as_a_list_of_tuples_in_its_order() {
    let keyed = ty("array<string, int>");
    let value = Value::Array(vec![
        (k("pear"), Value::Int(3)),
        (k("apple"), Value::Int(1)),
        (Key::Int(5), Value::Int(2)),
    ]);
    let pair = |key: &str, n: i64| Val::Tuple(vec![Val::String(key.to_owned()), Val::S64(n)]);
    assert_eq!(
        to_wit(&keyed, value).expect("the array converts"),
        Val::List(vec![pair("pear", 3), pair("apple", 1), pair("5", 2)]),
        "an `int` key crosses as its text where the key type is `string`"
    );
    let (host, extension) = fixture();
    let request = request(&host);
    let ordered = Value::Array(vec![
        (k("pear"), Value::Int(3)),
        (k("apple"), Value::Int(1)),
        (k("fig"), Value::Int(2)),
    ]);
    assert_eq!(
        echo(
            &request,
            &extension,
            "echoKeyed",
            "array<string, int>",
            &ordered
        ),
        ordered
    );
}
